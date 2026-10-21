//! The raw counter and its rate.
#![allow(missing_docs)]

use ergon_io::clock::{counter_frequency_hz, monotonic_ns, rdtsc, ticks_per_us};

/// Where the counter's rate is architected (aarch64: `CNTFRQ_EL0`), the
/// rate is read, not estimated from 20 ms against `CLOCK_MONOTONIC`.
#[test]
fn ticks_per_us_is_the_architected_counter_rate_where_there_is_one() {
    let hz = counter_frequency_hz();
    assert_eq!(hz.is_some(), cfg!(target_arch = "aarch64"), "{hz:?}");
    if let Some(hz) = hz {
        #[allow(clippy::cast_precision_loss)]
        let want = hz as f64 / 1e6;
        let got = ticks_per_us();
        assert!(
            (got - want).abs() <= want * 1e-12,
            "ticks_per_us {got} against CNTFRQ_EL0 {hz} Hz"
        );
    }
}

/// Whichever way the rate was found, it converts the counter to time: 50 ms
/// of counter ticks against `CLOCK_MONOTONIC` agree within 1 %.
#[test]
fn the_counter_rate_matches_the_monotonic_clock() {
    let tpu = ticks_per_us();
    let (m0, t0) = (monotonic_ns(), rdtsc());
    while monotonic_ns() - m0 < 50_000_000 {
        std::hint::spin_loop();
    }
    let (m1, t1) = (monotonic_ns(), rdtsc());
    #[allow(clippy::cast_precision_loss)]
    let measured = t1.wrapping_sub(t0) as f64 / ((m1 - m0) as f64 / 1000.0);
    assert!(
        (measured / tpu - 1.0).abs() < 0.01,
        "{measured} ticks/µs measured against ticks_per_us {tpu}"
    );
}
