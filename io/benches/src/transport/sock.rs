//! `udp_lat` and `tcp_lat`: the socket arms' flags, connection set-up,
//! threads and roles.
//!
//! Every measurement, the echo and the files are
//! [`point`](super::point)'s, exactly as `mpsc_lat` and `aeron_lat` use
//! it; an arm supplies only its `Tx` / `Rx` pair ([`super::mio_udp`],
//! [`super::eio_udp`], [`super::mio_tcp`], [`super::eio_tcp`]).
//!
//! | role | busy threads, in `--cpus` order | row |
//! |---|---|---|
//! | `ping` | ping: paces, sends, polls the echoes of every stream | `load` |
//! | `pong` | echo (stops on SIGTERM, or once every TCP peer closed) | none: `manifest.json` alone |
//! | `local --round-trip` | ping, echo | `load` |
//! | `local` | sender, receiver | `oneway` |
//!
//! Each thread pins itself, then builds its end (sockets, buffers, the
//! ergon-io driver with `loop_cpu` set to its CPU), so every page it
//! touches is first touched there. ergon-io's helper thread (its warning
//! thread) goes on the process's other CPUs (`Config::helper_cpus`), never
//! a `--cpus` one. A `pong` prints `ready` once its sockets are bound (or
//! listening).
//!
//! Streams: UDP stream `i` is the socket on `--local` port + `i`,
//! connected to `--peer` port + `i`. TCP stream `i` is the `i`-th
//! connection: the connecting end writes `i` (u32 LE) on it before
//! anything else, and the accepting end orders its connections by it.
//! No stream may use a port in 20000-21999 (the media driver's wildcard
//! ranges in the lab).
//!
//! Each end reads its counters off the hot path: before it builds its
//! sockets, once they are set up (before the first send), and after its
//! traffic. The row's `retrans` covers the traffic only (`retrans_window`:
//! warm-up, measured, drain); set-up is recorded apart. Manifest settings
//! (the other end's prefixed `echo.` in a local round trip):
//!
//! | setting | what |
//! |---|---|
//! | `tcp.retrans`, `tcp.retrans_setup` | `tcpi_total_retrans` growth over the traffic; during set-up (SYN, SYN-ACK) |
//! | `tcp.dsack_dups`, `tcp.dsack_dups_setup` | of those, segments the peer reported (DSACK) it already had: spurious |
//! | `tcp.retrans_not_dsack` | retransmits not reported spurious: a lost segment, or a DSACK not yet back |
//! | `tcp.lost_end` | `tcpi_lost` after the traffic |
//! | `tcp.rtos`, `tcp.rtos_setup` | `tcpi_total_rto` growth (Linux 6.7+, else `unavailable`) |
//! | `netstat.setup.*`, `netstat.window.*` | the namespace's [`kstat::NETSTAT`](super::kstat::NETSTAT) growth (`TCPLossProbes`, `TCPDSACKRecv`, `TCPTimeouts`, `TCPSynRetrans`, `ListenOverflows`, ...): every TCP socket in it |
//! | `sched.run_delay_ns`, `sched.involuntary_switches` | the end's thread over the traffic: time runnable but waiting for a CPU, and preemptions (every arm) |
//!
//! A retransmit that the peer reports a duplicate, on a thread that waited
//! for a CPU, is an ACK that came late (a tail-loss probe fires about 2 ms
//! after a stream's second unacknowledged segment, ahead of a delayed ACK),
//! not loss. On an isolated core `sched.run_delay_ns` stays near 0.
#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
#![allow(clippy::many_single_char_names, clippy::similar_names)] // c, f, s, l, n: the flags, sockets and counts

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::template;

use super::DEFAULT_RCVBUF;
use super::eio_tcp::{self, EioTcpOptions};
use super::eio_udp::{self, EioUdpOptions};
use super::kstat::{Netstat, Sched};
use super::mio_tcp::{self, MioTcpOptions};
use super::mio_udp::{self, MioUdpOptions};
use super::point::{
    self, Common, Error, Flags, POLL_LIMIT, Progress, Recorder, Report, Role, Rx, START_DELAY_NS,
    SendStats, Tx, thread_allocs,
};
use super::sockopt::{TcpCounters, os};

/// What every socket arm reports besides its row: retransmits, wire bytes,
/// settings and counters. Read off the hot path.
pub trait Inspect {
    /// The TCP counters so far, summed over every stream's connection
    /// (`TCP_INFO`: retransmits, DSACK-reported duplicates, RTOs, lost);
    /// `None` where the transport has none (the default).
    ///
    /// # Errors
    /// `getsockopt` fails.
    fn tcp_counters(&self) -> Result<Option<TcpCounters>, Error> {
        Ok(None)
    }

    /// Segments retransmitted so far over every stream (`TCP_INFO`
    /// `tcpi_total_retrans`); `None` where the transport has none. By
    /// default [`Inspect::tcp_counters`]'s.
    ///
    /// # Errors
    /// `getsockopt` fails.
    fn retrans(&self) -> Result<Option<u64>, Error> {
        Ok(self.tcp_counters()?.map(|c| c.retrans))
    }

    /// Bytes on the wire per message: payload, framing, transport, IP and
    /// Ethernet headers.
    ///
    /// # Errors
    /// The headers cannot be read back.
    fn wire_bytes(&self) -> Result<u64, Error>;

    /// The settings as applied (read back from stream 0's socket) and the
    /// arm's counters.
    ///
    /// # Errors
    /// A read-back fails.
    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error>;
}

/// The media driver's wildcard port ranges in the lab (sender 20000-20999,
/// receiver 21000-21999: vm.sh `common_env`): no socket arm binds or sends
/// to one.
pub const DPDK_PORTS: std::ops::RangeInclusive<u16> = 20_000..=21_999;

/// `base` with its port moved up by `i`.
///
/// # Errors
/// [`Error::Usage`] for port 0, a port past 65 535, or one in
/// [`DPDK_PORTS`].
pub fn port_plus(base: SocketAddr, i: u32) -> Result<SocketAddr, Error> {
    if base.port() == 0 {
        return Err(Error::Usage(format!(
            "{base}: a base port is needed (stream i uses port + i)"
        )));
    }
    let port = u16::try_from(u32::from(base.port()) + i)
        .map_err(|_| Error::Usage(format!("{base} + {i}: past port 65535")))?;
    if DPDK_PORTS.contains(&port) {
        return Err(Error::Usage(format!(
            "port {port} (stream {i} of base {base}): 20000-21999 are the media driver's wildcard ranges"
        )));
    }
    Ok(SocketAddr::new(base.ip(), port))
}

/// The technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impl {
    /// Raw mio.
    Mio,
    /// ergon-io.
    Ergon,
}

impl Impl {
    /// Its name on the command line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mio => "mio",
            Self::Ergon => "ergon",
        }
    }
}

/// Which end a thread builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The measuring end (a one-way sender too).
    Ping,
    /// The echoing end (a one-way receiver too).
    Pong,
}

/// Flags both socket binaries take.
#[derive(Debug, Clone)]
pub struct Sock {
    /// `--impl`.
    pub imp: Impl,
    /// `--rcvbuf`, default 4 MiB.
    pub rcvbuf: u32,
    /// `--sndbuf`; absent: the kernel's.
    pub sndbuf: Option<u32>,
    /// `--busy-read-us`, default 0: `SO_BUSY_POLL` on every socket of either
    /// arm (mio by `setsockopt`, ergon-io by `Config::busy_poll_us`).
    pub busy_read_us: u32,
    /// `--epoll-events`, default 64.
    pub epoll_events: u32,
    /// `--connect-timeout-s`, default 30: the longest wait for the peer's
    /// connections.
    pub connect_timeout: Duration,
}

impl Sock {
    /// The socket flags, as `udp_lat` and `tcp_lat` take them.
    ///
    /// `--impl`, `--rcvbuf`, `--sndbuf`, `--busy-read-us`, `--epoll-events`
    /// and `--connect-timeout-s`, taken from `f`.
    ///
    /// # Errors
    /// [`Error::Usage`] for one missing or refused.
    pub fn parse(f: &mut Flags) -> Result<Self, Error> {
        let imp = match f.req("impl")?.as_str() {
            "mio" => Impl::Mio,
            "ergon" => Impl::Ergon,
            i => return Err(Error::Usage(format!("--impl {i}: mio or ergon"))),
        };
        let busy_read_us = f.num("busy-read-us", 0)?;
        let sndbuf = match f.take("sndbuf") {
            None => None,
            Some(v) => Some(
                v.parse()
                    .map_err(|_| Error::Usage(format!("--sndbuf {v}: bytes")))?,
            ),
        };
        let epoll_events = f.num("epoll-events", 64)?;
        if epoll_events == 0 {
            return Err(Error::Usage("--epoll-events: at least 1".into()));
        }
        Ok(Self {
            imp,
            rcvbuf: f.num("rcvbuf", DEFAULT_RCVBUF)?,
            sndbuf,
            busy_read_us,
            epoll_events,
            connect_timeout: Duration::from_secs(f.num("connect-timeout-s", 30)?),
        })
    }
}

fn addr(f: &mut Flags, key: &str, default: Option<&str>) -> Result<Option<SocketAddr>, Error> {
    f.take(key)
        .or_else(|| default.map(str::to_owned))
        .map(|v| {
            v.parse()
                .map_err(|_| Error::Usage(format!("--{key} {v}: IP:PORT")))
        })
        .transpose()
}

/// The process's CPUs less every `--cpus` one (empty when none is left:
/// ergon-io then chooses).
#[allow(unsafe_code)]
fn helper_cpus(c: &Common) -> Vec<u32> {
    if c.cpus.is_empty() {
        return Vec::new();
    }
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its
    // size; CPU_ISSET reads within it.
    let all: Vec<usize> = unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return Vec::new();
        }
        (0..usize::try_from(libc::CPU_SETSIZE).unwrap_or(0))
            .filter(|&cpu| libc::CPU_ISSET(cpu, &set))
            .collect()
    };
    all.into_iter()
        .filter(|cpu| !c.cpus.contains(cpu))
        .filter_map(|cpu| u32::try_from(cpu).ok())
        .collect()
}

/// SIGTERM: a pong stops.
static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigterm(_signal: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

#[allow(unsafe_code)]
fn stop_on_sigterm() {
    let handler: extern "C" fn(libc::c_int) = on_sigterm;
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    unsafe { libc::signal(libc::SIGTERM, handler as libc::sighandler_t) };
}

fn joined<T>(r: std::thread::Result<Result<T, Error>>, who: &str) -> Result<T, Error> {
    r.map_err(|_| Error::Transport(format!("the {who} thread panicked")))?
}

const fn busy_threads(c: &Common) -> u32 {
    match c.role {
        Role::Local => 2,
        _ => 1,
    }
}

const fn thread_order(c: &Common) -> &'static str {
    match (c.role, c.round_trip) {
        (Role::Local, false) => "sender,receiver",
        (Role::Local, true) => "ping,echo",
        (Role::Pong, _) => "echo",
        _ => "ping",
    }
}

/// What the measuring end hands back.
struct Ends {
    rec: Recorder,
    send: SendStats,
    allocs: u64,
    start_realtime_ns: u128,
    retrans: Option<u64>,
    wire_bytes: u64,
    report: Vec<(String, String)>,
    /// the other thread's (local), prefixed
    other: Vec<(String, String)>,
}

/// Settings and counters, by name.
type Settings = Vec<(String, String)>;

/// What the manifest's `retrans_window` says the row's `retrans` covers.
pub const RETRANS_WINDOW: &str = "after set-up: warm-up, measured, drain";

/// The namespace's TCP counters before an end builds its sockets: the
/// start of its set-up.
fn setup_start() -> Result<Netstat, Error> {
    Netstat::read()
}

/// An end's counters as its traffic starts (its set-up over): read off
/// the hot path, before the first send.
struct Window {
    retrans: Option<u64>,
    tcp: Option<TcpCounters>,
    /// the namespace's at the start of set-up, and now
    net: (Netstat, Netstat),
    sched: Sched,
}

impl Window {
    fn open<R: Inspect>(rx: &R, setup: Netstat) -> Result<Self, Error> {
        Ok(Self {
            retrans: rx.retrans()?,
            tcp: rx.tcp_counters()?,
            net: (setup, Netstat::read()?),
            sched: Sched::read()?,
        })
    }
}

/// An end's numbers read off the hot path once its traffic is over:
/// retransmits since `w` opened, and its report. For TCP the report holds
/// the counters that tell a retransmit the peer already had (DSACK) from a
/// lost segment, for set-up and for the traffic apart (`tcp.*`, the
/// namespace's `netstat.setup.*` / `netstat.window.*`); for every arm,
/// the thread's run-queue wait over the traffic (`sched.*`).
fn inspect<R: Inspect>(rx: &R, w: &Window) -> Result<(Option<u64>, Settings), Error> {
    let r1 = rx.retrans()?;
    let sched = Sched::read()?.since(&w.sched);
    let mut report = Vec::new();
    rx.report(&mut report)?;
    let mut put = |k: &str, v: String| report.push((k.to_owned(), v));
    put("sched.run_delay_ns", sched.run_delay_ns.to_string());
    put(
        "sched.involuntary_switches",
        sched.involuntary_switches.to_string(),
    );
    if let (Some(t0), Some(t1)) = (w.tcp, rx.tcp_counters()?) {
        let opt = |v: Option<u64>| v.map_or_else(|| "unavailable".to_owned(), |v| v.to_string());
        let d = |a: Option<u64>, b: Option<u64>| a.zip(b).map(|(a, b)| b.saturating_sub(a));
        let retrans = t1.retrans.saturating_sub(t0.retrans);
        let dsack = d(t0.dsack_dups, t1.dsack_dups);
        put("tcp.retrans", retrans.to_string());
        put("tcp.retrans_setup", t0.retrans.to_string());
        put("tcp.dsack_dups", opt(dsack));
        put("tcp.dsack_dups_setup", opt(t0.dsack_dups));
        put(
            "tcp.retrans_not_dsack",
            opt(dsack.map(|k| retrans.saturating_sub(k))),
        );
        put("tcp.lost_end", t1.lost.to_string());
        put("tcp.rtos", opt(d(t0.rtos, t1.rtos)));
        put("tcp.rtos_setup", opt(t0.rtos));
        let (start, open) = &w.net;
        for (k, v) in open.since(start) {
            put(&format!("netstat.setup.{k}"), v.to_string());
        }
        for (k, v) in Netstat::read()?.since(open) {
            put(&format!("netstat.window.{k}"), v.to_string());
        }
    }
    Ok((w.retrans.zip(r1).map(|(a, b)| b.saturating_sub(a)), report))
}

fn add(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (a, b) => a.or(b),
    }
}

/// The ping role on its own thread.
fn ping_role<T, R, M>(c: &Common, clock: &TscClock, make: &M) -> Result<Ends, Error>
where
    T: Tx,
    R: Rx + Inspect,
    M: Fn(End, Option<usize>) -> Result<(T, R), Error> + Sync,
{
    std::thread::scope(|s| {
        joined(
            s.spawn(move || {
                point::pin(c.cpu(0))?;
                let n0 = setup_start()?;
                let (mut tx, mut rx) = make(End::Ping, c.cpu(0))?;
                let wire_bytes = rx.wire_bytes()?;
                let w = Window::open(&rx, n0)?;
                let (t, rt) = point::start_at(clock, START_DELAY_NS);
                let mut rec = Recorder::new(clock, t, c)?;
                let a0 = thread_allocs();
                let send = point::ping(clock, c, t, &mut tx, &mut rx, &mut rec)?;
                let allocs = thread_allocs() - a0;
                let (retrans, report) = inspect(&rx, &w)?;
                Ok(Ends {
                    rec,
                    send,
                    allocs,
                    start_realtime_ns: rt,
                    retrans,
                    wire_bytes,
                    report,
                    other: Vec::new(),
                })
            })
            .join(),
            "ping",
        )
    })
}

/// What the echo end hands back: echoes, allocations, retransmits, wire
/// bytes, report.
type EchoOut = (u64, u64, Option<u64>, u64, Vec<(String, String)>);

/// The echo end on this thread, once built (its set-up began at
/// `setup`): until `keep_going` says stop or every peer closed.
fn echo_end<T: Tx, R: Rx + Inspect>(
    tx: &mut T,
    rx: &mut R,
    setup: Netstat,
    keep_going: &mut impl FnMut() -> bool,
) -> Result<EchoOut, Error> {
    let wire_bytes = rx.wire_bytes()?;
    let w = Window::open(rx, setup)?;
    let a0 = thread_allocs();
    let echoes = point::echo(rx, tx, keep_going)?;
    let allocs = thread_allocs() - a0;
    let (retrans, report) = inspect(rx, &w)?;
    Ok((echoes, allocs, retrans, wire_bytes, report))
}

/// Both ends in this process, ping and echo, each on its own thread.
fn local_round_trip<T, R, M>(c: &Common, clock: &TscClock, make: &M) -> Result<Ends, Error>
where
    T: Tx,
    R: Rx + Inspect,
    M: Fn(End, Option<usize>) -> Result<(T, R), Error> + Sync,
{
    let (barrier, stop, echoed) = (
        Barrier::new(2),
        AtomicBool::new(false),
        AtomicBool::new(false),
    );
    let (barrier, stop, echoed) = (&barrier, &stop, &echoed);
    std::thread::scope(|s| {
        let echo = s.spawn(move || {
            let setup = point::pin(c.cpu(1)).and_then(|()| {
                let n0 = setup_start()?;
                make(End::Pong, c.cpu(1)).map(|e| (e, n0))
            });
            barrier.wait();
            let r = setup.and_then(|((mut tx, mut rx), n0)| {
                echo_end(&mut tx, &mut rx, n0, &mut || !stop.load(Ordering::Acquire))
            });
            echoed.store(true, Ordering::Release);
            r
        });
        let ping = s.spawn(move || {
            let setup = point::pin(c.cpu(0)).and_then(|()| {
                let n0 = setup_start()?;
                make(End::Ping, c.cpu(0)).map(|e| (e, n0))
            });
            barrier.wait();
            let run = || {
                let ((mut tx, mut rx), n0) = setup?;
                let wire_bytes = rx.wire_bytes()?;
                let w = Window::open(&rx, n0)?;
                let (t, rt) = point::start_at(clock, START_DELAY_NS);
                let mut rec = Recorder::new(clock, t, c)?;
                let a0 = thread_allocs();
                let send = point::ping(clock, c, t, &mut tx, &mut rx, &mut rec)?;
                let allocs = thread_allocs() - a0;
                let (retrans, report) = inspect(&rx, &w)?;
                // the echo end stops, and reads its counters, before this
                // end's sockets close under it
                stop.store(true, Ordering::Release);
                let until = Instant::now() + Duration::from_secs(10);
                while !echoed.load(Ordering::Acquire) && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(Ends {
                    rec,
                    send,
                    allocs,
                    start_realtime_ns: rt,
                    retrans,
                    wire_bytes,
                    report,
                    other: Vec::new(),
                })
            };
            let r = run();
            stop.store(true, Ordering::Release);
            r
        });
        let (mut ends, (echoes, allocs, retrans, _, report)) =
            point::both(joined(ping.join(), "ping"), joined(echo.join(), "echo"))?;
        ends.allocs += allocs;
        ends.retrans = add(ends.retrans, retrans);
        ends.other = report
            .into_iter()
            .map(|(k, v)| (format!("echo.{k}"), v))
            .collect();
        ends.other.push(("echo.echoes".into(), echoes.to_string()));
        Ok(ends)
    })
}

/// Both ends in this process, one way: a sender and a receiver thread.
fn local_oneway<T, R, M>(c: &Common, clock: &TscClock, make: &M) -> Result<Ends, Error>
where
    T: Tx,
    R: Rx + Inspect,
    M: Fn(End, Option<usize>) -> Result<(T, R), Error> + Sync,
{
    let (barrier, start, start_rt) = (Barrier::new(2), AtomicU64::new(0), AtomicU64::new(0));
    let (sent_done, recv_done) = (AtomicBool::new(false), AtomicBool::new(false));
    let progress = Progress::default();
    let (barrier, start, start_rt, progress) = (&barrier, &start, &start_rt, &progress);
    let (sent_done, recv_done) = (&sent_done, &recv_done);
    let wait = |flag: &AtomicBool| {
        let until = Instant::now() + Duration::from_secs(10);
        while !flag.load(Ordering::Acquire) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    std::thread::scope(|s| {
        let sender = s.spawn(move || {
            let setup = point::pin(c.cpu(0)).and_then(|()| {
                let n0 = setup_start()?;
                make(End::Ping, c.cpu(0)).map(|e| (e, n0))
            });
            barrier.wait();
            let (t, rt) = point::start_at(clock, START_DELAY_NS);
            start.store(t, Ordering::Release);
            start_rt.store(u64::try_from(rt).unwrap_or(u64::MAX), Ordering::Release);
            barrier.wait();
            let r = (|| {
                let ((mut tx, rx), n0) = setup.inspect_err(|_| progress.fail())?;
                let wire_bytes = rx.wire_bytes()?;
                let w = Window::open(&rx, n0)?;
                let a0 = thread_allocs();
                let send = point::send_oneway(clock, c, t, &mut tx, progress)?;
                let allocs = thread_allocs() - a0;
                let (retrans, report) = inspect(&rx, &w)?;
                // the receiver reads its counters before these sockets close
                sent_done.store(true, Ordering::Release);
                wait(recv_done);
                Ok((send, allocs, retrans, wire_bytes, report))
            })();
            sent_done.store(true, Ordering::Release);
            r
        });
        let receiver = s.spawn(move || {
            let setup = point::pin(c.cpu(1)).and_then(|()| make(End::Pong, c.cpu(1)));
            barrier.wait();
            barrier.wait();
            let r = (|| {
                let (_tx, mut rx) = setup.inspect_err(|_| progress.fail())?;
                let t = start.load(Ordering::Acquire);
                let mut rec = Recorder::new(clock, t, c).inspect_err(|_| progress.fail())?;
                let a0 = thread_allocs();
                point::recv_oneway(clock, c, t, &mut rx, &mut rec, progress)?;
                let allocs = thread_allocs() - a0;
                let mut report = Vec::new();
                rx.report(&mut report)?;
                recv_done.store(true, Ordering::Release);
                wait(sent_done);
                Ok((rec, allocs, report))
            })();
            recv_done.store(true, Ordering::Release);
            r
        });
        let ((send, sa, s_retrans, wire_bytes, report), (rec, ra, r_report)) = point::both(
            joined(sender.join(), "sender"),
            joined(receiver.join(), "receiver"),
        )?;
        Ok(Ends {
            rec,
            send,
            allocs: sa + ra,
            start_realtime_ns: u128::from(start_rt.load(Ordering::Acquire)),
            retrans: s_retrans,
            wire_bytes,
            report,
            other: r_report
                .into_iter()
                .map(|(k, v)| (format!("receiver.{k}"), v))
                .collect(),
        })
    })
}

/// Runs `c`'s role over the arm `make` builds (on each end's pinned thread)
/// and writes the point's files; the row, or `None` for a pong.
///
/// # Errors
/// What any end met; no row is then written.
pub fn run<T, R, M>(
    c: &Common,
    imp: Impl,
    transport: &str,
    arm: &[(String, String)],
    ready_said: bool,
    make: M,
) -> Result<Option<String>, Error>
where
    T: Tx,
    R: Rx + Inspect,
    M: Fn(End, Option<usize>) -> Result<(T, R), Error> + Sync,
{
    match c.role {
        Role::OnewaySend | Role::OnewayRecv => {
            return Err(Error::Usage(format!(
                "--role {}: the socket arms measure one way with --role local (both ends in this process, one counter)",
                c.role.name()
            )));
        }
        Role::Local | Role::Ping | Role::Pong => {}
    }
    c.check_cpus(busy_threads(c) as usize, thread_order(c))?;
    let name = format!("{}-{transport}", imp.name());
    let manifest = |wire_bytes: u64, report: &[(String, String)]| {
        let mut m = RunManifest::new(&name, &template(), wire_bytes);
        m.setting("transport", transport)
            .setting("impl", imp.name())
            .setting("backend", "epoll-spin")
            .setting("thread_order", thread_order(c));
        for (k, v) in arm.iter().chain(report) {
            m.setting(k, v);
        }
        m
    };
    if c.role == Role::Pong {
        stop_on_sigterm();
        let out = std::thread::scope(|s| {
            joined(
                s.spawn(|| {
                    point::pin(c.cpu(0))?;
                    let n0 = setup_start()?;
                    let (mut tx, mut rx) = make(End::Pong, c.cpu(0))?;
                    if !ready_said {
                        println!("ready");
                    }
                    echo_end(&mut tx, &mut rx, n0, &mut || !STOP.load(Ordering::Relaxed))
                })
                .join(),
                "echo",
            )
        })?;
        let (echoes, allocs, retrans, wire_bytes, report) = out;
        let mut m = manifest(wire_bytes, &report);
        m.setting("echoes", &echoes.to_string())
            .setting("hot_path_allocs", &allocs.to_string())
            .setting("busy_threads", &busy_threads(c).to_string())
            .setting("wire_bytes", &wire_bytes.to_string())
            .setting(
                "retrans",
                &retrans.map_or_else(|| "none".to_owned(), |r| r.to_string()),
            );
        if retrans.is_some() {
            m.setting("retrans_window", RETRANS_WINDOW);
        }
        point::write_manifest(c, m)?;
        return Ok(None);
    }
    let clock = TscClock::new();
    let ends = match c.role {
        Role::Ping => ping_role(c, &clock, &make)?,
        _ if c.round_trip => local_round_trip(c, &clock, &make)?,
        _ => local_oneway(c, &clock, &make)?,
    };
    let end = clock.raw();
    let mut m = manifest(ends.wire_bytes, &ends.report);
    for (k, v) in &ends.other {
        m.setting(k, v);
    }
    m.setting(
        "retrans_scope",
        if c.role == Role::Local && c.round_trip {
            "both ends"
        } else {
            "this end's sends"
        },
    );
    if ends.retrans.is_some() {
        m.setting("retrans_window", RETRANS_WINDOW);
    }
    let row = point::finish(
        c,
        &clock,
        ends.rec,
        ends.send,
        ends.start_realtime_ns,
        end,
        Report {
            channel: transport,
            retrans: ends.retrans,
            wire_bytes: ends.wire_bytes,
            busy_threads: busy_threads(c),
            hot_path_allocs: ends.allocs,
            manifest: m,
        },
    )?;
    Ok(Some(row))
}

/// `udp_lat`'s usage.
pub const UDP_USAGE: &str = "\
udp_lat --impl mio|ergon --role ping|pong|local [--round-trip] --rate MSG_PER_S --label SET:ARM --out DIR
        [--local 127.0.0.1:22000] [--peer 127.0.0.1:23000] [--streams 1] [--warmup-s 30] [--duration-s 30]
        [--size 64] [--cpus CSV] [--spike-ns 100000] [--stall-ns 10000] [--rcvbuf 4194304] [--sndbuf BYTES]
        [--epoll-events 64] [--busy-read-us 0] [--datagram-burst 64 (ergon)]
stream i: --local port + i <-> --peer port + i (local: the echo/receiver end binds --peer).
--cpus: ping: ping; pong: echo; local: sender,receiver (--round-trip: ping,echo).";

/// `tcp_lat`'s usage.
pub const TCP_USAGE: &str = "\
tcp_lat --impl mio|ergon --role ping|pong|local [--round-trip] --rate MSG_PER_S --label SET:ARM --out DIR
        (--listen IP:PORT | [--source IP] --connect IP:PORT; local: --listen, default 127.0.0.1:24000,
        and --source)
        [--streams 1] [--warmup-s 30] [--duration-s 30] [--size 64] [--cpus CSV] [--spike-ns 100000]
        [--stall-ns 10000] [--rcvbuf 4194304] [--sndbuf BYTES] [--epoll-events 64] [--nodelay]
        [--quickack-per-read] [--rto-min-us US] [--delack-max-us US] [--busy-read-us 0]
        [--connect-timeout-s 30]
stream i: connection i. local: the echo/receiver end listens, the other connects.
--source: the connecting end binds it first (so leaves by the NIC that address routes by).
--cpus: ping: ping; pong: echo; local: sender,receiver (--round-trip: ping,echo).";

/// `udp_lat`: parses `args` (without the program name) and runs the point;
/// the row, or `None` for a pong.
///
/// # Errors
/// A flag refused, or what the point met.
pub fn udp_main<I: IntoIterator<Item = String>>(args: I) -> Result<Option<String>, Error> {
    let mut f = Flags::parse(args)?;
    let c = Common::parse(&mut f)?;
    let s = Sock::parse(&mut f)?;
    let local = addr(&mut f, "local", Some("127.0.0.1:22000"))?
        .ok_or_else(|| Error::Usage("--local IP:PORT".into()))?;
    let peer = addr(&mut f, "peer", Some("127.0.0.1:23000"))?
        .ok_or_else(|| Error::Usage("--peer IP:PORT".into()))?;
    let burst = f.take("datagram-burst");
    if s.imp == Impl::Mio && burst.is_some() {
        return Err(Error::Usage(
            "--datagram-burst is ergon-io's: the mio arm reads each ready socket up to the point's per-stream limit"
                .into(),
        ));
    }
    let datagram_burst: u32 = match burst {
        None => u32::try_from(POLL_LIMIT).unwrap_or(64),
        Some(v) => v
            .parse()
            .map_err(|_| Error::Usage(format!("--datagram-burst {v}: a number")))?,
    };
    f.finish()?;
    // every stream's ports checked before any thread starts
    for i in 0..c.streams {
        port_plus(local, i)?;
        port_plus(peer, i)?;
    }
    // in one process the echo / receiving end binds the peer's ports
    let ends = move |end: End| match (c.role, end) {
        (Role::Local, End::Pong) => (peer, local),
        _ => (local, peer),
    };
    let arm = vec![
        ("local".to_owned(), local.to_string()),
        ("peer".to_owned(), peer.to_string()),
    ];
    let streams = c.streams;
    match s.imp {
        Impl::Mio => {
            let opts = MioUdpOptions {
                rcvbuf: s.rcvbuf,
                sndbuf: s.sndbuf,
                busy_read_us: s.busy_read_us,
                epoll_events: s.epoll_events as usize,
            };
            run(&c, s.imp, "udp", &arm, false, |end, _cpu| {
                let (l, p) = ends(end);
                mio_udp::pair(l, p, streams, opts.clone())
            })
        }
        Impl::Ergon => {
            let helpers = helper_cpus(&c);
            run(&c, s.imp, "udp", &arm, false, |end, cpu| {
                let (l, p) = ends(end);
                eio_udp::pair(
                    l,
                    p,
                    streams,
                    EioUdpOptions {
                        rcvbuf: s.rcvbuf,
                        sndbuf: s.sndbuf,
                        epoll_events: s.epoll_events,
                        datagram_burst,
                        busy_poll_us: s.busy_read_us,
                        loop_cpu: cpu.and_then(|c| u32::try_from(c).ok()),
                        helper_cpus: helpers.clone(),
                    },
                )
            })
        }
    }
}

/// Accepts `n` connections on `l` within `wait`, ordered by the index each
/// one's peer writes first.
///
/// # Errors
/// An accept or read fails, an index repeats or is out of range, or not all
/// came in time.
pub fn accept_all(l: &TcpListener, n: u32, wait: Duration) -> Result<Vec<TcpStream>, Error> {
    l.set_nonblocking(true).map_err(|e| os("O_NONBLOCK", e))?;
    let until = Instant::now() + wait;
    let mut slots: Vec<Option<TcpStream>> = (0..n).map(|_| None).collect();
    let mut got = 0;
    while got < n {
        match l.accept() {
            Ok((mut s, from)) => {
                s.set_nonblocking(false).map_err(|e| os("O_NONBLOCK", e))?;
                s.set_read_timeout(Some(wait))
                    .map_err(|e| os("SO_RCVTIMEO", e))?;
                let mut b = [0u8; 4];
                s.read_exact(&mut b)
                    .map_err(|e| os(&format!("reading {from}'s stream index"), e))?;
                s.set_read_timeout(None).map_err(|e| os("SO_RCVTIMEO", e))?;
                let i = u32::from_le_bytes(b);
                let slot = slots
                    .get_mut(i as usize)
                    .filter(|s| s.is_none())
                    .ok_or_else(|| {
                        Error::Transport(format!("{from}: stream index {i} of {n}, or twice"))
                    })?;
                *slot = Some(s);
                got += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > until {
                    return Err(Error::Timeout(format!(
                        "{got} of {n} connections came within {} s",
                        wait.as_secs()
                    )));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => return Err(os("accept", e)),
        }
    }
    Ok(slots.into_iter().flatten().collect())
}

/// Opens `n` connections to `to`, each bound to `source` first when given.
///
/// The bind is ergon-io's `net::connect_nonblocking`'s: on a host with a
/// routing table per NIC selected by source address, it picks the NIC. A
/// refused connect is retried within `wait`; each connection first carries
/// its stream index.
///
/// # Errors
/// A connect still refused when `wait` is up (or `source` is not this
/// host's address), or a write fails.
pub fn connect_all(
    to: SocketAddr,
    source: Option<IpAddr>,
    n: u32,
    wait: Duration,
) -> Result<Vec<TcpStream>, Error> {
    let until = Instant::now() + wait;
    let mut out = Vec::with_capacity(n as usize);
    let fail = |e: &dyn std::fmt::Display| {
        let from = source.map_or_else(|| "the kernel's pick".to_owned(), |ip| ip.to_string());
        Error::Transport(format!("connect {to} from {from}: {e}"))
    };
    for i in 0..n {
        let mut s = 'retry: loop {
            let s = match ergon_io::net::connect_nonblocking(to, source) {
                Ok(s) => s,
                // refused at once (nothing listens yet): again, within `wait`
                Err(ergon_io::Error::Io(_)) if Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(fail(&e)),
            };
            let attempt = Instant::now() + Duration::from_secs(1);
            loop {
                match ergon_io::net::connect_finished(&s) {
                    Ok(true) => break 'retry s,
                    Ok(false) if Instant::now() < attempt => {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Ok(false) | Err(_) if Instant::now() < until => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue 'retry;
                    }
                    Ok(false) => return Err(fail(&"timed out")),
                    Err(e) => return Err(fail(&e)),
                }
            }
        };
        s.set_nonblocking(false).map_err(|e| os("O_NONBLOCK", e))?;
        s.write_all(&i.to_le_bytes())
            .map_err(|e| os("writing the stream index", e))?;
        out.push(s);
    }
    Ok(out)
}

/// `tcp_lat`: parses `args` (without the program name) and runs the point;
/// the row, or `None` for a pong.
///
/// # Errors
/// A flag refused, or what the point met.
#[allow(clippy::too_many_lines)] // the flags, the listener and both arms, in order
pub fn tcp_main<I: IntoIterator<Item = String>>(args: I) -> Result<Option<String>, Error> {
    let mut f = Flags::parse(args)?;
    let c = Common::parse(&mut f)?;
    let s = Sock::parse(&mut f)?;
    if !f.bool("nodelay", true)? {
        return Err(Error::Usage("--nodelay is always on".into()));
    }
    let quickack_per_read = f.bool("quickack-per-read", false)?;
    let timer = |f: &mut Flags, k: &str| -> Result<Option<u32>, Error> {
        f.take(k)
            .map(|v| {
                v.parse()
                    .map_err(|_| Error::Usage(format!("--{k} {v}: µs")))
            })
            .transpose()
    };
    let rto_min_us = timer(&mut f, "rto-min-us")?;
    let delack_max_us = timer(&mut f, "delack-max-us")?;
    let listen = addr(&mut f, "listen", None)?;
    let connect = addr(&mut f, "connect", None)?;
    let source = f
        .take("source")
        .map(|v| {
            v.parse::<IpAddr>()
                .map_err(|_| Error::Usage(format!("--source {v}: an IP address")))
        })
        .transpose()?;
    f.finish()?;
    if source.is_some() && c.role != Role::Local && connect.is_none() {
        return Err(Error::Usage(
            "--source is the connecting end's (--connect, or --role local)".into(),
        ));
    }
    let listen = match (c.role, listen, connect) {
        (Role::Local, None, None) => Some(SocketAddr::from(([127, 0, 0, 1], 24_000))),
        (Role::Local, l, None) => l,
        (Role::Local, _, Some(_)) => {
            return Err(Error::Usage(
                "--role local: --listen only (the other end connects to it)".into(),
            ));
        }
        (_, Some(_), Some(_)) | (_, None, None) => {
            return Err(Error::Usage("one of --listen or --connect".into()));
        }
        (_, l, _) => l,
    };
    for a in listen.iter().chain(&connect) {
        port_plus(*a, 0)?;
    }
    let listener = match listen {
        Some(l) => Some(TcpListener::bind(l).map_err(|e| os(&format!("listen {l}"), e))?),
        None => None,
    };
    // a listening echo end is ready once it listens
    let ready_said = c.role == Role::Pong && listener.is_some();
    if ready_said {
        println!("ready");
    }
    let to = match (&listener, connect) {
        (Some(l), None) if c.role == Role::Local => {
            Some(l.local_addr().map_err(|e| os("getsockname", e))?)
        }
        (_, t) => t,
    };
    let (streams, wait) = (c.streams, s.connect_timeout);
    let listener = listener.as_ref();
    let role = c.role;
    let sockets = move |end: End| -> Result<Vec<TcpStream>, Error> {
        match (role, end, listener, to) {
            (Role::Local, End::Pong, Some(l), _) => accept_all(l, streams, wait),
            (Role::Local, End::Ping, _, Some(t)) => connect_all(t, source, streams, wait),
            (Role::Local, ..) => Err(Error::Usage("--role local needs --listen".into())),
            (_, _, Some(l), None) => accept_all(l, streams, wait),
            (_, _, None, Some(t)) => connect_all(t, source, streams, wait),
            _ => Err(Error::Usage("one of --listen or --connect".into())),
        }
    };
    let opt = |v: Option<u32>| v.map_or_else(|| "kernel".to_owned(), |b| b.to_string());
    let arm = vec![
        (
            "listen".to_owned(),
            listen.map_or_else(|| "-".to_owned(), |a| a.to_string()),
        ),
        (
            "connect".to_owned(),
            connect.map_or_else(|| "-".to_owned(), |a| a.to_string()),
        ),
        (
            "source".to_owned(),
            source.map_or_else(|| "-".to_owned(), |ip| ip.to_string()),
        ),
        ("rto_min_us_asked".to_owned(), opt(rto_min_us)),
        ("delack_max_us_asked".to_owned(), opt(delack_max_us)),
    ];
    match s.imp {
        Impl::Mio => {
            let opts = MioTcpOptions {
                rcvbuf: s.rcvbuf,
                sndbuf: s.sndbuf,
                busy_read_us: s.busy_read_us,
                epoll_events: s.epoll_events as usize,
                quickack_per_read,
                rto_min_us,
                delack_max_us,
            };
            run(&c, s.imp, "tcp", &arm, ready_said, |end, _cpu| {
                mio_tcp::pair(sockets(end)?, &opts)
            })
        }
        Impl::Ergon => {
            let helpers = helper_cpus(&c);
            run(&c, s.imp, "tcp", &arm, ready_said, |end, cpu| {
                eio_tcp::pair(
                    sockets(end)?,
                    EioTcpOptions {
                        rcvbuf: s.rcvbuf,
                        sndbuf: s.sndbuf,
                        busy_poll_us: s.busy_read_us,
                        epoll_events: s.epoll_events,
                        quickack_per_read,
                        rto_min_us,
                        delack_max_us,
                        loop_cpu: cpu.and_then(|c| u32::try_from(c).ok()),
                        helper_cpus: helpers.clone(),
                    },
                )
            })
        }
    }
}
