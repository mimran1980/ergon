//! Warmed batch latency distribution. Wall-clock percentiles are intentionally
//! limited to batch work where timer resolution is meaningful.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use std::hint::black_box;
use std::time::Instant;

use ergo_sbe_benchmarks::ergo_car::*;
use hdrhistogram::Histogram;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

const BATCH: usize = 1_000;
const SAMPLES: usize = 10_000;

fn decode_batch() -> u64 {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    let mut sum = 0u64;
    for _ in 0..BATCH {
        sum = sum.wrapping_add(
            CarDecoder::try_wrap(
                black_box(BASELINE),
                0,
                header.block_length() as usize,
                header.version(),
            )
            .serial_number(),
        );
    }
    sum
}

fn main() {
    for _ in 0..1_000 {
        black_box(decode_batch());
    }
    let mut histogram = Histogram::<u64>::new_with_max(1_000_000_000, 3).unwrap();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        black_box(decode_batch());
        histogram
            .record(started.elapsed().as_nanos().try_into().unwrap())
            .unwrap();
    }
    println!(
        "warm batch ({BATCH} messages): p50={}ns p99={}ns p99.9={}ns",
        histogram.value_at_quantile(0.50),
        histogram.value_at_quantile(0.99),
        histogram.value_at_quantile(0.999),
    );
}
