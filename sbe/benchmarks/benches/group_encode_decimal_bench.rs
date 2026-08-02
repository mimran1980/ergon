//! Group encode with rust_decimal::Decimal converters.
//!
//! Domain inputs are constructed outside the timed path. The domain result,
//! wire closure result, struct result, and bulk result are byte-identical.
//! 50 samples, 1s warm-up, 3s measurement.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ergo_sbe_benchmarks::l2book::*;
use std::hint::black_box;

fn make_entries(n: usize) -> Vec<LevelsEntry> {
    (0..n)
        .map(|i| LevelsEntry {
            price: Decimal::new((i as i64) * 100),
            qty: Decimal::new((i as i64) * 10),
            side: (i % 2) as u8,
        })
        .collect()
}

struct DomainLevelsEntry {
    price: rust_decimal::Decimal,
    qty: rust_decimal::Decimal,
    side: u8,
}

fn make_domain_entries(n: usize) -> Vec<DomainLevelsEntry> {
    (0..n)
        .map(|i| DomainLevelsEntry {
            price: rust_decimal::Decimal::new((i as i64) * 100, 2),
            qty: rust_decimal::Decimal::new((i as i64) * 10, 2),
            side: (i % 2) as u8,
        })
        .collect()
}

fn bench_group_encode_decimal(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_encode_decimal");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let domain_entries = make_domain_entries(n);
        let msg_len = L2BookEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        group.throughput(Throughput::Elements(n as u64));

        // ── wire parity across all ergon entry APIs ──
        {
            let mut wire_buf = vec![0u8; msg_len];
            let wire_len = L2BookEncoder::wrap_and_apply_header(&mut wire_buf, 0)
                .levels(n as u16, |g| {
                    for e in &entries {
                        g.add(|entry| {
                            entry.price_wire(e.price).qty_wire(e.qty).side(e.side);
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
                .unwrap()
                .encoded_length_with_header();
            assert_eq!(wire_len, msg_len, "wire length mismatch at n={n}");

            let mut domain_buf = vec![0u8; msg_len];
            let domain_len = L2BookEncoder::wrap_and_apply_header(&mut domain_buf, 0)
                .levels(n as u16, |g| {
                    for e in &domain_entries {
                        g.add(|entry| {
                            entry
                                .try_price(e.price)
                                .unwrap()
                                .try_qty(e.qty)
                                .unwrap()
                                .side(e.side);
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
                .unwrap()
                .encoded_length_with_header();
            assert_eq!(domain_len, wire_len, "domain length mismatch at n={n}");
            assert_eq!(domain_buf, wire_buf, "domain bytes mismatch at n={n}");

            let mut bulk_buf = vec![0u8; msg_len];
            let bulk_len = L2BookEncoder::wrap_and_apply_header(&mut bulk_buf, 0)
                .levels(n as u16, |group| group.bulk_add(&entries))
                .unwrap()
                .encoded_length_with_header();
            assert_eq!(bulk_len, wire_len, "bulk length mismatch at n={n}");
            assert_eq!(bulk_buf, wire_buf, "bulk bytes mismatch at n={n}");
        }

        // 1. add_closure with wire types (baseline — no conversion overhead)
        group.bench_with_input(
            BenchmarkId::new("add_closure_wire", n),
            &entries,
            |b, entries| {
                let mut buf = vec![0u8; msg_len];
                b.iter(|| {
                    let entries = black_box(entries);
                    let len = L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                        .levels(n as u16, |g| {
                            for e in entries {
                                g.add(|entry| {
                                    entry.price_wire(e.price).qty_wire(e.qty).side(e.side);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })
                        .unwrap()
                        .encoded_length_with_header();
                    black_box(&buf[..len]);
                    black_box(len)
                });
            },
        );

        // 2. add_closure with domain type converters (rust_decimal::Decimal)
        group.bench_with_input(
            BenchmarkId::new("add_closure_domain", n),
            &domain_entries,
            |b, entries| {
                let mut buf = vec![0u8; msg_len];
                b.iter(|| {
                    let entries = black_box(entries);
                    let len = L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                        .levels(n as u16, |g| {
                            for e in entries {
                                g.add(|entry| {
                                    entry
                                        .try_price(e.price)
                                        .unwrap()
                                        .try_qty(e.qty)
                                        .unwrap()
                                        .side(e.side);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })
                        .unwrap()
                        .encoded_length_with_header();
                    black_box(&buf[..len]);
                    black_box(len)
                });
            },
        );

        // 3. add_struct with wire entry structs
        group.bench_with_input(BenchmarkId::new("add_struct", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| {
                let entries = black_box(entries);
                let len = L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| {
                        for e in entries {
                            g.add_struct(e)?;
                        }
                        Ok(())
                    })
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buf[..len]);
                black_box(len)
            });
        });

        // 4. bulk_add with one checked destination region.
        group.bench_with_input(BenchmarkId::new("bulk_add", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| {
                let entries = black_box(entries);
                let len = L2BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |group| group.bulk_add(entries))
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buf[..len]);
                black_box(len)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_group_encode_decimal);
criterion_main!(benches);
