//! Per-operation latency. Each sample times one operation between two
//! ordered reads of quanta's counter (`lfence` + `rdtsc` on x86-64, `isb` +
//! `CNTVCT_EL0` on aarch64, else `CLOCK_MONOTONIC`), so every row carries
//! the counter's own cost and resolution.
//!
//! | Arm | One operation |
//! |---|---|
//! | `clock` | nothing: the floor and resolution every other arm carries |
//! | `car_decode` | `CarDecoder::wrap` + `serial_number` |
//! | `new_order_fixed` | `NewOrder` header + `fixed()` + `asset` |
//! | `new_order_encode_from` | `NewOrder` header + `encode_from` |
//! | `new_order_mapped` | `side()`, `price()`, `qty()` into application types |
//! | `new_order_raw` | `raw_side()` and both mantissas |
//!
//! Each row prints p50, p99, p99.9 and max, the max's sample index, the
//! samples at or above 1 µs and a first guess at the max's cause
//! (`max_cause`). The header prints one counter tick in ns: an operation
//! shorter than a tick reads 0 or one tick (aarch64's counter is often
//! 24 MHz, 41.67 ns), so medians come from Criterion and this run is for
//! the tail. `-- --quick` takes 10,000 samples an arm instead of 1,000,000.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unsafe_code)]

use std::hint::black_box;

use ergo_sbe_benchmarks::ergo_car::*;
use ergo_sbe_benchmarks::new_order_fixture::{
    AppPrice, AppSide, BUY, FRAME_LEN, encode_fixed, encode_from, read_mapped, read_raw,
};
use ergo_sbe_benchmarks::new_order_mapped::NewOrderDecoder;
use hdrhistogram::Histogram;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

const WARMUP: usize = 100_000;
const SAMPLES: usize = 1_000_000;
const QUICK_SAMPLES: usize = 10_000;
/// Samples at or above this are interrupt-scale: counted per arm.
const SLOW_NS: u64 = 1_000;

/// Holds the counter read that follows until every earlier instruction has
/// completed: quanta's raw read is unordered.
#[inline(always)]
fn ordering_fence() {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: `lfence` is SSE2, part of every x86-64 CPU; it touches no memory.
    unsafe {
        core::arch::x86_64::_mm_lfence()
    };
    #[cfg(target_arch = "aarch64")]
    // SAFETY: `isb` only flushes the pipeline; it touches no memory or stack.
    unsafe {
        core::arch::asm!("isb", options(nostack, preserves_flags))
    };
}

struct Row {
    name: &'static str,
    histogram: Histogram<u64>,
    max_at: usize,
    slow: u64,
}

/// Warm `op`, then time `samples` single calls of it. The histogram is
/// allocated before the loop; recording does not allocate.
fn measure(clock: &quanta::Clock, name: &'static str, samples: usize, mut op: impl FnMut()) -> Row {
    for _ in 0..WARMUP {
        op();
    }
    let mut histogram = Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).unwrap();
    let (mut max, mut max_at, mut slow) = (0u64, 0usize, 0u64);
    for i in 0..samples {
        ordering_fence();
        let start = clock.raw();
        op();
        ordering_fence();
        let end = clock.raw();
        let ns = clock.delta_as_nanos(start, end);
        histogram.saturating_record(ns);
        if ns >= SLOW_NS {
            slow += 1;
        }
        if ns > max {
            max = ns;
            max_at = i;
        }
    }
    Row {
        name,
        histogram,
        max_at,
        slow,
    }
}

/// A first guess at a row's max. The `clock` arm times nothing, so a max it
/// also reaches, give or take one counter tick, is the host (an interrupt,
/// a preemption, a counter migration) or the counter's resolution, not the
/// operation.
fn max_cause(row: &Row, clock_max: u64, tick_ns: f64, samples: usize) -> &'static str {
    let max = row.histogram.max();
    if row.max_at < samples / 100 {
        "early: in the first 1% of samples, warm-up not finished"
    } else if row.name == "clock" {
        "host: the clock arm times nothing"
    } else if max as f64 <= clock_max as f64 + tick_ns {
        "host: within one tick of the empty clock arm's max"
    } else if max >= SLOW_NS {
        "unattributed: interrupt-scale and above the clock arm's max; rerun pinned on an isolated CPU"
    } else {
        "unattributed: over a tick above the clock arm's max; check the operation"
    }
}

fn main() {
    let quick = std::env::args().any(|arg| arg == "--quick");
    let samples = if quick { QUICK_SAMPLES } else { SAMPLES };

    // Correctness before timing: equal frames, equal values.
    let mut fixed = [0x00u8; 128];
    let mut from = [0xAAu8; 128];
    assert_eq!(encode_fixed(&mut fixed, &BUY).unwrap(), FRAME_LEN);
    assert_eq!(encode_from(&mut from, &BUY).unwrap(), FRAME_LEN);
    assert_eq!(fixed[..FRAME_LEN], from[..FRAME_LEN]);
    let decoder = NewOrderDecoder::try_from(&fixed[..FRAME_LEN]).unwrap();
    let price = AppPrice {
        mantissa: BUY.price,
    };
    let qty = AppPrice { mantissa: BUY.qty };
    assert_eq!(read_mapped(&decoder).unwrap(), (AppSide::Buy, price, qty));
    assert_eq!(read_raw(&decoder), (1, BUY.price, BUY.qty));
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    let car = |buf: &[u8]| {
        CarDecoder::wrap(buf, 0, header.block_length() as usize, header.version()).serial_number()
    };
    assert_eq!(car(BASELINE), 1234);

    // quanta calibrates once a process, up to 200 ms: before any arm.
    let clock = quanta::Clock::new();
    let mut buf = [0u8; 128];
    let rows = [
        measure(&clock, "clock", samples, || black_box(())),
        measure(&clock, "car_decode", samples, || {
            black_box(car(black_box(BASELINE)));
        }),
        measure(&clock, "new_order_fixed", samples, || {
            black_box(encode_fixed(black_box(&mut buf), black_box(&BUY))).ok();
        }),
        measure(&clock, "new_order_encode_from", samples, || {
            black_box(encode_from(black_box(&mut buf), black_box(&BUY))).ok();
        }),
        measure(&clock, "new_order_mapped", samples, || {
            black_box(read_mapped(black_box(&decoder))).ok();
        }),
        measure(&clock, "new_order_raw", samples, || {
            black_box(read_raw(black_box(&decoder)));
        }),
    ];

    let clock_max = rows[0].histogram.max();
    // One counter tick in ns: an operation shorter than this reads 0 or one
    // tick, so take medians from Criterion and read this run for the tail.
    let tick_ns = clock.delta_as_nanos(0, 1 << 20) as f64 / f64::from(1u32 << 20);
    println!(
        "per-operation latency, ns, {samples} samples an arm (quanta counter, ordered reads, tick {tick_ns:.2} ns)"
    );
    for row in &rows {
        let h = &row.histogram;
        println!(
            "{:<22} p50={} p99={} p99.9={} max={} (sample {}) >={SLOW_NS}ns={} max_cause={}",
            row.name,
            h.value_at_quantile(0.50),
            h.value_at_quantile(0.99),
            h.value_at_quantile(0.999),
            h.max(),
            row.max_at,
            row.slow,
            max_cause(row, clock_max, tick_ns, samples),
        );
    }
}
