//! One load point of the transport latency suite, the same code in every arm.
//!
//! The common flags, the paced sender, the receive-side recording, the echo
//! and the files a point writes. An arm supplies only its transport, as a
//! [`Tx`] and an [`Rx`]; everything measured goes through here and
//! `ergon_latency_core`, so between arms only the technology changes.
//!
//! - **Send**: the TSC is read immediately before the send call; the slot's
//!   message (`seq`, that stamp, the slot's due time) is encoded between the
//!   read and the call.
//! - **Receive**: the TSC is read first thing in the callback that hands the
//!   payload over; then the payload is decoded (`Message::decode`) and
//!   recorded: latency from the intended and from the actual send time,
//!   stream 0 alone, loss, spikes, the max.
//! - **Echo**: the payload copied to a 64-byte buffer, `echo_in_place`, and
//!   sent back on its own stream, retried while the transport is busy.
//! - **Poll**: every pass reads each stream once, at most [`POLL_LIMIT`]
//!   messages from it.
//!
//! The hot paths never allocate or block: every buffer is built before the
//! schedule starts, and a binary that installs [`CountingAlloc`] reports
//! the allocations its busy threads made while measuring
//! (`hot_path_allocs` in the manifest, which the smoke tests require to be
//! 0).
//!
//! md-dpdk/harness's `aeron_lat` takes this file by path (that crate cannot
//! depend on this one), so it may use only `ergon_latency_core`, `libc` and
//! std.
#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use ergon_latency_core::clock::{TscClock, realtime_ns};
use ergon_latency_core::hist::{Latency, PointLatency};
use ergon_latency_core::loss::{Arrival, LossCounts, LossTracker};
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::{Kind, MESSAGE_LEN, Message, echo_in_place};
use ergon_latency_core::pace::{PaceStats, Pacer};
use ergon_latency_core::row::{Row, RowKind, Spikes, Stalls};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Messages read from one stream in one pass, at most.
pub const POLL_LIMIT: usize = 64;
/// Spike rows kept (the total counts past it).
pub const SPIKE_CAPACITY: usize = 100_000;
/// Stall rows kept.
pub const STALL_CAPACITY: usize = 1_000_000;
/// Sequence numbers each stream's loss tracker remembers behind its highest.
pub const LOSS_WINDOW: usize = 4096;
/// How long a receiver waits, once the schedule is done, for what is still
/// in flight.
pub const DRAIN_NS: u64 = 2_000_000_000;
/// The schedule starts this long after both ends are ready.
pub const START_DELAY_NS: u64 = 1_000_000;
/// Past twice the schedule and this, a point that has not finished fails.
pub const DEADLINE_SLACK_NS: u64 = 10_000_000_000;

/// What stops a point. A point that hits one exits non-zero and writes no
/// row.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A flag missing, unknown, repeated or not parsing.
    Usage(String),
    /// What the measurement core refused.
    Core(ergon_latency_core::Error),
    /// A file could not be written.
    Io {
        /// The file.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// The transport failed.
    Transport(String),
    /// The point did not finish in time.
    Timeout(String),
    /// This end stopped because the other end (another thread) failed: its
    /// error is the cause ([`both`]).
    Peer(&'static str),
    /// Payloads that did not decode as the expected message.
    Payload {
        /// How many.
        count: u64,
        /// The first one's error.
        first: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(why) => write!(f, "usage: {why}"),
            Self::Core(e) => write!(f, "{e}"),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Transport(why) => write!(f, "transport: {why}"),
            Self::Timeout(why) => write!(f, "timed out: {why}"),
            Self::Peer(who) => write!(f, "the {who} failed"),
            Self::Payload { count, first } => {
                write!(f, "{count} payload(s) refused, the first: {first}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<ergon_latency_core::Error> for Error {
    fn from(e: ergon_latency_core::Error) -> Self {
        Self::Core(e)
    }
}

/// `--key value` flags; a flag followed by another flag, or last, is `true`.
/// Every flag must be taken: [`Flags::finish`] refuses one left over.
#[derive(Debug, Default)]
pub struct Flags {
    map: BTreeMap<String, String>,
}

impl Flags {
    /// Parses `args` (without the program name).
    ///
    /// # Errors
    /// [`Error::Usage`] for a positional argument or a repeated flag.
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Self, Error> {
        let mut map = BTreeMap::new();
        let mut it = args.into_iter().peekable();
        while let Some(arg) = it.next() {
            let Some(key) = arg.strip_prefix("--") else {
                return Err(Error::Usage(format!("{arg:?}: every argument is a --flag")));
            };
            let value = match it.peek() {
                Some(next) if !next.starts_with("--") => it.next().unwrap_or_default(),
                _ => "true".to_owned(),
            };
            if map.insert(key.to_owned(), value).is_some() {
                return Err(Error::Usage(format!("--{key} given twice")));
            }
        }
        Ok(Self { map })
    }

    /// The flag's value, taken.
    pub fn take(&mut self, key: &str) -> Option<String> {
        self.map.remove(key)
    }

    /// The flag's value, taken; refused when absent.
    ///
    /// # Errors
    /// [`Error::Usage`] when it is not given.
    pub fn req(&mut self, key: &str) -> Result<String, Error> {
        self.take(key)
            .ok_or_else(|| Error::Usage(format!("--{key} is required")))
    }

    /// The flag parsed, or `default` when absent.
    ///
    /// # Errors
    /// [`Error::Usage`] when it does not parse.
    pub fn num<T: std::str::FromStr>(&mut self, key: &str, default: T) -> Result<T, Error> {
        self.take(key).map_or(Ok(default), |v| {
            v.parse()
                .map_err(|_| Error::Usage(format!("--{key} {v}: not a number")))
        })
    }

    /// `true` or `false`, or `default` when absent.
    ///
    /// # Errors
    /// [`Error::Usage`] for anything else.
    pub fn bool(&mut self, key: &str, default: bool) -> Result<bool, Error> {
        match self.take(key).as_deref() {
            None => Ok(default),
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(v) => Err(Error::Usage(format!("--{key} {v}: true or false"))),
        }
    }

    /// Refuses any flag not taken.
    ///
    /// # Errors
    /// [`Error::Usage`] naming them.
    pub fn finish(self) -> Result<(), Error> {
        if self.map.is_empty() {
            return Ok(());
        }
        let left: Vec<String> = self.map.keys().map(|k| format!("--{k}")).collect();
        Err(Error::Usage(format!("unknown flag(s) {}", left.join(" "))))
    }
}

/// Which end of a point this process is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Sends pings on the schedule and records their echoes (round trip).
    Ping,
    /// Echoes pings until the other end goes.
    Pong,
    /// Sends one-way messages on the schedule (same host).
    OnewaySend,
    /// Receives and records one-way messages (same host).
    OnewayRecv,
    /// Both ends in this process, on their own threads.
    Local,
}

impl Role {
    fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "ping" => Ok(Self::Ping),
            "pong" => Ok(Self::Pong),
            "oneway-send" => Ok(Self::OnewaySend),
            "oneway-recv" => Ok(Self::OnewayRecv),
            "local" => Ok(Self::Local),
            other => Err(Error::Usage(format!(
                "--role {other}: ping, pong, oneway-send, oneway-recv or local"
            ))),
        }
    }

    /// Its name on the command line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::Pong => "pong",
            Self::OnewaySend => "oneway-send",
            Self::OnewayRecv => "oneway-recv",
            Self::Local => "local",
        }
    }
}

/// The flags every binary of the suite takes.
#[derive(Debug, Clone)]
pub struct Common {
    /// `--role`.
    pub role: Role,
    /// `--round-trip` (local only): ping-pong between the two threads
    /// instead of one way.
    pub round_trip: bool,
    /// `--streams`, default 1.
    pub streams: u32,
    /// `--rate`, msg/s over every stream.
    pub rate: u64,
    /// `--warmup-s`, default 30.
    pub warmup_s: u64,
    /// `--duration-s`, default 30.
    pub duration_s: u64,
    /// `--size`: 64, the message, and nothing else.
    pub size: usize,
    /// `--cpus`: one CPU per busy thread in the binary's thread order;
    /// empty, every thread unpinned (smoke runs).
    pub cpus: Vec<usize>,
    /// `--label`.
    pub label: String,
    /// `--out`.
    pub out: PathBuf,
    /// `--spike-ns`, default 100 000.
    pub spike_ns: u64,
    /// `--stall-ns`, default 10 000.
    pub stall_ns: u64,
}

fn check_label(label: &str) -> Result<(), Error> {
    if label.is_empty() || label.contains([',', '\n', '\r', '/', '\0']) {
        return Err(Error::Usage(format!(
            "--label {label:?}: not empty, and no comma, slash or line break"
        )));
    }
    Ok(())
}

impl Common {
    /// Takes the common flags from `f`.
    ///
    /// # Errors
    /// [`Error::Usage`] for a flag missing or out of range.
    pub fn parse(f: &mut Flags) -> Result<Self, Error> {
        let role = Role::parse(&f.req("role")?)?;
        let round_trip = f.bool("round-trip", false)?;
        if round_trip && role != Role::Local {
            return Err(Error::Usage(
                "--round-trip is for --role local (ping and pong are a round trip already)".into(),
            ));
        }
        let c = Self {
            role,
            round_trip,
            streams: f.num("streams", 1)?,
            rate: if role == Role::Pong {
                f.num("rate", 0)?
            } else {
                f.req("rate")?
                    .parse()
                    .map_err(|_| Error::Usage("--rate: msg/s".into()))?
            },
            warmup_s: f.num("warmup-s", 30)?,
            duration_s: f.num("duration-s", 30)?,
            size: f.num("size", MESSAGE_LEN)?,
            cpus: match f.take("cpus") {
                None => Vec::new(),
                Some(csv) => csv
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        s.parse()
                            .map_err(|_| Error::Usage(format!("--cpus {csv}: CPU numbers")))
                    })
                    .collect::<Result<_, _>>()?,
            },
            label: f.req("label")?,
            out: PathBuf::from(f.req("out")?),
            spike_ns: f.num("spike-ns", 100_000)?,
            stall_ns: f.num("stall-ns", 10_000)?,
        };
        check_label(&c.label)?;
        if c.size != MESSAGE_LEN {
            return Err(Error::Usage(format!(
                "--size {}: every arm sends the {MESSAGE_LEN}-byte message",
                c.size
            )));
        }
        if c.streams == 0 {
            return Err(Error::Usage("--streams: at least 1".into()));
        }
        if c.role != Role::Pong && c.rate == 0 {
            return Err(Error::Usage("--rate: at least 1 msg/s".into()));
        }
        if c.duration_s == 0 && c.role != Role::Pong {
            return Err(Error::Usage("--duration-s: at least 1".into()));
        }
        let mut seen = c.cpus.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != c.cpus.len() {
            return Err(Error::Usage(format!(
                "--cpus {:?}: each busy thread its own CPU",
                c.cpus
            )));
        }
        Ok(c)
    }

    /// Whether this point is a round trip (`load` row) rather than one way.
    #[must_use]
    pub const fn is_round_trip(&self) -> bool {
        matches!(self.role, Role::Ping | Role::Pong) || self.round_trip
    }

    /// The warm-up's slots.
    ///
    /// # Errors
    /// [`Error::Usage`] when it overflows.
    pub fn warmup_slots(&self) -> Result<u64, Error> {
        self.rate
            .checked_mul(self.warmup_s)
            .ok_or_else(|| Error::Usage("--rate times --warmup-s overflows".into()))
    }

    /// The measured slots.
    ///
    /// # Errors
    /// [`Error::Usage`] when it overflows.
    pub fn measured_slots(&self) -> Result<u64, Error> {
        self.rate
            .checked_mul(self.duration_s)
            .ok_or_else(|| Error::Usage("--rate times --duration-s overflows".into()))
    }

    /// Refuses `--cpus` unless it names exactly `threads` CPUs (or none).
    ///
    /// # Errors
    /// [`Error::Usage`], naming the thread order.
    pub fn check_cpus(&self, threads: usize, order: &str) -> Result<(), Error> {
        if self.cpus.is_empty() || self.cpus.len() == threads {
            return Ok(());
        }
        Err(Error::Usage(format!(
            "--cpus {:?}: {threads} CPU(s) for --role {}, in this order: {order}",
            self.cpus,
            self.role.name()
        )))
    }

    /// Busy thread `i`'s CPU, `None` when unpinned.
    #[must_use]
    pub fn cpu(&self, i: usize) -> Option<usize> {
        self.cpus.get(i).copied()
    }

    /// The deadline past which a point fails, from raw `start_tsc`.
    #[must_use]
    pub fn deadline(&self, clock: &TscClock, start_tsc: u64) -> u64 {
        let slots = self
            .rate
            .saturating_mul(self.warmup_s.saturating_add(self.duration_s));
        let schedule_ns = slots.saturating_mul(1_000_000_000) / self.rate.max(1);
        start_tsc.saturating_add(
            clock.ns_to_ticks(
                schedule_ns
                    .saturating_mul(2)
                    .saturating_add(DEADLINE_SLACK_NS)
                    .saturating_add(DRAIN_NS),
            ),
        )
    }

    /// The point's settings for its manifest.
    pub fn settings(&self, m: &mut RunManifest) {
        let cpus: Vec<String> = self.cpus.iter().map(ToString::to_string).collect();
        m.setting("role", self.role.name())
            .setting("round_trip", &self.is_round_trip().to_string())
            .setting("streams", &self.streams.to_string())
            .setting("rate", &self.rate.to_string())
            .setting("size", &self.size.to_string())
            .setting("warmup_s", &self.warmup_s.to_string())
            .setting("duration_s", &self.duration_s.to_string())
            .setting(
                "cpus",
                if cpus.is_empty() {
                    "unpinned"
                } else {
                    "pinned"
                },
            )
            .setting("cpu_list", &cpus.join(","))
            .setting("label", &self.label)
            .setting("spike_ns", &self.spike_ns.to_string())
            .setting("stall_ns", &self.stall_ns.to_string())
            .setting("poll_limit", &POLL_LIMIT.to_string());
    }
}

/// Pins the calling thread to `cpu`; `None` leaves it where it is.
///
/// # Errors
/// [`Error::Usage`] when the kernel refuses it.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
pub fn pin(cpu: Option<usize>) -> Result<(), Error> {
    let Some(cpu) = cpu else { return Ok(()) };
    if cpu >= usize::try_from(libc::CPU_SETSIZE).unwrap_or(0) {
        return Err(Error::Usage(format!("CPU {cpu} past CPU_SETSIZE")));
    }
    // SAFETY: a zeroed cpu_set_t is the empty set; CPU_SET writes one bit
    // below CPU_SETSIZE (checked above); sched_setaffinity only reads it.
    let rc = unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(cpu, &mut set);
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw const set)
    };
    if rc != 0 {
        return Err(Error::Usage(format!(
            "pinning to CPU {cpu}: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

/// Pinning is Linux only: elsewhere a point runs unpinned (smoke runs), and
/// asking for a CPU is refused.
///
/// # Errors
/// [`Error::Usage`] for any CPU.
#[cfg(not(target_os = "linux"))]
pub fn pin(cpu: Option<usize>) -> Result<(), Error> {
    cpu.map_or(Ok(()), |c| {
        Err(Error::Usage(format!(
            "--cpus {c}: pinning is Linux only; leave --cpus out for a smoke run"
        )))
    })
}

thread_local! {
    /// This thread's allocations through [`CountingAlloc`].
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

/// The system allocator, counting each thread's allocations: a binary that
/// installs it (`#[global_allocator]`) reports what its busy threads
/// allocated while measuring ([`thread_allocs`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct CountingAlloc;

fn count_alloc() {
    // a const-initialised Cell has no destructor: this never fails
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
}

/// Allocations (and reallocations) this thread made through
/// [`CountingAlloc`] so far; 0 where it is not installed.
#[must_use]
pub fn thread_allocs() -> u64 {
    ALLOCS.try_with(Cell::get).unwrap_or(0)
}

// SAFETY: forwards every call to the system allocator unchanged; only adds a
// thread-local count.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_alloc();
        // SAFETY: the caller's contract, passed on.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_alloc();
        // SAFETY: the caller's contract, passed on.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, passed on.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_alloc();
        // SAFETY: the caller's contract, passed on.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// What a send attempt did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    /// The message went.
    Sent,
    /// The transport is full or busy: try again.
    Busy,
}

/// A transport's sending side, one stream per index.
pub trait Tx {
    /// Sends `msg` on stream `stream`, once, without blocking.
    ///
    /// # Errors
    /// [`Error::Transport`] when the transport failed for good.
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error>;
}

/// A transport's receiving side.
pub trait Rx {
    /// Reads each stream once, at most [`POLL_LIMIT`] messages from each,
    /// handing every payload to `on`; returns how many it read.
    ///
    /// # Errors
    /// [`Error::Transport`] when the transport failed.
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error>;

    /// Whether every sender is gone (an echo then stops).
    fn closed(&self) -> bool {
        false
    }
}

/// The largest latency of a point and where it was: what the spikes file's
/// `# max=` line says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Max {
    /// When it was due, ns from the schedule's start.
    pub due_ns: u64,
    /// Its latency from the intended send time, ns.
    pub latency_ns: u64,
    /// Its stream.
    pub stream: u32,
}

/// The receiving end's record of a point. Built before the schedule starts;
/// recording never allocates.
#[derive(Debug)]
pub struct Recorder {
    clock: TscClock,
    start_tsc: u64,
    streams: u64,
    warmup: u64,
    expect: Kind,
    latency: PointLatency,
    stream0: Latency,
    loss: LossTracker,
    spikes: Spikes,
    stalls: Stalls,
    max: Option<Max>,
    received: u64,
    refused: u64,
    first_refused: Option<ergon_latency_core::Error>,
}

impl Recorder {
    /// The record of `c`'s point, its schedule from raw `start_tsc`,
    /// expecting echoes (a round trip) or one-way messages.
    ///
    /// # Errors
    /// What the core refuses (histogram bounds, no streams).
    pub fn new(clock: &TscClock, start_tsc: u64, c: &Common) -> Result<Self, Error> {
        Ok(Self {
            clock: clock.clone(),
            start_tsc,
            streams: u64::from(c.streams),
            warmup: c.warmup_slots()?,
            expect: if c.is_round_trip() {
                Kind::Echo
            } else {
                Kind::Oneway
            },
            latency: PointLatency::new()?,
            stream0: Latency::new()?,
            loss: LossTracker::new(c.streams, LOSS_WINDOW)?,
            spikes: Spikes::new(c.spike_ns, SPIKE_CAPACITY),
            stalls: Stalls::new(c.stall_ns, STALL_CAPACITY, start_tsc),
            max: None,
            received: 0,
            refused: 0,
            first_refused: None,
        })
    }

    fn refuse(&mut self, e: ergon_latency_core::Error) {
        self.refused += 1;
        if self.first_refused.is_none() {
            self.first_refused = Some(e);
        }
    }

    /// A payload handed over at raw `recv` (read first thing in the
    /// callback that handed it over).
    #[inline]
    pub fn on_payload(&mut self, payload: &[u8], recv: u64) {
        let m = match Message::decode(payload) {
            Ok(m) => m,
            Err(e) => return self.refuse(e),
        };
        if m.kind != self.expect {
            return self.refuse(ergon_latency_core::Error::MessageKind(m.kind as u32));
        }
        match self.loss.on_message(m.stream, m.seq) {
            Err(e) => return self.refuse(e),
            // counted by the loss tracker; one sequence number, one latency
            Ok(Arrival::Duplicate | Arrival::Stale) => return,
            Ok(Arrival::InOrder | Arrival::Gap { .. } | Arrival::Reordered) => {}
        }
        self.received += 1;
        let index = m
            .seq
            .saturating_mul(self.streams)
            .saturating_add(u64::from(m.stream));
        if index < self.warmup {
            return;
        }
        self.latency.record(&self.clock, &m, recv);
        let ns = self.clock.delta_ns(m.intended_tsc, recv);
        if m.stream == 0 {
            self.stream0.record(ns);
        }
        let due_ns = self.clock.delta_ns(self.start_tsc, m.intended_tsc);
        self.spikes.record(m.stream, due_ns, ns, index);
        self.note_max(due_ns, ns, m.stream);
    }

    #[inline]
    fn note_max(&mut self, due_ns: u64, latency_ns: u64, stream: u32) {
        if self.max.is_none_or(|x| latency_ns > x.latency_ns) {
            self.max = Some(Max {
                due_ns,
                latency_ns,
                stream,
            });
        }
    }

    /// One pass of the receiving loop at raw `now` (its stalls).
    #[inline]
    pub fn pass(&mut self, now: u64) {
        self.stalls.pass(&self.clock, now);
    }

    /// Distinct messages received, warm-up included (a duplicate is the
    /// loss tracker's, not counted here).
    #[must_use]
    pub const fn received(&self) -> u64 {
        self.received
    }

    /// Slot `index`, due at raw `intended`, was owed at raw `end` and never
    /// sent: recorded censored at its lower bound.
    fn record_owed(&mut self, index: u64, intended: u64, end: u64) {
        self.latency.record_owed(&self.clock, intended, None, end);
        let ns = self.clock.delta_ns(intended, end);
        // below `streams`, a u32
        let stream = u32::try_from(index % self.streams).unwrap_or(0);
        if stream == 0 {
            self.stream0.record_censored(ns);
        }
        let due_ns = self.clock.delta_ns(self.start_tsc, intended);
        self.spikes.record(stream, due_ns, ns, index);
        self.note_max(due_ns, ns, stream);
    }
}

/// The sending end's record of a point.
#[derive(Debug, Clone)]
pub struct SendStats {
    /// The schedule, as it ended.
    pub pacer: Pacer,
    /// Slots that met back-pressure at least once.
    pub backpressured: u64,
    /// Raw counter when the last slot went (0: it never did).
    pub send_end_tsc: u64,
}

/// Both ends' results; when both failed, the cause, not the end that
/// stopped because of it ([`Error::Peer`]).
///
/// # Errors
/// The cause.
pub fn both<T, U>(a: Result<T, Error>, b: Result<U, Error>) -> Result<(T, U), Error> {
    match (a, b) {
        (Ok(a), Ok(b)) => Ok((a, b)),
        (Err(Error::Peer(_)), Err(cause)) => Err(cause),
        (Err(e), _) | (Ok(_), Err(e)) => Err(e),
    }
}

/// Where a one-way sender tells its receiver (another thread) it is done.
#[derive(Debug, Default)]
pub struct Progress {
    sent: AtomicU64,
    done: AtomicBool,
    failed: AtomicBool,
}

impl Progress {
    /// The sender finished, having sent `sent` messages.
    pub fn finish(&self, sent: u64) {
        self.sent.store(sent, Ordering::Release);
        self.done.store(true, Ordering::Release);
    }

    /// The sender failed: the receiver stops too.
    pub fn fail(&self) {
        self.failed.store(true, Ordering::Release);
    }
}

fn timeout(what: &str) -> Error {
    Error::Timeout(format!(
        "{what}: the point ran past twice its schedule plus {} s",
        (DEADLINE_SLACK_NS + DRAIN_NS) / 1_000_000_000
    ))
}

#[inline]
fn stamp(slot: &ergon_latency_core::pace::Slot, send_tsc: u64, kind: Kind) -> [u8; MESSAGE_LEN] {
    Message {
        seq: slot.seq,
        send_tsc,
        intended_tsc: slot.intended_tsc,
        stream: slot.stream,
        kind,
    }
    .to_bytes()
}

/// Sends the slot due at `now`, if any; `Ok(false)` once every slot went.
#[inline]
fn send_due<T: Tx>(
    clock: &TscClock,
    pacer: &mut Pacer,
    tx: &mut T,
    now: u64,
    kind: Kind,
    stats: &mut (u64, u64, u64),
) -> Result<bool, Error> {
    let Some(slot) = pacer.poll(now) else {
        return Ok(!pacer.done());
    };
    // the TSC immediately before the send call
    let send_tsc = clock.raw();
    let msg = stamp(&slot, send_tsc, kind);
    match tx.try_send(slot.stream, &msg)? {
        Sent::Sent => {
            pacer.sent(clock, &slot, send_tsc)?;
            if pacer.done() {
                stats.2 = send_tsc;
            }
        }
        Sent::Busy => {
            if stats.1 != slot.index {
                stats.1 = slot.index;
                stats.0 += 1;
            }
        }
    }
    Ok(true)
}

/// The round trip's sending end on one thread: sends each slot when due
/// and records the echoes as they come, then waits [`DRAIN_NS`] at most for
/// what is still in flight.
///
/// # Errors
/// The transport's, the core's, or [`Error::Timeout`].
pub fn ping<T: Tx, R: Rx>(
    clock: &TscClock,
    c: &Common,
    start_tsc: u64,
    tx: &mut T,
    rx: &mut R,
    rec: &mut Recorder,
) -> Result<SendStats, Error> {
    let mut pacer = Pacer::new(
        clock,
        start_tsc,
        c.rate,
        c.streams,
        c.warmup_slots()?,
        c.measured_slots()?,
    )?;
    let deadline = c.deadline(clock, start_tsc);
    // (backpressured, the slot last back-pressured, send end)
    let mut stats = (0u64, u64::MAX, 0u64);
    loop {
        let now = clock.raw();
        rec.pass(now);
        if !send_due(clock, &mut pacer, tx, now, Kind::Ping, &mut stats)? {
            break;
        }
        rx.poll(&mut |p: &[u8]| {
            let recv = clock.raw();
            rec.on_payload(p, recv);
        })?;
        if now > deadline {
            return Err(timeout("ping"));
        }
    }
    let until = clock.raw().saturating_add(clock.ns_to_ticks(DRAIN_NS));
    while rec.received() < pacer.stats().sent {
        let now = clock.raw();
        if now > until {
            break;
        }
        rec.pass(now);
        rx.poll(&mut |p: &[u8]| {
            let recv = clock.raw();
            rec.on_payload(p, recv);
        })?;
    }
    Ok(SendStats {
        pacer,
        backpressured: stats.0,
        send_end_tsc: stats.2,
    })
}

/// The one-way sending end: sends each slot when due, then tells the
/// receiver through `progress`.
///
/// # Errors
/// The transport's, the core's, or [`Error::Timeout`]; `progress` is
/// marked failed first.
pub fn send_oneway<T: Tx>(
    clock: &TscClock,
    c: &Common,
    start_tsc: u64,
    tx: &mut T,
    progress: &Progress,
) -> Result<SendStats, Error> {
    let mut run = || -> Result<SendStats, Error> {
        let mut pacer = Pacer::new(
            clock,
            start_tsc,
            c.rate,
            c.streams,
            c.warmup_slots()?,
            c.measured_slots()?,
        )?;
        let deadline = c.deadline(clock, start_tsc);
        let mut stats = (0u64, u64::MAX, 0u64);
        loop {
            let now = clock.raw();
            if !send_due(clock, &mut pacer, tx, now, Kind::Oneway, &mut stats)? {
                break;
            }
            if progress.failed.load(Ordering::Acquire) {
                return Err(Error::Peer("receiver"));
            }
            if now > deadline {
                return Err(timeout("oneway sender"));
            }
        }
        Ok(SendStats {
            pacer,
            backpressured: stats.0,
            send_end_tsc: stats.2,
        })
    };
    match run() {
        Ok(s) => {
            progress.finish(s.pacer.stats().sent);
            Ok(s)
        }
        Err(e) => {
            progress.fail();
            Err(e)
        }
    }
}

/// The one-way receiving end: records every message until the sender is
/// done and everything it sent came, or [`DRAIN_NS`] after it was done.
///
/// # Errors
/// The transport's, the sender's failure, or [`Error::Timeout`];
/// `progress` is marked failed first.
pub fn recv_oneway<R: Rx>(
    clock: &TscClock,
    c: &Common,
    start_tsc: u64,
    rx: &mut R,
    rec: &mut Recorder,
    progress: &Progress,
) -> Result<(), Error> {
    let deadline = c.deadline(clock, start_tsc);
    let mut run = || -> Result<(), Error> {
        let mut drain_until = 0u64;
        loop {
            let now = clock.raw();
            rec.pass(now);
            rx.poll(&mut |p: &[u8]| {
                let recv = clock.raw();
                rec.on_payload(p, recv);
            })?;
            if progress.failed.load(Ordering::Acquire) {
                return Err(Error::Peer("sender"));
            }
            if drain_until == 0 && progress.done.load(Ordering::Acquire) {
                drain_until = now.saturating_add(clock.ns_to_ticks(DRAIN_NS));
            }
            if drain_until != 0
                && (rec.received() >= progress.sent.load(Ordering::Acquire) || now > drain_until)
            {
                return Ok(());
            }
            if now > deadline {
                return Err(timeout("oneway receiver"));
            }
        }
    };
    run().inspect_err(|_| progress.fail())
}

/// The echoing end; returns the echoes sent.
///
/// Each ping is copied to a 64-byte buffer, turned into its echo in place
/// and sent back on its own stream (retried while the transport is busy),
/// until `rx` is closed or `keep_going` says stop (asked every 1024 empty
/// passes, and while an echo is retried).
///
/// # Errors
/// The transport's, or [`Error::Payload`] for a payload that is not a ping.
pub fn echo<R: Rx, T: Tx>(
    rx: &mut R,
    tx: &mut T,
    keep_going: &mut impl FnMut() -> bool,
) -> Result<u64, Error> {
    let mut echoes = 0u64;
    let mut failed: Option<Error> = None;
    let mut idle = 0u32;
    loop {
        let read = rx.poll(&mut |p: &[u8]| {
            if failed.is_some() {
                return;
            }
            let mut buf = [0u8; MESSAGE_LEN];
            let Some(src) = p.get(..MESSAGE_LEN).filter(|_| p.len() == MESSAGE_LEN) else {
                failed = Some(Error::Payload {
                    count: 1,
                    first: ergon_latency_core::Error::MessageLength(p.len()).to_string(),
                });
                return;
            };
            buf.copy_from_slice(src);
            let m = match echo_in_place(&mut buf) {
                Ok(m) => m,
                Err(e) => {
                    failed = Some(Error::Payload {
                        count: 1,
                        first: e.to_string(),
                    });
                    return;
                }
            };
            let mut spins = 0u32;
            loop {
                match tx.try_send(m.stream, &buf) {
                    Ok(Sent::Sent) => break,
                    Ok(Sent::Busy) => {
                        spins = spins.wrapping_add(1);
                        if spins.is_multiple_of(1024) && !keep_going() {
                            failed = Some(Error::Transport(
                                "stopped while an echo was back-pressured".into(),
                            ));
                            return;
                        }
                        std::hint::spin_loop();
                    }
                    Err(e) => {
                        failed = Some(e);
                        return;
                    }
                }
            }
            echoes += 1;
        })?;
        if let Some(e) = failed {
            return Err(e);
        }
        if read == 0 {
            idle = idle.wrapping_add(1);
            if idle.is_multiple_of(1024) && (rx.closed() || !keep_going()) {
                return Ok(echoes);
            }
        } else {
            idle = 0;
        }
    }
}

/// What the arm adds to a point's row and manifest.
#[derive(Debug)]
pub struct Report<'a> {
    /// The row's channel column (`ipc`, `udp`, `tcp`).
    pub channel: &'a str,
    /// Retransmits the transport counted; `None` where it has no counter.
    pub retrans: Option<u64>,
    /// Bytes on the wire per message, headers included.
    pub wire_bytes: u64,
    /// Busy-spinning threads the arm runs on this host.
    pub busy_threads: u32,
    /// Allocations the busy threads made while measuring.
    pub hot_path_allocs: u64,
    /// The arm's manifest, its own settings in it.
    pub manifest: RunManifest,
}

fn write(path: PathBuf, body: &str) -> Result<(), Error> {
    std::fs::write(&path, body).map_err(|source| Error::Io { path, source })
}

/// `csv` (the core's spikes file) with the `# max=<due_ns>,<latency_ns>,<stream>`
/// line after its two header comments.
#[must_use]
pub fn with_max_line(csv: &str, max: Option<Max>) -> String {
    let Some(m) = max else {
        return csv.to_owned();
    };
    let line = format!("# max={},{},{}\n", m.due_ns, m.latency_ns, m.stream);
    let mut cut = 0;
    for _ in 0..2 {
        match csv[cut..].find('\n') {
            Some(i) => cut += i + 1,
            None => break,
        }
    }
    format!("{}{line}{}", &csv[..cut], &csv[cut..])
}

/// Each stream's share of `sent` slots (slot `k` goes to stream `k mod n`).
#[must_use]
pub fn expected_per_stream(sent: u64, streams: u32) -> Vec<u64> {
    let n = u64::from(streams.max(1));
    (0..n).map(|i| sent / n + u64::from(i < sent % n)).collect()
}

/// The point's numbers once it ended.
struct Ended {
    pace: PaceStats,
    loss: LossCounts,
    achieved: f64,
}

/// What the schedule owed at raw `end` and never sent, recorded censored;
/// the loss totals and the achieved rate.
#[allow(clippy::cast_precision_loss)] // a rate to print
fn end_point(
    c: &Common,
    clock: &TscClock,
    rec: &mut Recorder,
    send: &mut SendStats,
    end: u64,
) -> Result<Ended, Error> {
    for index in send.pacer.finish(end) {
        let intended = send.pacer.due(index);
        rec.record_owed(index, intended, end);
    }
    let pace = send.pacer.stats();
    let loss = rec
        .loss
        .totals(&expected_per_stream(pace.sent, c.streams))?;
    let warmup = c.warmup_slots()?;
    let measured_sent = pace.sent.saturating_sub(warmup);
    let window_end = if send.send_end_tsc == 0 {
        end
    } else {
        send.send_end_tsc
    };
    let window_ns = clock.delta_ns(send.pacer.due(warmup), window_end);
    let achieved = if measured_sent == 0 || window_ns == 0 {
        0.0
    } else {
        measured_sent as f64 * 1e9 / window_ns as f64
    };
    Ok(Ended {
        pace,
        loss,
        achieved,
    })
}

/// The files beside the row: the two histograms, spikes (with the `# max=`
/// line) and stalls.
fn write_files(c: &Common, rec: &Recorder, start_realtime_ns: u128) -> Result<(), Error> {
    let (out, label) = (&c.out, &c.label);
    write(
        out.join(format!("histogram-{label}.csv")),
        &rec.latency.from_intended.csv(),
    )?;
    write(
        out.join(format!("histogram-send-{label}.csv")),
        &rec.latency.from_send.csv(),
    )?;
    write(
        out.join(format!("spikes-{label}.csv")),
        &with_max_line(&rec.spikes.csv(start_realtime_ns), rec.max),
    )?;
    write(
        out.join(format!("stalls-{label}.csv")),
        &rec.stalls.csv(start_realtime_ns),
    )
}

/// A point whose row stands for more streams than the one it measured.
///
/// `aeron_lat --load mixed`: a hot stream measured as every arm's is, beside
/// background streams of their own schedule. The row and the manifest's
/// `rate` and `streams` are the point's, every stream's; its latency
/// columns the measured stream's; its achieved rate theirs together.
#[derive(Debug)]
pub struct Shown<'a> {
    /// msg/s over every stream (the row's rate).
    pub rate: u64,
    /// Every stream.
    pub streams: u32,
    /// What the other streams achieved, msg/s, added to the measured one's.
    pub achieved_extra: f64,
    /// The other streams' latencies: `histogram-bg-<label>.csv`, and with
    /// the measured stream's `histogram-all-<label>.csv` (`bg.*` and `all.*`
    /// in the manifest).
    pub others: &'a Latency,
}

/// Ends the point at raw `end` and writes its files; returns the row.
///
/// What the schedule owed and never sent is recorded censored. `--out` gets
/// `results.csv` (the row), `manifest.json`, `histogram-<label>.csv`,
/// `histogram-send-<label>.csv` (latency from the actual send time),
/// `spikes-<label>.csv` (with its `# max=` line) and `stalls-<label>.csv`.
///
/// # Errors
/// [`Error::Payload`] when any payload was refused (the arm is broken, so
/// no row), [`Error::Io`] for a file, or the core's.
pub fn finish(
    c: &Common,
    clock: &TscClock,
    rec: Recorder,
    send: SendStats,
    start_realtime_ns: u128,
    end: u64,
    report: Report<'_>,
) -> Result<String, Error> {
    finish_shown(c, clock, rec, send, start_realtime_ns, end, report, None)
}

/// A latency summary into the manifest as `<prefix>.<stat>`.
fn summary_settings(m: &mut RunManifest, prefix: &str, s: &ergon_latency_core::hist::Summary) {
    for (k, v) in [
        ("min_ns", s.min),
        ("p50_ns", s.p50),
        ("p90_ns", s.p90),
        ("p99_ns", s.p99),
        ("p999_ns", s.p999),
        ("p9999_ns", s.p9999),
        ("max_ns", s.max),
        ("count", s.count),
        ("censored", s.censored),
    ] {
        m.setting(&format!("{prefix}.{k}"), &v.to_string());
    }
}

/// [`finish`], the row and manifest standing for `shown`'s streams where
/// given (`c` is then the measured stream's own schedule).
///
/// # Errors
/// As [`finish`].
#[allow(clippy::too_many_arguments)]
pub fn finish_shown(
    c: &Common,
    clock: &TscClock,
    mut rec: Recorder,
    mut send: SendStats,
    start_realtime_ns: u128,
    end: u64,
    report: Report<'_>,
    shown: Option<&Shown<'_>>,
) -> Result<String, Error> {
    if rec.refused > 0 {
        return Err(Error::Payload {
            count: rec.refused,
            first: rec
                .first_refused
                .as_ref()
                .map_or_else(String::new, ToString::to_string),
        });
    }
    let Ended {
        pace,
        loss,
        mut achieved,
    } = end_point(c, clock, &mut rec, &mut send, end)?;
    if let Some(s) = shown {
        achieved += s.achieved_extra;
    }
    let latency = rec.latency.from_intended.summary();
    let line = Row {
        kind: if c.is_round_trip() {
            RowKind::Load
        } else {
            RowKind::Oneway
        },
        label: &c.label,
        channel: report.channel,
        size: c.size,
        rate: shown.map_or(c.rate, |s| s.rate),
        latency,
        stream0: rec.stream0.summary(),
        pace,
        received: rec.received,
        backpressured: send.backpressured,
        achieved,
        loss,
        retrans: report.retrans,
        wire_bytes: report.wire_bytes,
        busy_threads: report.busy_threads,
    }
    .line()?;
    std::fs::create_dir_all(&c.out).map_err(|source| Error::Io {
        path: c.out.clone(),
        source,
    })?;
    write(c.out.join("results.csv"), &format!("{line}\n"))?;
    write_files(c, &rec, start_realtime_ns)?;
    let mut m = report.manifest;
    c.settings(&mut m);
    if let Some(s) = shown {
        let mut all = rec.latency.from_intended.clone();
        all.add(s.others)?;
        write(
            c.out.join(format!("histogram-bg-{}.csv", c.label)),
            &s.others.csv(),
        )?;
        write(
            c.out.join(format!("histogram-all-{}.csv", c.label)),
            &all.csv(),
        )?;
        m.setting("measured_rate", &c.rate.to_string())
            .setting("measured_streams", &c.streams.to_string())
            .setting("rate", &s.rate.to_string())
            .setting("streams", &s.streams.to_string());
        summary_settings(&mut m, "bg", &s.others.summary());
        summary_settings(&mut m, "all", &all.summary());
    }
    let from_send = rec.latency.from_send.summary();
    let retrans = report
        .retrans
        .map_or_else(|| "none".to_owned(), |r| r.to_string());
    m.setting("kind", if c.is_round_trip() { "load" } else { "oneway" })
        .setting("channel", report.channel)
        .setting(
            "clock_ticks_per_ns",
            &format!("{:.6}", clock.ticks_per_ns()),
        )
        .setting("start_realtime_ns", &start_realtime_ns.to_string())
        .setting("scheduled", &pace.scheduled.to_string())
        .setting("sent", &pace.sent.to_string())
        .setting("late", &pace.late.to_string())
        .setting("late_flagged", &pace.flagged().to_string())
        .setting("max_send_lag_ns", &pace.max_send_lag_ns.to_string())
        .setting("censored", &latency.censored.to_string())
        .setting("received", &rec.received.to_string())
        .setting("lost", &loss.lost.to_string())
        .setting("reordered", &loss.reordered.to_string())
        .setting("dup", &loss.duplicate.to_string())
        .setting("stale", &loss.stale.to_string())
        .setting("backpressured", &send.backpressured.to_string())
        .setting("achieved", &format!("{achieved:.0}"))
        .setting("max_ns", &latency.max.to_string())
        .setting("max_from_send_ns", &from_send.max.to_string())
        .setting("p50_from_send_ns", &from_send.p50.to_string())
        .setting("retrans", &retrans)
        .setting("wire_bytes", &report.wire_bytes.to_string())
        .setting("busy_threads", &report.busy_threads.to_string())
        .setting("hot_path_allocs", &report.hot_path_allocs.to_string());
    write(c.out.join("manifest.json"), &m.to_json())?;
    Ok(line)
}

/// Writes a manifest alone (an end that records no row: a pong).
///
/// # Errors
/// [`Error::Io`].
pub fn write_manifest(c: &Common, mut m: RunManifest) -> Result<(), Error> {
    c.settings(&mut m);
    std::fs::create_dir_all(&c.out).map_err(|source| Error::Io {
        path: c.out.clone(),
        source,
    })?;
    write(c.out.join("manifest.json"), &m.to_json())
}

/// The realtime clock and the raw counter read together, `delay_ns` ahead:
/// a schedule's start.
#[must_use]
pub fn start_at(clock: &TscClock, delay_ns: u64) -> (u64, u128) {
    let t = clock.raw().saturating_add(clock.ns_to_ticks(delay_ns));
    (t, realtime_ns() + u128::from(delay_ns))
}
