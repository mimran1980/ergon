//! Historic ergo regression benchmarks — ergo-only measurements compared
//! against stored baselines to detect silent regressions.
//!
//! Covers ergo-specific features sbe-tool does not implement: null-as-option
//! enums, optional presence fields, Decimal converters, bulk_add.
//!
//! Run with:
//!   cargo bench -p ergo-sbe-benchmarks --bench ergo_historic_bench
//!
//! Baselines in `sbe/benchmarks/ergo-historic-baseline.env`.
//! Gate: `scripts/check-bench-historic.sh`.

#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    missing_docs,
    unused,
    unused_variables,
    unused_must_use,
    unsafe_code
)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ergo_sbe_benchmarks::converters::*;
use ergo_sbe_benchmarks::null_option::*;
use std::hint::black_box;

// ── Null-as-option encode / decode ────────────────────────────────────────

fn bench_null_option(c: &mut Criterion) {
    let mut group = c.benchmark_group("ergo_historic/null_option");

    let mut enc_buf = [0u8; NullOptionEncoder::ENCODED_LENGTH];
    let enc_len = NullOptionEncoder::wrap_and_apply_header(&mut enc_buf, 0)
        .fixed(&NullOptionFixedFields {
            optional_enum: Some(EnumType::One),
            nullable_enum: OptionalEnum::Alpha,
            optional_scalar: Some(42u32),
            required_scalar: 7u32,
            optional_composite: OptionalComposite::new(100u16, 1u8),
            enum_composite: EnumComposite::new(2u8, 99u32),
        })
        .encoded_length_with_header();
    let encoded = &enc_buf[..enc_len];

    group.bench_function("decode_fixed", |b| {
        b.iter(|| {
            let dec = NullOptionDecoder::decode(black_box(encoded), black_box(0)).unwrap();
            black_box(dec.optional_enum());
            black_box(dec.nullable_enum());
            black_box(dec.optional_scalar());
            black_box(dec.required_scalar());
        });
    });

    group.bench_function("encode_fixed", |b| {
        let mut buf = [0u8; NullOptionEncoder::ENCODED_LENGTH];
        b.iter(|| {
            let len = NullOptionEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&NullOptionFixedFields {
                    optional_enum: Some(EnumType::Two),
                    nullable_enum: OptionalEnum::Beta,
                    optional_scalar: Some(99u32),
                    required_scalar: 3u32,
                    optional_composite: OptionalComposite::new(0u16, 0u8),
                    enum_composite: EnumComposite::new(1u8, 50u32),
                })
                .encoded_length_with_header();
            black_box(&buf[..len]);
        });
    });

    group.finish();
}

// ── Converter encode / decode ─────────────────────────────────────────────

fn bench_converters(c: &mut Criterion) {
    let mut group = c.benchmark_group("ergo_historic/converters");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(1));

    let symbol = b"BTCUSD";
    let payload: &[u8] = &[0u8; 64];
    let msg_len = ConvertersEncoder::try_compute_encoded_length_with_header(
        symbol.len().try_into().unwrap(),
        payload.len().try_into().unwrap(),
    )
    .unwrap();

    group.bench_function("encode_decimal_and_vardata", |b| {
        let mut buf = vec![0u8; msg_len];
        b.iter(|| {
            let len = ConvertersEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .fixed(&ConvertersFixedFields {
                    price: Decimal::new(12345_00),
                    qty: Decimal::new(100_00),
                    timestamp_nanos: 1_720_000_000_000_000_000i64,
                    timestamp_micros: 1_720_000_000_000_000i64,
                })
                .symbol(black_box(symbol))
                .unwrap()
                .payload(black_box(payload))
                .unwrap()
                .encoded_length_with_header();
            black_box(&buf[..len]);
        });
    });

    let mut dec_buf = vec![0u8; msg_len];
    let dec_len = ConvertersEncoder::wrap_and_apply_header(&mut dec_buf, 0)
        .fixed(&ConvertersFixedFields {
            price: Decimal::new(12345_00),
            qty: Decimal::new(100_00),
            timestamp_nanos: 1_720_000_000_000_000_000,
            timestamp_micros: 1_720_000_000_000_000,
        })
        .symbol(symbol)
        .unwrap()
        .payload(payload)
        .unwrap()
        .encoded_length_with_header();
    let dec_encoded = &dec_buf[..dec_len];

    group.bench_function("decode_decimal_and_vardata", |b| {
        b.iter(|| {
            let dec = ConvertersDecoder::decode(black_box(dec_encoded), black_box(0)).unwrap();
            black_box(dec.price_wire());
            black_box(dec.qty_wire());
            black_box(dec.timestamp_nanos());
            black_box(dec.timestamp_micros());
        });
    });

    group.finish();
}

criterion_group!(
    name = historic;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(1));
    targets = bench_null_option, bench_converters
);
criterion_main!(historic);
