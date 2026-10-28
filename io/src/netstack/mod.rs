//! The network-stack seam: every call the driver's epoll-family backends make
//! into a TCP/IP stack, so a user-space stack (F-Stack or another stack on
//! DPDK) can stand in for the kernel's.
//!
//! - [`Sockets`]: making sockets: a non-blocking TCP connect, its completion
//!   check, and a bound UDP socket. Every platform (the dials build on
//!   macOS too).
//! - `NetStack` (Linux): the polling loop's calls: a readiness poller
//!   (epoll-like), `recvmsg` into the driver's buffers with receive
//!   timestamps, stream and datagram sends, the socket options the driver
//!   sets, the NIC queue id, and `TCP_INFO`.
//! - [`Kernel`]: both, over the Linux kernel's sockets (`crate::sys`). It
//!   is zero-sized and every method is `#[inline]`, so code generic over a
//!   stack and instantiated with `Kernel` compiles to the direct system
//!   calls.
//! - [`Socket`]: a connected stream socket of one stack, as the TCP
//!   endpoints hold it (`FrameConn<T>`, `WsConn<T>`, `HttpConn<T>`):
//!   `std::net::TcpStream` (the kernel's) or, with the `fstack` feature,
//!   `FstackTcp`. Dropping one closes it through its own stack, so a kernel
//!   `close(2)` never reaches an F-Stack descriptor.
//! - `fstack` (Linux, the `fstack` feature): F-Stack, FreeBSD's TCP/IP
//!   stack on DPDK, behind `fstack-spin`.
//!
//! The epoll backends (`epoll-spin`, `recv-busy`) and the driver's send
//! passes are generic over `NetStack`, defaulting to `Kernel`; `Driver<E>`
//! is not. `uring-sqpoll`, the quick-ACK route check, `TCP_CORK` detection
//! and the timestamp-keeper socket stay kernel-only. Sockets are plain
//! `c_int` handles, as both the kernel's and F-Stack's are.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::RawFd;

use crate::error::Result;

#[cfg(all(target_os = "linux", feature = "fstack"))]
pub mod fstack;
pub mod fstack_conf;
mod kernel;

/// The Linux kernel's TCP/IP stack.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kernel;

/// Which stack owns a socket (and so which calls may touch it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackId {
    /// The kernel's: `std::net::TcpStream`.
    Kernel,
    /// F-Stack's: `FstackTcp`.
    Fstack,
}

impl StackId {
    /// The stack's name, for errors.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Kernel => "the kernel's",
            Self::Fstack => "F-Stack's",
        }
    }
}

/// A connected (or connecting) non-blocking TCP socket of one stack, as an
/// endpoint holds it. Reading and writing go through the stack; dropping it
/// closes it there.
pub trait Socket: io::Read + io::Write + Sized + 'static {
    /// The stack that owns it.
    const STACK: StackId;

    /// The stack's handle (a kernel descriptor, or F-Stack's).
    fn fd(&self) -> RawFd;

    /// Starts a non-blocking connect (`TCP_NODELAY` set), from `source`
    /// when given.
    ///
    /// # Errors
    /// As [`Sockets::tcp_connect`].
    fn start_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<Self>;

    /// Whether the connect has finished (`Ok(false)`: in progress).
    ///
    /// # Errors
    /// The connect failed.
    fn connect_finished(&self) -> Result<bool>;

    /// Makes a socket handed in (accepted elsewhere) ready for an endpoint:
    /// non-blocking, `TCP_NODELAY`, and `SO_SNDBUF` when given.
    ///
    /// # Errors
    /// An option fails, or `sndbuf` does not fit.
    fn configure(&self, sndbuf: Option<u32>) -> Result<()>;

    /// Runs the stack once, for the blocking helpers (`connect_blocking`)
    /// that wait on the calling thread: nothing for the kernel, which runs
    /// on its own; one F-Stack pass (`ff_run_once`).
    fn pump() {}
}

/// How a stack makes sockets.
pub trait Sockets {
    /// A TCP socket.
    type Tcp;
    /// A UDP socket.
    type Udp;

    /// Starts a non-blocking TCP connect (`TCP_NODELAY` set), from `source`
    /// when given (bound to it with port 0).
    ///
    /// # Errors
    /// [`crate::Error::Config`]: `source` is not of `addr`'s family. Socket
    /// creation, the bind, or an immediate connect failure.
    fn tcp_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<Self::Tcp>;

    /// Whether a non-blocking connect has finished (`Ok(false)`: still in
    /// progress).
    ///
    /// # Errors
    /// The connect failed.
    fn connect_finished(sock: &Self::Tcp) -> Result<bool>;

    /// A non-blocking UDP socket bound to `local`, with `SO_SNDBUF` set to
    /// `sndbuf` when given.
    ///
    /// # Errors
    /// [`crate::Error::Config`]: `sndbuf` does not fit this platform. Socket
    /// creation, the option or the bind fails.
    fn udp_bind(local: SocketAddr, sndbuf: Option<u32>) -> Result<Self::Udp>;
}

/// The polling loop's calls into a stack. Associated functions only: a stack
/// is process-wide (the kernel; F-Stack's `ff_*` calls), so a backend holds
/// no stack value. Sockets are the stack's `c_int` handles.
#[cfg(target_os = "linux")]
pub trait NetStack {
    /// A readiness poller (an epoll instance). Dropping it closes it.
    type Poller;
    /// One readiness event, as the poller fills them in.
    type Event: Copy;
    /// A receive's header and control buffer, at a stable address (boxed).
    type RxSlot;

    /// A new poller.
    ///
    /// # Errors
    /// The stack refuses one.
    fn poller() -> io::Result<Self::Poller>;

    /// An empty event, to size the event buffer.
    fn empty_event() -> Self::Event;

    /// The token `ev` was registered with.
    fn token(ev: &Self::Event) -> u64;

    /// Reports `fd` readable, level-triggered, as `token`.
    ///
    /// # Errors
    /// The poller refuses the socket.
    fn register(poller: &Self::Poller, fd: libc::c_int, token: u64) -> io::Result<()>;

    /// Stops reporting `fd`; an error is ignored (the socket is closing).
    fn deregister(poller: &Self::Poller, fd: libc::c_int);

    /// The ready sockets, without waiting: fills `events` from the start
    /// and returns how many. An interrupted wait is 0.
    ///
    /// # Errors
    /// The poller fails.
    fn poll_now(poller: &Self::Poller, events: &mut [Self::Event]) -> io::Result<usize>;

    /// A receive slot.
    fn rx_slot() -> Box<Self::RxSlot>;

    /// Points `slot` at `buf` for the next receive. Call before every
    /// receive; `buf` must stay put until it returns.
    fn arm(slot: &mut Self::RxSlot, buf: &mut [u8]);

    /// A non-blocking receive through the armed `slot`: `Ok(None)` when
    /// nothing is queued.
    ///
    /// # Errors
    /// Any other receive error.
    fn recv(fd: libc::c_int, slot: &mut Self::RxSlot) -> io::Result<Option<usize>>;

    /// Whether the last receive through `slot` cut a datagram short.
    fn truncated(slot: &Self::RxSlot) -> bool;

    /// The receive metadata of the last receive through `slot` (its
    /// timestamps), with read sequence number `read_seq`.
    fn rx_meta(slot: &Self::RxSlot, read_seq: u64) -> crate::RxMeta;

    /// A non-blocking stream send: bytes taken (0 when the send buffer is
    /// full).
    ///
    /// # Errors
    /// Any other send error.
    fn send(fd: libc::c_int, buf: &[u8]) -> io::Result<usize>;

    /// A non-blocking send of one whole datagram: `Ok(false)` when the send
    /// buffer is full.
    ///
    /// # Errors
    /// Any other send error.
    fn send_datagram(fd: libc::c_int, buf: &[u8]) -> io::Result<bool>;

    /// Whether `e` reports an earlier datagram's ICMP port unreachable
    /// (`ECONNREFUSED`, in this stack's numbering).
    fn refused(e: &io::Error) -> bool;

    /// Requests software and hardware receive timestamps.
    ///
    /// # Errors
    /// The option fails.
    fn enable_rx_timestamps(fd: libc::c_int) -> io::Result<()>;

    /// Sets `SO_RCVBUF`.
    ///
    /// # Errors
    /// The option fails.
    fn set_rcvbuf(fd: libc::c_int, bytes: u32) -> io::Result<()>;

    /// Sets `SO_BUSY_POLL`, µs.
    ///
    /// # Errors
    /// The option fails.
    fn set_busy_poll(fd: libc::c_int, usec: u32) -> io::Result<()>;

    /// Turns TCP keepalive on with these probe settings.
    ///
    /// # Errors
    /// An option fails or a value is out of range.
    fn set_tcp_keepalive(
        fd: libc::c_int,
        idle_s: u32,
        interval_s: u32,
        count: u32,
    ) -> io::Result<()>;

    /// Sets `TCP_USER_TIMEOUT`, ms.
    ///
    /// # Errors
    /// The option fails or the value is out of range.
    fn set_tcp_user_timeout(fd: libc::c_int, ms: u32) -> io::Result<()>;

    /// Sets `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US`, each when given.
    ///
    /// # Errors
    /// As `crate::sys::set_tcp_timer`.
    fn apply_tcp_timers(
        fd: libc::c_int,
        rto_min_us: Option<u32>,
        delack_max_us: Option<u32>,
    ) -> Result<()>;

    /// The NIC queue (NAPI id) of the socket's last packet, 0 when none is
    /// known.
    ///
    /// # Errors
    /// The query fails.
    fn napi_id(fd: libc::c_int) -> io::Result<u32>;

    /// The connection's `TCP_INFO`, `at_ns` left 0.
    ///
    /// # Errors
    /// The query fails.
    fn tcp_info(fd: libc::c_int) -> io::Result<crate::TcpInfo>;

    /// One pass of a user-space stack (NIC receive burst, timers, transmit
    /// drain), at the start of every poll and after a send pass that sent:
    /// nothing for the kernel, which runs on its own.
    #[inline]
    fn run_once() {}
}
