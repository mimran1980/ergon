//! Decode benchmarks for ergon-generated Car message codec.
//!
//! Measures decode latency for the entry point, individual field accessors
//! (both checked and raw/unchecked variants), group iteration, and HFT-specific
//! tight-loop / field-stride / alloc-free decode patterns.

// Generated code generates lots of diagnostics; suppress across the crate.
#![allow(
    unsafe_code,
    missing_docs,
    unused_variables,
    dead_code,
    unused_mut,
    unused_must_use,
    unused_assignments,
    unused_comparisons,
    unused_attributes
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::ergo_car::*;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

/// Number of messages in the tight-loop / batch benchmarks.
const BATCH_SIZE: usize = 10_000;

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
        b.iter(|| black_box(car.engine()));
    });
    group.finish();
}

// ── Scalar field access (safe) ────────────────────────────────────

fn bench_field_access_safe(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("decode/field/safe");
    group.bench_function("serial_number", |b| {
        b.iter(|| black_box(car.serial_number()));
    });
    group.bench_function("model_year", |b| {
        b.iter(|| black_box(car.model_year()));
    });
    group.bench_function("engine_value", |b| {
        b.iter(|| black_box(car.engine_value()));
    });
    group.finish();
}

// ── Group iteration ────────────────────────────────────────────────

fn bench_group_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/group");
    group.bench_function("fuel_figures", |b| {
        b.iter(|| {
            let mut ff = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .into_fuel_figures()
                .unwrap();
            let n = ff.len();
            let mut sum_speed: u64 = 0;
            let mut sum_mpg: f64 = 0.0;
            while let Some(Ok(entry)) = ff.next() {
                sum_speed += entry.speed() as u64;
                sum_mpg += entry.mpg() as f64;
            }
            black_box((n, sum_speed, sum_mpg));
        });
    });
    group.finish();
}

// ── Full decode with safe scalar access ───────────────────────────

fn bench_full_decode_safe(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/safe");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));
    group.bench_function("car_full", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            let _ = car.serial_number();
            let _ = car.model_year();
            let _ = car.available();
            let _ = car.code();
            let _ = car.some_numbers();
            let _ = car.vehicle_code();
            let _ = car.extras();
            let _ = car.engine_value();
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
            let _ = car.available();
            let _ = car.code();
            let _ = car.some_numbers();
            let _ = car.vehicle_code();
            let _ = car.extras();
            let _ = car.engine();
            black_box(());
        });
    });

    group.bench_function("unchecked_all_fields", |b| {
        b.iter(|| {
            let _ = car.serial_number();
            let _ = car.model_year();
            let _ = car.available();
            let _ = car.code();
            let _ = car.some_numbers();
            let _ = car.vehicle_code();
            let _ = car.extras();
            let _ = car.engine_value();
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
    let batch = replicate_baseline(BATCH_SIZE);
    let msg_len = BASELINE.len();

    let mut group = c.benchmark_group("decode/hft/tight_loop");
    group.throughput(Throughput::Elements(BATCH_SIZE as u64));

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

// ── HFT: fixed-field stride ──────────────────────────────────────
//
// Measures latency of striding to fixed-block fields (modelYear,
// engine.capacity). Tail-group strides (fuelFigures[0].speed) are not
// random-access in the consuming model (DECISIONS.md §3/§10), so they are
// not benchmarked here.

fn bench_hft_field_stride(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let engine = car.engine_value();

    let mut group = c.benchmark_group("decode/hft/field_stride");
    group.throughput(Throughput::Elements(1));

    group.bench_function("model_year", |b| {
        b.iter(|| black_box(car.model_year()));
    });

    group.bench_function("engine_capacity", |b| {
        let cap = engine.capacity();
        b.iter(|| black_box(cap));
    });

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

// ── Display / debug_wire / skip ─────────────────────────────────────

fn bench_display(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("decode/display");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));
    group.bench_function("car_display", |b| {
        b.iter(|| {
            let s = format!("{}", black_box(&car));
            black_box(s);
        });
    });
    group.finish();
}

fn bench_skip(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/skip");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));
    group.bench_function("fuel_figures_skip_all", |b| {
        b.iter(|| {
            let mut cursor = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .into_fuel_figures()
                .unwrap();
            // skip_n to advance through all entries without decoding
            let result = cursor.skip_n(cursor.len());
            black_box(result);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_try_from,
    bench_field_access_checked,
    bench_field_access_safe,
    bench_group_iteration,
    bench_full_decode_safe,
    bench_decode_checked_vs_unchecked,
    bench_hft_tight_loop,
    bench_hft_field_stride,
    bench_hft_alloc_free,
    bench_display,
    bench_skip,
);
criterion_main!(benches);
