//! The `udp_lat` and `tcp_lat` command lines: parse, place the threads,
//! build each end's [`Link`] on its own pinned thread, run the point
//! ([`point`](super::point)) and write its files.
//!
//! Threads, in `--cpus` order (one isolated CPU each):
//!
//! | role | `--cpus` | threads |
//! |---|---|---|
//! | `ping` | 1 | the ping thread: paces, sends, receives and records every stream |
//! | `pong` | 1 | the echo thread: echoes every stream |
//! | `local` | 2 | the ping thread, then the echo thread, in one process |
//!
//! The main thread only sets up, waits and writes files. ergon-io's helper
//! threads (its warning thread) go on the process's other CPUs
//! (`Config::helper_cpus`), never on a `--cpus` one.
//!
//! Files in `--out` (`<name>` is the label as lib.sh spells file names,
//! `<set>-<mode>`): `results.csv` (one row), `manifest.json`,
//! `histogram-<name>.csv` (from the intended send time),
//! `histogram-<name>-send.csv` (from the actual send),
//! `spikes-<name>.csv` (with its `# max=` line), `stalls-<name>.csv` (the
//! ping thread's; `-pong` for the echo thread's in `local`). A `pong`
//! writes `manifest.json` and `stalls-<name>.csv` only. Nothing is written
//! unless every thread succeeded; the row goes to stdout last.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::{MESSAGE_LEN, template};
use ergon_latency_core::row::{Row, RowKind};

use super::eio_tcp::{EioTcp, EioTcpOptions};
use super::eio_udp::{EioUdp, EioUdpOptions};
use super::mio_tcp::{MioTcp, MioTcpOptions};
use super::mio_udp::{MioUdp, MioUdpOptions};
use super::point::{Hooks, PingOutcome, PongEnd, PongOutcome, Spec, run_ping, run_pong};
use super::{DEFAULT_RCVBUF, Error, Link};
use crate::args::Args;

/// Which end(s) this process runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Sends, receives the echoes, measures: the row's end.
    Ping,
    /// Echoes.
    Pong,
    /// Both, on two pinned threads of one process.
    Local,
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
    const fn name(self) -> &'static str {
        match self {
            Self::Mio => "mio",
            Self::Ergon => "ergon",
        }
    }
}

/// One end of a [`Role::Local`] pair, or the end a process runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The measuring end.
    Ping,
    /// The echoing end.
    Pong,
}

/// Flags every transport binary takes.
const COMMON_KEYS: &[&str] = &[
    "impl",
    "role",
    "streams",
    "rate",
    "warmup-s",
    "duration-s",
    "size",
    "cpus",
    "label",
    "out",
    "spike-ns",
    "stall-ns",
    "rcvbuf",
    "sndbuf",
    "busy-read-us",
    "epoll-events",
    "wait-s",
    "idle-exit-s",
];
const UDP_KEYS: &[&str] = &["local", "peer", "datagram-burst"];
const TCP_KEYS: &[&str] = &[
    "listen",
    "connect",
    "nodelay",
    "quickack-per-read",
    "rto-min-us",
    "delack-max-us",
];

/// `udp_lat`'s usage.
pub const UDP_USAGE: &str = "\
udp_lat --impl mio|ergon --role ping|pong|local --local IP:PORT --peer IP:PORT
        --cpus CPU[,CPU] --label SET:ARM --out DIR
        [--streams 1] [--rate 10000] [--warmup-s 30] [--duration-s 30] [--size 64]
        [--spike-ns 100000] [--stall-ns 10000] [--rcvbuf 4194304] [--sndbuf BYTES]
        [--epoll-events 64] [--busy-read-us 0 (mio)] [--datagram-burst 16 (ergon)]
        [--wait-s 60] [--idle-exit-s 5]
stream i: local port + i <-> peer port + i. --cpus: ping|pong: the polling thread; local: ping, pong.";

/// `tcp_lat`'s usage.
pub const TCP_USAGE: &str = "\
tcp_lat --impl mio|ergon --role ping|pong|local (--listen IP:PORT | --connect IP:PORT)
        --cpus CPU[,CPU] --label SET:ARM --out DIR
        [--streams 1] [--rate 10000] [--warmup-s 30] [--duration-s 30] [--size 64]
        [--spike-ns 100000] [--stall-ns 10000] [--rcvbuf 4194304] [--sndbuf BYTES]
        [--epoll-events 64] [--nodelay] [--quickack-per-read] [--rto-min-us US]
        [--delack-max-us US] [--busy-read-us 0 (mio)] [--wait-s 60] [--idle-exit-s 5]
stream i: connection i. local: the echo thread listens on --listen. --cpus: ping|pong: the polling thread; local: ping, pong.";

/// What every transport binary was asked.
#[derive(Debug, Clone)]
pub struct Common {
    /// `--role`.
    pub role: Role,
    /// `--impl`.
    pub imp: Impl,
    /// The schedule and thresholds.
    pub spec: Spec,
    /// `--cpus`, one per busy thread.
    pub cpus: Vec<usize>,
    /// `--label`.
    pub label: String,
    /// `--out`.
    pub out: PathBuf,
    /// `--rcvbuf`.
    pub rcvbuf: u32,
    /// `--sndbuf`.
    pub sndbuf: Option<u32>,
    /// `--busy-read-us` (mio only).
    pub busy_read_us: u32,
    /// `--epoll-events`.
    pub epoll_events: u32,
    /// `--wait-s`: the longest wait for the peer, s.
    pub wait_s: u64,
    /// `--idle-exit-s`: a `pong` stops this long after the last ping, s.
    pub idle_exit_s: u64,
}

fn cfg(e: impl std::fmt::Display) -> Error {
    Error::Config(e.to_string())
}

fn addr(a: &Args, k: &str) -> Result<Option<SocketAddr>, Error> {
    if !a.has(k) {
        return Ok(None);
    }
    let v = a.req(k).map_err(cfg)?;
    v.parse()
        .map(Some)
        .map_err(|_| Error::Config(format!("--{k} {v}: not IP:PORT")))
}

impl Common {
    /// Parses the common flags, refusing any flag not in them or `extra`.
    ///
    /// # Errors
    /// [`Error::Config`] for an unknown or malformed flag, a size other
    /// than 64, or `--cpus` not one per busy thread.
    pub fn parse(a: &Args, extra: &[&str]) -> Result<Self, Error> {
        if !a.command.is_empty() || !a.positional.is_empty() {
            return Err(Error::Config(format!(
                "unexpected argument {:?}",
                if a.command.is_empty() {
                    &a.positional[0]
                } else {
                    &a.command
                }
            )));
        }
        let mut unknown: Vec<&str> = a
            .keys()
            .filter(|k| !COMMON_KEYS.contains(k) && !extra.contains(k))
            .collect();
        unknown.sort_unstable();
        if let Some(k) = unknown.first() {
            return Err(Error::Config(format!("unknown flag --{k}")));
        }
        let role = match a.req("role").map_err(cfg)?.as_str() {
            "ping" => Role::Ping,
            "pong" => Role::Pong,
            "local" => Role::Local,
            r => {
                return Err(Error::Config(format!(
                    "--role {r}: ping, pong or local (one-way roles are not socket arms)"
                )));
            }
        };
        let imp = match a.req("impl").map_err(cfg)?.as_str() {
            "mio" => Impl::Mio,
            "ergon" => Impl::Ergon,
            i => return Err(Error::Config(format!("--impl {i}: mio or ergon"))),
        };
        let size: usize = a.num_or("size", MESSAGE_LEN).map_err(cfg)?;
        if size != MESSAGE_LEN {
            return Err(Error::Config(format!(
                "--size {size}: the suite's message is {MESSAGE_LEN} bytes"
            )));
        }
        let spec = Spec {
            streams: a.num_or("streams", 1).map_err(cfg)?,
            rate: a.num_or("rate", 10_000).map_err(cfg)?,
            warmup_s: a.num_or("warmup-s", 30).map_err(cfg)?,
            duration_s: a.num_or("duration-s", 30).map_err(cfg)?,
            spike_ns: a.num_or("spike-ns", 100_000).map_err(cfg)?,
            stall_ns: a.num_or("stall-ns", 10_000).map_err(cfg)?,
        };
        if spec.streams == 0 || spec.rate == 0 || spec.duration_s == 0 {
            return Err(Error::Config(
                "--streams, --rate and --duration-s must be at least 1".into(),
            ));
        }
        let mut cpus = Vec::new();
        for c in a.list("cpus", "") {
            cpus.push(
                c.parse::<usize>()
                    .map_err(|_| Error::Config(format!("--cpus: bad CPU {c:?}")))?,
            );
        }
        let threads = if role == Role::Local { 2 } else { 1 };
        if cpus.len() != threads {
            return Err(Error::Config(format!(
                "--cpus: {threads} CPU(s) for role {role:?} (one per busy thread), got {}",
                cpus.len()
            )));
        }
        if threads == 2 && cpus[0] == cpus[1] {
            return Err(Error::Config(
                "--cpus: each busy thread needs its own CPU".into(),
            ));
        }
        let label = a.req("label").map_err(cfg)?;
        if label.is_empty() || label.contains([',', '\n', '\r']) {
            return Err(Error::Config(format!("--label {label:?}")));
        }
        let busy_read_us = a.num_or("busy-read-us", 0).map_err(cfg)?;
        if imp == Impl::Ergon && busy_read_us != 0 {
            return Err(Error::Config(
                "--busy-read-us is the mio arm's: ergon-io's epoll-spin sets SO_BUSY_POLL 0".into(),
            ));
        }
        let sndbuf = if a.has("sndbuf") {
            Some(a.num_or("sndbuf", 0).map_err(cfg)?)
        } else {
            None
        };
        Ok(Self {
            role,
            imp,
            spec,
            cpus,
            label,
            out: PathBuf::from(a.req("out").map_err(cfg)?),
            rcvbuf: a.num_or("rcvbuf", DEFAULT_RCVBUF).map_err(cfg)?,
            sndbuf,
            busy_read_us,
            epoll_events: a.num_or("epoll-events", 64).map_err(cfg)?,
            wait_s: a.num_or("wait-s", 60).map_err(cfg)?,
            idle_exit_s: a.num_or("idle-exit-s", 5).map_err(cfg)?,
        })
    }

    /// The label as lib.sh's file names spell it: `<set>-<mode>`, a `/` as
    /// `_` (`transport_points.file_name`).
    #[must_use]
    pub fn file_name(&self) -> String {
        match self.label.split_once(':') {
            Some((set, mode)) => format!("{set}-{mode}"),
            None => self.label.clone(),
        }
        .replace('/', "_")
    }

    /// The CPU of `end`'s thread.
    #[must_use]
    pub fn cpu(&self, end: End) -> usize {
        match (self.role, end) {
            (Role::Local, End::Pong) => self.cpus[1],
            _ => self.cpus[0],
        }
    }

    /// ergon-io's helper CPUs: the process's CPUs less every `--cpus` one
    /// (empty when none is left: ergon-io then chooses).
    #[must_use]
    pub fn helper_cpus(&self) -> Vec<u32> {
        allowed_cpus()
            .into_iter()
            .filter(|c| !self.cpus.contains(c))
            .filter_map(|c| u32::try_from(c).ok())
            .collect()
    }

    fn settings(&self, m: &mut RunManifest, transport: &str) {
        let threads = match self.role {
            Role::Ping => "ping",
            Role::Pong => "pong",
            Role::Local => "ping,pong",
        };
        let cpus: Vec<String> = self.cpus.iter().map(ToString::to_string).collect();
        for (k, v) in [
            ("role", format!("{:?}", self.role).to_lowercase()),
            ("impl", self.imp.name().to_owned()),
            ("transport", transport.to_owned()),
            ("kind", "load".to_owned()),
            ("label", self.label.clone()),
            ("streams", self.spec.streams.to_string()),
            ("rate", self.spec.rate.to_string()),
            ("warmup_s", self.spec.warmup_s.to_string()),
            ("duration_s", self.spec.duration_s.to_string()),
            ("size", MESSAGE_LEN.to_string()),
            ("spike_ns", self.spec.spike_ns.to_string()),
            ("stall_ns", self.spec.stall_ns.to_string()),
            ("cpus", cpus.join(",")),
            ("thread_order", threads.to_owned()),
            ("busy_threads", self.busy_threads().to_string()),
            ("backend", "epoll-spin".to_owned()),
        ] {
            m.setting(k, &v);
        }
    }

    const fn busy_threads(&self) -> u32 {
        if matches!(self.role, Role::Local) {
            2
        } else {
            1
        }
    }
}

/// The CPUs this process may run on.
fn allowed_cpus() -> Vec<usize> {
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its
    // size; CPU_ISSET reads within it.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return Vec::new();
        }
        (0..libc::CPU_SETSIZE as usize)
            .filter(|&c| libc::CPU_ISSET(c, &set))
            .collect()
    }
}

/// One end's link, measured counters and settings, back from its thread.
struct EndDone<O> {
    outcome: O,
    retrans: Option<u64>,
    wire_bytes: u64,
    report: Vec<(String, String)>,
}

fn pin(cpu: usize) -> Result<(), Error> {
    ergon_io::clock::pin_current_thread(cpu)
        .map_err(|e| Error::Config(format!("pinning to CPU {cpu}: {e}")))
}

fn ping_end<L: Link>(
    link: Result<L, Error>,
    clock: &TscClock,
    spec: &Spec,
    hooks: Hooks,
    local: Option<(&AtomicBool, &AtomicBool)>,
) -> Result<EndDone<PingOutcome>, Error> {
    let mut link = link?;
    let wire_bytes = link.wire_bytes()?;
    let r0 = link.retrans()?;
    let outcome = run_ping(&mut link, clock, spec, hooks)?;
    let r1 = link.retrans()?;
    let mut report = Vec::new();
    link.report(&mut report)?;
    // in one process the echo end stops, and reads its counters, before
    // this end's sockets close under it
    if let Some((stop, stopped)) = local {
        stop.store(true, Ordering::Release);
        let until = Instant::now() + Duration::from_secs(10);
        while !stopped.load(Ordering::Acquire) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    Ok(EndDone {
        outcome,
        retrans: r0.zip(r1).map(|(a, b)| b.saturating_sub(a)),
        wire_bytes,
        report,
    })
}

fn pong_end<L: Link>(
    link: Result<L, Error>,
    clock: &TscClock,
    c: &Common,
    end: PongEnd<'_>,
    hooks: Hooks,
) -> Result<EndDone<PongOutcome>, Error> {
    let mut link = link?;
    let wire_bytes = link.wire_bytes()?;
    let r0 = link.retrans()?;
    let outcome = run_pong(&mut link, clock, c.spec.stall_ns, end, hooks)?;
    let r1 = link.retrans()?;
    let mut report = Vec::new();
    link.report(&mut report)?;
    Ok(EndDone {
        outcome,
        retrans: r0.zip(r1).map(|(a, b)| b.saturating_sub(a)),
        wire_bytes,
        report,
    })
}

fn joined<T>(r: std::thread::Result<Result<T, Error>>, what: &str) -> Result<T, Error> {
    r.map_err(|_| Error::Protocol(format!("the {what} thread panicked")))?
}

/// Runs `c`'s role, each end's link made by `make` on its pinned thread,
/// and writes the point's files. `ready`: print `ready` on stdout once the
/// echo end's link is made (`false` when the caller said it already).
///
/// # Errors
/// Any end's error; the files are then not written.
pub fn execute<L, M>(
    c: &Common,
    transport: &str,
    make: M,
    ready: bool,
    hooks: Hooks,
) -> Result<(), Error>
where
    L: Link,
    M: Fn(End, usize) -> Result<L, Error> + Sync,
{
    std::fs::create_dir_all(&c.out)?;
    let clock = TscClock::new();
    let make = &make;
    let clock = &clock;
    match c.role {
        Role::Ping => {
            let cpu = c.cpu(End::Ping);
            let done = std::thread::scope(|s| {
                joined(
                    s.spawn(move || {
                        pin(cpu)?;
                        ping_end(make(End::Ping, cpu), clock, &c.spec, hooks, None)
                    })
                    .join(),
                    "ping",
                )
            })?;
            write_ping(c, transport, clock, &done, None)
        }
        Role::Pong => {
            let cpu = c.cpu(End::Pong);
            let end = PongEnd::Idle {
                idle_ns: c.idle_exit_s.saturating_mul(1_000_000_000),
                wait_ns: c.wait_s.saturating_mul(1_000_000_000),
            };
            let done = std::thread::scope(|s| {
                joined(
                    s.spawn(move || {
                        pin(cpu)?;
                        let link = make(End::Pong, cpu);
                        if ready && link.is_ok() {
                            println!("ready");
                        }
                        pong_end(link, clock, c, end, hooks)
                    })
                    .join(),
                    "pong",
                )
            })?;
            write_pong(c, transport, &done)
        }
        Role::Local => {
            let barrier = Barrier::new(2);
            let (stop, stopped) = (AtomicBool::new(false), AtomicBool::new(false));
            let (barrier, stop, stopped) = (&barrier, &stop, &stopped);
            let (ping, pong) = std::thread::scope(|s| {
                let pong = s.spawn(move || {
                    let cpu = c.cpu(End::Pong);
                    let link = pin(cpu).and_then(|()| make(End::Pong, cpu));
                    barrier.wait();
                    let r = pong_end(link, clock, c, PongEnd::Flag(stop), hooks);
                    stopped.store(true, Ordering::Release);
                    r
                });
                let ping = s.spawn(move || {
                    let cpu = c.cpu(End::Ping);
                    let link = pin(cpu).and_then(|()| make(End::Ping, cpu));
                    barrier.wait();
                    let r = ping_end(link, clock, &c.spec, hooks, Some((stop, stopped)));
                    stop.store(true, Ordering::Release);
                    r
                });
                (ping.join(), pong.join())
            });
            let ping = joined(ping, "ping")?;
            let pong = joined(pong, "pong")?;
            write_ping(c, transport, clock, &ping, Some(&pong))
        }
    }
}

fn manifest(
    c: &Common,
    transport: &str,
    wire_bytes: u64,
    report: &[(String, String)],
) -> RunManifest {
    let mut m = RunManifest::new(
        &format!("{}-{transport}", c.imp.name()),
        &template(),
        wire_bytes,
    );
    c.settings(&mut m, transport);
    for (k, v) in report {
        m.setting(k, v);
    }
    m
}

fn write_ping(
    c: &Common,
    transport: &str,
    clock: &TscClock,
    ping: &EndDone<PingOutcome>,
    pong: Option<&EndDone<PongOutcome>>,
) -> Result<(), Error> {
    let o = &ping.outcome;
    let name = c.file_name();
    // in one process the echo end's retransmits are this point's too
    let retrans = match pong.map(|p| p.retrans) {
        Some(Some(p)) => ping.retrans.map(|r| r + p),
        _ => ping.retrans,
    };
    let row = Row {
        kind: RowKind::Load,
        label: &c.label,
        channel: transport,
        size: MESSAGE_LEN,
        rate: c.spec.rate,
        latency: o.summary(),
        stream0: o.stream0.summary(),
        pace: o.pace,
        received: o.received,
        backpressured: o.backpressured,
        achieved: o.achieved,
        loss: o.loss,
        retrans,
        wire_bytes: ping.wire_bytes,
        busy_threads: c.busy_threads(),
    }
    .line()?;
    let mut m = manifest(c, transport, ping.wire_bytes, &ping.report);
    let s = o.summary();
    let send = o.latency.from_send.summary();
    for (k, v) in [
        ("clock.ticks_per_ns", format!("{:.6}", clock.ticks_per_ns())),
        ("pace.scheduled", o.pace.scheduled.to_string()),
        ("pace.sent", o.pace.sent.to_string()),
        ("pace.late", o.pace.late.to_string()),
        ("pace.max_send_lag_ns", o.pace.max_send_lag_ns.to_string()),
        ("pace.censored", o.pace.censored.to_string()),
        ("pace.flagged", o.pace.flagged().to_string()),
        ("loss.received", o.loss.received.to_string()),
        ("loss.lost", o.loss.lost.to_string()),
        ("loss.reordered", o.loss.reordered.to_string()),
        ("loss.duplicate", o.loss.duplicate.to_string()),
        ("loss.stale", o.loss.stale.to_string()),
        ("latency.count", s.count.to_string()),
        ("latency.censored", s.censored.to_string()),
        ("from_send.p50", send.p50.to_string()),
        ("from_send.p99", send.p99.to_string()),
        ("from_send.p99.99", send.p9999.to_string()),
        ("from_send.max", send.max.to_string()),
        (
            "retrans",
            retrans.map_or_else(|| "-".to_owned(), |r| r.to_string()),
        ),
        (
            "retrans.scope",
            if pong.is_some() {
                "both ends"
            } else {
                "this end's sends"
            }
            .to_owned(),
        ),
    ] {
        m.setting(k, &v);
    }
    if let Some(p) = pong {
        m.setting("pong.echoed", &p.outcome.echoed.to_string());
        m.setting("pong.ended", p.outcome.ended);
        for (k, v) in &p.report {
            m.setting(&format!("pong.{k}"), v);
        }
    }
    let out = &c.out;
    std::fs::write(
        out.join(format!("histogram-{name}.csv")),
        o.latency.from_intended.csv(),
    )?;
    std::fs::write(
        out.join(format!("histogram-{name}-send.csv")),
        o.latency.from_send.csv(),
    )?;
    std::fs::write(out.join(format!("spikes-{name}.csv")), o.spikes_csv())?;
    o.stalls
        .write(&out.join(format!("stalls-{name}.csv")), o.start_realtime_ns)?;
    if let Some(p) = pong {
        p.outcome.stalls.write(
            &out.join(format!("stalls-{name}-pong.csv")),
            p.outcome.start_realtime_ns,
        )?;
    }
    m.write(&out.join("manifest.json"))?;
    std::fs::write(out.join("results.csv"), format!("{row}\n"))?;
    println!("{row}");
    Ok(())
}

fn write_pong(c: &Common, transport: &str, pong: &EndDone<PongOutcome>) -> Result<(), Error> {
    let mut m = manifest(c, transport, pong.wire_bytes, &pong.report);
    m.setting("pong.echoed", &pong.outcome.echoed.to_string());
    m.setting("pong.ended", pong.outcome.ended);
    m.setting(
        "retrans",
        &pong
            .retrans
            .map_or_else(|| "-".to_owned(), |r| r.to_string()),
    );
    pong.outcome.stalls.write(
        &c.out.join(format!("stalls-{}.csv", c.file_name())),
        pong.outcome.start_realtime_ns,
    )?;
    m.write(&c.out.join("manifest.json"))?;
    Ok(())
}

/// `udp_lat`: parses `a` and runs it.
///
/// # Errors
/// Any setting refused, any end's failure.
pub fn udp_main(a: &Args, hooks: Hooks) -> Result<(), Error> {
    let c = Common::parse(a, UDP_KEYS)?;
    let local = addr(a, "local")?.ok_or_else(|| Error::Config("--local IP:PORT".into()))?;
    let peer = addr(a, "peer")?.ok_or_else(|| Error::Config("--peer IP:PORT".into()))?;
    let streams = c.spec.streams;
    // in one process the echo end binds the peer's ports
    let ends = |end: End| match (c.role, end) {
        (Role::Local, End::Pong) => (peer, local),
        _ => (local, peer),
    };
    match c.imp {
        Impl::Mio => {
            if a.has("datagram-burst") {
                return Err(Error::Config(
                    "--datagram-burst is ergon-io's: the mio arm reads each ready socket until WouldBlock"
                        .into(),
                ));
            }
            let opts = MioUdpOptions {
                rcvbuf: c.rcvbuf,
                sndbuf: c.sndbuf,
                busy_read_us: c.busy_read_us,
                epoll_events: c.epoll_events as usize,
            };
            execute(
                &c,
                "udp",
                |end, _cpu| {
                    let (l, p) = ends(end);
                    MioUdp::new(l, p, streams, opts.clone())
                },
                true,
                hooks,
            )
        }
        Impl::Ergon => {
            let burst = a.num_or("datagram-burst", 16).map_err(cfg)?;
            let helpers = c.helper_cpus();
            execute(
                &c,
                "udp",
                |end, cpu| {
                    let (l, p) = ends(end);
                    EioUdp::new(
                        l,
                        p,
                        streams,
                        EioUdpOptions {
                            rcvbuf: c.rcvbuf,
                            sndbuf: c.sndbuf,
                            epoll_events: c.epoll_events,
                            datagram_burst: burst,
                            loop_cpu: u32::try_from(cpu).ok(),
                            helper_cpus: helpers.clone(),
                        },
                    )
                },
                true,
                hooks,
            )
        }
    }
}

/// Accepts `n` connections on `l`, waiting at most `wait`.
fn accept_all(l: &TcpListener, n: u32, wait: Duration) -> Result<Vec<TcpStream>, Error> {
    l.set_nonblocking(true)?;
    let until = Instant::now() + wait;
    let mut out = Vec::with_capacity(n as usize);
    while out.len() < n as usize {
        match l.accept() {
            Ok((s, _)) => {
                s.set_nonblocking(false)?;
                out.push(s);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > until {
                    return Err(Error::Config(format!(
                        "{} of {n} connections came within {} s",
                        out.len(),
                        wait.as_secs()
                    )));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}

/// Opens `n` connections to `to`, retrying a refused one for at most
/// `wait`.
fn connect_all(to: SocketAddr, n: u32, wait: Duration) -> Result<Vec<TcpStream>, Error> {
    let until = Instant::now() + wait;
    let mut out = Vec::with_capacity(n as usize);
    while out.len() < n as usize {
        match TcpStream::connect_timeout(&to, Duration::from_secs(1)) {
            Ok(s) => out.push(s),
            Err(e) if Instant::now() < until => {
                let _ = e;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}

/// `tcp_lat`: parses `a` and runs it.
///
/// # Errors
/// Any setting refused, any end's failure.
pub fn tcp_main(a: &Args, hooks: Hooks) -> Result<(), Error> {
    let c = Common::parse(a, TCP_KEYS)?;
    if a.has("nodelay") && !a.flag("nodelay") {
        return Err(Error::Config("--nodelay is always on".into()));
    }
    let listen = addr(a, "listen")?;
    let connect = addr(a, "connect")?;
    let (listener, to) = match (c.role, listen, connect) {
        (Role::Local, Some(l), None) => (Some(TcpListener::bind(l)?), None),
        (Role::Local, _, _) => {
            return Err(Error::Config(
                "role local: --listen IP:PORT only (the ping thread connects to it)".into(),
            ));
        }
        (_, Some(l), None) => (Some(TcpListener::bind(l)?), None),
        (_, None, Some(t)) => (None, Some(t)),
        _ => return Err(Error::Config("one of --listen or --connect".into())),
    };
    // a listening echo end is ready once it listens
    let said_ready = c.role == Role::Pong && listener.is_some();
    if said_ready {
        println!("ready");
    }
    let to = match (&listener, to) {
        (Some(l), None) if c.role == Role::Local => Some(l.local_addr()?),
        (_, t) => t,
    };
    let streams = c.spec.streams;
    let wait = Duration::from_secs(c.wait_s);
    let quickack = a.flag("quickack-per-read");
    let rto_min_us = if a.has("rto-min-us") {
        Some(a.num_or("rto-min-us", 0).map_err(cfg)?)
    } else {
        None
    };
    let delack_max_us = if a.has("delack-max-us") {
        Some(a.num_or("delack-max-us", 0).map_err(cfg)?)
    } else {
        None
    };
    let listener = listener.as_ref();
    let sockets = |end: End| -> Result<Vec<TcpStream>, Error> {
        match (c.role, end, listener, to) {
            (Role::Local, End::Pong, Some(l), _) => accept_all(l, streams, wait),
            (Role::Local, End::Ping, _, Some(t)) => connect_all(t, streams, wait),
            (_, _, Some(l), None) => accept_all(l, streams, wait),
            (_, _, None, Some(t)) => connect_all(t, streams, wait),
            _ => Err(Error::Config(
                "no address to listen on or connect to".into(),
            )),
        }
    };
    match c.imp {
        Impl::Mio => {
            let opts = MioTcpOptions {
                rcvbuf: c.rcvbuf,
                sndbuf: c.sndbuf,
                busy_read_us: c.busy_read_us,
                epoll_events: c.epoll_events as usize,
                quickack_per_read: quickack,
                rto_min_us,
                delack_max_us,
            };
            execute(
                &c,
                "tcp",
                |end, _cpu| MioTcp::new(sockets(end)?, opts.clone()),
                !said_ready,
                hooks,
            )
        }
        Impl::Ergon => {
            let helpers = c.helper_cpus();
            execute(
                &c,
                "tcp",
                |end, cpu| {
                    EioTcp::new(
                        sockets(end)?,
                        EioTcpOptions {
                            rcvbuf: c.rcvbuf,
                            sndbuf: c.sndbuf,
                            epoll_events: c.epoll_events,
                            quickack_per_read: quickack,
                            rto_min_us,
                            delack_max_us,
                            loop_cpu: u32::try_from(cpu).ok(),
                            helper_cpus: helpers.clone(),
                        },
                    )
                },
                !said_ready,
                hooks,
            )
        }
    }
}
