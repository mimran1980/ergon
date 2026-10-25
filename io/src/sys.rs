//! Linux socket options and `recvmsg` / `send` with `SO_TIMESTAMPING` control
//! messages.
#![allow(unsafe_code)]

use std::io;
use std::net::SocketAddr;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::time::Duration;

use crate::RxMeta;

/// Control-message buffer size: room for one `scm_timestamping` (3 timespecs).
pub const CTRL_LEN: usize = 128;

/// Requests software and (where the NIC supports it) hardware receive timestamps.
///
/// # Errors
/// `setsockopt` fails.
pub fn enable_rx_timestamps(fd: RawFd) -> io::Result<()> {
    let flags = libc::SOF_TIMESTAMPING_RX_SOFTWARE
        | libc::SOF_TIMESTAMPING_SOFTWARE
        | libc::SOF_TIMESTAMPING_RX_HARDWARE
        | libc::SOF_TIMESTAMPING_RAW_HARDWARE;
    setsockopt_int(fd, libc::SOL_SOCKET, libc::SO_TIMESTAMPING, to_int(flags)?)
}

/// Sets `SO_BUSY_POLL` (microseconds; no privilege needed).
///
/// # Errors
/// `setsockopt` fails.
pub fn set_busy_poll(fd: RawFd, usec: u32) -> io::Result<()> {
    setsockopt_int(fd, libc::SOL_SOCKET, libc::SO_BUSY_POLL, to_int(usec)?)
}

const SO_PREFER_BUSY_POLL: libc::c_int = 69;

/// A socket's busy-poll settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusyPoll {
    /// `SO_BUSY_POLL`, microseconds.
    pub usec: u32,
    /// `SO_PREFER_BUSY_POLL`.
    pub prefer: bool,
}

/// Reads `SO_BUSY_POLL` and `SO_PREFER_BUSY_POLL`.
///
/// # Errors
/// `getsockopt` fails.
pub fn busy_poll_settings(fd: RawFd) -> io::Result<BusyPoll> {
    let get = |opt| getsockopt_int(fd, libc::SOL_SOCKET, opt);
    Ok(BusyPoll {
        usec: u32::try_from(get(libc::SO_BUSY_POLL)?)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?,
        prefer: get(SO_PREFER_BUSY_POLL)? != 0,
    })
}

/// For a TCP socket (`SO_PROTOCOL`): sets `TCP_NODELAY` and says whether
/// `TCP_CORK` is set. `Ok(None)` for a socket that is not TCP. Three system
/// calls.
///
/// # Errors
/// A `getsockopt` or the `setsockopt` fails.
pub fn tcp_nodelay_and_cork(fd: RawFd) -> io::Result<Option<bool>> {
    // SAFETY: the caller's socket stays open for the duration of the call;
    // the borrow does not outlive it.
    let sock = unsafe { BorrowedFd::borrow_raw(fd) };
    let r = socket2::SockRef::from(&sock);
    if r.protocol()? != Some(socket2::Protocol::TCP) {
        return Ok(None);
    }
    r.set_tcp_nodelay(true)?;
    Ok(Some(r.tcp_cork()?))
}

/// A socket's TCP keepalive settings and `TCP_USER_TIMEOUT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpKeepalive {
    /// `SO_KEEPALIVE`.
    pub enabled: bool,
    /// `TCP_KEEPIDLE`, seconds.
    pub idle_s: u32,
    /// `TCP_KEEPINTVL`, seconds.
    pub interval_s: u32,
    /// `TCP_KEEPCNT`.
    pub count: u32,
    /// `TCP_USER_TIMEOUT`, ms (0: the kernel default).
    pub user_timeout_ms: u32,
}

/// Turns TCP keepalive on with these probe settings (four `setsockopt`s).
///
/// # Errors
/// A `setsockopt` fails or a value is out of range.
pub fn set_tcp_keepalive(fd: RawFd, idle_s: u32, interval_s: u32, count: u32) -> io::Result<()> {
    setsockopt_int(fd, libc::IPPROTO_TCP, libc::TCP_KEEPIDLE, to_int(idle_s)?)?;
    setsockopt_int(
        fd,
        libc::IPPROTO_TCP,
        libc::TCP_KEEPINTVL,
        to_int(interval_s)?,
    )?;
    setsockopt_int(fd, libc::IPPROTO_TCP, libc::TCP_KEEPCNT, to_int(count)?)?;
    setsockopt_int(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE, 1)
}

/// Sets `TCP_USER_TIMEOUT` (ms; 0 restores the kernel default).
///
/// # Errors
/// `setsockopt` fails or the value is out of range.
pub fn set_tcp_user_timeout(fd: RawFd, ms: u32) -> io::Result<()> {
    setsockopt_int(fd, libc::IPPROTO_TCP, libc::TCP_USER_TIMEOUT, to_int(ms)?)
}

/// A per-socket TCP timer added in Linux 6.15.
///
/// Commits f38805c5d26f and 9552f90835ef. The kernel keeps it in ticks (`usecs_to_jiffies`, rounding
/// up) and takes 2 ticks (`TCP_TIMEOUT_MIN`) to `HZ / 5` ticks (200 ms):
/// outside that, `EINVAL`. An older kernel answers `ENOPROTOOPT`. No
/// capability is needed, so a pod sets it in its own spec.
///
/// User space cannot read `CONFIG_HZ`: `sysconf(_SC_CLK_TCK)` is `USER_HZ`
/// (always 100 on Linux). So [`set_tcp_timer`] reads the value back and
/// refuses one the kernel rounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpTimer {
    /// `TCP_RTO_MIN_US`: the retransmission timeout's floor.
    RtoMin,
    /// `TCP_DELACK_MAX_US`: the longest delayed ACK.
    DelackMax,
}

impl TcpTimer {
    /// The option number, `include/uapi/linux/tcp.h` (Linux 6.15):
    /// `#define TCP_RTO_MIN_US 45`, `#define TCP_DELACK_MAX_US 46`. The libc
    /// crate does not define them.
    const fn opt(self) -> libc::c_int {
        match self {
            Self::RtoMin => 45,
            Self::DelackMax => 46,
        }
    }

    /// The socket option's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RtoMin => "TCP_RTO_MIN_US",
            Self::DelackMax => "TCP_DELACK_MAX_US",
        }
    }

    /// The `Config` key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::RtoMin => "tcp_rto_min_us",
            Self::DelackMax => "tcp_delack_max_us",
        }
    }

    const fn needs(self) -> &'static str {
        match self {
            Self::RtoMin => "TCP_RTO_MIN_US (tcp_rto_min_us) needs Linux 6.15 or newer",
            Self::DelackMax => "TCP_DELACK_MAX_US (tcp_delack_max_us) needs Linux 6.15 or newer",
        }
    }
}

/// Sets a [`TcpTimer`] to `us` and reads it back.
///
/// # Errors
/// [`crate::Error::Unsupported`] naming the option and Linux 6.15 when the
/// kernel does not know it; [`crate::Error::Config`] when it knows it but
/// refuses `us` (below two of its ticks) or keeps a rounded value (`us` is
/// not whole ticks); [`crate::Error::Io`] for any other failure.
pub fn set_tcp_timer(fd: RawFd, timer: TcpTimer, us: u32) -> crate::Result<()> {
    if let Err(e) = setsockopt_int(fd, libc::IPPROTO_TCP, timer.opt(), to_int(us)?) {
        // A kernel that knows the option can also read it: EINVAL is then
        // the range check, not a missing option.
        let known = getsockopt_int(fd, libc::IPPROTO_TCP, timer.opt()).is_ok();
        return Err(timer_error(timer, us, &e, known).unwrap_or(crate::Error::Io(e)));
    }
    let kept = tcp_timer(fd, timer)?;
    if kept != us {
        return Err(crate::Error::Config(format!(
            "{} {us} µs: this kernel keeps {kept} µs ({} rounds up to its timer tick, \
             CONFIG_HZ); set {kept}",
            timer.key(),
            timer.name()
        )));
    }
    Ok(())
}

/// The typed error for a failed [`set_tcp_timer`], given whether the kernel
/// can read the option (`known`); `None` for an error that is neither.
pub(crate) fn timer_error(
    timer: TcpTimer,
    us: u32,
    e: &io::Error,
    known: bool,
) -> Option<crate::Error> {
    match e.raw_os_error() {
        Some(libc::EINVAL) if known => Some(crate::Error::Config(format!(
            "{} {us} µs: this kernel takes {} from 2 timer ticks (2 ms at CONFIG_HZ=1000, \
             8 ms at 250, 20 ms at 100) to 200 ms",
            timer.key(),
            timer.name()
        ))),
        Some(libc::ENOPROTOOPT | libc::EINVAL) => Some(crate::Error::Unsupported(timer.needs())),
        _ => None,
    }
}

/// Reads a [`TcpTimer`], µs.
///
/// # Errors
/// `getsockopt` fails (`ENOPROTOOPT` before Linux 6.15).
pub fn tcp_timer(fd: RawFd, timer: TcpTimer) -> io::Result<u32> {
    getsockopt_int(fd, libc::IPPROTO_TCP, timer.opt())
        .and_then(|v| u32::try_from(v).map_err(|_| io::Error::from(io::ErrorKind::InvalidData)))
}

/// Sets `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US` on a fresh TCP socket, so
/// a value this kernel refuses or rounds fails at start-up, not at the first
/// connection.
///
/// # Errors
/// As [`set_tcp_timer`]; or the probe socket cannot be created.
pub fn probe_tcp_timers(rto_min_us: Option<u32>, delack_max_us: Option<u32>) -> crate::Result<()> {
    if rto_min_us.is_none() && delack_max_us.is_none() {
        return Ok(());
    }
    let s = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
    apply_tcp_timers(s.as_raw_fd(), rto_min_us, delack_max_us)
}

/// [`set_tcp_timer`] for each timer that is set.
///
/// # Errors
/// As [`set_tcp_timer`].
pub fn apply_tcp_timers(
    fd: RawFd,
    rto_min_us: Option<u32>,
    delack_max_us: Option<u32>,
) -> crate::Result<()> {
    if let Some(us) = rto_min_us {
        set_tcp_timer(fd, TcpTimer::RtoMin, us)?;
    }
    if let Some(us) = delack_max_us {
        set_tcp_timer(fd, TcpTimer::DelackMax, us)?;
    }
    Ok(())
}

/// Offsets into `struct tcp_info` (`include/uapi/linux/tcp.h`): eight `__u8`
/// fields, then `__u32` fields from `tcpi_rto`.
pub(crate) mod tcpi {
    pub const RTO: usize = 8;
    pub const LOST: usize = 32;
    pub const RTT: usize = 68;
    pub const RTTVAR: usize = 72;
    pub const TOTAL_RETRANS: usize = 100;
    /// After four `__u64` and three `__u32` fields (Linux 4.6+).
    pub const MIN_RTT: usize = 148;
}

/// Reads `TCP_INFO`: one `getsockopt`, which takes the socket's lock briefly
/// (`lock_sock_fast`), so a softirq delivering to the socket at that moment
/// waits for it. `at_ns` is left 0 for the caller.
///
/// # Errors
/// `getsockopt` fails, or the kernel's `tcp_info` stops before
/// `tcpi_total_retrans`.
pub fn tcp_info(fd: RawFd) -> io::Result<crate::TcpInfo> {
    let mut b = [0u8; 256];
    let mut len = libc::socklen_t::try_from(b.len()).map_err(io::Error::other)?;
    // SAFETY: `b` and `len` are valid for the call; the kernel writes at
    // most `len` bytes.
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_INFO,
            b.as_mut_ptr().cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    let got = usize::try_from(len).map_err(io::Error::other)?;
    let field = |at: usize| -> Option<u32> {
        let bytes = b.get(at..at + 4).filter(|_| at + 4 <= got)?;
        Some(u32::from_ne_bytes(bytes.try_into().ok()?))
    };
    let short = || io::Error::new(io::ErrorKind::InvalidData, "tcp_info too short");
    Ok(crate::TcpInfo {
        at_ns: 0,
        rtt_us: field(tcpi::RTT).ok_or_else(short)?,
        rttvar_us: field(tcpi::RTTVAR).ok_or_else(short)?,
        // `~0U` until the first RTT sample.
        min_rtt_us: field(tcpi::MIN_RTT).filter(|&v| v != u32::MAX),
        total_retrans: field(tcpi::TOTAL_RETRANS).ok_or_else(short)?,
        lost: field(tcpi::LOST).ok_or_else(short)?,
        rto_us: field(tcpi::RTO).ok_or_else(short)?,
    })
}

/// Reads a socket's keepalive settings and `TCP_USER_TIMEOUT`.
///
/// # Errors
/// A `getsockopt` fails.
pub fn tcp_keepalive(fd: RawFd) -> io::Result<TcpKeepalive> {
    let get = |level, opt| {
        getsockopt_int(fd, level, opt)
            .and_then(|v| u32::try_from(v).map_err(|_| io::Error::from(io::ErrorKind::InvalidData)))
    };
    Ok(TcpKeepalive {
        enabled: get(libc::SOL_SOCKET, libc::SO_KEEPALIVE)? != 0,
        idle_s: get(libc::IPPROTO_TCP, libc::TCP_KEEPIDLE)?,
        interval_s: get(libc::IPPROTO_TCP, libc::TCP_KEEPINTVL)?,
        count: get(libc::IPPROTO_TCP, libc::TCP_KEEPCNT)?,
        user_timeout_ms: get(libc::IPPROTO_TCP, libc::TCP_USER_TIMEOUT)?,
    })
}

/// What the kernel grants for an `SO_RCVBUF` of `bytes`: `getsockopt` on a
/// fresh TCP socket after setting it (twice the size it keeps, capped at
/// `net.core.rmem_max`, socket(7)).
///
/// # Errors
/// The socket cannot be created or an option fails.
pub fn granted_rcvbuf(bytes: u32) -> io::Result<u32> {
    let s = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
    s.set_recv_buffer_size(usize::try_from(bytes).map_err(io::Error::other)?)?;
    u32::try_from(s.recv_buffer_size()?).map_err(io::Error::other)
}

/// Sets `SO_RCVBUF`.
///
/// # Errors
/// `setsockopt` fails.
pub fn set_rcvbuf(fd: RawFd, bytes: u32) -> io::Result<()> {
    setsockopt_int(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, to_int(bytes)?)
}

/// `SO_INCOMING_NAPI_ID`: the NAPI id (NIC queue) of the socket's last packet; 0
/// when none is known yet.
///
/// # Errors
/// `getsockopt` fails.
pub fn incoming_napi_id(fd: RawFd) -> io::Result<u32> {
    const SO_INCOMING_NAPI_ID: libc::c_int = 56;
    getsockopt_int(fd, libc::SOL_SOCKET, SO_INCOMING_NAPI_ID).map(i32::cast_unsigned)
}

/// `SO_INCOMING_CPU`: the CPU that processed the socket's last packet.
///
/// # Errors
/// `getsockopt` fails.
pub fn incoming_cpu(fd: RawFd) -> io::Result<i32> {
    getsockopt_int(fd, libc::SOL_SOCKET, libc::SO_INCOMING_CPU)
}

/// The CPU the calling thread is running on, if known.
#[must_use]
pub fn current_cpu() -> Option<usize> {
    // SAFETY: plain syscall without arguments.
    usize::try_from(unsafe { libc::sched_getcpu() }).ok()
}

/// Parses a kernel CPU list such as `0-3,6,8-9`.
#[must_use]
pub fn parse_cpu_list(s: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for part in s.trim().split(',').filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                    out.extend(a..=b);
                }
            }
            None => {
                if let Ok(c) = part.trim().parse() {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Restricts the calling thread to `cpus` (ignored when empty).
///
/// # Errors
/// `sched_setaffinity` fails.
pub fn set_affinity(cpus: &[usize]) -> io::Result<()> {
    if cpus.is_empty() {
        return Ok(());
    }
    // SAFETY: zeroed cpu_set_t is valid; CPU_SET is bounds-checked below.
    let rc = unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        for &c in cpus.iter().filter(|&&c| c < libc::CPU_SETSIZE as usize) {
            libc::CPU_SET(c, &mut set);
        }
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw const set)
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Starts a detached thread that runs only on `cpus`, from its first
/// instruction.
///
/// The mask is set in the thread attributes, so unlike a thread that moves
/// itself it never runs on its creator's (often isolated, busy-spinning) CPU.
/// An empty `cpus` keeps the creator's affinity. The thread is named `name`
/// (at most 15 bytes are kept); a panic in `f` ends the thread only.
///
/// # Errors
/// The attributes are rejected or the thread cannot be created.
pub fn spawn_pinned(
    name: &str,
    cpus: &[usize],
    f: impl FnOnce() + Send + 'static,
) -> io::Result<()> {
    type Job = Box<dyn FnOnce() + Send>;
    extern "C" fn start(arg: *mut libc::c_void) -> *mut libc::c_void {
        // SAFETY: `arg` is the `Box<Job>` leaked by `spawn_pinned`, taken back
        // exactly once, here.
        let job = unsafe { Box::from_raw(arg.cast::<Job>()) };
        // A panic must not unwind into the C caller.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
        std::ptr::null_mut()
    }
    let check = |rc: libc::c_int| {
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(rc))
        }
    };
    let cname = std::ffi::CString::new(
        name.as_bytes()
            .iter()
            .take(15)
            .copied()
            .collect::<Vec<u8>>(),
    )
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "thread name contains NUL"))?;
    let job: Job = Box::new(move || {
        // SAFETY: PR_SET_NAME reads a NUL-terminated string of at most 16 bytes.
        unsafe { libc::prctl(libc::PR_SET_NAME, cname.as_ptr()) };
        f();
    });
    // SAFETY: the attribute object is initialised before use and destroyed on
    // every path; the cpu set is zeroed and filled within its bounds.
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        check(libc::pthread_attr_init(&raw mut attr))?;
        let result = (|| {
            if !cpus.is_empty() {
                let mut set: libc::cpu_set_t = std::mem::zeroed();
                for &c in cpus.iter().filter(|&&c| c < libc::CPU_SETSIZE as usize) {
                    libc::CPU_SET(c, &mut set);
                }
                check(libc::pthread_attr_setaffinity_np(
                    &raw mut attr,
                    std::mem::size_of::<libc::cpu_set_t>(),
                    &raw const set,
                ))?;
            }
            check(libc::pthread_attr_setdetachstate(
                &raw mut attr,
                libc::PTHREAD_CREATE_DETACHED,
            ))?;
            let arg = Box::into_raw(Box::new(job)).cast::<libc::c_void>();
            let mut tid: libc::pthread_t = 0;
            let rc = libc::pthread_create(&raw mut tid, &raw const attr, start, arg);
            if rc != 0 {
                // The thread never started: take the job back.
                drop(Box::from_raw(arg.cast::<Job>()));
            }
            check(rc)
        })();
        libc::pthread_attr_destroy(&raw mut attr);
        result
    }
}

/// A connected socket's addressing, as the route lookup needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SocketFlow {
    /// Local address.
    pub local: SocketAddr,
    /// Peer address.
    pub peer: SocketAddr,
    /// `SO_BINDTOIFINDEX`: the interface the socket is bound to, 0 when none
    /// (also before Linux 5.0, which cannot report it).
    pub oif: u32,
    /// `SO_MARK`, 0 when none.
    pub mark: u32,
    /// The type of service the kernel routes the connection by: `IP_TOS`
    /// for an IPv4 peer (also over an IPv6 socket), `IPV6_TCLASS` for an
    /// IPv6 one; 0 when unset.
    pub tos: u8,
}

/// Reads a connected socket's addresses, bound interface and mark.
///
/// # Errors
/// The socket has no peer (`ENOTCONN`) or is not an IP socket.
pub(crate) fn socket_flow(fd: RawFd) -> io::Result<SocketFlow> {
    // SAFETY: the caller's socket stays open for the duration of the call;
    // the borrow does not outlive it.
    let sock = unsafe { BorrowedFd::borrow_raw(fd) };
    let r = socket2::SockRef::from(&sock);
    let ip = |a: socket2::SockAddr| {
        a.as_socket()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not an IP socket"))
    };
    let local = ip(r.local_addr()?)?;
    let peer = ip(r.peer_addr()?)?;
    let opt = |name| getsockopt_int(fd, libc::SOL_SOCKET, name).map_or(0, i32::cast_unsigned);
    let v4 = match peer {
        SocketAddr::V4(_) => true,
        SocketAddr::V6(a) => a.ip().to_ipv4_mapped().is_some(),
    };
    let (level, name) = if v4 {
        (libc::IPPROTO_IP, libc::IP_TOS)
    } else {
        (libc::IPPROTO_IPV6, libc::IPV6_TCLASS)
    };
    let tos = getsockopt_int(fd, level, name)
        .ok()
        .and_then(|t| u8::try_from(t).ok())
        .unwrap_or(0);
    Ok(SocketFlow {
        local,
        peer,
        oif: opt(libc::SO_BINDTOIFINDEX),
        mark: opt(libc::SO_MARK),
        tos,
    })
}

/// One request and its reply on a fresh `NETLINK_ROUTE` socket: sends `req`
/// to the kernel and receives the kernel's reply into `reply`, waiting at
/// most `timeout` for each. Returns the reply's length.
///
/// A timeout below a microsecond waits a microsecond: a zero one would be no
/// timeout at all.
///
/// Hidden test lever: the netlink exchange is an implementation detail of
/// [`crate::route::lookup`]; `io/tests/route.rs` uses it to stand in for the
/// kernel.
///
/// # Errors
/// The socket or a system call fails, `TimedOut` when the kernel does not
/// answer in time, `InvalidData` when the reply does not fit `reply` or does
/// not come from the kernel.
#[doc(hidden)]
pub fn rtnetlink_exchange(req: &[u8], reply: &mut [u8], timeout: Duration) -> io::Result<usize> {
    let timed_out = |e: io::Error| {
        if e.kind() == io::ErrorKind::WouldBlock {
            io::Error::from(io::ErrorKind::TimedOut)
        } else {
            e
        }
    };
    // SAFETY: plain syscall.
    let raw = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_RAW | libc::SOCK_CLOEXEC,
            libc::NETLINK_ROUTE,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh descriptor we own; closed when `sock` drops.
    let sock = unsafe { OwnedFd::from_raw_fd(raw) };
    let fd = sock.as_raw_fd();
    // A zero `timeval` disables the timeout: wait at least a microsecond.
    let timeout = timeout.max(Duration::from_micros(1));
    let tv = libc::timeval {
        tv_sec: libc::time_t::try_from(timeout.as_secs()).unwrap_or(libc::time_t::MAX),
        tv_usec: libc::suseconds_t::from(timeout.subsec_micros().min(999_999)),
    };
    for opt in [libc::SO_RCVTIMEO, libc::SO_SNDTIMEO] {
        // SAFETY: `tv` outlives the call and its size is passed.
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                opt,
                (&raw const tv).cast(),
                socklen_of::<libc::timeval>(),
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: all-zero is a valid sockaddr_nl: port 0, the kernel.
    let mut kernel: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    kernel.nl_family = libc::sa_family_t::try_from(libc::AF_NETLINK).unwrap_or_default();
    // SAFETY: `req` and `kernel` are valid for the lengths passed.
    let sent = unsafe {
        libc::sendto(
            fd,
            req.as_ptr().cast(),
            req.len(),
            0,
            (&raw const kernel).cast(),
            socklen_of::<libc::sockaddr_nl>(),
        )
    };
    if usize::try_from(sent).ok() != Some(req.len()) {
        return Err(if sent < 0 {
            timed_out(io::Error::last_os_error())
        } else {
            io::Error::new(io::ErrorKind::WriteZero, "netlink request cut short")
        });
    }
    // The kernel answers inside `sendto`, so the reply is already queued; a
    // signal is the only reason to look twice.
    for _ in 0..3 {
        // SAFETY: as above.
        let mut from: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        let mut from_len = socklen_of::<libc::sockaddr_nl>();
        // SAFETY: `reply`, `from` and `from_len` are valid for the call;
        // MSG_TRUNC only makes the kernel report the full length.
        let n = unsafe {
            libc::recvfrom(
                fd,
                reply.as_mut_ptr().cast(),
                reply.len(),
                libc::MSG_TRUNC,
                (&raw mut from).cast(),
                &raw mut from_len,
            )
        };
        let Ok(n) = usize::try_from(n) else {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(timed_out(e));
        };
        if from.nl_pid != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "netlink reply not from the kernel",
            ));
        }
        if n > reply.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "netlink reply larger than the buffer",
            ));
        }
        return Ok(n);
    }
    Err(io::Error::from(io::ErrorKind::Interrupted))
}

/// The name of interface `index`, if it exists.
#[must_use]
pub(crate) fn if_name(index: u32) -> Option<String> {
    let mut buf = [0u8; libc::IF_NAMESIZE];
    // SAFETY: `buf` holds IF_NAMESIZE bytes, as if_indextoname requires.
    let p = unsafe { libc::if_indextoname(index, buf.as_mut_ptr().cast()) };
    if p.is_null() {
        return None;
    }
    std::ffi::CStr::from_bytes_until_nul(&buf)
        .ok()?
        .to_str()
        .ok()
        .map(str::to_owned)
}

#[allow(clippy::cast_possible_truncation)] // socket address sizes fit
const fn socklen_of<T>() -> libc::socklen_t {
    std::mem::size_of::<T>() as libc::socklen_t
}

fn to_int<T: TryInto<libc::c_int>>(v: T) -> io::Result<libc::c_int> {
    v.try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "socket option out of range"))
}

#[allow(clippy::cast_possible_truncation)] // 4 bytes
const INT_LEN: libc::socklen_t = std::mem::size_of::<libc::c_int>() as libc::socklen_t;

fn getsockopt_int(fd: RawFd, level: libc::c_int, name: libc::c_int) -> io::Result<i32> {
    let mut v: libc::c_int = 0;
    let mut len = INT_LEN;
    // SAFETY: `v` and `len` are valid for the call.
    let rc = unsafe { libc::getsockopt(fd, level, name, (&raw mut v).cast(), &raw mut len) };
    if rc == 0 {
        Ok(v)
    } else {
        Err(io::Error::last_os_error())
    }
}

fn setsockopt_int(
    fd: RawFd,
    level: libc::c_int,
    name: libc::c_int,
    v: libc::c_int,
) -> io::Result<()> {
    // SAFETY: `v` outlives the call and its size is passed.
    let rc = unsafe { libc::setsockopt(fd, level, name, (&raw const v).cast(), INT_LEN) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// A `msghdr` with one iovec and a control buffer at a stable address, as
/// `io_uring` needs (it reads the header after submission).
#[repr(C)]
pub struct MsgSlot {
    /// The header passed to `recvmsg` / `IORING_OP_RECVMSG`.
    pub hdr: libc::msghdr,
    iov: libc::iovec,
    ctrl: [u8; CTRL_LEN],
}

impl MsgSlot {
    /// A boxed slot (stable address).
    #[must_use]
    pub fn new() -> Box<Self> {
        // SAFETY: all-zero is a valid msghdr, iovec and byte array.
        let mut s: Box<Self> = Box::new(unsafe { std::mem::zeroed() });
        s.hdr.msg_iovlen = 1;
        s
    }

    /// Points the slot at `buf` and resets the control length. Call before every
    /// receive.
    pub const fn arm(&mut self, buf: &mut [u8]) {
        self.iov.iov_base = buf.as_mut_ptr().cast();
        self.iov.iov_len = buf.len();
        self.hdr.msg_iov = &raw mut self.iov;
        self.hdr.msg_iovlen = 1;
        self.hdr.msg_control = self.ctrl.as_mut_ptr().cast();
        #[allow(clippy::cast_possible_truncation)]
        {
            self.hdr.msg_controllen = CTRL_LEN as _;
        }
        self.hdr.msg_flags = 0;
    }

    /// Software and hardware receive timestamps after a completed receive (0 when
    /// absent).
    #[must_use]
    pub fn timestamps(&self) -> (i64, i64) {
        let mut sw = 0i64;
        let mut hw = 0i64;
        // SAFETY: the kernel filled `msg_controllen` bytes of well-formed cmsgs
        // into `ctrl`; the CMSG_* helpers stay within that length.
        unsafe {
            let mut c = libc::CMSG_FIRSTHDR(&raw const self.hdr);
            while !c.is_null() {
                if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SO_TIMESTAMPING {
                    let ts = libc::CMSG_DATA(c)
                        .cast::<[libc::timespec; 3]>()
                        .read_unaligned();
                    sw = ts_ns(&ts[0]);
                    hw = ts_ns(&ts[2]);
                }
                c = libc::CMSG_NXTHDR(&raw const self.hdr, c);
            }
        }
        (sw, hw)
    }

    /// [`RxMeta`] for a completed receive.
    #[must_use]
    pub fn meta(&self, read_seq: u64) -> RxMeta {
        let (rx_sw_ns, rx_hw_ns) = self.timestamps();
        RxMeta {
            rx_sw_ns,
            rx_hw_ns,
            read_seq,
            ..RxMeta::default()
        }
    }
}

const fn ts_ns(t: &libc::timespec) -> i64 {
    t.tv_sec * 1_000_000_000 + t.tv_nsec
}

/// Non-blocking `recvmsg` through `slot`. `Ok(None)` on `EAGAIN`.
///
/// # Errors
/// Any other `recvmsg` error.
pub fn recvmsg(fd: RawFd, slot: &mut MsgSlot) -> io::Result<Option<usize>> {
    // SAFETY: `slot.hdr` was armed with buffers that outlive the call.
    let n = unsafe { libc::recvmsg(fd, &raw mut slot.hdr, libc::MSG_DONTWAIT) };
    if let Ok(n) = usize::try_from(n) {
        return Ok(Some(n));
    }
    let e = io::Error::last_os_error();
    if e.raw_os_error() == Some(libc::EAGAIN) {
        Ok(None)
    } else {
        Err(e)
    }
}

/// Non-blocking `send`. Returns bytes sent (0 on `EAGAIN`).
///
/// # Errors
/// Any other `send` error.
pub fn send(fd: RawFd, buf: &[u8]) -> io::Result<usize> {
    // SAFETY: `buf` is valid for `buf.len()` bytes.
    let n = unsafe {
        libc::send(
            fd,
            buf.as_ptr().cast(),
            buf.len(),
            libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
        )
    };
    if let Ok(n) = usize::try_from(n) {
        return Ok(n);
    }
    let e = io::Error::last_os_error();
    if e.raw_os_error() == Some(libc::EAGAIN) {
        Ok(0)
    } else {
        Err(e)
    }
}
