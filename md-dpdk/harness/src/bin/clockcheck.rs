//! The benchmark clocks on this host: what each costs to read, and how far the
//! clients' clock (`lab::Clock`: ergon-clock's calibrated counter, the TSC on
//! x86-64, `CNTVCT_EL0` on aarch64) drifts from `CLOCK_MONOTONIC`.
//!
//! clockcheck [seconds]   (drift checkpoints up to `seconds`, default 60)
//!
//! Prints `clock,<name>,read_ns,<mean>,delta_ns,<p50>,<p99>,<max>,backwards,<n>`
//! per clock: the mean cost of one read over 10 M back-to-back reads, and the
//! distribution of the gap between consecutive reads in a second pass that
//! also records each gap (resolution plus cost plus the recording, plus
//! anything that interrupts the thread), and how many of those reads came
//! back lower than the one before (0 for a monotonic clock; the bare counter
//! reads, `rdtsc` and `cntvct`, are the transport's hot-path reads, which
//! wait for no earlier instruction). The counter's reads are ergon-clock's
//! (`ergon_clock::advanced::raw`): bare, `rdtscp` or `isb; mrs cntvct_el0`, and
//! `CNTVCTSS_EL0` where the CPU has FEAT_ECV; then the clients' latency
//! stamp (`<scaled>`, the ordered read in ns) and their unordered read
//! (`<scaled>_unordered`). Then
//! `drift,<seconds>,<counter ns>,<monotonic ns>,<ppm>,minstant_ppm,<ppm>,raw_ppm,<ppm>,
//! mono_vs_raw_ppm,<ppm>` at each checkpoint: the clients' clock's and
//! `minstant`'s drift against `CLOCK_MONOTONIC`, the clock's against
//! `CLOCK_MONOTONIC_RAW`, and NTP's slew of `CLOCK_MONOTONIC` itself; and
//! `host,...` lines, as ergon-clock detects them: CPUID's invariant-TSC bit,
//! on aarch64 the counter's frequency register and whether the CPU has
//! FEAT_ECV, and the kernel clock source. ergon-clock's calibration report
//! is the `harness clock:` line on stderr.
//! Run it pinned (`taskset`) on an idle CPU.

#[path = "../lab.rs"]
#[allow(dead_code)]
mod lab;

use ergon_clock::advanced::HostFacts;
use hdrhistogram::Histogram;
use lab::Clock;
use std::hint::black_box;
use std::time::{Duration, Instant};

const READS: u64 = 10_000_000;

/// The counter's names in the output: (bare read, ordered read, scaled clock,
/// rate), x86-64's as they always were.
const NAMES: (&str, &str, &str, &str) = if cfg!(target_arch = "aarch64") {
    ("cntvct", "cntvct_isb", "counter_clock_ns", "counter_ghz")
} else {
    ("rdtsc", "rdtscp", "tsc_clock_ns", "tsc_ghz")
};

fn measure(name: &str, ns_per_unit: f64, mut read: impl FnMut() -> u64) {
    for _ in 0..1_000_000 {
        black_box(read());
    }
    let start = Instant::now();
    for _ in 0..READS {
        black_box(read());
    }
    let mean = start.elapsed().as_nanos() as f64 / READS as f64;

    let mut gaps = Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3).unwrap();
    let mut backwards = 0u64;
    let mut last = read();
    for _ in 0..READS {
        let now = read();
        if now < last {
            backwards += 1;
        } else {
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
}

/// The counter's bare and ordered reads, ergon-clock's own instructions:
/// x86-64's `rdtsc` and `rdtscp`.
#[cfg(target_arch = "x86_64")]
fn counter_reads(ns_per_tick: f64) {
    let (bare, ordered, _, _) = NAMES;
    measure(bare, ns_per_tick, ergon_clock::advanced::raw::rdtsc);
    if let Some(rdtscp) = ergon_clock::advanced::raw::Rdtscp::detect() {
        measure(ordered, ns_per_tick, || rdtscp.read());
    }
}

/// The counter's bare and ordered reads, ergon-clock's own instructions:
/// aarch64's `mrs cntvct_el0`, `isb; mrs cntvct_el0` and, where the CPU has
/// FEAT_ECV, `mrs cntvctss_el0` (the ordered read ergon-clock then takes).
#[cfg(target_arch = "aarch64")]
fn counter_reads(ns_per_tick: f64) {
    let (bare, ordered, _, _) = NAMES;
    measure(bare, ns_per_tick, ergon_clock::advanced::raw::cntvct);
    measure(ordered, ns_per_tick, ergon_clock::advanced::raw::isb_cntvct);
    if let Some(ecv) = ergon_clock::advanced::raw::Ecv::detect() {
        measure("cntvctss", ns_per_tick, || ecv.read());
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn counter_reads(_ns_per_tick: f64) {}

fn main() {
    let seconds: u64 = std::env::args().nth(1).map_or(60, |s| s.parse().expect("seconds"));
    let facts = HostFacts::detect();
    println!("host,invariant_tsc,{}", facts.invariant_tsc.unwrap_or(false));
    if let Some(hz) = facts.register_hz {
        println!("host,counter_hz,{hz}");
        println!("host,ecv,{}", facts.ecv.unwrap_or(false));
    }
    println!("host,clocksource,{}", facts.clocksource.as_deref().unwrap_or(""));

    let origin = Instant::now();
    measure("clock_gettime_monotonic", 1.0, || origin.elapsed().as_nanos() as u64);
    measure("aeron_nano_clock", 1.0, || rusteron_client::Aeron::nano_clock() as u64);
    let clock = Clock::calibrate();
    if clock.is_counter() {
        let (_, _, scaled, rate) = NAMES;
        // ticks to ns through the calibrated factor
        let ns_per_tick = clock.inner().ticks_to_ns(1 << 32) as f64 / (1u64 << 32) as f64;
        println!("host,{rate},{:.4}", 1.0 / ns_per_tick);
        counter_reads(ns_per_tick);
        measure(scaled, 1.0, || clock.stamp());
        measure(&format!("{scaled}_unordered"), 1.0, || clock.now());
        let anchor = minstant::Instant::now();
        measure("minstant", 1.0, || anchor.elapsed().as_nanos() as u64);

        // drift of the clients' clock against CLOCK_MONOTONIC
        let raw_ns = ergon_clock::advanced::monotonic_raw_ns;
        let (t0, m0, i0, r0) = (clock.now(), minstant::Instant::now(), Instant::now(), raw_ns());
        for checkpoint in [1u64, 10, 30, 60, 120, 300, 600].into_iter().filter(|&c| c <= seconds) {
            while i0.elapsed() < Duration::from_secs(checkpoint) {
                std::thread::sleep(Duration::from_millis(1));
            }
            let (t, m, i, r) = (clock.now(), minstant::Instant::now(), Instant::now(), raw_ns());
            let counter_ns = (t - t0) as f64;
            let minstant_ns = (m - m0).as_nanos() as f64;
            let mono_ns = (i - i0).as_nanos() as f64;
            let raw = (r - r0) as f64;
            println!(
                "drift,{checkpoint},{counter_ns:.0},{mono_ns:.0},{:.3},minstant_ppm,{:.3},raw_ppm,{:.3},mono_vs_raw_ppm,{:.3}",
                (counter_ns - mono_ns) / mono_ns * 1e6,
                (minstant_ns - mono_ns) / mono_ns * 1e6,
                (counter_ns - raw) / raw * 1e6,
                (mono_ns - raw) / raw * 1e6
            );
        }
    } else {
        println!("host,tsc,unavailable");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ergon_clock::advanced::{HostFacts, Trust};

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

    /// The clients' clock is ergon-clock's counter wherever ergon-clock
    /// lets the counter be the clock on calibration's own evidence
    /// (`Trust::CrossCheck`): every lab host, as the harness's own clock
    /// was (an invariant TSC with `rdtscp` under `tsc` or `kvm-clock`;
    /// Graviton's generic timer), and this Mac's counter too, which the
    /// kernel-trust default refuses (no clock source to vouch for it), so
    /// the counter path is what a smoke test here exercises.
    #[test]
    fn the_clock_is_the_counter_wherever_ergon_clock_cross_checks_it() {
        let clock = Clock::calibrate();
        let forced = std::env::var("BENCH_CLOCK").is_ok_and(|c| c == "monotonic");
        let gate = HostFacts::detect().gate(Trust::CrossCheck);
        assert_eq!(clock.is_counter(), !forced && gate.is_ok(), "the gate: {gate:?}");
        if cfg!(target_arch = "aarch64") && !forced {
            assert!(clock.is_counter(), "aarch64: the generic timer's counter");
        }
    }

    /// A latency stamp pair, the ordered read at both ends, measures a
    /// known sleep in nanoseconds: at least the sleep (the kernel sleeps no
    /// less on its monotonic clock), at most the `Instant` bracket around
    /// the pair, each within the calibration's rate error (0.1%) and a
    /// microsecond. Raw ticks, or a clock that is not counting
    /// nanoseconds, fall outside one bound or the other.
    #[test]
    fn a_stamp_pair_measures_a_known_sleep() {
        let clock = Clock::calibrate();
        for sleep_ms in [1u64, 20, 100] {
            let sleep_ns = sleep_ms * 1_000_000;
            let i0 = Instant::now();
            let s0 = clock.stamp();
            std::thread::sleep(Duration::from_millis(sleep_ms));
            let s1 = clock.stamp();
            let outer = u64::try_from(i0.elapsed().as_nanos()).unwrap();
            let measured = s1.wrapping_sub(s0);
            assert!(
                measured + sleep_ns / 1000 + 1_000 >= sleep_ns,
                "a {sleep_ms} ms sleep measured {measured} ns: less than the sleep"
            );
            assert!(
                measured <= outer + outer / 1000 + 1_000,
                "a {sleep_ms} ms sleep measured {measured} ns: more than the {outer} ns bracket around it"
            );
        }
    }

    /// The unordered read (pacing, deadlines, loop gaps) counts nanoseconds
    /// that track `CLOCK_MONOTONIC` within 0.1% over 50 ms.
    #[test]
    fn the_clock_tracks_real_time() {
        let clock = Clock::calibrate();
        let (i0, c0) = sample(&clock);
        while i0.elapsed() < Duration::from_millis(50) {
            std::hint::spin_loop();
        }
        let (i1, c1) = sample(&clock);
        let mono = u64::try_from((i1 - i0).as_nanos()).unwrap();
        let err = (c1 - c0).abs_diff(mono);
        assert!(
            err * 1000 <= mono,
            "clock advanced {} ns against {mono} ns monotonic",
            c1 - c0
        );
    }
}
