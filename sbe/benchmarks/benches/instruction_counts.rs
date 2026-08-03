//! Scalar-access and verify diagnostics.
//! iai-callgrind removed (HFT-005): pulled unmaintained proc-macro-error2.
//! Replaced with a criterion harness that measures the same operations.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{Criterion, black_box};
use ergo_sbe_benchmarks::{ergo_car::*, sbe_tool_car_body_decoder};

#[path = "_common.rs"]
mod common;
use common::BASELINE;

const ACCESS_REPETITIONS: usize = 1_024;

pub fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("instruction_counts");

    g.bench_function("ergo_scalar_batch", |b| {
        b.iter(|| {
            let header = MessageHeader(read_bytes::<8>(black_box(BASELINE), 0));
            let car = CarDecoder::wrap(
                black_box(BASELINE),
                0,
                header.block_length() as usize,
                header.version(),
            );
            for _ in 0..ACCESS_REPETITIONS {
                let car = black_box(&car);
                black_box((car.serial_number(), car.model_year()));
            }
        });
    });

    g.bench_function("sbe_tool_scalar_batch", |b| {
        b.iter(|| {
            let block_length = u16::from_le_bytes(BASELINE[0..2].try_into().unwrap());
            let version = u16::from_le_bytes(BASELINE[6..8].try_into().unwrap());
            let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, block_length, version);
            for _ in 0..ACCESS_REPETITIONS {
                let car = black_box(&car);
                black_box((car.serial_number(), car.model_year()));
            }
        });
    });

    g.bench_function("ergo_composite_batch", |b| {
        b.iter(|| {
            let header = MessageHeader(read_bytes::<8>(black_box(BASELINE), 0));
            let car = CarDecoder::wrap(
                black_box(BASELINE),
                0,
                header.block_length() as usize,
                header.version(),
            );
            for _ in 0..ACCESS_REPETITIONS {
                let engine = black_box(&car).engine();
                black_box((engine.capacity(), engine.num_cylinders()));
            }
        });
    });

    g.bench_function("sbe_tool_composite_batch", |b| {
        b.iter(|| {
            let block_length = u16::from_le_bytes(BASELINE[0..2].try_into().unwrap());
            let version = u16::from_le_bytes(BASELINE[6..8].try_into().unwrap());
            let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, block_length, version);
            for _ in 0..ACCESS_REPETITIONS {
                let engine = black_box(&car).engine_decoder();
                black_box((engine.capacity(), engine.num_cylinders()));
            }
        });
    });

    g.finish();
}

criterion::criterion_group!(benches, bench);
criterion::criterion_main!(benches);
