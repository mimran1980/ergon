//! The kernel's clocks, read exactly (cold paths: through the vDSO), and the
//! offset from a clock to the wall clock.

use crate::arch;
use crate::clock::{Clock, TimeSource};
use crate::error::Error;

/// The platform's monotonic clock, `std::time::Instant`'s: Linux's
/// `CLOCK_MONOTONIC` (NTP slews its rate); macOS's `CLOCK_UPTIME_RAW` (its
/// `CLOCK_MONOTONIC` has microsecond resolution).
#[cfg(target_os = "macos")]
pub const MONOTONIC: libc::clockid_t = libc::CLOCK_UPTIME_RAW;
/// The platform's monotonic clock, `std::time::Instant`'s.
#[cfg(not(target_os = "macos"))]
pub const MONOTONIC: libc::clockid_t = libc::CLOCK_MONOTONIC;
/// The monotonic clock's name in reports.
pub const MONOTONIC_NAME: &str = if cfg!(target_os = "macos") {
    "CLOCK_UPTIME_RAW"
} else {
    "CLOCK_MONOTONIC"
};

/// The unslewed clock calibration measures against: `CLOCK_MONOTONIC_RAW`
/// (Linux: the kernel's own scaling of its clock source, never adjusted;
/// macOS: the timebase, likewise).
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
pub const RAW: libc::clockid_t = libc::CLOCK_MONOTONIC_RAW;
/// Elsewhere there is no raw clock: the monotonic one stands in.
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
pub const RAW: libc::clockid_t = libc::CLOCK_MONOTONIC;
/// The raw clock's name in reports.
pub const RAW_NAME: &str = if cfg!(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos"
)) {
    "CLOCK_MONOTONIC_RAW"
} else {
    "CLOCK_MONOTONIC"
};

/// The wall clock.
pub const REALTIME: libc::clockid_t = libc::CLOCK_REALTIME;

/// `CLOCK_REALTIME`, ns since the UNIX epoch: the domain of the kernel's
/// receive timestamps (`SO_TIMESTAMPING`) and of NIC hardware timestamps
/// that chrony keeps the wall clock on.
#[must_use]
pub fn realtime_ns() -> i64 {
    arch::clock_ns_unchecked(REALTIME).cast_signed()
}

/// The platform's monotonic clock, ns (see [`Domain::Monotonic`](crate::Domain)).
#[must_use]
pub fn monotonic_ns() -> u64 {
    arch::clock_ns_unchecked(MONOTONIC)
}

/// `CLOCK_MONOTONIC_RAW`, ns: never slewed, the reference calibration
/// measures the counter against.
#[must_use]
pub fn monotonic_raw_ns() -> u64 {
    arch::clock_ns_unchecked(RAW)
}

/// One clock, checked.
pub fn read(id: libc::clockid_t, name: &'static str) -> Result<u64, Error> {
    arch::clock_ns(id).map_err(|errno| Error::ClockRead { clock: name, errno })
}

/// The monotonic clock, checked.
pub fn monotonic() -> Result<u64, Error> {
    read(MONOTONIC, MONOTONIC_NAME)
}

/// The raw clock, checked.
pub fn raw() -> Result<u64, Error> {
    read(RAW, RAW_NAME)
}

/// The smallest step `read` is seen to advance by: the least non-zero
/// difference between consecutive reads, over its first 64 advances or
/// `max_reads` reads; `None` when it never advanced. 64, not a handful: a
/// clock's first reads are its slowest (caches cold, a vCPU just
/// scheduled), and each slow pair is seen as a larger step.
pub fn observed_step(
    mut read: impl FnMut() -> Result<u64, Error>,
    max_reads: u32,
) -> Result<Option<u64>, Error> {
    let mut previous = read()?;
    let mut smallest: Option<u64> = None;
    let mut advances = 0;
    for _ in 0..max_reads {
        let now = read()?;
        if now > previous {
            let step = now - previous;
            smallest = Some(smallest.map_or(step, |s| s.min(step)));
            advances += 1;
            if advances == 64 {
                break;
            }
        }
        previous = now;
    }
    Ok(smallest)
}

/// A kernel clock's resolution, ns: the step its reads are seen to
/// advance by (the clock source's tick, or a read's cost where that is
/// longer), which is what bounds a read's error. Not `clock_getres`: on
/// Linux that is the timer resolution, 1 ns with high-resolution timers
/// (whatever the clock source's tick: 41.7 ns for a 24 MHz Arm counter)
/// and a jiffy without them (1 ms, for clocks whose reads advance by
/// nanoseconds). `clock_getres` only for a clock that did not advance in a
/// million reads.
pub fn resolution_ns(id: libc::clockid_t, name: &'static str) -> Result<u64, Error> {
    observed_step(|| read(id, name), 1_000_000)?.map_or_else(
        || arch::clock_res_ns(id).map_err(|errno| Error::ClockRead { clock: name, errno }),
        Ok,
    )
}

/// The monotonic clock's resolution, ns ([`resolution_ns`]).
pub fn monotonic_res_ns() -> Result<u64, Error> {
    resolution_ns(MONOTONIC, MONOTONIC_NAME)
}

/// `CLOCK_REALTIME`'s resolution, ns: the step its reads advance by.
///
/// Up to a million reads, usually 64: microseconds, cold. What a
/// [`Sampler`]'s error bound takes as the wall clock's; measure it once,
/// where a clock is made, never on a hot path.
///
/// # Errors
/// [`Error::ClockRead`] when the wall clock cannot be read.
pub fn realtime_res_ns() -> Result<u64, Error> {
    resolution_ns(REALTIME, "CLOCK_REALTIME")
}

/// The tightest of 16 brackets `outer, inner, outer`: `(outer at the
/// bracket's middle, inner, the bracket's width)`, all in their own units.
pub fn bracket(
    mut inner: impl FnMut() -> Result<u64, Error>,
    mut outer: impl FnMut() -> Result<u64, Error>,
) -> Result<(u64, u64, u64), Error> {
    let mut best = (0, 0, u64::MAX);
    for _ in 0..16 {
        let a = outer()?;
        let value = inner()?;
        let b = outer()?;
        let width = b.wrapping_sub(a);
        if width < best.2 {
            best = (a + width / 2, value, width);
        }
    }
    Ok(best)
}

/// One measurement of the wall clock against a clock's ticks: what a
/// [`Discipline`](crate::advanced::discipline::Discipline) update consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    /// The ticks at the middle of the tightest bracket: the instant
    /// `wall_ns` is taken to have been read at.
    pub mid_ticks: u64,
    /// `CLOCK_REALTIME`, ns since the epoch, read inside that bracket.
    pub wall_ns: i64,
    /// How far `wall_ns` may be from the wall clock at `mid_ticks`, ns:
    /// half of the bracket's width plus half of each clock's resolution,
    /// rounded up (as [`WallOffset`] bounds its error). Never zero.
    pub error_ns: u64,
    /// The tightest bracket's width, ticks.
    pub width_ticks: u64,
    /// `CLOCK_REALTIME - CLOCK_MONOTONIC`, where that moves only when the
    /// wall clock steps (Linux); `None` elsewhere.
    pub step_probe: Option<StepProbe>,
}

/// `CLOCK_REALTIME - CLOCK_MONOTONIC`, bracketed.
///
/// On Linux both clocks are one timekeeper's, slewed together by NTP, so
/// the difference moves only when the wall clock is stepped
/// (`settimeofday`, chrony's `makestep`, an unsmeared leap second): a slew
/// of any rate leaves it alone. Comparing two of them tells a step from a
/// slew exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepProbe {
    /// The wall clock less the monotonic clock at the bracket's middle, ns.
    pub real_minus_mono_ns: i64,
    /// The bracket's width on the monotonic clock, ns.
    pub width_ns: u64,
}

/// What a [`Sampler`] reads: the kernel's clocks and the clock's counter
/// ([`KernelSource`]), or a scripted fake that makes a discipline's tests
/// deterministic.
pub trait SampleSource {
    /// The clock's ticks, ordered (taken after every earlier instruction).
    fn ticks_ordered(&mut self) -> u64;
    /// `CLOCK_REALTIME`, ns since the epoch.
    fn realtime_ns(&mut self) -> i64;
    /// `CLOCK_MONOTONIC`, ns, where `CLOCK_REALTIME - CLOCK_MONOTONIC`
    /// moves only on wall-clock steps (Linux); `None` elsewhere (macOS,
    /// whose monotonic clock NTP does not slew).
    fn monotonic_ns(&mut self) -> Option<i64>;
}

/// The real clocks: `clock`'s ordered read, and the kernel's wall and
/// monotonic clocks through the vDSO (no system call).
#[derive(Clone, Copy, Debug)]
pub struct KernelSource<'a>(pub &'a Clock);

impl SampleSource for KernelSource<'_> {
    #[inline]
    fn ticks_ordered(&mut self) -> u64 {
        self.0.ticks_ordered()
    }

    #[inline]
    fn realtime_ns(&mut self) -> i64 {
        realtime_ns()
    }

    #[inline]
    fn monotonic_ns(&mut self) -> Option<i64> {
        if cfg!(any(target_os = "linux", target_os = "android")) {
            Some(monotonic_ns().cast_signed())
        } else {
            None
        }
    }
}

/// How many recent samples the early-exit floors remember.
const FLOOR_SAMPLES: usize = 64;

/// Takes [`Sample`]s: the tightest of a few brackets of the wall clock
/// between two ordered reads of the clock, stopping early once one is as
/// tight as the recent ones were.
///
/// Its state is two inline rings (the tightest widths of the last 64
/// samples, of the wall-clock brackets and of the step probes) and three
/// numbers: no allocation, no global state. A sample costs one to `tries`
/// brackets (each two ordered reads and a vDSO read: 45 to 70 ns on the
/// hosts measured in `clock/BENCHMARKS.md`) and one to `d_tries` step
/// probes (three vDSO reads each).
#[derive(Clone, Debug)]
pub struct Sampler {
    /// ns per tick, 32.32, at the calibrated rate: a bracket's width in ns.
    nominal_mult: u64,
    /// The clock's resolution, ns.
    tick_ns: u64,
    /// The wall clock's resolution, ns.
    wall_res_ns: u64,
    /// The tightest bracket of each of the last 64 samples, ticks
    /// (`u32::MAX`: none yet, or wider).
    widths: [u32; FLOOR_SAMPLES],
    /// The tightest step probe of each, ns (likewise).
    probe_widths: [u32; FLOOR_SAMPLES],
    /// Where the next sample's widths go.
    next: usize,
}

impl Sampler {
    /// A sampler for a clock whose calibrated factor is `nominal_mult` (ns
    /// per tick, 32.32), whose reads advance by `tick_ns`, against a wall
    /// clock whose reads advance by `wall_res_ns` ([`realtime_res_ns`]).
    #[must_use]
    pub const fn new(nominal_mult: u64, tick_ns: u64, wall_res_ns: u64) -> Self {
        Self {
            nominal_mult,
            tick_ns,
            wall_res_ns,
            widths: [u32::MAX; FLOOR_SAMPLES],
            probe_widths: [u32::MAX; FLOOR_SAMPLES],
            next: 0,
        }
    }

    /// [`new`](Self::new) for `clock` as it is now (its factor and
    /// resolution): make it from the calibration's clock, before any
    /// discipline retunes it.
    #[must_use]
    pub fn for_clock(clock: &Clock, wall_res_ns: u64) -> Self {
        Self::new(clock.params().mult, clock.resolution_ns(), wall_res_ns)
    }

    /// One sample: up to `tries` brackets (`ticks_ordered`, `realtime_ns`,
    /// `ticks_ordered`), the tightest kept, then up to `d_tries` step probes
    /// (`monotonic_ns`, `realtime_ns`, `monotonic_ns`) where the source has
    /// them. Each loop stops early once a bracket is within a quarter (and
    /// one unit) of the tightest of the last 64 samples. At least one
    /// bracket is taken; no probe where `d_tries` is 0.
    pub fn take(&mut self, src: &mut impl SampleSource, tries: u32, d_tries: u32) -> Sample {
        let floor = floor_of(&self.widths);
        let mut best = (0_u64, 0_i64, u64::MAX);
        for _ in 0..tries.max(1) {
            let a = src.ticks_ordered();
            let wall = src.realtime_ns();
            let b = src.ticks_ordered();
            let width = b.wrapping_sub(a);
            if width < best.2 {
                best = (a.wrapping_add(width / 2), wall, width);
            }
            if tight_enough(width, floor) {
                break;
            }
        }
        let (mid_ticks, wall_ns, width_ticks) = best;
        let step_probe = self.probe(src, d_tries);
        self.widths[self.next] = clamp_u32(width_ticks);
        self.probe_widths[self.next] = step_probe.map_or(u32::MAX, |p| clamp_u32(p.width_ns));
        self.next = (self.next + 1) % FLOOR_SAMPLES;
        let width_ns = crate::scale::ticks_to_ns(width_ticks, self.nominal_mult);
        let error_ns = width_ns
            .saturating_add(self.tick_ns)
            .saturating_add(self.wall_res_ns)
            .div_ceil(2)
            .max(1);
        Sample {
            mid_ticks,
            wall_ns,
            error_ns,
            width_ticks,
            step_probe,
        }
    }

    /// The tightest of up to `tries` step probes, or `None` where the
    /// source has no step-only monotonic clock (or `tries` is 0).
    fn probe(&self, src: &mut impl SampleSource, tries: u32) -> Option<StepProbe> {
        let floor = floor_of(&self.probe_widths);
        let mut best: Option<StepProbe> = None;
        for _ in 0..tries {
            let m1 = src.monotonic_ns()?;
            let wall = src.realtime_ns();
            let m2 = src.monotonic_ns()?;
            let width_ns = m2.wrapping_sub(m1).cast_unsigned();
            let mid = m1.wrapping_add((width_ns / 2).cast_signed());
            if best.is_none_or(|b| width_ns < b.width_ns) {
                best = Some(StepProbe {
                    real_minus_mono_ns: wall.wrapping_sub(mid),
                    width_ns,
                });
            }
            if tight_enough(width_ns, floor) {
                break;
            }
        }
        best
    }
}

/// The least of a ring of widths (`u32::MAX` where it holds none).
fn floor_of(ring: &[u32; FLOOR_SAMPLES]) -> u32 {
    ring.iter().copied().min().unwrap_or(u32::MAX)
}

/// Whether `width` is as tight as the floor allows: within a quarter of it,
/// and one unit. Never, with no floor yet.
const fn tight_enough(width: u64, floor: u32) -> bool {
    if floor == u32::MAX {
        return false;
    }
    let floor = floor as u64;
    width <= floor + floor / 4 + 1
}

/// A width in 32 bits, saturating below the "none" marker.
#[allow(clippy::cast_possible_truncation)] // range-checked first
const fn clamp_u32(width: u64) -> u32 {
    if width >= u32::MAX as u64 {
        u32::MAX - 1
    } else {
        width as u32
    }
}

/// The offset from a clock's nanoseconds to `CLOCK_REALTIME`, with its error
/// bound: the cheap way to put a counter stamp on the wall clock (one add)
/// for comparison with kernel or NIC timestamps.
///
/// The counter runs at `CLOCK_MONOTONIC_RAW`'s rate; the wall clock is
/// slewed by the NTP client (by hundreds of ppm while systemd-timesyncd
/// removes a boot's offset, about a ppm once settled: `clock/BENCHMARKS.md`),
/// so an offset goes stale at that rate: its owner measures it again off the
/// hot path and keeps the copy the hot path reads.
///
/// Its error bound is the measurement's (the bracket and both clocks'
/// resolutions), not the wall clock's own error against UTC: that is the
/// time-sync bound ([`TimeSync::bound`](crate::advanced::TimeSync::bound)), from
/// microseconds disciplined to a PTP hardware clock to none at all.
///
/// On a clock its owner disciplines to the wall clock
/// ([`Discipline`](crate::advanced::discipline::Discipline)) the offset is the
/// residual: tens of nanoseconds once settled, not the milliseconds an hour
/// an undisciplined counter drifts by. Its error still excludes
/// `CLOCK_REALTIME`'s own error against UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WallOffset {
    offset: i64,
    error: u64,
    measured_at: u64,
}

impl WallOffset {
    /// Measures it now: the tightest of 16 brackets (the clock's ordered
    /// read, `CLOCK_REALTIME`, the ordered read again). The error is half the
    /// bracket plus half of each clock's resolution (the wall clock's: the
    /// step its reads are seen to advance by), rounded up: never zero.
    #[must_use]
    pub fn measure<C: TimeSource>(clock: &C) -> Self {
        let wall_res = resolution_ns(REALTIME, "CLOCK_REALTIME")
            .unwrap_or_else(|_| arch::clock_res_ns(REALTIME).unwrap_or(1));
        Self::measure_with(clock, wall_res)
    }

    /// [`measure`](Self::measure) with the wall clock's resolution given.
    fn measure_with<C: TimeSource>(clock: &C, wall_res: u64) -> Self {
        let mut best = (u64::MAX, 0_u64, 0_i64);
        for _ in 0..16 {
            let a = clock.now_ns_ordered();
            let wall = realtime_ns();
            let b = clock.now_ns_ordered();
            let width = b.saturating_sub(a);
            if width < best.0 {
                best = (width, a + width / 2, wall);
            }
        }
        let (width, at, wall) = best;
        let error_ns = (width + clock.resolution_ns() + wall_res).div_ceil(2);
        Self {
            offset: wall.wrapping_sub(at.cast_signed()),
            error: error_ns,
            measured_at: at,
        }
    }

    /// The wall-clock time, ns since the epoch, of the clock's `clock_ns`.
    #[inline]
    #[must_use]
    pub const fn to_realtime_ns(&self, clock_ns: u64) -> i64 {
        clock_ns.wrapping_add_signed(self.offset).cast_signed()
    }

    /// The clock's nanoseconds at wall-clock time `realtime_ns`.
    #[inline]
    #[must_use]
    pub const fn from_realtime_ns(&self, realtime_ns: i64) -> u64 {
        realtime_ns.wrapping_sub(self.offset).cast_unsigned()
    }

    /// `CLOCK_REALTIME` minus the clock, ns.
    #[must_use]
    pub const fn offset_ns(&self) -> i64 {
        self.offset
    }

    /// How far off the offset may be, ns, when measured.
    #[must_use]
    pub const fn error_ns(&self) -> u64 {
        self.error
    }

    /// The clock's time, ns, when it was measured (its age, for its owner).
    #[must_use]
    pub const fn measured_at_ns(&self) -> u64 {
        self.measured_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manual::ManualClock;
    use std::cell::Cell;

    /// The tightest bracket wins, and its outer time is the bracket's
    /// middle: the inner read's best estimate on the outer clock.
    #[test]
    fn a_bracket_is_its_tightest_pairs_middle() -> Result<(), Error> {
        // outer reads in pairs around each inner read: widths 30, 10, 20,
        // then 40 for the other 13 of the 16
        let mut outer = vec![100_u64, 130, 200, 210, 300, 320];
        for k in 0..13 {
            outer.extend([1_000 + 100 * k, 1_040 + 100 * k]);
        }
        let i = Cell::new(0_usize);
        let next = || {
            let k = i.get();
            i.set(k + 1);
            Ok(outer[k])
        };
        let inner = Cell::new(0_u64);
        let (mid, value, width) = bracket(
            || {
                inner.set(inner.get() + 1);
                Ok(inner.get())
            },
            next,
        )?;
        assert_eq!((mid, value, width), (205, 2, 10));
        Ok(())
    }

    /// The offset's error is never zero: a clock that does not move between
    /// the bracket's reads still has half of each clock's resolution of
    /// error (macOS's realtime clock: a microsecond).
    #[test]
    fn the_offset_error_includes_both_clocks_resolutions() {
        let clock = ManualClock::new(5_000);
        let offset = WallOffset::measure_with(&clock, 1_000);
        assert_eq!(offset.error_ns(), (1 + 1_000_u64).div_ceil(2));
        assert_eq!(offset.measured_at_ns(), 5_000);
        let offset = WallOffset::measure(&clock);
        assert!(offset.error_ns() >= 1);
    }

    /// A source that plays back scripted reads, and counts them.
    struct Script {
        ticks: Vec<u64>,
        wall: Vec<i64>,
        mono: Option<Vec<i64>>,
        reads: (usize, usize, usize),
    }

    impl SampleSource for Script {
        fn ticks_ordered(&mut self) -> u64 {
            self.reads.0 += 1;
            self.ticks[self.reads.0 - 1]
        }
        fn realtime_ns(&mut self) -> i64 {
            self.reads.1 += 1;
            self.wall[self.reads.1 - 1]
        }
        fn monotonic_ns(&mut self) -> Option<i64> {
            let mono = self.mono.as_ref()?;
            self.reads.2 += 1;
            Some(mono[self.reads.2 - 1])
        }
    }

    /// A sample is the tightest of its brackets, at the bracket's middle,
    /// with the error bound `WallOffset` gives a bracket of that width (half
    /// the width plus half of each clock's resolution, rounded up); its
    /// step probe is the tightest probe's wall-less-monotonic at its middle.
    #[test]
    fn a_sample_is_its_tightest_bracket() {
        // brackets of widths 30, 10, 20 ticks at 1 GHz (m0 = 2^32)
        let mut src = Script {
            ticks: vec![100, 130, 200, 210, 300, 320],
            wall: vec![5_000, 6_000, 7_000, 9_000, 9_100, 9_200],
            mono: Some(vec![1_000, 1_040, 2_000, 2_010, 3_000, 3_030]),
            reads: (0, 0, 0),
        };
        let mut sampler = Sampler::new(1 << 32, 1, 20);
        let s = sampler.take(&mut src, 3, 3);
        assert_eq!((s.mid_ticks, s.wall_ns, s.width_ticks), (205, 6_000, 10));
        assert_eq!(s.error_ns, 16, "(10 + 1 + 20) / 2, rounded up");
        assert_eq!(
            s.step_probe,
            Some(StepProbe {
                real_minus_mono_ns: 9_100 - 2_005,
                width_ns: 10
            })
        );
        assert_eq!(src.reads, (6, 6, 6), "every try taken: no floor yet");
    }

    /// Once a sample has set a floor (the tightest of the last 64), a
    /// bracket within a quarter of it
    /// (and one unit) ends the loop: the first try usually; a wider one
    /// tries again. No step probe where the source has none, or `d_tries`
    /// is 0.
    #[test]
    fn a_sample_stops_at_a_bracket_as_tight_as_the_recent_ones() {
        let mut sampler = Sampler::new(1 << 32, 1, 1);
        let mut first = Script {
            ticks: vec![0, 40],
            wall: vec![1],
            mono: None,
            reads: (0, 0, 0),
        };
        let s = sampler.take(&mut first, 1, 3);
        assert_eq!((s.width_ticks, s.step_probe), (40, None));
        // a bracket of 51 (40 + 10 + 1) ends the loop; 52 does not
        for (width, brackets) in [(51, 1), (52, 2)] {
            let mut src = Script {
                ticks: vec![0, width, 100, 140],
                wall: vec![1, 2],
                mono: Some(vec![0, 10, 20, 30, 40, 50]),
                reads: (0, 0, 0),
            };
            let s = sampler.take(&mut src, 8, 0);
            assert_eq!(src.reads.0, 2 * brackets, "width {width}");
            assert_eq!(s.step_probe, None, "no probe at d_tries 0");
            assert_eq!(src.reads.2, 0);
        }
    }

    /// The kernel's clocks through a clock: a sample of the wall clock
    /// against the fallback agrees with `WallOffset`'s measurement, and the
    /// step probe exists on Linux only.
    #[test]
    fn the_kernel_source_samples_the_wall_clock() -> Result<(), Error> {
        let (clock, error) = Clock::fallback(crate::Domain::Realtime);
        assert_eq!(error, None);
        let mut sampler = Sampler::for_clock(&clock, realtime_res_ns()?);
        let s = sampler.take(&mut KernelSource(&clock), 8, 3);
        let offset = WallOffset::measure(&clock);
        let residual = s.wall_ns - clock.to_ns(s.mid_ticks).cast_signed();
        assert!(
            (residual - offset.offset_ns()).unsigned_abs()
                <= 2 * (s.error_ns + offset.error_ns()) + 1_000_000,
            "{residual} against {offset:?}"
        );
        assert_eq!(
            s.step_probe.is_some(),
            cfg!(any(target_os = "linux", target_os = "android"))
        );
        Ok(())
    }

    /// A clock's resolution is the step its reads are seen to advance by,
    /// not what `clock_getres` says: Linux's is the timer resolution (a
    /// jiffy on a kernel without high-resolution timers, 1 ns with them),
    /// whatever the clock source's tick.
    #[test]
    fn a_clocks_resolution_is_the_step_its_reads_advance_by() -> Result<(), Error> {
        // a 24 MHz counter in ns, read twice a tick: 0, 0, 41, 41, 83, ...
        let k = Cell::new(0_u64);
        let coarse = || {
            let i = k.get();
            k.set(i + 1);
            Ok(i / 2 * 125 / 3)
        };
        assert_eq!(observed_step(coarse, 1_000)?, Some(41));
        // a fine clock read every 18 to 23 ns
        let k = Cell::new(0_u64);
        let fine = || {
            let i = k.get();
            k.set(i + 1);
            Ok(i * 20 + [0, 3, 1, 2][usize::try_from(i % 4).unwrap_or(0)])
        };
        assert_eq!(observed_step(fine, 1_000)?, Some(18));
        // a clock that never advances: no step seen
        assert_eq!(observed_step(|| Ok(5), 1_000)?, None);
        // a slow start (the vDSO's page and this code not yet cached, a
        // vCPU just scheduled): the 24 MHz counter read once every four
        // ticks for its first 16 advances, then once a tick, is still seen
        // to advance by one tick (in a loaded VM, 10 starts in 300 saw
        // 166 ns for a 41.7 ns tick in their first 8 advances)
        let k = Cell::new(0_u64);
        let slow_start = || {
            let i = k.get();
            k.set(i + 1);
            let ticks = if i < 16 { i * 4 } else { 48 + i };
            Ok(ticks * 125 / 3)
        };
        assert_eq!(observed_step(slow_start, 1_000)?, Some(41));
        // the kernel's clocks advance, by under a millisecond
        for id in [MONOTONIC, RAW, REALTIME] {
            let step = resolution_ns(id, "clock")?;
            assert!((1..1_000_000).contains(&step), "{id}: {step} ns");
        }
        Ok(())
    }
}
