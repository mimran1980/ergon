//! Extended performance parity: ergon vs sbe-tool for gap-feature schemas.
//!
//! Covers null-as-option enums and group/var-data messages.
//! Follows the same fairness rules as perf_parity_bench.
//!
//! Run with:
//!   cargo bench -p ergo-sbe-benchmarks --bench perf_parity_extended_bench

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

use criterion::{Criterion, criterion_group, criterion_main};
use ergo_sbe_benchmarks::parity_group_with_data::*;
use ergo_sbe_benchmarks::parity_optional_enum_nullify::*;
use std::hint::black_box;

fn bench_optional_enum_nullify(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity_extended/optional_enum_nullify");

    let mut enc_buf = [0u8; OptionalEnumNullifyEncoder::ENCODED_LENGTH];
    let enc_len = OptionalEnumNullifyEncoder::wrap_and_apply_header(&mut enc_buf, 0)
        .fixed(&OptionalEnumNullifyFixedFields {
            optional_enum: Some(EnumType::One),
            required_enum_from_optional_type: OptionalEncodingEnumType::Alpha,
            optional_composite: OptionalComposite::new(42u16),
        })
        .encoded_length_with_header();
    let encoded = &enc_buf[..enc_len];

    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let dec = OptionalEnumNullifyDecoder::decode(black_box(encoded), black_box(0)).unwrap();
            black_box(dec.optional_enum());
            black_box(dec.required_enum_from_optional_type());
            black_box(dec.optional_composite());
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use sbe_tool_optional_enum_nullify::{
                ReadBuf,
                optional_enum_nullify_codec::decoder::OptionalEnumNullifyDecoder as StDecoder,
            };
            let dec = StDecoder::default().wrap(
                ReadBuf::new(black_box(encoded)),
                8,
                OptionalEnumNullifyDecoder::BLOCK_LENGTH as u16,
                OptionalEnumNullifyDecoder::SCHEMA_VERSION,
            );
            black_box(dec.optional_enum());
            black_box(dec.required_enum_from_optional_type());
            black_box(dec.optional_composite_decoder());
        });
    });

    group.finish();
}

fn bench_group_with_data_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity_extended/group_with_data");

    // Simple message: 1 entry, minimal var-data
    let var_data = b"test";
    let len = TestMessage1Encoder::compute_length()
        .entries(1)
        .var_data_field(var_data.len())
        .unwrap()
        .encoded_length_with_header();
    let mut enc_buf = vec![0u8; len];
    let enc_len = TestMessage1Encoder::wrap_and_apply_header(&mut enc_buf, 0)
        .fixed(&TestMessage1FixedFields { tag1: 42u32 })
        .entries(1, |g| {
            g.add(|e| {
                e.var_data_field(var_data)?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap()
        .encoded_length_with_header();
    let encoded = &enc_buf[..enc_len];

    // ergon decode — scalar field only
    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let dec = TestMessage1Decoder::decode(black_box(encoded), black_box(0)).unwrap();
            black_box(dec.tag1());
        });
    });

    // sbe-tool decode — same scalar field
    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use sbe_tool_group_with_data::{
                ReadBuf, test_message_1_codec::decoder::TestMessage1Decoder as StDecoder,
            };
            let dec = StDecoder::default().wrap(
                ReadBuf::new(black_box(encoded)),
                8,
                TestMessage1Decoder::BLOCK_LENGTH as u16,
                TestMessage1Decoder::SCHEMA_VERSION,
            );
            black_box(dec.tag_1());
        });
    });

    group.finish();
}

criterion_group!(
    parity_extended,
    bench_optional_enum_nullify,
    bench_group_with_data_scalar
);
criterion_main!(parity_extended);
