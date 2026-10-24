//! HTTP/1.1 client: one keep-alive connection.
//!
//! - [`HttpDial`] sets the connection up without blocking.
//! - [`HttpConn::request`] writes a request straight into one TLS record: one
//!   `send`, one TCP segment for a small request.
//! - Responses are parsed in place: a `Content-Length` body is a slice of the
//!   receive buffer. Responses match requests in order.
//! - One request in flight by default ([`HttpOptions::max_in_flight`] 1). A
//!   second [`HttpConn::request`] before the response returns
//!   [`Error::Busy`]: no hidden queue.
//! - A request past [`HttpOptions::request_timeout_ms`] closes its connection
//!   ([`Handler::on_closed`] with [`Error::Timeout`]): HTTP/1.1 cannot drop
//!   one request on a live connection.
//! - A connection that closes with requests in flight reports
//!   [`Error::Unanswered`]: the cause, and the first request whose TLS
//!   record never wholly reached the kernel (it, and every later one,
//!   cannot have run). Cost: one add per `send`, one store per request.
//! - Raising `max_in_flight` turns on pipelining. Off by default: not every
//!   server or CDN handles it, and one slow response delays every request
//!   behind it.
//! - Several requests at once: keep a small pool of warm connections, each
//!   with nothing in flight ([`HttpConn::in_flight`] 0), or use
//!   `HttpClient`, which does this. A WebSocket API (many requests on one
//!   connection, matched by id) is the other option.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;

use rustls::ClientConfig;

use crate::RxMeta;
use crate::clock::monotonic_ns;
use crate::endpoint::sealed::{Deliver, DialIo, Io};
use crate::endpoint::{ConnId, ConnMut, Handler};
use crate::error::{Error, Result};
use crate::net::{Target, TlsDial};
use crate::stream::TlsStream;
use crate::tls::MAX_RECORD;

/// Largest request (line, headers and body) [`HttpConn::request`] accepts: one
/// TLS record of plaintext.
pub const MAX_REQUEST: usize = 1 << 14;
const MAX_HEADERS: usize = 64;
/// Upper bound for [`HttpOptions::max_in_flight`].
pub const MAX_IN_FLIGHT: usize = 1024;

/// Header names the client manages itself and callers may not set.
const RESERVED: [&str; 8] = [
    "content-length",
    "transfer-encoding",
    "host",
    "connection",
    "te",
    "upgrade",
    "trailer",
    "expect",
];

/// RFC 9110 token characters.
pub(crate) const fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn validate_header(name: &str, value: &str) -> Result<()> {
    if name.is_empty() || !name.bytes().all(is_tchar) {
        return Err(Error::Http(format!("invalid header name {name:?}")));
    }
    if RESERVED.iter().any(|r| name.eq_ignore_ascii_case(r)) {
        return Err(Error::Http(format!("header {name} is set by the client")));
    }
    // Field values: visible ASCII, space, tab, or obs-text; never CR, LF or NUL.
    if value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
        return Err(Error::Http(format!("invalid value for header {name}")));
    }
    Ok(())
}

pub(crate) fn validate_request(method: &str, path: &str, headers: &[(&str, &str)]) -> Result<()> {
    if method.is_empty() || !method.bytes().all(is_tchar) {
        return Err(Error::Http(format!("invalid method {method:?}")));
    }
    if path.is_empty() || path.bytes().any(|b| b <= b' ' || b == 0x7F) {
        return Err(Error::Http("invalid request target".into()));
    }
    for (k, v) in headers {
        validate_header(k, v)?;
    }
    Ok(())
}

/// Whether a response ends the connection: a `close` token in any
/// `Connection` header, or HTTP/1.0 without `keep-alive`.
fn closes(version: Option<u8>, headers: &[httparse::Header<'_>]) -> bool {
    let token = |t: &[u8]| {
        headers
            .iter()
            .filter(|h| h.name.eq_ignore_ascii_case("connection"))
            .flat_map(|h| h.value.split(|&b| b == b','))
            .any(|v| v.trim_ascii().eq_ignore_ascii_case(t))
    };
    token(b"close") || (version == Some(0) && !token(b"keep-alive"))
}

/// A response, borrowed from the receive buffer for the duration of
/// [`HttpHandler::on_response`].
#[derive(Debug)]
pub struct Response<'a> {
    /// Status code.
    pub status: u16,
    /// Headers in arrival order.
    pub headers: &'a [httparse::Header<'a>],
    /// The body (de-chunked if the server used chunked encoding).
    pub body: &'a [u8],
}

impl Response<'_> {
    /// The first header named `name` (ASCII case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value)
    }
}

/// Receives HTTP responses.
pub trait HttpHandler: Handler {
    /// The response to `request` (the id [`HttpConn::request`] returned).
    fn on_response(&mut self, conn: ConnId, request: u64, response: &Response<'_>, meta: &RxMeta);
}

/// Per-connection settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpOptions {
    /// Time from sending a request to its complete response, in milliseconds.
    pub request_timeout_ms: u32,
    /// Requests allowed in flight on the connection (1: no pipelining).
    pub max_in_flight: usize,
    /// Receive buffer size; it must hold the largest response.
    pub buf_size: usize,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            request_timeout_ms: 5_000,
            max_in_flight: 1,
            buf_size: 1 << 20,
        }
    }
}

/// Counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HttpStats {
    /// Requests queued.
    pub requests: u64,
    /// Responses delivered.
    pub responses: u64,
    /// Bytes received from the kernel.
    pub bytes_in: u64,
    /// TLS records opened.
    pub records: u64,
    /// Plaintext bytes moved because a response spanned records.
    pub span_copy_bytes: u64,
    /// Bytes moved by buffer compaction.
    pub compact_bytes: u64,
    /// Bytes copied to de-chunk bodies.
    pub chunk_copy_bytes: u64,
}

struct InFlight {
    id: u64,
    deadline_ns: i64,
    head: bool,
    /// Bytes the connection must hand to the kernel before this request's
    /// record is wholly handed (a count since the connection opened).
    end: u64,
}

/// A connected HTTP/1.1 client connection. Hand it to a `Driver`; the socket
/// closes when it is dropped.
pub struct HttpConn {
    stream: TlsStream,
    /// `Host` and the target's headers, serialised once.
    fixed: Vec<u8>,
    in_flight: VecDeque<InFlight>,
    next_id: u64,
    opts: HttpOptions,
    body: Vec<u8>,
    /// Window length needed before the front response can complete.
    need: usize,
    requests: u64,
    responses: u64,
    chunk_copy_bytes: u64,
    /// Bytes handed to the kernel since the connection opened.
    handed: u64,
}

impl HttpConn {
    /// Queues a request for the driver's next `poll` or `flush` and returns its
    /// id. `headers` come after the connection's fixed headers;
    /// `Content-Length` is added when there is a body or the method is `POST`,
    /// `PUT` or `PATCH`.
    ///
    /// # Errors
    /// [`Error::Busy`] when `max_in_flight` requests are outstanding, or the
    /// request is larger than [`MAX_REQUEST`] or contains CR/LF in a field.
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<u64> {
        self.queue(method, path, headers, body, |_| {})
    }

    /// Builds one request into one TLS record; `query` appends to the target
    /// right after `path`.
    fn queue(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        query: impl FnOnce(&mut Vec<u8>),
    ) -> Result<u64> {
        if self.in_flight.len() >= self.opts.max_in_flight {
            return Err(Error::Busy("max_in_flight requests outstanding"));
        }
        validate_request(method, path, headers)?;
        let with_length = !body.is_empty() || matches!(method, "POST" | "PUT" | "PATCH");
        let fixed = &self.fixed;
        self.stream.seal_build(|out| {
            let start = out.len();
            out.extend_from_slice(method.as_bytes());
            out.push(b' ');
            out.extend_from_slice(path.as_bytes());
            query(out);
            out.extend_from_slice(b" HTTP/1.1\r\n");
            out.extend_from_slice(fixed);
            for (k, v) in headers {
                out.extend_from_slice(k.as_bytes());
                out.extend_from_slice(b": ");
                out.extend_from_slice(v.as_bytes());
                out.extend_from_slice(b"\r\n");
            }
            if with_length {
                out.extend_from_slice(b"Content-Length: ");
                push_decimal(out, body.len());
                out.extend_from_slice(b"\r\n");
            }
            out.extend_from_slice(b"\r\n");
            out.extend_from_slice(body);
            if out.len() - start > MAX_REQUEST {
                return Err(Error::Http("request larger than one TLS record".into()));
            }
            Ok(())
        })?;
        let id = self.next_id;
        self.next_id += 1;
        let queued = self.stream.tx_queue().len() as u64;
        self.in_flight.push_back(InFlight {
            id,
            deadline_ns: monotonic_ns() + i64::from(self.opts.request_timeout_ms) * 1_000_000,
            head: method == "HEAD",
            end: self.handed + queued,
        });
        self.requests += 1;
        Ok(id)
    }

    /// Queues a signed request: `params` (already percent-encoded), then
    /// `timestamp=<now, ms>` and a `signature` over exactly those bytes, in
    /// the query string. The signature is computed into the TLS record being
    /// built, without allocating. Set the API key header once with
    /// `Target::with_header`.
    ///
    /// # Errors
    /// As [`HttpConn::request`], or `params` needs percent-encoding.
    pub fn signed_request(
        &mut self,
        method: &str,
        path: &str,
        params: &str,
        headers: &[(&str, &str)],
        signer: &crate::auth::Signer,
    ) -> Result<u64> {
        let now_ms = u64::try_from(crate::clock::realtime_ns() / 1_000_000).unwrap_or(0);
        self.signed_request_at(method, path, params, headers, signer, now_ms)
    }

    /// [`HttpConn::signed_request`] with an explicit timestamp (tests, replay).
    ///
    /// # Errors
    /// As [`HttpConn::signed_request`].
    pub fn signed_request_at(
        &mut self,
        method: &str,
        path: &str,
        params: &str,
        headers: &[(&str, &str)],
        signer: &crate::auth::Signer,
        timestamp_ms: u64,
    ) -> Result<u64> {
        crate::auth::validate_params(params)?;
        if path.contains('?') {
            return Err(Error::Http("put query parameters in `params`".into()));
        }
        self.queue(method, path, headers, &[], |out| {
            out.push(b'?');
            crate::auth::append_signed_params(out, params, timestamp_ms, signer);
        })
    }

    /// Requests sent and not yet answered.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> HttpStats {
        let s = self.stream.stats();
        HttpStats {
            requests: self.requests,
            responses: self.responses,
            bytes_in: s.bytes_in,
            records: s.records,
            span_copy_bytes: s.span_copy_bytes,
            compact_bytes: s.compact_bytes,
            chunk_copy_bytes: self.chunk_copy_bytes,
        }
    }

    /// The socket's descriptor, for socket options and diagnostics. Do not read
    /// from or write to it: the driver owns the byte stream.
    #[must_use]
    pub fn raw_fd(&self) -> std::os::fd::RawFd {
        self.stream.fd()
    }

    /// What the quick-ACK route check found for this connection
    /// ([`crate::route::RouteCheck::Pending`] while ergon-io's warning
    /// thread is still looking it up).
    #[must_use]
    pub fn route(&self) -> &crate::route::RouteCheck {
        self.stream.route()
    }

    #[inline]
    fn deliver<H: HttpHandler>(
        &mut self,
        id: ConnId,
        n: usize,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        self.stream.received(n);
        let mut delivered = 0;
        if n == 0 {
            delivered += self.parse(id, meta, handler)?;
        }
        while self.stream.open_next()? {
            delivered += self.parse(id, meta, handler)?;
        }
        self.stream.compact();
        Ok(delivered)
    }

    /// Delivers every complete response in the window.
    fn parse<H: HttpHandler>(
        &mut self,
        id: ConnId,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        let mut delivered = 0;
        loop {
            let window = self.stream.window();
            if window.is_empty() || window.len() < self.need {
                return Ok(delivered);
            }
            self.need = 0;
            let Some(req) = self.in_flight.front() else {
                return Err(Error::Http("response without a request".into()));
            };
            let mut hdrs = [httparse::EMPTY_HEADER; MAX_HEADERS];
            let mut resp = httparse::Response::new(&mut hdrs);
            let hl = match resp.parse(window).map_err(|e| Error::Http(e.to_string()))? {
                httparse::Status::Partial => return Ok(delivered),
                httparse::Status::Complete(n) => n,
            };
            let status = resp.code.unwrap_or(0);
            if (100..200).contains(&status) {
                // Interim response (100 Continue): skip it.
                self.stream.consume(hl);
                continue;
            }
            let framing = framing(resp.headers, status, req.head)?;
            let close = closes(resp.version, resp.headers);
            let limit = self.opts.buf_size - 2 * MAX_RECORD;
            let (body, total) = match framing {
                Framing::Length(len) => {
                    let total = hl
                        .checked_add(len)
                        .filter(|&t| t <= limit)
                        .ok_or_else(|| Error::Http("response larger than the buffer".into()))?;
                    if window.len() < total {
                        // Skip re-parsing until the whole response is here.
                        self.need = total;
                        return Ok(delivered);
                    }
                    (&window[hl..total], total)
                }
                Framing::Chunked => {
                    let Some(end) = dechunk(&window[hl..], &mut self.body, limit)? else {
                        if window.len() > limit {
                            return Err(Error::Http("response larger than the buffer".into()));
                        }
                        return Ok(delivered);
                    };
                    self.chunk_copy_bytes += self.body.len() as u64;
                    (self.body.as_slice(), hl + end)
                }
            };
            let request = req.id;
            handler.on_response(
                id,
                request,
                &Response {
                    status,
                    headers: resp.headers,
                    body,
                },
                meta,
            );
            self.in_flight.pop_front();
            self.stream.consume(total);
            self.responses += 1;
            delivered += 1;
            if close {
                return Err(Error::Closed);
            }
        }
    }
}

/// A driver-owned [`HttpConn`]: the same request methods.
impl ConnMut<'_, HttpConn> {
    /// [`HttpConn::request`].
    ///
    /// # Errors
    /// As [`HttpConn::request`].
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<u64> {
        self.inner().request(method, path, headers, body)
    }

    /// [`HttpConn::signed_request`].
    ///
    /// # Errors
    /// As [`HttpConn::signed_request`].
    pub fn signed_request(
        &mut self,
        method: &str,
        path: &str,
        params: &str,
        headers: &[(&str, &str)],
        signer: &crate::auth::Signer,
    ) -> Result<u64> {
        self.inner()
            .signed_request(method, path, params, headers, signer)
    }

    /// [`HttpConn::signed_request_at`].
    ///
    /// # Errors
    /// As [`HttpConn::signed_request_at`].
    pub fn signed_request_at(
        &mut self,
        method: &str,
        path: &str,
        params: &str,
        headers: &[(&str, &str)],
        signer: &crate::auth::Signer,
        timestamp_ms: u64,
    ) -> Result<u64> {
        self.inner()
            .signed_request_at(method, path, params, headers, signer, timestamp_ms)
    }
}

enum Framing {
    Length(usize),
    Chunked,
}

fn framing(headers: &[httparse::Header<'_>], status: u16, head: bool) -> Result<Framing> {
    if head || status == 204 || status == 304 {
        return Ok(Framing::Length(0));
    }
    let mut length = None;
    for h in headers {
        if h.name.eq_ignore_ascii_case("transfer-encoding") {
            if h.value.eq_ignore_ascii_case(b"chunked") {
                return Ok(Framing::Chunked);
            }
            return Err(Error::Http("unsupported transfer encoding".into()));
        }
        if h.name.eq_ignore_ascii_case("content-length") {
            let v = std::str::from_utf8(h.value)
                .ok()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .ok_or_else(|| Error::Http("bad Content-Length".into()))?;
            if length.is_some_and(|l| l != v) {
                return Err(Error::Http("conflicting Content-Length".into()));
            }
            length = Some(v);
        }
    }
    length
        .map(Framing::Length)
        .ok_or_else(|| Error::Http("response without Content-Length".into()))
}

/// De-chunks `data` into `out`. `Ok(None)` if incomplete; otherwise the bytes
/// consumed (including the last chunk and trailers).
fn dechunk(data: &[u8], out: &mut Vec<u8>, limit: usize) -> Result<Option<usize>> {
    out.clear();
    let mut pos = 0;
    loop {
        let (hdr, size) = match httparse::parse_chunk_size(&data[pos..])
            .map_err(|_| Error::Http("bad chunk size".into()))?
        {
            httparse::Status::Partial => return Ok(None),
            httparse::Status::Complete(v) => v,
        };
        let size = usize::try_from(size)
            .ok()
            .filter(|&s| s <= limit)
            .ok_or_else(|| Error::Http("chunk too large".into()))?;
        pos += hdr;
        if size == 0 {
            // Trailers end with an empty line.
            let rest = &data[pos..];
            let Some(end) = rest.windows(2).position(|w| w == b"\r\n") else {
                return Ok(None);
            };
            if end == 0 {
                return Ok(Some(pos + 2));
            }
            // Skip trailer fields up to the blank line.
            let Some(blank) = rest.windows(4).position(|w| w == b"\r\n\r\n") else {
                return Ok(None);
            };
            return Ok(Some(pos + blank + 4));
        }
        let end = pos
            .checked_add(size)
            .and_then(|e| e.checked_add(2))
            .ok_or_else(|| Error::Http("chunk too large".into()))?;
        if data.len() < end {
            return Ok(None);
        }
        out.extend_from_slice(&data[pos..end - 2]);
        if &data[end - 2..end] != b"\r\n" {
            return Err(Error::Http("chunk without CRLF".into()));
        }
        pos = end;
    }
}

fn push_decimal(out: &mut Vec<u8>, mut v: usize) {
    let mut digits = [0u8; 20];
    let mut i = digits.len();
    loop {
        i -= 1;
        #[allow(clippy::cast_possible_truncation)]
        {
            digits[i] = b'0' + (v % 10) as u8;
        }
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[i..]);
}

/// An HTTP connection being set up without blocking (TCP connect, TLS). Hand it
/// to `Driver::connect`.
pub struct HttpDial {
    target: Target,
    opts: HttpOptions,
    tls: TlsDial,
}

impl HttpDial {
    /// A dial to `target`; its headers are sent with every request.
    ///
    /// # Errors
    /// The buffer in `opts` is too small, or a header contains CR/LF.
    pub fn new(target: Target, cfg: Arc<ClientConfig>, opts: HttpOptions) -> Result<Self> {
        if opts.buf_size < 8 * MAX_RECORD {
            return Err(Error::Config(
                "receive buffer must hold at least 8 records".into(),
            ));
        }
        if opts.max_in_flight == 0 {
            return Err(Error::Config("max_in_flight must be at least 1".into()));
        }
        if opts.max_in_flight > MAX_IN_FLIGHT {
            return Err(Error::Config(format!(
                "max_in_flight above {MAX_IN_FLIGHT}"
            )));
        }
        for (k, v) in &target.headers {
            validate_header(k, v)?;
        }
        let sni = target.host.clone();
        Ok(Self {
            target,
            opts,
            tls: TlsDial::new(cfg, sni)?,
        })
    }

    fn fixed_headers(&self) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(b"Host: ");
        f.extend_from_slice(self.target.host_header().as_bytes());
        f.extend_from_slice(b"\r\n");
        for (k, v) in &self.target.headers {
            f.extend_from_slice(k.as_bytes());
            f.extend_from_slice(b": ");
            f.extend_from_slice(v.as_bytes());
            f.extend_from_slice(b"\r\n");
        }
        f
    }
}

impl DialIo for HttpDial {
    type Conn = HttpConn;

    fn host(&self) -> &str {
        &self.target.host
    }

    fn port(&self) -> u16 {
        self.target.port
    }

    fn addr(&self) -> Option<SocketAddr> {
        self.target.addr
    }

    fn start(&mut self, addr: SocketAddr) -> Result<()> {
        self.tls.start(addr)
    }

    fn step(&mut self) -> Result<Option<HttpConn>> {
        let Some((sock, tls, rest)) = self.tls.step()? else {
            return Ok(None);
        };
        // Non-zero fill writes every page now (zeroed memory is mapped lazily),
        // so the first receives do not take page faults.
        let mut buf = vec![1u8; self.opts.buf_size].into_boxed_slice();
        if rest.len() > buf.len() {
            return Err(Error::Record("handshake leftover larger than the buffer"));
        }
        buf[..rest.len()].copy_from_slice(&rest);
        let mut stream = TlsStream::new(sock, buf, 0, rest.len(), tls, Vec::with_capacity(1 << 16));
        if let Some(check) = self.tls.take_route() {
            stream.set_route(crate::endpoint::sealed::RouteSlot::Ready(check));
        }
        Ok(Some(HttpConn {
            stream,
            fixed: self.fixed_headers(),
            in_flight: VecDeque::with_capacity(self.opts.max_in_flight),
            next_id: 0,
            opts: self.opts,
            body: Vec::with_capacity(1 << 16),
            need: 0,
            requests: 0,
            responses: 0,
            chunk_copy_bytes: 0,
            handed: 0,
        }))
    }

    fn gate_route(&mut self) {
        self.tls.gate_route();
    }

    fn awaiting_route(&self) -> Option<std::os::fd::RawFd> {
        self.tls.awaiting_route()
    }

    fn route_checked(&mut self, check: crate::route::RouteCheck) {
        self.tls.route_checked(check);
    }
}

impl HttpDial {
    /// Connects and runs the TLS handshake, waiting until done: a convenience
    /// for tools and tests. In a busy-spin loop use `Driver::connect`. Without
    /// [`Target::addr`] the host is resolved with the system resolver (blocking).
    ///
    /// # Errors
    /// Resolution, connect or TLS failure, or no progress for 30 s.
    pub fn connect_blocking(mut self) -> Result<HttpConn> {
        let addr = match self.target.addr {
            Some(a) => a,
            None => std::net::ToSocketAddrs::to_socket_addrs(&(
                self.target.host.as_str(),
                self.target.port,
            ))?
            .find(SocketAddr::is_ipv4)
            .ok_or_else(|| Error::Dns(format!("{}: no IPv4 address", self.target.host)))?,
        };
        self.start(addr)?;
        let start = std::time::Instant::now();
        loop {
            if let Some(c) = self.step()? {
                return Ok(c);
            }
            if start.elapsed() > std::time::Duration::from_secs(30) {
                return Err(Error::Timeout("http connect"));
            }
            std::thread::yield_now();
        }
    }
}

impl Io for HttpConn {
    const DEADLINES: bool = true;

    #[inline]
    fn fd(&self) -> std::os::fd::RawFd {
        self.stream.fd()
    }
    #[inline]
    fn rx_space(&mut self) -> &mut [u8] {
        self.stream.rx_space()
    }
    #[inline]
    fn tx_queue(&mut self) -> &mut crate::stream::TxQueue {
        self.stream.tx_queue()
    }
    fn has_buffered_input(&self) -> bool {
        self.stream.has_buffered_input()
    }
    fn set_route(&mut self, route: crate::endpoint::sealed::RouteSlot) {
        self.stream.set_route(route);
    }
    fn route_state(&self) -> &crate::endpoint::sealed::RouteSlot {
        self.stream.route_state()
    }
    fn expire(&mut self, now_ns: i64) -> Result<()> {
        match self.in_flight.front() {
            Some(r) if r.deadline_ns <= now_ns => Err(Error::Timeout("http request")),
            _ => Ok(()),
        }
    }
    #[inline]
    fn on_handed(&mut self, n: usize) {
        self.handed += n as u64;
    }
    /// With requests in flight: [`Error::Unanswered`], naming the first whose
    /// record the kernel never wholly took.
    #[cold]
    fn close_reason(&mut self, error: Error) -> Error {
        if self.in_flight.is_empty() {
            return error;
        }
        let first_unsent = self
            .in_flight
            .iter()
            .find(|r| r.end > self.handed)
            .map(|r| r.id);
        Error::Unanswered(Box::new(crate::error::Unanswered {
            cause: error,
            first_unsent,
        }))
    }
}

impl<H: HttpHandler> Deliver<H> for HttpConn {
    #[inline]
    fn on_received(
        &mut self,
        id: ConnId,
        n: usize,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        self.deliver(id, n, meta, handler)
    }
}
