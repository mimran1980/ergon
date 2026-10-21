//! Disciplining a clock to the wall clock: one owner re-anchors it once a
//! second so its rate and phase follow `CLOCK_REALTIME`, within a rate cap,
//! never going back.
//!
//! A calibrated counter clock runs at `CLOCK_MONOTONIC_RAW`'s rate from
//! `CLOCK_REALTIME` at calibration, so it parts from the wall clock by the
//! oscillator's error and NTP's corrections: milliseconds an hour. A
//! [`Discipline`] keeps it on the wall clock instead. Once a second its
//! owner takes a [`Sample`] (the wall clock bracketed between two ordered
//! reads of the counter, and on Linux `CLOCK_REALTIME - CLOCK_MONOTONIC`,
//! which moves only on wall-clock steps) and the discipline returns the
//! clock re-anchored ([`Clock::retuned`]): the same time at an ordered read
//! taken after the sample, then a new factor.
//!
//! The controller, all in integers, rounded to the nearest:
//!
//! - **Frequency**: the slope of the wall clock against the counter at its
//!   calibrated factor m₀ between consecutive accepted samples, less any
//!   wall-clock step seen in between: ρ̂, in parts per 10¹² (ppt). A slope
//!   over [`DisciplineConfig::sane_ppm`] is a step or a counter jump, never
//!   a rate, and is not used.
//! - **Phase**: the residual θ = wall − clock at the sample, removed over τ
//!   ([`DisciplineConfig::tau_ns`], 2 s: 500 ppt of rate per ns of error).
//! - **Rate**: r = ρ̂ + θ/τ, clamped to ±[`DisciplineConfig::cap_ppm`]
//!   (500 ppm, NTP's and Linux's `MAXFREQ`), and the factor
//!   m = m₀ + m₀·r, clamped to m₀ ± ⌊m₀·cap⌋ exactly: the interval between
//!   two reads of one segment is within cap·d + 1 ns of the counter's.
//! - **Steps**: forward only. At start ([`Phase::Start`], before anything
//!   has been stamped against timers) a clock behind the wall clock is
//!   stepped forward by the whole residual; after start never, unless
//!   [`DisciplineConfig::step_after_start_ns`] opts in. A clock ahead of
//!   the wall clock always slews back at up to the cap.
//! - **Rejects**: a sample whose error bound is over twice the floor (the
//!   least of the accepted samples among the last 64) plus 250 ns was taken
//!   across an interrupt; the clock is re-anchored at ρ̂ alone (the phase
//!   term dropped, so it is never applied twice) and the state is kept.
//!
//! Every re-anchor starts where the old segment reads, or later by a
//! forward step, and every factor is positive: reads on the owner's thread
//! never go back, whatever the wall clock does. Copies follow their owner
//! by adopting its parameters ([`Clock::adopt`], and the seqlock in
//! [`share`](crate::advanced::share) for other threads); none disciplines itself.
//!
//! No global state, no thread-local, no lazy initialisation, no allocation:
//! the [`Discipline`] is a plain value its owner keeps, and
//! [`Discipline::update_at`] is pure (no read of any clock), so the tests
//! drive it from a scripted wall clock.
//!
//! ```
//! use ergon_clock::advanced::discipline::{Discipline, DisciplineConfig, Phase};
//! use ergon_clock::advanced::share::{ClockPublisher, SharedClock};
//! use ergon_clock::advanced::{KernelSource, Sampler, calibrate, realtime_res_ns};
//! use ergon_clock::{Config, Domain};
//!
//! let mut clock = calibrate(&Config {
//!     domain: Domain::Realtime,
//!     ..Config::default()
//! })?
//! .clock;
//! let mut discipline = Discipline::new(&clock, DisciplineConfig::default())?;
//! let mut sampler = Sampler::for_clock(&clock, realtime_res_ns()?);
//! // other threads follow through this (a `static`, or one leaked per owner)
//! let shared = SharedClock::new();
//! let mut publisher = ClockPublisher::new(&shared).ok_or("claimed")?;
//! // before the clock's users start: the start update; then once a second
//! // from housekeeping, Phase::Running
//! for phase in [Phase::Start, Phase::Running] {
//!     let reader = clock;
//!     let update =
//!         discipline.sample_and_apply(&mut clock, &mut sampler, &mut KernelSource(&reader), phase);
//!     publisher.publish(&clock);
//!     assert!(update.error_ns > 0);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::clock::{Clock, Domain};
use crate::error::Error;
use crate::wall::{Sample, SampleSource, Sampler};

/// Parts per 10¹² in one: rates are kept in ppt.
const PPT: i128 = 1_000_000_000_000;
/// ppt in one ppm.
const PPT_PER_PPM: i64 = 1_000_000;
/// The accepted errors remembered for the reject floor.
const FLOOR_SAMPLES: usize = 64;
/// No slope over an interval shorter than this, ns: two brackets' noise
/// over less than 10 ms is a rate error of several ppm.
const MIN_SLOPE_NS: i128 = 10_000_000;
/// A residual over this, ns: [`flags::CONVERGING`].
const CONVERGING_NS: i64 = 10_000;
/// Where `CLOCK_REALTIME - CLOCK_MONOTONIC` is not available, an
/// innovation this large, ns, is taken for a wall-clock step.
const INFERRED_STEP_NS: i128 = 1_000_000;

/// The bits of [`Adjustment::flags`]: what the controller decided.
pub mod flags {
    /// The sample was rejected (its bracket too wide): held at ρ̂.
    pub const REJECTED: u32 = 1 << 0;
    /// The rate cap bound: the clock follows the wall clock at the cap.
    pub const SLEW_LIMITED: u32 = 1 << 2;
    /// The residual was over 10 µs.
    pub const CONVERGING: u32 = 1 << 3;
    /// The wall clock stepped since the last sample.
    pub const WALL_STEP: u32 = 1 << 4;
    /// The clock was stepped forward.
    pub const STEPPED: u32 = 1 << 5;
    /// The start update ([`Phase::Start`](super::Phase::Start)).
    pub const START: u32 = 1 << 6;
    /// The wall clock moved faster than the sanity limit against the counter.
    ///
    /// With no step seen ([`sane_ppm`](super::DisciplineConfig::sane_ppm)):
    /// a counter jump (live migration, a broken TSC) or a slew no daemon
    /// makes. The slope was not used; recheck the calibration.
    pub const FAST_SLEW_OR_JUMP: u32 = 1 << 9;
    /// A step probe disagreed with the innovation and was ignored.
    pub const D_GLITCH: u32 = 1 << 10;
}

/// How a clock is disciplined: [`Default`] is the specified controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisciplineConfig {
    /// The rate cap, ppm of the calibrated rate (50 to 500; 500): NTP's
    /// `MAXFREQ` and Linux's `adjtimex` limit, so the clock never slews
    /// harder than `CLOCK_MONOTONIC` itself can, and follows chrony's
    /// normal slews (−411 to +533 ppm measured) within a few ms. Lower it
    /// for a tighter interval bound (cap·d + 1 ns), at the price of longer
    /// episodes after a wall-clock step.
    pub cap_ppm: u32,
    /// The phase time constant, ns (2 s): the residual is removed at θ/τ.
    /// With samples a second apart the loop's pole is 1 − 1/τ: 0.5, no
    /// overshoot for gaps under τ, stable under 2τ. At least 1 s.
    pub tau_ns: u64,
    /// Brackets per sample at most, once running (8).
    pub tries: u32,
    /// Brackets for the start update (16): it may step.
    pub start_tries: u32,
    /// Step probes per sample at most (3; 0: none).
    pub d_tries: u32,
    /// The largest slope taken for a rate, ppm (1,000).
    pub sane_ppm: u32,
    /// Step forward after start when behind by at least this, ns (`None`:
    /// never, the default). A forward step fires every timer due within
    /// it at once; slewing does not. At least 1 ms.
    pub step_after_start_ns: Option<u64>,
    /// A dead band, ns (`None`: none, the default): while the residual is
    /// within it the applied rate is exactly 0, so the factor stays m₀.
    ///
    /// For the monotonic fallback on Linux, whose `CLOCK_MONOTONIC` runs at
    /// `CLOCK_REALTIME`'s rate (one timekeeper, slewed together): its
    /// factor stays one (2^32) and its read an add, not a multiply, while
    /// nothing but bracket noise separates it from the wall clock. Beyond
    /// the band (a wall-clock step) the controller runs as without one,
    /// and the band returns once the residual is within a quarter of it
    /// (hysteresis: noise near the edge never flips the factor). Rejected
    /// samples inside the band also keep the factor m₀.
    pub unit_rate_within_ns: Option<u64>,
}

impl DisciplineConfig {
    /// The least rate cap, ppm.
    pub const MIN_CAP_PPM: u32 = 50;
    /// The greatest rate cap, ppm.
    pub const MAX_CAP_PPM: u32 = 500;
    /// The least `step_after_start_ns`, ns.
    pub const MIN_STEP_AFTER_START_NS: u64 = 1_000_000;

    /// Whether every field is in its range.
    ///
    /// # Errors
    /// [`Error::InvalidParams`] naming the field out of range.
    pub const fn check(&self) -> Result<(), Error> {
        if self.cap_ppm < Self::MIN_CAP_PPM || self.cap_ppm > Self::MAX_CAP_PPM {
            return Err(Error::InvalidParams("cap_ppm: 50 to 500"));
        }
        if self.tau_ns < 1_000_000_000 || self.tau_ns > 1_000_000_000_000 {
            return Err(Error::InvalidParams("tau_ns: 1 s to 1,000 s"));
        }
        if self.tries == 0 || self.start_tries == 0 {
            return Err(Error::InvalidParams("tries: at least one bracket"));
        }
        if self.sane_ppm < self.cap_ppm || self.sane_ppm > 100_000 {
            return Err(Error::InvalidParams("sane_ppm: the cap to 100,000"));
        }
        if let Some(ns) = self.step_after_start_ns
            && ns < Self::MIN_STEP_AFTER_START_NS
        {
            return Err(Error::InvalidParams("step_after_start_ns: at least 1 ms"));
        }
        Ok(())
    }
}

impl Default for DisciplineConfig {
    fn default() -> Self {
        Self {
            cap_ppm: 500,
            tau_ns: 2_000_000_000,
            tries: 8,
            start_tries: 16,
            d_tries: 3,
            sane_ppm: 1_000,
            step_after_start_ns: None,
            unit_rate_within_ns: None,
        }
    }
}

/// Which update this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The first, before the clock's users start (no timers armed, nothing
    /// stamped that a step could overtake): a clock behind the wall clock
    /// is stepped forward.
    Start,
    /// Every later one: slew only, unless
    /// [`DisciplineConfig::step_after_start_ns`] opts in.
    Running,
}

/// What an update did to the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Re-anchored at a new rate.
    Slewed,
    /// Re-anchored `ns` later, then at a new rate.
    Stepped {
        /// The forward step, ns.
        ns: u64,
    },
    /// The sample was rejected: re-anchored at the frequency estimate alone.
    Held,
}

/// One update's result: the clock to install, and what was measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adjustment {
    /// The clock re-anchored at the update's ordered read: install it (and
    /// publish it to every copy) before the owner reads the clock again.
    pub clock: Clock,
    /// What was done.
    pub action: Action,
    /// θ: the wall clock less the clock at the sample, before the update,
    /// ns. The quality metric (positive: the clock was behind).
    pub residual_ns: i64,
    /// The wall clock less the clock after the update, ns: the residual
    /// less any step (for a held sample, the last accepted one's). Add it
    /// to a read for the kernel's wall-clock time.
    pub wall_offset_ns: i64,
    /// The sample's error bound, ns.
    pub error_ns: u64,
    /// The rate applied against the calibrated rate, ppt (within the cap).
    pub rate_ppt: i64,
    /// The wall clock's rate against the calibrated rate, ppt: ρ̂.
    pub freq_ppt: i64,
    /// The wall-clock step seen since the last sample, ns (0: none).
    pub wall_step_ns: i64,
    /// [`flags`] bits.
    pub flags: u32,
}

/// The discipline of one clock: its owner's state, a plain value.
///
/// Made from the calibration's clock ([`new`](Self::new)), it keeps the
/// calibrated factor m₀, the last accepted sample, the frequency estimate,
/// the last trusted step probe and the reject floor. Each update returns
/// the clock to install; the owner installs it and publishes it, and every
/// copy adopts the same parameters.
#[derive(Clone, Debug)]
pub struct Discipline {
    cfg: DisciplineConfig,
    /// The calibrated factor, ns per tick, 32.32: never changes but on a
    /// hand-over ([`rebase`](Self::rebase)).
    m0: u64,
    /// ⌊m₀·cap⌋: the factor stays within m₀ ± this.
    lim: u64,
    /// The last accepted sample: (ticks, wall ns). `None` after a hand-over
    /// to another tick domain, until the next sample.
    prev: Option<(u64, i64)>,
    /// ρ̂, ppt.
    rho_hat: i64,
    /// The last trusted step probe: (D, its width).
    d_prev: Option<(i64, u64)>,
    /// A step probe seen once and not confirmed: if the next agrees with
    /// it, the trusted one was the glitch.
    d_candidate: Option<(i64, u64)>,
    /// Each of the last 64 samples' error bound, ns (`u32::MAX`: rejected,
    /// or none yet).
    errors: [u32; FLOOR_SAMPLES],
    next: usize,
    /// The wall offset after the last accepted sample, ns.
    wall_offset: i64,
    /// Samples rejected since the last accepted one: the owner's health
    /// check reports it.
    consecutive_rejects: u64,
    /// Inside the dead band ([`DisciplineConfig::unit_rate_within_ns`]):
    /// the applied rate is 0.
    in_band: bool,
}

impl Discipline {
    /// The discipline of `clock`, the calibration's clock as made (its
    /// factor is taken for m₀; its anchor, the calibration's bracket of the
    /// wall clock, is the first sample).
    ///
    /// # Errors
    /// [`Error::InvalidParams`] for a configuration out of range
    /// ([`DisciplineConfig::check`]), or a clock in the monotonic domain:
    /// only a clock counting from the epoch can follow the wall clock.
    pub fn new(clock: &Clock, cfg: DisciplineConfig) -> Result<Self, Error> {
        cfg.check()?;
        if clock.domain() != Domain::Realtime {
            return Err(Error::InvalidParams(
                "domain: only a realtime-domain clock follows the wall clock",
            ));
        }
        let p = clock.params();
        Ok(Self {
            cfg,
            m0: p.mult,
            lim: lim_of(p.mult, cfg.cap_ppm),
            prev: Some((p.base_ticks, p.base_ns.cast_signed())),
            rho_hat: 0,
            d_prev: None,
            d_candidate: None,
            errors: [u32::MAX; FLOOR_SAMPLES],
            next: 0,
            wall_offset: 0,
            consecutive_rejects: 0,
            in_band: cfg.unit_rate_within_ns.is_some(),
        })
    }

    /// The calibrated factor m₀, ns per tick, 32.32: durations measured in
    /// raw ticks convert at it, immune to the discipline.
    #[must_use]
    pub const fn nominal_mult(&self) -> u64 {
        self.m0
    }

    /// Samples rejected since the last accepted one: the owner's health
    /// check reports it.
    #[must_use]
    pub const fn consecutive_rejects(&self) -> u64 {
        self.consecutive_rejects
    }

    /// The configuration this discipline was made with. An owner that hands
    /// a [`DisciplineConfig`] to [`Discipline::new`] and needs the same
    /// values later (a sampler's tries) reads them here rather than keeping
    /// a second copy.
    #[must_use]
    pub const fn config(&self) -> &DisciplineConfig {
        &self.cfg
    }

    /// The clock at the frequency estimate alone, re-anchored at `at_ticks`
    /// with no phase term: what an owner hands to code that will not follow
    /// (a benchmark's clock, another process, C code). It parts from the
    /// disciplined clock at the wall clock's rate change since, never at a
    /// slew's. The same clock an update that only holds would install.
    #[must_use]
    pub fn snapshot(&self, clock: &Clock, at_ticks: u64) -> Clock {
        clock
            .retuned(at_ticks, self.mult_at(self.held_rate()), 0)
            .unwrap_or(*clock)
    }

    /// One update from a sample taken by `sampler` from `src`, with the
    /// re-anchor at an ordered read of `src` taken after it, installed in
    /// `clock`: the whole of an owner's once-a-second step, against any
    /// source (the kernel's, or a scripted one).
    pub fn sample_and_apply(
        &mut self,
        clock: &mut Clock,
        sampler: &mut Sampler,
        src: &mut impl SampleSource,
        phase: Phase,
    ) -> Adjustment {
        let tries = match phase {
            Phase::Start => self.cfg.start_tries,
            Phase::Running => self.cfg.tries,
        };
        let sample = sampler.take(src, tries, self.cfg.d_tries);
        let at_ticks = src.ticks_ordered();
        let adjustment = self.update_at(clock, &sample, phase, at_ticks);
        *clock = adjustment.clock;
        adjustment
    }

    /// [`update_at`](Self::update_at) re-anchored at the clock's own
    /// ordered read, taken now (after the sample), and installed in
    /// `clock`.
    pub fn apply(&mut self, clock: &mut Clock, sample: &Sample, phase: Phase) -> Adjustment {
        let at_ticks = clock.ticks_ordered();
        let adjustment = self.update_at(clock, sample, phase, at_ticks);
        *clock = adjustment.clock;
        adjustment
    }

    /// One update: `clock` (the segment in force) re-anchored at
    /// `at_ticks`, from `sample`. Pure: no read of any clock.
    ///
    /// `at_ticks` must be an ordered read of the clock's ticks taken after
    /// the sample, and after every read of `clock` the owner made: the new
    /// segment starts where the old one reads there, so no read goes back.
    pub fn update_at(
        &mut self,
        clock: &Clock,
        sample: &Sample,
        phase: Phase,
        at_ticks: u64,
    ) -> Adjustment {
        let theta = i128::from(sample.wall_ns) - i128::from(clock.to_ns(sample.mid_ticks));
        if let Some(floor) = self.error_floor()
            && u128::from(sample.error_ns) > 2 * u128::from(floor) + 250
        {
            return self.hold(clock, sample, at_ticks, theta);
        }
        self.remember_error(clamp_u32(sample.error_ns));
        self.consecutive_rejects = 0;
        let mut flags = if phase == Phase::Start {
            flags::START
        } else {
            0
        };

        // the innovation over the interval, and any wall-clock step in it
        let interval = self
            .prev
            .map(|(t, r)| (self.elapsed(t, sample.mid_ticks), r));
        let innovation = interval.and_then(|(n, r_prev)| {
            (n > 0).then(|| {
                let predicted =
                    i128::from(r_prev) + n + div_round(n * i128::from(self.rho_hat), PPT);
                i128::from(sample.wall_ns) - predicted
            })
        });
        let step_seen = self.wall_step(sample, innovation, interval.map_or(0, |(n, _)| n));
        let wall_step = match step_seen {
            WallStep::None => 0,
            WallStep::Glitch => {
                flags |= flags::D_GLITCH;
                0
            }
            WallStep::Step(delta) => {
                flags |= flags::WALL_STEP;
                delta
            }
        };

        // the frequency: the slope since the last accepted sample, less
        // the step
        if let Some((n, r_prev)) = interval
            && n >= MIN_SLOPE_NS
        {
            let rise = i128::from(sample.wall_ns) - i128::from(r_prev) - wall_step - n;
            let rho = div_round(rise * PPT, n);
            if rho.abs() <= i128::from(self.cfg.sane_ppm) * i128::from(PPT_PER_PPM) {
                self.rho_hat = saturate(rho);
            } else {
                flags |= flags::FAST_SLEW_OR_JUMP;
            }
        }
        self.prev = Some((sample.mid_ticks, sample.wall_ns));

        // the phase: step forward where allowed, slew the rest
        let step = match (phase, self.cfg.step_after_start_ns) {
            (Phase::Start, _) if theta > 0 => theta,
            (Phase::Running, Some(x)) if theta >= i128::from(x) => theta,
            _ => 0,
        };
        let cap = i128::from(self.cfg.cap_ppm) * i128::from(PPT_PER_PPM);
        let wanted = if self.within_band(theta - step) {
            0
        } else {
            i128::from(self.rho_hat) + div_round((theta - step) * PPT, i128::from(self.cfg.tau_ns))
        };
        let rate = wanted.clamp(-cap, cap);
        if rate != wanted {
            flags |= flags::SLEW_LIMITED;
        }
        let mult = self.mult_at(rate);
        let step_ns = u64::try_from(step).unwrap_or(u64::MAX);
        let retuned = clock.retuned(at_ticks, mult, step_ns);
        let action = if step > 0 && retuned.is_some() {
            flags |= flags::STEPPED;
            Action::Stepped { ns: step_ns }
        } else {
            Action::Slewed
        };
        let residual = saturate(theta);
        let applied = if matches!(action, Action::Stepped { .. }) {
            step
        } else {
            0
        };
        self.wall_offset = saturate(theta - applied);
        if residual.abs() > CONVERGING_NS {
            flags |= flags::CONVERGING;
        }
        Adjustment {
            clock: retuned.unwrap_or(*clock),
            action,
            residual_ns: residual,
            wall_offset_ns: self.wall_offset,
            error_ns: sample.error_ns,
            rate_ppt: saturate(rate),
            freq_ppt: self.rho_hat,
            wall_step_ns: saturate(wall_step),
            flags,
        }
    }

    /// A rejected sample: re-anchored at ρ̂ alone (the phase term dropped:
    /// it is never applied twice), the state kept.
    fn hold(&mut self, clock: &Clock, sample: &Sample, at_ticks: u64, theta: i128) -> Adjustment {
        self.consecutive_rejects += 1;
        self.remember_error(u32::MAX);
        let mut flags = flags::REJECTED;
        if self.wall_offset.abs() > CONVERGING_NS {
            flags |= flags::CONVERGING;
        }
        let rate = self.held_rate();
        Adjustment {
            clock: clock
                .retuned(at_ticks, self.mult_at(rate), 0)
                .unwrap_or(*clock),
            action: Action::Held,
            residual_ns: saturate(theta),
            wall_offset_ns: self.wall_offset,
            error_ns: sample.error_ns,
            rate_ppt: saturate(rate),
            freq_ppt: self.rho_hat,
            wall_step_ns: 0,
            flags,
        }
    }

    /// The step the wall clock made since the last sample, from the step
    /// probes where the source has them, else from the innovation.
    fn wall_step(&mut self, sample: &Sample, innovation: Option<i128>, n: i128) -> WallStep {
        let Some(probe) = sample.step_probe else {
            self.d_candidate = None;
            return match innovation {
                Some(nu) if nu.abs() >= INFERRED_STEP_NS => WallStep::Step(nu),
                _ => WallStep::None,
            };
        };
        let d = probe.real_minus_mono_ns;
        let w = probe.width_ns;
        let Some((d_prev, w_prev)) = self.d_prev else {
            self.d_prev = Some((d, w));
            return WallStep::None;
        };
        let moved = i128::from(d) - i128::from(d_prev);
        let threshold = (4 * (u128::from(w) + u128::from(w_prev))).max(1_000);
        if moved.unsigned_abs() <= threshold {
            self.d_prev = Some((d, w));
            self.d_candidate = None;
            return WallStep::None;
        }
        // D moved: a step, if the innovation moved with it
        let slack = 4 * i128::from(sample.error_ns) + 100 + n * 5 / 1_000_000;
        if let Some(nu) = innovation
            && (nu - moved).abs() <= slack
        {
            self.d_prev = Some((d, w));
            self.d_candidate = None;
            return WallStep::Step(moved);
        }
        // not confirmed: keep the trusted probe, unless this one agrees
        // with the last unconfirmed one (then the trusted one was the
        // glitch, or a step went unconfirmed: either way D is here now)
        if let Some((c, cw)) = self.d_candidate
            && (i128::from(d) - i128::from(c)).unsigned_abs()
                <= (4 * (u128::from(w) + u128::from(cw))).max(1_000)
        {
            self.d_prev = Some((d, w));
            self.d_candidate = None;
        } else {
            self.d_candidate = Some((d, w));
        }
        WallStep::Glitch
    }

    /// The rate with no phase term, ppt: 0 inside the dead band, else ρ̂
    /// within the cap.
    fn held_rate(&self) -> i128 {
        if self.in_band {
            return 0;
        }
        let cap = i128::from(self.cfg.cap_ppm) * i128::from(PPT_PER_PPM);
        i128::from(self.rho_hat).clamp(-cap, cap)
    }

    /// Whether the residual `residual` (ns, after any step) is inside the
    /// dead band, which is entered within a quarter of its width and left
    /// beyond it; never without a band.
    fn within_band(&mut self, residual: i128) -> bool {
        let Some(within) = self.cfg.unit_rate_within_ns else {
            return false;
        };
        let limit = if self.in_band { within } else { within / 4 };
        self.in_band = residual.unsigned_abs() <= u128::from(limit);
        self.in_band
    }

    /// The hand-over to a new calibration (the counter dropped for the
    /// fallback, say): `new` anchored where `old` reads after `new` was
    /// read, so never behind `old` (and ahead by at most the bracket), its
    /// factor the new m₀, the slope state reset to the new tick domain, the
    /// residual and the step-probe reference kept. Reads both clocks:
    /// `old`, `new`, `old`, all ordered.
    ///
    /// # Errors
    /// [`Error::InvalidParams`] where `new` counts from another domain than
    /// `old`, or cannot be anchored there; the caller keeps `old`.
    pub fn rebase(&mut self, old: &Clock, new: &Clock) -> Result<Clock, Error> {
        let _before = old.ticks_ordered();
        let new_ticks = new.ticks_ordered();
        let old_after = old.ticks_ordered();
        self.rebase_at(old, new, new_ticks, old_after)
    }

    /// [`rebase`](Self::rebase) with its reads given: `new_ticks` a read of
    /// `new`'s ticks, `old_after` an ordered read of `old`'s taken after it.
    ///
    /// # Errors
    /// As [`rebase`](Self::rebase).
    pub fn rebase_at(
        &mut self,
        old: &Clock,
        new: &Clock,
        new_ticks: u64,
        old_after: u64,
    ) -> Result<Clock, Error> {
        if new.domain() != old.domain() {
            return Err(Error::InvalidParams("domain: a hand-over keeps the domain"));
        }
        let mut p = new.params();
        p.base_ticks = new_ticks;
        p.base_ns = old.to_ns(old_after);
        let anchored = new.adopt(&p)?;
        self.m0 = p.mult;
        self.lim = lim_of(p.mult, self.cfg.cap_ppm);
        self.prev = None;
        self.rho_hat = 0;
        self.errors = [u32::MAX; FLOOR_SAMPLES];
        self.next = 0;
        Ok(anchored)
    }

    /// The factor for rate `rate` (ppt), within m₀ ± ⌊m₀·cap⌋.
    fn mult_at(&self, rate: i128) -> u64 {
        let m0 = i128::from(self.m0);
        let lim = i128::from(self.lim);
        let mult = (m0 + div_round(m0 * rate, PPT)).clamp(m0 - lim, m0 + lim);
        u64::try_from(mult).unwrap_or(self.m0)
    }

    /// ns from `from` to `to` at m₀ (signed).
    fn elapsed(&self, from: u64, to: u64) -> i128 {
        (i128::from(to.wrapping_sub(from).cast_signed()) * i128::from(self.m0)) >> 32
    }

    /// The least error bound of the accepted samples among the last 64.
    fn error_floor(&self) -> Option<u32> {
        self.errors.iter().copied().filter(|&e| e != u32::MAX).min()
    }

    const fn remember_error(&mut self, error: u32) {
        self.errors[self.next] = error;
        self.next = (self.next + 1) % FLOOR_SAMPLES;
    }
}

/// What the step probes (or the innovation) say about the interval.
enum WallStep {
    None,
    Glitch,
    Step(i128),
}

/// ⌊m₀·cap⌋, cap in ppm.
fn lim_of(m0: u64, cap_ppm: u32) -> u64 {
    let lim = u128::from(m0) * u128::from(cap_ppm) / 1_000_000;
    u64::try_from(lim).unwrap_or(u64::MAX)
}

/// `n / d` rounded to the nearest, halves away from zero; `d` positive.
const fn div_round(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

/// An i128 as an i64, saturating.
fn saturate(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

/// An error bound in 32 bits, saturating below the "none" marker.
fn clamp_u32(v: u64) -> u32 {
    u32::try_from(v).map_or(u32::MAX - 1, |v| v.min(u32::MAX - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{CounterClock, MonotonicClock};
    use crate::detect::OrderedRead;
    use crate::scale::Scale;
    use crate::wall::StepProbe;

    type Res = Result<(), Box<dyn std::error::Error>>;

    /// 2026-10-16, ns since the epoch.
    const R0: i64 = 1_792_108_800_000_000_000;
    /// m₀ of a 1 GHz counter: one ns a tick.
    const M0: u64 = 1 << 32;

    /// A 1 GHz counter calibrated to read `R0` at tick 0.
    fn clock() -> Result<Clock, Error> {
        let scale = Scale::from_hz(1_000_000_000, 0, R0.cast_unsigned())
            .ok_or(Error::InvalidParams("rate"))?;
        Ok(Clock::Counter(CounterClock::new(
            scale,
            Domain::Realtime,
            OrderedRead::Cntvctss,
        )))
    }

    /// A sample at tick `t` of a wall clock reading `R0 + t + off`, with an
    /// error bound `e` and, where given, a step probe of width 10 ns.
    const fn sample(t: u64, off: i64, e: u64, d: Option<i64>) -> Sample {
        Sample {
            mid_ticks: t,
            wall_ns: R0 + t.cast_signed() + off,
            error_ns: e,
            width_ticks: 2 * e,
            step_probe: match d {
                Some(d) => Some(StepProbe {
                    real_minus_mono_ns: d,
                    width_ns: 10,
                }),
                None => None,
            },
        }
    }

    fn discipline(cfg: DisciplineConfig) -> Result<(Clock, Discipline), Error> {
        let c = clock()?;
        Ok((c, Discipline::new(&c, cfg)?))
    }

    /// The frequency, the phase term and the factor each round to the
    /// nearest (halves away from zero), never down and never toward zero:
    /// a floor biases the residual (by about 70 ns for a floor in ppb).
    #[test]
    fn every_quotient_rounds_to_the_nearest() -> Res {
        // the slope: +1 ns over 1.5 s is 666.67 ppt; -1 ns over 3 s is -333.33
        for (t, off, rho) in [(1_500_000_000, 1, 667), (3_000_000_000, -1, -333)] {
            let (c, mut d) = discipline(DisciplineConfig::default())?;
            let a = d.update_at(&c, &sample(t, off, 30, None), Phase::Running, t + 10);
            assert_eq!(a.freq_ppt, rho, "slope of {off} ns over {t} ns");
        }
        // the factor: θ/τ of 2,000 ns is 1 ppm, m₀·1 ppm = 4,294.97 units;
        // of 1,000 ns, 2,147.48; and negated
        for (theta, units) in [
            (2_000, 4_295),
            (1_000, 2_147),
            (-1_000, -2_147),
            (-2_000, -4_295),
        ] {
            let (c, mut d) = discipline(DisciplineConfig::default())?;
            // 5 ms after calibration: under 10 ms, so no slope
            let a = d.update_at(
                &c,
                &sample(5_000_000, theta, 30, None),
                Phase::Running,
                5_000_100,
            );
            assert_eq!(a.freq_ppt, 0);
            assert_eq!(a.rate_ppt, 500 * theta);
            assert_eq!(
                i128::from(a.clock.params().mult) - i128::from(M0),
                units,
                "theta {theta}"
            );
        }
        // the phase term at τ = 3 s: 2 ns is 666.67 ppt
        let cfg = DisciplineConfig {
            tau_ns: 3_000_000_000,
            ..DisciplineConfig::default()
        };
        let (c, mut d) = discipline(cfg)?;
        let a = d.update_at(
            &c,
            &sample(5_000_000, 2, 30, None),
            Phase::Running,
            5_000_100,
        );
        assert_eq!(a.rate_ppt, 667);
        Ok(())
    }

    /// No slope over less than 10 ms: two brackets' noise over a short
    /// interval (the start soon after calibration) is no rate.
    #[test]
    fn no_slope_over_less_than_ten_milliseconds() -> Res {
        let (c, mut d) = discipline(DisciplineConfig::default())?;
        let a = d.update_at(
            &c,
            &sample(9_999_999, 500, 30, None),
            Phase::Start,
            10_000_100,
        );
        assert_eq!(a.freq_ppt, 0, "under 10 ms");
        let (c, mut d) = discipline(DisciplineConfig::default())?;
        let a = d.update_at(
            &c,
            &sample(10_000_000, 500, 30, None),
            Phase::Start,
            10_000_100,
        );
        assert_eq!(a.freq_ppt, 50_000_000, "500 ns over 10 ms is 50 ppm");
        Ok(())
    }

    /// The start steps a clock behind by exactly its residual; a running
    /// update steps only where opted in, behind by at least the threshold,
    /// and then by exactly the residual; a clock ahead is never stepped.
    /// The wall offset is the residual less the step.
    #[test]
    fn steps_are_forward_and_only_where_allowed() -> Res {
        let opt_in = DisciplineConfig {
            step_after_start_ns: Some(1_000_000),
            ..DisciplineConfig::default()
        };
        for (cfg, phase, theta, step) in [
            (
                DisciplineConfig::default(),
                Phase::Start,
                5_000,
                Some(5_000),
            ),
            (DisciplineConfig::default(), Phase::Start, 1, Some(1)),
            (DisciplineConfig::default(), Phase::Start, 0, None),
            (DisciplineConfig::default(), Phase::Start, -5_000, None),
            (DisciplineConfig::default(), Phase::Running, 5_000_000, None),
            (opt_in, Phase::Running, 999_999, None),
            (opt_in, Phase::Running, 1_000_000, Some(1_000_000)),
            (opt_in, Phase::Running, 7_654_321, Some(7_654_321)),
            (opt_in, Phase::Running, -7_654_321, None),
        ] {
            let (c, mut d) = discipline(cfg)?;
            let at = 5_000_100;
            let a = d.update_at(&c, &sample(5_000_000, theta, 30, None), phase, at);
            assert_eq!(a.residual_ns, theta);
            if let Some(ns) = step {
                assert_eq!(a.action, Action::Stepped { ns }, "{phase:?} {theta}");
                assert_eq!(a.clock.to_ns(at), c.to_ns(at) + ns);
                assert_eq!(a.wall_offset_ns, theta - ns.cast_signed());
                assert_ne!(a.flags & flags::STEPPED, 0);
            } else {
                assert_eq!(a.action, Action::Slewed, "{phase:?} {theta}");
                assert_eq!(a.clock.to_ns(at), c.to_ns(at));
                assert_eq!(a.wall_offset_ns, theta);
            }
            assert_eq!(
                a.rate_ppt,
                (500 * a.wall_offset_ns).clamp(-500_000_000, 500_000_000),
                "the phase term is the offset left"
            );
            assert_eq!(a.flags & flags::START != 0, phase == Phase::Start);
        }
        Ok(())
    }

    /// The rate is clamped to the cap and the factor to m₀ ± ⌊m₀·cap⌋,
    /// exactly: at the cap the factor is at the limit, and flagged.
    #[test]
    fn the_cap_is_exact_in_integers() -> Res {
        for cap in [50, 100, 500] {
            let cfg = DisciplineConfig {
                cap_ppm: cap,
                ..DisciplineConfig::default()
            };
            let lim = u64::try_from(u128::from(M0) * u128::from(cap) / 1_000_000)?;
            for theta in [10_000_000, -10_000_000] {
                let (c, mut d) = discipline(cfg)?;
                let a = d.update_at(
                    &c,
                    &sample(5_000_000, theta, 30, None),
                    Phase::Running,
                    5_000_100,
                );
                let cap_ppt = i64::from(cap) * 1_000_000;
                assert_eq!(a.rate_ppt, cap_ppt * theta.signum());
                let want = if theta > 0 { M0 + lim } else { M0 - lim };
                assert_eq!(a.clock.params().mult, want, "cap {cap}");
                assert_ne!(a.flags & flags::SLEW_LIMITED, 0);
            }
            // just inside the cap: not limited
            let (c, mut d) = discipline(cfg)?;
            let theta = i64::from(cap) * 1_000_000 / 500;
            let a = d.update_at(
                &c,
                &sample(5_000_000, theta, 30, None),
                Phase::Running,
                5_000_100,
            );
            assert_eq!(a.flags & flags::SLEW_LIMITED, 0, "{theta} ns at cap {cap}");
        }
        Ok(())
    }

    /// A sample whose error bound is over twice the floor plus 250 ns is
    /// rejected; at the limit it is accepted. A rejected sample is held at
    /// ρ̂ (no phase term) and changes nothing but the reject count.
    #[test]
    fn the_reject_limit_is_twice_the_floor_and_250_ns() -> Res {
        let (mut c, mut d) = discipline(DisciplineConfig::default())?;
        let first = d.update_at(
            &c,
            &sample(1_000_000_000, 300, 100, None),
            Phase::Running,
            1_000_000_100,
        );
        assert_eq!(first.action, Action::Slewed);
        c = first.clock;
        assert_eq!(first.freq_ppt, 300_000, "300 ns over the second");
        let at_limit = d.update_at(
            &c,
            &sample(2_000_000_000, 600, 450, None),
            Phase::Running,
            2_000_000_100,
        );
        assert_eq!(at_limit.action, Action::Slewed, "2 x 100 + 250 is accepted");
        c = at_limit.clock;
        let rho = at_limit.freq_ppt;
        let before = d.clone();
        let over = d.update_at(
            &c,
            &sample(3_000_000_000, 5_000, 451, None),
            Phase::Running,
            3_000_000_100,
        );
        assert_eq!(over.action, Action::Held, "451 is rejected");
        assert_eq!(over.flags & flags::REJECTED, flags::REJECTED);
        assert_eq!(over.freq_ppt, rho, "the estimate is kept");
        assert_eq!(
            (d.prev, d.d_prev, d.wall_offset),
            (before.prev, before.d_prev, before.wall_offset)
        );
        let m0 = i128::from(M0);
        let want = m0 + (m0 * i128::from(rho) + PPT / 2) / PPT;
        assert_eq!(i128::from(over.clock.params().mult), want, "ρ̂ alone");
        assert_eq!(
            over.clock.to_ns(3_000_000_100),
            c.to_ns(3_000_000_100),
            "no step"
        );
        assert_eq!(
            over.wall_offset_ns, at_limit.wall_offset_ns,
            "the last accepted offset"
        );
        assert_eq!(d.consecutive_rejects(), 1);
        Ok(())
    }

    /// A step probe that moves by more than its threshold (1 µs, or four
    /// times the two probes' widths) with the innovation moving with it
    /// (within four error bounds, 100 ns and 5 ppm of the interval) is a
    /// wall step of its size and sign; at the threshold it is not; with the
    /// innovation not moving with it, it is a glitch and the trusted probe
    /// is kept.
    #[test]
    fn a_step_probe_is_a_step_only_beyond_its_threshold_and_confirmed() -> Res {
        let d0 = 7_000_000;
        // (probe moved, wall moved, flags expected, step seen)
        let slack = 4 * 30 + 100 + 5_000; // N = 1 s
        for (moved, wall, want, seen) in [
            (1_000, 1_000, 0, 0),
            (1_001, 1_001, flags::WALL_STEP, 1_001),
            (-1_001, -1_001, flags::WALL_STEP, -1_001),
            (50_000, 50_000 + slack, flags::WALL_STEP, 50_000),
            (-50_000, -50_000 - slack, flags::WALL_STEP, -50_000),
            (50_000, 50_000 + slack + 1, flags::D_GLITCH, 0),
            (50_000, 0, flags::D_GLITCH, 0),
        ] {
            let (c, mut d) = discipline(DisciplineConfig::default())?;
            let a = d.update_at(
                &c,
                &sample(1_000_000_000, 0, 30, Some(d0)),
                Phase::Running,
                1_000_000_000,
            );
            let b = d.update_at(
                &a.clock,
                &sample(2_000_000_000, wall, 30, Some(d0 + moved)),
                Phase::Running,
                2_000_000_000,
            );
            assert_eq!(
                b.flags & (flags::WALL_STEP | flags::D_GLITCH),
                want,
                "moved {moved}, wall {wall}"
            );
            assert_eq!(b.wall_step_ns, seen);
            let trusted = if want == flags::D_GLITCH {
                d0
            } else {
                d0 + moved
            };
            assert_eq!(d.d_prev.map(|p| p.0), Some(trusted), "moved {moved}");
            if want == flags::WALL_STEP {
                assert_eq!(
                    b.freq_ppt,
                    i64::from(i32::try_from(wall - moved)?) * 1_000,
                    "the step out of the slope"
                );
            }
        }
        // wider probes: the threshold is four times their widths
        let (clk, mut disc) = discipline(DisciplineConfig::default())?;
        let mut smp = sample(1_000_000_000, 0, 30, Some(d0));
        smp.step_probe = Some(StepProbe {
            real_minus_mono_ns: d0,
            width_ns: 200,
        });
        let a = disc.update_at(&clk, &smp, Phase::Running, 1_000_000_000);
        let mut smp = sample(2_000_000_000, 1_600, 30, None);
        smp.step_probe = Some(StepProbe {
            real_minus_mono_ns: d0 + 1_600,
            width_ns: 200,
        });
        let b = disc.update_at(&a.clock, &smp, Phase::Running, 2_000_000_000);
        assert_eq!(
            b.flags & flags::WALL_STEP,
            0,
            "1,600 is within 4 x (200 + 200)"
        );
        Ok(())
    }

    /// Where there is no step probe, an innovation of 1 ms or more is a
    /// wall step, and the slope over it is the estimate's.
    #[test]
    fn without_a_probe_a_millisecond_of_innovation_is_a_step() -> Res {
        for (off, step) in [
            (999_999, 0),
            (1_000_000, 1_000_000),
            (-1_000_000, -1_000_000),
        ] {
            let (c, mut d) = discipline(DisciplineConfig::default())?;
            let a = d.update_at(
                &c,
                &sample(1_000_000_000, 0, 30, None),
                Phase::Running,
                1_000_000_000,
            );
            let b = d.update_at(
                &a.clock,
                &sample(2_000_000_000, off, 30, None),
                Phase::Running,
                2_000_000_000,
            );
            assert_eq!(b.wall_step_ns, step, "{off}");
            assert_eq!(b.freq_ppt, if step == 0 { off * 1_000 } else { 0 });
        }
        Ok(())
    }

    /// A hand-over keeps the domain; one that would change it is refused
    /// and changes nothing.
    #[test]
    fn a_hand_over_keeps_the_domain() -> Res {
        let (c, mut d) = discipline(DisciplineConfig::default())?;
        let scale = Scale::from_hz(1_000_000_000, 0, 0).ok_or("rate")?;
        let other = Clock::Counter(CounterClock::new(
            scale,
            Domain::Monotonic,
            OrderedRead::Cntvctss,
        ));
        assert_eq!(
            d.rebase_at(&c, &other, 10, 20).err(),
            Some(Error::InvalidParams("domain: a hand-over keeps the domain"))
        );
        assert_eq!(d.prev, Some((0, R0)), "unchanged");
        let fresh = Clock::Counter(CounterClock::new(
            Scale::from_hz(999_999_000, 0, 0).ok_or("rate")?,
            Domain::Realtime,
            OrderedRead::Cntvctss,
        ));
        let new = d.rebase_at(&c, &fresh, 10, 20)?;
        assert_eq!(
            new.to_ns(10),
            c.to_ns(20),
            "anchored where the old clock reads after it"
        );
        assert_eq!(d.nominal_mult(), fresh.params().mult);
        assert_eq!(d.prev, None);
        Ok(())
    }

    /// The configuration's ranges, and only a realtime clock is disciplined.
    #[test]
    fn a_configuration_out_of_range_is_refused() -> Res {
        let ok = DisciplineConfig::default();
        assert_eq!(ok.check(), Ok(()));
        for bad in [
            DisciplineConfig { cap_ppm: 49, ..ok },
            DisciplineConfig { cap_ppm: 501, ..ok },
            DisciplineConfig {
                tau_ns: 999_999_999,
                ..ok
            },
            DisciplineConfig {
                tau_ns: 1_000_000_000_001,
                ..ok
            },
            DisciplineConfig { tries: 0, ..ok },
            DisciplineConfig {
                start_tries: 0,
                ..ok
            },
            DisciplineConfig {
                sane_ppm: 499,
                ..ok
            },
            DisciplineConfig {
                sane_ppm: 100_001,
                ..ok
            },
            DisciplineConfig {
                step_after_start_ns: Some(999_999),
                ..ok
            },
        ] {
            assert!(bad.check().is_err(), "{bad:?}");
            assert!(Discipline::new(&clock()?, bad).is_err());
        }
        for good in [
            DisciplineConfig { cap_ppm: 50, ..ok },
            DisciplineConfig { cap_ppm: 500, ..ok },
            DisciplineConfig {
                tau_ns: 1_000_000_000,
                ..ok
            },
            DisciplineConfig { d_tries: 0, ..ok },
            DisciplineConfig {
                step_after_start_ns: Some(1_000_000),
                ..ok
            },
        ] {
            assert_eq!(good.check(), Ok(()), "{good:?}");
        }
        let scale = Scale::from_hz(1_000_000_000, 0, 0).ok_or("rate")?;
        let monotonic = Clock::Counter(CounterClock::new(
            scale,
            Domain::Monotonic,
            OrderedRead::Cntvctss,
        ));
        assert!(Discipline::new(&monotonic, ok).is_err());
        Ok(())
    }

    /// A source whose every bracket is as wide as the last: no early exit
    /// until a floor is set, so the tries taken are counted.
    struct Even {
        tick: u64,
        brackets: u32,
    }

    impl SampleSource for Even {
        fn ticks_ordered(&mut self) -> u64 {
            self.tick += 20;
            self.tick
        }
        fn realtime_ns(&mut self) -> i64 {
            self.brackets += 1;
            R0 + self.tick.cast_signed()
        }
        fn monotonic_ns(&mut self) -> Option<i64> {
            None
        }
    }

    /// The start update takes up to 16 brackets, a running one up to 8,
    /// and the re-anchor is an ordered read taken after the sample.
    #[test]
    fn the_start_takes_more_brackets() -> Res {
        let (mut c, mut d) = discipline(DisciplineConfig::default())?;
        let mut src = Even {
            tick: 1_000,
            brackets: 0,
        };
        let mut sampler = Sampler::new(M0, 1, 1);
        let a = d.sample_and_apply(&mut c, &mut sampler, &mut src, Phase::Start);
        assert_eq!(src.brackets, 16);
        assert_eq!(
            a.clock.params().base_ticks,
            src.tick,
            "anchored at the last read"
        );
        let mut fresh = Sampler::new(M0, 1, 1);
        src.brackets = 0;
        d.sample_and_apply(&mut c, &mut fresh, &mut src, Phase::Running);
        assert_eq!(src.brackets, 8);
        Ok(())
    }

    /// Halves round away from zero for either sign, as documented: a tie
    /// never leans one way.
    #[test]
    fn halves_round_away_from_zero() {
        for (n, d, q) in [
            (3, 2, 2),
            (-3, 2, -2),
            (5, 10, 1),
            (-5, 10, -1),
            (4, 10, 0),
            (-4, 10, 0),
            (6, 10, 1),
            (-6, 10, -1),
            (0, 7, 0),
        ] {
            assert_eq!(div_round(n, d), q, "{n} / {d}");
        }
    }

    /// A counter clock on this CPU's counter, in the realtime domain,
    /// anchored now (its factor its rate where the CPU states one, else a
    /// nominal 1 GHz: only order and nearness are checked with it).
    fn live_counter() -> Result<Clock, Error> {
        #[cfg(target_arch = "aarch64")]
        let hz = crate::arch::counter_frequency();
        #[cfg(not(target_arch = "aarch64"))]
        let hz = 1_000_000_000;
        let scale = Scale::from_hz(
            hz,
            crate::arch::ticks(),
            crate::wall::realtime_ns().cast_unsigned(),
        )
        .ok_or(Error::InvalidParams("rate"))?;
        Ok(Clock::Counter(CounterClock::from_params(
            &Clock::Counter(CounterClock::new(
                scale,
                Domain::Realtime,
                OrderedRead::None,
            ))
            .params(),
        )?))
    }

    /// The reading hand-over, between the two tick domains either way (the
    /// counter's ticks and the monotonic clock's ns): the new clock reads no
    /// earlier than the old one read before it, and within a millisecond of
    /// what the old one reads after it (anchored where the old one read,
    /// not at a tick of the other domain).
    #[test]
    fn a_hand_over_between_tick_domains_never_reads_behind() -> Res {
        let (fallback, error) = Clock::fallback(Domain::Realtime);
        assert_eq!(error, None);
        let counter = live_counter()?;
        for (old, new) in [(counter, fallback), (fallback, counter)] {
            for _ in 0..1_000 {
                let mut d = Discipline::new(&old, DisciplineConfig::default())?;
                let before = old.now_ns_ordered();
                let handed = d.rebase(&old, &new)?;
                let first = handed.now_ns_ordered();
                let old_after = old.now_ns_ordered();
                assert!(first >= before, "back by {} ns", before - first);
                assert!(
                    first.abs_diff(old_after) < 1_000_000,
                    "{first} against the old clock's {old_after}"
                );
                assert_eq!(handed.is_counter(), new.is_counter());
                assert_eq!(d.nominal_mult(), new.params().mult);
            }
        }
        Ok(())
    }

    /// `apply` re-anchors at the clock's own ordered read, taken inside the
    /// call, and installs the result: the clock then reads no earlier than
    /// it read before.
    #[test]
    fn apply_anchors_at_its_own_ordered_read_and_installs() -> Res {
        for mut c in [Clock::fallback(Domain::Realtime).0, live_counter()?] {
            let mut d = Discipline::new(&c, DisciplineConfig::default())?;
            let mut sampler = Sampler::for_clock(&c, crate::wall::realtime_res_ns()?);
            for phase in [Phase::Start, Phase::Running, Phase::Running] {
                let reader = c;
                let s = sampler.take(&mut crate::wall::KernelSource(&reader), 8, 3);
                let ticks_before = c.ticks_ordered();
                let read_before = c.now_ns_ordered();
                let a = d.apply(&mut c, &s, phase);
                let ticks_after = c.ticks_ordered();
                assert_eq!(c, a.clock, "installed");
                let at = c.params().base_ticks;
                assert!(
                    ticks_before <= at && at <= ticks_after,
                    "anchored at {at}, outside {ticks_before}..{ticks_after}"
                );
                assert!(c.now_ns_ordered() >= read_before);
            }
        }
        Ok(())
    }

    /// xorshift64*: deterministic, no dependency.
    const fn xorshift(x: &mut u64) -> u64 {
        *x ^= *x >> 12;
        *x ^= *x << 25;
        *x ^= *x >> 27;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// `x`'s next value in `-half..=half`.
    fn noise(x: &mut u64, half: u64) -> i64 {
        i64::try_from(xorshift(x) % (2 * half + 1)).unwrap_or(0) - i64::try_from(half).unwrap_or(0)
    }

    /// The Linux fallback in UTC with a 1 µs dead band: its factor stays
    /// exactly one (its read an add) while the residual is bracket noise,
    /// rejected samples included; a 50 ms wall step takes it
    /// out (slewing at the cap) until the residual is within 250 ns, then
    /// it stays; noise around 900 ns inside the band never flips it.
    #[test]
    fn a_fallback_inside_the_band_keeps_factor_one() -> Res {
        let cfg = DisciplineConfig {
            unit_rate_within_ns: Some(1_000),
            ..DisciplineConfig::default()
        };
        let fallback = || {
            Clock::Monotonic(MonotonicClock::unretuned(
                R0.cast_unsigned(),
                Domain::Realtime,
                1,
            ))
        };
        let mut c = fallback();
        let mut d = Discipline::new(&c, cfg)?;
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut tick = 0_u64;
        for k in 0..100 {
            tick += 1_000_000_000;
            let wide = k % 10 == 9;
            let theta = noise(&mut seed, 900);
            let a = d.update_at(
                &c,
                &sample(tick, theta, if wide { 5_000 } else { 30 }, Some(R0)),
                Phase::Running,
                tick + 100,
            );
            assert_eq!(a.action == Action::Held, wide, "update {k}");
            assert_eq!(a.clock.params().mult, 1 << 32, "update {k}: θ {theta}");
            c = a.clock;
        }
        // a 50 ms wall step (the probe moves with it)
        let step = 50_000_000;
        let mut back = None;
        for k in 0..400 {
            tick += 1_000_000_000;
            let off = step + noise(&mut seed, 50);
            let a = d.update_at(
                &c,
                &sample(tick, off, 30, Some(R0 + step)),
                Phase::Running,
                tick + 100,
            );
            let one = a.clock.params().mult == 1 << 32;
            if k == 0 {
                assert!(!one, "a 50 ms gap slews");
                assert_ne!(a.flags & flags::SLEW_LIMITED, 0);
            }
            if one && back.is_none() {
                assert!(a.residual_ns.abs() <= 250, "back at θ {}", a.residual_ns);
                back = Some(k);
            }
            assert_eq!(one, back.is_some(), "update {k}: θ {}", a.residual_ns);
            c = a.clock;
        }
        assert!(back.is_some_and(|k| k > 100), "back after {back:?}");
        // noise around 900 ns, inside the band: never flips
        let mut c = fallback();
        let mut d = Discipline::new(&c, cfg)?;
        for k in 1..=200 {
            let t = k * 1_000_000_000;
            let a = d.update_at(
                &c,
                &sample(t, 900 + noise(&mut seed, 50), 30, Some(R0)),
                Phase::Running,
                t + 100,
            );
            assert_eq!(a.clock.params().mult, 1 << 32, "update {k}");
            c = a.clock;
        }
        Ok(())
    }

    /// Converging is flagged over 10 µs of residual, and not at it.
    #[test]
    fn converging_is_over_ten_microseconds() -> Res {
        for (theta, flagged) in [(10_000, false), (10_001, true), (-10_001, true)] {
            let (c, mut d) = discipline(DisciplineConfig::default())?;
            let a = d.update_at(
                &c,
                &sample(5_000_000, theta, 30, None),
                Phase::Running,
                5_000_100,
            );
            assert_eq!(a.flags & flags::CONVERGING != 0, flagged, "{theta}");
        }
        Ok(())
    }
}
