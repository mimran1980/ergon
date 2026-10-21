//! The front clock: an owner that keeps its segment on UTC with one cheap
//! update a second, followers that read it on other threads, and the
//! health check.

use crate::advanced::discipline::{Action, Discipline, DisciplineConfig, Phase};
use crate::advanced::share::{self, ClockPublisher, SharedClock};
use crate::arch;
use crate::calibrate::{self, Calibrated, Config, Report};
use crate::clock::{self, Clock as Segment, Domain, FACTOR_ONE, TimeSource};
use crate::detect::OrderedRead;
use crate::error::Error;
use crate::scale::{self, Conversion};
use crate::wall::{self, KernelSource, SampleSource, Sampler};
use crate::warn::{self, Warning};
use std::cell::Cell;

/// No accepted update for longer than this, ns: stale (the discipline's
/// holdover).
const STALE_NS: u64 = 5_000_000_000;
/// The update's period, ns: real work at most once a second.
const PERIOD_NS: u64 = 1_000_000_000;
/// The worst hardware rate error measured, 20.2 ppm (about 73 ms an hour,
/// `clock/BENCHMARKS.md`), rounded up, in parts per 10¹².
const DRIFT_PPT: u128 = 25_000_000;
/// The Linux fallback's dead band, ns: its factor stays one, its read an
/// add, while the wall clock is within this.
const FALLBACK_BAND_NS: u64 = 1_000;

/// Calibrates the clock, once, at start-up, and prints one line to stderr
/// per problem that makes it slow or its UTC stamps wrong.
///
/// The clock counts nanoseconds since 1970, UTC, by default
/// ([`Domain::Realtime`]); [`Domain::Monotonic`] gives nanoseconds since
/// boot. Blocks for tens of milliseconds (the counter's checks, its rate,
/// a verification; the time-sync probe, up to 50 ms more). Call it before
/// the hot loop starts, never on it.
///
/// The warnings ([`Report::warnings`]) say: the clock fell back to the
/// operating system's clock (slower), the host's clock is not synchronised,
/// still being corrected, known to UTC only loosely, or a leap second is
/// pending. `ERGON_HOST_CHECKS=off` (or `0`) or
/// [`Config::print_warnings`] turns the lines off; the report keeps them.
///
/// Not live yet and warned of a gap? Wait for time sync to settle and
/// calibrate again (before making followers): the new clock starts on the
/// corrected wall clock. A clock already running never steps: it slews
/// at most 0.5 ms a second.
///
/// # Errors
/// As [`advanced::calibrate`](crate::advanced::calibrate):
/// [`Error::CounterUnavailable`] under
/// [`Policy::RequireCounter`](crate::advanced::Policy::RequireCounter)
/// where the counter cannot be used; [`Error::ClockRead`] when a kernel
/// clock cannot be read; [`Error::NotPtpGrade`] under
/// [`TimeSyncPolicy::RequirePtp`](crate::advanced::TimeSyncPolicy::RequirePtp)
/// where the wall clock is not disciplined to a PTP hardware clock.
pub fn calibrate(config: &Config) -> Result<Clock, Error> {
    let Calibrated { clock, report } = calibrate::calibrate(config)?;
    let clock = Clock::keep(clock, report)?;
    warn::print(
        clock.report(),
        config.print_warnings,
        std::env::var_os(warn::HOST_CHECKS_ENV).as_deref(),
        &mut std::io::stderr(),
    );
    Ok(clock)
}

/// The calibrated clock, and its owner's state: one per process (or per
/// calibration), kept by the thread that calls [`update`](Self::update).
///
/// Reads are [`now_ns`](Self::now_ns) (unordered: event stamps, deadlines,
/// ages), [`now_ns_begin`](Self::now_ns_begin) (a measurement's start) and
/// [`now_ns_ordered`](Self::now_ns_ordered) (its end, and stamps that must
/// never be earlier than another thread's). Each is one counter read and one
/// multiply: the owner's state is never touched.
///
/// In UTC ([`Domain::Realtime`], the default) the owner calls
/// [`update`](Self::update) from its loop's idle time, about once a second
/// or more often: it keeps the clock on `CLOCK_REALTIME` (the rate within
/// ±500 ppm, never back) and returns its [`Health`]. Other threads read
/// through a [`Follower`] ([`follower`](Self::follower)).
///
/// Not `Copy` and not `Clone`: a copy would never follow the updates.
///
/// ```compile_fail,E0382
/// let clock = ergon_clock::calibrate(&ergon_clock::Config::default())?;
/// let moved = clock;
/// let _ = clock.now_ns();
/// # Ok::<(), ergon_clock::Error>(())
/// ```
#[derive(Debug)]
#[repr(C)]
pub struct Clock {
    /// What a read reads: first, so a read touches only these five words.
    segment: Segment,
    /// Everything else, off the read's cache lines.
    keeping: Box<Keeping>,
}

/// The owner's state.
#[derive(Debug)]
struct Keeping {
    report: Report,
    /// `None` in the monotonic domain: nothing to keep.
    utc: Option<Utc>,
    /// Made by the first [`Clock::follower`].
    sharing: Option<Sharing>,
}

/// What keeps a UTC clock on `CLOCK_REALTIME`.
#[derive(Debug)]
struct Utc {
    discipline: Discipline,
    sampler: Sampler,
    /// Ticks in [`PERIOD_NS`].
    period_ticks: u64,
    /// Ticks at the last sample taken (calibration's at first).
    last_try: u64,
    /// Ticks at the last accepted sample (calibration's at first).
    last_good: u64,
    /// `CLOCK_REALTIME` less the clock after the last accepted sample, ns.
    gap_ns: i64,
    /// That sample's error bound, ns.
    error_ns: u64,
    /// |the rate applied − the wall clock's rate|, ppt.
    slew_ppt: u64,
    rejected_in_a_row: u64,
}

/// The shared clock followers read, and its one writer.
#[derive(Debug)]
struct Sharing {
    shared: &'static SharedClock,
    publisher: ClockPublisher<'static>,
}

/// A shared clock on a cache line of its own: every follower read loads
/// its sequence, which must not share a line with data other threads
/// write.
#[repr(align(128))]
struct Aligned(SharedClock);

impl Sharing {
    /// A shared clock, leaked (128 bytes, once per clock that has
    /// followers), holding `segment`.
    fn new(segment: &Segment) -> Self {
        let leaked: &'static Aligned = Box::leak(Box::new(Aligned(SharedClock::new())));
        let mut publisher = ClockPublisher::claimed(&leaked.0);
        publisher.publish(segment);
        Self {
            shared: &leaked.0,
            publisher,
        }
    }
}

impl Utc {
    /// The keeping of `segment`, a realtime-domain calibration as made: its
    /// anchor is the first sample, and now is its last update.
    fn new(segment: &Segment, wall_res_ns: u64) -> Result<Self, Error> {
        // the Linux fallback: CLOCK_MONOTONIC runs at CLOCK_REALTIME's rate
        let band = (!segment.is_counter() && cfg!(any(target_os = "linux", target_os = "android")))
            .then_some(FALLBACK_BAND_NS);
        let cfg = DisciplineConfig {
            unit_rate_within_ns: band,
            ..DisciplineConfig::default()
        };
        let now = segment.ticks();
        Ok(Self {
            discipline: Discipline::new(segment, cfg)?,
            sampler: Sampler::for_clock(segment, wall_res_ns),
            period_ticks: segment.ns_to_ticks(PERIOD_NS).max(1),
            last_try: now,
            last_good: now,
            gap_ns: 0,
            error_ns: 0,
            slew_ppt: 0,
            rejected_in_a_row: 0,
        })
    }
}

impl Clock {
    /// The owner of `segment`, a calibration as made, with its report.
    pub(crate) fn keep(segment: Segment, report: Report) -> Result<Self, Error> {
        let utc = match segment.domain() {
            Domain::Monotonic => None,
            Domain::Realtime => Some(Utc::new(&segment, wall::realtime_res_ns()?)?),
        };
        Ok(Self {
            segment,
            keeping: Box::new(Keeping {
                report,
                utc,
                sharing: None,
            }),
        })
    }

    /// The time now, ns, unordered: the read may be taken before earlier
    /// work finishes. For event stamps, deadlines and ages. Never the start
    /// of a measurement: use [`now_ns_begin`](Self::now_ns_begin).
    #[inline]
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        self.segment.now_ns()
    }

    /// The time now, ns, ordered: taken after every earlier instruction
    /// has finished. For the end of a measurement, and stamps that must
    /// never be earlier than another thread's.
    #[inline]
    #[must_use]
    pub fn now_ns_ordered(&self) -> u64 {
        self.segment.now_ns_ordered()
    }

    /// The time now, ns, for the start of a measurement: ordered after
    /// earlier work and before the region's work can start
    /// ([`TimeSource::now_ns_begin`](crate::TimeSource::now_ns_begin)). The
    /// end is [`now_ns_ordered`](Self::now_ns_ordered).
    ///
    /// This stops the CPU, not the compiler only: `std::hint::black_box` is
    /// not a fence and does not order a stamp. It does not say that a store
    /// has reached memory or that a packet is on the wire — see the crate
    /// README's measurement section.
    #[inline]
    #[must_use]
    pub fn now_ns_begin(&self) -> u64 {
        self.segment.now_ns_begin()
    }

    /// Nanoseconds since `start_ns`, never negative.
    #[inline]
    #[must_use]
    pub fn since_ns(&self, start_ns: u64) -> u64 {
        TimeSource::since_ns(&self.segment, start_ns)
    }

    /// Keeps the clock on UTC: call it from the loop's idle time, as often
    /// as you like. Never prints, never blocks, never allocates.
    ///
    /// At most once a second it samples `CLOCK_REALTIME` (up to 8 brackets
    /// and 3 step probes, through the vDSO) and re-anchors the clock: the
    /// rate follows the wall clock's within ±500 ppm, and the clock never
    /// goes back. Measured: 0.3 µs on Graviton (max 2.9 µs), 3.5 µs on c7i
    /// (max 18.7 µs, `clock/BENCHMARKS.md`). Every call returns the
    /// [`Health`], which costs a counter read and some arithmetic (on the
    /// fallback, one `clock_gettime` too); the sample, on the calls that
    /// take one, is the rest. Followers adopt each new segment.
    ///
    /// Log [`Health::warning`] off the hot path, and only where
    /// [`Health::sampled`] says this call took a sample (else a gap or a
    /// stale clock would report on every call). In the monotonic domain
    /// there is nothing to keep.
    pub fn update(&mut self) -> Health {
        let now = self.segment.ticks();
        let reader = self.segment;
        self.update_at(now, &mut KernelSource(&reader))
    }

    /// [`update`](Self::update) at counter value `now_ticks`, sampling
    /// `src`.
    pub(crate) fn update_at(&mut self, now_ticks: u64, src: &mut impl SampleSource) -> Health {
        let mut sampled = false;
        if let Some(utc) = self.keeping.utc.as_mut()
            && now_ticks.saturating_sub(utc.last_try) >= utc.period_ticks
        {
            sampled = true;
            utc.last_try = now_ticks;
            let a = utc.discipline.sample_and_apply(
                &mut self.segment,
                &mut utc.sampler,
                src,
                Phase::Running,
            );
            if a.action == Action::Held {
                utc.rejected_in_a_row = utc.discipline.consecutive_rejects();
            } else {
                utc.last_good = now_ticks;
                utc.rejected_in_a_row = 0;
                utc.gap_ns = a.wall_offset_ns;
                utc.error_ns = a.error_ns;
            }
            utc.slew_ppt = a.rate_ppt.abs_diff(a.freq_ppt);
            if let Some(sharing) = self.keeping.sharing.as_mut() {
                sharing.publisher.publish(&self.segment);
            }
        }
        self.health_at(now_ticks, sampled)
    }

    /// The health now: one counter read, no system call.
    #[must_use]
    pub fn health(&self) -> Health {
        self.health_at(self.segment.ticks(), false)
    }

    /// The health at counter value `now_ticks`; `sampled` as
    /// [`update_at`](Self::update_at) took one.
    fn health_at(&self, now_ticks: u64, sampled: bool) -> Health {
        let gap_warn_ns = self.keeping.report.gap_warn_ns;
        let Some(utc) = &self.keeping.utc else {
            return Health {
                utc: false,
                sampled: false,
                since_update_ns: 0,
                gap_ns: 0,
                bound_ns: 0,
                catch_up_ns: 0,
                rejected_in_a_row: 0,
                gap_warn_ns,
            };
        };
        let since = scale::ticks_to_ns(
            now_ticks.saturating_sub(utc.last_good),
            utc.discipline.nominal_mult(),
        );
        let drift = u128::from(since) * (u128::from(utc.slew_ppt) + DRIFT_PPT) / 1_000_000_000_000;
        let gap = utc.gap_ns.unsigned_abs();
        Health {
            utc: true,
            sampled,
            since_update_ns: since,
            gap_ns: utc.gap_ns,
            bound_ns: gap
                .saturating_add(utc.error_ns)
                .saturating_add(u64::try_from(drift).unwrap_or(u64::MAX)),
            catch_up_ns: warn::catch_up_ns(gap),
            rejected_in_a_row: utc.rejected_in_a_row,
            gap_warn_ns,
        }
    }

    /// A reader for another thread: move it there at start-up. It reads
    /// this clock as last updated (one acquire load and a compare, then the
    /// same read).
    ///
    /// Make followers from the clock you keep. This clock's
    /// [`update`](Self::update) publishes to them; a *new* calibration does
    /// not: if this clock is dropped or replaced (by calibrating again),
    /// its followers keep its last segment and drift. Make them again from
    /// the new clock. A follower's reads can go back by about 1 ns at an
    /// update (more while the clock slews at the cap and the thread loses
    /// milliseconds between its check and its read); the owner's never do.
    #[must_use]
    pub fn follower(&mut self) -> Follower {
        let segment = self.segment;
        let sharing = self
            .keeping
            .sharing
            .get_or_insert_with(|| Sharing::new(&segment));
        Follower::new(sharing.shared, share::sequence(sharing.shared), segment)
    }

    /// What calibration chose and why (the domain is `report().domain`).
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.keeping.report
    }
}

impl TimeSource for Clock {
    #[inline]
    fn now_ns(&self) -> u64 {
        self.segment.now_ns()
    }
    #[inline]
    fn now_ns_ordered(&self) -> u64 {
        self.segment.now_ns_ordered()
    }
    #[inline]
    fn now_ns_begin(&self) -> u64 {
        self.segment.now_ns_begin()
    }
    #[inline]
    fn resolution_ns(&self) -> u64 {
        self.segment.resolution_ns()
    }
}

/// A fixed copy of `clock`'s segment, as last updated.
///
/// `Copy`, five words, and it never follows later updates (it drifts from
/// UTC by the hardware's rate error). For tests, tools and code that keeps
/// raw ticks.
#[must_use]
pub const fn segment(clock: &Clock) -> Segment {
    clock.segment
}

/// Another thread's reader of a [`Clock`]: made by
/// [`Clock::follower`], moved to its thread. `Send`, not `Sync`: one per
/// thread (a clone follows too).
///
/// Each read loads the owner's sequence (acquire) and compares it; when
/// the owner has updated, it adopts the new segment (cold), then reads as
/// the segment does, each field loaded where the read uses it: the
/// instructions of [`ClockFollower`](crate::advanced::share::ClockFollower)'s
/// read, within one register move (release, LTO, diffed on arm64 and
/// x86-64).
#[derive(Clone, Debug)]
pub struct Follower {
    shared: &'static SharedClock,
    seq: Cell<u64>,
    /// The counter's ordered read, or `None` for the fallback: the byte a
    /// read branches on.
    counter: Cell<Option<OrderedRead>>,
    /// The segment's conversion, field by field.
    base_ticks: Cell<u64>,
    base_ns: Cell<u64>,
    mult: Cell<u64>,
    /// The segment as published: what a refresh adopts from.
    segment: Cell<Segment>,
}

impl Follower {
    /// A follower of `shared`, holding `segment` as published at `seq`.
    fn new(shared: &'static SharedClock, seq: u64, segment: Segment) -> Self {
        let follower = Self {
            shared,
            seq: Cell::new(seq),
            counter: Cell::new(None),
            base_ticks: Cell::new(0),
            base_ns: Cell::new(0),
            mult: Cell::new(0),
            segment: Cell::new(segment),
        };
        follower.hold(segment);
        follower
    }

    /// Holds `segment`: its read and its conversion.
    fn hold(&self, segment: Segment) {
        let p = segment.params();
        self.counter.set(match segment {
            Segment::Counter(c) => Some(c.ordered_read()),
            Segment::Monotonic(_) => None,
        });
        self.base_ticks.set(p.base_ticks);
        self.base_ns.set(p.base_ns);
        self.mult.set(p.mult);
        self.segment.set(segment);
    }

    /// Adopts what the owner published since, if anything: one acquire load
    /// and a compare.
    #[inline]
    fn follow(&self) {
        let seq = share::sequence(self.shared);
        if seq != self.seq.get() {
            self.refresh(seq);
        }
    }

    /// The segment held after following the owner.
    #[cfg(test)]
    fn followed(&self) -> Segment {
        self.follow();
        self.segment.get()
    }

    /// Adopts what was published after the cached sequence.
    #[cold]
    #[inline(never)]
    fn refresh(&self, s1: u64) {
        if let Some((seq, segment)) =
            share::newer(self.shared, self.seq.get(), s1, &self.segment.get())
        {
            self.seq.set(seq);
            self.hold(segment);
        }
    }

    /// The time at `ticks`, ns: the segment's conversion.
    #[inline]
    const fn ns_at(&self, ticks: u64) -> u64 {
        Conversion {
            base_ticks: self.base_ticks.get(),
            base_ns: self.base_ns.get(),
            mult: self.mult.get(),
        }
        .ns_at(ticks)
    }

    /// The fallback's time at monotonic `ticks`: with a factor of one an
    /// add, as `MonotonicClock`'s read.
    #[inline]
    const fn fallback_ns(&self, ticks: u64) -> u64 {
        if self.mult.get() == FACTOR_ONE {
            ticks.wrapping_add(self.base_ns.get().wrapping_sub(self.base_ticks.get()))
        } else {
            clock::cold();
            self.ns_at(ticks)
        }
    }

    /// The time now, ns, unordered ([`Clock::now_ns`]).
    #[inline]
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        self.follow();
        match self.counter.get() {
            Some(_) => self.ns_at(arch::ticks()),
            None => self.fallback_ns(arch::clock_ns_unchecked(wall::MONOTONIC)),
        }
    }

    /// The time now, ns, ordered ([`Clock::now_ns_ordered`]).
    #[inline]
    #[must_use]
    pub fn now_ns_ordered(&self) -> u64 {
        self.follow();
        self.counter.get().map_or_else(
            || self.fallback_ns(arch::clock_ns_unchecked(wall::MONOTONIC)),
            |ordered| {
                let ticks = ordered.read();
                self.ns_at(ticks)
            },
        )
    }

    /// The time now, ns, for the start of a measurement ([`Clock::now_ns_begin`]).
    #[inline]
    #[must_use]
    pub fn now_ns_begin(&self) -> u64 {
        self.follow();
        self.counter.get().map_or_else(
            || self.fallback_ns(arch::clock_ns_unchecked(wall::MONOTONIC)),
            |ordered| {
                let ticks = ordered.read_begin();
                self.ns_at(ticks)
            },
        )
    }

    /// Nanoseconds since `start_ns`, never negative.
    #[inline]
    #[must_use]
    pub fn since_ns(&self, start_ns: u64) -> u64 {
        crate::clock::elapsed_ns(start_ns, self.now_ns())
    }
}

impl TimeSource for Follower {
    #[inline]
    fn now_ns(&self) -> u64 {
        Self::now_ns(self)
    }
    #[inline]
    fn now_ns_ordered(&self) -> u64 {
        Self::now_ns_ordered(self)
    }
    #[inline]
    fn now_ns_begin(&self) -> u64 {
        Self::now_ns_begin(self)
    }
    #[inline]
    fn resolution_ns(&self) -> u64 {
        self.segment.get().resolution_ns()
    }
}

/// How well the clock is kept on UTC: what [`Clock::update`] and
/// [`Clock::health`] return.
///
/// Measured against `CLOCK_REALTIME`, the host's clock; whether that is on
/// UTC is the start-up check's ([`Report::warnings`]) and housekeeping's
/// ([`Report::recheck_findings`], off the loop).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Health {
    /// Whether the clock is in UTC (else nothing is kept, and the rest is
    /// 0).
    pub utc: bool,
    /// Whether this call took a sample (an [`update`](Clock::update) at
    /// most once a second). Log [`warning`](Self::warning) only where this
    /// is `true`, or a gap or a stale clock reports on every call.
    pub sampled: bool,
    /// Since the last accepted update (calibration counts as one), ns.
    pub since_update_ns: u64,
    /// `CLOCK_REALTIME` less the clock after that update, ns (positive:
    /// the clock is behind).
    pub gap_ns: i64,
    /// How far the clock may be from `CLOCK_REALTIME` now, ns: the gap,
    /// the sample's error, and since the update the slew in progress and
    /// 25 ppm of hardware drift. The clock's side only: a wall-clock step
    /// since the last update shows at the next one, as the gap.
    pub bound_ns: u64,
    /// How long closing the gap takes at 0.5 ms a second, ns.
    pub catch_up_ns: u64,
    /// Samples rejected in a row (the host interrupted them).
    pub rejected_in_a_row: u64,
    /// The gap over which [`warning`](Self::warning) warns.
    gap_warn_ns: u64,
}

impl Health {
    /// What is wrong, if anything: no accepted update for over 5 s
    /// ([`Warning::Stale`]), else a gap over the configured threshold
    /// ([`Warning::Gap`]). Allocates nothing; [`Warning::line`] formats it
    /// for a log, off the hot path.
    #[must_use]
    pub const fn warning(&self) -> Option<Warning> {
        if !self.utc {
            None
        } else if self.since_update_ns > STALE_NS {
            Some(Warning::Stale {
                since_ns: self.since_update_ns,
                bound_ns: self.bound_ns,
                rejected_in_a_row: self.rejected_in_a_row,
            })
        } else if self.gap_ns.unsigned_abs() > self.gap_warn_ns {
            Some(Warning::Gap {
                gap_ns: self.gap_ns,
                catch_up_ns: self.catch_up_ns,
            })
        } else {
            None
        }
    }
}

// A read touches the segment's five words at the front clock's start; the
// shared clock has its line.
const _: () = assert!(size_of::<Segment>() == 40);
const _: () = assert!(core::mem::offset_of!(Clock, segment) == 0);
const _: () = assert!(size_of::<Clock>() == 48);
const _: () = assert!(align_of::<Aligned>() == 128);

// The owner is `Send + Sync` and never `Clone`; a follower is `Send` and
// not `Sync`: an impl of either makes the call below ambiguous.
const _: fn() = || {
    trait AmbiguousIfClone<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    struct IsClone;
    impl<T: Clone> AmbiguousIfClone<IsClone> for T {}
    trait AmbiguousIfSync<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfSync<()> for T {}
    struct IsSync;
    impl<T: ?Sized + Sync> AmbiguousIfSync<IsSync> for T {}
    const fn send_sync<T: Send + Sync>() {}
    const fn send<T: Send>() {}
    <Clock as AmbiguousIfClone<_>>::check();
    <Follower as AmbiguousIfSync<_>>::check();
    send_sync::<Clock>();
    send::<Follower>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{CounterClock, MonotonicClock};
    use crate::detect::{Arch, HostFacts, OrderedRead, Os};
    use crate::scale::Scale;

    type Res = Result<(), Box<dyn std::error::Error>>;

    /// 2026-10-16, ns since the epoch.
    const R0: i64 = 1_792_108_800_000_000_000;
    /// One second, ns (and ticks of the 1 GHz counter).
    const S: u64 = 1_000_000_000;

    /// A scripted host: a 1 GHz counter at `tick`, a wall clock of
    /// `R0 + tick + wall_off`, `CLOCK_REALTIME - CLOCK_MONOTONIC` moving
    /// with the steps; each bracket is `read_ticks` wide; brackets counted.
    struct Host {
        tick: u64,
        wall_off: i64,
        read_ticks: u64,
        brackets: u32,
    }

    impl Host {
        const fn at(tick: u64) -> Self {
            Self {
                tick,
                wall_off: 0,
                read_ticks: 20,
                brackets: 0,
            }
        }
    }

    impl SampleSource for Host {
        fn ticks_ordered(&mut self) -> u64 {
            self.tick += self.read_ticks / 2;
            self.tick
        }
        fn realtime_ns(&mut self) -> i64 {
            self.brackets += 1;
            R0 + self.tick.cast_signed() + self.wall_off
        }
        fn monotonic_ns(&mut self) -> Option<i64> {
            Some(self.tick.cast_signed())
        }
    }

    fn report(domain: Domain) -> Report {
        Report::new(
            HostFacts::unknown(Arch::Aarch64, Os::Linux),
            &Config {
                domain,
                ..Config::default()
            },
        )
    }

    /// A UTC owner of a 1 GHz counter reading `R0` at tick 0, calibrated
    /// at tick `at`.
    fn owner_at(at: u64) -> Result<Clock, Error> {
        let scale = Scale::from_hz(1_000_000_000, 0, R0.cast_unsigned())
            .ok_or(Error::InvalidParams("rate"))?;
        let segment = Segment::Counter(CounterClock::new(
            scale,
            Domain::Realtime,
            OrderedRead::None,
        ));
        let mut clock = Clock::keep(segment, report(Domain::Realtime))?;
        if let Some(utc) = clock.keeping.utc.as_mut() {
            utc.last_try = at;
            utc.last_good = at;
        }
        Ok(clock)
    }

    /// Real work at most once a second: no sample before a second has
    /// passed since the last, one after.
    #[test]
    fn update_samples_at_most_once_a_second() -> Res {
        let mut clock = owner_at(0)?;
        let mut host = Host::at(0);
        for (t, sampled) in [
            (S / 2, false),
            (S - 1, false),
            (S, true),
            (S + S / 2, false),
            (2 * S - 1, false),
            (2 * S, true),
        ] {
            host.tick = t;
            host.brackets = 0;
            let health = clock.update_at(t, &mut host);
            assert_eq!(host.brackets > 0, sampled, "at {t}");
            assert_eq!(health.sampled, sampled, "sampled at {t}");
        }
        assert!(!clock.health().sampled, "health never samples");
        Ok(())
    }

    /// A gap warns only over the threshold, as the start-up checks cut:
    /// a gap at the threshold is not a warning, one over it is (either
    /// sign). The live gap is compared the same way.
    #[test]
    fn a_gap_warns_only_over_the_threshold() -> Res {
        let at = |gap_ns: i64, gap_warn_ns: u64| Health {
            utc: true,
            sampled: true,
            since_update_ns: 0,
            gap_ns,
            bound_ns: 0,
            catch_up_ns: 0,
            rejected_in_a_row: 0,
            gap_warn_ns,
        };
        assert_eq!(at(0, 0).warning(), None);
        assert_eq!(at(0, 100).warning(), None);
        assert_eq!(at(100, 100).warning(), None, "at the threshold");
        assert_eq!(at(-100, 100).warning(), None, "at the threshold, behind");
        assert!(matches!(at(101, 100).warning(), Some(Warning::Gap { .. })));
        assert!(matches!(at(-101, 100).warning(), Some(Warning::Gap { .. })));

        let scale = Scale::from_hz(1_000_000_000, 0, R0.cast_unsigned())
            .ok_or(Error::InvalidParams("rate"))?;
        let segment = Segment::Counter(CounterClock::new(
            scale,
            Domain::Realtime,
            OrderedRead::None,
        ));
        let mut clock = Clock::keep(
            segment,
            Report::new(
                HostFacts::unknown(Arch::Aarch64, Os::Linux),
                &Config {
                    domain: Domain::Realtime,
                    gap_warn_ns: 1_000,
                    ..Config::default()
                },
            ),
        )?;
        if let Some(utc) = clock.keeping.utc.as_mut() {
            (utc.last_try, utc.last_good) = (0, 0);
        }
        let mut host = Host::at(S);
        let health = clock.update_at(S, &mut host);
        assert_eq!(health.gap_warn_ns, 1_000);
        assert!(health.gap_ns.abs_diff(0) < 100, "{health:?}");
        assert_eq!(health.warning(), None, "{health:?}");
        host.tick = 2 * S;
        host.wall_off = 50_000;
        let health = clock.update_at(2 * S, &mut host);
        assert!(matches!(health.warning(), Some(Warning::Gap { .. })));
        Ok(())
    }

    /// The wall clock 120 ms ahead after start: the update reports the gap
    /// and its catch-up time at the cap, and the warning's line says so.
    #[test]
    fn a_gap_after_start_is_reported_with_its_catch_up() -> Res {
        let mut clock = owner_at(0)?;
        let mut host = Host::at(S);
        host.wall_off = 120_000_000;
        let health = clock.update_at(S, &mut host);
        assert!(health.utc);
        assert!(health.gap_ns.abs_diff(120_000_000) < 100, "{health:?}");
        assert_eq!(
            health.catch_up_ns,
            warn::catch_up_ns(health.gap_ns.unsigned_abs())
        );
        assert!(
            health.catch_up_ns.abs_diff(240 * S) < 1_000_000,
            "{health:?}"
        );
        let warning = health.warning().ok_or("no warning")?;
        assert!(matches!(warning, Warning::Gap { .. }), "{warning:?}");
        assert_eq!(
            warning.line(),
            "ergon-clock warning: the clock is 120.00 ms behind CLOCK_REALTIME; it closes the \
             gap at 0.5 ms a second (about 4.0 min); stamps are off until then"
        );
        Ok(())
    }

    /// No accepted update for over 5 s is stale, with the bound: the gap,
    /// the sample's error, and the time since times the slew in progress
    /// and 25 ppm. Rejected samples are named instead of `Clock::update`.
    #[test]
    fn no_update_for_five_seconds_is_stale() -> Res {
        let mut clock = owner_at(0)?;
        let mut host = Host::at(S);
        host.wall_off = 3_000;
        let first = clock.update_at(S, &mut host);
        assert_eq!(first.warning(), None, "{first:?}");
        let utc = clock.keeping.utc.as_ref().ok_or("utc")?;
        let (gap, error, slew) = (utc.gap_ns, utc.error_ns, utc.slew_ppt);
        assert!(
            gap.abs_diff(3_000) < 100 && error > 0 && slew > 0,
            "{gap} {error} {slew}"
        );
        let at = 7 * S;
        let health = clock.health_at(at, false);
        assert_eq!(health.since_update_ns, 6 * S);
        let drift = u128::from(6 * S) * (u128::from(slew) + 25_000_000) / 1_000_000_000_000;
        assert_eq!(
            u128::from(health.bound_ns),
            u128::from(gap.unsigned_abs()) + u128::from(error) + drift
        );
        let warning = health.warning().ok_or("no warning")?;
        assert!(warning.line().contains("call Clock::update"), "{warning}");
        // the owner calls update, but every sample is rejected: wide brackets
        let mut clock = owner_at(0)?;
        let mut host = Host::at(S);
        clock.update_at(S, &mut host);
        host.read_ticks = 20_000;
        for t in [5 * S, 6 * S, 7 * S + 1] {
            host.tick = t;
            let h = clock.update_at(t, &mut host);
            assert!(h.rejected_in_a_row > 0, "{h:?}");
        }
        let health = clock.health_at(7 * S + 1, false);
        assert_eq!(health.rejected_in_a_row, 3);
        let line = health.warning().ok_or("no warning")?.line();
        assert!(line.contains("3 samples in a row rejected"), "{line}");
        assert!(!line.contains("Clock::update"), "{line}");
        Ok(())
    }

    /// Followers made before an update adopt it, and so does a clone.
    #[test]
    fn followers_adopt_each_update() -> Res {
        let mut clock = owner_at(0)?;
        let follower = clock.follower();
        let clone = follower.clone();
        assert_eq!(follower.followed(), clock.segment);
        let mut host = Host::at(S);
        host.wall_off = 50_000;
        clock.update_at(S, &mut host);
        assert_ne!(clock.segment.params(), owner_at(0)?.segment.params());
        assert_eq!(follower.followed(), clock.segment);
        assert_eq!(clone.followed(), clock.segment);
        let late = clock.follower();
        assert_eq!(late.followed(), clock.segment);
        Ok(())
    }

    /// The monotonic domain keeps nothing: no sample, a health of zeros,
    /// never a warning.
    #[test]
    fn the_monotonic_domain_has_nothing_to_keep() -> Res {
        let scale = Scale::from_hz(1_000_000_000, 0, 0).ok_or("rate")?;
        let segment = Segment::Counter(CounterClock::new(
            scale,
            Domain::Monotonic,
            OrderedRead::None,
        ));
        let mut clock = Clock::keep(segment, report(Domain::Monotonic))?;
        let mut host = Host::at(100 * S);
        let health = clock.update_at(100 * S, &mut host);
        assert_eq!(host.brackets, 0);
        assert!(!health.utc);
        assert_eq!(
            (health.since_update_ns, health.gap_ns, health.bound_ns),
            (0, 0, 0)
        );
        assert_eq!(health.warning(), None);
        Ok(())
    }

    /// The UTC fallback (ticks are monotonic ns) through a 1 s backward
    /// step of the wall clock: reads never go back, and it slews at the
    /// cap; with nothing between it and the wall clock, its factor is one.
    #[test]
    fn the_utc_fallback_never_goes_back() -> Res {
        let segment = Segment::Monotonic(MonotonicClock::unretuned(
            R0.cast_unsigned(),
            Domain::Realtime,
            1,
        ));
        let mut clock = Clock::keep(segment, report(Domain::Realtime))?;
        let mut host = Host::at(S);
        if let Some(utc) = clock.keeping.utc.as_mut() {
            (utc.last_try, utc.last_good) = (0, 0);
        }
        clock.update_at(S, &mut host);
        // Linux's fallback keeps the add path (the dead band); macOS's
        // CLOCK_UPTIME_RAW is disciplined in rate too
        assert_eq!(
            clock.segment.params().mult == 1 << 32,
            cfg!(any(target_os = "linux", target_os = "android")),
            "{:?}",
            clock.segment.params()
        );
        host.wall_off = -1_000_000_000;
        let mut last = 0;
        for k in 2..60 {
            let t = k * S;
            // reads through the segment in force, from its anchor
            let base = clock.segment.params().base_ticks;
            for step in 0..100 {
                let read = clock.segment.to_ns(base + step * (S / 100));
                assert!(read >= last, "back by {} at {t}", last - read);
                last = read;
            }
            host.tick = t;
            clock.update_at(t, &mut host);
            let read = clock.segment.to_ns(host.tick);
            assert!(
                read >= last,
                "back by {} after the update at {t}",
                last - read
            );
            last = read;
            let mult = clock.segment.params().mult;
            assert_eq!(mult, (1 << 32) - (1_u64 << 32) / 2_000, "at the cap at {t}");
        }
        Ok(())
    }

    /// The child for `update_never_prints`: an update that finds a gap.
    #[test]
    fn child_updates_with_a_gap() -> Res {
        let mut clock = owner_at(0)?;
        let mut host = Host::at(S);
        host.wall_off = 50_000_000;
        assert!(clock.update_at(S, &mut host).warning().is_some());
        assert!(clock.health_at(S, false).warning().is_some());
        Ok(())
    }

    /// An update that finds a gap prints nothing: it is the caller's to
    /// log, off the hot path.
    #[test]
    fn update_never_prints() -> Res {
        let out = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "owner::tests::child_updates_with_a_gap",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_remove("ERGON_HOST_CHECKS")
            .output()?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success() && stdout.contains("1 passed"),
            "{stdout}{stderr}"
        );
        assert!(!stderr.contains("ergon-clock"), "{stderr}");
        assert!(!stdout.contains("ergon-clock warning"), "{stdout}");
        Ok(())
    }

    /// The front clock's read touches its segment at offset 0; the
    /// follower is a `ClockFollower`'s size; the shared clock has a line of
    /// its own.
    #[test]
    fn the_front_clock_reads_its_segment_at_offset_zero() {
        assert_eq!(size_of::<Segment>(), 40);
        assert_eq!(size_of::<CounterClock>(), 40);
        assert_eq!(size_of::<MonotonicClock>(), 32);
        assert_eq!(core::mem::offset_of!(Clock, segment), 0);
        assert!(size_of::<Follower>() <= 96, "{}", size_of::<Follower>());
        assert_eq!(align_of::<Aligned>(), 128);
    }

    /// xorshift64*: deterministic, no dependency.
    const fn xorshift(x: &mut u64) -> u64 {
        *x ^= *x >> 12;
        *x ^= *x << 25;
        *x ^= *x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A follower converts as the segment it holds, bit for bit: the
    /// counter's conversion, and the fallback's (an add with a factor of
    /// one, else the multiply), over the whole range of ticks.
    #[test]
    fn a_follower_converts_as_its_segment_bit_for_bit() -> Res {
        let shared: &'static SharedClock = Box::leak(Box::new(SharedClock::new()));
        let mut x = 0x9e37_79b9_7f4a_7c15_u64;
        for i in 0..100_000 {
            let mult = match i % 4 {
                0 | 1 => 1 << 32,
                2 => (1 << 32) - (1 << 21) + xorshift(&mut x) % (1 << 22),
                _ => (1 << 26) + xorshift(&mut x) % ((1 << 40) - (1 << 26)),
            };
            let (base_ticks, base_ns) = (xorshift(&mut x), xorshift(&mut x));
            let scale = Scale::from_mult(mult, base_ticks, base_ns).ok_or("mult")?;
            let counter = Segment::Counter(CounterClock::new(
                scale,
                Domain::Realtime,
                OrderedRead::IsbCntvct,
            ));
            let fallback = Segment::Monotonic(MonotonicClock::unretuned(77, Domain::Realtime, 1))
                .retuned(base_ticks, mult, 0)
                .ok_or("retune")?;
            let ticks = match i % 3 {
                0 => base_ticks.wrapping_add(xorshift(&mut x) >> 20),
                1 => xorshift(&mut x),
                _ => base_ticks.wrapping_sub(xorshift(&mut x) % 1_000),
            };
            let f = Follower::new(shared, 0, counter);
            assert_eq!(f.ns_at(ticks), counter.to_ns(ticks), "{ticks} at {scale:?}");
            assert_eq!(f.counter.get(), Some(OrderedRead::IsbCntvct));
            let f = Follower::new(shared, 0, fallback);
            assert_eq!(
                f.fallback_ns(ticks),
                fallback.to_ns(ticks),
                "{ticks}: {fallback:?}"
            );
            assert_eq!(f.counter.get(), None);
        }
        Ok(())
    }
}
