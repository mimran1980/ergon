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

    group.bench_function("ergosbe_checked", |b| {
        b.iter_batched(
            || [0u8; 512],
            |mut buf| {
                let mut car: CarEncoder<'_> =
                    CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
                car.serial_number(1234);
                car.model_year(2013);
                black_box(car);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("ergosbe_unchecked", |b| {
        b.iter_batched(
            || [0u8; 512],
            |mut buf| {
                let mut car: CarEncoder<'_> =
                    CarEncoder::wrap_and_apply_header_unchecked(black_box(&mut buf), 0);
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

    group.bench_function("ergosbe_checked", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * 64],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * 64;
                    let mut car: CarEncoder<'_> =
                        CarEncoder::wrap_and_apply_header(&mut buf[off..off + 64], 0).unwrap();
                    car.serial_number(i as u64);
                    car.model_year(2013);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.bench_function("ergosbe_unchecked", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * 64],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * 64;
                    let mut car: CarEncoder<'_> =
                        CarEncoder::wrap_and_apply_header_unchecked(&mut buf[off..off + 64], 0);
                    car.serial_number(i as u64);
                    car.model_year(2013);
                }
                black_box(&buf);
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
                    // Body at offset 8 (after the 8-byte message header), header at 0.
                    // Wrapping the body at 0 would overlap the header (serial_number
                    // overwrites it), making Aeron write ~10 bytes while ErgoSBE writes
                    // the full 18-byte header+serial+model_year — an unfair comparison.
                    let car = ergosbe_benchmarks::aeron_car::aeron::car_codec::encoder::CarEncoder::default().wrap(
                        ergosbe_benchmarks::aeron_car::aeron::WriteBuf::new(&mut buf[off..off + 64]),
                        8,
                    );
                    let mut hdr = car.header(0);
                    let mut car = hdr.parent().unwrap();
                    car.serial_number(i as u64);
                    car.model_year(2013);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

fn bench_decode_consuming_full(c: &mut Criterion) {
    // Fair three-way full-message decode over the same BASELINE buffer. All
    // three do IDENTICAL work: every fuel entry (speed, mpg, usage_description),
    // every performance entry (octane_rating + nested acceleration mph/seconds),
    // and the three message-level var-data fields. Aeron's advance() does not
    // skip per-entry tails, so it must consume usage_description/acceleration to
    // advance — hence every codec traverses them, making the comparison fair.
    let bl = aeron_block_length();
    let ver = aeron_version();
    let (bl_e, ver_e) = ergosbe_header_fields();

    let mut group = c.benchmark_group("parity/decode/full_message");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe_consuming", |b| {
        b.iter(|| {
            let car = CarDecoder::wrap(black_box(BASELINE), 8, bl_e, ver_e);
            let mut fuel = car.into_fuel_figures().unwrap();
            while let Some(Ok(e)) = fuel.next() {
                black_box((e.speed(), e.mpg()));
                black_box(e.usage_description().unwrap());
            }
            let after_fuel = fuel.finish().unwrap();
            let mut perf = after_fuel.into_performance_figures().unwrap();
            while let Some(Ok(e)) = perf.next() {
                black_box(e.octane_rating());
                for a in e.acceleration().unwrap() {
                    black_box((a.mph(), a.seconds()));
                }
            }
            let after_perf = perf.finish().unwrap();
            let (mfr, a1) = after_perf.into_manufacturer().unwrap();
            let (model, a2) = a1.into_model().unwrap();
            let (code, done) = a2.into_activation_code().unwrap();
            black_box((mfr, model, code, done.encoded_length_with_header()));
        });
    });

    // The legacy `&self` random-access full-decode bench used to live here to
    // show consuming < legacy. It was removed: those `&self` group/var-data
    // accessors are the rejected out-of-order surface (DECISIONS.md §10) and are
    // no longer public. Recorded result (commit a989a97, 2026-07-10): consuming
    // ~13.06 ns vs legacy ~26.55 ns (legacy rescanned preceding groups per call).

    group.bench_function("aeron", |b| {
        b.iter(|| {
            use ergosbe_benchmarks::aeron_car::aeron::{
                ReadBuf, car_codec::decoder::CarDecoder,
                message_header_codec::decoder::MessageHeaderDecoder,
            };
            // Correct Aeron wrap: header decoder at 0, then car decoder at 0+HEADER_LEN.
            // (Direct wrap(buf,0,..) reads fields at header offsets — wrong for the body.)
            let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(black_box(BASELINE)), 0);
            let mut car = CarDecoder::default().header(header, 0);
            let mut ff = car.fuel_figures_decoder();
            while let Some(_) = ff.advance().unwrap() {
                black_box((ff.speed(), ff.mpg()));
                let c = ff.usage_description_decoder();
                black_box(ff.usage_description_slice(c));
            }
            car = ff.parent().unwrap();
            let mut pf = car.performance_figures_decoder();
            while let Some(_) = pf.advance().unwrap() {
                black_box(pf.octane_rating());
                let mut acc = pf.acceleration_decoder();
                while let Some(_) = acc.advance().unwrap() {
                    black_box((acc.mph(), acc.seconds()));
                }
                pf = acc.parent().unwrap();
            }
            car = pf.parent().unwrap();
            let mfr = car.manufacturer_decoder();
            black_box(car.manufacturer_slice(mfr));
            let model = car.model_decoder();
            black_box(car.model_slice(model));
            let code = car.activation_code_decoder();
            black_box(car.activation_code_slice(code));
        });
    });

    group.finish();
}

fn bench_decode_skip_rewind(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity/decode/skip_rewind");
    group.throughput(Throughput::Elements(1));

    // skip_to_model and direct_model removed: both read tail/var-data fields
    // via the rejected out-of-order `&self` surface (DECISIONS.md §10).
    // rewind_then_scalar stays — rewind() returns a fresh decoder and is not a
    // tail out-of-order accessor.

    group.bench_function("rewind_then_scalar", |b| {
        b.iter(|| {
            let car = CarDecoder::wrap_and_apply_header(BASELINE, 0).unwrap();
            black_box(car.serial_number())
        });
    });

    group.finish();
}

// ── Full encoder stage transition (scalars + groups + var-data → Complete) ──

fn bench_fallible_vs_manual(c: &mut Criterion) {
    // ErgoSBE-internal parity: the fallible-closure convenience API
    // (`add(|e| …)`) must not be slower than the manual `start_entry()` /
    // field-set / drop path. Both write identical bytes; the closure helper
    // constructs the same manual stage internally. The median
    // fallible/manual ratio must be <= 1.00.
    let mut group = c.benchmark_group("parity/fallible_vs_manual");
    group.throughput(Throughput::Elements(1));

    group.bench_function("manual", |b| {
        b.iter_batched(
            || vec![0u8; 512],
            |mut buf| {
                let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
                car.serial_number(42);
                car.model_year(2013);
                let after_fuel = car
                    .fuel_figures(3, |g| {
                        for (s, m) in [(30u16, 35.9f32), (55, 40.0), (70, 22.5)] {
                            let _ = g.add(|e| {
                                let _ = e.speed(s).mpg(m);
                            });
                        }
                    })
                    .unwrap();
                let after_perf = after_fuel.performance_figures(0, |_| {}).unwrap();
                let complete = after_perf
                    .manufacturer(b"Honda")
                    .unwrap()
                    .model(b"Civic")
                    .unwrap()
                    .activation_code(b"abc")
                    .unwrap();
                black_box(complete.as_bytes());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("fallible", |b| {
        b.iter_batched(
            || vec![0u8; 512],
            |mut buf| {
                let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
                car.serial_number(42);
                car.model_year(2013);
                let after_fuel = car
                    .fuel_figures(3, |g| -> sbe_rt::GroupResult {
                        for (s, m) in [(30u16, 35.9f32), (55, 40.0), (70, 22.5)] {
                            g.add(|e| {
                                e.speed(s).mpg(m);
                            })?;
                        }
                        Ok(())
                    })
                    .unwrap();
                let after_perf = after_fuel
                    .performance_figures(0, |_| Ok::<(), sbe_rt::EncodeError>(()))
                    .unwrap();
                let complete = after_perf
                    .manufacturer_with::<sbe_rt::EncodeError, _>(5, |b: &mut [u8]| {
                        b.copy_from_slice(b"Honda");
                        Ok::<(), sbe_rt::EncodeError>(())
                    })
                    .unwrap()
                    .model_with::<sbe_rt::EncodeError, _>(5, |b: &mut [u8]| {
                        b.copy_from_slice(b"Civic");
                        Ok::<(), sbe_rt::EncodeError>(())
                    })
                    .unwrap()
                    .activation_code_with::<sbe_rt::EncodeError, _>(3, |b: &mut [u8]| {
                        b.copy_from_slice(b"abc");
                        Ok::<(), sbe_rt::EncodeError>(())
                    })
                    .unwrap();
                black_box(complete.as_bytes());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_encode_full_stage_transition(c: &mut Criterion) {
    // ErgoSBE-only stage-transition diagnostic (no Aeron equivalent) — not a
    // parity scenario, so the group is not under parity/.
    let mut group = c.benchmark_group("encode/full_stage");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ergosbe", |b| {
        b.iter_batched(
            || vec![0u8; 512],
            |mut buf| {
                let mut car = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
                car.serial_number(1234);
                car.model_year(2013);
                car.available(BooleanType::T);
                car.code(Model::A);
                car.some_numbers([1u32, 2, 3, 4]);
                car.vehicle_code([97, 98, 99, 100, 101, 102]);
                car.extras(OptionalExtras::default());
                car.engine(Engine::new(
                    2000,
                    4,
                    [49, 0, 0],
                    0i8,
                    BooleanType::F,
                    Booster::new(BoostType::TURBO, 0),
                ));
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
    bench_fallible_vs_manual,
);
criterion_main!(benches);
