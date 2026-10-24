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

/// [`is_tchar`] as a table: one load per byte.
static TCHAR: [bool; 256] = {
    let mut t = [false; 256];
    let mut i = 0;
    while i < 256 {
        #[allow(clippy::cast_possible_truncation)]
        {
            t[i] = is_tchar(i as u8);
        }
        i += 1;
    }
    t
};

/// Every byte of `b` is below 0x80: sixteen bytes per step, no branch per
/// word.
#[inline]
fn all_ascii(b: &[u8]) -> bool {
    let (words, rest) = b.as_chunks::<16>();
    let mut acc = 0u128;
    for w in words {
        acc |= u128::from_ne_bytes(*w);
    }
    let mut tail = 0u8;
    for &x in rest {
        tail |= x;
    }
    acc & 0x8080_8080_8080_8080_8080_8080_8080_8080 == 0 && tail < 0x80
}

/// A byte of `x` is zero.
const fn has_zero(x: u64) -> bool {
    x.wrapping_sub(0x0101_0101_0101_0101) & !x & 0x8080_8080_8080_8080 != 0
}

/// No CR, LF or NUL in `v`: eight bytes per step.
fn clean_value(v: &[u8]) -> bool {
    const CR: u64 = 0x0D0D_0D0D_0D0D_0D0D;
    const LF: u64 = 0x0A0A_0A0A_0A0A_0A0A;
    let (words, rest) = v.as_chunks::<8>();
    for w in words {
        let x = u64::from_ne_bytes(*w);
        if has_zero(x) || has_zero(x ^ CR) || has_zero(x ^ LF) {
            return false;
        }
    }
    !rest.iter().any(|&b| b == b'\r' || b == b'\n' || b == 0)
}

fn validate_header(name: &str, value: &str) -> Result<()> {
    if name.is_empty() || !name.bytes().all(|b| TCHAR[usize::from(b)]) {
        return Err(Error::Http(format!("invalid header name {name:?}")));
    }
    // Only names as long as a reserved one are compared.
    if matches!(name.len(), 2 | 4 | 6 | 7 | 10 | 14 | 17)
        && RESERVED.iter().any(|r| name.eq_ignore_ascii_case(r))
    {
        return Err(Error::Http(format!("header {name} is set by the client")));
    }
    // Field values: visible ASCII, space, tab, or obs-text; never CR, LF or NUL.
    if !clean_value(value.as_bytes()) {
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
    /// Every byte of the status line and headers is ASCII.
    pub head_ascii: bool,
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

/// A request's fixed bytes, built and checked once per endpoint.
///
/// It holds the request line around the target (`METHOD path` and
/// ` HTTP/1.1`) and the endpoint's fixed headers (`Host` and
/// [`Target::headers`]). [`HttpConn::request_with`] then writes only what
/// varies: a target suffix, per-request headers, `Content-Length` and the
/// body. The bytes on the wire equal [`HttpConn::request`]'s.
#[derive(Debug, Clone)]
pub struct RequestTemplate {
    /// `METHOD path`, then ` HTTP/1.1\r\n` and the fixed headers.
    bytes: Vec<u8>,
    /// End of `METHOD path`.
    line: usize,
    /// Start of the fixed headers.
    fixed: usize,
    /// Unique per construction (clones share it): lets a connection skip
    /// comparing fixed headers it has already matched.
    id: u64,
    head: bool,
    with_length: bool,
    has_query: bool,
}

impl RequestTemplate {
    /// The template for `method` and `path` on `target`.
    ///
    /// # Errors
    /// [`Error::Http`]: an invalid method, path or target header.
    pub fn new(target: &Target, method: &str, path: &str) -> Result<Self> {
        validate_request(method, path, &[])?;
        let fixed = fixed_headers(target)?;
        let mut bytes = Vec::with_capacity(method.len() + path.len() + 12 + fixed.len());
        bytes.extend_from_slice(method.as_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(path.as_bytes());
        let line = bytes.len();
        bytes.extend_from_slice(b" HTTP/1.1\r\n");
        let fixed_at = bytes.len();
        bytes.extend_from_slice(&fixed);
        Ok(Self {
            bytes,
            line,
            fixed: fixed_at,
            id: NEXT_TEMPLATE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            head: method == "HEAD",
            with_length: matches!(method, "POST" | "PUT" | "PATCH"),
            has_query: path.contains('?'),
        })
    }

    /// Appends one request's plaintext to `out`: the template's line with
    /// `suffix` right after its path (`""`, `"?a=1"`, `"/42"`), the fixed
    /// headers, `headers`, `Content-Length` (as [`HttpConn::request`]
    /// adds it) and `body`.
    ///
    /// # Errors
    /// [`Error::Http`]: an invalid suffix or header.
    pub fn write(
        &self,
        out: &mut Vec<u8>,
        suffix: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<()> {
        check_suffix(suffix)?;
        for (k, v) in headers {
            validate_header(k, v)?;
        }
        self.put(
            out,
            |out| out.extend_from_slice(suffix.as_bytes()),
            headers,
            body,
        );
        Ok(())
    }

    /// The fixed headers this template carries.
    fn fixed_headers(&self) -> &[u8] {
        &self.bytes[self.fixed..]
    }

    /// Writes a checked request: line, `target` (the suffix), fixed headers,
    /// `headers`, `Content-Length` when due, the blank line and `body`.
    #[inline]
    fn put(
        &self,
        out: &mut Vec<u8>,
        target: impl FnOnce(&mut Vec<u8>),
        headers: &[(&str, &str)],
        body: &[u8],
    ) {
        out.extend_from_slice(&self.bytes[..self.line]);
        target(out);
        out.extend_from_slice(&self.bytes[self.line..]);
        put_headers(out, headers);
        end_head(out, body, self.with_length);
    }
}

/// Template ids: unique per [`RequestTemplate::new`].
static NEXT_TEMPLATE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A target suffix follows the same rule as a path: no space, control
/// byte or DEL.
fn check_suffix(suffix: &str) -> Result<()> {
    if suffix.bytes().any(|b| b <= b' ' || b == 0x7F) {
        return Err(Error::Http("invalid request target".into()));
    }
    Ok(())
}

#[inline]
fn put_headers(out: &mut Vec<u8>, headers: &[(&str, &str)]) {
    for (k, v) in headers {
        out.extend_from_slice(k.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(v.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
}

/// `Content-Length` (when there is a body or the method takes one), the
/// blank line, then the body.
#[inline]
fn end_head(out: &mut Vec<u8>, body: &[u8], method_takes_body: bool) {
    if method_takes_body || !body.is_empty() {
        out.extend_from_slice(b"Content-Length: ");
        push_decimal(out, body.len());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
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
    /// What a response head with a non-ASCII byte does.
    pub non_ascii_head: NonAsciiHead,
    /// The check on every response body.
    pub body_check: TextCheck,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            request_timeout_ms: 5_000,
            max_in_flight: 1,
            buf_size: 1 << 20,
            non_ascii_head: NonAsciiHead::Flag,
            body_check: TextCheck::None,
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
    /// Response heads parsed (each `httparse` parse call).
    pub head_parses: u64,
    /// Responses whose head held a non-ASCII byte ([`NonAsciiHead::Flag`]).
    pub non_ascii_heads: u64,
}

/// What a response head with a non-ASCII byte does. Status lines and
/// headers are ASCII by RFC 9112; `httparse` still accepts bytes 0x80-0xFF
/// in the reason phrase and header values (obs-text).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NonAsciiHead {
    /// Deliver it with [`Response::head_ascii`] false and count it in
    /// [`HttpStats::non_ascii_heads`].
    #[default]
    Flag,
    /// Close the connection with [`Error::Http`].
    Reject,
}

/// A check on every response body. HTTP has no text rule for bodies, so the
/// default is none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextCheck {
    /// No check.
    #[default]
    None,
    /// Every byte is ASCII, else the connection closes with [`Error::Http`].
    Ascii,
    /// The body is UTF-8, else the connection closes with [`Error::Http`].
    Utf8,
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
    /// De-chunked bodies: whole-window copies, or a streamed response's head
    /// then its body. Allocated on first use.
    body: Vec<u8>,
    /// The front response, when its head is parsed and its body is not in.
    pending: Pending,
    /// The pending response's head, as offsets.
    head: Box<Head>,
    /// A streamed chunked body's framing state.
    dechunk: Dechunk,
    /// Bytes of the streamed chunked response consumed so far.
    wire: usize,
    requests: u64,
    responses: u64,
    chunk_copy_bytes: u64,
    head_parses: u64,
    non_ascii_heads: u64,
    /// The last [`RequestTemplate`] id whose fixed headers matched.
    template_ok: u64,
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

    /// [`HttpConn::request`] from a [`RequestTemplate`]: `suffix` goes right
    /// after the template's path. The template must come from this
    /// connection's target.
    ///
    /// # Errors
    /// As [`HttpConn::request`], or the template is for another target.
    pub fn request_with(
        &mut self,
        template: &RequestTemplate,
        suffix: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<u64> {
        self.ready_for(template)?;
        check_suffix(suffix)?;
        for (k, v) in headers {
            validate_header(k, v)?;
        }
        self.enqueue(template.head, |out| {
            template.put(
                out,
                |o| o.extend_from_slice(suffix.as_bytes()),
                headers,
                body,
            );
        })
    }

    /// [`HttpConn::signed_request_at`] from a [`RequestTemplate`] whose path
    /// has no query.
    ///
    /// # Errors
    /// As [`HttpConn::signed_request_at`], or the template is for another
    /// target.
    pub fn signed_request_with(
        &mut self,
        template: &RequestTemplate,
        params: &str,
        headers: &[(&str, &str)],
        signer: &crate::auth::Signer,
        timestamp_ms: u64,
    ) -> Result<u64> {
        self.ready_for(template)?;
        crate::auth::validate_params(params)?;
        if template.has_query {
            return Err(Error::Http("put query parameters in `params`".into()));
        }
        for (k, v) in headers {
            validate_header(k, v)?;
        }
        self.enqueue(template.head, |out| {
            template.put(
                out,
                |o| {
                    o.push(b'?');
                    crate::auth::append_signed_params(o, params, timestamp_ms, signer);
                },
                headers,
                &[],
            );
        })
    }

    /// Room for one more request, and `template` carries this connection's
    /// fixed headers (compared once per template).
    #[inline]
    fn ready_for(&mut self, template: &RequestTemplate) -> Result<()> {
        if self.in_flight.len() >= self.opts.max_in_flight {
            return Err(Error::Busy("max_in_flight requests outstanding"));
        }
        if template.id != self.template_ok {
            if template.fixed_headers() != self.fixed.as_slice() {
                return Err(Error::Http("request template is for another target".into()));
            }
            self.template_ok = template.id;
        }
        Ok(())
    }

    /// Seals one checked request (`build` writes it) into one TLS record and
    /// records it in flight.
    #[inline]
    fn enqueue(&mut self, head: bool, build: impl FnOnce(&mut Vec<u8>)) -> Result<u64> {
        seal_request(&mut self.stream, build)?;
        Ok(self.push_in_flight(head))
    }

    /// Records the request just queued as in flight; returns its id.
    #[inline]
    fn push_in_flight(&mut self, head: bool) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let queued = self.stream.tx_queue().len() as u64;
        self.in_flight.push_back(InFlight {
            id,
            deadline_ns: monotonic_ns() + i64::from(self.opts.request_timeout_ms) * 1_000_000,
            head,
            end: self.handed + queued,
        });
        self.requests += 1;
        id
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
        let with_length = matches!(method, "POST" | "PUT" | "PATCH");
        let fixed = &self.fixed;
        seal_request(&mut self.stream, |out| {
            out.extend_from_slice(method.as_bytes());
            out.push(b' ');
            out.extend_from_slice(path.as_bytes());
            query(out);
            out.extend_from_slice(b" HTTP/1.1\r\n");
            out.extend_from_slice(fixed);
            put_headers(out, headers);
            end_head(out, body, with_length);
        })?;
        Ok(self.push_in_flight(method == "HEAD"))
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
            head_parses: self.head_parses,
            non_ascii_heads: self.non_ascii_heads,
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

    fn build(stream: TlsStream, fixed: Vec<u8>, opts: HttpOptions) -> Self {
        Self {
            stream,
            fixed,
            in_flight: VecDeque::with_capacity(opts.max_in_flight),
            next_id: 0,
            opts,
            body: Vec::new(),
            pending: Pending::None,
            head: Box::new(Head::EMPTY),
            dechunk: Dechunk::new(),
            wire: 0,
            requests: 0,
            responses: 0,
            chunk_copy_bytes: 0,
            head_parses: 0,
            non_ascii_heads: 0,
            template_ok: 0,
            handed: 0,
        }
    }

    /// Test and bench lever: a connection over `sock` whose TLS keys are
    /// `tls`, sending `target`'s fixed headers. Nothing is read from or
    /// written to `sock`: [`HttpConn::feed`] and [`HttpConn::deliver_staged`]
    /// stand in for the kernel.
    ///
    /// # Errors
    /// Bad options, or a target header with CR/LF.
    #[doc(hidden)]
    pub fn from_record_layer(
        sock: std::net::TcpStream,
        tls: crate::tls::RecordLayer,
        target: &Target,
        opts: HttpOptions,
    ) -> Result<Self> {
        check_options(&opts)?;
        let fixed = fixed_headers(target)?;
        let buf = vec![1u8; opts.buf_size].into_boxed_slice();
        let stream = TlsStream::new(sock, buf, 0, 0, tls, send_queue());
        Ok(Self::build(stream, fixed, opts))
    }

    /// Test lever: `cipher` arrives as if the kernel wrote it, then every
    /// complete record is opened and every complete response delivered.
    ///
    /// # Errors
    /// `cipher` does not fit the free buffer, or as a read would fail.
    #[doc(hidden)]
    pub fn feed<H: HttpHandler>(
        &mut self,
        id: ConnId,
        cipher: &[u8],
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        let space = self.stream.rx_space();
        if cipher.len() > space.len() {
            return Err(Error::Config("feed larger than the free buffer".into()));
        }
        space[..cipher.len()].copy_from_slice(cipher);
        self.deliver(id, cipher.len(), meta, handler)
    }

    /// Bench lever: queues one unencrypted stand-in record of `plain` (at
    /// most 16 KiB) for [`HttpConn::deliver_staged`].
    ///
    /// # Errors
    /// `plain` is too large or does not fit.
    #[doc(hidden)]
    pub fn stage_plain_record(&mut self, plain: &[u8]) -> Result<()> {
        self.stream.stage_plain_record(plain)
    }

    /// Bench lever: [`HttpConn::feed`]'s path minus the kernel copy and the
    /// AEAD: opens each staged stand-in record and parses, then compacts.
    ///
    /// # Errors
    /// As a read would fail.
    #[doc(hidden)]
    pub fn deliver_staged<H: HttpHandler>(
        &mut self,
        id: ConnId,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        let mut delivered = 0;
        while self.stream.open_next_plain()? {
            delivered += self.parse(id, meta, handler)?;
        }
        self.stream.compact();
        Ok(delivered)
    }

    /// Test lever: the encrypted requests queued to send.
    #[doc(hidden)]
    #[must_use]
    pub fn queued_tx(&self) -> &[u8] {
        self.stream.queued_tx()
    }

    /// Test lever: drops the queued requests, as a send would.
    #[doc(hidden)]
    pub fn clear_tx(&mut self) {
        self.stream.tx_queue().clear();
    }

    /// Test lever: the send queue's allocation, as (address, capacity).
    #[doc(hidden)]
    #[must_use]
    pub fn tx_allocation(&mut self) -> (usize, usize) {
        let q = self.stream.tx_queue();
        (q.as_ptr().addr(), q.capacity())
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
        let limit = self.opts.buf_size - 2 * MAX_RECORD;
        let mut delivered = 0;
        loop {
            if self.stream.window().is_empty() {
                return Ok(delivered);
            }
            let Some(req) = self.in_flight.front() else {
                return Err(Error::Http("response without a request".into()));
            };
            let at = At {
                conn: id,
                request: req.id,
                head_only: req.head,
                check: self.opts.body_check,
                limit,
                meta,
            };
            let step = match self.pending {
                Pending::Chunked => self.stream_chunks(handler, &at)?,
                Pending::Length(total) => self.complete_length(handler, &at, total)?,
                Pending::None => self.start(handler, &at)?,
            };
            match step {
                Step::Wait => return Ok(delivered),
                Step::Again => {}
                Step::Delivered { close } => {
                    self.in_flight.pop_front();
                    self.responses += 1;
                    delivered += 1;
                    if close {
                        return Err(Error::Closed);
                    }
                }
            }
        }
    }

    /// A streamed chunked body: the window's chunk data joins the head in
    /// `body`, and the window is released.
    fn stream_chunks<H: HttpHandler>(&mut self, handler: &mut H, at: &At<'_>) -> Result<Step> {
        let window = self.stream.window();
        let before = self.body.len();
        let used = self.dechunk.run(window, Some(&mut self.body), at.limit)?;
        self.chunk_copy_bytes += (self.body.len() - before) as u64;
        self.stream.consume(used);
        self.wire += used;
        if self.wire > at.limit {
            return Err(Error::Http("response larger than the buffer".into()));
        }
        if !self.dechunk.done() {
            return Ok(Step::Wait);
        }
        self.pending = Pending::None;
        let (head, body) = self.body.split_at(self.head.len);
        let mut hdrs = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let headers = self.head.headers(head, &mut hdrs)?;
        emit(handler, at, &self.head.response(headers, body))?;
        Ok(Step::Delivered {
            close: self.head.close,
        })
    }

    /// A `Content-Length` response whose head was parsed earlier: done once
    /// the window holds `total` bytes.
    fn complete_length<H: HttpHandler>(
        &mut self,
        handler: &mut H,
        at: &At<'_>,
        total: usize,
    ) -> Result<Step> {
        let window = self.stream.window();
        if window.len() < total {
            return Ok(Step::Wait);
        }
        self.pending = Pending::None;
        let hl = self.head.len;
        let mut hdrs = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let headers = self.head.headers(&window[..hl], &mut hdrs)?;
        emit(
            handler,
            at,
            &self.head.response(headers, &window[hl..total]),
        )?;
        self.stream.consume(total);
        Ok(Step::Delivered {
            close: self.head.close,
        })
    }

    /// Parses the front response's head (once) and delivers the response if
    /// its body is in the window.
    fn start<H: HttpHandler>(&mut self, handler: &mut H, at: &At<'_>) -> Result<Step> {
        let window = self.stream.window();
        let mut hdrs = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut resp = httparse::Response::new(&mut hdrs);
        self.head_parses += 1;
        let hl = match resp.parse(window).map_err(|e| Error::Http(e.to_string()))? {
            httparse::Status::Partial => return Ok(Step::Wait),
            httparse::Status::Complete(n) => n,
        };
        let status = resp.code.unwrap_or(0);
        if (100..200).contains(&status) {
            // Interim response (100 Continue): skip it.
            self.stream.consume(hl);
            return Ok(Step::Again);
        }
        // Status line and headers are ASCII by spec; httparse lets obs-text
        // (0x80-0xFF) through.
        let ascii = all_ascii(&window[..hl]);
        if !ascii {
            if self.opts.non_ascii_head == NonAsciiHead::Reject {
                return Err(Error::Http("non-ASCII byte in the response head".into()));
            }
            self.non_ascii_heads += 1;
        }
        let framing = framing(resp.headers, status, at.head_only)?;
        let close = closes(resp.version, resp.headers);
        let response = |body| Response {
            status,
            headers: resp.headers,
            body,
            head_ascii: ascii,
        };
        let total = match framing {
            Framing::Length(len) => {
                let total = hl
                    .checked_add(len)
                    .filter(|&t| t <= at.limit)
                    .ok_or_else(|| Error::Http("response larger than the buffer".into()))?;
                if window.len() < total {
                    // Parsed once: the body's arrival only checks the
                    // window's length.
                    self.head
                        .record(window, hl, status, close, ascii, resp.headers)?;
                    self.pending = Pending::Length(total);
                    return Ok(Step::Wait);
                }
                emit(handler, at, &response(&window[hl..total]))?;
                total
            }
            Framing::Chunked => {
                let mut scan = Dechunk::new();
                let used = scan.run(&window[hl..], None, at.limit)?;
                if !scan.done() {
                    // The body is still arriving: keep the head and stream
                    // each chunk out as it comes, so the window empties and
                    // later records are read in place.
                    self.head
                        .record(window, hl, status, close, ascii, resp.headers)?;
                    reserve_body(&mut self.body, at.limit);
                    self.body.clear();
                    self.body.extend_from_slice(&window[..hl]);
                    self.chunk_copy_bytes += hl as u64;
                    self.stream.consume(hl);
                    self.wire = hl;
                    self.dechunk = Dechunk::new();
                    self.pending = Pending::Chunked;
                    return Ok(Step::Again);
                }
                if scan.chunks <= 1 {
                    // One chunk, whole in the window: borrowed.
                    let from = hl + scan.first;
                    emit(handler, at, &response(&window[from..from + scan.body]))?;
                } else {
                    if self.body.capacity() < scan.body {
                        reserve_body(&mut self.body, at.limit);
                    }
                    self.body.clear();
                    Dechunk::new().run(&window[hl..], Some(&mut self.body), at.limit)?;
                    self.chunk_copy_bytes += self.body.len() as u64;
                    emit(handler, at, &response(&self.body))?;
                }
                hl + used
            }
        };
        self.stream.consume(total);
        Ok(Step::Delivered { close })
    }
}

/// What one parse step did.
enum Step {
    /// The front response needs more bytes.
    Wait,
    /// Progress without a delivery (an interim response, a streamed head).
    Again,
    /// The front response was delivered; `close` ends the connection.
    Delivered { close: bool },
}

/// The front request and how its response is checked.
struct At<'m> {
    conn: ConnId,
    request: u64,
    head_only: bool,
    check: TextCheck,
    limit: usize,
    meta: &'m RxMeta,
}

/// One allocation of the largest body, so de-chunking never reallocates (a
/// reallocation would move the body's bytes again).
#[cold]
fn reserve_body(body: &mut Vec<u8>, limit: usize) {
    if body.capacity() < limit {
        *body = Vec::with_capacity(limit);
    }
}

/// Checks `response`'s body against the body check, then hands it over.
#[inline]
fn emit<H: HttpHandler>(handler: &mut H, at: &At<'_>, response: &Response<'_>) -> Result<()> {
    match at.check {
        TextCheck::None => {}
        TextCheck::Ascii => {
            if !response.body.is_ascii() {
                return Err(Error::Http("non-ASCII byte in the response body".into()));
            }
        }
        TextCheck::Utf8 => {
            if std::str::from_utf8(response.body).is_err() {
                return Err(Error::Http("response body is not UTF-8".into()));
            }
        }
    }
    handler.on_response(at.conn, at.request, response, at.meta);
    Ok(())
}

/// The front response's progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    /// No head parsed yet.
    None,
    /// `Content-Length`: the head is parsed; complete at this window length.
    Length(usize),
    /// Chunked: the head is copied to `body`, chunks stream in after it.
    Chunked,
}

/// One header's place in its head: offsets from the head's first byte.
#[derive(Debug, Clone, Copy, Default)]
struct HeaderAt {
    name: u32,
    name_len: u32,
    value: u32,
    value_len: u32,
}

/// A parsed head, kept as offsets so a response that completes in a later
/// record is not parsed again. Offsets survive the window moving
/// (compaction, a streamed head copied out).
struct Head {
    len: usize,
    status: u16,
    close: bool,
    ascii: bool,
    count: usize,
    at: [HeaderAt; MAX_HEADERS],
}

impl Head {
    const EMPTY: Self = Self {
        len: 0,
        status: 0,
        close: false,
        ascii: true,
        count: 0,
        at: [HeaderAt {
            name: 0,
            name_len: 0,
            value: 0,
            value_len: 0,
        }; MAX_HEADERS],
    };

    /// Records a head that `httparse` parsed out of `base` (`len` bytes).
    fn record(
        &mut self,
        base: &[u8],
        len: usize,
        status: u16,
        close: bool,
        ascii: bool,
        headers: &[httparse::Header<'_>],
    ) -> Result<()> {
        let b = base.as_ptr().addr();
        let off = |p: *const u8, n: usize| -> Result<(u32, u32)> {
            p.addr()
                .checked_sub(b)
                .filter(|&o| o + n <= len)
                .and_then(|o| Some((u32::try_from(o).ok()?, u32::try_from(n).ok()?)))
                .ok_or_else(|| Error::Http("header outside the response head".into()))
        };
        for (slot, h) in self.at.iter_mut().zip(headers) {
            let (name, name_len) = off(h.name.as_ptr(), h.name.len())?;
            let (value, value_len) = off(h.value.as_ptr(), h.value.len())?;
            *slot = HeaderAt {
                name,
                name_len,
                value,
                value_len,
            };
        }
        self.count = headers.len().min(MAX_HEADERS);
        self.len = len;
        self.status = status;
        self.close = close;
        self.ascii = ascii;
        Ok(())
    }

    /// The recorded headers over `head`, the head's bytes wherever they are
    /// now.
    fn headers<'a, 'h>(
        &self,
        head: &'a [u8],
        out: &'h mut [httparse::Header<'a>; MAX_HEADERS],
    ) -> Result<&'h [httparse::Header<'a>]> {
        let bad = || Error::Http("header outside the response head".into());
        let span = |at: u32, n: u32| -> Result<&'a [u8]> {
            let at = at as usize;
            head.get(at..at + n as usize).ok_or_else(bad)
        };
        for (h, at) in out.iter_mut().zip(&self.at[..self.count]) {
            // Names are tokens (ASCII): httparse checked them.
            h.name = std::str::from_utf8(span(at.name, at.name_len)?).map_err(|_| bad())?;
            h.value = span(at.value, at.value_len)?;
        }
        Ok(&out[..self.count])
    }

    const fn response<'a>(
        &self,
        headers: &'a [httparse::Header<'a>],
        body: &'a [u8],
    ) -> Response<'a> {
        Response {
            status: self.status,
            headers,
            body,
            head_ascii: self.ascii,
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

/// Where a chunked body's framing has got to. The size line follows
/// `httparse::parse_chunk_size` byte for byte, so a body cut anywhere is
/// read as if it had arrived whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chunk {
    /// A size line: the value and digits so far, whether digits may still
    /// come, whether an extension has started, and a CR seen.
    Size {
        size: u64,
        digits: u8,
        in_size: bool,
        in_ext: bool,
        cr: bool,
    },
    /// Chunk data still to come.
    Data(usize),
    /// The CR, then the LF, after chunk data.
    DataCr,
    DataLf,
    /// After the last chunk: how much of CRLF CRLF has matched. It starts
    /// at 2 (the size line's CRLF), so an empty trailer section ends at the
    /// next CRLF.
    Trailer(u8),
    Done,
}

const SIZE_LINE: Chunk = Chunk::Size {
    size: 0,
    digits: 0,
    in_size: true,
    in_ext: false,
    cr: false,
};

/// A chunked body decoder that can stop at any byte and resume.
#[derive(Debug, Clone, Copy)]
struct Dechunk {
    state: Chunk,
    /// Data bytes seen.
    body: usize,
    /// Chunks with data seen.
    chunks: usize,
    /// Offset of the first chunk's data in the input of the first `run`.
    first: usize,
}

impl Dechunk {
    const fn new() -> Self {
        Self {
            state: SIZE_LINE,
            body: 0,
            chunks: 0,
            first: 0,
        }
    }

    const fn done(&self) -> bool {
        matches!(self.state, Chunk::Done)
    }

    /// Reads `input` until the body ends or the input does; chunk data is
    /// appended to `out` (or only counted). Returns the bytes used: the
    /// caller drops them, so each data byte is moved at most once.
    #[inline]
    fn run(&mut self, input: &[u8], mut out: Option<&mut Vec<u8>>, limit: usize) -> Result<usize> {
        let mut pos = 0;
        while pos < input.len() {
            match self.state {
                Chunk::Data(rem) => {
                    let k = rem.min(input.len() - pos);
                    if let Some(o) = out.as_deref_mut() {
                        o.extend_from_slice(&input[pos..pos + k]);
                    }
                    pos += k;
                    self.body += k;
                    self.state = if k == rem {
                        Chunk::DataCr
                    } else {
                        Chunk::Data(rem - k)
                    };
                }
                Chunk::Done => break,
                // Whole size lines, data CRLFs and an empty trailer section
                // are read in one step; the byte machine below takes a cut
                // one.
                SIZE_LINE => {
                    match httparse::parse_chunk_size(&input[pos..])
                        .map_err(|_| Error::Http("bad chunk size".into()))?
                    {
                        httparse::Status::Complete((n, size)) => {
                            pos += n;
                            self.state = self.size_line(size, limit)?;
                            if self.chunks == 1 && matches!(self.state, Chunk::Data(_)) {
                                self.first = pos;
                            }
                        }
                        httparse::Status::Partial => self.bytes(&input[pos..], &mut pos, limit)?,
                    }
                }
                Chunk::DataCr if input.len() - pos >= 2 => {
                    if &input[pos..pos + 2] != b"\r\n" {
                        return Err(Error::Http("chunk without CRLF".into()));
                    }
                    pos += 2;
                    self.state = SIZE_LINE;
                }
                Chunk::Trailer(2) if input[pos..].starts_with(b"\r\n") => {
                    pos += 2;
                    self.state = Chunk::Done;
                }
                state => {
                    let b = input[pos];
                    pos += 1;
                    self.state = self.byte(state, b, limit)?;
                    if matches!(self.state, Chunk::Data(_)) && self.chunks == 1 {
                        self.first = pos;
                    }
                }
            }
        }
        Ok(pos)
    }

    /// Feeds `rest` (a cut size line) through the byte machine; `pos`
    /// advances past it.
    #[cold]
    fn bytes(&mut self, rest: &[u8], pos: &mut usize, limit: usize) -> Result<()> {
        for &b in rest {
            *pos += 1;
            self.state = self.byte(self.state, b, limit)?;
            if !matches!(self.state, Chunk::Size { .. }) {
                // httparse said partial: the line cannot end in `rest`.
                return Err(Error::Http("bad chunk size".into()));
            }
        }
        Ok(())
    }

    /// One framing byte.
    fn byte(&mut self, state: Chunk, b: u8, limit: usize) -> Result<Chunk> {
        let bad_size = || Error::Http("bad chunk size".into());
        Ok(match state {
            Chunk::Size {
                size,
                digits,
                in_size,
                in_ext,
                cr,
            } => {
                if cr {
                    if b != b'\n' {
                        return Err(bad_size());
                    }
                    return self.size_line(size, limit);
                }
                let mut next = (size, digits, in_size, in_ext, false);
                match b {
                    b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' if in_size => {
                        if digits > 15 {
                            return Err(bad_size());
                        }
                        let v = u64::from(char::from(b).to_digit(16).ok_or_else(bad_size)?);
                        next.0 = size
                            .checked_mul(16)
                            .and_then(|s| s.checked_add(v))
                            .ok_or_else(bad_size)?;
                        next.1 = digits + 1;
                    }
                    b'\r' => next.4 = true,
                    b';' if !in_ext => {
                        next.3 = true;
                        next.2 = false;
                    }
                    b'\t' | b' ' if !in_ext && !in_size => {}
                    b'\t' | b' ' if in_size => next.2 = false,
                    _ if in_ext => {}
                    _ => return Err(bad_size()),
                }
                Chunk::Size {
                    size: next.0,
                    digits: next.1,
                    in_size: next.2,
                    in_ext: next.3,
                    cr: next.4,
                }
            }
            Chunk::DataCr if b == b'\r' => Chunk::DataLf,
            Chunk::DataLf if b == b'\n' => SIZE_LINE,
            Chunk::DataCr | Chunk::DataLf => {
                return Err(Error::Http("chunk without CRLF".into()));
            }
            Chunk::Trailer(k) => {
                let want = if k % 2 == 0 { b'\r' } else { b'\n' };
                if b == want {
                    if k == 3 {
                        Chunk::Done
                    } else {
                        Chunk::Trailer(k + 1)
                    }
                } else if b == b'\r' {
                    Chunk::Trailer(1)
                } else {
                    Chunk::Trailer(0)
                }
            }
            Chunk::Data(_) | Chunk::Done => state,
        })
    }

    /// A complete size line: the last chunk, or data to read.
    fn size_line(&mut self, size: u64, limit: usize) -> Result<Chunk> {
        let size = usize::try_from(size)
            .ok()
            .filter(|&s| s <= limit)
            .ok_or_else(|| Error::Http("chunk too large".into()))?;
        if size == 0 {
            return Ok(Chunk::Trailer(2));
        }
        if self.body + size > limit {
            return Err(Error::Http("response larger than the buffer".into()));
        }
        self.chunks += 1;
        Ok(Chunk::Data(size))
    }
}

/// A 64 KiB send queue with every page written once, so the first requests
/// take no page fault on the hot path (fresh memory is mapped lazily, on
/// first write).
fn send_queue() -> Vec<u8> {
    let mut q = vec![1u8; 1 << 16];
    q.clear();
    q
}

/// Seals one request (`build` writes its plaintext) into one TLS record at
/// the end of the send queue.
#[inline]
fn seal_request(stream: &mut TlsStream, build: impl FnOnce(&mut Vec<u8>)) -> Result<()> {
    stream.seal_build(|out| {
        let start = out.len();
        build(out);
        if out.len() - start > MAX_REQUEST {
            return Err(Error::Http("request larger than one TLS record".into()));
        }
        Ok(())
    })
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
    /// `Host` and the target's headers, serialised once.
    fixed: Vec<u8>,
    opts: HttpOptions,
    tls: TlsDial,
}

impl HttpDial {
    /// A dial to `target`; its headers are sent with every request.
    ///
    /// # Errors
    /// The buffer in `opts` is too small, or a header contains CR/LF.
    pub fn new(target: Target, cfg: Arc<ClientConfig>, opts: HttpOptions) -> Result<Self> {
        check_options(&opts)?;
        let fixed = fixed_headers(&target)?;
        let sni = target.host.clone();
        Ok(Self {
            target,
            fixed,
            opts,
            tls: TlsDial::new(cfg, sni)?,
        })
    }
}

fn check_options(opts: &HttpOptions) -> Result<()> {
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
    Ok(())
}

/// `Host` and the target's headers, serialised once per endpoint.
fn fixed_headers(target: &Target) -> Result<Vec<u8>> {
    let mut f = Vec::new();
    f.extend_from_slice(b"Host: ");
    f.extend_from_slice(target.host_header().as_bytes());
    f.extend_from_slice(b"\r\n");
    for (k, v) in &target.headers {
        validate_header(k, v)?;
        f.extend_from_slice(k.as_bytes());
        f.extend_from_slice(b": ");
        f.extend_from_slice(v.as_bytes());
        f.extend_from_slice(b"\r\n");
    }
    Ok(f)
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
        let mut stream = TlsStream::new(sock, buf, 0, rest.len(), tls, send_queue());
        if let Some(check) = self.tls.take_route() {
            stream.set_route(crate::endpoint::sealed::RouteSlot::Ready(check));
        }
        Ok(Some(HttpConn::build(stream, self.fixed.clone(), self.opts)))
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
    fn tx_queue(&mut self) -> &mut Vec<u8> {
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
