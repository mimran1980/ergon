//! One point's measurement, the same code for every arm: the sending and
//! receiving end ([`run_ping`]) and the echoing end ([`run_pong`]), each
//! one busy-spinning thread over all of a [`Link`]'s streams.
//!
//! The ping end, per pass of its loop:
//!
//! 1. read the counter; when the schedule (`ergon_latency_core::pace`)
//!    owes a slot, read the counter again, encode the message
//!    (`seq`, that `send_tsc`, the slot's `intended_tsc`, its stream) and
//!    hand it to [`Link::send`] at once;
//! 2. [`Link::poll`]: for each message handed over, read the counter
//!    first, then decode it (`Message::decode`), account it
//!    (`ergon_latency_core::loss`) and, past the warm-up, record it
//!    (`ergon_latency_core::hist`, from the intended and from the actual
//!    send time), stream 0 alone too, and any spike.
//!
//! After the schedule, replies still in flight get up to [`DRAIN_NS`]. A
//! measured message without a reply then, or never sent, is recorded at
//! its lower bound and counted censored. Nothing on the loop allocates
//! ([`Hooks::hot`] brackets it for the allocation test).

use std::sync::atomic::{AtomicBool, Ordering};

use ergon_latency_core::clock::{TscClock, realtime_ns};
use ergon_latency_core::hist::{Latency, PointLatency, Summary};
use ergon_latency_core::loss::{Arrival, LossCounts, LossTracker};
use ergon_latency_core::message::{Kind, Message};
use ergon_latency_core::pace::{PaceStats, Pacer};
use ergon_latency_core::row::{Spikes, Stalls};

use super::{Error, Link};

/// How long replies still in flight get after the last send, ns.
pub const DRAIN_NS: u64 = 2_000_000_000;
/// How far ahead of now the schedule starts, ns.
const LEAD_NS: u64 = 1_000_000;
/// The loss tracker's window per stream (sequence numbers).
const LOSS_WINDOW: usize = 4_096;
/// Send stamps kept per stream, for messages owed at the end.
const SENT_RING: usize = 1_024;
/// Spike rows kept.
const SPIKE_ROWS: usize = 100_000;
/// Stall rows kept per thread.
const STALL_ROWS: usize = 1_000_000;

/// Called on each polling thread with `true` as its loop starts and
/// `false` as it ends: the allocation test counts what happens between.
#[derive(Debug, Clone, Copy)]
pub struct Hooks {
    /// The hot-loop bracket.
    pub hot: fn(bool),
}

const fn nothing(_: bool) {}

impl Default for Hooks {
    fn default() -> Self {
        Self { hot: nothing }
    }
}

/// A point's schedule and thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    /// Streams.
    pub streams: u32,
    /// Messages per second, every stream together.
    pub rate: u64,
    /// Warm-up, seconds: sent, not measured.
    pub warmup_s: u64,
    /// Measured, seconds.
    pub duration_s: u64,
    /// A latency at least this is a spike row, ns.
    pub spike_ns: u64,
    /// A gap between loop passes at least this is a stall row, ns.
    pub stall_ns: u64,
}

impl Spec {
    /// Slots in the warm-up.
    #[must_use]
    pub const fn warmup_slots(&self) -> u64 {
        self.rate.saturating_mul(self.warmup_s)
    }

    /// Measured slots.
    #[must_use]
    pub const fn measured_slots(&self) -> u64 {
        self.rate.saturating_mul(self.duration_s)
    }
}

/// The point's largest latency and where it was: `(due_ns from the
/// schedule's start, latency_ns, stream)`.
pub type Max = (u64, u64, u32);

/// What the ping end measured.
#[derive(Debug, Clone)]
pub struct PingOutcome {
    /// Every stream's latencies.
    pub latency: PointLatency,
    /// Stream 0's, from the intended send time.
    pub stream0: Latency,
    /// The schedule's counts.
    pub pace: PaceStats,
    /// Replies received, warm-up included.
    pub received: u64,
    /// Sends that met back-pressure at least once.
    pub backpressured: u64,
    /// The measured msg/s achieved.
    pub achieved: f64,
    /// Every stream's loss accounting summed.
    pub loss: LossCounts,
    /// The largest latency recorded, and where.
    pub max: Option<Max>,
    /// Latencies of at least the spike threshold.
    pub spikes: Spikes,
    /// The loop's own stalls.
    pub stalls: Stalls,
    /// `CLOCK_REALTIME` at the schedule's start, ns.
    pub start_realtime_ns: u128,
}

impl PingOutcome {
    /// Every stream's summary from the intended send time.
    #[must_use]
    pub fn summary(&self) -> Summary {
        self.latency.from_intended.summary()
    }

    /// The spikes file: the core's, then the `# max=<due_ns>,<latency_ns>,<stream>`
    /// line.
    #[must_use]
    pub fn spikes_csv(&self) -> String {
        let mut s = self.spikes.csv(self.start_realtime_ns);
        if let Some((due, lat, stream)) = self.max {
            s.push_str(&format!("# max={due},{lat},{stream}\n"));
        }
        s
    }
}

/// The ping end's state, touched by the loop and by every arrival.
struct Ping<'c> {
    clock: &'c TscClock,
    streams: u64,
    start: u64,
    warmup: u64,
    lat: PointLatency,
    stream0: Latency,
    loss: LossTracker,
    spikes: Spikes,
    /// per stream: one past the highest sequence number replied to
    next_rx: Vec<u64>,
    /// per stream: messages sent
    sent: Vec<u64>,
    /// per stream, `SENT_RING` send stamps by `seq % SENT_RING`
    sent_at: Vec<u64>,
    received: u64,
    max: Option<Max>,
}

impl Ping<'_> {
    #[inline]
    fn note(&mut self, stream: u32, intended: u64, latency: u64, seq: u64) {
        let due = self.clock.delta_ns(self.start, intended);
        self.spikes.record(stream, due, latency, seq);
        if self.max.is_none_or(|(_, m, _)| latency > m) {
            self.max = Some((due, latency, stream));
        }
    }

    /// One reply handed over at raw `rx`.
    #[inline]
    fn arrived(&mut self, rx: u64, stream: u32, payload: &[u8]) -> Result<(), Error> {
        let m = Message::decode(payload)?;
        if m.kind != Kind::Echo || m.stream != stream {
            return Err(Error::Protocol(format!(
                "stream {stream} got {:?} of stream {} (seq {})",
                m.kind, m.stream, m.seq
            )));
        }
        let arrival = self.loss.on_message(stream, m.seq)?;
        self.received += 1;
        let s = stream as usize;
        if m.seq >= self.next_rx[s] {
            self.next_rx[s] = m.seq + 1;
        }
        if matches!(arrival, Arrival::Duplicate | Arrival::Stale) {
            return Ok(());
        }
        if m.seq.saturating_mul(self.streams) + u64::from(stream) >= self.warmup {
            self.lat.record(self.clock, &m, rx);
            let l = self.clock.delta_ns(m.intended_tsc, rx);
            if stream == 0 {
                self.stream0.record(l);
            }
            self.note(stream, m.intended_tsc, l, m.seq);
        }
        Ok(())
    }

    /// Whether every stream has its last message's reply.
    fn all_back(&self) -> bool {
        self.next_rx.iter().zip(&self.sent).all(|(r, s)| r >= s)
    }

    /// A measured message owed at raw `end` without a reply: recorded at
    /// its lower bound, censored.
    fn owed(&mut self, stream: u32, seq: u64, intended: u64, send: Option<u64>, end: u64) {
        self.lat.record_owed(self.clock, intended, send, end);
        let l = self.clock.delta_ns(intended, end);
        if stream == 0 {
            self.stream0.record_censored(l);
        }
        self.note(stream, intended, l, seq);
    }
}

/// The ping loop's own counts.
#[derive(Default)]
struct Sends {
    backpressured: u64,
    /// the slot that last met back-pressure, so it counts once
    pressed: Option<u64>,
    last_send: u64,
}

/// The hot loop: paced sends and every reply, until the schedule is done
/// and every reply is in, or `deadline` (raw).
#[inline(never)]
fn ping_loop<L: Link>(
    link: &mut L,
    clock: &TscClock,
    pacer: &mut Pacer,
    st: &mut Ping<'_>,
    stalls: &mut Stalls,
    deadline: u64,
    sends: &mut Sends,
) -> Result<(), Error> {
    loop {
        let now = clock.raw();
        stalls.pass(clock, now);
        if let Some(slot) = pacer.poll(now) {
            // the stamp right before the send call
            let t = clock.raw();
            let bytes = Message {
                seq: slot.seq,
                send_tsc: t,
                intended_tsc: slot.intended_tsc,
                stream: slot.stream,
                kind: Kind::Ping,
            }
            .to_bytes();
            if link.send(slot.stream, &bytes)? {
                pacer.sent(clock, &slot, t)?;
                sends.last_send = t;
                let s = slot.stream as usize;
                st.sent[s] += 1;
                // SENT_RING is a power of two
                #[allow(clippy::cast_possible_truncation)] // masked below the ring
                let at = s * SENT_RING + (slot.seq as usize & (SENT_RING - 1));
                st.sent_at[at] = t;
            } else if sends.pressed != Some(slot.index) {
                sends.backpressured += 1;
                sends.pressed = Some(slot.index);
            }
        }
        link.poll(&mut |stream: u32, payload: &[u8]| {
            // the stamp first: the moment the transport handed it over
            let rx = clock.raw();
            st.arrived(rx, stream, payload)
        })?;
        if now >= deadline || (pacer.done() && st.all_back()) {
            return Ok(());
        }
    }
}

/// Runs the ping end of one point over `link`, on the calling thread
/// (pinned by the caller).
///
/// # Errors
/// The link fails, a reply is malformed or of another stream, or no
/// measured reply came at all.
pub fn run_ping<L: Link>(
    link: &mut L,
    clock: &TscClock,
    spec: &Spec,
    hooks: Hooks,
) -> Result<PingOutcome, Error> {
    if spec.streams != link.streams() {
        return Err(Error::Config(format!(
            "{} streams asked, the link has {}",
            spec.streams,
            link.streams()
        )));
    }
    let n = spec.streams as usize;
    let warmup = spec.warmup_slots();
    let measured = spec.measured_slots();
    if measured == 0 {
        return Err(Error::Config(
            "nothing to measure: rate x duration is 0".into(),
        ));
    }
    let rt0 = realtime_ns();
    let raw0 = clock.raw();
    let start = raw0 + clock.ns_to_ticks(LEAD_NS);
    let start_realtime_ns = rt0 + u128::from(LEAD_NS);
    let mut pacer = Pacer::new(clock, start, spec.rate, spec.streams, warmup, measured)?;
    let mut st = Ping {
        clock,
        streams: u64::from(spec.streams),
        start,
        warmup,
        lat: PointLatency::new()?,
        stream0: Latency::new()?,
        loss: LossTracker::new(spec.streams, LOSS_WINDOW)?,
        spikes: Spikes::new(spec.spike_ns, SPIKE_ROWS),
        next_rx: vec![0; n],
        sent: vec![0; n],
        sent_at: vec![u64::MAX; n * SENT_RING],
        received: 0,
        max: None,
    };
    let mut stalls = Stalls::new(spec.stall_ns, STALL_ROWS, start);
    let deadline = pacer.due(warmup + measured) + clock.ns_to_ticks(DRAIN_NS);
    let mut sends = Sends::default();

    (hooks.hot)(true);
    let looped = ping_loop(
        link,
        clock,
        &mut pacer,
        &mut st,
        &mut stalls,
        deadline,
        &mut sends,
    );
    (hooks.hot)(false);
    looped?;

    // What the schedule owed and never got back: each measured message
    // after its stream's last reply (sent, unanswered) and each one never
    // sent, at its lower bound, censored. A hole before a stream's last
    // reply is a loss (UDP), counted in `lost`, not a latency.
    let end = clock.raw();
    for stream in 0..spec.streams {
        let s = stream as usize;
        let last_sent = st.sent[s];
        for seq in st.next_rx[s]..last_sent {
            let index = seq * u64::from(spec.streams) + u64::from(stream);
            if index < warmup {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)] // masked below the ring
            let at = s * SENT_RING + (seq as usize & (SENT_RING - 1));
            let send = (last_sent - seq <= SENT_RING as u64).then(|| st.sent_at[at]);
            st.owed(stream, seq, pacer.due(index), send, end);
        }
    }
    for index in pacer.finish(end) {
        let slot = pacer.slot(index);
        st.owed(slot.stream, slot.seq, slot.intended_tsc, None, end);
    }
    let pace = pacer.stats();
    let measured_sent = pace.sent.saturating_sub(warmup);
    let window = clock.delta_ns(pacer.due(warmup), sends.last_send);
    #[allow(clippy::cast_precision_loss)] // a rate to print
    let achieved = if measured_sent == 0 || window == 0 {
        0.0
    } else {
        measured_sent as f64 * 1e9 / window as f64
    };
    let loss = st.loss.totals(&st.sent)?;
    if st.lat.from_intended.summary().count == 0 {
        return Err(Error::Protocol(
            "no measured reply and nothing owed: no latency to report".into(),
        ));
    }
    Ok(PingOutcome {
        latency: st.lat,
        stream0: st.stream0,
        pace,
        received: st.received,
        backpressured: sends.backpressured,
        achieved,
        loss,
        max: st.max,
        spikes: st.spikes,
        stalls,
        start_realtime_ns,
    })
}

/// What the echo end did.
#[derive(Debug, Clone)]
pub struct PongOutcome {
    /// Echoes sent.
    pub echoed: u64,
    /// The loop's own stalls.
    pub stalls: Stalls,
    /// `CLOCK_REALTIME` at the loop's start, ns.
    pub start_realtime_ns: u128,
    /// Why it stopped.
    pub ended: &'static str,
}

/// When the echo end stops.
#[derive(Debug, Clone, Copy)]
pub enum PongEnd<'a> {
    /// When the flag is set (both ends in one process).
    Flag(&'a AtomicBool),
    /// When a peer closes, or nothing came for `idle_ns` after the first
    /// ping; failing when no ping came within `wait_ns`.
    Idle {
        /// Quiet time that ends a run, ns.
        idle_ns: u64,
        /// The longest wait for the first ping, ns.
        wait_ns: u64,
    },
}

#[inline(never)]
fn pong_loop<L: Link>(
    link: &mut L,
    clock: &TscClock,
    end: PongEnd<'_>,
    stalls: &mut Stalls,
    echoed: &mut u64,
) -> Result<&'static str, Error> {
    let begin = clock.raw();
    let (idle, wait) = match end {
        PongEnd::Flag(_) => (u64::MAX, u64::MAX),
        PongEnd::Idle { idle_ns, wait_ns } => {
            (clock.ns_to_ticks(idle_ns), clock.ns_to_ticks(wait_ns))
        }
    };
    let mut last = begin;
    loop {
        let now = clock.raw();
        stalls.pass(clock, now);
        match link.echo() {
            Ok(0) => {}
            Ok(k) => {
                *echoed += k as u64;
                last = now;
            }
            Err(Error::Closed(_)) if *echoed > 0 => return Ok("peer closed"),
            Err(e) => return Err(e),
        }
        match end {
            PongEnd::Flag(stop) => {
                if stop.load(Ordering::Acquire) {
                    return Ok("stopped");
                }
            }
            PongEnd::Idle { .. } => {
                if *echoed > 0 && now.wrapping_sub(last) > idle {
                    return Ok("idle");
                }
                if *echoed == 0 && now.wrapping_sub(begin) > wait {
                    return Err(Error::Protocol("no ping came".into()));
                }
            }
        }
    }
}

/// Runs the echo end over `link` on the calling thread (pinned by the
/// caller) until `end` says stop.
///
/// # Errors
/// The link fails, a message is not a ping, or (`PongEnd::Idle`) no ping
/// came in time.
pub fn run_pong<L: Link>(
    link: &mut L,
    clock: &TscClock,
    stall_ns: u64,
    end: PongEnd<'_>,
    hooks: Hooks,
) -> Result<PongOutcome, Error> {
    let start_realtime_ns = realtime_ns();
    let mut stalls = Stalls::new(stall_ns, STALL_ROWS, clock.raw());
    let mut echoed = 0u64;
    (hooks.hot)(true);
    let looped = pong_loop(link, clock, end, &mut stalls, &mut echoed);
    (hooks.hot)(false);
    let ended = looped?;
    Ok(PongOutcome {
        echoed,
        stalls,
        start_realtime_ns,
        ended,
    })
}
