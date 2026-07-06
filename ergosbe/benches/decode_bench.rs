//! Decode benchmarks for ErgoSBE-generated Car message codec.
//!
//! Measures decode latency for the entry point, individual field accessors
//! (both checked and raw/unchecked variants), and group iteration.

// Generated code generates lots of diagnostics; suppress across the crate.
#![allow(unsafe_code, missing_docs, unused_variables, dead_code, unused_mut)]
#![allow(clippy::useless_comparison)]

include!("generated/car_patched.rs");

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

#[path = "_common.rs"]
mod common;
use common::BASELINE;

// ── Decode entry point ───────────────────────────────────────────────

fn bench_try_from(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/try_from");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));
    group.bench_function("car", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            black_box(car);
        });
    });
    group.finish();
}

// ── Scalar field access (checked) ───────────────────────────────────

fn bench_field_access_checked(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("decode/field/checked");
    group.bench_function("serial_number", |b| {
        b.iter(|| black_box(car.serial_number().unwrap()));
    });
    group.bench_function("model_year", |b| {
        b.iter(|| black_box(car.model_year().unwrap()));
    });
    group.bench_function("engine", |b| {
        b.iter(|| black_box(car.engine().unwrap()));
    });
    group.finish();
}

// ── Scalar field access (raw/unchecked) ────────────────────────────

fn bench_field_access_raw(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("decode/field/raw");
    group.bench_function("raw_serial_number", |b| {
        b.iter(|| black_box(car.raw_serial_number()));
    });
    group.bench_function("raw_model_year", |b| {
        b.iter(|| black_box(car.raw_model_year()));
    });
    group.bench_function("raw_engine", |b| {
        b.iter(|| black_box(unsafe { car.engine_unchecked() }));
    });
    group.finish();
}

// ── Group iteration ────────────────────────────────────────────────

fn bench_group_iteration(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("decode/group");
    group.bench_function("fuel_figures", |b| {
        b.iter(|| {
            let ff = car.fuel_figures().unwrap();
            let n = ff.len();
            let mut sum_speed: u64 = 0;
            let mut sum_mpg: f64 = 0.0;
            for entry in ff {
                sum_speed += entry.speed().unwrap() as u64;
                sum_mpg += entry.mpg().unwrap() as f64;
            }
            black_box((n, sum_speed, sum_mpg));
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_try_from,
    bench_field_access_checked,
    bench_field_access_raw,
    bench_group_iteration,
);
criterion_main!(benches);
