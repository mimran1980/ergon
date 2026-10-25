//! The counter clock every stamp reads.
//!
//! The raw TSC (`x86_64`) or CNTVCT (`aarch64`) through quanta, scaled to
//! nanoseconds with quanta's own calibration against the monotonic clock.
//! Never a millisecond clock.
//!
//! Messages carry raw counter values, so two processes on one host share a
//! timebase (one-way latency) and a round trip needs only the sender's.

use crate::Error;
use std::sync::atomic::{AtomicU64, Ordering};

/// How long [`TscClock::new`] watches the counter to learn ticks per ns.
const CALIBRATION_NS: u64 = 10_000_000;
/// How long a [`cross_cpu_check`] side waits for its peer before giving up.
const PEER_TIMEOUT_NS: u64 = 10_000_000_000;

/// The raw counter and its scale.
#[derive(Debug, Clone)]
pub struct TscClock {
    clock: quanta::Clock,
    /// ticks per ns, 32.32 fixed point
    ticks_per_ns_fp: u128,
}

impl Default for TscClock {
    fn default() -> Self {
        Self::new()
    }
}

impl TscClock {
    /// The clock, with ticks per ns measured over 10 ms of busy-spin.
    #[must_use]
    pub fn new() -> Self {
        let clock = quanta::Clock::new();
        let r0 = clock.raw();
        let mut r1 = r0;
        while clock.delta_as_nanos(r0, r1) < CALIBRATION_NS {
            std::hint::spin_loop();
            r1 = clock.raw();
        }
        let ns = u128::from(clock.delta_as_nanos(r0, r1).max(1));
        let ticks = u128::from(r1.wrapping_sub(r0));
        Self {
            clock,
            ticks_per_ns_fp: ((ticks << 32) / ns).max(1),
        }
    }

    /// The raw counter now.
    #[must_use]
    #[inline]
    pub fn raw(&self) -> u64 {
        self.clock.raw()
    }

    /// Nanoseconds from raw `start` to raw `end`; 0 when `end` is not after
    /// `start`.
    #[must_use]
    #[inline]
    pub fn delta_ns(&self, start: u64, end: u64) -> u64 {
        self.clock.delta_as_nanos(start, end)
    }

    /// Raw ticks in `ns` nanoseconds.
    #[must_use]
    #[inline]
    pub fn ns_to_ticks(&self, ns: u64) -> u64 {
        u64::try_from((u128::from(ns) * self.ticks_per_ns_fp) >> 32).unwrap_or(u64::MAX)
    }

    /// Raw ticks per nanosecond, for a run's manifest.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // a rate to print
    pub fn ticks_per_ns(&self) -> f64 {
        self.ticks_per_ns_fp as f64 / 4_294_967_296.0
    }
}

/// `CLOCK_REALTIME` now in ns since the epoch, to line a run's files up with
/// traces. Never a latency source.
#[must_use]
pub fn realtime_ns() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

/// What [`cross_cpu_check`] found between two CPUs' counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossCpu {
    /// The two CPUs, as given.
    pub cpus: (usize, usize),
    /// Handoffs compared: each a read on one CPU after the other's stamp.
    pub rounds: u64,
    /// Handoffs whose reader's counter was behind the writer's stamp.
    pub backward: u64,
    /// The furthest behind, in ns.
    pub max_backward_ns: u64,
    /// The shortest forward handoff, in ns: an upper bound on any skew the
    /// check cannot see, and the handoff's own cost.
    pub min_forward_ns: u64,
}

impl CrossCpu {
    /// Whether no read ever ran behind the other CPU's stamp.
    #[must_use]
    pub const fn consistent(&self) -> bool {
        self.backward == 0
    }
}

#[derive(Default)]
struct Side {
    backward: u64,
    max_backward: u64,
    min_forward: Option<u64>,
}

/// One side of the check: acts on every turn of its parity.
fn side(
    clock: &TscClock,
    stamp: &AtomicU64,
    turn: &AtomicU64,
    parity: u64,
    turns: u64,
) -> Result<Side, Error> {
    let mut out = Side::default();
    let mut t = parity;
    while t < turns {
        let since = clock.raw();
        let mut spins = 0u32;
        while turn.load(Ordering::Acquire) != t {
            std::hint::spin_loop();
            spins = spins.wrapping_add(1);
            if spins.is_multiple_of(4096) && clock.delta_ns(since, clock.raw()) > PEER_TIMEOUT_NS {
                // let the peer see a turn it is not waiting for and stop too
                turn.store(u64::MAX, Ordering::Release);
                return Err(Error::CrossCpu("the peer thread stopped answering"));
            }
        }
        let now = clock.raw();
        if t > 0 {
            let prev = stamp.load(Ordering::Acquire);
            if now < prev {
                out.backward += 1;
                out.max_backward = out.max_backward.max(prev - now);
            } else {
                let forward = clock.delta_ns(prev, now);
                out.min_forward = Some(out.min_forward.map_or(forward, |m| m.min(forward)));
            }
        }
        stamp.store(clock.raw(), Ordering::Release);
        turn.store(t + 1, Ordering::Release);
        t += 2;
    }
    Ok(out)
}

/// Checks that CPU `a`'s and CPU `b`'s counters agree.
///
/// A thread on each hands a counter stamp back and forth, `rounds` times
/// each way: each reads its own counter after the other's stamp and must
/// never be behind it. `pin` runs first on each
/// thread with its CPU, to pin it there (the core pins nothing itself).
///
/// # Errors
/// [`Error::CrossCpu`] when a side stops answering for 10 s or panics.
pub fn cross_cpu_check<F>(
    clock: &TscClock,
    a: usize,
    b: usize,
    rounds: u64,
    pin: F,
) -> Result<CrossCpu, Error>
where
    F: Fn(usize) + Sync,
{
    let stamp = AtomicU64::new(0);
    let turn = AtomicU64::new(0);
    // turn 0 only stamps; turns 1..=2 * rounds each compare
    let turns = rounds.saturating_mul(2).saturating_add(1);
    let (ra, rb) = std::thread::scope(|s| {
        let run = |cpu: usize, parity: u64| {
            let (pin, stamp, turn) = (&pin, &stamp, &turn);
            s.spawn(move || {
                pin(cpu);
                side(clock, stamp, turn, parity, turns)
            })
        };
        let (ha, hb) = (run(a, 0), run(b, 1));
        (ha.join(), hb.join())
    });
    let panicked = || Error::CrossCpu("a side panicked");
    let (sa, sb) = (ra.map_err(|_| panicked())??, rb.map_err(|_| panicked())??);
    let max_backward = sa.max_backward.max(sb.max_backward);
    Ok(CrossCpu {
        cpus: (a, b),
        rounds: rounds.saturating_mul(2),
        backward: sa.backward + sb.backward,
        max_backward_ns: clock.delta_ns(0, max_backward),
        min_forward_ns: match (sa.min_forward, sb.min_forward) {
            (Some(x), Some(y)) => x.min(y),
            (x, y) => x.or(y).unwrap_or(0),
        },
    })
}
