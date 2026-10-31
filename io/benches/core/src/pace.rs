//! The paced open-loop sender's schedule.
//!
//! Slot `k` is due at
//! `start + k * interval` on the raw counter (32.32 fixed point, so a long
//! run does not drift), one message per slot (burst 1), and goes to stream
//! `k mod N` as that stream's message `k / N`. With
//! [`with_burst(b)`](Pacer::with_burst) slots go in groups of `b`: group
//! `g = k / b` is due at `start + g * b * interval` (the same average rate)
//! and goes whole to stream `g mod N`, as its messages
//! `(g / N) * b ..` in order. The sender busy-spins until a
//! slot is due and stamps it with when it was due, whatever the replies do,
//! so time spent queued counts against the latency (no coordinated
//! omission).
//!
//! The first `warmup` slots are sent but not measured. Over the measured
//! ones the pacer counts the sends more than [`LATE_NS`] past their due time
//! and the worst lag; more than 1 % of sends late flags the point
//! ([`PaceStats::flagged`]): the generator, not the transport, set its
//! latency.

use crate::Error;
use crate::clock::TscClock;
use std::ops::Range;

/// A measured send this far past its due time is late.
pub const LATE_NS: u64 = 100_000;

/// One slot of the schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// Its index in the schedule, from 0.
    pub index: u64,
    /// `index mod streams`.
    pub stream: u32,
    /// `index / streams`: its sequence number within its stream.
    pub seq: u64,
    /// The raw counter it is due at.
    pub intended_tsc: u64,
    /// Whether it is past the warm-up.
    pub measured: bool,
}

/// What the sender did against its schedule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaceStats {
    /// Slots the run owed, warm-up included.
    pub scheduled: u64,
    /// Slots sent, warm-up included.
    pub sent: u64,
    /// Measured sends more than [`LATE_NS`] past their due time.
    pub late: u64,
    /// The worst measured send's lag past its due time, in ns.
    pub max_send_lag_ns: u64,
    /// Measured slots due before the run's end and never sent
    /// ([`Pacer::finish`]).
    pub censored: u64,
}

impl PaceStats {
    /// More than 1 % of sends late: the point's latency is the generator's.
    #[must_use]
    pub const fn flagged(&self) -> bool {
        self.late.saturating_mul(100) > self.sent
    }
}

/// The schedule and its counts. Never allocates.
#[derive(Debug, Clone)]
pub struct Pacer {
    start_tsc: u64,
    /// raw ticks per slot, 32.32 fixed point
    interval_fp: u128,
    streams: u32,
    burst: u64,
    warmup: u64,
    total: u64,
    next: u64,
    late_ticks: u64,
    max_lag_ticks: u64,
    stats: PaceStats,
}

impl Pacer {
    /// A schedule of `rate` slots per second from raw `start_tsc`, over
    /// `streams` streams: `warmup` slots, then `measured`.
    ///
    /// # Errors
    /// [`Error::Schedule`] for a zero rate, no streams, or a schedule whose
    /// length overflows.
    pub fn new(
        clock: &TscClock,
        start_tsc: u64,
        rate: u64,
        streams: u32,
        warmup: u64,
        measured: u64,
    ) -> Result<Self, Error> {
        if rate == 0 {
            return Err(Error::Schedule("a rate of 0 msg/s"));
        }
        if streams == 0 {
            return Err(Error::Schedule("no streams"));
        }
        let total = warmup
            .checked_add(measured)
            .ok_or(Error::Schedule("more than u64::MAX slots"))?;
        let second = u128::from(clock.ns_to_ticks(1_000_000_000));
        Ok(Self {
            start_tsc,
            interval_fp: (second << 32) / u128::from(rate),
            streams,
            burst: 1,
            warmup,
            total,
            next: 0,
            late_ticks: clock.ns_to_ticks(LATE_NS),
            max_lag_ticks: 0,
            stats: PaceStats {
                scheduled: total,
                ..PaceStats::default()
            },
        })
    }

    /// Sends slots in groups of `burst`, each group at one due time and to
    /// one stream (module docs). 1 is one slot at a time.
    ///
    /// # Errors
    /// [`Error::Schedule`] for a burst of 0.
    pub const fn with_burst(mut self, burst: u32) -> Result<Self, Error> {
        if burst == 0 {
            return Err(Error::Schedule("a burst of 0"));
        }
        self.burst = burst as u64;
        Ok(self)
    }

    /// The slots a group sends at once.
    #[must_use]
    pub fn burst(&self) -> u32 {
        // set from a u32
        u32::try_from(self.burst).unwrap_or(u32::MAX)
    }

    /// The raw counter slot `index` is due at.
    #[must_use]
    #[inline]
    pub fn due(&self, index: u64) -> u64 {
        let first = index - index % self.burst;
        let offset = (self.interval_fp * u128::from(first)) >> 32;
        self.start_tsc
            .wrapping_add(u64::try_from(offset).unwrap_or(u64::MAX))
    }

    /// Slot `index`, due or not.
    #[must_use]
    #[inline]
    pub fn slot(&self, index: u64) -> Slot {
        let streams = u64::from(self.streams);
        let group = index / self.burst;
        Slot {
            index,
            // below `streams`, a u32
            stream: u32::try_from(group % streams).unwrap_or(0),
            seq: group / streams * self.burst + index % self.burst,
            intended_tsc: self.due(index),
            measured: index >= self.warmup,
        }
    }

    /// The next slot when it is due at raw `now`; `None` before then, or
    /// once every slot is sent. Does not advance: [`sent`](Self::sent) does.
    #[must_use]
    #[inline]
    pub fn poll(&self, now: u64) -> Option<Slot> {
        if self.next >= self.total {
            return None;
        }
        let slot = self.slot(self.next);
        (now >= slot.intended_tsc).then_some(slot)
    }

    /// Busy-spins until the next slot is due and returns it; `None` once
    /// every slot is sent.
    #[must_use]
    #[inline]
    pub fn wait(&self, clock: &TscClock) -> Option<Slot> {
        if self.next >= self.total {
            return None;
        }
        let due = self.due(self.next);
        while clock.raw() < due {
            std::hint::spin_loop();
        }
        self.poll(u64::MAX)
    }

    /// `slot` went at raw `at`: the schedule moves to the next slot, and a
    /// measured send's lag is counted.
    ///
    /// # Errors
    /// [`Error::SlotOutOfTurn`] for any slot but the next.
    #[inline]
    pub fn sent(&mut self, clock: &TscClock, slot: &Slot, at: u64) -> Result<(), Error> {
        if self.done() || slot.index != self.next {
            return Err(Error::SlotOutOfTurn {
                expected: self.next,
                got: slot.index,
            });
        }
        if slot.measured {
            let lag = at.saturating_sub(slot.intended_tsc);
            if lag > self.late_ticks {
                self.stats.late += 1;
            }
            if lag > self.max_lag_ticks {
                self.max_lag_ticks = lag;
                self.stats.max_send_lag_ns = clock.delta_ns(slot.intended_tsc, at);
            }
        }
        self.next += 1;
        self.stats.sent += 1;
        Ok(())
    }

    /// Whether every slot is sent.
    #[must_use]
    pub const fn done(&self) -> bool {
        self.next >= self.total
    }

    /// The run ended at raw `end`: the measured slots due before it and never
    /// sent, which [`PaceStats::censored`] now counts. Their latency is at
    /// least `end` minus their due time
    /// ([`PointLatency::record_owed`](crate::hist::PointLatency::record_owed)).
    pub fn finish(&mut self, end: u64) -> Range<u64> {
        let from = self.next.max(self.warmup);
        let mut to = from;
        while to < self.total && self.due(to) < end {
            to += 1;
        }
        self.stats.censored = to - from;
        from..to
    }

    /// The counts so far.
    #[must_use]
    pub const fn stats(&self) -> PaceStats {
        self.stats
    }
}
