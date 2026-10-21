//! The library's clocks plus the read-cost probe.

pub use ergon_io::clock::*;

/// The raw counter [`rdtsc`] reads on this architecture.
#[must_use]
pub const fn counter_name() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "cntvct_el0"
    } else if cfg!(target_arch = "x86_64") {
        "rdtsc"
    } else {
        "none"
    }
}

/// `meta.txt` lines naming the counter the loops time themselves with, its
/// rate in Hz and where the rate came from: `CNTFRQ_EL0` (aarch64, exact) or
/// `measured` (the TSC, against `CLOCK_MONOTONIC`).
#[must_use]
pub fn counter_meta() -> String {
    let rate = if counter_frequency_hz().is_some() {
        "CNTFRQ_EL0"
    } else {
        "measured"
    };
    format!(
        "counter={}\ncounter_hz={:.0}\ncounter_rate={rate}\n",
        counter_name(),
        ticks_per_us() * 1e6
    )
}

/// Mean cost in nanoseconds of `clock_gettime(CLOCK_REALTIME)` and the raw counter
/// ([`rdtsc`]), over `iters` reads each.
#[must_use]
pub fn read_costs(iters: u32) -> (f64, f64) {
    let mut sink = 0i64;
    let t0 = monotonic_ns();
    for _ in 0..iters {
        sink = sink.wrapping_add(std::hint::black_box(realtime_ns()));
    }
    let t1 = monotonic_ns();
    let mut sink2 = 0u64;
    for _ in 0..iters {
        sink2 = sink2.wrapping_add(std::hint::black_box(rdtsc()));
    }
    let t2 = monotonic_ns();
    std::hint::black_box((sink, sink2));
    let n = f64::from(iters);
    ((t1 - t0) as f64 / n, (t2 - t1) as f64 / n)
}
