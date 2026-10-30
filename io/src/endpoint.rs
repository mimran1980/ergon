//! What a `Driver` drives: connection ids, the sealed endpoint traits and the
//! application callbacks. Platform-independent.

use crate::error::Error;

/// Identifies a connection within one `Driver`.
///
/// A slot index plus a generation: once a connection is closed its id never matches again, even when
/// the slot is reused, so a stale id cannot reach a newer connection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnId {
    idx: u32,
    generation: u32,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl ConnId {
    pub(crate) const fn new(idx: u32, generation: u32) -> Self {
        Self { idx, generation }
    }

    pub(crate) const fn generation(self) -> u32 {
        self.generation
    }

    /// The slot index (dense, reused after close; for per-connection arrays).
    #[must_use]
    pub const fn index(self) -> usize {
        self.idx as usize
    }

    /// Test lever: an id from its parts, for stand-in drivers in tests. A
    /// made-up id never reaches a connection: the driver checks the generation.
    #[doc(hidden)]
    #[must_use]
    pub const fn from_parts(index: u32, generation: u32) -> Self {
        Self::new(index, generation)
    }
}

/// The driver-facing side of endpoints and dials. The module is private, so
/// code outside this crate can neither implement these traits nor call their
/// methods: only the driver moves bytes for a connection it owns.
pub(crate) mod sealed {
    use std::net::SocketAddr;
    use std::os::fd::RawFd;
    use std::sync::{Arc, OnceLock};

    use super::ConnId;
    use crate::RxMeta;
    use crate::error::Result;

    /// Socket, receive buffer and send queue of a connection.
    pub trait Io {
        /// Whether [`Io::expire`] must be called (request timeouts). The
        /// driver's periodic deadline check is compiled out when `false`.
        const DEADLINES: bool = false;
        /// Fails the connection if something it waits for is overdue, and
        /// runs its timers (WebSocket pings).
        fn expire(&mut self, now_ns: i64) -> Result<()> {
            let _ = now_ns;
            Ok(())
        }
        /// The error to report when the connection ends with `error`: a
        /// WebSocket whose close handshake finished reports the Close, an
        /// HTTP connection with requests in flight which of them were sent.
        fn close_reason(&mut self, error: crate::error::Error) -> crate::error::Error {
            error
        }
        /// The kernel took `n` bytes from the front of [`Io::tx_queue`]
        /// (`io_uring`: they were submitted, an upper bound). One add per
        /// send on the HTTP connection; nothing elsewhere.
        #[inline]
        fn on_handed(&mut self, n: usize) {
            let _ = n;
        }
        /// A datagram endpoint (UDP): every read is one whole message (a
        /// 0-byte read is an empty datagram, not a close), sends go out one
        /// datagram at a time ([`Io::pending_tx`]), and no TCP option or
        /// route check applies. Only `epoll-spin` drives one. The driver's
        /// datagram paths are compiled out when `false`.
        const DATAGRAM: bool = false;
        /// The stack the socket belongs to: a `Driver` drives only the
        /// endpoints of its backend's stack.
        const STACK: crate::netstack::StackId = crate::netstack::StackId::Kernel;
        /// Datagram endpoints: the next whole datagram to send, if any.
        #[inline]
        fn pending_tx(&mut self) -> Option<&[u8]> {
            None
        }
        /// Datagram endpoints: the `k`-th queued datagram (0 is the one
        /// [`Io::pending_tx`] returns), if that many are queued.
        #[inline]
        fn pending_tx_at(&self, k: usize) -> Option<&[u8]> {
            let _ = k;
            None
        }
        /// Datagram endpoints: the kernel took the datagram
        /// [`Io::pending_tx`] returned.
        #[inline]
        fn sent_tx(&mut self) {}
        /// Whether anything waits to be sent.
        #[inline]
        fn has_tx(&mut self) -> bool {
            !self.tx_queue().is_empty()
        }
        /// Datagram endpoints: a datagram longer than the receive buffer
        /// arrived (`MSG_TRUNC`); it was dropped.
        #[inline]
        fn on_truncated(&mut self) {}
        /// Datagram endpoints: the kernel reported `ECONNREFUSED` (an ICMP
        /// port unreachable from the peer) on a read or a send.
        #[inline]
        fn on_refused(&mut self) {}
        /// The socket.
        fn fd(&self) -> RawFd;
        /// The free part of the receive buffer. With `io_uring` the kernel
        /// writes into it after the call returns, so it must stay at the same
        /// address until `on_received`.
        fn rx_space(&mut self) -> &mut [u8];
        /// Datagram endpoints: the receive slots for a batched read, as the
        /// whole buffer and the slot size: `len / stride` datagrams of up to
        /// `stride` bytes. One slot, [`Io::rx_space`], by default.
        #[inline]
        fn rx_slots(&mut self) -> (&mut [u8], usize) {
            let b = self.rx_space();
            let n = b.len();
            (b, n)
        }
        /// Bytes waiting to be sent; the driver sends from the front and
        /// marks what the kernel took ([`crate::stream::TxQueue::sent`]).
        fn tx_queue(&mut self) -> &mut crate::stream::TxQueue;
        /// Input received before the endpoint was added, not yet decoded.
        fn has_buffered_input(&self) -> bool;
        /// Records what the quick-ACK route check found, or the cell
        /// ergon-io's warning thread fills in (once, when the driver takes
        /// the connection, or at TCP connect under `require`).
        fn set_route(&mut self, route: RouteSlot);
        /// What the connection holds of its route check.
        fn route_state(&self) -> &RouteSlot;
    }

    /// Decoding received bytes for handler `H`.
    pub trait Deliver<H: ?Sized>: Io {
        /// Accounts for `n` bytes written into `rx_space`, decodes and delivers
        /// every complete message. `n == 0` decodes buffered input.
        fn on_received(
            &mut self,
            id: ConnId,
            n: usize,
            meta: &RxMeta,
            handler: &mut H,
        ) -> Result<usize>;

        /// Datagram endpoints: delivers the `n`-byte datagram a batched read
        /// left in receive slot `slot` ([`Io::rx_slots`]). The default has
        /// one slot, the one [`Deliver::on_received`] reads.
        #[inline]
        fn on_datagram_at(
            &mut self,
            id: ConnId,
            slot: usize,
            n: usize,
            meta: &RxMeta,
            handler: &mut H,
        ) -> Result<usize> {
            let _ = slot;
            self.on_received(id, n, meta, handler)
        }
    }

    /// What a connection holds of its route check: the outcome, or the cell
    /// ergon-io's warning thread fills in. Reading it is one atomic load, and
    /// nothing on the read or write path reads it.
    #[derive(Debug, Clone)]
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub enum RouteSlot {
        /// Decided where the connection was set up.
        Ready(crate::route::RouteCheck),
        /// Filled in by the warning thread.
        Pending(Arc<OnceLock<crate::route::RouteCheck>>),
    }

    impl Default for RouteSlot {
        fn default() -> Self {
            Self::Ready(crate::route::RouteCheck::NotChecked)
        }
    }

    /// What a connection reads while its lookup is queued.
    static PENDING: crate::route::RouteCheck = crate::route::RouteCheck::Pending;

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    impl RouteSlot {
        /// The outcome, or [`crate::route::RouteCheck::Pending`].
        pub fn get(&self) -> &crate::route::RouteCheck {
            match self {
                Self::Ready(c) => c,
                Self::Pending(cell) => cell.get().unwrap_or(&PENDING),
            }
        }

        /// Whether the route was checked where the connection was set up (a
        /// gated dial under `require`).
        pub const fn decided(&self) -> bool {
            !matches!(
                self,
                Self::Ready(crate::route::RouteCheck::NotChecked) | Self::Pending(_)
            )
        }
    }

    /// A connection being set up.
    pub trait DialIo {
        /// The connection it produces.
        type Conn: Io;
        /// Host to resolve.
        fn host(&self) -> &str;
        /// TCP port.
        fn port(&self) -> u16;
        /// A fixed address, if DNS should be skipped.
        fn addr(&self) -> Option<SocketAddr>;
        /// Starts the non-blocking TCP connect.
        fn start(&mut self, addr: SocketAddr) -> Result<()>;
        /// Advances as far as the socket allows; `Some` once ready.
        fn step(&mut self) -> Result<Option<Self::Conn>>;
        /// Stop once TCP is connected, before the TLS handshake, until
        /// [`DialIo::route_checked`]: the driver checks the route there
        /// under `quickack_check=require`, so a refused route costs no
        /// handshake.
        fn gate_route(&mut self);
        /// The socket, while the dial waits at that point.
        fn awaiting_route(&self) -> Option<RawFd>;
        /// The route check's outcome: the dial goes on to TLS, and the
        /// connection it produces carries it.
        fn route_checked(&mut self, check: crate::route::RouteCheck);
    }
}

/// A connection a driver can own: `WsConn`, `HttpConn`, `AnyConn`,
/// `FrameConn` or `UdpConn`.
///
/// Sealed: with `io_uring` the kernel writes into the receive buffer after the
/// submitting call returns, so only this crate's types, whose buffers never move
/// while owned by a driver, can be endpoints.
#[allow(private_bounds)] // sealed on purpose: the bound is unnameable outside the crate
pub trait Endpoint: sealed::Io {}
impl<T: sealed::Io> Endpoint for T {}

/// An endpoint that can decode for application handler `H`.
#[allow(private_bounds)]
pub trait Receive<H: ?Sized>: Endpoint + sealed::Deliver<H> {}
impl<H: ?Sized, T: sealed::Deliver<H>> Receive<H> for T {}

/// A connection being set up without blocking (`WsDial`, `HttpDial`,
/// `AnyDial`): DNS is the driver's job; the dial does TCP, TLS and any
/// protocol upgrade.
#[allow(private_bounds)]
pub trait Dial: sealed::DialIo {}
impl<T: sealed::DialIo> Dial for T {}

/// Mutable access to a connection a `Driver` owns.
///
/// It offers every method of the connection but no way to replace, swap or
/// move it out. With `io_uring` a receive is always armed into the
/// connection's receive buffer, and every backend tracks the connection by its
/// socket descriptor and slot, so the connection stays where it is until the
/// driver closes it.
///
/// ```compile_fail,E0594
/// fn replace(mut c: ergon_io::ConnMut<'_, ergon_io::WsConn>, other: ergon_io::WsConn) {
///     *c = other;
/// }
/// ```
///
/// ```compile_fail,E0596
/// fn swap(mut a: ergon_io::ConnMut<'_, ergon_io::WsConn>, mut b: ergon_io::ConnMut<'_, ergon_io::WsConn>) {
///     std::mem::swap(&mut *a, &mut *b);
/// }
/// ```
pub struct ConnMut<'a, E>(&'a mut E);

impl<'a, E> ConnMut<'a, E> {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub(crate) const fn new(conn: &'a mut E) -> Self {
        Self(conn)
    }

    /// The connection itself, for this crate's forwarding methods only.
    pub(crate) const fn inner(&mut self) -> &mut E {
        self.0
    }
}

impl<E> std::ops::Deref for ConnMut<'_, E> {
    type Target = E;
    fn deref(&self) -> &E {
        self.0
    }
}

/// Connection lifecycle callbacks.
pub trait Handler {
    /// A connection started with `Driver::connect` is ready.
    fn on_open(&mut self, id: ConnId) {
        let _ = id;
    }
    /// The driver closed `id` because of `error`: [`Error::Closed`] for a peer
    /// close, [`Error::WsClose`] after a WebSocket close handshake. The
    /// connection is gone when this is called.
    fn on_closed(&mut self, id: ConnId, error: Error) {
        let _ = (id, error);
    }
}

/// Kernel-interface counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DriverStats {
    /// `poll` calls.
    pub polls: u64,
    /// `recvmsg` calls.
    pub recvmsg_calls: u64,
    /// `recvmsg` calls that returned `EAGAIN`.
    pub recvmsg_eagain: u64,
    /// `epoll_wait` calls.
    pub epoll_wait_calls: u64,
    /// `send` syscalls (epoll backends; `io_uring` sends are submitted through
    /// `io_uring_enter` or the `SQPOLL` thread and are not syscalls of their own).
    pub send_calls: u64,
    /// Sends started, one per batch of queued bytes, on every backend.
    pub sends: u64,
    /// `io_uring_enter` calls.
    pub enter_calls: u64,
    /// Receive completions with data.
    pub completions: u64,
    /// Messages delivered.
    pub messages: u64,
    /// Connections closed because of an error or peer close.
    pub closed: u64,
    /// Connections opened by `Driver::connect`.
    pub connects: u64,
    /// `Driver::connect` attempts that failed or timed out.
    pub connect_failures: u64,
    /// Longest single connection-setup step on the polling thread, in
    /// nanoseconds (the TLS handshake's crypto lands here).
    pub connect_step_max_ns: u64,
    /// Quick-ACK route lookups the polling thread made itself: under
    /// `quickack_check=require`, one per connection it set up
    /// (`Driver::connect` at TCP connect, before the TLS handshake;
    /// `Driver::add`). Under `warn` the lookup runs on ergon-io's warning
    /// thread, and `Driver::connect_background` makes it on the helper
    /// thread: neither is counted.
    pub route_lookups: u64,
    /// Polling-thread time spent on the route check, in nanoseconds: the
    /// lookups above, or under `warn` reading the connection's addresses
    /// and queuing the lookup (see [`crate::route`]).
    pub route_check_ns: u64,
    /// The longest single one of those, in nanoseconds.
    pub route_check_max_ns: u64,
    /// Passive `TCP_INFO` samples taken (`Config::tcp_info_interval_ms`).
    pub tcp_info_samples: u64,
    /// The longest single one of those `getsockopt` calls on the polling
    /// thread, in nanoseconds.
    pub tcp_info_max_ns: u64,
}

/// One connection's passive `TCP_INFO` sample.
///
/// Taken every `Config::tcp_info_interval_ms`, read with
/// `Driver::tcp_info`: what the kernel's TCP state said at `at_ns`. Fields
/// follow `struct tcp_info` (`include/uapi/linux/tcp.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TcpInfo {
    /// When it was read, `CLOCK_MONOTONIC` ns.
    pub at_ns: i64,
    /// Smoothed round-trip time, µs (`tcpi_rtt`).
    pub rtt_us: u32,
    /// Round-trip time variation, µs (`tcpi_rttvar`).
    pub rttvar_us: u32,
    /// The lowest round-trip time seen over the kernel's window, µs
    /// (`tcpi_min_rtt`); `None` before the first sample, or on a kernel
    /// whose `tcp_info` stops short of it (before 4.6).
    pub min_rtt_us: Option<u32>,
    /// Segments retransmitted over the connection's life
    /// (`tcpi_total_retrans`).
    pub total_retrans: u32,
    /// Segments the kernel currently counts as lost (`tcpi_lost`).
    pub lost: u32,
    /// The current retransmission timeout, µs (`tcpi_rto`).
    pub rto_us: u32,
}

impl DriverStats {
    /// Total syscalls made by the driver.
    #[must_use]
    pub const fn syscalls(&self) -> u64 {
        self.recvmsg_calls + self.epoll_wait_calls + self.send_calls + self.enter_calls
    }
}
