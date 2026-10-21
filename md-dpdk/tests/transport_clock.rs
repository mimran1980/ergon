//! The transport's clock: the CPU counter at the rate DPDK's EAL gives it,
//! else `CLOCK_MONOTONIC`, started by the native transport at create.
//!
//! The choice (`RUSTERON_DPDK_CLOCK`) and the kernel clock-source rule run on
//! every platform. With the `dpdk` feature (Linux x86_64 or aarch64, against
//! the DPDK-free fakes, whose EAL reports a rate a test sets: none after the
//! reset) the native side is checked: the counter path at the EAL's rate, the
//! conversion against the 32.32 arithmetic, the rule for a rate the CPU does
//! not vouch for, the fallback, the configuration's check and the test pin.
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

use ergon_md_dpdk::DpdkError;
use ergon_md_dpdk::clock::{CLOCK_ENV, ClockChoice, MonotonicReason, kernel_refuses_counter};
use serial_test::serial;

// --- The choice and the kernel's clock source (every platform) ---------------

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
    assert_eq!(ClockChoice::default(), ClockChoice::Counter);
}

/// The kernel's clock source keeps the counter only where it is the counter
/// (`tsc` on x86_64, `arch_sys_counter` on aarch64) or a hypervisor's scaled
/// view of it (`kvm-clock`, Azure's `hyperv_clocksource_tsc_page`, `xen`), as
/// sysfs gives it (a trailing newline). Any other source (`hpet` or `acpi_pm`,
/// where the kernel's watchdog put it after finding the counter unstable; the
/// other architecture's counter) rules the counter out, naming the source.
#[test]
fn the_kernel_clocksource_rules_out_a_counter_it_does_not_keep_time_with() {
    let (counter, other) = if cfg!(target_arch = "aarch64") {
        ("arch_sys_counter", "tsc")
    } else {
        ("tsc", "arch_sys_counter")
    };
    for kept in [counter, "kvm-clock", "hyperv_clocksource_tsc_page", "xen"] {
        assert_eq!(kernel_refuses_counter(kept), None, "{kept}");
        assert_eq!(kernel_refuses_counter(&format!("{kept}\n")), None, "{kept} from sysfs");
    }
    for refused in ["hpet", "acpi_pm", "jiffies", other, ""] {
        assert_eq!(
            kernel_refuses_counter(&format!("{refused}\n")),
            Some(MonotonicReason::KernelClocksource(refused.to_string())),
            "{refused:?}"
        );
    }
    let text = MonotonicReason::KernelClocksource("hpet".to_string()).to_string();
    assert!(text.contains("hpet"), "{text}");
}

// --- The native transport's clock (`dpdk`) ------------------------------------

#[cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod native {
    use super::common::*;
    use serial_test::serial;
    use std::os::raw::c_int;

    fn now_ns() -> u64 {
        unsafe { rusteron_dpdk_clock_ns() }
    }

    /// A transport created over `config` with the fake EAL reporting what
    /// `setup` set, closed again: the clock it started stays (it is
    /// process-lifetime) until the next reset.
    fn start(config: &rusteron_dpdk_config_t, setup: impl FnOnce(&TestEnv)) -> TestEnv {
        let env = TestEnv::new();
        env.eal_skip();
        setup(&env);
        let t = create(config).unwrap_or_else(|e| panic!("create: {e}"));
        assert_eq!(close(t), 0);
        env
    }

    /// The tightest of 16 reads of the transport's clock bracketed by
    /// `CLOCK_MONOTONIC`: (the native read, the bracket's midpoint, its width).
    fn bracketed() -> (u64, u64, u64) {
        let mut best = (0, 0, u64::MAX);
        for _ in 0..16 {
            let m0 = monotonic_ns();
            let n = now_ns();
            let m1 = monotonic_ns();
            if m1 - m0 < best.2 {
                best = (n, m0 + (m1 - m0) / 2, m1 - m0);
            }
        }
        best
    }

    /// Every native read lies between `CLOCK_MONOTONIC` reads either side of
    /// it, with no slack: the fallback reads `CLOCK_MONOTONIC` itself.
    fn assert_reads_clock_monotonic() {
        let mut last = 0;
        for i in 0..10_000 {
            let before = monotonic_ns();
            let native = now_ns();
            let after = monotonic_ns();
            assert!(
                before <= native && native <= after,
                "read {i}: native {native} outside CLOCK_MONOTONIC's {before}..{after}"
            );
            assert!(native >= last, "read {i}: {last} then {native}");
            last = native;
        }
    }

    /// The 32.32 conversion of `ticks` as the C hot path computes it from a
    /// started clock's parameters, in Rust: the reference.
    fn reference_ns_at(base_ticks: u64, base_ns: u64, mult: u64, ticks: u64) -> u64 {
        let delta = ticks.wrapping_sub(base_ticks) as i64;
        base_ns.wrapping_add(((i128::from(delta) * i128::from(mult as i64)) >> 32) as i64 as u64)
    }

    /// ns per tick at `hz`, 32.32 and rounded to the nearest.
    fn mult_for(hz: u64) -> u64 {
        u64::try_from(((1_000_000_000u128 << 32) + u128::from(hz / 2)) / u128::from(hz)).unwrap()
    }

    fn params() -> (u64, u64, u64) {
        let (mut base_ticks, mut base_ns, mut mult) = (0, 0, 0);
        unsafe { rusteron_dpdk_test_clock_params(&mut base_ticks, &mut base_ns, &mut mult) };
        (base_ticks, base_ns, mult)
    }

    /// The counter path: with the EAL giving the counter's own rate, the
    /// transport reads the counter (counter type 80's source for this
    /// architecture) at that rate, in nanoseconds since boot (its first read
    /// within 50 us of `CLOCK_MONOTONIC`), advancing as `CLOCK_MONOTONIC`
    /// does to 0.1% over 100 ms; a read is never more than a microsecond
    /// behind the one before it on one thread (the reads are unordered).
    #[test]
    #[serial]
    fn the_counter_path_runs_at_the_eal_rate() {
        let hz = measured_counter_hz();
        let env = TestEnv::new();
        env.eal_skip();
        env.set_counter_rate(hz);
        let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
        let report = clock_report();
        assert_eq!(unsafe { rusteron_dpdk_clock_source() }, COUNTER_SOURCE, "{report:?}");
        assert_eq!(
            (report.source, report.why, report.hz, report.eal_hz),
            (COUNTER_SOURCE, WHY_COUNTER, hz, hz),
            "{report:?}"
        );

        let (native, mono, width) = bracketed();
        assert!(
            native.abs_diff(mono) <= 50_000 + width,
            "first read {native} ns, CLOCK_MONOTONIC {mono} ns (bracket {width} ns)"
        );

        let (n0, m0, _) = bracketed();
        while monotonic_ns() - m0 < 100_000_000 {
            std::hint::spin_loop();
        }
        let (n1, m1, _) = bracketed();
        let (native, mono) = (n1 - n0, m1 - m0);
        assert!(
            native.abs_diff(mono) * 1000 <= mono,
            "native advanced {native} ns against {mono} ns of CLOCK_MONOTONIC (more than 0.1% apart)"
        );

        let mut last = now_ns();
        for i in 0..10_000 {
            let now = now_ns();
            assert!(now + 1_000 >= last, "read {i}: {last} then {now}");
            last = last.max(now);
        }
        assert_eq!(close(t), 0);
    }

    /// The conversion is the EAL's rate in 32.32 fixed point, rounded: for
    /// each rate the factor is 1e9 * 2^32 / hz to the nearest, the native
    /// result matches the Rust reference to the bit, a second of ticks is a
    /// second (within 1 ns), and any k ticks are k * 1e9 / hz ns within the
    /// factor's rounding (k / 2^33 ns) and 1 ns. A tick behind the base (an
    /// unordered read on another core, just after the start) is a little
    /// before the base's time, never 2^64 ns later.
    #[test]
    #[serial]
    fn the_counter_conversion_is_the_eal_rate() {
        for hz in [2_400_000_000u64, 1_000_000_000, 121_875_000, 24_000_000] {
            let _env = start(&rusteron_dpdk_config_t::valid(), |env| env.set_counter_rate(hz));
            assert_eq!(clock_report().hz, hz);
            let (base_ticks, base_ns, mult) = params();
            assert_eq!(mult, mult_for(hz), "{hz} Hz");
            let ns_at = |t: u64| unsafe { rusteron_dpdk_test_clock_ns_at(t) };

            let mut ticks = vec![
                base_ticks,
                base_ticks + 1,
                base_ticks - 1,
                base_ticks - hz / 100,
                base_ticks + (1 << 45),
                base_ticks + u64::from(u32::MAX),
            ];
            let mut x = 0x9e37_79b9_7f4a_7c15u64 ^ hz;
            let mut next = || {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x
            };
            for _ in 0..2_000 {
                // within ±2^40 ticks of the base: minutes to hours of time
                ticks.push(base_ticks.wrapping_add(next() >> 23).wrapping_sub(1 << 40));
            }
            for &t in &ticks {
                assert_eq!(
                    ns_at(t),
                    reference_ns_at(base_ticks, base_ns, mult, t),
                    "{hz} Hz: ticks {t}"
                );
            }

            for &t in &ticks[..8] {
                let second = ns_at(t + hz) - ns_at(t);
                assert!(
                    second.abs_diff(1_000_000_000) <= 1,
                    "{hz} Hz: a second of ticks is {second} ns"
                );
            }
            for _ in 0..2_000 {
                let k = next() >> 24; // < 2^40
                let t = base_ticks.wrapping_add(next() >> 34);
                let diff = u128::from(ns_at(t + k) - ns_at(t));
                // |diff - k * 1e9 / hz| <= k / 2^33 + 1, scaled by hz
                let err = (diff * u128::from(hz)).abs_diff(u128::from(k) * 1_000_000_000);
                let bound = (u128::from(k) * u128::from(hz) >> 33) + u128::from(hz) + 1;
                assert!(err <= bound, "{hz} Hz: {k} ticks are {diff} ns");
            }

            for behind in [1, hz / 100] {
                let ns = ns_at(base_ticks - behind);
                assert!(
                    ns < base_ns && base_ns - ns <= behind * 1_000_000_000 / hz + 1,
                    "{hz} Hz: {behind} ticks behind the base is {ns} ns, the base {base_ns} ns"
                );
            }
        }
    }

    /// No rate from the EAL (the fakes' default, and what the real EAL
    /// reports before it is up): the transport reads `CLOCK_MONOTONIC`,
    /// reported as source 0 with no rate, and every read is `CLOCK_MONOTONIC`
    /// itself.
    #[test]
    #[serial]
    fn the_fallback_is_clock_monotonic_when_the_eal_gives_no_counter_rate() {
        let _env = start(&rusteron_dpdk_config_t::valid(), |_| {});
        let report = clock_report();
        assert_eq!(
            (report.source, report.why, report.hz, report.eal_hz),
            (SOURCE_MONOTONIC, WHY_NO_RATE, 0, 0),
            "{report:?}"
        );
        assert_eq!(unsafe { rusteron_dpdk_clock_source() }, SOURCE_MONOTONIC);
        assert_eq!(params(), (0, 0, 1 << 32), "a factor of one");
        assert_reads_clock_monotonic();
    }

    /// The configuration's `monotonic` (`RUSTERON_DPDK_CLOCK=monotonic`, read
    /// in Rust) wins over any rate the EAL gives.
    #[test]
    #[serial]
    fn the_monotonic_choice_forces_the_fallback_whatever_the_eal_rate() {
        let mut config = rusteron_dpdk_config_t::valid();
        config.clock = CLOCK_CHOICE_MONOTONIC;
        let _env = start(&config, TestEnv::use_counter);
        let report = clock_report();
        assert_eq!(
            (report.source, report.why, report.hz),
            (SOURCE_MONOTONIC, WHY_CHOSEN, 0),
            "{report:?}"
        );
        assert_eq!(report.eal_hz, measured_counter_hz(), "what the EAL said, reported");
        assert_reads_clock_monotonic();
    }

    /// The rule for the EAL's rate, for both counters whatever this host's:
    /// a rate in 1 MHz..=20 GHz, and the TSC invariant (x86_64) or the rate
    /// `CNTFRQ_EL0`'s (aarch64: a DPDK timing with the PMU's cycle counter
    /// reports another). Through create, a rate this CPU does not vouch for
    /// leaves the transport on `CLOCK_MONOTONIC`, saying why.
    #[test]
    #[serial]
    fn a_counter_rate_the_cpu_does_not_vouch_for_is_not_used() {
        let rule = |source: c_int, eal_hz: u64, invariant: c_int, cntfrq: u64| {
            let rate = CounterRate {
                eal_hz,
                cntfrq,
                invariant,
            };
            unsafe { rusteron_dpdk_clock_rule(source, &rate) }
        };
        let ghz = 1_000_000_000;
        assert_eq!(rule(SOURCE_TSC, 3 * ghz, 1, 0), WHY_COUNTER);
        assert_eq!(rule(SOURCE_TSC, 3 * ghz, 0, 0), WHY_NOT_INVARIANT);
        assert_eq!(rule(SOURCE_CNTVCT, ghz, 0, ghz), WHY_COUNTER);
        assert_eq!(rule(SOURCE_CNTVCT, 3 * ghz, 0, ghz), WHY_NOT_CNTFRQ);
        assert_eq!(rule(SOURCE_CNTVCT, ghz, 0, 0), WHY_NOT_CNTFRQ);
        for (hz, why) in [
            (0, WHY_NO_RATE),
            (999_999, WHY_NO_RATE),
            (1_000_000, WHY_COUNTER),
            (20 * ghz, WHY_COUNTER),
            (20 * ghz + 1, WHY_NO_RATE),
        ] {
            assert_eq!(rule(SOURCE_TSC, hz, 1, hz), why, "TSC at {hz} Hz");
            assert_eq!(rule(SOURCE_CNTVCT, hz, 1, hz), why, "CNTVCT at {hz} Hz");
        }

        let hz = measured_counter_hz();
        let (invariant, cntfrq, why) = if cfg!(target_arch = "aarch64") {
            (true, hz / 2, WHY_NOT_CNTFRQ)
        } else {
            (false, 0, WHY_NOT_INVARIANT)
        };
        let _env = start(&rusteron_dpdk_config_t::valid(), |env| {
            env.set_counter_facts(hz, invariant, cntfrq)
        });
        let report = clock_report();
        assert_eq!(
            (report.source, report.why, report.hz, report.eal_hz, report.cntfrq),
            (SOURCE_MONOTONIC, why, 0, hz, cntfrq),
            "{report:?}"
        );
        assert_reads_clock_monotonic();
    }

    /// A clock choice the transport does not know is refused at create,
    /// naming the field, before any device work, and the clock already
    /// started stays as it was.
    #[test]
    #[serial]
    fn a_clock_choice_the_transport_does_not_know_is_refused() {
        let env = start(&rusteron_dpdk_config_t::valid(), TestEnv::use_counter);
        let live = (clock_report(), params());
        assert_eq!(live.0.source, COUNTER_SOURCE);
        for bad in [2u8, 255] {
            let logged = env.log_count();
            let mut config = rusteron_dpdk_config_t::valid();
            config.clock = bad;
            let err = create(&config).unwrap_err();
            assert!(err.contains("clock"), "{bad}: {err}");
            assert_eq!(env.log_count(), logged, "{bad}: device work before the check");
            assert_eq!((clock_report(), params()), live, "{bad}");
        }
    }

    /// A second transport, refused while the first is up (one EAL a process),
    /// leaves the clock the first started alone: its agents read it.
    #[test]
    #[serial]
    fn a_refused_second_transport_leaves_the_live_clock_alone() {
        let env = TestEnv::new();
        env.eal_skip();
        env.set_counter_rate(2_400_000_000);
        let config = rusteron_dpdk_config_t::valid();
        let first = create(&config).unwrap_or_else(|e| panic!("{e}"));
        let live = (clock_report(), params());
        env.set_counter_rate(1_000_000_000);
        let err = create(&config).unwrap_err();
        assert!(err.contains("already initialized"), "{err}");
        assert_eq!((clock_report(), params()), live);
        assert_eq!(live.0.hz, 2_400_000_000);
        let (base_ticks, _, _) = live.1;
        let second = unsafe {
            rusteron_dpdk_test_clock_ns_at(base_ticks + 2_400_000_000) - rusteron_dpdk_test_clock_ns_at(base_ticks)
        };
        assert_eq!(second, 1_000_000_000);
        assert_eq!(close(first), 0);
    }

    /// The test pin stays in force while a transport starts the clock, which
    /// still reports the counter it started; the reset unpins and returns the
    /// clock to its boot default, `CLOCK_MONOTONIC`, never started.
    #[test]
    #[serial]
    fn a_pinned_clock_stays_pinned_while_the_clock_starts() {
        let env = TestEnv::new();
        env.eal_skip();
        env.use_counter();
        pin_clock_ns(42_000);
        let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(now_ns(), 42_000, "still pinned");
        assert_eq!(unsafe { rusteron_dpdk_clock_source() }, COUNTER_SOURCE, "started");
        assert_eq!(close(t), 0);
        drop(env);
        let _env = TestEnv::new();
        assert_ne!(now_ns(), 42_000, "the reset unpins");
        let report = clock_report();
        assert_eq!(
            (report.source, report.why),
            (SOURCE_MONOTONIC, WHY_NOT_STARTED),
            "{report:?}"
        );
        assert_reads_clock_monotonic();
    }
}
