//! Benchmarks the current nested-group sample API.
//!
//! Run with:
//! `cargo bench --manifest-path samples/l3-book/Cargo.toml --bench api_combinations`

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use l3_book::{book_encoded_length, encode_book};
use rust_decimal::Decimal;

const BUFFER_CAPACITY: usize = 65_536;

fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

fn bench_nested_known_counts(c: &mut Criterion) {
    let bid_orders = [
        (1_u64, decimal(2)),
        (2_u64, decimal(3)),
        (3_u64, decimal(5)),
    ];
    let ask_orders = [(4_u64, decimal(7)), (5_u64, decimal(11))];
    let bids = [
        (decimal(50_800), decimal(10), bid_orders.as_slice()),
        (decimal(50_750), decimal(20), bid_orders.as_slice()),
    ];
    let asks = [
        (decimal(50_850), decimal(15), ask_orders.as_slice()),
        (decimal(50_900), decimal(25), ask_orders.as_slice()),
    ];
    let expected = book_encoded_length(&bids, &asks, b"BTCUSDT")
        .expect("static benchmark shape must have a valid encoded length");

    let mut group = c.benchmark_group("l3_book/current_api");
    group.throughput(Throughput::Elements(1));
    group.bench_function("size_nested_book", |bencher| {
        bencher.iter(|| {
            black_box(
                book_encoded_length(black_box(&bids), black_box(&asks), black_box(b"BTCUSDT"))
                    .expect("static benchmark shape must remain valid"),
            )
        });
    });
    group.bench_function("encode_nested_book", |bencher| {
        let mut storage = [0_u8; BUFFER_CAPACITY];
        bencher.iter(|| {
            let written = encode_book(
                black_box(&mut storage[..expected]),
                black_box(&bids),
                black_box(&asks),
                black_box(b"BTCUSDT"),
            )
            .expect("pre-sized benchmark buffer must encode");
            assert_eq!(written, expected);
            black_box(written)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_nested_known_counts);
criterion_main!(benches);
