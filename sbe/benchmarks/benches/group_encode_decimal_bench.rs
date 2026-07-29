//! Group encode with rust_decimal::Decimal converters: wire vs domain vs bulk_add.
//! Tests whether bulk_add's hoisted bounds checks show benefit when the
//! compiler can't constant-fold through Decimal conversion.
//! 50 samples, 1s warm-up, 3s measurement.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::l2book::*;

fn make_entries(n: usize) -> Vec<LevelsEntry> {
    (0..n).map(|i| LevelsEntry {
        price: Decimal::new((i as i64) * 100),
        qty: Decimal::new((i as i64) * 10),
        side: (i % 2) as u8,
    }).collect()
}

fn make_decimal_entries(n: usize) -> Vec<rust_decimal::Decimal> {
    (0..n).map(|i| {
        rust_decimal::Decimal::new((i as i64) * 100, 2)
    }).collect()
}

fn bench_group_encode_decimal(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_encode_decimal");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let dec_prices = make_decimal_entries(n);
        let dec_qtys: Vec<rust_decimal::Decimal> = (0..n)
            .map(|i| rust_decimal::Decimal::new((i as i64) * 10, 2))
            .collect();
        let msg_len = L2BookEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        group.throughput(Throughput::Elements(n as u64));

        // ── length parity ──
        {
            let mut ebuf = vec![0u8; msg_len];
            let elen = L2BookEncoder::wrap_and_apply_header(&mut ebuf, 0)
                .levels(n as u16, |g| {
                    for e in &entries { g.add(|entry| { entry.price_wire(e.price).qty_wire(e.qty).side(e.side); Ok(()) })?; }
                    Ok(())
                }).unwrap().encoded_length_with_header();
            assert_eq!(elen, msg_len, "wire length mismatch at n={n}");
        }

        // 1. add_closure with wire types (baseline — no conversion overhead)
        group.bench_with_input(BenchmarkId::new("add_closure_wire", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| {
                        for e in entries {
                            g.add(|entry| {
                                entry.price_wire(e.price).qty_wire(e.qty).side(e.side);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    }).unwrap().encoded_length_with_header()
            ));
        });

        // 2. add_closure with domain type converters (rust_decimal::Decimal)
        group.bench_with_input(BenchmarkId::new("add_closure_domain", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| {
                        for (i, e) in entries.iter().enumerate() {
                            g.add(|entry| {
                                let p: rust_decimal::Decimal = rust_decimal::Decimal::new((i as i64) * 100, 2);
                                let q: rust_decimal::Decimal = rust_decimal::Decimal::new((i as i64) * 10, 2);
                                entry.price(p).qty(q).side(e.side);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    }).unwrap().encoded_length_with_header()
            ));
        });

        // 3. add_struct with wire entry structs
        group.bench_with_input(BenchmarkId::new("add_struct", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| {
                        for e in entries { g.add_struct(e)?; }
                        Ok(())
                    }).unwrap().encoded_length_with_header()
            ));
        });

    }
    group.finish();
}

criterion_group!(benches, bench_group_encode_decimal);
criterion_main!(benches);
