//! Decode benchmarks for ErgoSBE-generated Car message codec.
//!
//! Measures decode latency for the entry point, individual field accessors
//! (both checked and raw/unchecked variants), group iteration, and HFT-specific
//! tight-loop / field-stride / alloc-free decode patterns.

// Generated code generates lots of diagnostics; suppress across the crate.
#![allow(unsafe_code, missing_docs, unused_variables, dead_code, unused_mut)]
#![allow(clippy::useless_comparison)]

include!("generated/car_patched.rs");

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

#[path = "_common.rs"]
mod common;
use common::BASELINE;

/// Number of messages in the tight-loop / batch benchmarks.
const HFT_BATCH: usize = 10_000;

/// Pre-allocate a buffer containing `count` copies of the baseline message.
fn replicate_baseline(count: usize) -> Vec<u8> {
    let msg_len = BASELINE.len();
    let mut buf = Vec::with_capacity(count * msg_len);
    // SAFETY: we immediately fill the capacity with known bytes.
    unsafe { buf.set_len(count * msg_len) };
    for chunk in buf.chunks_mut(msg_len) {
        chunk.copy_from_slice(BASELINE);
    }
    buf
}

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
        b.iter(|| black_box(car.serial_number()));
    });
    group.bench_function("model_year", |b| {
        b.iter(|| black_box(car.model_year()));
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
        b.iter(|| black_box(car.serial_number()));
    });
    group.bench_function("raw_model_year", |b| {
        b.iter(|| black_box(car.model_year()));
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

// ── Full decode with unchecked scalar access ───────────────────────

fn bench_full_decode_unchecked(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/unchecked");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));
    group.bench_function("car_full", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            let _ = car.serial_number();
            let _ = car.model_year();
            let _ = unsafe { car.available_unchecked() };
            let _ = unsafe { car.code_unchecked() };
            let _ = car.raw_some_numbers();
            let _ = car.raw_vehicle_code();
            let _ = unsafe { car.extras_unchecked() };
            let _ = unsafe { car.engine_unchecked() };
            black_box(());
        });
    });
    group.finish();
}

// ── Checked vs unchecked comparison ──────────────────────────────

fn bench_decode_checked_vs_unchecked(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("decode/checked_vs_unchecked");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));

    group.bench_function("checked_all_fields", |b| {
        b.iter(|| {
            let _ = car.serial_number();
            let _ = car.model_year();
            let _ = car.available().unwrap();
            let _ = car.code().unwrap();
            let _ = car.some_numbers().unwrap();
            let _ = car.vehicle_code().unwrap();
            let _ = car.extras().unwrap();
            let _ = car.engine().unwrap();
            black_box(());
        });
    });

    group.bench_function("unchecked_all_fields", |b| {
        b.iter(|| {
            let _ = car.serial_number();
            let _ = car.model_year();
            let _ = unsafe { car.available_unchecked() };
            let _ = unsafe { car.code_unchecked() };
            let _ = car.raw_some_numbers();
            let _ = car.raw_vehicle_code();
            let _ = unsafe { car.extras_unchecked() };
            let _ = unsafe { car.engine_unchecked() };
            black_box(());
        });
    });

    group.finish();
}

// ── HFT: tight-loop decode ────────────────────────────────────────
//
// Simulates a feed handler: decode 10k messages from a pre-allocated buffer.
// Each iteration decodes one message and reads a few key fields.

fn bench_hft_tight_loop(c: &mut Criterion) {
    let batch = replicate_baseline(HFT_BATCH);
    let msg_len = BASELINE.len();

    let mut group = c.benchmark_group("decode/hft/tight_loop");
    group.throughput(Throughput::Elements(HFT_BATCH as u64));

    group.bench_function("10k_messages", |b| {
        b.iter(|| {
            let mut pos = 0usize;
            let end = batch.len();
            let mut sum_serial: u64 = 0;
            let mut sum_year: u64 = 0;
            while pos + msg_len <= end {
                let car = CarDecoder::try_from(black_box(&batch[pos..pos + msg_len])).unwrap();
                sum_serial += car.serial_number();
                sum_year += car.model_year() as u64;
                pos += msg_len;
            }
            black_box((sum_serial, sum_year));
        });
    });

    group.finish();
}

// ── HFT: field stride ─────────────────────────────────────────────
//
// Measures latency of striding through specific fields in sequence:
// modelYear, engine.capacity, fuelFigures[0].speed.
// Each benchmark measures one stride pattern independently.

fn bench_hft_field_stride(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let engine = unsafe { car.engine_unchecked() };
    let ff = car.fuel_figures().unwrap();
    let first_entry = if ff.len() > 0 {
        Some(ff.into_iter().next().unwrap())
    } else {
        None
    };

    let mut group = c.benchmark_group("decode/hft/field_stride");
    group.throughput(Throughput::Elements(1));

    group.bench_function("model_year", |b| {
        b.iter(|| black_box(car.model_year()));
    });

    group.bench_function("engine_capacity", |b| {
        let cap = engine.capacity();
        b.iter(|| black_box(cap));
    });

    if let Some(entry) = first_entry {
        group.bench_function("fuel_figures[0].speed", |b| {
            b.iter(|| black_box(entry.raw_speed()));
        });

        group.bench_function("all_three_strided", |b| {
            b.iter(|| {
                let m = car.model_year();
                let e = engine.capacity();
                let s = entry.raw_speed();
                black_box((m, e, s));
            });
        });
    }

    group.finish();
}

// ── HFT: alloc-free stack buffer ──────────────────────────────────
//
// Demonstrates that the decoder operates entirely on a stack-allocated
// buffer with no heap allocation.

fn bench_hft_alloc_free(c: &mut Criterion) {
    // Stack buffer containing the baseline message.
    let stack_buf = {
        let mut buf = [0u8; 1024];
        buf[..BASELINE.len()].copy_from_slice(BASELINE);
        buf
    };
    let msg_len = BASELINE.len();

    let mut group = c.benchmark_group("decode/hft/alloc_free");
    group.throughput(Throughput::Bytes(msg_len as u64));

    group.bench_function("decode_from_stack", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(&stack_buf[..msg_len])).unwrap();
            let s = car.serial_number();
            let y = car.model_year();
            black_box((s, y));
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
    bench_full_decode_unchecked,
    bench_decode_checked_vs_unchecked,
    bench_hft_tight_loop,
    bench_hft_field_stride,
    bench_hft_alloc_free,
);
criterion_main!(benches);
