//! The transport's clock: ergon-clock's calibration, made once in Rust and
//! handed to the native transport in its configuration, whose hot path reads
//! the counter inline and converts it with ergon-clock's arithmetic.
//!
//! The choice (`RUSTERON_DPDK_CLOCK`) and the calibration run on every
//! platform. With the `dpdk` feature (Linux x86_64 or aarch64, against the
//! DPDK-free fakes) the native side is checked against ergon-clock: the
//! conversion bit for bit, the reads within a bracket of ergon-clock's, the
//! fallback, the configuration's checks and the test pin.
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]

mod common;

use ergon_clock::Domain;
use ergon_clock::advanced::{Fallback, KIND_MONOTONIC, Policy, TimeSyncPolicy, Trust};
use ergon_md_dpdk::DpdkError;
use ergon_md_dpdk::clock::{CLOCK_ENV, ClockChoice, clock_config, transport_clock};
use serial_test::serial;

// --- The choice and the calibration (every platform) ---------------------------

/// `RUSTERON_DPDK_CLOCK`: the counter when unset or empty, `counter`, or
/// `tsc` (its name before aarch64, on either architecture); the fallback for
/// `monotonic`; anything else refuses, naming the variable.
#[test]
#[serial]
fn the_clock_choice_is_read_from_the_environment() {
    let _unset = common::EnvGuard::unset(CLOCK_ENV);
    assert_eq!(ClockChoice::from_env(), Ok(ClockChoice::Counter));
    for (value, choice) in [
        ("", ClockChoice::Counter),
        ("counter", ClockChoice::Counter),
        ("tsc", ClockChoice::Counter),
        ("monotonic", ClockChoice::Monotonic),
    ] {
        let _v = common::EnvGuard::set(CLOCK_ENV, value);
        assert_eq!(ClockChoice::from_env(), Ok(choice), "{value:?}");
    }
    for bad in ["hpet", "TSC", "Monotonic", " counter", "cntvct"] {
        let _v = common::EnvGuard::set(CLOCK_ENV, bad);
        let e = ClockChoice::from_env().unwrap_err();
        assert!(matches!(e, DpdkError::InvalidEnvironment(_)), "{bad:?}: {e:?}");
        let text = e.to_string();
        assert!(text.contains("RUSTERON_DPDK_CLOCK") && text.contains(bad), "{text}");
    }
}

/// The transport's clock is calibrated for intervals: in the monotonic
/// domain, the counter trusted on calibration's own measurement and
/// verification (as the transport's own calibration did: a paravirtual
/// clock source keeps the counter), the wall clock's discipline not probed;
/// `monotonic` forces ergon-clock's fallback.
#[test]
fn the_clock_is_calibrated_for_intervals_in_the_monotonic_domain() {
    for (choice, policy) in [
        (ClockChoice::Counter, Policy::Auto),
        (ClockChoice::Monotonic, Policy::Monotonic),
    ] {
        let config = clock_config(choice);
        assert_eq!(config.policy, policy, "{choice:?}");
        assert_eq!(config.trust, Trust::CrossCheck, "{choice:?}");
        assert_eq!(config.domain, Domain::Monotonic, "{choice:?}");
        assert_eq!(config.time_sync, TimeSyncPolicy::Off, "{choice:?}");
    }
    assert_eq!(ClockChoice::default(), ClockChoice::Counter);
}

/// The counter is calibrated once a process: every transport adopts the
/// same parameters, ergon-clock's own choice for this host.
#[test]
#[serial]
fn the_counter_is_calibrated_once_a_process() {
    let first = transport_clock(ClockChoice::Counter).unwrap();
    let again = transport_clock(ClockChoice::Counter).unwrap();
    assert_eq!(first.clock.params(), again.clock.params(), "one calibration");
    assert_eq!(first.report, again.report);
    assert_eq!(first.clock.domain(), Domain::Monotonic);
    let fresh = ergon_clock::advanced::calibrate(&clock_config(ClockChoice::Counter)).unwrap();
    assert_eq!(
        fresh.clock.is_counter(),
        first.clock.is_counter(),
        "ergon-clock's choice: {}",
        fresh.report
    );
    let (a, b) = (first.clock.now_ns(), first.clock.now_ns());
    assert!(b >= a, "{a} then {b}");
}

/// `monotonic` is ergon-clock's fallback, forced: `CLOCK_MONOTONIC`'s
/// nanoseconds through a factor of one, from no offset.
#[test]
#[serial]
fn the_monotonic_choice_is_ergon_clocks_fallback() {
    let m = transport_clock(ClockChoice::Monotonic).unwrap();
    assert!(!m.clock.is_counter());
    assert_eq!(m.report.fallback, Some(Fallback::Forced));
    let p = m.clock.params();
    assert_eq!(
        (p.kind, p.domain, p.base_ticks, p.base_ns, p.mult),
        (KIND_MONOTONIC, 0, 0, 0, 1 << 32),
        "{p:?}"
    );
}

// --- The native transport's clock (`dpdk`) ------------------------------------

#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod native {
    use super::common::*;
    use ergon_clock::Domain;
    use ergon_clock::advanced::{Clock, KIND_COUNTER, KIND_MONOTONIC, MonotonicClock, PARAMS_VERSION, Params, Scale};
    use ergon_md_dpdk::clock::{ClockChoice, transport_clock};
    use serial_test::serial;
    use std::os::raw::c_int;
    use std::time::{Duration, Instant};

    unsafe extern "C" {
        /// The adopted conversion applied to `ticks`: what the hot path
        /// computes from a counter read (or, for the fallback, from a read
        /// of `CLOCK_MONOTONIC`).
        fn rusteron_dpdk_test_clock_ns_at(ticks: u64) -> u64;
    }

    /// The transport clock's source as counter type 80 reports it: 1 the
    /// TSC, 2 `CNTVCT_EL0`, 0 `CLOCK_MONOTONIC`.
    fn source_of(clock: &Clock) -> c_int {
        match (clock.is_counter(), cfg!(target_arch = "aarch64")) {
            (false, _) => 0,
            (true, false) => 1,
            (true, true) => 2,
        }
    }

    /// A transport created with `params` as its clock, closed again: the
    /// calibration stays adopted (it is process-lifetime).
    fn adopt(params: Params) {
        let env = TestEnv::new();
        env.eal_skip();
        let mut config = rusteron_dpdk_config_t::valid();
        config.clock = params;
        let t = create(&config).unwrap_or_else(|e| panic!("create with {params:?}: {e}"));
        assert_eq!(close(t), 0);
    }

    /// Native reads bracketed by ergon-clock's: each within `slack_ns` of
    /// the reads either side of it (both unordered, on one thread).
    fn assert_bracketed(clock: &Clock, slack_ns: u64) {
        for i in 0..10_000 {
            let before = clock.now_ns();
            let native = unsafe { rusteron_dpdk_clock_ns() };
            let after = clock.now_ns();
            assert!(
                native + slack_ns >= before && native <= after + slack_ns,
                "read {i}: native {native} outside ergon-clock's {before}..{after}"
            );
        }
    }

    /// Over a 50 ms busy wait the native clock advances as ergon-clock's does
    /// and as `Instant` (`CLOCK_MONOTONIC`) does, within 0.1%.
    fn assert_tracks(clock: &Clock) {
        let (n0, c0, i0) = (unsafe { rusteron_dpdk_clock_ns() }, clock.now_ns(), Instant::now());
        while i0.elapsed() < Duration::from_millis(50) {
            std::hint::spin_loop();
        }
        let (n1, c1, i1) = (unsafe { rusteron_dpdk_clock_ns() }, clock.now_ns(), Instant::now());
        let (native, ergon) = (n1 - n0, c1 - c0);
        let mono = u64::try_from((i1 - i0).as_nanos()).unwrap();
        assert!(
            native.abs_diff(ergon) <= 20_000,
            "native advanced {native} ns, ergon-clock {ergon} ns"
        );
        assert!(
            native.abs_diff(mono) * 1000 <= mono,
            "native advanced {native} ns against {mono} ns monotonic (more than 0.1% apart)"
        );
    }

    /// A transport adopts the clock its configuration carries: the clock
    /// counter (type 80) reports ergon-clock's choice, and every read is
    /// ergon-clock's time, within a bracket of its reads.
    #[test]
    #[serial]
    fn a_transport_reads_the_clock_its_configuration_carries() {
        let calibrated = transport_clock(ClockChoice::Counter).unwrap();
        adopt(calibrated.clock.params());
        assert_eq!(
            unsafe { rusteron_dpdk_clock_source() },
            source_of(&calibrated.clock),
            "ergon-clock chose: {}",
            calibrated.report
        );
        assert_bracketed(&calibrated.clock, 1_000);
        assert_tracks(&calibrated.clock);
    }

    /// The counter path: the transport reports the counter and every read is
    /// ergon-clock's counter clock's time, within a bracket of its reads.
    #[test]
    #[serial]
    fn the_counter_path_reads_ergon_clocks_counter_time() {
        let counter = counter_clock();
        adopt(counter.params());
        assert_eq!(unsafe { rusteron_dpdk_clock_source() }, source_of(&counter));
        assert_ne!(unsafe { rusteron_dpdk_clock_source() }, 0, "the counter");
        assert_bracketed(&counter, 1_000);
        adopt(transport_clock(ClockChoice::Counter).unwrap().clock.params());
    }

    /// The native conversion is ergon-clock's to the bit, for the counter's
    /// parameters and the fallback's: a tick behind the base (an unordered
    /// read on another core, just after calibration) is a little before the
    /// base's time, never 2^64 ns later.
    #[test]
    #[serial]
    fn the_native_conversion_is_ergon_clocks_to_the_bit() {
        let calibrated = transport_clock(ClockChoice::Counter).unwrap().clock.params();
        let base = 1u64 << 40;
        let synthetic = [
            Scale::from_hz(2_400_000_000, base, 5_000_000_000).unwrap(),
            Scale::from_hz(24_000_000, base, 0).unwrap(),
            Scale::from_hz(1_000_000_000, 0, 7).unwrap(),
            Scale::from_mult((1 << 32) + 12_345, base, 1 << 50).unwrap(),
        ];
        let mut cases: Vec<Params> = synthetic
            .iter()
            .map(|s| Params {
                version: PARAMS_VERSION,
                kind: KIND_COUNTER,
                domain: 0,
                reserved: 0,
                base_ticks: s.base_ticks(),
                base_ns: s.base_ns(),
                mult: s.mult(),
                hz: s.hz(),
            })
            .collect();
        cases.push(calibrated);
        // the fallback, as made and retuned (the formula with the monotonic
        // clock as the ticks)
        let (mono, _) = MonotonicClock::always(Domain::Monotonic);
        cases.push(Params {
            kind: KIND_MONOTONIC,
            ..mono.params()
        });
        let retuned = mono.retuned(1_000_000_000, (1 << 32) - 4_295, 5).unwrap();
        cases.push(retuned.params());

        for p in cases {
            adopt(p);
            let reference = Scale::from_mult(p.mult, p.base_ticks, p.base_ns).unwrap();
            let mut ticks: Vec<u64> = vec![
                p.base_ticks,
                p.base_ticks.wrapping_add(1),
                p.base_ticks.wrapping_sub(1),
                p.base_ticks.wrapping_sub(3_000_000_000),
                p.base_ticks.wrapping_add(1 << 45),
                p.base_ticks.wrapping_add(u64::from(u32::MAX)),
            ];
            let mut x = 0x9e37_79b9_7f4a_7c15u64 ^ p.mult;
            for _ in 0..2_000 {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                // within ±2^46 ticks of the base: hours to days of time
                ticks.push(p.base_ticks.wrapping_add(x >> 17).wrapping_sub(1 << 46));
            }
            for t in ticks {
                assert_eq!(
                    unsafe { rusteron_dpdk_test_clock_ns_at(t) },
                    reference.to_ns(t),
                    "ticks {t} with {p:?}"
                );
            }
        }
        adopt(calibrated);
    }

    /// `monotonic`, or a host whose counter ergon-clock will not take: the
    /// fallback reads `CLOCK_MONOTONIC` through ergon-clock's conversion,
    /// reported as 0, monotonic, and ergon-clock's fallback's time.
    #[test]
    #[serial]
    fn the_fallback_reads_clock_monotonic_as_ergon_clocks_fallback_does() {
        let fallback = transport_clock(ClockChoice::Monotonic).unwrap().clock;
        adopt(fallback.params());
        assert_eq!(unsafe { rusteron_dpdk_clock_source() }, 0, "monotonic fallback");
        let mut last = unsafe { rusteron_dpdk_clock_ns() };
        for _ in 0..1_000 {
            let now = unsafe { rusteron_dpdk_clock_ns() };
            assert!(now >= last, "monotonic: {last} then {now}");
            last = now;
        }
        assert_bracketed(&fallback, 0);
        assert_tracks(&fallback);
        // a retuned fallback is converted, not read raw
        let (mono, _) = MonotonicClock::always(Domain::Monotonic);
        let retuned = Clock::Monotonic(mono.retuned(mono.ticks(), 1 << 32, 1 << 40).unwrap());
        adopt(retuned.params());
        assert_bracketed(&retuned, 0);
        adopt(transport_clock(ClockChoice::Counter).unwrap().clock.params());
    }

    /// Parameters the transport cannot use are refused at create, naming the
    /// field, before any device work, and the clock in use stays as it was.
    #[test]
    #[serial]
    fn clock_parameters_the_transport_cannot_use_are_refused() {
        let good = transport_clock(ClockChoice::Counter).unwrap().clock;
        adopt(good.params());
        let p = good.params();
        for (bad, field) in [
            (Params { version: 0, ..p }, "version"),
            (Params { version: 2, ..p }, "version"),
            (Params { kind: 2, ..p }, "kind"),
            (Params { domain: 1, ..p }, "domain"),
            (Params { reserved: 1, ..p }, "reserved"),
            (Params { mult: 0, ..p }, "mult"),
            (Params { mult: 1 << 63, ..p }, "mult"),
        ] {
            let env = TestEnv::new();
            env.eal_skip();
            let mut config = rusteron_dpdk_config_t::valid();
            config.clock = bad;
            let err = create(&config).unwrap_err();
            assert!(err.contains("clock") && err.contains(field), "{field}: {err}");
            assert!(
                (0..env.log_count()).all(|i| !env.log_at(i).contains("probe")),
                "{field}: device work before the check"
            );
            assert_eq!(unsafe { rusteron_dpdk_clock_source() }, source_of(&good), "{field}");
            assert_bracketed(&good, 1_000);
        }
    }

    /// A second transport, refused while the first is up (one EAL a process),
    /// leaves the clock the first adopted alone: its agents read it.
    #[test]
    #[serial]
    fn a_refused_second_transport_leaves_the_live_clock_alone() {
        let live = Scale::from_hz(2_400_000_000, 1 << 40, 5_000_000_000).unwrap();
        let other = Scale::from_hz(1_000_000_000, 7, 9).unwrap();
        let params = |s: &Scale| Params {
            version: PARAMS_VERSION,
            kind: KIND_COUNTER,
            domain: 0,
            reserved: 0,
            base_ticks: s.base_ticks(),
            base_ns: s.base_ns(),
            mult: s.mult(),
            hz: s.hz(),
        };
        let env = TestEnv::new();
        env.eal_skip();
        let mut config = rusteron_dpdk_config_t::valid();
        config.clock = params(&live);
        let first = create(&config).unwrap_or_else(|e| panic!("{e}"));
        config.clock = params(&other);
        let err = create(&config).unwrap_err();
        assert!(err.contains("already initialized"), "{err}");
        for t in [0, 1 << 40, (1 << 40) + 12_345, u64::MAX / 3] {
            assert_eq!(unsafe { rusteron_dpdk_test_clock_ns_at(t) }, live.to_ns(t), "ticks {t}");
        }
        assert_eq!(close(first), 0);
        adopt(transport_clock(ClockChoice::Counter).unwrap().clock.params());
    }

    /// The test pin stays in force while a transport adopts a calibration,
    /// and the reset hands the clock back to the calibration adopted.
    #[test]
    #[serial]
    fn a_pinned_clock_stays_pinned_while_a_calibration_is_adopted() {
        let good = transport_clock(ClockChoice::Counter).unwrap().clock;
        let env = TestEnv::new();
        env.eal_skip();
        pin_clock_ns(42_000);
        let mut config = rusteron_dpdk_config_t::valid();
        config.clock = good.params();
        let t = create(&config).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(unsafe { rusteron_dpdk_clock_ns() }, 42_000, "still pinned");
        assert_eq!(close(t), 0);
        drop(TestEnv::new());
        assert_ne!(unsafe { rusteron_dpdk_clock_ns() }, 42_000, "the reset unpins");
        assert_eq!(unsafe { rusteron_dpdk_clock_source() }, source_of(&good));
        assert_bracketed(&good, 1_000);
    }
}
