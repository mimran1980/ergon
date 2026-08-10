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

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ergo_sbe_benchmarks::parity_group_with_data::{
    MessageHeader, TestMessage1Decoder, TestMessage1Encoder, TestMessage1FixedFields, read_bytes,
};
use ergo_sbe_benchmarks::parity_optional_enum_nullify::{
    EnumType, OptionalComposite, OptionalEncodingEnumType, OptionalEnumNullifyDecoder,
    OptionalEnumNullifyEncoder, OptionalEnumNullifyFixedFields,
};
use std::hint::black_box;

const AMP: usize = 1024;

fn bench_optional_enum_nullify(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity_extended/optional_enum_nullify");
    group.throughput(Throughput::Elements(AMP as u64));

    let mut enc_buf = [0u8; OptionalEnumNullifyEncoder::ENCODED_LENGTH];
    let enc_len = OptionalEnumNullifyEncoder::wrap_and_apply_header(&mut enc_buf, 0)
        .fixed(&OptionalEnumNullifyFixedFields {
            optional_enum: Some(EnumType::One),
            required_enum_from_optional_type: OptionalEncodingEnumType::Alpha,
            optional_composite: OptionalComposite::new(42u16),
        })
        .encoded_length_with_header();
    let encoded = &enc_buf[..enc_len];

    // Pre-parse header — sbe-tool does zero validation
    let oe_header = MessageHeader(read_bytes::<8>(encoded, 0));
    let oe_bl = oe_header.block_length() as usize;
    let oe_version = oe_header.version();

    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let mut count: u32 = 0;
            for _ in 0..AMP {
                let dec = unsafe {
                    OptionalEnumNullifyDecoder::wrap_unchecked(
                        black_box(encoded),
                        black_box(0),
                        oe_bl,
                        oe_version,
                    )
                };
                count = count.wrapping_add(dec.optional_enum() as u32);
                count = count.wrapping_add(dec.required_enum_from_optional_type() as u32);
            }
            black_box(count);
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use sbe_tool_optional_enum_nullify::{
                ReadBuf,
                optional_enum_nullify_codec::decoder::OptionalEnumNullifyDecoder as StDecoder,
            };
            let mut count: u32 = 0;
            for _ in 0..AMP {
                let dec = StDecoder::default().wrap(
                    ReadBuf::new(black_box(encoded)),
                    8,
                    OptionalEnumNullifyDecoder::BLOCK_LENGTH as u16,
                    OptionalEnumNullifyDecoder::SCHEMA_VERSION,
                );
                count = count.wrapping_add(dec.optional_enum() as u32);
                count = count.wrapping_add(dec.required_enum_from_optional_type() as u32);
            }
            black_box(count);
        });
    });

    group.finish();
}

fn bench_group_with_data_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity_extended/group_with_data");
    group.throughput(Throughput::Elements(AMP as u64));

    let var_data = b"test";
    let len = TestMessage1Encoder::compute_length()
        .entries(1)
        .var_data_field(var_data.len())
        .unwrap()
        .encoded_length_with_header();

    // Build batch of 1024 identical messages
    let mut big_buf = vec![0u8; len * AMP];
    for i in 0..AMP {
        let elen = TestMessage1Encoder::wrap_and_apply_header(&mut big_buf[i * len..], 0)
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
        assert_eq!(elen, len);
    }

    let header = MessageHeader(read_bytes::<8>(&big_buf, 0));
    let bl = header.block_length() as usize;
    let version = header.version();

    // ergon — wrap_unchecked matches sbe-tool's zero-validation path
    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let mut total: u32 = 0;
            for i in 0..AMP {
                let dec = unsafe {
                    TestMessage1Decoder::wrap_unchecked(
                        black_box(&big_buf),
                        black_box(i * len),
                        bl,
                        version,
                    )
                };
                total = total.wrapping_add(dec.tag1());
            }
            black_box(total);
        });
    });

    // sbe-tool — same operation
    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use sbe_tool_group_with_data::{
                ReadBuf, test_message_1_codec::decoder::TestMessage1Decoder as StDecoder,
            };
            let mut total: u32 = 0;
            for i in 0..AMP {
                let dec = StDecoder::default().wrap(
                    ReadBuf::new(black_box(&big_buf)),
                    black_box(8 + i * len),
                    bl as u16,
                    version,
                );
                total = total.wrapping_add(dec.tag_1());
            }
            black_box(total);
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
