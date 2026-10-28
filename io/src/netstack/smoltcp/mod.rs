//! [`Smol`]: the seam over smoltcp on a DPDK port (the `smoltcp-spin`
//! backend). See [`crate::smoltcp_dpdk`] for what it does and does not do.
//!
//! - The stack is the polling thread's (a thread-local), as the seam's
//!   associated functions expect of a process-wide stack. Every driver with
//!   this backend on the thread shares it; each holds a poller id.
//! - `run_once` is a stack pass ([`stack::Inner::poll`]: an RX burst,
//!   smoltcp's ingress and egress, a TX burst); `poll_now` reports the
//!   poller's readable sockets; [`egress`], after a send pass, sends what
//!   the pass queued.
//! - Sockets: [`SmolTcp`] (TCP endpoints' `T`, made by their dials'
//!   `on_stack`) and [`SmolUdp`] (`UdpConn<SmolUdp>`), each a handle into
//!   the stack ([`stack`]); dropping one closes it there.

mod port;
mod stack;

use std::cell::RefCell;
use std::io;
use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::os::fd::RawFd;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Datagram, NetStack, Socket, Sockets, StackId};
use crate::error::{Error, Result};
use crate::smoltcp_dpdk::{StackConfig, StackStats, TcpStats};

/// The smoltcp stack on this thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Smol;

thread_local! {
    static STACK: RefCell<Option<Box<stack::Inner>>> = const { RefCell::new(None) };
}

/// A stack runs somewhere in this process (one RX queue: one stack).
static RUNNING: AtomicBool = AtomicBool::new(false);
/// EAL and port start-up, one thread at a time.
static SETUP: Mutex<()> = Mutex::new(());

fn no_stack() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotConnected,
        "no smoltcp stack on this thread: call ergon_io::smoltcp_dpdk::Stack::init on the polling thread first",
    )
}

/// Runs `f` on this thread's stack.
#[inline]
fn with<R>(f: impl FnOnce(&mut stack::Inner) -> io::Result<R>) -> io::Result<R> {
    STACK.with(|c| {
        c.borrow_mut()
            .as_deref_mut()
            .map_or_else(|| Err(no_stack()), f)
    })
}

/// Runs `f` on this thread's stack if there is one.
#[inline]
fn with_opt<R>(f: impl FnOnce(&mut stack::Inner) -> R) -> Option<R> {
    STACK
        .try_with(|c| c.try_borrow_mut().ok()?.as_deref_mut().map(f))
        .ok()
        .flatten()
}

/// Starts the stack on this thread (`Stack::init`).
pub fn init(cfg: &StackConfig) -> Result<()> {
    if STACK.with(|c| c.borrow().is_some()) || RUNNING.swap(true, Ordering::AcqRel) {
        return Err(Error::Busy(
            "a smoltcp stack is already running in this process",
        ));
    }
    let started = (|| -> io::Result<Box<stack::Inner>> {
        let _g = SETUP
            .lock()
            .map_err(|_| io::Error::other("smoltcp set-up lock poisoned"))?;
        port::eal_init(&cfg.eal_args)?;
        port::thread_attach()?;
        let p = port::port_start(cfg)?;
        stack::Inner::new(cfg, &p).map(Box::new)
    })();
    match started {
        Ok(s) => {
            STACK.with(|c| *c.borrow_mut() = Some(s));
            Ok(())
        }
        Err(e) => {
            port::thread_detach();
            RUNNING.store(false, Ordering::Release);
            Err(Error::Io(e))
        }
    }
}

/// Drops this thread's stack (`Stack`'s drop): every connection reset.
pub fn shutdown() {
    let s = STACK.try_with(|c| c.borrow_mut().take()).ok().flatten();
    if let Some(mut s) = s {
        s.abort_all();
        drop(s);
        port::thread_detach();
        RUNNING.store(false, Ordering::Release);
    }
}

pub fn stats() -> StackStats {
    with_opt(|s| s.stats()).unwrap_or_default()
}

pub fn mac() -> [u8; 6] {
    with_opt(|s| s.mac()).unwrap_or_default()
}

pub fn driver() -> String {
    with_opt(|s| s.driver.clone()).unwrap_or_default()
}

pub fn tx_checksum_offload() -> bool {
    with_opt(|s| s.tx_checksum_offload()).unwrap_or(false)
}

pub fn tcp_stats(fd: RawFd) -> Option<TcpStats> {
    with_opt(|s| s.tcp_stats(fd)).flatten()
}

/// Sends what a send pass queued, and bursts it.
pub fn egress() {
    let _ = with_opt(stack::Inner::egress);
}

/// Whether there is a stack on this thread.
pub fn present() -> bool {
    STACK.with(|c| c.borrow().is_some())
}

fn v4(addr: SocketAddr, source: Option<IpAddr>) -> Result<SocketAddrV4> {
    let SocketAddr::V4(a) = addr else {
        return Err(Error::Unsupported("the smoltcp-spin backend is IPv4 only"));
    };
    if let Some(ip) = source
        && !ip.is_ipv4()
    {
        return Err(Error::Config(format!(
            "source {ip} is not of the destination {addr}'s address family"
        )));
    }
    Ok(a)
}

/// A TCP socket of this thread's smoltcp stack.
///
/// What a `smoltcp-spin` endpoint holds (`FrameConn<SmolTcp>`,
/// `WsConn<SmolTcp>`, `HttpConn<SmolTcp>`, made by their dials'
/// `on_stack`). Its handle is the stack's, not a kernel descriptor;
/// dropping it closes the connection on the stack (FIN, or RST with unread
/// data). Neither `Send` nor `Sync`: the stack belongs to its thread.
#[derive(Debug)]
pub struct SmolTcp {
    fd: RawFd,
    _thread_bound: PhantomData<*const ()>,
}

impl SmolTcp {
    /// The socket's handle (the stack's).
    #[must_use]
    pub const fn raw(&self) -> RawFd {
        self.fd
    }
}

impl Drop for SmolTcp {
    fn drop(&mut self) {
        let fd = self.fd;
        let _ = with_opt(|s| s.release(fd));
    }
}

impl io::Read for SmolTcp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        with(|s| s.setup_read(self.fd, buf))
    }
}

impl io::Write for SmolTcp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        with(|s| s.setup_write(self.fd, buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Socket for SmolTcp {
    const STACK: StackId = StackId::Smoltcp;

    #[inline]
    fn fd(&self) -> RawFd {
        self.fd
    }

    fn start_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<Self> {
        Smol::tcp_connect(addr, source)
    }

    fn connect_finished(&self) -> Result<bool> {
        Smol::connect_finished(self)
    }

    fn configure(&self, sndbuf: Option<u32>) -> Result<()> {
        // Made non-blocking and without Nagle; its buffers are the stack's
        // (`StackConfig::tcp_tx_buf`).
        let _ = sndbuf;
        with(|s| s.tcp_stats(self.fd).map(|_| ()).ok_or_else(stack::no_slot)).map_err(Error::Io)
    }

    fn pump() {
        let _ = with_opt(stack::Inner::poll);
    }
}

/// A UDP socket of this thread's smoltcp stack: what a `smoltcp-spin`
/// `UdpConn<SmolUdp>` holds (`Stack::udp_connect`, `Stack::udp_bind`).
/// Dropping it closes it on the stack.
#[derive(Debug)]
pub struct SmolUdp {
    fd: RawFd,
    _thread_bound: PhantomData<*const ()>,
}

impl SmolUdp {
    /// A socket on this thread's stack from `port` (0: ephemeral), sending
    /// to `peer` when given.
    pub(crate) fn open(port: u16, peer: Option<SocketAddrV4>) -> Result<Self> {
        let fd = with(|s| s.udp_socket(port, peer)).map_err(Error::Io)?;
        Ok(Self {
            fd,
            _thread_bound: PhantomData,
        })
    }
}

impl Drop for SmolUdp {
    fn drop(&mut self) {
        let fd = self.fd;
        let _ = with_opt(|s| s.release(fd));
    }
}

impl Datagram for SmolUdp {
    const STACK: StackId = StackId::Smoltcp;

    #[inline]
    fn fd(&self) -> RawFd {
        self.fd
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        with_opt(|s| s.udp_addrs(self.fd))
            .flatten()
            .map(|(l, _)| SocketAddr::V4(l))
            .ok_or_else(|| Error::Io(stack::no_slot()))
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        with_opt(|s| s.udp_addrs(self.fd))
            .flatten()
            .and_then(|(_, p)| p.map(SocketAddr::V4))
    }

    fn recv_buffer_size(&self) -> Result<usize> {
        with_opt(|s| s.udp_buf()).ok_or_else(|| Error::Io(no_stack()))
    }
}

impl Sockets for Smol {
    type Tcp = SmolTcp;
    type Udp = SmolUdp;

    fn tcp_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<SmolTcp> {
        let a = v4(addr, source)?;
        let fd = with(|s| {
            if let Some(IpAddr::V4(ip)) = source
                && ip != s.ip()
            {
                return Err(io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    format!("source {ip} is not the smoltcp stack's address {}", s.ip()),
                ));
            }
            s.tcp_connect(a)
        })
        .map_err(Error::Io)?;
        Ok(SmolTcp {
            fd,
            _thread_bound: PhantomData,
        })
    }

    fn connect_finished(sock: &SmolTcp) -> Result<bool> {
        with(|s| s.connect_finished(sock.fd)).map_err(Error::Io)
    }

    fn udp_bind(local: SocketAddr, sndbuf: Option<u32>) -> Result<SmolUdp> {
        let _ = sndbuf;
        SmolUdp::open(v4(local, None)?.port(), None)
    }
}

/// A poller: an id on this thread's stack. Dropping it closes the sockets
/// still registered with it.
pub struct SmolPoller {
    id: u32,
}

impl Drop for SmolPoller {
    fn drop(&mut self) {
        let id = self.id;
        let _ = with_opt(|s| s.drop_poller(id));
    }
}

/// A receive's target: the buffer armed for it, and what the read found.
pub struct SmolRx {
    ptr: *mut u8,
    len: usize,
    truncated: bool,
    stamp: i64,
}

/// The armed buffer of `slot`.
#[allow(unsafe_code)]
const fn armed(slot: &mut SmolRx) -> &mut [u8] {
    if slot.ptr.is_null() {
        return &mut [];
    }
    // SAFETY: `NetStack::arm`'s contract: the buffer passed to `arm` stays
    // put and unborrowed until the receive that follows returns, and the
    // driver arms right before each receive (`recv_one`).
    unsafe { std::slice::from_raw_parts_mut(slot.ptr, slot.len) }
}

impl NetStack for Smol {
    type Poller = SmolPoller;
    type Event = u64;
    type RxSlot = SmolRx;

    fn poller() -> io::Result<SmolPoller> {
        with(|s| Ok(SmolPoller { id: s.new_poller() }))
    }

    #[inline]
    fn empty_event() -> u64 {
        0
    }

    #[inline]
    fn token(ev: &u64) -> u64 {
        *ev
    }

    fn register(poller: &SmolPoller, fd: libc::c_int, token: u64) -> io::Result<()> {
        with(|s| s.register(fd, poller.id, token))
    }

    fn deregister(poller: &SmolPoller, fd: libc::c_int) {
        let _ = poller;
        let _ = with_opt(|s| s.release(fd));
    }

    #[inline]
    fn poll_now(poller: &SmolPoller, events: &mut [u64]) -> io::Result<usize> {
        with(|s| Ok(s.ready(poller.id, events)))
    }

    fn rx_slot() -> Box<SmolRx> {
        Box::new(SmolRx {
            ptr: std::ptr::null_mut(),
            len: 0,
            truncated: false,
            stamp: 0,
        })
    }

    #[inline]
    fn arm(slot: &mut SmolRx, buf: &mut [u8]) {
        slot.ptr = buf.as_mut_ptr();
        slot.len = buf.len();
    }

    #[inline]
    fn recv(fd: libc::c_int, slot: &mut SmolRx) -> io::Result<Option<usize>> {
        let (mut truncated, mut stamp) = (false, 0);
        let r = with(|s| s.recv(fd, armed(slot), &mut truncated, &mut stamp));
        slot.truncated = truncated;
        slot.stamp = stamp;
        // Disarmed: the buffer is the endpoint's again.
        slot.ptr = std::ptr::null_mut();
        r
    }

    #[inline]
    fn truncated(slot: &SmolRx) -> bool {
        slot.truncated
    }

    #[inline]
    fn rx_meta(slot: &SmolRx, read_seq: u64) -> crate::RxMeta {
        crate::RxMeta {
            rx_sw_ns: slot.stamp,
            read_seq,
            ..crate::RxMeta::default()
        }
    }

    #[inline]
    fn send(fd: libc::c_int, buf: &[u8]) -> io::Result<usize> {
        with(|s| s.send(fd, buf))
    }

    #[inline]
    fn send_datagram(fd: libc::c_int, buf: &[u8]) -> io::Result<bool> {
        with(|s| s.send_datagram(fd, buf))
    }

    #[inline]
    fn refused(e: &io::Error) -> bool {
        let _ = e;
        false
    }

    fn enable_rx_timestamps(fd: libc::c_int) -> io::Result<()> {
        with(|s| s.enable_stamps(fd))
    }

    fn set_rcvbuf(fd: libc::c_int, bytes: u32) -> io::Result<()> {
        // The stack's buffers are `StackConfig::tcp_rx_buf` / `udp_buf`.
        let _ = (fd, bytes);
        Ok(())
    }

    fn set_busy_poll(fd: libc::c_int, usec: u32) -> io::Result<()> {
        // The stack polls the NIC every pass.
        let _ = (fd, usec);
        Ok(())
    }

    fn set_tcp_keepalive(
        fd: libc::c_int,
        idle_s: u32,
        interval_s: u32,
        count: u32,
    ) -> io::Result<()> {
        let _ = (interval_s, count);
        with(|s| s.set_keepalive(fd, idle_s))
    }

    fn set_tcp_user_timeout(fd: libc::c_int, ms: u32) -> io::Result<()> {
        with(|s| s.set_user_timeout(fd, ms))
    }

    fn apply_tcp_timers(
        fd: libc::c_int,
        rto_min_us: Option<u32>,
        delack_max_us: Option<u32>,
    ) -> Result<()> {
        if rto_min_us.is_some() {
            return Err(Error::Unsupported(
                "smoltcp-spin: the RTO floor is the stack's (StackConfig::rto_min_us), not tcp_rto_min_us",
            ));
        }
        if let Some(us) = delack_max_us {
            with(|s| s.set_ack_delay(fd, us)).map_err(Error::Io)?;
        }
        Ok(())
    }

    fn napi_id(fd: libc::c_int) -> io::Result<u32> {
        // The stack polls its NIC queue every pass: report a queue, the
        // port's (plus one: 0 means none).
        with(|s| {
            if s.owns(fd) {
                Ok(u32::from(s.port_id) + 1)
            } else {
                Err(no_stack())
            }
        })
    }

    /// A stack pass: one RX burst through smoltcp, its egress, one TX burst.
    #[inline]
    fn run_once() {
        let _ = with_opt(stack::Inner::poll);
    }

    fn tcp_info(fd: libc::c_int) -> io::Result<crate::TcpInfo> {
        let _ = fd;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "smoltcp-spin has no TCP_INFO: see Stack::tcp_stats",
        ))
    }
}
