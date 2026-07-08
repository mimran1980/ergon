//! Throughput benchmarks for ErgoSBE-generated Car message codec.
//!
//! Measures decode throughput in millions of messages per second across
//! varying batch sizes.  Compares the checked decode path against a
//! hand-written unsafe raw-decode loop using pointer arithmetic.

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

include!("generated/car_patched.rs");

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

#[path = "_common.rs"]
mod common;
use common::BASELINE;

/// Batch sizes for throughput sweeps.
const BATCH_SIZES: &[usize] = &[1, 10, 100, 1_000, 10_000];

// ── Pre-allocated multi-message buffer ──────────────────────────────

fn build_batch_buffer(count: usize) -> Vec<u8> {
    let msg_len = BASELINE.len();
    let mut buf = Vec::with_capacity(count * msg_len);
    // SAFETY: we immediately fill the capacity.
    unsafe { buf.set_len(count * msg_len) };
    for chunk in buf.chunks_mut(msg_len) {
        chunk.copy_from_slice(BASELINE);
    }
    buf
}

// ── Throughput: checked decode + field access ──────────────────────
//
// For each batch size: decode every message and read a few key fields.

fn bench_throughput_checked(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput/checked");

    for &n in BATCH_SIZES {
        let buf = build_batch_buffer(n);
        let label = format!("{}_messages", n);

        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&label, |b| {
            b.iter(|| {
                let mut total_serial: u64 = 0;
                let mut total_year: u64 = 0;
                let mut total_capacity: u64 = 0;
                let mut off = 0usize;
                for _ in 0..n {
                    let car =
                        CarDecoder::try_from(black_box(&buf[off..off + BASELINE.len()])).unwrap();
                    total_serial += car.serial_number();
                    total_year += car.model_year() as u64;
                    let engine = car.engine();
                    total_capacity += engine.capacity() as u64;
                    off += BASELINE.len();
                }
                black_box((total_serial, total_year, total_capacity));
            });
        });
    }

    group.finish();
}

// ── Throughput: raw/unchecked decode + field access ────────────────

fn bench_throughput_unchecked(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput/unchecked");

    for &n in BATCH_SIZES {
        let buf = build_batch_buffer(n);
        let label = format!("{}_messages", n);

        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&label, |b| {
            b.iter(|| {
                let mut total_serial: u64 = 0;
                let mut total_year: u64 = 0;
                let mut total_capacity: u64 = 0;
                let mut off = 0usize;
                for _ in 0..n {
                    let car =
                        CarDecoder::try_from(black_box(&buf[off..off + BASELINE.len()])).unwrap();
                    total_serial += car.serial_number();
                    total_year += car.model_year() as u64;
                    let engine = car.engine_as_struct();
                    total_capacity += engine.capacity() as u64;
                    off += BASELINE.len();
                }
                black_box((total_serial, total_year, total_capacity));
            });
        });
    }

    group.finish();
}

// ── Hand-written unsafe raw-decode loop ────────────────────────────
//
// Direct byte reads from the buffer using pointer arithmetic.
// No struct allocation, no trait dispatch — just raw bytes.
// Mirrors what a latency-optimised C++ feed handler might do.

mod raw_decode {
    use criterion::black_box;

    #[inline(always)]
    unsafe fn read_u16_le(ptr: *const u8) -> u16 {
        u16::from_le_bytes(unsafe { *(ptr as *const [u8; 2]) })
    }

    #[inline(always)]
    unsafe fn read_u64_le(ptr: *const u8) -> u64 {
        u64::from_le_bytes(unsafe { *(ptr as *const [u8; 8]) })
    }

    /// Raw-decode one Car message from `buf` at absolute offset `off`.
    ///
    /// Offsets are schema-derived:
    ///
    /// | Field              | Body offset | Size | Type |
    /// |--------------------|-------------|------|------|
    /// | serialNumber       | 0           | 8    | u64  |
    /// | modelYear          | 8           | 2    | u16  |
    /// | engine.capacity    | 35          | 2    | u16  |
    ///
    /// # Safety
    ///
    /// `off + message_len` must be within `buf.len()`. Callers guarantee this
    /// by striding in message-length increments over a buffer built from
    /// whole copies of the baseline fixture.
    pub unsafe fn decode_one(buf: &[u8], off: usize) -> (u64, u16, u16) {
        unsafe {
            let base = buf.as_ptr().add(off);
            // SBE header (8 bytes), body starts after header.
            let body = base.add(8);
            let serial_number = read_u64_le(body);
            let model_year = read_u16_le(body.add(8));
            let engine_capacity = read_u16_le(body.add(35));
            black_box((serial_number, model_year, engine_capacity))
        }
    }
}

fn bench_throughput_raw(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput/raw_unsafe");

    for &n in BATCH_SIZES {
        let buf = build_batch_buffer(n);
        let label = format!("{}_messages", n);

        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(&label, |b| {
            b.iter(|| {
                let mut total_serial: u64 = 0;
                let mut total_year: u64 = 0;
                let mut total_capacity: u64 = 0;
                let mut off = 0usize;
                for _ in 0..n {
                    let (s, y, c) = unsafe { raw_decode::decode_one(black_box(&buf), off) };
                    total_serial += s;
                    total_year += y as u64;
                    total_capacity += c as u64;
                    off += BASELINE.len();
                }
                black_box((total_serial, total_year, total_capacity));
            });
        });
    }

    group.finish();
}

// ── Throughput comparison (n=1000) ─────────────────────────────────

fn bench_throughput_comparison(c: &mut Criterion) {
    const COMP_N: usize = 1_000;
    let buf = build_batch_buffer(COMP_N);
    let mut group = c.benchmark_group("throughput/comparison");
    group.throughput(Throughput::Elements(COMP_N as u64));

    group.bench_function("checked", |b| {
        b.iter(|| {
            let mut total_serial: u64 = 0;
            let mut total_year: u64 = 0;
            let mut total_capacity: u64 = 0;
            let mut off = 0usize;
            for _ in 0..COMP_N {
                let car = CarDecoder::try_from(black_box(&buf[off..off + BASELINE.len()])).unwrap();
                total_serial += car.serial_number();
                total_year += car.model_year() as u64;
                let engine = car.engine();
                total_capacity += engine.capacity() as u64;
                off += BASELINE.len();
            }
            black_box((total_serial, total_year, total_capacity));
        });
    });

    group.bench_function("unchecked", |b| {
        b.iter(|| {
            let mut total_serial: u64 = 0;
            let mut total_year: u64 = 0;
            let mut total_capacity: u64 = 0;
            let mut off = 0usize;
            for _ in 0..COMP_N {
                let car = CarDecoder::try_from(black_box(&buf[off..off + BASELINE.len()])).unwrap();
                total_serial += car.serial_number();
                total_year += car.model_year() as u64;
                let engine = car.engine_as_struct();
                total_capacity += engine.capacity() as u64;
                off += BASELINE.len();
            }
            black_box((total_serial, total_year, total_capacity));
        });
    });

    group.bench_function("raw_unsafe", |b| {
        b.iter(|| {
            let mut total_serial: u64 = 0;
            let mut total_year: u64 = 0;
            let mut total_capacity: u64 = 0;
            let mut off = 0usize;
            for _ in 0..COMP_N {
                let (s, y, c) = unsafe { raw_decode::decode_one(black_box(&buf), off) };
                total_serial += s;
                total_year += y as u64;
                total_capacity += c as u64;
                off += BASELINE.len();
            }
            black_box((total_serial, total_year, total_capacity));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_throughput_checked,
    bench_throughput_unchecked,
    bench_throughput_raw,
    bench_throughput_comparison,
);
criterion_main!(benches);
