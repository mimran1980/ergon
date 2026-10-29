//! Length-prefixed frames over plain TCP (no TLS).
//!
//! - A frame on the wire is a `u32` little-endian payload length, then the
//!   payload. A 0-length frame is a frame.
//! - [`FrameConn::connect`] (blocking, for tools and tests), [`FrameDial`]
//!   with `Driver::connect` (never blocks), or [`FrameConn::from_stream`]
//!   over a socket accepted elsewhere, then `Driver::add`.
//! - Frames are parsed in place: [`FrameHandler::on_frame`] gets a slice of
//!   the receive buffer the kernel wrote into, with the read's [`RxMeta`].
//! - [`FrameConn::send_frame`] appends header and payload to the send queue,
//!   preallocated at [`FrameOptions::tx_buf`]: nothing allocates on the send
//!   path, and a frame that would take the queue past it is
//!   [`Error::Busy`].
//! - A received length over [`FrameOptions::max_frame`], or a peer close in
//!   the middle of a frame, closes the connection with [`Error::Frame`].
//! - `TCP_NODELAY` is always set; the quick-ACK route check
//!   (`Config::quickack_check`) and the TCP keys (keepalive,
//!   `tcp_rto_min_us`, `tcp_delack_max_us`) apply as to every TCP
//!   connection. [`FrameOptions::quickack_per_read`] sets `TCP_QUICKACK`
//!   again after every read with data.
//! - Every backend drives it. Over F-Stack (`fstack-spin`) it is
//!   `FrameConn<FstackTcp>`, made by [`FrameDial::on_stack`] or
//!   [`FrameConn::from_stream`].

use std::net::{SocketAddr, TcpStream};
use std::os::fd::RawFd;
use std::time::Duration;

use crate::RxMeta;
use crate::endpoint::sealed::{Deliver, DialIo, Io, RouteSlot};
use crate::endpoint::{ConnId, ConnMut, Handler};
use crate::error::{Error, Result};
use crate::net::{BindSource, StreamSocket, Target};
use crate::netstack::StackId;
use crate::route::RouteCheck;
use crate::stream::TxQueue;

/// Bytes of the length prefix.
pub const HEADER_LEN: usize = 4;

/// Receives frames.
pub trait FrameHandler: Handler {
    /// One frame from `conn`: its payload (possibly empty), a slice of the
    /// receive buffer valid for this call only.
    fn on_frame(&mut self, conn: ConnId, payload: &[u8], meta: &RxMeta);
}

/// Framed-TCP settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameOptions {
    /// The longest payload received or sent, bytes. Default 64 KiB; at most
    /// `u32::MAX`.
    pub max_frame: usize,
    /// Receive buffer, bytes: at least `max_frame` + 4. Default 256 KiB.
    pub rx_buf: usize,
    /// Send queue, bytes: the most queued and not yet taken by the kernel;
    /// at least `max_frame` + 4. Default 256 KiB.
    pub tx_buf: usize,
    /// `SO_SNDBUF`, bytes (`None`, the default: the kernel's).
    pub sndbuf: Option<u32>,
    /// Sets `TCP_QUICKACK` after every read that returned data (one
    /// `setsockopt` each, counted in [`FrameStats::quickacks`]): the
    /// kernel leaves quick-ACK mode on its own once it judges the
    /// connection interactive, and this puts it back. Linux kernel sockets
    /// only (F-Stack's delayed ACKs are off: `fstack-spin` refuses it).
    /// Default off.
    pub quickack_per_read: bool,
}

impl Default for FrameOptions {
    fn default() -> Self {
        Self {
            max_frame: 64 << 10,
            rx_buf: 256 << 10,
            tx_buf: 256 << 10,
            sndbuf: None,
            quickack_per_read: false,
        }
    }
}

impl FrameOptions {
    fn check(&self) -> Result<()> {
        if u32::try_from(self.max_frame).is_err() {
            return Err(Error::Config(format!(
                "frame max_frame {} is over u32::MAX",
                self.max_frame
            )));
        }
        let need = self.max_frame + HEADER_LEN;
        for (key, v) in [("rx_buf", self.rx_buf), ("tx_buf", self.tx_buf)] {
            if v < need {
                return Err(Error::Config(format!(
                    "frame {key} {v}: at least max_frame + 4 ({need})"
                )));
            }
        }
        if self.quickack_per_read && !cfg!(target_os = "linux") {
            return Err(Error::Unsupported(
                "frame quickack_per_read: TCP_QUICKACK is Linux only",
            ));
        }
        Ok(())
    }

    /// [`FrameOptions::check`], and what `stack` cannot honour.
    fn check_for(&self, stack: StackId) -> Result<()> {
        self.check()?;
        if self.quickack_per_read && stack != StackId::Kernel {
            return Err(Error::Unsupported(
                "frame quickack_per_read: TCP_QUICKACK is the kernel's (F-Stack runs with delayed ACKs off)",
            ));
        }
        Ok(())
    }
}

/// Per-connection counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// Bytes received from the kernel.
    pub bytes_in: u64,
    /// Frames delivered.
    pub frames_in: u64,
    /// Frames queued by [`FrameConn::send_frame`].
    pub frames_out: u64,
    /// Bytes of a partial frame moved to the front of the receive buffer.
    pub compact_bytes: u64,
    /// `TCP_QUICKACK` set after a read ([`FrameOptions::quickack_per_read`]).
    pub quickacks: u64,
}

/// A connected TCP socket that carries length-prefixed frames. Hand it to a
/// `Driver`; the socket closes when it is dropped. `T` is the socket's
/// stack ([`StreamSocket`]): the kernel's `TcpStream` by default.
pub struct FrameConn<T = TcpStream> {
    sock: T,
    buf: Box<[u8]>,
    /// Unparsed bytes are `head..tail`.
    head: usize,
    tail: usize,
    max_frame: usize,
    tx_limit: usize,
    txq: TxQueue,
    /// Read only on Linux (`FrameOptions::check` refuses it elsewhere).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    quickack_per_read: bool,
    route: RouteSlot,
    stats: FrameStats,
}

impl FrameConn {
    /// Connects to `addr`, waiting at most 10 s: a convenience for tools and
    /// tests. In a busy-spin loop use [`FrameDial`] with `Driver::connect`,
    /// which never waits. Other stacks: [`FrameDial::connect_blocking`].
    ///
    /// # Errors
    /// Bad options, or the connect fails or times out.
    pub fn connect(addr: SocketAddr, opts: FrameOptions) -> Result<Self> {
        opts.check()?;
        let sock = TcpStream::connect_timeout(&addr, Duration::from_secs(10))?;
        Self::from_stream(sock, opts)
    }
}

impl<T: StreamSocket> FrameConn<T> {
    /// A connection over `sock`, a connected TCP socket (one a listener
    /// accepted, say): made non-blocking, `TCP_NODELAY` set. Hand it to
    /// `Driver::add`.
    ///
    /// # Errors
    /// Bad options, or a socket option fails.
    pub fn from_stream(sock: T, opts: FrameOptions) -> Result<Self> {
        opts.check_for(T::STACK)?;
        sock.configure(opts.sndbuf).map_err(|e| match e {
            Error::Config(m) => Error::Config(format!("frame {m}")),
            e => e,
        })?;
        Ok(Self {
            sock,
            // Non-zero fill writes every page now (zeroed memory is mapped
            // lazily), so the first receives take no page faults.
            buf: vec![1u8; opts.rx_buf].into_boxed_slice(),
            head: 0,
            tail: 0,
            max_frame: opts.max_frame,
            tx_limit: opts.tx_buf,
            txq: TxQueue::prefaulted(opts.tx_buf),
            quickack_per_read: opts.quickack_per_read,
            route: RouteSlot::default(),
            stats: FrameStats::default(),
        })
    }

    /// Queues one frame: its length and `payload` go on the send queue, sent
    /// by the driver's next send pass (`Driver::poll` or `Driver::flush`).
    ///
    /// # Errors
    /// [`Error::Frame`] for a payload over [`FrameOptions::max_frame`];
    /// [`Error::Busy`] when the queue would pass [`FrameOptions::tx_buf`]
    /// (nothing is queued).
    #[inline]
    pub fn send_frame(&mut self, payload: &[u8]) -> Result<()> {
        if payload.len() > self.max_frame {
            std::hint::cold_path();
            return Err(Error::Frame("frame to send is over max_frame"));
        }
        let len = u32::try_from(payload.len())
            .map_err(|_| Error::Frame("frame to send is over u32::MAX"))?;
        let q = self
            .txq
            .room_within(HEADER_LEN + payload.len(), self.tx_limit)
            .ok_or(Error::Busy("frame send queue full"))?;
        q.extend_from_slice(&len.to_le_bytes());
        q.extend_from_slice(payload);
        self.stats.frames_out += 1;
        Ok(())
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> FrameStats {
        self.stats
    }

    /// Received bytes not yet delivered: a partial frame.
    #[must_use]
    pub const fn buffered(&self) -> usize {
        self.tail - self.head
    }

    /// Bytes queued and not yet taken by the kernel.
    #[must_use]
    pub const fn queued(&self) -> usize {
        self.txq.len()
    }

    /// The socket descriptor (for inspection; the driver owns the socket).
    /// An F-Stack socket's is F-Stack's own handle, not a kernel descriptor.
    #[must_use]
    pub fn raw_fd(&self) -> RawFd {
        self.sock.fd()
    }

    /// What the quick-ACK route check found ([`crate::route`]), or
    /// [`RouteCheck::Pending`] until ergon-io's warning thread has made it.
    #[must_use]
    pub fn route(&self) -> &RouteCheck {
        self.route.get()
    }

    /// Test lever: `bytes` arrive as if the kernel wrote them, then every
    /// complete frame is delivered.
    ///
    /// # Errors
    /// `bytes` do not fit the free buffer, or as a read would fail.
    #[doc(hidden)]
    pub fn feed<H: FrameHandler>(
        &mut self,
        id: ConnId,
        bytes: &[u8],
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        let space = self.rx_space();
        if bytes.len() > space.len() {
            return Err(Error::Config("feed larger than the free buffer".into()));
        }
        space[..bytes.len()].copy_from_slice(bytes);
        self.deliver(id, bytes.len(), meta, handler)
    }

    /// Test lever: the free part of the receive buffer, bytes.
    #[doc(hidden)]
    #[must_use]
    pub const fn free(&self) -> usize {
        self.buf.len() - self.tail
    }

    /// Test lever: the queued bytes.
    #[doc(hidden)]
    #[must_use]
    pub fn queued_tx(&self) -> &[u8] {
        self.txq.pending()
    }

    /// Test lever: drops the queued bytes, as a send would.
    #[doc(hidden)]
    pub fn clear_tx(&mut self) {
        self.txq.clear();
    }

    /// Accounts for `n` bytes written into the free tail and delivers every
    /// complete frame.
    #[inline]
    fn deliver<H: FrameHandler>(
        &mut self,
        id: ConnId,
        n: usize,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        self.tail = (self.tail + n).min(self.buf.len());
        self.stats.bytes_in += n as u64;
        // Off Linux `FrameOptions::check` refuses the option, and on another
        // stack `check_for` does.
        #[cfg(target_os = "linux")]
        if T::STACK == StackId::Kernel && self.quickack_per_read && n > 0 {
            crate::sys::set_quickack(self.sock.fd())?;
            self.stats.quickacks += 1;
        }
        let mut delivered = 0;
        while self.tail - self.head >= HEADER_LEN {
            let h = self.head;
            let len = u32::from_le_bytes([
                self.buf[h],
                self.buf[h + 1],
                self.buf[h + 2],
                self.buf[h + 3],
            ]) as usize;
            if len > self.max_frame {
                std::hint::cold_path();
                return Err(Error::Frame("received frame is over max_frame"));
            }
            let end = h + HEADER_LEN + len;
            if end > self.tail {
                break;
            }
            handler.on_frame(id, &self.buf[h + HEADER_LEN..end], meta);
            self.head = end;
            delivered += 1;
        }
        self.stats.frames_in += delivered as u64;
        self.compact();
        Ok(delivered)
    }

    /// Resets an empty buffer; moves a partial frame to the front once the
    /// free tail could not hold a whole one. The moved bytes are at most
    /// one frame, and a whole frame then always fits.
    #[inline]
    fn compact(&mut self) {
        if self.head == self.tail {
            self.head = 0;
            self.tail = 0;
        } else if self.buf.len() - self.tail < self.max_frame + HEADER_LEN && self.head > 0 {
            let live = self.tail - self.head;
            self.buf.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = live;
            self.stats.compact_bytes += live as u64;
        }
    }
}

/// A driver-owned [`FrameConn`]: the same send.
impl<T: StreamSocket> ConnMut<'_, FrameConn<T>> {
    /// [`FrameConn::send_frame`].
    ///
    /// # Errors
    /// As [`FrameConn::send_frame`].
    #[inline]
    pub fn send_frame(&mut self, payload: &[u8]) -> Result<()> {
        self.inner().send_frame(payload)
    }
}

impl<T: StreamSocket> Io for FrameConn<T> {
    const STACK: StackId = T::STACK;

    #[inline]
    fn fd(&self) -> RawFd {
        self.sock.fd()
    }
    #[inline]
    fn rx_space(&mut self) -> &mut [u8] {
        &mut self.buf[self.tail..]
    }
    #[inline]
    fn tx_queue(&mut self) -> &mut TxQueue {
        &mut self.txq
    }
    fn has_buffered_input(&self) -> bool {
        // Nothing is read before the driver takes the socket.
        false
    }
    fn set_route(&mut self, route: RouteSlot) {
        self.route = route;
    }
    fn route_state(&self) -> &RouteSlot {
        &self.route
    }
    fn close_reason(&mut self, error: Error) -> Error {
        if matches!(error, Error::Closed) && self.head < self.tail {
            Error::Frame("peer closed inside a frame")
        } else {
            error
        }
    }
}

impl<T: StreamSocket, H: FrameHandler> Deliver<H> for FrameConn<T> {
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

enum Stage<T> {
    Idle,
    Tcp(T),
    /// Connected; waits for the route check under `require`.
    Gated(T),
    Done,
}

/// A framed-TCP connection being set up without blocking: the TCP connect,
/// advanced by the driver's `poll`. Hand it to `Driver::connect`. `T` is
/// the socket's stack, as for [`FrameConn`].
pub struct FrameDial<T = TcpStream> {
    target: Target,
    opts: FrameOptions,
    stage: Stage<T>,
    gate: bool,
    route: Option<RouteCheck>,
}

impl FrameDial {
    /// A dial to `target`: its host (resolved by the driver unless
    /// [`Target::addr`] is set), port and source; the path and headers are
    /// not used.
    ///
    /// # Errors
    /// Bad options.
    pub fn new(target: Target, opts: FrameOptions) -> Result<Self> {
        Self::on_stack(target, opts)
    }
}

impl<T: StreamSocket> FrameDial<T> {
    /// [`FrameDial::new`] on the stack of socket type `T`
    /// (`FrameDial::<FstackTcp>::on_stack` for F-Stack).
    ///
    /// # Errors
    /// Bad options, or one `T`'s stack cannot honour.
    pub fn on_stack(target: Target, opts: FrameOptions) -> Result<Self> {
        opts.check_for(T::STACK)?;
        Ok(Self {
            target,
            opts,
            stage: Stage::Idle,
            gate: false,
            route: None,
        })
    }

    /// Where it connects.
    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }

    /// Connects to [`Target::addr`] (needed: no DNS here), waiting at most
    /// 10 s on the calling thread, which runs the stack meanwhile (for
    /// F-Stack: its polling thread): a convenience for tools and tests.
    ///
    /// # Errors
    /// No `Target::addr`, or the connect fails or times out.
    pub fn connect_blocking(mut self) -> Result<FrameConn<T>> {
        let addr = self
            .target
            .addr
            .ok_or_else(|| Error::Config("connect_blocking needs Target::addr".into()))?;
        self.start(addr)?;
        crate::net::wait_for::<T, _>(Duration::from_secs(10), "frame connect", || self.step())
    }
}

impl<T: StreamSocket> BindSource for FrameDial<T> {
    fn bind_source(&mut self, ip: std::net::IpAddr) {
        self.target.source = Some(ip);
    }
}

impl<T: StreamSocket> DialIo for FrameDial<T> {
    type Conn = FrameConn<T>;

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
        self.stage = Stage::Tcp(T::start_connect(addr, self.target.source)?);
        Ok(())
    }

    fn step(&mut self) -> Result<Option<FrameConn<T>>> {
        loop {
            match std::mem::replace(&mut self.stage, Stage::Done) {
                Stage::Idle => {
                    self.stage = Stage::Idle;
                    return Ok(None);
                }
                Stage::Tcp(sock) => {
                    if !sock.connect_finished()? {
                        self.stage = Stage::Tcp(sock);
                        return Ok(None);
                    }
                    self.stage = Stage::Gated(sock);
                }
                Stage::Gated(sock) => {
                    if self.gate && self.route.is_none() {
                        self.stage = Stage::Gated(sock);
                        return Ok(None);
                    }
                    let mut conn = FrameConn::from_stream(sock, self.opts)?;
                    if let Some(check) = self.route.take() {
                        conn.route = RouteSlot::Ready(check);
                    }
                    return Ok(Some(conn));
                }
                Stage::Done => return Err(Error::Config("dial already finished".into())),
            }
        }
    }

    fn gate_route(&mut self) {
        self.gate = true;
    }

    fn awaiting_route(&self) -> Option<RawFd> {
        match &self.stage {
            Stage::Gated(sock) if self.gate && self.route.is_none() => Some(sock.fd()),
            _ => None,
        }
    }

    fn route_checked(&mut self, check: RouteCheck) {
        self.route = Some(check);
    }
}
