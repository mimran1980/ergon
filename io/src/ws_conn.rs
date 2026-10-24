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
///
/// Only [`WsHandler::on_message`] is required. The two other methods let a
/// handler skip messages it does not want and take fragmented messages
/// without a copy:
///
/// | first frame's [`Interest`] | unfragmented message | fragmented message |
/// |---|---|---|
/// | [`Interest::Keep`] (default) | `on_message`, a slice of the receive buffer | `on_message`, reassembled by copy ([`WsStats::frag_copy_bytes`]) |
/// | [`Interest::Fragments`] | `on_message`, a slice of the receive buffer | `on_fragments`, one slice of the receive buffer per fragment |
/// | [`Interest::Skip`] | dropped | every fragment dropped, never copied |
///
/// [`WsOptions::validate_utf8`] checks a text message only once it is kept,
/// just before delivery.
pub trait WsHandler: Handler {
    /// One complete data message (`opcode` is [`ws::OP_TEXT`] or
    /// [`ws::OP_BINARY`]). `payload` borrows the receive buffer: it is valid only
    /// during the call. `pos` is its index among the messages decoded from the
    /// same read.
    fn on_message(&mut self, conn: ConnId, opcode: u8, payload: &[u8], meta: &RxMeta, pos: u16);

    /// Decides what to do with a data message from its first frame: `first`
    /// is that frame's payload (the whole message when `fin`), not yet
    /// checked for UTF-8. Called once per data message, before any delivery.
    /// The default keeps every message.
    #[inline]
    fn on_message_start(
        &mut self,
        conn: ConnId,
        opcode: u8,
        first: &[u8],
        fin: bool,
        meta: &RxMeta,
    ) -> Interest {
        let _ = (conn, opcode, first, fin, meta);
        Interest::Keep
    }

    /// One complete fragmented message whose first frame returned
    /// [`Interest::Fragments`]: its fragments' payloads, in order, borrowed
    /// from the receive buffer and valid only during the call. When the
    /// message could not stay in the buffer it arrives as one copied slice
    /// ([`WsStats::frag_fallbacks`]). A handler that returns
    /// [`Interest::Fragments`] implements this; the default drops the
    /// message.
    #[inline]
    fn on_fragments(
        &mut self,
        conn: ConnId,
        opcode: u8,
        message: &Fragments<'_>,
        meta: &RxMeta,
        pos: u16,
    ) {
        let _ = (conn, opcode, message, meta, pos);
    }
}

/// What [`WsHandler::on_message_start`] wants done with a data message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Interest {
    /// Deliver to [`WsHandler::on_message`] as one slice; a fragmented
    /// message is reassembled by copy.
    #[default]
    Keep,
    /// Like `Keep` for an unfragmented message. A fragmented message goes to
    /// [`WsHandler::on_fragments`] as its fragments' payloads in the receive
    /// buffer, with no copy.
    Fragments,
    /// Drop the message: no copy, no UTF-8 check, no delivery
    /// ([`WsStats::skipped`]). Control frames inside it are still answered.
    Skip,
}

/// Most fragments a message keeps in the receive buffer; a message with
/// more falls back to one copy ([`WsStats::frag_fallbacks`]).
pub const MAX_FRAGMENTS: usize = 32;

/// One fragment's payload inside the held bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Part {
    start: usize,
    len: usize,
}

/// A fragmented message as its fragments' payloads, in order
/// ([`WsHandler::on_fragments`]). It borrows the receive buffer: no copy, no
/// allocation.
#[derive(Debug, Clone, Copy)]
pub struct Fragments<'a> {
    base: &'a [u8],
    parts: &'a [Part],
    len: usize,
}

impl<'a> Fragments<'a> {
    /// Payload bytes, all fragments together.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the message has no payload.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of slices.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.parts.len()
    }

    /// Slice `i`, in order.
    #[must_use]
    pub fn get(&self, i: usize) -> Option<&'a [u8]> {
        self.parts
            .get(i)
            .and_then(|p| self.base.get(p.start..p.start + p.len))
    }

    /// The slices, in order.
    #[must_use]
    pub fn iter(&self) -> FragmentIter<'a> {
        FragmentIter {
            base: self.base,
            parts: self.parts.iter(),
        }
    }

    /// The payload as one slice, when it is one (a single fragment, or a
    /// message that fell back to a copy).
    #[must_use]
    pub fn contiguous(&self) -> Option<&'a [u8]> {
        if self.parts.len() == 1 {
            self.get(0)
        } else {
            None
        }
    }

    /// Copies the payload into the front of `out`; `None` when `out` is
    /// shorter than [`Fragments::len`] (nothing is copied then).
    #[must_use]
    pub fn copy_to(&self, out: &mut [u8]) -> Option<usize> {
        if out.len() < self.len {
            return None;
        }
        let mut at = 0;
        for s in self {
            out[at..at + s.len()].copy_from_slice(s);
            at += s.len();
        }
        Some(at)
    }
}

impl<'a> IntoIterator for &Fragments<'a> {
    type Item = &'a [u8];
    type IntoIter = FragmentIter<'a>;

    fn into_iter(self) -> FragmentIter<'a> {
        self.iter()
    }
}

/// The slices of a [`Fragments`], in order.
#[derive(Debug, Clone)]
pub struct FragmentIter<'a> {
    base: &'a [u8],
    parts: std::slice::Iter<'a, Part>,
}

impl<'a> Iterator for FragmentIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        let p = self.parts.next()?;
        self.base.get(p.start..p.start + p.len)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.parts.size_hint()
    }
}

impl ExactSizeIterator for FragmentIter<'_> {}

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
    /// Bytes copied to reassemble fragmented messages: those kept with
    /// [`Interest::Keep`], and those [`WsStats::frag_fallbacks`] counts.
    pub frag_copy_bytes: u64,
    /// Fragmented messages that asked for [`Interest::Fragments`] but were
    /// copied: more than [`MAX_FRAGMENTS`] fragments, or more bytes than the
    /// buffer holds behind the window.
    pub frag_fallbacks: u64,
    /// Data messages dropped by [`Interest::Skip`].
    pub skipped: u64,
    /// Server Pings answered.
    pub pings: u64,
    /// Pings sent on a quiet connection ([`WsOptions::ping_interval_ms`]).
    pub pings_sent: u64,
    /// Pongs received.
    pub pongs: u64,
    /// Frames the connection sent by itself: Pongs to the server's Pings,
    /// Pings on a quiet connection, and the echo of the server's Close. A
    /// venue that limits messages per connection may count them (including Pongs): charge the change since the last pass
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
    /// once more (`str::from_utf8`). Only kept messages are checked, after
    /// [`WsHandler::on_message_start`] and before delivery: a message it
    /// skips is never read, so invalid UTF-8 in a skipped message does not
    /// close the connection (a deliberate relaxation of RFC 6455 §8.1).
    /// Leave it off for a parser that checks text itself.
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
    /// How the fragmented message in progress is taken (when `frag_op != 0`).
    frag_mode: FragMode,
    /// Its payload bytes so far.
    frag_len: usize,
    /// Its fragments in the held bytes ([`FragMode::Hold`]).
    /// Boxed (once per connection) to keep `WsConn` near `HttpConn`'s size.
    parts: Box<[Part; MAX_FRAGMENTS]>,
    nparts: usize,
    /// Most bytes a held message may span: what a single frame may.
    hold_limit: usize,
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
    frag_fallbacks: u64,
    skipped: u64,
    pings: u64,
    pongs: u64,
    /// Frames queued by the connection itself ([`WsStats::auto_frames`]).
    auto_frames: u64,
    timers: Timers,
    protocol: Option<String>,
}

/// How a fragmented message is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FragMode {
    /// [`Interest::Keep`]: copied into `frag`, delivered by `on_message`.
    Copy,
    /// [`Interest::Fragments`]: left in the receive buffer, which holds it.
    Hold,
    /// [`Interest::Fragments`] that could not be held: copied into `frag`,
    /// delivered by `on_fragments` as one slice.
    Copied,
    /// [`Interest::Skip`]: dropped.
    Skip,
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
    /// A connection over an upgraded stream.
    fn build(stream: TlsStream, opts: &WsOptions, rng: u64, protocol: Option<String>) -> Self {
        let buf_len = stream.capacity();
        let max_message = match opts.max_message {
            0 => max_frame(buf_len),
            m => m,
        };
        Self {
            stream,
            frag: Vec::with_capacity(1 << 20),
            frag_op: 0,
            frag_mode: FragMode::Copy,
            frag_len: 0,
            parts: Box::new([Part::default(); MAX_FRAGMENTS]),
            nparts: 0,
            hold_limit: max_frame(buf_len),
            closing: false,
            discard: false,
            validate_utf8: opts.validate_utf8,
            frame_limit: max_message.min(max_frame(buf_len)),
            max_message,
            rng,
            messages: 0,
            frag_copy_bytes: 0,
            frag_fallbacks: 0,
            skipped: 0,
            pings: 0,
            pongs: 0,
            auto_frames: 0,
            timers: Timers::new(opts),
            protocol,
        }
    }

    /// Test and bench lever: a connection over `sock` whose upgrade is
    /// already done and whose TLS keys are `tls`. Nothing is read from or
    /// written to `sock`: [`WsConn::feed`] and [`WsConn::deliver_staged`]
    /// stand in for the kernel.
    ///
    /// # Errors
    /// Bad options.
    #[doc(hidden)]
    pub fn from_record_layer(sock: TcpStream, tls: RecordLayer, opts: &WsOptions) -> Result<Self> {
        opts.check()?;
        let buf = vec![1u8; opts.buf_size].into_boxed_slice();
        let stream = TlsStream::new(sock, buf, 0, 0, tls, Vec::with_capacity(1 << 16));
        Ok(Self::build(stream, opts, 0x9E37_79B9_7F4A_7C15, None))
    }

    /// Test lever: `cipher` arrives as if the kernel wrote it, then every
    /// complete record is opened and every complete message delivered.
    ///
    /// # Errors
    /// `cipher` does not fit the free buffer, or as a read would fail.
    #[doc(hidden)]
    pub fn feed<H: WsHandler>(
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

    /// Bench lever: puts `plain` into the empty window, as an opened record.
    ///
    /// # Errors
    /// The buffer is not empty or too small.
    #[doc(hidden)]
    pub fn preload_plain(&mut self, plain: &[u8]) -> Result<()> {
        self.stream.preload_window(plain)
    }

    /// Bench lever: queues one unencrypted stand-in record of `plain` (at
    /// most 16 KiB) for [`WsConn::deliver_staged`].
    ///
    /// # Errors
    /// `plain` is too large or does not fit.
    #[doc(hidden)]
    pub fn stage_plain_record(&mut self, plain: &[u8]) -> Result<()> {
        self.stream.stage_plain_record(plain)
    }

    /// Bench lever: [`WsConn::feed`]'s path minus the kernel copy and the
    /// AEAD: parses the window, then opens each staged stand-in record and
    /// parses again, then compacts.
    ///
    /// # Errors
    /// As a read would fail.
    #[doc(hidden)]
    pub fn deliver_staged<H: WsHandler>(
        &mut self,
        id: ConnId,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        let mut delivered = self.parse_frames(id, meta, handler, 0)?;
        while self.stream.open_next_plain()? {
            delivered += self.parse_frames(id, meta, handler, delivered)?;
        }
        self.stream.compact();
        Ok(delivered)
    }

    /// Test lever: drops the queued frames, as a send would.
    #[doc(hidden)]
    pub fn clear_tx(&mut self) {
        self.stream.tx_queue().clear();
    }

    /// Test lever: the encrypted frames queued to send.
    #[doc(hidden)]
    #[must_use]
    pub fn queued_tx(&self) -> &[u8] {
        self.stream.queued_tx()
    }

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
            frag_fallbacks: self.frag_fallbacks,
            skipped: self.skipped,
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
            let Some(h) = ws::parse_header(self.stream.window())? else {
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
            // A held message must leave the buffer room for the rest: one
            // that would outgrow it, or the slice list, is copied out before
            // waiting for this frame.
            if self.frag_mode == FragMode::Hold
                && self.frag_op != 0
                && (self.stream.held_len() + h.frame_len() > self.hold_limit
                    || (h.opcode == ws::OP_CONT && self.nparts == MAX_FRAGMENTS))
            {
                self.spill();
            }
            let window = self.stream.window();
            if window.len() < h.frame_len() {
                break;
            }
            let payload = &window[h.header_len..h.frame_len()];
            let plen = payload.len();
            match h.opcode {
                // Inside a fragmented message the arm below refuses it.
                ws::OP_TEXT | ws::OP_BINARY if h.fin && self.frag_op == 0 => {
                    if handler.on_message_start(id, h.opcode, payload, true, meta) == Interest::Skip
                    {
                        self.skipped += 1;
                    } else {
                        if self.validate_utf8 && h.opcode == ws::OP_TEXT {
                            check_utf8(payload)?;
                        }
                        handler.on_message(id, h.opcode, payload, meta, pos16(pos));
                        pos += 1;
                        count += 1;
                        self.messages += 1;
                    }
                }
                ws::OP_TEXT | ws::OP_BINARY => self.first_fragment(id, &h, meta, handler)?,
                ws::OP_CONT => {
                    if self.continuation(&h)? {
                        self.stream.consume(h.frame_len());
                        if self.end_message(id, meta, handler, pos16(pos))? {
                            pos += 1;
                            count += 1;
                        }
                        continue;
                    }
                }
                ws::OP_PING => {
                    let mut body = [0u8; ws::MAX_CONTROL];
                    body[..plen].copy_from_slice(payload);
                    self.stream.consume(h.frame_len());
                    self.pings += 1;
                    // Nothing may follow our Close.
                    if !self.closing {
                        self.queue(ws::OP_PONG, &body[..plen])?;
                        self.auto_frames += 1;
                    }
                    continue;
                }
                ws::OP_PONG => self.pongs += 1,
                ws::OP_CLOSE => {
                    let mut body = [0u8; ws::MAX_CONTROL];
                    body[..plen].copy_from_slice(payload);
                    self.stream.consume(h.frame_len());
                    self.on_close(&body[..plen])?;
                    return Ok(count);
                }
                _ => return Err(Error::Ws("unexpected opcode")),
            }
            self.stream.consume(h.frame_len());
        }
        Ok(count)
    }

    /// The first frame of a fragmented message (the frame is complete in
    /// the window): the handler decides how it is taken.
    #[inline]
    fn first_fragment<H: WsHandler>(
        &mut self,
        id: ConnId,
        h: &ws::FrameHeader,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<()> {
        if self.frag_op != 0 {
            return Err(Error::Ws("new message inside a fragmented one"));
        }
        let payload = &self.stream.window()[h.header_len..h.frame_len()];
        let interest = handler.on_message_start(id, h.opcode, payload, false, meta);
        self.frag_op = h.opcode;
        self.frag_len = payload.len();
        match interest {
            Interest::Keep => {
                self.frag_mode = FragMode::Copy;
                self.frag.clear();
                self.frag.extend_from_slice(payload);
                self.frag_copy_bytes += payload.len() as u64;
            }
            Interest::Skip => self.frag_mode = FragMode::Skip,
            Interest::Fragments => {
                self.frag_mode = FragMode::Hold;
                self.parts[0] = Part {
                    start: h.header_len,
                    len: payload.len(),
                };
                self.nparts = 1;
                self.stream.hold();
            }
        }
        Ok(())
    }

    /// A continuation frame (complete in the window), taken as the first
    /// frame decided. `true` when it ends the message.
    #[inline]
    fn continuation(&mut self, h: &ws::FrameHeader) -> Result<bool> {
        if self.frag_op == 0 {
            return Err(Error::Ws("continuation without a first fragment"));
        }
        let payload = &self.stream.window()[h.header_len..h.frame_len()];
        if self.frag_len + payload.len() > self.max_message {
            return Err(Error::Ws("message too large"));
        }
        self.frag_len += payload.len();
        match self.frag_mode {
            FragMode::Skip => {}
            FragMode::Copy | FragMode::Copied => {
                self.frag.extend_from_slice(payload);
                self.frag_copy_bytes += payload.len() as u64;
            }
            FragMode::Hold => {
                let part = Part {
                    start: self.stream.held_len() + h.header_len,
                    len: payload.len(),
                };
                // The spill check before the frame keeps a free slot.
                let Some(p) = self.parts.get_mut(self.nparts) else {
                    return Err(Error::Ws("fragment list overflow"));
                };
                *p = part;
                self.nparts += 1;
            }
        }
        Ok(h.fin)
    }

    /// The last fragment is in: deliver the message as its first frame
    /// asked, checking text first. `true` when delivered.
    #[inline]
    fn end_message<H: WsHandler>(
        &mut self,
        id: ConnId,
        meta: &RxMeta,
        handler: &mut H,
        pos: u16,
    ) -> Result<bool> {
        let op = self.frag_op;
        self.frag_op = 0;
        let check = self.validate_utf8 && op == ws::OP_TEXT;
        match self.frag_mode {
            FragMode::Skip => {
                self.skipped += 1;
                return Ok(false);
            }
            FragMode::Copy => {
                if check {
                    check_utf8(&self.frag)?;
                }
                handler.on_message(id, op, &self.frag, meta, pos);
            }
            FragMode::Copied => {
                if check {
                    check_utf8(&self.frag)?;
                }
                let one = [Part {
                    start: 0,
                    len: self.frag.len(),
                }];
                let m = Fragments {
                    base: &self.frag,
                    parts: &one,
                    len: self.frag.len(),
                };
                handler.on_fragments(id, op, &m, meta, pos);
            }
            FragMode::Hold => {
                let m = Fragments {
                    base: self.stream.held(),
                    parts: &self.parts[..self.nparts],
                    len: self.frag_len,
                };
                if check {
                    check_utf8_parts(&m)?;
                }
                handler.on_fragments(id, op, &m, meta, pos);
                self.nparts = 0;
                self.stream.release();
            }
        }
        self.messages += 1;
        Ok(true)
    }

    /// A held message cannot stay in the buffer: copy its fragments so far
    /// into `frag` and release them; the rest is copied as it arrives.
    #[cold]
    fn spill(&mut self) {
        let held = self.stream.held();
        self.frag.clear();
        for p in &self.parts[..self.nparts] {
            self.frag.extend_from_slice(&held[p.start..p.start + p.len]);
        }
        self.frag_copy_bytes += self.frag.len() as u64;
        self.stream.release();
        self.nparts = 0;
        self.frag_mode = FragMode::Copied;
        self.frag_fallbacks += 1;
    }

    /// Drops the plaintext not yet parsed (input after the server's Close),
    /// and any fragmented message in progress with its held bytes.
    #[cold]
    fn drop_window(&mut self) {
        self.frag_op = 0;
        self.nparts = 0;
        self.stream.release();
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
        .map_err(|_| not_utf8())
}

#[cold]
const fn not_utf8() -> Error {
    Error::Ws("text message is not UTF-8")
}

/// [`check_utf8`] over a message's fragments without joining them: a
/// character cut between two fragments is completed from the next.
fn check_utf8_parts(m: &Fragments<'_>) -> Result<()> {
    let mut carry = [0u8; 4];
    let mut have = 0usize;
    for mut s in m {
        if have > 0 {
            let need = utf8_width(carry[0]);
            let take = need.saturating_sub(have).min(s.len());
            carry[have..have + take].copy_from_slice(&s[..take]);
            have += take;
            s = &s[take..];
            if have < need {
                continue;
            }
            check_utf8(&carry[..need])?;
            have = 0;
        }
        if let Err(e) = std::str::from_utf8(s) {
            // A character cut by the end of this fragment: carry its start.
            if e.error_len().is_some() {
                return Err(not_utf8());
            }
            let rest = &s[e.valid_up_to()..];
            carry[..rest.len()].copy_from_slice(rest);
            have = rest.len();
        }
    }
    if have > 0 {
        return Err(not_utf8());
    }
    Ok(())
}

/// Bytes in the UTF-8 sequence that `lead` starts. A lead byte that
/// `from_utf8` reports as cut short is always one of these.
const fn utf8_width(lead: u8) -> usize {
    match lead {
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 2,
    }
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
        let stream = TlsStream::new(sock, buf, l, ct, tls, txq);
        Ok(WsConn::build(stream, opts, rng, up.protocol))
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
