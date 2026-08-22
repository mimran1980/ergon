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

// Timing for this pair is a memory-bound two-byte-enum load. The gate is
// literal 1.00. Instruction-probe Ir/op (`just bench-instructions`) is a
// Linux-only mechanism check, not a substitute for this ceiling. Re-run
// `just bench` on an idle machine if wall-clock flips.
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

    {
        let ergo =
            unsafe { OptionalEnumNullifyDecoder::wrap_unchecked(encoded, 0, oe_bl, oe_version) };
        use sbe_tool_optional_enum_nullify::{
            ReadBuf, optional_enum_nullify_codec::decoder::OptionalEnumNullifyDecoder as StDecoder,
        };
        let tool = StDecoder::default().wrap(
            ReadBuf::new(encoded),
            8,
            OptionalEnumNullifyDecoder::BLOCK_LENGTH as u16,
            OptionalEnumNullifyDecoder::SCHEMA_VERSION,
        );
        assert_eq!(ergo.optional_enum() as u32, tool.optional_enum() as u32);
        assert_eq!(
            ergo.required_enum_from_optional_type() as u32,
            tool.required_enum_from_optional_type() as u32
        );
    }

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

const GROUP_SYMBOL: [u8; 9] = *b"ABCDEFGHI";
const GROUP_TAG2: i64 = 7;
const GROUP_VAR: &[u8] = b"test";

fn encode_group_with_data_message(buf: &mut [u8]) -> usize {
    TestMessage1Encoder::wrap_and_apply_header(buf, 0)
        .fixed(&TestMessage1FixedFields { tag1: 42u32 })
        .entries(1, |g| {
            g.add(|mut e| {
                e.tag_group1(GROUP_SYMBOL).tag_group2(GROUP_TAG2);
                e.var_data_field(GROUP_VAR)
            })?;
            Ok(())
        })
        .unwrap()
        .encoded_length_with_header()
}

fn fold_group_entry(tag1: u32, symbol: &[u8], tag2: i64, var: &[u8]) -> u32 {
    let mut total = tag1;
    total = total.wrapping_add(tag2 as u32);
    total = total.wrapping_add(var.len() as u32);
    for byte in symbol.iter().chain(var) {
        total = total.wrapping_add(u32::from(*byte));
    }
    total
}

fn decode_group_with_data_ergon(buf: &[u8], msg_offset: usize, bl: usize, version: u16) -> u32 {
    let dec = unsafe { TestMessage1Decoder::wrap_unchecked(buf, msg_offset, bl, version) };
    let tag1 = dec.tag1();
    let mut entries = dec.into_entries().expect("entries");
    let entry = entries.next().expect("one entry").expect("entry");
    fold_group_entry(
        tag1,
        &entry.tag_group1(),
        entry.tag_group2(),
        entry.var_data_field().expect("var"),
    )
}

fn decode_group_with_data_tool(buf: &[u8], msg_offset: usize, bl: u16, version: u16) -> u32 {
    use sbe_tool_group_with_data::{
        ReadBuf, test_message_1_codec::decoder::TestMessage1Decoder as StDecoder,
    };
    let dec = StDecoder::default().wrap(ReadBuf::new(buf), 8 + msg_offset, bl, version);
    let tag1 = dec.tag_1();
    let mut entries = dec.entries_decoder();
    assert!(entries.advance().expect("advance").is_some());
    let symbol = entries.tag_group_1();
    let tag2 = entries.tag_group_2();
    let coords = entries.var_data_field_decoder();
    let var = entries.var_data_field_slice(coords);
    fold_group_entry(tag1, &symbol, tag2, var)
}

fn assert_group_with_data_value_parity(encoded: &[u8], bl: usize, version: u16) {
    let ergo = decode_group_with_data_ergon(encoded, 0, bl, version);
    let tool = decode_group_with_data_tool(encoded, 0, bl as u16, version);
    assert_eq!(ergo, tool, "group+var-data decode must match before timing");
    assert_eq!(
        ergo,
        fold_group_entry(42, &GROUP_SYMBOL, GROUP_TAG2, GROUP_VAR)
    );
}

fn bench_group_with_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity_extended/group_with_data");
    group.throughput(Throughput::Elements(AMP as u64));

    let len = TestMessage1Encoder::compute_length()
        .entries(1)
        .var_data_field(GROUP_VAR.len())
        .unwrap()
        .encoded_length_with_header();

    let mut big_buf = vec![0u8; len * AMP];
    for i in 0..AMP {
        let elen = encode_group_with_data_message(&mut big_buf[i * len..]);
        assert_eq!(elen, len);
    }

    let header = MessageHeader(read_bytes::<8>(&big_buf, 0));
    let bl = header.block_length() as usize;
    let version = header.version();
    assert_group_with_data_value_parity(&big_buf[..len], bl, version);

    group.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            let buf = black_box(&big_buf);
            let mut total: u32 = 0;
            for i in 0..AMP {
                total = total.wrapping_add(decode_group_with_data_ergon(buf, i * len, bl, version));
            }
            black_box(total);
        });
    });

    group.bench_function("sbe-tool", |b| {
        b.iter(|| {
            let buf = black_box(&big_buf);
            let mut total: u32 = 0;
            for i in 0..AMP {
                total = total.wrapping_add(decode_group_with_data_tool(
                    buf,
                    i * len,
                    bl as u16,
                    version,
                ));
            }
            black_box(total);
        });
    });

    group.finish();
}

criterion_group!(
    parity_extended,
    bench_optional_enum_nullify,
    bench_group_with_data
);
criterion_main!(parity_extended);
