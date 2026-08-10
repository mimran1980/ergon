//! Extended performance parity: ergon vs sbe-tool for gap-feature schemas.
//!
//! Covers null-as-option enums, version-gated fields, nested groups.
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

    // ergon decode
    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let dec = OptionalEnumNullifyDecoder::decode(black_box(encoded), black_box(0)).unwrap();
            black_box(dec.optional_enum());
            black_box(dec.required_enum_from_optional_type());
            black_box(dec.optional_composite());
        });
    });

    // sbe-tool decode
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

criterion_group!(parity_extended, bench_optional_enum_nullify);
criterion_main!(parity_extended);
