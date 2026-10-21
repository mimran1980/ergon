//! Calibration on this host: what it chooses, why, and that the clock it
//! returns keeps time. Every assertion holds on any host (a developer's Mac,
//! a CI VM whose kernel keeps time with Hyper-V's clock, a Graviton): what
//! differs by host is read from the host and asserted conditionally, never
//! skipped.
#![allow(missing_docs, clippy::cast_precision_loss)]

use ergon_clock::advanced::{
    Arch, Calibrated, Clock, CounterClock, Fallback, HostFacts, Kind, OrderedRead, Policy,
    RateSource, Trust, calibrate, monotonic_ns, monotonic_raw_ns, raw, realtime_ns,
};
use ergon_clock::{Config, Domain, Error, TimeSource};
use std::time::Duration;

type R = Result<(), Box<dyn std::error::Error>>;

fn config(trust: Trust) -> Config {
    Config {
        trust,
        ..Config::default()
    }
}

/// The fallbacks calibration may land on where the gate let the counter
/// through: what it measures of the counter itself.
const fn measured_fallback(f: &Fallback) -> bool {
    matches!(
        f,
        Fallback::Trapped { .. }
            | Fallback::Unstable { .. }
            | Fallback::CounterStalled
            | Fallback::RateOutOfRange { .. }
    )
}

/// The host's own facts decide: where the gate refuses the counter the
/// clock is the monotonic fallback for exactly the gate's reason; where it
/// lets it through, the counter, unless measuring it found it unusable.
#[test]
fn calibration_follows_the_gate() -> R {
    let facts = HostFacts::detect();
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        let Calibrated { clock, report } = calibrate(&config(trust))?;
        match facts.gate(trust) {
            Err(why) => {
                assert_eq!(report.kind, Kind::Monotonic, "{trust:?}: {report}");
                assert_eq!(report.fallback, Some(why), "{trust:?}: {report}");
                assert!(!clock.is_counter());
            }
            Ok(()) => match &report.fallback {
                None => {
                    assert_eq!(report.kind, Kind::Counter, "{trust:?}: {report}");
                    assert!(clock.is_counter());
                }
                Some(f) => assert!(
                    measured_fallback(f),
                    "{trust:?}: the gate passed, yet {f:?}: {report}"
                ),
            },
        }
    }
    Ok(())
}

/// Every Arm host has the generic timer's counter, and a cross-check
/// accepts it wherever the kernel does not say otherwise: a Mac's too.
#[cfg(target_arch = "aarch64")]
#[test]
fn every_arm_host_gets_the_counter_with_a_cross_check() -> R {
    let report = calibrate(&config(Trust::CrossCheck))?.report;
    assert_eq!(report.kind, Kind::Counter, "{report}");
    Ok(())
}

/// On Arm the counter's rate is its frequency register, `CNTFRQ_EL0`, the
/// rate the kernel's own clocks scale the same counter by, once a
/// measurement against `CLOCK_MONOTONIC_RAW` cannot tell them apart: so a
/// second of ticks is a second to the bit, and two processes convert the
/// same tick to the same nanosecond rate.
#[cfg(target_arch = "aarch64")]
#[test]
fn the_arm_rate_is_the_frequency_register() -> R {
    let Calibrated { clock, report } = calibrate(&config(Trust::CrossCheck))?;
    let hz = ergon_clock::advanced::raw::cntfrq();
    assert_eq!(report.rate_source, RateSource::Register, "{report}");
    assert_eq!(clock.hz(), hz);
    let want = u64::try_from(((1_000_000_000_u128 << 32) + u128::from(hz) / 2) / u128::from(hz))?;
    assert_eq!(clock.params().mult, want, "{report}");
    Ok(())
}

/// `Policy::Monotonic`: the fallback, said to be forced.
#[test]
fn the_fallback_can_be_forced() -> R {
    let Calibrated { clock, report } = calibrate(&Config {
        policy: Policy::Monotonic,
        trust: Trust::CrossCheck,
        ..Config::default()
    })?;
    assert!(!clock.is_counter());
    assert_eq!(report.kind, Kind::Monotonic);
    assert_eq!(report.fallback, Some(Fallback::Forced));
    Ok(())
}

/// `Policy::RequireCounter`, for a process that must not run on the slow
/// clock: an error naming why exactly where Auto falls back, the counter
/// otherwise.
#[test]
fn a_required_counter_fails_where_auto_falls_back() -> R {
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        let auto = calibrate(&config(trust))?.report;
        let required = calibrate(&Config {
            policy: Policy::RequireCounter,
            trust,
            ..Config::default()
        });
        match (auto.fallback, required) {
            (None, Ok(c)) => assert!(c.clock.is_counter()),
            (Some(why), Err(Error::CounterUnavailable(got))) => {
                // a measured fallback may differ between two calibrations
                assert!(
                    got == why || (measured_fallback(&got) && measured_fallback(&why)),
                    "{why:?} then {got:?}"
                );
            }
            (auto, required) => {
                return Err(format!("{trust:?}: auto {auto:?}, required {required:?}").into());
            }
        }
    }
    Ok(())
}

/// (raw ns at the middle of the tightest of 16 brackets, the clock's ns)
fn sample(clock: &impl TimeSource) -> (u64, u64) {
    let mut best = (u64::MAX, 0, 0);
    for _ in 0..16 {
        let r0 = monotonic_raw_ns();
        let ns = clock.now_ns_ordered();
        let r1 = monotonic_raw_ns();
        if r1 - r0 < best.0 {
            best = (r1 - r0, r0 + (r1 - r0) / 2, ns);
        }
    }
    (best.1, best.2)
}

/// How far the clock ran from `CLOCK_MONOTONIC_RAW` over `span`, in ppm.
fn drift_ppm(clock: &impl TimeSource, span: Duration) -> f64 {
    let (r0, c0) = sample(clock);
    while monotonic_raw_ns() - r0 < u64::try_from(span.as_nanos()).unwrap_or(u64::MAX) {
        std::hint::spin_loop();
    }
    let (r1, c1) = sample(clock);
    let raw = (r1 - r0) as f64;
    (c1.wrapping_sub(c0) as f64 - raw) / raw * 1e6
}

/// The counter clock keeps `CLOCK_MONOTONIC_RAW`'s rate, which it was
/// calibrated against (within 200 ppm over 50 ms: bracket noise, a 24 MHz
/// counter's 42 ns tick); the fallback keeps `CLOCK_MONOTONIC`'s, which NTP
/// slews by up to a few thousand ppm after a boot (within 1%).
#[test]
fn the_clock_keeps_time() -> R {
    for policy in [Policy::Auto, Policy::Monotonic] {
        let Calibrated { clock, report } = calibrate(&Config {
            policy,
            trust: Trust::CrossCheck,
            ..Config::default()
        })?;
        let ppm = drift_ppm(&clock, Duration::from_millis(50));
        let limit = if clock.is_counter() { 200.0 } else { 10_000.0 };
        assert!(
            ppm.abs() <= limit,
            "{policy:?}: {ppm:.1} ppm against CLOCK_MONOTONIC_RAW: {report}"
        );
    }
    Ok(())
}

/// Each domain starts at its kernel clock: the monotonic domain at
/// `CLOCK_MONOTONIC` (the platform's `Instant` clock), the realtime one at
/// `CLOCK_REALTIME` (epoch nanoseconds), within a millisecond (macOS's
/// realtime clock has microsecond resolution).
#[test]
fn each_domain_starts_at_its_kernel_clock() -> R {
    for policy in [Policy::Auto, Policy::Monotonic] {
        for domain in [Domain::Monotonic, Domain::Realtime] {
            let clock = calibrate(&Config {
                policy,
                domain,
                trust: Trust::CrossCheck,
                ..Config::default()
            })?
            .clock;
            assert_eq!(clock.domain(), domain);
            let (kernel, ours) = match domain {
                Domain::Monotonic => (i128::from(monotonic_ns()), i128::from(clock.now_ns())),
                Domain::Realtime => (i128::from(realtime_ns()), i128::from(clock.now_ns())),
            };
            assert!(
                (kernel - ours).abs() < 1_000_000,
                "{policy:?} {domain:?}: kernel {kernel}, clock {ours}"
            );
        }
    }
    Ok(())
}

// ANCHOR: report
/// The report says what was chosen and how: the counter's rate and its
/// source, the read and the reference it was calibrated against, and the
/// fallback's reason or none.
#[test]
fn the_report_describes_the_choice() -> R {
    for policy in [Policy::Auto, Policy::Monotonic] {
        let Calibrated { clock, report } = calibrate(&Config {
            policy,
            trust: Trust::CrossCheck,
            ..Config::default()
        })?;
        assert_eq!(report.hz, clock.hz());
        assert!(report.calibration > Duration::ZERO);
        let line = report.to_string();
        for key in [
            "kind=",
            "hz=",
            "rate_source=",
            "reference=",
            "clocksource=",
            "fallback=",
        ] {
            assert!(line.contains(key), "no {key} in {line}");
        }
        if clock.is_counter() {
            assert_eq!(report.fallback, None);
            assert_ne!(report.rate_source, RateSource::None);
            let counter = if cfg!(target_arch = "aarch64") {
                "cntvct_el0"
            } else {
                "rdtsc"
            };
            assert_eq!(report.counter, counter);
            assert!(
                report.uncertainty_ppm.is_some_and(|u| u > 0.0 && u < 100.0),
                "{report}"
            );
        } else {
            assert_eq!(report.hz, 1_000_000_000);
            assert_eq!(report.rate_source, RateSource::None);
            assert!(report.fallback.is_some());
        }
    }
    Ok(())
}
// ANCHOR_END: report

/// The ordered read (a measurement bracket's) never comes back lower than
/// the one before it on one thread.
#[test]
fn ordered_reads_never_go_backwards_on_one_thread() -> R {
    for policy in [Policy::Auto, Policy::Monotonic] {
        let clock = calibrate(&Config {
            policy,
            trust: Trust::CrossCheck,
            ..Config::default()
        })?
        .clock;
        let mut last = clock.ticks_ordered();
        for _ in 0..200_000 {
            let now = clock.ticks_ordered();
            assert!(now >= last, "{policy:?}: {now} after {last}");
            last = now;
        }
    }
    Ok(())
}

/// One calibration serves every thread: the clock is a `Copy` value with
/// no thread-local state, and a read on another thread after a handoff is
/// not earlier than the read before it (by more than the counters' skew
/// between cores, a microsecond here).
#[test]
fn one_calibration_serves_every_thread() -> R {
    fn send_sync<T: Send + Sync + Copy>() {}
    send_sync::<Clock>();
    let clock = calibrate(&config(Trust::CrossCheck))?.clock;
    for _ in 0..100 {
        let before = clock.now_ns_ordered();
        let after = std::thread::spawn(move || clock.now_ns_ordered())
            .join()
            .map_err(|_| "the reading thread failed")?;
        assert!(
            after + 1_000 >= before,
            "{after} on another thread after {before}"
        );
    }
    Ok(())
}

/// The calibration runs in tens of milliseconds, never the 200 ms deadline
/// quanta's can hit: a 50 ms measurement and a 20 ms verification, plus
/// reads (under a second, allowing for a loaded machine).
#[test]
fn calibration_takes_tens_of_milliseconds() -> R {
    let report = calibrate(&config(Trust::CrossCheck))?.report;
    assert!(report.calibration < Duration::from_secs(1), "{report}");
    if report.kind == Kind::Counter {
        assert!(report.calibration >= Duration::from_millis(20), "{report}");
    }
    Ok(())
}

/// The architecture detected is the one compiled for.
#[test]
fn the_facts_are_this_hosts() {
    let facts = HostFacts::detect();
    let arch = if cfg!(target_arch = "x86_64") {
        Arch::X86_64
    } else if cfg!(target_arch = "aarch64") {
        Arch::Aarch64
    } else {
        Arch::Other
    };
    assert_eq!(facts.arch, arch);
    assert_eq!(
        facts.register_hz.is_some(),
        cfg!(target_arch = "aarch64"),
        "{facts:?}"
    );
    assert_eq!(
        facts.invariant_tsc.is_some(),
        cfg!(target_arch = "x86_64"),
        "{facts:?}"
    );
    assert_eq!(
        facts.clocksource.is_some(),
        std::path::Path::new(ergon_clock::advanced::CLOCKSOURCE).exists()
    );
}

/// The counter clock's ordered read is the one this host's facts choose,
/// fixed in the clock at calibration and named in the report; a clock
/// adopted from the calibration's parameters chooses it again for itself.
/// On Linux Arm, `CNTVCTSS_EL0` exactly where the kernel lists `ecv`.
#[test]
fn the_clock_takes_the_hosts_ordered_read() -> R {
    let facts = HostFacts::detect();
    let chosen = facts.ordered_read();
    let Calibrated { clock, report } = calibrate(&config(Trust::CrossCheck))?;
    assert_eq!(report.ordered_read, chosen.name(), "{report}");
    match clock {
        Clock::Counter(counter) => {
            assert_eq!(counter.ordered_read(), chosen, "{report}");
            let adopted = CounterClock::from_params(&counter.params())?;
            assert_eq!(adopted.ordered_read(), chosen);
        }
        Clock::Monotonic(_) => assert!(report.fallback.is_some(), "{report}"),
    }
    let expected = if cfg!(target_arch = "x86_64") {
        OrderedRead::Rdtscp
    } else if cfg!(target_arch = "aarch64") {
        if facts.ecv == Some(true) {
            OrderedRead::Cntvctss
        } else {
            OrderedRead::IsbCntvct
        }
    } else {
        OrderedRead::None
    };
    assert_eq!(chosen, expected, "{facts:?}");
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    if let Some(listed) = std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|c| {
        c.lines()
            .find(|l| l.starts_with("Features"))
            .map(|l| l.split_whitespace().any(|f| f == "ecv"))
    }) {
        assert_eq!(chosen == OrderedRead::Cntvctss, listed, "{facts:?}");
    }
    Ok(())
}

/// The clock's ordered read, whichever instruction it is, is ordered with
/// the bare ordered reads around it on one thread: never lower than one
/// issued before it, never higher than one issued after it.
#[test]
fn the_ordered_read_is_ordered_with_the_bare_one() -> R {
    let Calibrated { clock, report } = calibrate(&config(Trust::CrossCheck))?;
    match clock {
        Clock::Counter(counter) => {
            // the bare ordered read: `rdtscp` (x86-64, which has it wherever
            // the counter is the clock) or `isb; mrs cntvct_el0` (Arm)
            #[cfg(target_arch = "x86_64")]
            let bare = {
                let rdtscp = raw::Rdtscp::detect().ok_or("the counter, yet no rdtscp")?;
                move || rdtscp.read()
            };
            #[cfg(target_arch = "aarch64")]
            let bare = raw::isb_cntvct;
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            let bare = || 0_u64;
            for _ in 0..200_000 {
                let a = bare();
                let b = counter.ticks_ordered();
                let c = bare();
                assert!(
                    a <= b && b <= c,
                    "{a} {b} {c}: {}",
                    counter.ordered_read().name()
                );
            }
        }
        Clock::Monotonic(_) => assert!(report.fallback.is_some(), "{report}"),
    }
    Ok(())
}

/// One cycle through `n` slots in a random order (Sattolo's shuffle, a
/// fixed seed): chased from any slot, each hop a load that depends on the
/// last and, over 32 MB, misses the caches.
fn ring(n: u32) -> Vec<u32> {
    let mut ring: Vec<u32> = (0..n).collect();
    let mut x = 0x9e37_79b9_7f4a_7c15_u64;
    for i in (1..n).rev() {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let j = x.wrapping_mul(0x2545_f491_4f6c_dd1d) % u64::from(i);
        ring.swap(i as usize, usize::try_from(j).unwrap_or(0));
    }
    ring
}

/// 16 dependent cache-missing loads from a slot `seed` picks (so not
/// before the read that gave it), ending in `black_box` (so in program
/// order before the read after it): a microsecond or two of work, all of
/// it in flight at once in an out-of-order core's reorder buffer. A plain
/// index and a `while` loop, so that a debug build, which calls a function
/// for each `get` or range step, still keeps the loop to a few
/// instructions a hop and the buffer full of loads.
fn chase(ring: &[u32], seed: u64) -> usize {
    let len = u64::try_from(ring.len()).unwrap_or(u64::MAX).max(1);
    let mut i = usize::try_from(seed % len).unwrap_or(0);
    let mut hops = 0;
    while hops < 16 {
        i = ring[i] as usize;
        hops += 1;
    }
    std::hint::black_box(i)
}

/// The ordered read waits for earlier work to finish. Across the same
/// cache-missing work, from a bare ordered read to the clock's ordered read
/// takes as long as between two bare ordered reads: the medians of 1,000
/// tries each, interleaved, within 5%, 20 ns and a tick. An unordered read
/// is taken while the loads are still in flight: on an M1 Pro, about 65%
/// short in a debug build (13 of 37 ticks) and at once (0 of 45) in a
/// release one.
#[test]
fn the_ordered_read_waits_for_earlier_work() -> R {
    let Calibrated { clock, report } = calibrate(&config(Trust::CrossCheck))?;
    match clock {
        Clock::Counter(counter) => {
            #[cfg(target_arch = "x86_64")]
            let bare = {
                let rdtscp = raw::Rdtscp::detect().ok_or("the counter, yet no rdtscp")?;
                move || rdtscp.read()
            };
            #[cfg(target_arch = "aarch64")]
            let bare = raw::isb_cntvct;
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            let bare = || 0_u64;
            let ring = ring(1 << 23);
            let (mut between_bare, mut to_ours) = (Vec::new(), Vec::new());
            for _ in 0..1_000 {
                let a = bare();
                chase(&ring, a);
                between_bare.push(bare().saturating_sub(a));
                let a = bare();
                chase(&ring, a);
                to_ours.push(counter.ticks_ordered().saturating_sub(a));
            }
            let median = |mut v: Vec<u64>| {
                v.sort_unstable();
                v.get(v.len() / 2).copied().unwrap_or(0)
            };
            let (bare_ticks, ours) = (median(between_bare), median(to_ours));
            let tolerance = bare_ticks / 20 + counter.ns_to_ticks(20) + 1;
            assert!(
                ours + tolerance >= bare_ticks,
                "{}: a median {ours} ticks across the work, {bare_ticks} between bare ordered reads",
                counter.ordered_read().name()
            );
        }
        Clock::Monotonic(_) => assert!(report.fallback.is_some(), "{report}"),
    }
    Ok(())
}

// ANCHOR: recheck
/// Re-read after calibration, the kernel's clock source still vouches for
/// the clock calibration chose (nothing changed it in between), at the
/// trust calibration was given: the cold-path recheck a long-running
/// process makes from its housekeeping.
#[test]
fn the_recheck_passes_while_the_kernel_is_unchanged() -> R {
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        let report = calibrate(&config(trust))?.report;
        assert_eq!(report.trust, trust);
        assert_eq!(report.recheck(), Ok(()), "{trust:?}: {report}");
    }
    Ok(())
}
// ANCHOR_END: recheck

/// A clock is always there to hold: the fallback in either domain, with no
/// `Option` and no `Result` for its holder (and so no extra compare on its
/// reads), and the error, where there was one, beside it. Here, as on every
/// supported platform, the kernel's clocks read, so there is none, and the
/// clock is `MonotonicClock::new`'s and keeps time in its domain.
#[test]
fn a_fallback_clock_is_always_there_with_its_error_beside_it() {
    for domain in [Domain::Monotonic, Domain::Realtime] {
        let (clock, error) = Clock::fallback(domain);
        assert_eq!(error, None, "{domain:?}");
        assert!(!clock.is_counter());
        assert_eq!(clock.domain(), domain);
        let (monotonic, error) = ergon_clock::advanced::MonotonicClock::always(domain);
        assert_eq!(error, None);
        assert_eq!(monotonic.domain(), domain);
        let (kernel, ours) = match domain {
            Domain::Monotonic => (i128::from(monotonic_ns()), i128::from(clock.now_ns())),
            Domain::Realtime => (i128::from(realtime_ns()), i128::from(clock.now_ns())),
        };
        assert!(
            (kernel - ours).abs() < 1_000_000,
            "{domain:?}: kernel {kernel}, clock {ours}"
        );
    }
}
