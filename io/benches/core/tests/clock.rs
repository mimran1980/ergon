//! The counter clock: raw reads scale to the reference clock's nanoseconds,
//! and two CPUs' counters agree.

use ergon_latency_core::clock::{TscClock, cross_cpu_check};
use std::time::{Duration, Instant};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_raw_delta_scales_to_the_monotonic_clocks_nanoseconds() -> R {
    let clock = TscClock::new();
    let (t0, r0) = (Instant::now(), clock.raw());
    std::thread::sleep(Duration::from_millis(20));
    let (r1, t1) = (clock.raw(), Instant::now());
    let ns = clock.delta_ns(r0, r1);
    let reference = u64::try_from(t1.duration_since(t0).as_nanos())?;
    // r0..r1 lies inside t0..t1; reading either clock costs well under 50 us
    assert!(ns <= reference, "{ns} > {reference}");
    assert!(ns + 50_000 >= reference, "{ns} vs {reference}");
    Ok(())
}

#[test]
fn a_backward_delta_is_zero_not_a_wrap() {
    let clock = TscClock::new();
    let r = clock.raw();
    assert_eq!(clock.delta_ns(r + 1000, r), 0);
}

#[test]
fn nanoseconds_convert_to_ticks_and_back_within_a_thousandth() {
    let clock = TscClock::new();
    assert!(clock.ticks_per_ns() > 0.0);
    for ns in [100_000u64, 1_000_000, 1_000_000_000] {
        let ticks = clock.ns_to_ticks(ns);
        let back = clock.delta_ns(1, 1 + ticks);
        assert!(
            back.abs_diff(ns) * 1000 <= ns,
            "{ns} ns -> {ticks} ticks -> {back} ns"
        );
    }
}

/// Two threads hand a stamp back and forth: the reader's counter is never
/// behind the writer's. Unpinned here (the pin hook does nothing); the
/// suite's hostcheck runs it pinned to each hot CPU.
#[test]
fn the_counter_never_runs_backward_across_threads() -> R {
    let clock = TscClock::new();
    let report = cross_cpu_check(&clock, 0, 1, 10_000, |_| {})?;
    assert_eq!(report.cpus, (0, 1));
    assert_eq!(report.rounds, 20_000);
    assert!(report.consistent(), "{report:?}");
    assert_eq!(report.backward, 0);
    assert_eq!(report.max_backward_ns, 0);
    Ok(())
}

#[test]
fn the_pin_hook_runs_once_on_each_side() -> R {
    let clock = TscClock::new();
    let pinned = std::sync::Mutex::new(Vec::new());
    cross_cpu_check(&clock, 3, 5, 10, |cpu| {
        if let Ok(mut p) = pinned.lock() {
            p.push(cpu);
        }
    })?;
    let mut pinned = pinned.into_inner()?;
    pinned.sort_unstable();
    assert_eq!(pinned, [3, 5]);
    Ok(())
}
