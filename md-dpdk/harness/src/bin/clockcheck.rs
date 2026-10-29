//! The benchmark clocks on this host: what each costs to read, and how far the
//! clients' clock (`lab::Clock`: quanta's, the CPU counter where quanta trusts
//! one) drifts from `CLOCK_MONOTONIC`.
//!
//! clockcheck [seconds]   (drift checkpoints up to `seconds`, default 60)
//!
//! Prints `host,clocksource,<name>` (the kernel's clock source, from sysfs),
//! then `clock,<name>,read_ns,<mean>,delta_ns,<p50>,<p99>,<max>,backwards,<n>`
//! per clock: the mean cost of one read over 10 M back-to-back reads, and the
//! distribution of the gap between consecutive reads in a second pass that
//! also records each gap (resolution plus cost plus the recording, plus
//! anything that interrupts the thread), and how many of those reads came
//! back lower than the one before (0 for a monotonic clock). The clocks:
//! `clock_gettime_monotonic` and `std_instant` (`CLOCK_MONOTONIC` direct and
//! through `std::time::Instant`), `clock_gettime_realtime` (`CLOCK_REALTIME`,
//! the clock timestamps are written with), `aeron_nano_clock`, quanta's raw
//! read (`quanta_raw`, in its ticks) and scaled read (`quanta_now`), and the
//! clients' latency stamp (`harness_stamp`, the ordered read) and unordered
//! read (`harness_now`). `host,quanta_ghz,<rate>`
//! is quanta's calibrated rate (1 where it reads `CLOCK_MONOTONIC`). Then
//! `drift,<seconds>,<clock ns>,<monotonic ns>,<ppm>,raw_ppm,<ppm>,mono_vs_raw_ppm,<ppm>`
//! at each checkpoint: the clients' clock's drift against `CLOCK_MONOTONIC`
//! and `CLOCK_MONOTONIC_RAW`, and NTP's slew of `CLOCK_MONOTONIC` itself.
//! Run it pinned (`taskset`) on an idle CPU.

#[path = "../lab.rs"]
#[allow(dead_code)]
mod lab;

use hdrhistogram::Histogram;
use lab::{Clock, ClockKind};
use std::error::Error;
use std::hint::black_box;
use std::time::{Duration, Instant};

const READS: u64 = 10_000_000;

/// Where Linux names the clock source the kernel keeps time with.
const CLOCKSOURCE: &str = "/sys/devices/system/clocksource/clocksource0/current_clocksource";

#[allow(
    clippy::cast_precision_loss,
    reason = "a diagnostic's ns and ppm figures: f64's 52-bit mantissa holds 52 days of ns exactly, and a ratio needs no more"
)]
fn measure(name: &str, ns_per_unit: f64, mut read: impl FnMut() -> u64) -> Result<(), Box<dyn Error>> {
    for _ in 0..1_000_000 {
        black_box(read());
    }
    let start = Instant::now();
    for _ in 0..READS {
        black_box(read());
    }
    let mean = start.elapsed().as_nanos() as f64 / READS as f64;

    let mut gaps = Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3)?;
    let mut backwards = 0u64;
    let mut last = read();
    for _ in 0..READS {
        let now = read();
        if now < last {
            backwards += 1;
        } else {
            // a gap in ns: not negative, and saturating where it would not fit
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a non-negative gap, saturated by the cast"
            )]
            gaps.saturating_record(((now - last) as f64 * ns_per_unit) as u64);
        }
        last = now;
    }
    println!(
        "clock,{name},read_ns,{mean:.1},delta_ns,{},{},{},backwards,{backwards}",
        gaps.value_at_quantile(0.5),
        gaps.value_at_quantile(0.99),
        gaps.max()
    );
    Ok(())
}

/// `clock_gettime(clock)`, ns.
#[allow(
    unsafe_code,
    reason = "CLOCK_MONOTONIC_RAW and a direct clock_gettime have no safe std equivalent"
)]
fn clock_gettime_ns(clock: libc::clockid_t) -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: clock_gettime writes only the timespec it is given.
    let rc = unsafe { libc::clock_gettime(clock, &raw mut ts) };
    assert_eq!(rc, 0, "clock_gettime({clock})");
    ts.tv_sec.cast_unsigned() * 1_000_000_000 + ts.tv_nsec.cast_unsigned()
}

/// `d` in ns, saturating at `u64::MAX` (584 years).
fn ns(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

/// `CLOCK_MONOTONIC_RAW`, ns: the counter as the kernel scales it, with no
/// NTP slew.
fn monotonic_raw_ns() -> u64 {
    clock_gettime_ns(libc::CLOCK_MONOTONIC_RAW)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a diagnostic's ns and ppm figures: f64's 52-bit mantissa holds 52 days of ns exactly, and a ratio needs no more"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let seconds: u64 = std::env::args()
        .nth(1)
        .map_or(Ok(60), |s| s.parse().map_err(|e| format!("seconds {s:?}: {e}")))?;
    let clocksource = std::fs::read_to_string(CLOCKSOURCE).unwrap_or_default();
    println!("host,clocksource,{}", clocksource.trim());

    measure("clock_gettime_monotonic", 1.0, || {
        clock_gettime_ns(libc::CLOCK_MONOTONIC)
    })?;
    let origin = Instant::now();
    measure("std_instant", 1.0, || ns(origin.elapsed()))?;
    measure("clock_gettime_realtime", 1.0, || clock_gettime_ns(libc::CLOCK_REALTIME))?;
    measure("aeron_nano_clock", 1.0, || {
        rusteron_client::Aeron::nano_clock().cast_unsigned()
    })?;

    // quanta's own reads, then the clients' clock on it (whatever BENCH_CLOCK)
    let quanta = quanta::Clock::new();
    let clock = Clock::new(ClockKind::Quanta);
    // ns per raw unit, through quanta's calibrated factor
    let ns_per_tick = quanta.delta_as_nanos(0, 1 << 32) as f64 / (1u64 << 32) as f64;
    println!("host,quanta_ghz,{:.4}", 1.0 / ns_per_tick);
    measure("quanta_raw", ns_per_tick, || quanta.raw())?;
    let anchor = quanta.now();
    measure("quanta_now", 1.0, || ns(quanta.now().duration_since(anchor)))?;
    measure("harness_stamp", 1.0, || clock.stamp())?;
    measure("harness_now", 1.0, || clock.now())?;

    // drift of the clients' clock against CLOCK_MONOTONIC and CLOCK_MONOTONIC_RAW
    let (t0, i0, r0) = (clock.now(), Instant::now(), monotonic_raw_ns());
    for checkpoint in [1u64, 10, 30, 60, 120, 300, 600].into_iter().filter(|&c| c <= seconds) {
        while i0.elapsed() < Duration::from_secs(checkpoint) {
            std::thread::sleep(Duration::from_millis(1));
        }
        let (t, i, r) = (clock.now(), Instant::now(), monotonic_raw_ns());
        let clock_ns = (t - t0) as f64;
        let mono_ns = (i - i0).as_nanos() as f64;
        let raw = (r - r0) as f64;
        println!(
            "drift,{checkpoint},{clock_ns:.0},{mono_ns:.0},{:.3},raw_ppm,{:.3},mono_vs_raw_ppm,{:.3}",
            (clock_ns - mono_ns) / mono_ns * 1e6,
            (clock_ns - raw) / raw * 1e6,
            (mono_ns - raw) / raw * 1e6
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [ClockKind; 2] = [ClockKind::Quanta, ClockKind::Monotonic];

    /// The tightest of a few bracketed reads: (Instant, the clock's ns).
    fn sample(clock: &Clock) -> (Instant, u64) {
        let mut best = (Duration::MAX, Instant::now(), 0);
        for _ in 0..16 {
            let i0 = Instant::now();
            let ns = clock.now();
            let i1 = Instant::now();
            if i1 - i0 < best.0 {
                best = (i1 - i0, i0 + (i1 - i0) / 2, ns);
            }
        }
        (best.1, best.2)
    }

    /// `BENCH_CLOCK=monotonic` stamps with `CLOCK_MONOTONIC`; anything else
    /// (the lab's `tsc`), or unset, with quanta.
    #[test]
    fn bench_clock_monotonic_is_the_only_way_off_quanta() {
        assert_eq!(ClockKind::from_bench_clock(Some("monotonic")), ClockKind::Monotonic);
        for other in [None, Some("tsc"), Some("counter"), Some(""), Some("Monotonic")] {
            assert_eq!(ClockKind::from_bench_clock(other), ClockKind::Quanta, "{other:?}");
        }
        assert_eq!(Clock::new(ClockKind::Monotonic).kind(), ClockKind::Monotonic);
        assert_eq!(Clock::new(ClockKind::Quanta).kind(), ClockKind::Quanta);
    }

    /// A latency stamp pair, the ordered read at both ends, measures a
    /// known sleep in nanoseconds, with either clock: at least the sleep (the
    /// kernel sleeps no less on its monotonic clock), at most the `Instant`
    /// bracket around the pair, each within the calibration's rate error
    /// (0.1%) and a microsecond. Raw ticks, or a clock that is not counting
    /// nanoseconds, fall outside one bound or the other. Which source quanta
    /// reads is its own choice (it has no public way to say): the stamps are
    /// what is checked.
    #[test]
    fn a_stamp_pair_measures_a_known_sleep() -> Result<(), std::num::TryFromIntError> {
        for kind in KINDS {
            let clock = Clock::new(kind);
            for sleep_ms in [1u64, 20, 100] {
                let sleep_ns = sleep_ms * 1_000_000;
                let i0 = Instant::now();
                let s0 = clock.stamp();
                std::thread::sleep(Duration::from_millis(sleep_ms));
                let s1 = clock.stamp();
                let outer = u64::try_from(i0.elapsed().as_nanos())?;
                let measured = s1.wrapping_sub(s0);
                assert!(
                    measured + sleep_ns / 1000 + 1_000 >= sleep_ns,
                    "{kind:?}: a {sleep_ms} ms sleep measured {measured} ns: less than the sleep"
                );
                assert!(
                    measured <= outer + outer / 1000 + 1_000,
                    "{kind:?}: a {sleep_ms} ms sleep measured {measured} ns: more than the {outer} ns bracket around it"
                );
            }
        }
        Ok(())
    }

    /// The unordered read (pacing, deadlines, loop gaps) counts nanoseconds
    /// from the clock's calibration that track `CLOCK_MONOTONIC` within 0.1%
    /// over 50 ms, with either clock.
    #[test]
    fn the_clock_tracks_real_time() -> Result<(), std::num::TryFromIntError> {
        for kind in KINDS {
            let start = Instant::now();
            let clock = Clock::new(kind);
            let made = start.elapsed();
            let (i0, c0) = sample(&clock);
            // ns since the clock was made, which `Instant` brackets
            let (since, made) = (u64::try_from((i0 - start).as_nanos())?, u64::try_from(made.as_nanos())?);
            assert!(
                c0 <= since + since / 1000 + 1_000 && c0 + made + since / 1000 + 1_000 >= since,
                "{kind:?}: {c0} ns since the clock was made, {since} ns since before it ({made} ns to make)"
            );
            while i0.elapsed() < Duration::from_millis(50) {
                std::hint::spin_loop();
            }
            let (i1, c1) = sample(&clock);
            let mono = u64::try_from((i1 - i0).as_nanos())?;
            let err = (c1 - c0).abs_diff(mono);
            assert!(
                err * 1000 <= mono,
                "{kind:?}: clock advanced {} ns against {mono} ns monotonic",
                c1 - c0
            );
        }
        Ok(())
    }
}
