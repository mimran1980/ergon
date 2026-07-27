//! L2 orderbook benchmark with rust_decimal converters.
//!
//! Measures encode/decode throughput for an L2 book with Decimal price/qty
//! fields (SBE Decimal with constant exponent -2). Compares closure add vs
//! bulk_add for group encoding, and measures converter overhead on decode
//! (rust_decimal::Decimal vs raw wire access).

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::l2book::*;
use rust_decimal::Decimal as RustDecimal;

fn make_decimal(mantissa: i64) -> RustDecimal {
    RustDecimal::from_i128_with_scale(mantissa as i128, 2)
}

fn make_entries(n: usize) -> Vec<LevelsEntry> {
    (0..n)
        .map(|i| LevelsEntry {
            price: Decimal::new((i as i64 + 1) * 10000),
            qty: Decimal::new((i as i64 + 1) * 100),
            side: (i % 2) as u8,
        })
        .collect()
}

fn bench_l2_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_book/encode");
    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let msg_len =
            L2BookEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();

        group.throughput(Throughput::Elements(n as u64));
        group.sample_size(20);
        group.warm_up_time(std::time::Duration::from_millis(250));
        group.measurement_time(std::time::Duration::from_millis(500));

        // Closure add with rust_decimal domain_type setters (converter per entry)
        group.bench_with_input(
            BenchmarkId::new("add_closure_decimal", n),
            &entries,
            |b, entries| {
                let mut buf = vec![0u8; msg_len];
                b.iter(|| {
                    black_box(
                        L2BookEncoder::try_wrap_and_apply_header(black_box(&mut buf), 0)
                            .unwrap()
                            .levels(n as u16, |g| {
                                for e in entries {
                                    g.add(|entry| {
                                        entry
                                            .price(black_box(make_decimal(e.price.mantissa())))
                                            .qty(black_box(make_decimal(e.qty.mantissa())))
                                            .side(black_box(e.side));
                                        Ok(())
                                    })?;
                                }
                                Ok(())
                            })
                            .unwrap()
                            .encoded_length_with_header(),
                    )
                });
            },
        );

        // bulk_add with wire Decimal values (no per-entry converter)
        group.bench_with_input(
            BenchmarkId::new("bulk_add_wire", n),
            &n,
            |b, &n| {
                let entries = make_entries(n);
                let msg_len = L2BookEncoder::try_compute_encoded_length_with_header(
                    n as u16,
                )
                .unwrap();
                let mut buf = vec![0u8; msg_len];
                b.iter(|| {
                    black_box(
                        L2BookEncoder::try_wrap_and_apply_header(black_box(&mut buf), 0)
                            .unwrap()
                            .levels(n as u16, |g| {
                                g.bulk_add(black_box(&entries))?;
                                Ok(())
                            })
                            .unwrap()
                            .encoded_length_with_header(),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_l2_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_book/decode");
    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let msg_len =
            L2BookEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        let mut buf = vec![0u8; msg_len];
        let written = L2BookEncoder::try_wrap_and_apply_header(&mut buf, 0)
            .unwrap()
            .levels(n as u16, |g| {
                g.bulk_add(&entries)?;
                Ok(())
            })
            .unwrap()
            .encoded_length_with_header();
        assert_eq!(written, msg_len);

        group.throughput(Throughput::Elements(n as u64));
        group.sample_size(20);
        group.warm_up_time(std::time::Duration::from_millis(250));
        group.measurement_time(std::time::Duration::from_millis(500));

        // Decode with rust_decimal converter (price() returns RustDecimal)
        group.bench_with_input(
            BenchmarkId::new("decode_decimal", n),
            &buf,
            |b, buf| {
                b.iter(|| {
                    let dec = L2BookDecoder::try_from(black_box(&buf[..written])).unwrap();
                    let levels = dec.into_levels().unwrap();
                    let mut total: i128 = 0;
                    for level in levels {
                        let price: RustDecimal = black_box(level.price());
                        total = total.wrapping_add(price.mantissa() as i128);
                        let qty: RustDecimal = black_box(level.qty());
                        total = total.wrapping_add(qty.mantissa() as i128);
                        black_box(level.side());
                    }
                    black_box(total)
                });
            },
        );

        // Decode raw wire values (no converter overhead)
        group.bench_with_input(
            BenchmarkId::new("decode_wire", n),
            &buf,
            |b, buf| {
                b.iter(|| {
                    let dec = L2BookDecoder::try_from(black_box(&buf[..written])).unwrap();
                    let levels = dec.into_levels().unwrap();
                    let mut total: i64 = 0;
                    for level in levels {
                        let price_wire = black_box(level.price_value());
                        total = total.wrapping_add(price_wire.mantissa());
                        let qty_wire = black_box(level.qty_value());
                        total = total.wrapping_add(qty_wire.mantissa());
                        black_box(level.side());
                    }
                    black_box(total)
                });
            },
        );

        // bulk_decode — one bounds check, returns Vec<LevelsEntry>
        group.bench_with_input(
            BenchmarkId::new("bulk_decode_wire", n),
            &n,
            |b, &n| {
                b.iter(|| {
                    let dec = L2BookDecoder::try_from(black_box(&buf[..written])).unwrap();
                    let mut levels = dec.into_levels().unwrap();
                    let entries: Vec<LevelsEntry> =
                        black_box(levels.bulk_decode().unwrap());
                    let mut total: i64 = 0;
                    for e in &entries {
                        total = total.wrapping_add(e.price.mantissa());
                        total = total.wrapping_add(e.qty.mantissa());
                    }
                    black_box((entries.len(), total))
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_l2_encode, bench_l2_decode);
criterion_main!(benches);
