//! [`Fstack`]: the seam over F-Stack (FreeBSD 15's TCP/IP stack on DPDK,
//! F-Stack v2.0 as `io/benches/fstack/build-fstack.sh` builds it), for
//! `fstack-spin`.
//!
//! | seam call | F-Stack |
//! |---|---|
//! | `run_once` | `ff_run_once`: RX burst, the stack, timers, TX drain |
//! | `poller`, `register`, `poll_now` | `ff_kqueue`, `ff_kevent` `EVFILT_READ`, level-triggered (no `EV_CLEAR`, as `epoll-spin` relies on), zero timeout |
//! | `recv`, `send` | `ff_read`, `ff_write` |
//! | `rx_meta` | no stack timestamps: only the read sequence (and `rx_read_ns`) |
//! | `set_rcvbuf`, `set_tcp_keepalive` | `ff_setsockopt` (Linux names, which F-Stack translates) |
//! | `set_busy_poll`, `enable_rx_timestamps` | nothing: F-Stack always polls its queue, and has no timestamps |
//! | `set_tcp_user_timeout`, `apply_tcp_timers` | refused: FreeBSD has neither per socket (`Config::validate` refuses the keys first) |
//! | `napi_id` | 0 |
//! | `tcp_info` | `ff_getsockopt_freebsd`, FreeBSD's `struct tcp_info` |
//!
//! `errno`: every `ff_*` call reports Linux values (F-Stack's
//! `ff_os_errno`); `SO_ERROR`, read as data, holds FreeBSD's and is mapped
//! ([`super::fstack_conf::bsd_errno_to_linux`]).
//!
//! F-Stack is single-threaded: [`init`] (`ff_init`) binds it to the calling
//! thread, F-Stack's one lcore, which must make every later call. Each call
//! from another thread is refused ([`Error::Unsupported`]); [`FstackTcp`] is
//! neither `Send` nor `Sync`.

use std::convert::Infallible;
use std::ffi::{CString, c_char, c_int};
use std::io;
use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::ThreadId;

use super::fstack_conf::{BsdTcpInfo, FREEBSD_TCP_INFO, TCP_INFO_BUF, bsd_errno_to_linux};
use super::{NetStack, Socket, Sockets, StackId};
use crate::backend::{Backend, Config};
use crate::error::{Error, Result};

mod ffi;

use ffi::Kevent;

/// F-Stack's TCP/IP stack (one per process).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fstack;

/// Whether F-Stack is linked in: the build found `FSTACK_PREFIX` (F-Stack
/// and its DPDK, built by `io/benches/fstack/build-fstack.sh`). Without it
/// the `fstack` feature compiles, and [`init`] refuses.
#[must_use]
pub const fn linked() -> bool {
    cfg!(ergon_fstack_linked)
}

/// What [`init`] started F-Stack with.
#[derive(Debug)]
pub struct FstackInit {
    conf: String,
    path: PathBuf,
    lcore: usize,
    thread: ThreadId,
}

impl FstackInit {
    /// The config file's text, as `ff_init` read it.
    #[must_use]
    pub fn conf(&self) -> &str {
        &self.conf
    }

    /// Where the config file was written.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// F-Stack's lcore: the polling thread's CPU.
    #[must_use]
    pub const fn lcore(&self) -> usize {
        self.lcore
    }
}

static INIT: OnceLock<FstackInit> = OnceLock::new();

/// Set by the first [`init`] to get past the checks: a second thread's
/// meanwhile is refused.
static STARTING: AtomicBool = AtomicBool::new(false);

/// Set while `ff_init` runs: the EAL ends the process (`rte_exit`) on most
/// failures, and the `atexit` hook then says so.
static IN_FF_INIT: AtomicBool = AtomicBool::new(false);

extern "C" fn exit_in_ff_init() {
    if IN_FF_INIT.load(Ordering::Relaxed) {
        eprintln!(
            "ergon-io: F-Stack ended the process inside ff_init (rte_exit): the EAL and F-Stack lines above say why"
        );
    }
}

/// What [`init`] started F-Stack with, once it has.
#[must_use]
pub fn initialised() -> Option<&'static FstackInit> {
    INIT.get()
}

/// Starts F-Stack on the calling thread, once per process.
///
/// The thread becomes F-Stack's one lcore and must make every later
/// F-Stack call (the driver's polling thread). Writes the config file rendered from `cfg.fstack` (and the lcore) and
/// calls `ff_init`. Once per process: a second call on the same thread
/// with the same settings returns the first's record; anything else is
/// refused. `Driver::new` calls it for `fstack-spin` when it has not run.
///
/// The lcore is `cfg.loop_cpu`, else the one CPU the thread is pinned to.
/// The EAL's control threads take `cfg.helper_cpus` (the thread's affinity
/// is widened to them for the call; the EAL pins the thread back).
///
/// The EAL ends the process on many failures (no hugepages, no port,
/// missing privileges) instead of returning; its own lines say why.
///
/// # Errors
/// [`Error::Unsupported`]: F-Stack is not linked in ([`linked`]).
/// [`Error::Config`]: `cfg` is not `fstack-spin`'s or fails
/// `Config::validate` / `FstackConf::check`; no lcore (unpinned thread and
/// no `loop_cpu`); F-Stack already started with other settings or on
/// another thread. [`Error::Io`]: the config file cannot be written, or
/// `ff_init` returned an error.
pub fn init(cfg: &Config) -> Result<&'static FstackInit> {
    if !linked() {
        return Err(Error::Unsupported(
            "F-Stack is not linked in: build ergon-io's `fstack` feature with FSTACK_PREFIX \
             (io/benches/fstack/build-fstack.sh)",
        ));
    }
    if cfg.backend != Backend::FstackSpin {
        return Err(Error::Config(format!(
            "fstack::init takes an fstack-spin Config, not {}",
            cfg.backend
        )));
    }
    cfg.validate()?;
    let lcore = match cfg.loop_cpu {
        Some(c) => usize::try_from(c).map_err(|_| Error::Config(format!("loop_cpu {c}")))?,
        None => pinned_cpu().ok_or_else(|| {
            Error::Config(
                "fstack-spin: set loop_cpu or pin the polling thread to one CPU (F-Stack's lcore)"
                    .into(),
            )
        })?,
    };
    let text = cfg.fstack.render(lcore)?;
    let me = std::thread::current().id();
    if let Some(done) = INIT.get() {
        return if done.conf == text && done.thread == me {
            Ok(done)
        } else {
            Err(Error::Config(
                "F-Stack already runs (ff_init is once per process) with other settings or on \
                 another thread"
                    .into(),
            ))
        };
    }
    if STARTING.swap(true, Ordering::AcqRel) {
        return Err(Error::Config(
            "F-Stack is starting on another thread".into(),
        ));
    }
    let path = cfg.fstack.conf_path.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("ergon-io-fstack-{}.conf", std::process::id()))
    });
    std::fs::write(&path, &text)?;
    // The EAL takes its control threads' CPUs from this thread's affinity,
    // less the lcore: the helper CPUs, the lcore last.
    let helpers: Vec<usize> = cfg
        .helper_cpus
        .iter()
        .filter_map(|&c| usize::try_from(c).ok())
        .filter(|&c| c != lcore)
        .collect();
    if !helpers.is_empty() {
        let mut cpus = helpers;
        cpus.push(lcore);
        crate::sys::set_affinity(&cpus)?;
    }
    let words = [
        "ergon-io".to_owned(),
        format!("--conf={}", path.display()),
        "--proc-type=primary".to_owned(),
        "--proc-id=0".to_owned(),
    ];
    let strings: Vec<CString> = words
        .iter()
        .map(|a| CString::new(a.as_str()))
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| Error::Config("a NUL in the F-Stack config path".into()))?;
    // Kept for the process's life: F-Stack and the EAL may hold argv.
    let strings: &'static [CString] = strings.leak();
    let mut ptrs: Vec<*mut c_char> = strings.iter().map(|a| a.as_ptr().cast_mut()).collect();
    ptrs.push(std::ptr::null_mut());
    let argv: &'static [*mut c_char] = ptrs.leak();
    IN_FF_INIT.store(true, Ordering::Relaxed);
    register_atexit();
    let rc = ffi::init(argv);
    IN_FF_INIT.store(false, Ordering::Relaxed);
    if rc != 0 {
        return Err(Error::Io(io::Error::other(format!(
            "ff_init returned {rc} (the EAL and F-Stack lines above say why)"
        ))));
    }
    let _ = INIT.set(FstackInit {
        conf: text,
        path,
        lcore,
        thread: me,
    });
    INIT.get()
        .ok_or_else(|| Error::Config("F-Stack's start was not recorded".into()))
}

/// `atexit(exit_in_ff_init)`, once.
fn register_atexit() {
    static DONE: AtomicBool = AtomicBool::new(false);
    if !DONE.swap(true, Ordering::Relaxed) {
        ffi_atexit();
    }
}

#[allow(unsafe_code)] // libc's atexit
fn ffi_atexit() {
    // SAFETY: registers a plain extern "C" fn that only reads an atomic and
    // writes to stderr.
    let _ = unsafe { libc::atexit(exit_in_ff_init) };
}

/// The one CPU the calling thread is pinned to, if it is pinned to one.
fn pinned_cpu() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/thread-self/status").ok()?;
    let list = status
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))?;
    match crate::sys::parse_cpu_list(list).as_slice() {
        &[c] => Some(c),
        _ => None,
    }
}

/// `Ok` on F-Stack's thread once it runs.
///
/// # Errors
/// [`Error::Unsupported`]: F-Stack has not started, or this is another
/// thread.
pub fn on_lcore() -> Result<()> {
    match INIT.get() {
        Some(i) if i.thread == std::thread::current().id() => Ok(()),
        Some(_) => Err(Error::Unsupported(
            "F-Stack runs on the thread that started it (fstack::init): call it there",
        )),
        None => Err(Error::Unsupported(
            "F-Stack has not started: fstack::init (or Driver::new with fstack-spin) first",
        )),
    }
}

/// One F-Stack pass on its thread (test lever; the driver runs its own).
/// Elsewhere, or before [`init`], nothing.
pub fn run_once() {
    if on_lcore().is_ok() {
        ffi::run_once();
    }
}

fn errno() -> io::Error {
    io::Error::last_os_error()
}

fn check(rc: c_int) -> io::Result<()> {
    if rc < 0 { Err(errno()) } else { Ok(()) }
}

fn sockaddr(a: SocketAddrV4) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_family: libc::sa_family_t::try_from(libc::AF_INET).unwrap_or(2),
        sin_port: a.port().to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from(*a.ip()).to_be(),
        },
        sin_zero: [0; 8],
    }
}

const fn v4(addr: SocketAddr) -> Result<SocketAddrV4> {
    match addr {
        SocketAddr::V4(a) => Ok(a),
        SocketAddr::V6(_) => Err(Error::Unsupported(
            "fstack-spin is IPv4 only (FstackConf::addr)",
        )),
    }
}

fn set_int(fd: c_int, level: c_int, opt: c_int, v: c_int) -> io::Result<()> {
    check(ffi::setsockopt_int(fd, level, opt, v))
}

fn to_int(v: u32, what: &str) -> io::Result<c_int> {
    c_int::try_from(v)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{what} {v}")))
}

/// A TCP socket of F-Stack's.
///
/// What an F-Stack endpoint holds (`FrameConn<FstackTcp>`,
/// `WsConn<FstackTcp>`, `HttpConn<FstackTcp>`). Its handle is F-Stack's, not a kernel descriptor; dropping it closes it
/// with `ff_close`. Neither `Send` nor `Sync`: F-Stack runs on one thread.
#[derive(Debug)]
pub struct FstackTcp {
    fd: c_int,
    _thread_bound: PhantomData<*const ()>,
}

impl FstackTcp {
    const fn from_fd(fd: c_int) -> Self {
        Self {
            fd,
            _thread_bound: PhantomData,
        }
    }

    /// A new non-blocking TCP socket with `TCP_NODELAY`.
    fn new_socket() -> Result<Self> {
        on_lcore()?;
        let fd = ffi::socket_tcp();
        if fd < 0 {
            return Err(errno().into());
        }
        let s = Self::from_fd(fd);
        check(ffi::set_nonblocking(fd))?;
        set_int(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1)?;
        Ok(s)
    }

    /// The socket's handle (F-Stack's).
    #[must_use]
    pub const fn raw(&self) -> RawFd {
        self.fd
    }
}

impl Drop for FstackTcp {
    fn drop(&mut self) {
        let _ = ffi::close(self.fd);
    }
}

impl io::Read for FstackTcp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = ffi::read(self.fd, buf);
        usize::try_from(n).map_err(|_| errno())
    }
}

impl io::Write for FstackTcp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = ffi::write(self.fd, buf);
        usize::try_from(n).map_err(|_| errno())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Socket for FstackTcp {
    const STACK: StackId = StackId::Fstack;

    #[inline]
    fn fd(&self) -> RawFd {
        self.fd
    }

    fn start_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<Self> {
        Fstack::tcp_connect(addr, source)
    }

    fn connect_finished(&self) -> Result<bool> {
        Fstack::connect_finished(self)
    }

    fn configure(&self, sndbuf: Option<u32>) -> Result<()> {
        on_lcore()?;
        check(ffi::set_nonblocking(self.fd))?;
        set_int(self.fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1)?;
        if let Some(b) = sndbuf {
            let b = c_int::try_from(b)
                .map_err(|_| Error::Config(format!("sndbuf {b} does not fit this platform")))?;
            set_int(self.fd, libc::SOL_SOCKET, libc::SO_SNDBUF, b)?;
        }
        Ok(())
    }

    fn pump() {
        run_once();
    }
}

impl Sockets for Fstack {
    type Tcp = FstackTcp;
    type Udp = Infallible;

    fn tcp_connect(addr: SocketAddr, source: Option<IpAddr>) -> Result<FstackTcp> {
        let to = v4(addr)?;
        let s = FstackTcp::new_socket()?;
        if let Some(ip) = source {
            let IpAddr::V4(ip) = ip else {
                return Err(Error::Config(format!(
                    "source {ip} is not of the destination {addr}'s address family"
                )));
            };
            check(ffi::bind(s.fd, &sockaddr(SocketAddrV4::new(ip, 0))))?;
        }
        if ffi::connect(s.fd, &sockaddr(to)) < 0 {
            let e = errno();
            if e.raw_os_error() != Some(libc::EINPROGRESS) && e.kind() != io::ErrorKind::WouldBlock
            {
                return Err(e.into());
            }
        }
        Ok(s)
    }

    fn connect_finished(sock: &FstackTcp) -> Result<bool> {
        let mut so_error: c_int = 0;
        check(ffi::getsockopt_int(
            sock.fd,
            libc::SOL_SOCKET,
            libc::SO_ERROR,
            &mut so_error,
        ))?;
        if so_error != 0 {
            // FreeBSD's number, as data: F-Stack does not translate it.
            return Err(io::Error::from_raw_os_error(bsd_errno_to_linux(so_error)).into());
        }
        if ffi::getpeername(sock.fd) == 0 {
            return Ok(true);
        }
        let e = errno();
        if e.raw_os_error() == Some(libc::ENOTCONN) {
            Ok(false)
        } else {
            Err(e.into())
        }
    }

    fn udp_bind(_: SocketAddr, _: Option<u32>) -> Result<Infallible> {
        Err(Error::Unsupported("fstack-spin carries TCP only"))
    }
}

/// An F-Stack kqueue; dropping it closes it.
pub struct Kqueue(c_int);

impl Drop for Kqueue {
    fn drop(&mut self) {
        let _ = ffi::close(self.0);
    }
}

/// The armed receive buffer: where the next `ff_read` writes.
pub struct RxSlot {
    ptr: *mut u8,
    len: usize,
}

impl NetStack for Fstack {
    type Poller = Kqueue;
    type Event = Kevent;
    type RxSlot = RxSlot;

    fn poller() -> io::Result<Kqueue> {
        on_lcore().map_err(io::Error::other)?;
        let kq = ffi::kqueue();
        if kq < 0 {
            return Err(errno());
        }
        Ok(Kqueue(kq))
    }

    #[inline]
    fn empty_event() -> Kevent {
        Kevent::EMPTY
    }

    #[inline]
    fn token(ev: &Kevent) -> u64 {
        ev.udata.addr() as u64
    }

    fn register(poller: &Kqueue, fd: c_int, token: u64) -> io::Result<()> {
        let change = Kevent {
            ident: usize::try_from(fd).map_err(|_| io::Error::from_raw_os_error(libc::EBADF))?,
            filter: ffi::EVFILT_READ,
            flags: ffi::EV_ADD,
            udata: std::ptr::without_provenance_mut(usize::try_from(token).unwrap_or(usize::MAX)),
            ..Kevent::EMPTY
        };
        check(ffi::kevent(poller.0, &[change], &mut []))
    }

    fn deregister(poller: &Kqueue, fd: c_int) {
        let Ok(ident) = usize::try_from(fd) else {
            return;
        };
        let change = Kevent {
            ident,
            filter: ffi::EVFILT_READ,
            flags: ffi::EV_DELETE,
            ..Kevent::EMPTY
        };
        let _ = ffi::kevent(poller.0, &[change], &mut []);
    }

    #[inline]
    fn poll_now(poller: &Kqueue, events: &mut [Kevent]) -> io::Result<usize> {
        let n = ffi::kevent(poller.0, &[], events);
        if n < 0 {
            let e = errno();
            return if e.kind() == io::ErrorKind::Interrupted {
                Ok(0)
            } else {
                Err(e)
            };
        }
        Ok(usize::try_from(n).unwrap_or(0))
    }

    fn rx_slot() -> Box<RxSlot> {
        Box::new(RxSlot {
            ptr: std::ptr::null_mut(),
            len: 0,
        })
    }

    #[inline]
    fn arm(slot: &mut RxSlot, buf: &mut [u8]) {
        slot.ptr = buf.as_mut_ptr();
        slot.len = buf.len();
    }

    #[inline]
    fn recv(fd: c_int, slot: &mut RxSlot) -> io::Result<Option<usize>> {
        let n = ffi::read_raw(fd, slot.ptr, slot.len);
        if let Ok(n) = usize::try_from(n) {
            return Ok(Some(n));
        }
        let e = errno();
        if e.kind() == io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(e)
        }
    }

    #[inline]
    fn truncated(_: &RxSlot) -> bool {
        false
    }

    #[inline]
    fn rx_meta(_: &RxSlot, read_seq: u64) -> crate::RxMeta {
        crate::RxMeta {
            read_seq,
            ..crate::RxMeta::default()
        }
    }

    #[inline]
    fn send(fd: c_int, buf: &[u8]) -> io::Result<usize> {
        let n = ffi::write(fd, buf);
        if let Ok(n) = usize::try_from(n) {
            return Ok(n);
        }
        let e = errno();
        if e.kind() == io::ErrorKind::WouldBlock {
            Ok(0)
        } else {
            Err(e)
        }
    }

    fn send_datagram(_: c_int, _: &[u8]) -> io::Result<bool> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    #[inline]
    fn refused(e: &io::Error) -> bool {
        e.raw_os_error() == Some(libc::ECONNREFUSED)
    }

    fn enable_rx_timestamps(_: c_int) -> io::Result<()> {
        Ok(())
    }

    fn set_rcvbuf(fd: c_int, bytes: u32) -> io::Result<()> {
        set_int(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVBUF,
            to_int(bytes, "rcvbuf")?,
        )
    }

    fn set_busy_poll(_: c_int, _: u32) -> io::Result<()> {
        Ok(())
    }

    fn set_tcp_keepalive(fd: c_int, idle_s: u32, interval_s: u32, count: u32) -> io::Result<()> {
        set_int(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE, 1)?;
        set_int(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPIDLE,
            to_int(idle_s, "keepalive idle")?,
        )?;
        set_int(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPINTVL,
            to_int(interval_s, "keepalive interval")?,
        )?;
        set_int(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_KEEPCNT,
            to_int(count, "keepalive count")?,
        )
    }

    fn set_tcp_user_timeout(_: c_int, _: u32) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "FreeBSD has no TCP_USER_TIMEOUT",
        ))
    }

    fn apply_tcp_timers(
        _: c_int,
        rto_min_us: Option<u32>,
        delack_max_us: Option<u32>,
    ) -> Result<()> {
        if rto_min_us.is_some() || delack_max_us.is_some() {
            return Err(Error::Unsupported(
                "FreeBSD has no per-socket TCP_RTO_MIN_US or TCP_DELACK_MAX_US",
            ));
        }
        Ok(())
    }

    fn napi_id(_: c_int) -> io::Result<u32> {
        Ok(0)
    }

    fn tcp_info(fd: c_int) -> io::Result<crate::TcpInfo> {
        let mut buf = [0u8; TCP_INFO_BUF];
        let (rc, n) = ffi::getsockopt_freebsd(fd, libc::IPPROTO_TCP, FREEBSD_TCP_INFO, &mut buf);
        check(rc)?;
        BsdTcpInfo::parse(&buf[..n])
            .map(BsdTcpInfo::to_tcp_info)
            .ok_or_else(|| io::Error::other(format!("FreeBSD tcp_info: {n} bytes")))
    }

    #[inline]
    fn run_once() {
        ffi::run_once();
    }
}

/// FreeBSD's `struct tcp_info` of an F-Stack socket (tests and checks: its
/// state, options and MSS, which ergon-io's [`crate::TcpInfo`] leaves out).
///
/// # Errors
/// The query fails.
pub fn bsd_tcp_info(sock: &FstackTcp) -> io::Result<BsdTcpInfo> {
    let mut buf = [0u8; TCP_INFO_BUF];
    let (rc, n) = ffi::getsockopt_freebsd(sock.fd, libc::IPPROTO_TCP, FREEBSD_TCP_INFO, &mut buf);
    check(rc)?;
    BsdTcpInfo::parse(&buf[..n]).ok_or_else(|| io::Error::other("FreeBSD tcp_info: short"))
}

/// A listening F-Stack socket (test lever: ergon-io itself only connects),
/// for an F-Stack peer. Dropping it closes it.
#[derive(Debug)]
pub struct FstackListener {
    fd: c_int,
    _thread_bound: PhantomData<*const ()>,
}

impl FstackListener {
    /// Listens on `at`, non-blocking.
    ///
    /// # Errors
    /// Not F-Stack's thread, or a call fails.
    pub fn bind(at: SocketAddrV4) -> Result<Self> {
        let s = FstackTcp::new_socket()?;
        set_int(s.fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, 1)?;
        check(ffi::bind(s.fd, &sockaddr(at)))?;
        check(ffi::listen(s.fd, 128))?;
        let fd = s.fd;
        std::mem::forget(s);
        Ok(Self {
            fd,
            _thread_bound: PhantomData,
        })
    }

    /// The next connection, non-blocking, `TCP_NODELAY` set; `None` when
    /// none is waiting.
    ///
    /// # Errors
    /// The accept fails.
    pub fn accept(&self) -> Result<Option<FstackTcp>> {
        let fd = ffi::accept(self.fd);
        if fd < 0 {
            let e = errno();
            return if e.kind() == io::ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(e.into())
            };
        }
        let s = FstackTcp::from_fd(fd);
        s.configure(None)?;
        Ok(Some(s))
    }
}

impl Drop for FstackListener {
    fn drop(&mut self) {
        let _ = ffi::close(self.fd);
    }
}
