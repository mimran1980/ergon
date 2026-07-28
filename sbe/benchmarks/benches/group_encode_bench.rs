//! Group encode benchmark: ergon add_closure vs add_struct vs bulk_add.
//!
//! Same flat group ({price: i64, qty: i64, numOrders: u32}) at 10/100/1000 entries.
//! bulk_add hoists bounds checks outside the loop; add_struct checks per-entry.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::orderbook::*;

fn make_entries(n: usize) -> Vec<LevelsEntry> {
    (0..n).map(|i| LevelsEntry {
        price: (i as i64) * 100, qty: (i as i64) * 10, num_orders: (i % 10) as u32 + 1,
    }).collect()
}

fn bench_group_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_encode");
    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let msg_len = BookSnapshotEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        group.throughput(Throughput::Elements(n as u64));
        group.sample_size(20);
        group.warm_up_time(std::time::Duration::from_millis(250));
        group.measurement_time(std::time::Duration::from_millis(500));

        group.bench_with_input(BenchmarkId::new("add_closure", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                BookSnapshotEncoder::try_wrap_and_apply_header(black_box(&mut buf), 0).unwrap()
                    .levels(n as u16, |g| {
                        for e in entries { g.add(|entry| { entry.price(e.price).qty(e.qty).num_orders(e.num_orders); Ok(()) })?; }
                        Ok(())
                    }).unwrap().encoded_length_with_header()
            ));
        });

        group.bench_with_input(BenchmarkId::new("add_struct", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                BookSnapshotEncoder::try_wrap_and_apply_header(black_box(&mut buf), 0).unwrap()
                    .levels(n as u16, |g| { for e in entries { g.add_struct(e)?; } Ok(()) }).unwrap()
                    .encoded_length_with_header()
            ));
        });

        group.bench_with_input(BenchmarkId::new("bulk_add", n), &n, |b, &n| {
            let entries = make_entries(n);
            let msg_len = BookSnapshotEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                BookSnapshotEncoder::try_wrap_and_apply_header(black_box(&mut buf), 0).unwrap()
                    .levels(n as u16, |g| { g.bulk_add(black_box(&entries))?; Ok(()) }).unwrap()
                    .encoded_length_with_header()
            ));
        });
    }
    group.finish();
}
criterion_group!(benches, bench_group_encode);
criterion_main!(benches);
