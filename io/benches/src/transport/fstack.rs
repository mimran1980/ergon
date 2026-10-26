//! TCP over F-Stack (FreeBSD 15's stack on DPDK, F-Stack v2.0).
//!
//! One F-Stack per process, on the end's one `--cpus` lcore. `fstack_lat`'s
//! arm, run by [`super::sock::run`] exactly as `tcp_lat`'s mio and ergon-io
//! arms are: the same point (pacing, recording, echo, files), the same
//! `ergon_latency_core::frame` u32 LE frames, the same stream-index
//! handshake.
//!
//! - The loop: the bench's own. `ff_run_once()` (io/benches/fstack's
//!   `ff-run-once.patch`) is one pass of the loop `ff_run` would own:
//!   timers, TX drain, RX burst and the stack, HPTS. [`Rx::poll`] runs a
//!   pass, reads what it made readable (`ff_kevent`, edge-triggered, then
//!   `ff_read` to `EAGAIN`, at most [`POLL_LIMIT`] frames a stream), and
//!   runs a second pass when it wrote (an echo), so a frame written goes to
//!   the NIC in the same poll, as a kernel `write` sends at once. A ping's
//!   send goes out at the next poll's first pass.
//! - Send ([`FstackTx`]): the frame is queued on the connection's
//!   preallocated send queue and `ff_write` at once; what F-Stack has no
//!   room for goes on the next poll. A full queue is busy (as mio's).
//! - Set-up (`TCP_NODELAY`, `SO_RCVBUF`, `SO_SNDBUF`) by `ff_setsockopt`,
//!   read back by `ff_getsockopt`; FreeBSD's `TCP_INFO` (retransmits, RTT,
//!   MSS) by `ff_getsockopt_freebsd`.
//! - Allocation: F-Stack's buffers are DPDK's and FreeBSD's (not Rust's
//!   allocator), so the row's `hot_path_allocs` counts the bench's own only.
//!
//! `ff_init` runs on the end's thread, already pinned to its CPU: the EAL
//! makes that thread the lcore (`lcore_mask` is that CPU alone). Its
//! control threads (interrupts, IPC) take the process's other CPUs: the
//! thread's affinity is widened to them for `ff_init` (DPDK computes their
//! set from it), then the EAL pins the thread back.
#![allow(unsafe_code)] // F-Stack's C API
#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::cell::RefCell;
use std::ffi::{CString, c_char, c_int, c_ulong, c_void};
use std::net::SocketAddrV4;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ergon_latency_core::frame::{self, Deframer, FRAME_HEADER};
use ergon_latency_core::message::MESSAGE_LEN;

use super::fstack_conf::{
    BsdTcpInfo, FREEBSD_TCP_INFO, FSTACK_USAGE, FstackArgs, FstackConf, TCP_INFO_BUF,
    TCPI_OPT_TIMESTAMPS, TCPS_ESTABLISHED,
};
use super::mio_tcp::tcp_wire_bytes;
use super::point::{Common, Error, Flags, POLL_LIMIT, Role, Rx, Sent, Tx};
use super::sock::{End, Impl, Inspect};
use super::{MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF};

/// F-Stack's `struct kevent` (`ff_event.h`: FreeBSD 12+'s layout).
#[repr(C)]
#[derive(Clone, Copy)]
struct Kevent {
    ident: usize,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: i64,
    udata: *mut c_void,
    ext: [u64; 4],
}

const EVFILT_READ: i16 = -1;
const EVFILT_WRITE: i16 = -2;
const EV_ADD: u16 = 0x0001;
const EV_DELETE: u16 = 0x0002;
const EV_CLEAR: u16 = 0x0020;
const EV_ERROR: u16 = 0x4000;
const EV_EOF: u16 = 0x8000;
/// Linux's `FIONBIO`, which `ff_ioctl` translates.
const LINUX_FIONBIO: c_ulong = 0x5421;

// libfstack.a (io/benches/build.rs links it with --features fstack)
unsafe extern "C" {
    fn ff_init(argc: c_int, argv: *const *mut c_char) -> c_int;
    fn ff_run_once() -> c_int;
    fn ff_socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn ff_setsockopt(
        s: c_int,
        level: c_int,
        opt: c_int,
        val: *const c_void,
        len: libc::socklen_t,
    ) -> c_int;
    fn ff_getsockopt(
        s: c_int,
        level: c_int,
        opt: c_int,
        val: *mut c_void,
        len: *mut libc::socklen_t,
    ) -> c_int;
    fn ff_getsockopt_freebsd(
        s: c_int,
        level: c_int,
        opt: c_int,
        val: *mut c_void,
        len: *mut libc::socklen_t,
    ) -> c_int;
    fn ff_ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn ff_bind(s: c_int, addr: *const libc::sockaddr, len: libc::socklen_t) -> c_int;
    fn ff_listen(s: c_int, backlog: c_int) -> c_int;
    fn ff_accept(s: c_int, addr: *mut libc::sockaddr, len: *mut libc::socklen_t) -> c_int;
    fn ff_connect(s: c_int, addr: *const libc::sockaddr, len: libc::socklen_t) -> c_int;
    fn ff_close(fd: c_int) -> c_int;
    fn ff_read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn ff_write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn ff_kqueue() -> c_int;
    fn ff_kevent(
        kq: c_int,
        changes: *const Kevent,
        nchanges: c_int,
        events: *mut Kevent,
        nevents: c_int,
        timeout: *const libc::timespec,
    ) -> c_int;
}

/// F-Stack's tag, commit, patches and DPDK (build-fstack.sh's `BUILD`
/// file, read by build.rs).
const FSTACK_BUILD: &str = env!("FSTACK_BUILD");

fn errno() -> std::io::Error {
    std::io::Error::last_os_error()
}

fn fail(what: &str) -> Error {
    Error::Transport(format!("{what}: {}", errno()))
}

fn would_block(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(libc::EAGAIN | libc::EINPROGRESS))
}

/// One F-Stack pass.
#[inline]
fn pass() {
    // SAFETY: ff_init ran on this thread (the only thread that calls in).
    unsafe { ff_run_once() };
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

#[allow(clippy::cast_possible_truncation)] // 16 bytes
const SOCKADDR_LEN: libc::socklen_t = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;

fn set_int(fd: c_int, level: c_int, opt: c_int, v: c_int, what: &str) -> Result<(), Error> {
    // SAFETY: `v` lives for the call and its length is passed.
    let rc = unsafe { ff_setsockopt(fd, level, opt, (&raw const v).cast(), 4) };
    if rc != 0 {
        return Err(fail(what));
    }
    Ok(())
}

fn get_int(fd: c_int, level: c_int, opt: c_int, what: &str) -> Result<c_int, Error> {
    let mut v: c_int = 0;
    let mut len: libc::socklen_t = 4;
    // SAFETY: `v` and `len` live for the call; `len` is `v`'s size.
    let rc = unsafe { ff_getsockopt(fd, level, opt, (&raw mut v).cast(), &raw mut len) };
    if rc != 0 {
        return Err(fail(what));
    }
    Ok(v)
}

fn nonblocking(fd: c_int) -> Result<(), Error> {
    let on: c_int = 1;
    // SAFETY: FIONBIO reads one int through the pointer, which lives for the call.
    if unsafe { ff_ioctl(fd, LINUX_FIONBIO, &raw const on) } != 0 {
        return Err(fail("FIONBIO"));
    }
    Ok(())
}

fn tcp_info(fd: c_int) -> Result<BsdTcpInfo, Error> {
    let mut buf = [0u8; TCP_INFO_BUF];
    let mut len = libc::socklen_t::try_from(TCP_INFO_BUF).unwrap_or(0);
    // SAFETY: `buf` and `len` live for the call; `len` is `buf`'s size.
    let rc = unsafe {
        ff_getsockopt_freebsd(
            fd,
            libc::IPPROTO_TCP,
            FREEBSD_TCP_INFO,
            buf.as_mut_ptr().cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(fail("TCP_INFO"));
    }
    BsdTcpInfo::parse(&buf[..len as usize])
}

/// A new non-blocking TCP socket with the arm's options.
fn socket(conf: &FstackConf) -> Result<c_int, Error> {
    // SAFETY: plain call; the result is checked.
    let fd = unsafe { ff_socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(fail("ff_socket"));
    }
    nonblocking(fd)?;
    set_int(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1, "TCP_NODELAY")?;
    let rcv = c_int::try_from(conf.rcvbuf).unwrap_or(c_int::MAX);
    set_int(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, rcv, "SO_RCVBUF")?;
    if let Some(b) = conf.sndbuf {
        let b = c_int::try_from(b).unwrap_or(c_int::MAX);
        set_int(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, b, "SO_SNDBUF")?;
    }
    Ok(fd)
}

/// Starts F-Stack on this thread from `conf`, written to `path` first.
#[allow(clippy::similar_names)] // args, argv, argc
fn init(conf: &FstackConf, path: &std::path::Path, helpers: &[usize]) -> Result<(), Error> {
    std::fs::write(path, conf.render())
        .map_err(|e| Error::Transport(format!("writing {}: {e}", path.display())))?;
    // the EAL takes its control threads' CPUs from this thread's affinity
    // less the lcore: widen it to the helper CPUs, the lcore last
    if !helpers.is_empty() {
        // SAFETY: a zeroed cpu_set_t is empty; CPU_SET writes below CPU_SETSIZE.
        let rc = unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for &c in helpers.iter().chain(std::iter::once(&conf.lcore)) {
                if c < usize::try_from(libc::CPU_SETSIZE).unwrap_or(0) {
                    libc::CPU_SET(c, &mut set);
                }
            }
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw const set)
        };
        if rc != 0 {
            return Err(fail("sched_setaffinity (helper CPUs)"));
        }
    }
    let args = [
        "fstack_lat".to_owned(),
        format!("--conf={}", path.display()),
        "--proc-type=primary".to_owned(),
        "--proc-id=0".to_owned(),
    ];
    let strings: Vec<CString> = args
        .iter()
        .map(|a| CString::new(a.as_str()))
        .collect::<Result<_, _>>()
        .map_err(|_| Error::Usage("a NUL in the config path".into()))?;
    // kept for the process's life: F-Stack and the EAL may hold argv's strings
    let strings: &'static [CString] = strings.leak();
    let mut argv: Vec<*mut c_char> = strings.iter().map(|a| a.as_ptr().cast_mut()).collect();
    argv.push(std::ptr::null_mut());
    let argc = c_int::try_from(strings.len()).unwrap_or(0);
    // SAFETY: argv is argc NUL-terminated strings, never freed, and a NULL.
    if unsafe { ff_init(argc, argv.as_ptr()) } != 0 {
        return Err(Error::Transport("ff_init failed (its log is above)".into()));
    }
    // the EAL pinned this thread to the lcore; say so if not
    super::point::pin(Some(conf.lcore))
}

/// Accepts `n` connections on a listener at `at`, ordered by the stream
/// index each peer writes first; `ready` is printed once it listens.
fn accept_all(
    conf: &FstackConf,
    at: SocketAddrV4,
    n: u32,
    wait: Duration,
) -> Result<Vec<c_int>, Error> {
    let l = socket(conf)?;
    let sa = sockaddr(at);
    // SAFETY: `sa` is a sockaddr_in of SOCKADDR_LEN bytes, alive for the call.
    if unsafe { ff_bind(l, (&raw const sa).cast(), SOCKADDR_LEN) } != 0 {
        return Err(fail(&format!("bind {at}")));
    }
    // SAFETY: plain call.
    if unsafe { ff_listen(l, 1024) } != 0 {
        return Err(fail("listen"));
    }
    println!("ready");
    let until = Instant::now() + wait;
    let mut slots: Vec<Option<c_int>> = vec![None; n as usize];
    let mut got = 0;
    while got < n {
        pass();
        // SAFETY: a NULL address asks for none.
        let fd = unsafe { ff_accept(l, std::ptr::null_mut(), std::ptr::null_mut()) };
        if fd < 0 {
            let e = errno();
            if !would_block(&e) {
                return Err(Error::Transport(format!("accept: {e}")));
            }
            if Instant::now() > until {
                return Err(Error::Timeout(format!(
                    "{got} of {n} connections came within {} s",
                    wait.as_secs()
                )));
            }
            continue;
        }
        nonblocking(fd)?;
        let mut b = [0u8; 4];
        let mut have = 0;
        while have < 4 {
            pass();
            // SAFETY: reads at most the 4 - have bytes left in `b`.
            let r = unsafe { ff_read(fd, b[have..].as_mut_ptr().cast(), 4 - have) };
            if r > 0 {
                have += r.unsigned_abs();
            } else if r == 0 {
                return Err(Error::Transport(
                    "a peer closed before its stream index".into(),
                ));
            } else if !would_block(&errno()) {
                return Err(fail("reading a stream index"));
            } else if Instant::now() > until {
                return Err(Error::Timeout("a stream index did not come".into()));
            }
        }
        let i = u32::from_le_bytes(b);
        let slot = slots
            .get_mut(i as usize)
            .filter(|s| s.is_none())
            .ok_or_else(|| Error::Transport(format!("stream index {i} of {n}, or twice")))?;
        *slot = Some(fd);
        got += 1;
    }
    // SAFETY: the listener is ours and no longer used.
    unsafe { ff_close(l) };
    Ok(slots.into_iter().flatten().collect())
}

/// Waits (passing) until `fd` is writable; `false` when the connect failed.
fn connected(kq: c_int, fd: c_int, attempt: Instant) -> Result<bool, Error> {
    let change = Kevent {
        ident: usize::try_from(fd).unwrap_or(0),
        filter: EVFILT_WRITE,
        flags: EV_ADD,
        fflags: 0,
        data: 0,
        udata: std::ptr::null_mut(),
        ext: [0; 4],
    };
    let zero = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let mut ev = [change; 1];
    // SAFETY: one change in, room for one event out, all alive for the call.
    if unsafe {
        ff_kevent(
            kq,
            &raw const change,
            1,
            ev.as_mut_ptr(),
            0,
            &raw const zero,
        )
    } < 0
    {
        return Err(fail("kevent (connect)"));
    }
    let done = loop {
        pass();
        // SAFETY: as above, no changes.
        let n = unsafe { ff_kevent(kq, std::ptr::null(), 0, ev.as_mut_ptr(), 1, &raw const zero) };
        if n < 0 {
            return Err(fail("kevent (connect)"));
        }
        if n == 1 {
            break ev[0].flags & (EV_EOF | EV_ERROR) == 0
                && get_int(fd, libc::SOL_SOCKET, libc::SO_ERROR, "SO_ERROR")? == 0;
        }
        if Instant::now() > attempt {
            break false;
        }
    };
    let del = Kevent {
        flags: EV_DELETE,
        ..change
    };
    // SAFETY: as above.
    unsafe { ff_kevent(kq, &raw const del, 1, ev.as_mut_ptr(), 0, &raw const zero) };
    Ok(done)
}

/// Opens `n` connections to `to`, each carrying its stream index first; a
/// refused or unanswered connect is retried within `wait`.
fn connect_all(
    conf: &FstackConf,
    to: SocketAddrV4,
    n: u32,
    wait: Duration,
) -> Result<Vec<c_int>, Error> {
    let until = Instant::now() + wait;
    // SAFETY: plain call.
    let kq = unsafe { ff_kqueue() };
    if kq < 0 {
        return Err(fail("kqueue"));
    }
    let sa = sockaddr(to);
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let fd = loop {
            let fd = socket(conf)?;
            // SAFETY: `sa` is a sockaddr_in of SOCKADDR_LEN bytes.
            let rc = unsafe { ff_connect(fd, (&raw const sa).cast(), SOCKADDR_LEN) };
            let ok = if rc == 0 {
                true
            } else if would_block(&errno()) {
                connected(kq, fd, Instant::now() + Duration::from_secs(1))?
            } else {
                false
            };
            if ok {
                break fd;
            }
            // SAFETY: ours, not used again.
            unsafe { ff_close(fd) };
            if Instant::now() > until {
                return Err(Error::Timeout(format!(
                    "connect {to} from {}: no connection within {} s",
                    conf.addr,
                    wait.as_secs()
                )));
            }
            let pause = Instant::now() + Duration::from_millis(10);
            while Instant::now() < pause {
                pass();
            }
        };
        let b = i.to_le_bytes();
        let mut sent = 0;
        while sent < 4 {
            // SAFETY: writes the 4 - sent bytes left of `b`.
            let w = unsafe { ff_write(fd, b[sent..].as_ptr().cast(), 4 - sent) };
            if w > 0 {
                sent += w.unsigned_abs();
            } else if w < 0 && !would_block(&errno()) {
                return Err(fail("writing the stream index"));
            }
            pass();
        }
        out.push(fd);
    }
    // SAFETY: ours, not used again.
    unsafe { ff_close(kq) };
    Ok(out)
}

/// Counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FstackStats {
    /// Frames received.
    pub frames_in: u64,
    /// Frames queued to send.
    pub frames_out: u64,
    /// Bytes received.
    pub bytes_in: u64,
    /// `ff_read` calls.
    pub reads: u64,
    /// `ff_write` calls.
    pub writes: u64,
    /// Sends refused because the send queue was full.
    pub tx_full: u64,
    /// Writes F-Stack had no room for.
    pub would_block: u64,
    /// `Rx::poll` calls.
    pub polls: u64,
    /// `ff_run_once` passes made by polls.
    pub passes: u64,
}

/// A connection's send queue: bytes `head..tail` F-Stack has not taken.
struct TxQueue {
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
}

impl TxQueue {
    fn new() -> Self {
        // non-zero fill maps every page now
        Self {
            buf: vec![1u8; TCP_TX_BUF].into_boxed_slice(),
            head: 0,
            tail: 0,
        }
    }

    #[inline]
    fn queue(&mut self, payload: &[u8]) -> Result<bool, Error> {
        let need = FRAME_HEADER + payload.len();
        if self.tail + need > self.buf.len() {
            if self.head > 0 {
                self.buf.copy_within(self.head..self.tail, 0);
                self.tail -= self.head;
                self.head = 0;
            }
            if self.tail + need > self.buf.len() {
                return Ok(false);
            }
        }
        self.tail += frame::encode(payload, &mut self.buf[self.tail..])?;
        Ok(true)
    }

    /// Writes until F-Stack has no room; `true` when bytes are left.
    #[inline]
    fn write_out(&mut self, fd: c_int, stats: &mut FstackStats) -> Result<bool, Error> {
        while self.head < self.tail {
            stats.writes += 1;
            let left = &self.buf[self.head..self.tail];
            // SAFETY: writes from `left`, which lives for the call.
            let w = unsafe { ff_write(fd, left.as_ptr().cast(), left.len()) };
            if w > 0 {
                self.head += w.unsigned_abs();
            } else if w == 0 {
                return Err(Error::Transport("ff_write returned 0".into()));
            } else if would_block(&errno()) {
                stats.would_block += 1;
                return Ok(true);
            } else {
                return Err(fail("ff_write"));
            }
        }
        self.head = 0;
        self.tail = 0;
        Ok(false)
    }
}

struct TxState {
    queues: Vec<TxQueue>,
    dirty: Vec<u32>,
    in_dirty: Vec<bool>,
    /// written since the last pass
    wrote: bool,
    stats: FstackStats,
}

/// The connections and the send side's state, shared by both halves.
struct Streams {
    fds: Vec<c_int>,
    tx: RefCell<TxState>,
    conf: FstackConf,
}

impl Streams {
    #[inline]
    fn send(&self, stream: u32, payload: &[u8]) -> Result<Sent, Error> {
        let i = stream as usize;
        let fd = *self
            .fds
            .get(i)
            .ok_or_else(|| Error::Transport(format!("stream {i} of {}", self.fds.len())))?;
        let mut t = self.tx.borrow_mut();
        let t = &mut *t;
        if !t.queues[i].queue(payload)? {
            t.stats.tx_full += 1;
            return Ok(Sent::Busy);
        }
        t.stats.frames_out += 1;
        t.wrote = true;
        if t.queues[i].write_out(fd, &mut t.stats)? && !t.in_dirty[i] {
            t.in_dirty[i] = true;
            t.dirty.push(stream);
        }
        Ok(Sent::Sent)
    }

    #[inline]
    fn flush(&self) -> Result<(), Error> {
        let mut t = self.tx.borrow_mut();
        let t = &mut *t;
        let mut k = 0;
        while let Some(&s) = t.dirty.get(k) {
            let i = s as usize;
            t.wrote = true;
            if t.queues[i].write_out(self.fds[i], &mut t.stats)? {
                k += 1;
            } else {
                t.in_dirty[i] = false;
                t.dirty.swap_remove(k);
            }
        }
        Ok(())
    }
}

/// The sending side.
pub struct FstackTx {
    s: Rc<Streams>,
}

/// The receiving side.
pub struct FstackRx {
    s: Rc<Streams>,
    kq: c_int,
    events: Vec<Kevent>,
    de: Vec<Deframer>,
    closed: Vec<bool>,
    open: usize,
    ready: Vec<u32>,
    in_ready: Vec<bool>,
    stats: FstackStats,
}

impl Tx for FstackTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        self.s.send(stream, msg)
    }
}

impl FstackRx {
    /// Reads `stream` to `EAGAIN` or [`POLL_LIMIT`] frames; `true` when done
    /// (nothing left to read before its next event).
    #[inline]
    fn read<F: FnMut(&[u8])>(
        &mut self,
        stream: u32,
        on: &mut F,
        n: &mut usize,
    ) -> Result<bool, Error> {
        let i = stream as usize;
        let fd = self.s.fds[i];
        let de = &mut self.de[i];
        let mut delivered = 0;
        loop {
            while delivered < POLL_LIMIT {
                match de.next_frame()? {
                    Some(f) => {
                        self.stats.frames_in += 1;
                        delivered += 1;
                        on(f);
                    }
                    None => break,
                }
            }
            if delivered == POLL_LIMIT {
                *n += delivered;
                return Ok(false);
            }
            let spare = de.spare();
            if spare.is_empty() {
                return Err(Error::Transport(format!("stream {i}: receive buffer full")));
            }
            self.stats.reads += 1;
            // SAFETY: reads into `spare`, at most its length.
            let r = unsafe { ff_read(fd, spare.as_mut_ptr().cast(), spare.len()) };
            if r > 0 {
                let len = r.unsigned_abs();
                de.filled(len)?;
                self.stats.bytes_in += len as u64;
                continue;
            }
            let peer_closed = r == 0 || errno().raw_os_error() == Some(libc::ECONNRESET);
            if r < 0 && !peer_closed && !would_block(&errno()) {
                return Err(fail("ff_read"));
            }
            if peer_closed && !self.closed[i] {
                self.closed[i] = true;
                self.open -= 1;
            }
            *n += delivered;
            return Ok(true);
        }
    }
}

impl Rx for FstackRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        self.s.flush()?;
        self.s.tx.borrow_mut().wrote = false;
        pass();
        self.stats.polls += 1;
        self.stats.passes += 1;
        let zero = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let cap = c_int::try_from(self.events.len()).unwrap_or(c_int::MAX);
        // SAFETY: no changes; room for `cap` events, alive for the call.
        let got = unsafe {
            ff_kevent(
                self.kq,
                std::ptr::null(),
                0,
                self.events.as_mut_ptr(),
                cap,
                &raw const zero,
            )
        };
        if got < 0 {
            return Err(fail("kevent"));
        }
        for k in 0..usize::try_from(got).unwrap_or(0) {
            let r = self.events[k].udata.addr();
            if matches!(
                (self.in_ready.get(r), self.closed.get(r)),
                (Some(false), Some(false))
            ) {
                self.in_ready[r] = true;
                self.ready.push(u32::try_from(r).unwrap_or(u32::MAX));
            }
        }
        let mut n = 0;
        let mut k = 0;
        while let Some(&r) = self.ready.get(k) {
            if self.read(r, on, &mut n)? {
                self.in_ready[r as usize] = false;
                self.ready.swap_remove(k);
            } else {
                k += 1;
            }
        }
        self.s.flush()?;
        // an echo written in this poll leaves now, not at the next poll
        if self.s.tx.borrow().wrote {
            pass();
            self.stats.passes += 1;
        }
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.open == 0
    }
}

impl Inspect for FstackRx {
    fn retrans(&self) -> Result<Option<u64>, Error> {
        let mut sum = 0u64;
        for &fd in &self.s.fds {
            sum += u64::from(tcp_info(fd)?.rexmit_packets);
        }
        Ok(Some(sum))
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        let ts = match self.s.fds.first() {
            Some(&fd) => tcp_info(fd)?.options & TCPI_OPT_TIMESTAMPS != 0,
            None => false,
        };
        Ok(tcp_wire_bytes(ts, false))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let t = self.s.tx.borrow().stats;
        let r = self.stats;
        let mut put = |k: &str, v: String| out.push((k.to_owned(), v));
        put("tcp.max_frame", MAX_FRAME.to_string());
        put("tcp.rx_buf", TCP_RX_BUF.to_string());
        put("tcp.tx_buf", TCP_TX_BUF.to_string());
        put("tcp.frames_per_pass", POLL_LIMIT.to_string());
        put("nodelay", "true".to_owned());
        put("rcvbuf", self.s.conf.rcvbuf.to_string());
        put(
            "sndbuf",
            self.s
                .conf
                .sndbuf
                .map_or_else(|| "fstack".to_owned(), |b| b.to_string()),
        );
        for (k, v) in [
            ("stat.frames_in", r.frames_in),
            ("stat.frames_out", t.frames_out),
            ("stat.bytes_in", r.bytes_in),
            ("stat.reads", r.reads),
            ("stat.writes", t.writes),
            ("stat.tx_full", t.tx_full),
            ("stat.would_block", t.would_block),
            ("stat.polls", r.polls),
            ("stat.passes", r.passes),
        ] {
            put(k, v.to_string());
        }
        if let Some(&fd) = self.s.fds.first() {
            let i = tcp_info(fd)?;
            put("tcp_info.state", i.state.to_string());
            put("tcp_info.options", i.options.to_string());
            put("tcp_info.rto_us", i.rto_us.to_string());
            put("tcp_info.snd_mss", i.snd_mss.to_string());
            put("tcp_info.rtt_us", i.rtt_us.to_string());
            put("tcp_info.snd_cwnd", i.snd_cwnd.to_string());
            put("tcp_info.tlp", i.tlp.to_string());
            put("tcp_info.ooo_packets", i.ooo_packets.to_string());
            if i.state != TCPS_ESTABLISHED {
                put("tcp_info.warning", "stream 0 not established".to_owned());
            }
            let nodelay = get_int(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, "TCP_NODELAY")?;
            put("readback.nodelay", (nodelay != 0).to_string());
            for (k, o) in [
                ("readback.rcvbuf", libc::SO_RCVBUF),
                ("readback.sndbuf", libc::SO_SNDBUF),
            ] {
                put(k, get_int(fd, libc::SOL_SOCKET, o, k)?.to_string());
            }
        }
        for line in FSTACK_BUILD.split(';') {
            if let Some((k, v)) = line.split_once('=') {
                put(&format!("fstack.build.{k}"), v.to_owned());
            }
        }
        out.extend(self.s.conf.settings());
        Ok(())
    }
}

/// Takes connected sockets, stream `i` the `i`-th; their two sides, read
/// through a kqueue of their own.
fn pair(fds: Vec<c_int>, conf: FstackConf) -> Result<(FstackTx, FstackRx), Error> {
    let n = fds.len();
    // SAFETY: plain call.
    let kq = unsafe { ff_kqueue() };
    if kq < 0 {
        return Err(fail("kqueue"));
    }
    let changes: Vec<Kevent> = fds
        .iter()
        .enumerate()
        .map(|(i, &fd)| Kevent {
            ident: usize::try_from(fd).unwrap_or(0),
            filter: EVFILT_READ,
            flags: EV_ADD | EV_CLEAR,
            fflags: 0,
            data: 0,
            udata: std::ptr::without_provenance_mut(i),
            ext: [0; 4],
        })
        .collect();
    let zero = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let nc = c_int::try_from(changes.len()).unwrap_or(c_int::MAX);
    // SAFETY: `nc` changes in, none out.
    if unsafe {
        ff_kevent(
            kq,
            changes.as_ptr(),
            nc,
            std::ptr::null_mut(),
            0,
            &raw const zero,
        )
    } < 0
    {
        return Err(fail("kevent (register)"));
    }
    let mut de = Vec::with_capacity(n);
    for _ in 0..n {
        let mut d = Deframer::new(MAX_FRAME)?;
        d.spare().fill(1);
        de.push(d);
    }
    let s = Rc::new(Streams {
        fds,
        tx: RefCell::new(TxState {
            queues: (0..n).map(|_| TxQueue::new()).collect(),
            dirty: Vec::with_capacity(n),
            in_dirty: vec![false; n],
            wrote: false,
            stats: FstackStats::default(),
        }),
        conf,
    });
    // every stream ready once: data that came with the handshake is read
    let ready: Vec<u32> = (0..u32::try_from(n).unwrap_or(0)).collect();
    Ok((
        FstackTx { s: Rc::clone(&s) },
        FstackRx {
            s,
            kq,
            events: vec![
                changes.first().copied().unwrap_or(Kevent {
                    ident: 0,
                    filter: 0,
                    flags: 0,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                    ext: [0; 4],
                });
                n.max(1) * 2
            ],
            de,
            closed: vec![false; n],
            open: n,
            ready,
            in_ready: vec![true; n],
            stats: FstackStats::default(),
        },
    ))
}

/// The process's CPUs less `cpus` (the EAL's control threads go there).
fn helper_cpus(cpus: &[usize]) -> Vec<usize> {
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its
    // size; CPU_ISSET reads within it.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return Vec::new();
        }
        (0..usize::try_from(libc::CPU_SETSIZE).unwrap_or(0))
            .filter(|&c| libc::CPU_ISSET(c, &set) && !cpus.contains(&c))
            .collect()
    }
}

/// `fstack_lat`: parses `args` (without the program name) and runs the
/// point; the row, or `None` for a pong.
///
/// # Errors
/// A flag refused, or what the point met.
pub fn fstack_main<I: IntoIterator<Item = String>>(args: I) -> Result<Option<String>, Error> {
    let mut f = Flags::parse(args)?;
    let c = Common::parse(&mut f)?;
    let a = FstackArgs::parse(&mut f, c.role, &c.cpus)?;
    f.finish()?;
    let helpers = helper_cpus(&c.cpus);
    let path = c.out.join(format!("f-stack-{}.conf", c.role.name()));
    std::fs::create_dir_all(&c.out)
        .map_err(|e| Error::Transport(format!("{}: {e}", c.out.display())))?;
    let opt = |v: Option<SocketAddrV4>| v.map_or_else(|| "-".to_owned(), |a| a.to_string());
    let arm = vec![
        ("listen".to_owned(), opt(a.listen)),
        ("connect".to_owned(), opt(a.connect)),
        ("source".to_owned(), a.conf.addr.to_string()),
        ("fstack.helper_cpus".to_owned(), format!("{helpers:?}")),
    ];
    let (streams, wait) = (c.streams, a.connect_timeout);
    let role = c.role;
    super::sock::run(
        &c,
        Impl::Fstack,
        "tcp",
        &arm,
        role == Role::Pong,
        |end, _cpu| {
            init(&a.conf, &path, &helpers)?;
            let fds = match (end, a.listen, a.connect) {
                (End::Pong, Some(l), _) => accept_all(&a.conf, l, streams, wait)?,
                (End::Ping, _, Some(t)) => connect_all(&a.conf, t, streams, wait)?,
                _ => return Err(Error::Usage(FSTACK_USAGE.into())),
            };
            pair(fds, a.conf.clone())
        },
    )
}
