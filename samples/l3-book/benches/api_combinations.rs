//! Benchmarks exercising every group-encode API combination.
//! Run with: `cargo bench`

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use l3_book::*;

const T: u64 = 1_720_000_000_000_000_000;
const BUF_SZ: usize = 65536;

fn fixed() -> L3BookFixedFields {
    L3BookFixedFields { exchange_timestamp: T, sequence: 42 }
}

fn bench_explicit_count_add(c: &mut Criterion) {
    let mut g = c.benchmark_group("group/explicit_count");
    g.throughput(Throughput::Elements(1));
    g.bench_function("10_bids_5_asks_add", |b| {
        let mut buf = vec![0u8; BUF_SZ];
        b.iter(|| {
            let complete = L3BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&fixed())
                .bids(10, |bg| {
                    for i in 0..10u16 {
                        bg.add(|e| { e.price(i as i64 * 100).size(i as i64 * 10); Ok(()) }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .asks(5, |ag| {
                    for i in 0..5u16 {
                        ag.add(|e| { e.price(i as i64 * 200).size(i as i64 * 20); Ok(()) }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .symbol(black_box(b"BTCUSDT")).unwrap();
            black_box(complete.encoded_length());
        });
    });
    g.finish();
}

fn bench_unknown_size_add(c: &mut Criterion) {
    let mut g = c.benchmark_group("group/unknown_size");
    g.throughput(Throughput::Elements(1));
    g.bench_function("10_bids_5_asks_add", |b| {
        let mut buf = vec![0u8; BUF_SZ];
        b.iter(|| {
            let complete = L3BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&fixed())
                .bids_unknown_size(|bg| {
                    for i in 0..10u16 {
                        bg.add(|e| { e.price(i as i64 * 100).size(i as i64 * 10); Ok(()) }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .asks_unknown_size(|ag| {
                    for i in 0..5u16 {
                        ag.add(|e| { e.price(i as i64 * 200).size(i as i64 * 20); Ok(()) }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .symbol(black_box(b"BTCUSDT")).unwrap();
            black_box(complete.encoded_length());
        });
    });
    g.finish();
}

fn bench_explicit_count_add_struct(c: &mut Criterion) {
    let mut g = c.benchmark_group("group/explicit_count_add_struct");
    g.throughput(Throughput::Elements(1));
    g.bench_function("3_bids_2_nested_orders_struct", |b| {
        let mut buf = vec![0u8; BUF_SZ];
        b.iter(|| {
            let complete = L3BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&fixed())
                .bids(3, |bg| {
                    for i in 0..3u16 {
                        bg.add(|e| {
                            e.price(i as i64 * 100).size(i as i64 * 10);
                            e.orders(2, |og| {
                                og.add_struct(&BidsOrdersEntry { order_id: i as u64, quantity: 1, price: i as i64 * 100 }).unwrap();
                                og.add_struct(&BidsOrdersEntry { order_id: i as u64 + 100, quantity: 2, price: i as i64 * 100 + 1 }).unwrap();
                                Ok(())
                            }).unwrap();
                            Ok(())
                        }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .asks(3, |ag| {
                    for i in 0..3u16 {
                        ag.add(|e| {
                            e.price(i as i64 * 200).size(i as i64 * 20);
                            e.orders(2, |og| {
                                og.add_struct(&AsksOrdersEntry { order_id: i as u64 + 200, quantity: 1, price: i as i64 * 200 }).unwrap();
                                og.add_struct(&AsksOrdersEntry { order_id: i as u64 + 300, quantity: 2, price: i as i64 * 200 + 1 }).unwrap();
                                Ok(())
                            }).unwrap();
                            Ok(())
                        }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .symbol(black_box(b"BTCUSDT")).unwrap();
            black_box(complete.encoded_length());
        });
    });
    g.finish();
}

fn bench_mixed_unknown_struct(c: &mut Criterion) {
    let mut g = c.benchmark_group("group/mixed_unknown_struct");
    g.throughput(Throughput::Elements(1));
    g.bench_function("5_bids_nested_unknown_struct", |b| {
        let mut buf = vec![0u8; BUF_SZ];
        b.iter(|| {
            let complete = L3BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&fixed())
                .bids_unknown_size(|bg| {
                    for i in 0..5u16 {
                        bg.add(|e| {
                            e.price(i as i64 * 100).size(i as i64 * 10);
                            e.orders_unknown_size(|og| {
                                og.add_struct(&BidsOrdersEntry { order_id: i as u64, quantity: 1, price: i as i64 * 100 }).unwrap();
                                og.add_struct(&BidsOrdersEntry { order_id: i as u64 + 100, quantity: 2, price: i as i64 * 100 + 1 }).unwrap();
                                Ok(())
                            }).unwrap();
                            Ok(())
                        }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .asks_unknown_size(|ag| {
                    for i in 0..5u16 {
                        ag.add(|e| {
                            e.price(i as i64 * 200).size(i as i64 * 20);
                            e.orders_unknown_size(|og| {
                                og.add_struct(&AsksOrdersEntry { order_id: i as u64 + 200, quantity: 1, price: i as i64 * 200 }).unwrap();
                                og.add_struct(&AsksOrdersEntry { order_id: i as u64 + 300, quantity: 2, price: i as i64 * 200 + 1 }).unwrap();
                                Ok(())
                            }).unwrap();
                            Ok(())
                        }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .symbol(black_box(b"BTCUSDT")).unwrap();
            black_box(complete.encoded_length());
        });
    });
    g.finish();
}

fn bench_large_batch(c: &mut Criterion) {
    let mut g = c.benchmark_group("group/large_batch");
    g.throughput(Throughput::Elements(1));
    g.bench_function("100_bids_50_asks", |b| {
        let mut buf = vec![0u8; BUF_SZ];
        b.iter(|| {
            let complete = L3BookEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&fixed())
                .bids_unknown_size(|bg| {
                    for i in 0..100u16 {
                        bg.add(|e| { e.price(i as i64).size(i as i64 * 2); Ok(()) }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .asks_unknown_size(|ag| {
                    for i in 0..50u16 {
                        ag.add(|e| { e.price(i as i64 + 1000).size(i as i64 * 3); Ok(()) }).unwrap();
                    }
                    Ok(())
                }).unwrap()
                .symbol(black_box(b"MANY")).unwrap();
            black_box(complete.encoded_length());
        });
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_explicit_count_add,
    bench_unknown_size_add,
    bench_explicit_count_add_struct,
    bench_mixed_unknown_struct,
    bench_large_batch,
);
criterion_main!(benches);
