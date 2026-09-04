//! Decode benchmarks for ergon-generated Car message codec.
//!
//! Measures decode latency for the entry point, individual field accessors
//! (both checked and raw/unchecked variants), group iteration, and latency-oriented
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

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ergo_sbe_benchmarks::ergo_car::*;
use std::hint::black_box;

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

fn bench_group_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/group");
    group.bench_function("fuel_figures", |b| {
        b.iter(|| {
            let mut ff = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .into_fuel_figures()
                .unwrap();
            let n = ff.remaining_entries();
            let mut sum_speed: u64 = 0;
            let mut sum_mpg: f64 = 0.0;
            while let Some(Ok(entry)) = ff.next() {
                sum_speed += entry.speed() as u64;
                sum_mpg += entry.mpg() as f64;
            }
            black_box((n, sum_speed, sum_mpg));
        });
    });
    group.bench_function("fuel_figures_visit_entries", |b| {
        b.iter(|| {
            let ff = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .into_fuel_figures()
                .unwrap();
            let n = ff.remaining_entries();
            let mut sum_speed: u64 = 0;
            let mut sum_mpg: f64 = 0.0;
            let _ = ff
                .visit_entries(
                    |entry| -> Result<_, ergo_sbe_benchmarks::ergo_car::sbe_rt::DecodeError> {
                        sum_speed += entry.speed() as u64;
                        sum_mpg += entry.mpg() as f64;
                        entry.into_usage_description().map(|(_, complete)| complete)
                    },
                )
                .unwrap();
            black_box((n, sum_speed, sum_mpg));
        });
    });
    group.bench_function("fuel_figures_mutable_ordered", |b| {
        b.iter(|| {
            let mut car = CarDecoder::try_from(black_box(BASELINE)).unwrap().ordered();
            let ff = car.fuel_figures().unwrap();
            let n = ff.remaining_entries();
            let mut sum_speed: u64 = 0;
            let mut sum_mpg: f64 = 0.0;
            ff.visit_entries(|entry| -> Result<(), sbe_rt::DecodeError> {
                sum_speed += entry.speed() as u64;
                sum_mpg += entry.mpg() as f64;
                Ok(())
            })
            .unwrap();
            black_box((n, sum_speed, sum_mpg));
        });
    });
    group.finish();
}

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

fn bench_hot_tight_loop(c: &mut Criterion) {
    let batch = replicate_baseline(BATCH_SIZE);
    let msg_len = BASELINE.len();

    let mut group = c.benchmark_group("decode/hot/tight_loop");
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

fn bench_hot_field_stride(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let engine = car.engine_value();

    let mut group = c.benchmark_group("decode/hot/field_stride");
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

fn bench_hot_alloc_free(c: &mut Criterion) {
    // Stack buffer containing the baseline message.
    let stack_buf = {
        let mut buf = [0u8; 1024];
        buf[..BASELINE.len()].copy_from_slice(BASELINE);
        buf
    };
    let msg_len = BASELINE.len();

    let mut group = c.benchmark_group("decode/hot/alloc_free");
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

fn bench_decode_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/decode_frame");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));
    group.bench_function("car_known", |b| {
        b.iter(|| {
            let decoded = AnyMessage::decode_frame(black_box(BASELINE), 0, BASELINE.len()).unwrap();
            black_box(decoded);
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
            let result = cursor.skip_n(cursor.remaining());
            black_box(result);
        });
    });
    group.finish();
}

fn read_full_random(car: &CarDecoder<'_>) {
    black_box(car.serial_number());
    let _ = car.fuel_figures();
    let _ = car.performance_figures();
    black_box(car.manufacturer().unwrap());
    black_box(car.model().unwrap());
    black_box(car.activation_code().unwrap());
}

fn read_full_random_memoized(car: &CarMemoizedDecoder<'_>) {
    black_box(car.serial_number());
    let _ = car.fuel_figures();
    let _ = car.performance_figures();
    black_box(car.manufacturer().unwrap());
    black_box(car.model().unwrap());
    black_box(car.activation_code().unwrap());
}

/// Base lane vs memoized lane over identical reads.
///
/// The two arms of each pair touch the same fields the same number of times;
/// only the lane differs. `warm_final_tail` is the one pair that is not
/// symmetric by construction — the base decoder has no cache to warm, which is
/// exactly the difference being measured, so both arms still perform one
/// `activation_code()` read on an already-constructed decoder.
fn bench_tail_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode/tail_access");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));

    group.bench_function("base/construction_plus_fixed", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            black_box((car.serial_number(), car.model_year()));
        });
    });
    group.bench_function("memoized/construction_plus_fixed", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .memoized();
            black_box((car.serial_number(), car.model_year()));
        });
    });

    group.bench_function("base/cold_final_tail", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            black_box(car.activation_code().unwrap());
        });
    });
    group.bench_function("memoized/cold_final_tail", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .memoized();
            black_box(car.activation_code().unwrap());
        });
    });

    let base_warm = CarDecoder::try_from(BASELINE).unwrap();
    let memo_warm = CarDecoder::try_from(BASELINE).unwrap().memoized();
    let _ = memo_warm.activation_code();
    group.bench_function("base/warm_final_tail", |b| {
        b.iter(|| black_box(black_box(&base_warm).activation_code().unwrap()));
    });
    group.bench_function("memoized/warm_final_tail", |b| {
        b.iter(|| black_box(black_box(&memo_warm).activation_code().unwrap()));
    });

    group.bench_function("base/full_schema_order", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            read_full_random(&car);
        });
    });
    group.bench_function("memoized/full_schema_order", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .memoized();
            read_full_random_memoized(&car);
        });
    });

    group.bench_function("base/full_reverse_order", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            black_box(car.activation_code().unwrap());
            black_box(car.model().unwrap());
            black_box(car.manufacturer().unwrap());
            let _ = car.performance_figures();
            let _ = car.fuel_figures();
            black_box(car.serial_number());
        });
    });
    group.bench_function("memoized/full_reverse_order", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .memoized();
            black_box(car.activation_code().unwrap());
            black_box(car.model().unwrap());
            black_box(car.manufacturer().unwrap());
            let _ = car.performance_figures();
            let _ = car.fuel_figures();
            black_box(car.serial_number());
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
    bench_hot_tight_loop,
    bench_hot_field_stride,
    bench_hot_alloc_free,
    bench_display,
    bench_decode_frame,
    bench_skip,
    bench_tail_access,
);
criterion_main!(benches);
