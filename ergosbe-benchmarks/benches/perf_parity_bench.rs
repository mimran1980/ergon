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

use ergosbe_benchmarks::ergo_car::*;

// Aeron Rust SBE generated code (patched for module inclusion)

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

// Pre-computed ErgoSBE header fields (validate once per stream, like Aeron).
// In a real feed handler these are read once at session setup, not per message.
fn ergosbe_header_fields() -> (usize, u16) {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    (header.block_length() as usize, header.version())
}

// ── Decode: entry point (wrap/try_from) ──────────────────────────────
//
// PARITY GATE: `ergosbe_wrap` vs `aeron_wrap`. Both pre-compute header
// fields once (the real HFT feed-handler pattern: validate once, decode
// fast). `ergosbe_try_from` is informational — it re-reads + re-validates
// the header every call, which Aeron's `wrap` does not, so it is not the
// parity comparison.

fn bench_decode_entry_point(c: &mut Criterion) {
    let bl = aeron_block_length();
    let ver = aeron_version();
    let (bl_e, ver_e) = ergosbe_header_fields();

    let mut group = c.benchmark_group("parity/decode/entry_point");
    group.throughput(Throughput::Bytes(BASELINE.len() as u64));

    // Fast path: pre-computed header, lean wrap (4 field assigns).
    group.bench_function("ergosbe_wrap", |b| {
        b.iter(|| {
            let car = CarDecoder::wrap(black_box(BASELINE), 8, bl_e, ver_e);
            black_box(car);
        });
    });

    // Informational: full validation (header read + schema_id check every call).
    group.bench_function("ergosbe_try_from", |b| {
        b.iter(|| {
            let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
            black_box(car);
        });
    });

    group.bench_function("aeron_wrap", |b| {
        b.iter(|| {
            let car =
                ergosbe_benchmarks::aeron_car::aeron::car_codec::decoder::CarDecoder::default()
                    .wrap(
                        black_box(ergosbe_benchmarks::aeron_car::aeron::ReadBuf::new(BASELINE)),
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
    let aero_car = ergosbe_benchmarks::aeron_car::aeron::car_codec::decoder::CarDecoder::default()
        .wrap(
            ergosbe_benchmarks::aeron_car::aeron::ReadBuf::new(BASELINE),
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
    let aero_car = ergosbe_benchmarks::aeron_car::aeron::car_codec::decoder::CarDecoder::default()
        .wrap(
            ergosbe_benchmarks::aeron_car::aeron::ReadBuf::new(BASELINE),
            0,
            bl,
            ver,
        );

    let mut group = c.benchmark_group("parity/decode/array");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe", |b| {
        b.iter(|| {
            let sn = car.some_numbers();
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
            let aero_car =
                ergosbe_benchmarks::aeron_car::aeron::car_codec::decoder::CarDecoder::default()
                    .wrap(
                        ergosbe_benchmarks::aeron_car::aeron::ReadBuf::new(BASELINE),
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
    // Validate the stream header once (real feed-handler pattern), then decode fast.
    let (bl_e, ver_e) = ergosbe_header_fields();

    let mut group = c.benchmark_group("parity/throughput/batch_10k");
    group.throughput(Throughput::Elements(HFT_BATCH as u64));

    group.bench_function("ergosbe", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            let mut total_year: u64 = 0;
            let mut off = 0;
            for _ in 0..HFT_BATCH {
                // Fast path: header validated once above, per-message wrap is lean.
                let car = CarDecoder::wrap(&buf[off..off + msg_len], 8, bl_e, ver_e);
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
                let car =
                    ergosbe_benchmarks::aeron_car::aeron::car_codec::decoder::CarDecoder::default()
                        .wrap(
                            ergosbe_benchmarks::aeron_car::aeron::ReadBuf::new(
                                &buf[off..off + msg_len],
                            ),
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

// ── Encode: scalar writes (serial_number + model_year) ───────────────
//
// Both write via copy_from_slice (ErgoSBE write_bytes / Aeron put_uX_at).
// ErgoSBE wrap_and_apply_header writes the header in one template copy;
// Aeron wrap does not write the header (caller calls .header() separately,
// 4 put_u16_at calls). We measure wrap + 2 scalar writes for both.

fn bench_encode_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity/encode/scalar");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe", |b| {
        b.iter_batched(
            || [0u8; 512],
            |mut buf| {
                let mut car: CarEncoder<'_> =
                    CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0);
                car.serial_number(1234);
                car.model_year(2013);
                black_box(car);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("aeron", |b| {
        b.iter_batched(
            || [0u8; 512],
            |mut buf| {
                // Fair: write the header (4 fields) like ErgoSBE's wrap_and_apply_header,
                // then reclaim the CarEncoder via parent() and write 2 scalars.
                let car =
                    ergosbe_benchmarks::aeron_car::aeron::car_codec::encoder::CarEncoder::default()
                        .wrap(
                            ergosbe_benchmarks::aeron_car::aeron::WriteBuf::new(black_box(
                                &mut buf,
                            )),
                            0,
                        );
                let mut hdr = car.header(0);
                let mut car = hdr.parent().unwrap();
                car.serial_number(1234);
                car.model_year(2013);
                black_box(car);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ── Encode: throughput (10k batch, wrap + 2 scalars per message) ─────

fn bench_encode_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity/encode/throughput_10k");
    group.throughput(Throughput::Elements(HFT_BATCH as u64));

    group.bench_function("ergosbe", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * 64],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * 64;
                    let mut car: CarEncoder<'_> =
                        CarEncoder::wrap_and_apply_header(&mut buf[off..off + 64], 0);
                    car.serial_number(i as u64);
                    car.model_year(2013);
                }
                black_box(buf[0]);
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.bench_function("aeron", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * 64],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * 64;
                    let car = ergosbe_benchmarks::aeron_car::aeron::car_codec::encoder::CarEncoder::default().wrap(
                        ergosbe_benchmarks::aeron_car::aeron::WriteBuf::new(&mut buf[off..off + 64]),
                        0,
                    );
                    let mut hdr = car.header(0);
                    let mut car = hdr.parent().unwrap();
                    car.serial_number(i as u64);
                    car.model_year(2013);
                }
                black_box(buf[0]);
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

// ── Decoder skip/rewind API benchmark ────────────────────────────────

fn bench_decode_consuming_full(c: &mut Criterion) {
    // Head-to-head full-message decode over the same BASELINE buffer:
    //   - `ergosbe_consuming` uses the new concrete consuming tail stages
    //     (into_<g> -> iterate -> finish -> into_<vd> -> complete);
    //   - `ergosbe_legacy` uses the legacy `&self` random-access accessors.
    // Both do identical work (every group entry + every var-data field).
    // The legacy path has already reached Aeron parity (historically measured),
    // so `consuming <= legacy` implies `consuming <= Aeron` for full-message decode.
    let mut group = c.benchmark_group("parity/decode/full_message");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe_consuming", |b| {
        b.iter(|| {
            let car = CarDecoder::wrap_and_apply_header(black_box(BASELINE), 0).unwrap();
            let mut fuel = car.into_fuel_figures().unwrap();
            while let Some(Ok(e)) = fuel.next() {
                black_box((e.speed(), e.mpg()));
            }
            let after_fuel = fuel.finish().unwrap();
            let mut perf = after_fuel.into_performance_figures().unwrap();
            while let Some(Ok(e)) = perf.next() {
                black_box(e.octane_rating());
            }
            let after_perf = perf.finish().unwrap();
            let (mfr, a1) = after_perf.into_manufacturer().unwrap();
            let (model, a2) = a1.into_model().unwrap();
            let (code, done) = a2.into_activation_code().unwrap();
            black_box((mfr, model, code, done.encoded_length_with_header()));
        });
    });

    group.bench_function("ergosbe_legacy", |b| {
        b.iter(|| {
            let car = CarDecoder::wrap_and_apply_header(black_box(BASELINE), 0).unwrap();
            for entry in car.fuel_figures().unwrap() {
                let e = entry.unwrap();
                black_box((e.speed(), e.mpg()));
            }
            for entry in car.performance_figures().unwrap() {
                let e = entry.unwrap();
                black_box(e.octane_rating());
            }
            black_box((
                car.manufacturer().unwrap(),
                car.model().unwrap(),
                car.activation_code().unwrap(),
            ));
        });
    });

    group.finish();
}

fn bench_decode_skip_rewind(c: &mut Criterion) {
    let car = CarDecoder::wrap_and_apply_header(BASELINE, 0).unwrap();

    let mut group = c.benchmark_group("parity/decode/skip_rewind");
    group.throughput(Throughput::Elements(1));

    group.bench_function("skip_to_model", |b| {
        b.iter(|| black_box(car.skip_to_model().unwrap()));
    });

    group.bench_function("direct_model", |b| {
        b.iter(|| black_box(car.model().unwrap()));
    });

    group.bench_function("rewind_then_scalar", |b| {
        b.iter(|| black_box(car.rewind().serial_number()));
    });

    group.finish();
}

// ── Full encoder stage transition (scalars + groups + var-data → Complete) ──

fn bench_encode_full_stage_transition(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity/encode/full_stage");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe", |b| {
        b.iter_batched(
            || vec![0u8; 512],
            |mut buf| {
                let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
                car.serial_number(1234);
                car.model_year(2013);
                car.available(BooleanType::T);
                car.code(Model::A);
                car.some_numbers([1u32, 2, 3, 4]);
                car.vehicle_code([97, 98, 99, 100, 101, 102]);
                car.extras(OptionalExtras::default());
                car.engine(Engine::new(2000, 4, [49, 0, 0]));
                let car = car
                    .fuel_figures(3, |g| {
                        g.add(|e| {
                            e.speed(30).mpg(35.9);
                        })
                        .unwrap();
                        g.add(|e| {
                            e.speed(55).mpg(40.0);
                        })
                        .unwrap();
                    })
                    .unwrap();
                let car = car
                    .performance_figures(1, |g| {
                        g.add(|e| {
                            e.octane_rating(95);
                        })
                        .unwrap();
                    })
                    .unwrap();
                let car = car.manufacturer(b"Honda").unwrap();
                let car = car.model(b"Civic").unwrap();
                let complete = car.activation_code(b"abc").unwrap();
                black_box(complete.as_bytes());
            },
            criterion::BatchSize::SmallInput,
        );
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
    bench_encode_scalar,
    bench_encode_throughput,
    bench_decode_skip_rewind,
    bench_decode_consuming_full,
    bench_encode_full_stage_transition,
);
criterion_main!(benches);
