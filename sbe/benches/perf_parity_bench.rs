//! Performance parity: ErgoSBE vs Aeron Rust SBE head-to-head.
//!
//! Both codecs generated from the same Car schema, decoding the same
//! Java-produced binary fixture. If ErgoSBE is slower in any scenario,
//! that is a blocking v1 release bug (todo 105).
//!
//! Note: Aeron SBE uses a different API pattern (mutable self, parent
//! references, advance()-based group iteration). These benchmarks compare
//! semantically equivalent operations — same fields, same buffer, same count.

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

// ErgoSBE generated code
include!("generated/car_patched.rs");

// Aeron Rust SBE generated code (patched for module inclusion)
mod aeron_code {
    #![allow(
        non_camel_case_types,
        non_snake_case,
        clippy::all,
        ambiguous_glob_reexports,
        unused_imports,
        dead_code
    )]
    include!("generated/aeron_car_patched.rs");
}

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

#[path = "_common.rs"]
mod common;
use common::BASELINE;

// Header bytes for Aeron decoder construction
fn aeron_block_length() -> u16 {
    u16::from_le_bytes(BASELINE[0..2].try_into().unwrap())
}
fn aeron_version() -> u16 {
    u16::from_le_bytes(BASELINE[6..8].try_into().unwrap())
}

// ── Decode: entry point (wrap/try_from) ──────────────────────────────

fn bench_decode_entry_point(c: &mut Criterion) {
    let bl = aeron_block_length();
    let ver = aeron_version();

    let mut group = c.benchmark_group("parity/decode/entry_point");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));

    group.bench_function("ergosbe_try_from", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            black_box(car);
        });
    });

    group.bench_function("aeron_wrap", |b| {
        b.iter(|| {
            let car = aeron_code::aeron::car_codec::decoder::CarDecoder::default().wrap(
                black_box(aeron_code::aeron::ReadBuf::new(BASELINE)),
                0,
                bl,
                ver,
            );
            black_box(car);
        });
    });

    group.finish();
}

// ── Decode: scalar field access (serial_number + model_year) ─────────

fn bench_decode_scalar(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let bl = aeron_block_length();
    let ver = aeron_version();
    let aero_car = aeron_code::aeron::car_codec::decoder::CarDecoder::default().wrap(
        aeron_code::aeron::ReadBuf::new(BASELINE),
        0,
        bl,
        ver,
    );

    let mut group = c.benchmark_group("parity/decode/scalar");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe", |b| {
        b.iter(|| {
            let sn = car.serial_number();
            let my = car.model_year();
            black_box((sn, my));
        });
    });

    group.bench_function("aeron", |b| {
        b.iter(|| {
            let sn = aero_car.serial_number();
            let my = aero_car.model_year();
            black_box((sn, my));
        });
    });

    group.finish();
}

// ── Decode: array field (some_numbers: [u32; 4]) ─────────────────────

fn bench_decode_array(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let bl = aeron_block_length();
    let ver = aeron_version();
    let aero_car = aeron_code::aeron::car_codec::decoder::CarDecoder::default().wrap(
        aeron_code::aeron::ReadBuf::new(BASELINE),
        0,
        bl,
        ver,
    );

    let mut group = c.benchmark_group("parity/decode/array");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe", |b| {
        b.iter(|| {
            let sn = car.some_numbers().unwrap();
            black_box(sn);
        });
    });

    group.bench_function("aeron", |b| {
        b.iter(|| {
            let sn = aero_car.some_numbers();
            black_box(sn);
        });
    });

    group.finish();
}

// ── Decode: composite (Engine) — ErgoSBE copies eagerly, Aeron flyweight ──

fn bench_decode_composite(c: &mut Criterion) {
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let mut group = c.benchmark_group("parity/decode/composite");
    group.throughput(Throughput::Elements(1));

    // ErgoSBE: eager copy of 6 bytes into value struct
    group.bench_function("ergosbe_engine", |b| {
        b.iter(|| {
            let engine = car.engine(); // Engine value struct (Copy, 6 bytes)
            let cap = engine.capacity();
            let cyl = engine.num_cylinders();
            black_box((cap, cyl));
        });
    });

    // Aeron: flyweight decoder (parent reference, no copy)
    group.bench_function("aeron_engine", |b| {
        b.iter(|| {
            let bl = aeron_block_length();
            let ver = aeron_version();
            let aero_car = aeron_code::aeron::car_codec::decoder::CarDecoder::default().wrap(
                aeron_code::aeron::ReadBuf::new(BASELINE),
                0,
                bl,
                ver,
            );
            let engine = aero_car.engine_decoder();
            let cap = engine.capacity();
            let cyl = engine.num_cylinders();
            black_box((cap, cyl));
        });
    });

    group.finish();
}

// ── HFT batch decode throughput ──────────────────────────────────────

const HFT_BATCH: usize = 10_000;

fn replicate_baseline(count: usize) -> Vec<u8> {
    let msg_len = BASELINE.len();
    let mut buf = Vec::with_capacity(count * msg_len);
    unsafe { buf.set_len(count * msg_len) };
    for chunk in buf.chunks_mut(msg_len) {
        chunk.copy_from_slice(BASELINE);
    }
    buf
}

fn bench_throughput_batch(c: &mut Criterion) {
    let buf = replicate_baseline(HFT_BATCH);
    let msg_len = BASELINE.len();
    let bl = aeron_block_length();
    let ver = aeron_version();

    let mut group = c.benchmark_group("parity/throughput/batch_10k");
    group.throughput(Throughput::Elements(HFT_BATCH as u64));

    group.bench_function("ergosbe", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            let mut total_year: u64 = 0;
            let mut off = 0;
            for _ in 0..HFT_BATCH {
                let car = CarDecoder::try_from(&buf[off..off + msg_len]).unwrap();
                total += car.serial_number();
                total_year += car.model_year() as u64;
                off += msg_len;
            }
            black_box((total, total_year));
        });
    });

    group.bench_function("aeron", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            let mut total_year: u64 = 0;
            let mut off = 0;
            for _ in 0..HFT_BATCH {
                let car = aeron_code::aeron::car_codec::decoder::CarDecoder::default().wrap(
                    aeron_code::aeron::ReadBuf::new(&buf[off..off + msg_len]),
                    0,
                    bl,
                    ver,
                );
                total += car.serial_number() as u64;
                total_year += car.model_year() as u64;
                off += msg_len;
            }
            black_box((total, total_year));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_decode_entry_point,
    bench_decode_scalar,
    bench_decode_array,
    bench_decode_composite,
    bench_throughput_batch,
);
criterion_main!(benches);
