//! Chrono converter benchmarks — measures conversion cost vs raw i64.
//!
//! Run with:
//!   cargo bench -p ergo-sbe-benchmarks --bench chrono_converter_bench --all-features
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::similar_names
)]

use std::hint::black_box;

fn criterion_benchmark(c: &mut criterion::Criterion) {
    let wire_ns: i64 = black_box(1_720_000_000_000_000_000); // ~2024-07
    let now_dt = chrono::DateTime::from_timestamp_nanos(wire_ns);

    // Wire → DateTime (decode path)
    c.bench_function("chrono/nanos_to_datetime", |b| {
        b.iter(|| {
            let dt = ergo_sbe::chrono_converters::i64_nanos_to_datetime(black_box(wire_ns));
            black_box(dt)
        });
    });

    // DateTime → wire (encode path)
    c.bench_function("chrono/datetime_to_nanos", |b| {
        b.iter(|| {
            let ns = ergo_sbe::chrono_converters::datetime_to_i64_nanos(black_box(now_dt));
            black_box(ns)
        });
    });

    // Wire → NaiveDateTime (decode path, micros)
    let wire_us: i64 = black_box(1_720_000_000_000_000); // ~2024-07 in micros
    c.bench_function("chrono/micros_to_naive", |b| {
        b.iter(|| {
            let dt = ergo_sbe::chrono_converters::i64_micros_to_naive(black_box(wire_us));
            black_box(dt)
        });
    });

    // NaiveDateTime → wire (encode path, micros)
    let now_naive = ergo_sbe::chrono_converters::i64_micros_to_naive(wire_us);
    c.bench_function("chrono/naive_to_micros", |b| {
        b.iter(|| {
            let us = ergo_sbe::chrono_converters::naive_to_i64_micros(black_box(now_naive));
            black_box(us)
        });
    });

    // Baseline: raw i64 pass-through
    c.bench_function("chrono/raw_i64_noop", |b| {
        b.iter(|| {
            let v = black_box(wire_ns);
            black_box(v)
        });
    });
}

criterion::criterion_group!(benches, criterion_benchmark);
criterion::criterion_main!(benches);
