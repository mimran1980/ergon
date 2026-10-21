//! A TLS WebSocket client connection.
//!
//! - [`WsDial`] sets it up without blocking: TCP, TLS, then the upgrade.
//!   [`WsOptions`] holds its limits and timers.
//! - [`WsConn`] parses server frames in place in the receive buffer: a
//!   payload is a slice of the bytes the kernel wrote.
//! - Close handshake both ways ([`WsConn::close`]; a server Close is answered
//!   with its code). On a quiet connection, Pings and a Pong timeout; an idle
//!   timeout. The timers run on the driver's deadline check (every 250 µs),
//!   never on the read or send path.
//! - Not supported: compression (`permessage-deflate`) and other extensions,
//!   proxies, sending fragmented messages. The handshake offers no extension
//!   and refuses a server that picks one.

use std::io::{ErrorKind, Read};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;

use rustls::ClientConfig;

use crate::RxMeta;
use crate::clock::monotonic_ns;
use crate::endpoint::sealed::{Deliver, DialIo, Io};
use crate::endpoint::{ConnId, ConnMut, Handler};
use crate::error::{Error, Result};
pub use crate::net::Target;
use crate::net::{TlsDial, flush_nonblocking};
use crate::stream::TlsStream;
use crate::tls::{self, MAX_RECORD, Opened, RecordLayer};
use crate::ws::{self, Close};

/// Receives decoded WebSocket messages.
pub trait WsHandler: Handler {
    /// One complete data message (`opcode` is [`ws::OP_TEXT`] or
    /// [`ws::OP_BINARY`]). `payload` borrows the receive buffer: it is valid only
    /// during the call. `pos` is its index among the messages decoded from the
    /// same read.
    fn on_message(&mut self, conn: ConnId, opcode: u8, payload: &[u8], meta: &RxMeta, pos: u16);
}

/// Per-connection counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WsStats {
    /// Bytes received from the kernel.
    pub bytes_in: u64,
    /// TLS records opened.
    pub records: u64,
    /// Data messages delivered.
    pub messages: u64,
    /// Plaintext bytes moved because a frame spanned two records.
    pub span_copy_bytes: u64,
    /// Bytes moved by buffer compaction.
    pub compact_bytes: u64,
    /// Bytes copied to reassemble fragmented messages.
    pub frag_copy_bytes: u64,
    /// Server Pings answered.
    pub pings: u64,
    /// Pings sent on a quiet connection ([`WsOptions::ping_interval_ms`]).
    pub pings_sent: u64,
    /// Pongs received.
    pub pongs: u64,
    /// Frames the connection sent by itself: Pongs to the server's Pings,
    /// Pings on a quiet connection, and the echo of the server's Close. A
    /// venue that limits messages per connection may count them (Binance
    /// spot counts Pings and Pongs): charge the change since the last pass
    /// with `Budget::force`.
    pub auto_frames: u64,
}

/// WebSocket settings. `0` turns the ping, pong and idle timers off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WsOptions {
    /// Receive buffer, bytes; at least [`MIN_BUF`]. Every frame must fit.
    pub buf_size: usize,
    /// Largest message, bytes; `0`: the largest the buffer holds. A
    /// fragmented message may be larger than the buffer.
    pub max_message: usize,
    /// Send a Ping after this long with nothing received, ms. A busy
    /// connection sends none.
    pub ping_interval_ms: u32,
    /// Close with [`Error::Timeout`] if nothing arrives this long after a
    /// Ping, ms.
    pub pong_timeout_ms: u32,
    /// Close with [`Error::Timeout`] after this long with nothing received,
    /// ms.
    pub idle_timeout_ms: u32,
    /// Longest wait for the other side of a close handshake, ms; then the
    /// socket is closed anyway. `0`: no wait.
    pub close_timeout_ms: u32,
    /// Check that text messages are UTF-8 (RFC 6455 requires it); invalid
    /// text closes the connection. Off by default: it reads every text byte
    /// once more (`str::from_utf8`).
    pub validate_utf8: bool,
    /// Subprotocols to offer (`Sec-WebSocket-Protocol`), most preferred
    /// first. [`WsConn::protocol`] is the server's pick.
    pub protocols: Vec<String>,
}

impl Default for WsOptions {
    fn default() -> Self {
        Self {
            buf_size: 1 << 20,
            max_message: 0,
            ping_interval_ms: 10_000,
            pong_timeout_ms: 5_000,
            idle_timeout_ms: 0,
            close_timeout_ms: 1_000,
            validate_utf8: false,
            protocols: Vec::new(),
        }
    }
}

impl WsOptions {
    fn check(&self) -> Result<()> {
        if self.buf_size < MIN_BUF {
            return Err(Error::Config(
                "receive buffer must hold at least 8 records".into(),
            ));
        }
        for p in &self.protocols {
            if p.is_empty() || !p.bytes().all(crate::http::is_tchar) {
                return Err(Error::Config(format!("invalid subprotocol {p:?}")));
            }
        }
        Ok(())
    }
}

/// A connected, upgraded TLS WebSocket. Hand it to a `Driver`; the
/// socket closes when it is dropped.
pub struct WsConn {
    stream: TlsStream,
    frag: Vec<u8>,
    /// Opcode of the fragmented message in progress, 0 when none.
    frag_op: u8,
    /// A Close was sent or received: nothing more may be sent.
    closing: bool,
    /// The server's Close arrived: further input is dropped.
    discard: bool,
    validate_utf8: bool,
    /// Largest single frame: the configured message limit, capped by the
    /// buffer.
    frame_limit: usize,
    /// Largest message, reassembled or not.
    max_message: usize,
    rng: u64,
    messages: u64,
    frag_copy_bytes: u64,
    pings: u64,
    pongs: u64,
    /// Frames queued by the connection itself ([`WsStats::auto_frames`]).
    auto_frames: u64,
    timers: Timers,
    protocol: Option<String>,
}

/// The connection's timers, read only by [`Io::expire`].
struct Timers {
    ping_ns: i64,
    pong_ns: i64,
    idle_ns: i64,
    close_ns: i64,
    /// When bytes last arrived; 0 until the first deadline check.
    last_rx_ns: i64,
    /// `bytes_in` at the last deadline check.
    seen_bytes: u64,
    /// No Ping before this.
    next_ping_ns: i64,
    /// When the Pong is due (0: none awaited), and `bytes_in` when the Ping
    /// went: any byte after it answers.
    pong_due_ns: i64,
    bytes_at_ping: u64,
    pings_sent: u64,
    /// When the close handshake gives up.
    close_due_ns: i64,
    /// The server's Close, reported when the connection ends.
    received: Option<Box<Close>>,
}

const MS: i64 = 1_000_000;

impl Timers {
    fn new(o: &WsOptions) -> Self {
        Self {
            ping_ns: i64::from(o.ping_interval_ms) * MS,
            pong_ns: i64::from(o.pong_timeout_ms) * MS,
            idle_ns: i64::from(o.idle_timeout_ms) * MS,
            close_ns: i64::from(o.close_timeout_ms) * MS,
            last_rx_ns: 0,
            seen_bytes: 0,
            next_ping_ns: 0,
            pong_due_ns: 0,
            bytes_at_ping: 0,
            pings_sent: 0,
            close_due_ns: 0,
            received: None,
        }
    }
}

/// Smallest receive buffer [`WsConn::connect`] accepts.
pub const MIN_BUF: usize = 8 * MAX_RECORD;

/// Largest payload one record carries with its frame header (8 bytes up to
/// 64 KiB); larger frames are spread over several records.
const ONE_RECORD: usize = (1 << 14) - 8;

impl WsConn {
    /// [`WsConn::connect_with`] with default options and this receive buffer.
    ///
    /// # Errors
    /// As [`WsConn::connect_with`].
    pub fn connect(target: &Target, cfg: Arc<ClientConfig>, buf_size: usize) -> Result<Self> {
        Self::connect_with(
            target,
            cfg,
            WsOptions {
                buf_size,
                ..WsOptions::default()
            },
        )
    }

    /// Connects, runs the TLS handshake and the WebSocket upgrade, waiting until
    /// done: a convenience for tools and tests. In a busy-spin loop use
    /// [`WsDial`] with `Driver::connect`, which never waits. Without
    /// [`Target::addr`] the host is resolved with the system resolver (blocking).
    ///
    /// # Errors
    /// Bad options, resolution, connect, TLS or upgrade failure
    /// ([`Error::Upgrade`] for a refusal), or no progress for 30 s.
    pub fn connect_with(target: &Target, cfg: Arc<ClientConfig>, opts: WsOptions) -> Result<Self> {
        let addr = match target.addr {
            Some(a) => a,
            None => (target.host.as_str(), target.port)
                .to_socket_addrs()?
                .find(SocketAddr::is_ipv4)
                .ok_or_else(|| Error::Dns(format!("{}: no IPv4 address", target.host)))?,
        };
        let mut dial = WsDial::with_options(target.clone(), cfg, opts)?;
        dial.start(addr)?;
        let start = std::time::Instant::now();
        loop {
            if let Some(c) = dial.step()? {
                return Ok(c);
            }
            if start.elapsed() > std::time::Duration::from_secs(30) {
                return Err(Error::Timeout("websocket connect"));
            }
            std::thread::yield_now();
        }
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> WsStats {
        let s = self.stream.stats();
        WsStats {
            bytes_in: s.bytes_in,
            records: s.records,
            messages: self.messages,
            span_copy_bytes: s.span_copy_bytes,
            compact_bytes: s.compact_bytes,
            frag_copy_bytes: self.frag_copy_bytes,
            pings: self.pings,
            pings_sent: self.timers.pings_sent,
            pongs: self.pongs,
            auto_frames: self.auto_frames,
        }
    }

    /// The subprotocol the server picked ([`WsOptions::protocols`]).
    #[must_use]
    pub fn protocol(&self) -> Option<&str> {
        self.protocol.as_deref()
    }

    /// Whether a Close was sent or received: sends now fail.
    #[must_use]
    pub const fn is_closing(&self) -> bool {
        self.closing
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

    /// Accounts for `n` received bytes, decrypts every complete record and
    /// delivers every complete message. `n == 0` parses what is already buffered
    /// (frames that arrived with the upgrade).
    #[inline]
    fn deliver<H: WsHandler>(
        &mut self,
        id: ConnId,
        n: usize,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        self.stream.received(n);
        let mut delivered = 0usize;
        if n == 0 {
            delivered += self.parse_frames(id, meta, handler, 0)?;
        }
        while self.stream.open_next()? {
            delivered += self.parse_frames(id, meta, handler, delivered)?;
        }
        self.stream.compact();
        Ok(delivered)
    }

    #[inline]
    fn parse_frames<H: WsHandler>(
        &mut self,
        id: ConnId,
        meta: &RxMeta,
        handler: &mut H,
        already: usize,
    ) -> Result<usize> {
        if self.discard {
            self.drop_window();
            return Ok(0);
        }
        let mut pos = already;
        let mut count = 0;
        loop {
            let window = self.stream.window();
            let Some(h) = ws::parse_header(window)? else {
                break;
            };
            if h.mask.is_some() {
                return Err(Error::Ws("server frame is masked"));
            }
            // Checked before waiting for the frame: one that can never fit
            // fails instead of stalling.
            if h.payload_len > self.frame_limit {
                return Err(self.too_large(h.payload_len));
            }
            if window.len() < h.frame_len() {
                break;
            }
            let payload = &window[h.header_len..h.frame_len()];
            match h.opcode {
                // Inside a fragmented message the arm below refuses it.
                ws::OP_TEXT | ws::OP_BINARY if h.fin && self.frag_op == 0 => {
                    if self.validate_utf8 && h.opcode == ws::OP_TEXT {
                        check_utf8(payload)?;
                    }
                    handler.on_message(id, h.opcode, payload, meta, pos16(pos));
                    pos += 1;
                    count += 1;
                    self.messages += 1;
                }
                ws::OP_TEXT | ws::OP_BINARY => {
                    if self.frag_op != 0 {
                        return Err(Error::Ws("new message inside a fragmented one"));
                    }
                    self.frag.clear();
                    self.frag.extend_from_slice(payload);
                    self.frag_op = h.opcode;
                    self.frag_copy_bytes += payload.len() as u64;
                }
                ws::OP_CONT => {
                    if self.frag_op == 0 {
                        return Err(Error::Ws("continuation without a first fragment"));
                    }
                    if self.frag.len() + payload.len() > self.max_message {
                        return Err(Error::Ws("message too large"));
                    }
                    self.frag.extend_from_slice(payload);
                    self.frag_copy_bytes += payload.len() as u64;
                    if h.fin {
                        if self.validate_utf8 && self.frag_op == ws::OP_TEXT {
                            check_utf8(&self.frag)?;
                        }
                        handler.on_message(id, self.frag_op, &self.frag, meta, pos16(pos));
                        self.frag_op = 0;
                        pos += 1;
                        count += 1;
                        self.messages += 1;
                    }
                }
                ws::OP_PING => {
                    let mut body = [0u8; ws::MAX_CONTROL];
                    let len = payload.len();
                    body[..len].copy_from_slice(payload);
                    self.stream.consume(h.frame_len());
                    self.pings += 1;
                    // Nothing may follow our Close.
                    if !self.closing {
                        self.queue(ws::OP_PONG, &body[..len])?;
                        self.auto_frames += 1;
                    }
                    continue;
                }
                ws::OP_PONG => self.pongs += 1,
                ws::OP_CLOSE => {
                    let mut body = [0u8; ws::MAX_CONTROL];
                    let len = payload.len();
                    body[..len].copy_from_slice(payload);
                    self.stream.consume(h.frame_len());
                    self.on_close(&body[..len])?;
                    return Ok(count);
                }
                _ => return Err(Error::Ws("unexpected opcode")),
            }
            self.stream.consume(h.frame_len());
        }
        Ok(count)
    }

    /// Drops the plaintext not yet parsed (input after the server's Close).
    #[cold]
    fn drop_window(&mut self) {
        let n = self.stream.window().len();
        self.stream.consume(n);
    }

    #[cold]
    const fn too_large(&self, len: usize) -> Error {
        if len > self.max_message {
            Error::Ws("message too large")
        } else {
            Error::Ws("frame larger than the receive buffer")
        }
    }

    /// The server's Close. Answering ours: the handshake is done, report it
    /// (the driver closes the socket). Its own: echo the code, drop further
    /// input and wait for the server to close TCP (or `close_timeout_ms`).
    #[cold]
    fn on_close(&mut self, body: &[u8]) -> Result<()> {
        let (code, reason) = ws::parse_close(body)?;
        let close = Box::new(Close {
            code,
            reason: reason.to_owned(),
            by_server: !self.closing,
        });
        if self.closing {
            return Err(Error::WsClose(close));
        }
        let (reply, n) = ws::close_payload(code, "")?;
        self.queue(ws::OP_CLOSE, &reply[..n])?;
        self.auto_frames += 1;
        self.closing = true;
        self.discard = true;
        self.timers.close_due_ns = monotonic_ns() + self.timers.close_ns;
        self.timers.received = Some(close);
        self.drop_window();
        Ok(())
    }

    /// Queues one masked client frame, encrypted, for sending on the driver's
    /// next `poll` or `flush`. The payload is written once, straight into the
    /// record being built; a payload larger than one TLS record is spread
    /// over several.
    ///
    /// # Errors
    /// An opcode other than text, binary, ping or pong, a control payload
    /// over 125 bytes, or a connection that is closing. Close with
    /// [`WsConn::close`].
    #[inline]
    pub fn send_frame(&mut self, opcode: u8, payload: &[u8]) -> Result<()> {
        match opcode {
            ws::OP_TEXT | ws::OP_BINARY => {}
            ws::OP_PING | ws::OP_PONG if payload.len() <= ws::MAX_CONTROL => {}
            _ => return Err(bad_send()),
        }
        if self.closing {
            return Err(Error::Ws("the connection is closing"));
        }
        self.queue(opcode, payload)
    }

    /// Sends a text message.
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    #[inline]
    pub fn send_text(&mut self, text: &str) -> Result<()> {
        self.send_frame(ws::OP_TEXT, text.as_bytes())
    }

    /// Sends a binary message.
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    #[inline]
    pub fn send_binary(&mut self, data: &[u8]) -> Result<()> {
        self.send_frame(ws::OP_BINARY, data)
    }

    /// Sends a Ping (at most 125 bytes). The connection pings by itself when
    /// quiet ([`WsOptions::ping_interval_ms`]).
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    pub fn ping(&mut self, payload: &[u8]) -> Result<()> {
        self.send_frame(ws::OP_PING, payload)
    }

    /// Starts the close handshake: sends a Close with `code` and `reason`,
    /// then waits up to [`WsOptions::close_timeout_ms`] for the server's
    /// Close before the socket closes. `Handler::on_closed` then gets
    /// [`Error::WsClose`] (the server's answer), or [`Error::Timeout`].
    /// `Driver::close` instead closes the socket at once, with no Close.
    ///
    /// # Errors
    /// A code [`ws::close_code_ok`] refuses, a reason over 123 bytes, or a
    /// close already under way.
    pub fn close(&mut self, code: u16, reason: &str) -> Result<()> {
        if self.closing {
            return Err(Error::Ws("the connection is already closing"));
        }
        let (body, n) = ws::close_payload(Some(code), reason)?;
        self.queue(ws::OP_CLOSE, &body[..n])?;
        self.closing = true;
        self.timers.close_due_ns = monotonic_ns() + self.timers.close_ns;
        Ok(())
    }

    /// Queues a frame without the checks of [`WsConn::send_frame`].
    #[inline]
    fn queue(&mut self, opcode: u8, payload: &[u8]) -> Result<()> {
        if payload.len() > ONE_RECORD {
            return self.queue_large(opcode, payload);
        }
        let mask = self.next_mask();
        self.stream.seal_build(|out| {
            let start = out.len();
            let hl = ws::header_len(payload.len(), true);
            out.resize(start + hl + payload.len(), 0);
            ws::encode_frame(opcode, payload, Some(mask), &mut out[start..])?;
            Ok(())
        })
    }

    /// A frame over several records: the header and the first part in one,
    /// then up to 16 KiB per record, masked where it lands.
    #[cold]
    fn queue_large(&mut self, opcode: u8, payload: &[u8]) -> Result<()> {
        let mask = self.next_mask();
        let hl = ws::header_len(payload.len(), true);
        let (first, mut rest) = payload.split_at(((1 << 14) - hl).min(payload.len()));
        self.stream.seal_build(|out| {
            let start = out.len();
            out.resize(start + hl, 0);
            ws::encode_header(opcode, payload.len(), Some(mask), &mut out[start..])?;
            let body = out.len();
            out.extend_from_slice(first);
            ws::apply_mask(&mut out[body..], mask);
            Ok(())
        })?;
        let mut offset = first.len();
        while !rest.is_empty() {
            let (part, tail) = rest.split_at(rest.len().min(1 << 14));
            self.stream.seal_build(|out| {
                let start = out.len();
                out.extend_from_slice(part);
                ws::apply_mask_at(&mut out[start..], mask, offset);
                Ok(())
            })?;
            offset += part.len();
            rest = tail;
        }
        Ok(())
    }

    #[inline]
    const fn next_mask(&mut self) -> [u8; 4] {
        let m = xorshift(&mut self.rng).to_ne_bytes();
        [m[0], m[1], m[2], m[3]]
    }

    /// Bytes still waiting in the receive buffer (diagnostics).
    #[must_use]
    pub const fn buffered(&self) -> usize {
        self.stream.buffered()
    }

    /// The deadline check: the close handshake's timeout, the idle and Pong
    /// timeouts, and a Ping when the connection has been quiet.
    fn timers(&mut self, now: i64) -> Result<()> {
        let bytes = self.stream.stats().bytes_in;
        let t = &mut self.timers;
        if t.last_rx_ns == 0 || bytes != t.seen_bytes {
            t.seen_bytes = bytes;
            t.last_rx_ns = now;
        }
        if self.closing {
            if now >= t.close_due_ns {
                return Err(t
                    .received
                    .take()
                    .map_or(Error::Timeout("websocket close"), Error::WsClose));
            }
            return Ok(());
        }
        let quiet = now - t.last_rx_ns;
        if t.idle_ns > 0 && quiet >= t.idle_ns {
            return Err(Error::Timeout("websocket idle"));
        }
        if t.pong_due_ns != 0 {
            if bytes != t.bytes_at_ping {
                t.pong_due_ns = 0;
            } else if now >= t.pong_due_ns {
                return Err(Error::Timeout("websocket pong"));
            }
        }
        if t.ping_ns > 0 && t.pong_due_ns == 0 && quiet >= t.ping_ns && now >= t.next_ping_ns {
            t.pings_sent += 1;
            t.next_ping_ns = now + t.ping_ns;
            if t.pong_ns > 0 {
                t.pong_due_ns = now + t.pong_ns;
                t.bytes_at_ping = bytes;
            }
            let n = t.pings_sent.to_be_bytes();
            self.queue(ws::OP_PING, &n)?;
            self.auto_frames += 1;
        }
        Ok(())
    }
}

#[cold]
const fn bad_send() -> Error {
    Error::Ws("send_frame sends text, binary, or ping and pong up to 125 bytes; close with close()")
}

#[inline]
fn check_utf8(text: &[u8]) -> Result<()> {
    std::str::from_utf8(text)
        .map(|_| ())
        .map_err(|_| Error::Ws("text message is not UTF-8"))
}

/// A driver-owned [`WsConn`]: the same send and close methods.
impl ConnMut<'_, WsConn> {
    /// [`WsConn::send_frame`].
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    #[inline]
    pub fn send_frame(&mut self, opcode: u8, payload: &[u8]) -> Result<()> {
        self.inner().send_frame(opcode, payload)
    }

    /// [`WsConn::send_text`].
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    #[inline]
    pub fn send_text(&mut self, text: &str) -> Result<()> {
        self.inner().send_text(text)
    }

    /// [`WsConn::send_binary`].
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    #[inline]
    pub fn send_binary(&mut self, data: &[u8]) -> Result<()> {
        self.inner().send_binary(data)
    }

    /// [`WsConn::ping`].
    ///
    /// # Errors
    /// As [`WsConn::send_frame`].
    pub fn ping(&mut self, payload: &[u8]) -> Result<()> {
        self.inner().ping(payload)
    }

    /// [`WsConn::close`].
    ///
    /// # Errors
    /// As [`WsConn::close`].
    pub fn close(&mut self, code: u16, reason: &str) -> Result<()> {
        self.inner().close(code, reason)
    }
}

/// A WebSocket connection being set up without blocking: TCP connect, TLS
/// handshake and upgrade, each advanced by the driver's `poll`. Hand it to
/// `Driver::connect`.
pub struct WsDial {
    target: Target,
    opts: WsOptions,
    tls: TlsDial,
    up: Option<Box<Upgrade>>,
}

impl WsDial {
    /// [`WsDial::with_options`] with default options and this receive buffer.
    ///
    /// # Errors
    /// As [`WsDial::with_options`].
    pub fn new(target: Target, cfg: Arc<ClientConfig>, buf_size: usize) -> Result<Self> {
        Self::with_options(
            target,
            cfg,
            WsOptions {
                buf_size,
                ..WsOptions::default()
            },
        )
    }

    /// A dial to `target`.
    ///
    /// # Errors
    /// A buffer under [`MIN_BUF`], a bad subprotocol name, or a path or
    /// header the upgrade request cannot carry (CR/LF, a bad name, or a
    /// field the handshake sets itself).
    pub fn with_options(target: Target, cfg: Arc<ClientConfig>, opts: WsOptions) -> Result<Self> {
        opts.check()?;
        // The path and headers go into the upgrade request verbatim: no CR/LF,
        // valid tokens, and none of the fields the handshake sets itself.
        let headers: Vec<(&str, &str)> = target
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        crate::http::validate_request("GET", &target.path, &headers)?;
        if let Some((k, _)) = headers
            .iter()
            .find(|(k, _)| k.len() >= 14 && k[..14].eq_ignore_ascii_case("sec-websocket-"))
        {
            return Err(Error::Http(format!(
                "header {k} is set by the WebSocket handshake"
            )));
        }
        let sni = target.host.clone();
        Ok(Self {
            target,
            opts,
            tls: TlsDial::new(cfg, sni)?,
            up: None,
        })
    }
}

impl DialIo for WsDial {
    type Conn = WsConn;

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

    fn step(&mut self) -> Result<Option<WsConn>> {
        if self.up.is_none() {
            let Some((sock, tls, rest)) = self.tls.step()? else {
                return Ok(None);
            };
            self.up = Some(Box::new(Upgrade::new(
                &self.target,
                &self.opts,
                sock,
                tls,
                &rest,
            )?));
        }
        let Some(up) = self.up.as_mut() else {
            return Ok(None);
        };
        if !up.step(&self.opts.protocols)? {
            return Ok(None);
        }
        let Some(mut conn) = self.up.take().map(|u| u.finish(&self.opts)).transpose()? else {
            return Ok(None);
        };
        if let Some(check) = self.tls.take_route() {
            conn.stream
                .set_route(crate::endpoint::sealed::RouteSlot::Ready(check));
        }
        Ok(Some(conn))
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

/// The upgrade request in flight and the response being read.
struct Upgrade {
    sock: TcpStream,
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
    tls: RecordLayer,
    txq: Vec<u8>,
    http: Vec<u8>,
    key: String,
    rng: u64,
    upgraded: Option<ws::Upgraded>,
}

impl Upgrade {
    fn new(
        target: &Target,
        opts: &WsOptions,
        sock: TcpStream,
        mut tls: RecordLayer,
        rest: &[u8],
    ) -> Result<Self> {
        let mut seed = [0u8; 8];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut seed)
            .map_err(|_| Error::Config("no system randomness".into()))?;
        let mut rng = u64::from_le_bytes(seed) | 1;
        let mut nonce = [0u8; 16];
        nonce[..8].copy_from_slice(&xorshift(&mut rng).to_le_bytes());
        nonce[8..].copy_from_slice(&xorshift(&mut rng).to_le_bytes());
        let key = ws::client_key(nonce);
        let offered = opts.protocols.join(", ");
        let mut headers: Vec<(&str, &str)> = target
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        if !offered.is_empty() {
            headers.push(("Sec-WebSocket-Protocol", &offered));
        }
        let req = ws::client_request(&target.host_header(), &target.path, &key, &headers);
        let mut txq = Vec::with_capacity(1 << 16);
        tls.seal_app(&req, &mut txq)?;
        // Non-zero fill writes every page now (zeroed memory is mapped lazily),
        // so the first receives do not take page faults.
        let mut buf = vec![1u8; opts.buf_size].into_boxed_slice();
        if rest.len() > buf.len() {
            return Err(Error::Record("handshake leftover larger than the buffer"));
        }
        buf[..rest.len()].copy_from_slice(rest);
        Ok(Self {
            sock,
            buf,
            head: 0,
            tail: rest.len(),
            tls,
            txq,
            http: Vec::new(),
            key,
            rng,
            upgraded: None,
        })
    }

    /// `true` once the request is sent and the 101 response parsed.
    fn step(&mut self, protocols: &[String]) -> Result<bool> {
        loop {
            let sent = flush_nonblocking(&mut self.sock, &mut self.txq)?;
            if self.upgraded.is_none() {
                while let Some(len) =
                    RecordLayer::complete_record_len(&self.buf[self.head..self.tail])?
                {
                    let rec = &mut self.buf[self.head..self.head + len];
                    if let Opened::AppData(n) = self.tls.open_in_place(rec, &mut self.txq)? {
                        self.http
                            .extend_from_slice(&rec[tls::HEADER_LEN..tls::HEADER_LEN + n]);
                    }
                    self.head += len;
                }
                self.upgraded = ws::parse_upgrade(&self.http, &self.key, protocols)?;
            }
            if self.upgraded.is_some() {
                // Control replies (key updates) queued while reading must go out.
                if sent && self.txq.is_empty() {
                    return Ok(true);
                }
                if !sent {
                    return Ok(false);
                }
                continue;
            }
            if self.tail == self.buf.len() {
                return Err(Error::Http("upgrade response too large".into()));
            }
            match self.sock.read(&mut self.buf[self.tail..]) {
                Ok(0) => return Err(Error::Closed),
                Ok(n) => self.tail += n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(false),
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn finish(self, opts: &WsOptions) -> Result<WsConn> {
        let up = self
            .upgraded
            .ok_or(Error::Http("upgrade not complete".into()))?;
        let Self {
            sock,
            mut buf,
            head,
            tail,
            tls,
            txq,
            http,
            rng,
            ..
        } = self;
        // Put any WebSocket bytes that arrived with the 101 in front of the
        // remaining ciphertext (one-time setup copy).
        let leftover = &http[up.header_len..];
        let ct = tail - head;
        let l = leftover.len();
        if l + ct > buf.len() {
            return Err(Error::Http(
                "upgrade leftover larger than the buffer".into(),
            ));
        }
        buf.copy_within(head..tail, l);
        buf[..l].copy_from_slice(leftover);
        let buf_len = buf.len();
        let max_message = match opts.max_message {
            0 => max_frame(buf_len),
            m => m,
        };
        Ok(WsConn {
            stream: TlsStream::new(sock, buf, l, ct, tls, txq),
            frag: Vec::with_capacity(1 << 20),
            frag_op: 0,
            closing: false,
            discard: false,
            validate_utf8: opts.validate_utf8,
            frame_limit: max_message.min(max_frame(buf_len)),
            max_message,
            rng,
            messages: 0,
            frag_copy_bytes: 0,
            pings: 0,
            pongs: 0,
            auto_frames: 0,
            timers: Timers::new(opts),
            protocol: up.protocol,
        })
    }
}

impl Io for WsConn {
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
        self.timers(now_ns)
    }
    fn close_reason(&mut self, error: Error) -> Error {
        self.timers.received.take().map_or(error, Error::WsClose)
    }
}

impl<H: WsHandler> Deliver<H> for WsConn {
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

/// Frames and reassembled messages must fit the buffer with room for two
/// records of ciphertext behind them.
const fn max_frame(buf_len: usize) -> usize {
    buf_len.saturating_sub(2 * MAX_RECORD)
}

fn pos16(pos: usize) -> u16 {
    u16::try_from(pos).unwrap_or(u16::MAX)
}

const fn xorshift(s: &mut u64) -> u64 {
    let mut x = *s;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *s = x;
    x
}
