//! ergon benchmarks — on-the-fly generated codecs.
//!
//! The ergon Car codec is generated at build time by `build.rs` from the
//! example schema. This ensures benchmarks always measure the latest codegen,
//! never stale checked-in generated code.
//!
//! sbe-tool reference code is checked in (stable, generated once from upstream).

#![allow(unsafe_code, unused_unsafe)]
#![allow(
    missing_docs,
    unused_variables,
    unused_imports,
    dead_code,
    unused_mut,
    unused_must_use,
    unused_assignments,
    unused_comparisons,
    unused_attributes
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(non_camel_case_types, non_snake_case)]

// ergon-generated Car codec (from build.rs → `car_bench.rs`).
ergo_sbe::sbe_mod!(pub ergo_car = "car_bench");

// Large 256-byte composite (BigBlock) for flyweight-vs-value access benches.
ergo_sbe::sbe_mod!(pub large_comp = "large_comp_bench");
// Same shape, big-endian body — for encode LE vs BE cost.
ergo_sbe::sbe_mod!(pub large_comp_be = "large_comp_be_bench");
// LE payload/operation benchmark matrix, including owned DTOs.
ergo_sbe::sbe_mod!(pub codec_matrix = "codec_matrix_bench");
// BE fixed-block benchmark probe.
ergo_sbe::sbe_mod!(pub codec_matrix_be = "codec_matrix_be_bench");
// Custom-header fixed-block benchmark probe.
ergo_sbe::sbe_mod!(pub codec_matrix_custom_header = "codec_matrix_custom_header_bench");
// Orderbook-like group schema for bulk_add benchmarks.
ergo_sbe::sbe_mod!(pub orderbook = "orderbook_bench");
// Orderbook with Decimal composite (price: mantissa+exponent, qty: mantissa+exponent).
ergo_sbe::sbe_mod!(pub orderbook_decimal = "orderbook_decimal_bench");
// L2 orderbook with Decimal (rust_decimal conversion) for converter benchmarks.
ergo_sbe::sbe_mod!(pub l2book = "l2book_bench");
// Null/optional field benchmark schema.
ergo_sbe::sbe_mod!(pub null_option = "null_option_bench");
// Converter benchmark schema (Decimal, compact_str, chrono, bytes).
ergo_sbe::sbe_mod!(pub converters = "converters_bench");
// Extended parity schemas (ergo side — codegen from test fixtures).
ergo_sbe::sbe_mod!(pub parity_optional_enum_nullify = "parity_optional_enum_nullify_bench");
ergo_sbe::sbe_mod!(pub parity_extension = "parity_extension_bench");
ergo_sbe::sbe_mod!(pub parity_group_with_data = "parity_group_with_data_bench");
/// sbe-tool-generated Car codec (checked in, stable reference).
pub mod sbe_tool_car {
    include!("sbe_tool_car_patched.rs");
}
/// sbe-tool-generated Orderbook codec for group encode benchmark comparison.
pub mod sbe_tool_ob {
    include!("sbe_tool_ob_patched.rs");
}

/// Wrap the sbe-tool Car decoder at a framed message's body.
///
/// `message_offset` points to the first byte of the standard eight-byte SBE
/// header. The generated sbe-tool `wrap` API expects the body offset instead.
#[inline]
pub fn sbe_tool_car_body_decoder(
    buf: &[u8],
    message_offset: usize,
    acting_block_length: u16,
    acting_version: u16,
) -> sbe_tool_car::sbe_tool::car_codec::decoder::CarDecoder<'_> {
    use sbe_tool_car::sbe_tool::{ReadBuf, car_codec::decoder::CarDecoder, message_header_codec};

    CarDecoder::default().wrap(
        ReadBuf::new(buf),
        message_offset + message_header_codec::ENCODED_LENGTH,
        acting_block_length,
        acting_version,
    )
}

// ─── Untimed extent preflights ─────────────────────────────────────────────
//
// Maintained pairs run in validation class `none`: sbe-tool's `wrap` performs
// no bounds, header, or version check, so timing ergon's validating `wrap`
// against it would charge ergon for work its reference never does. The timed
// regions therefore call `wrap_unchecked`, whose safety contract is that the
// message extent is already proven.
//
// These helpers are that proof. They run once, outside every timed region, and
// panic before any measurement is taken — so a benchmark can never report a
// number for a buffer that would have been rejected.

/// Prove one framed message at `message_offset` has a full header plus the
/// version-aware fixed body extent.
///
/// Panics with the offending offset rather than returning, because a benchmark
/// that cannot legally wrap has nothing meaningful to measure.
#[inline]
pub fn assert_baseline_wrap_extent(
    buf: &[u8],
    message_offset: usize,
    acting_block_length: usize,
    acting_version: u16,
) {
    use crate::ergo_car::CarDecoder;

    let body_pos = message_offset
        .checked_add(CarDecoder::HEADER_LENGTH)
        .unwrap_or_else(|| panic!("message offset {message_offset} overflows the header length"));
    let min_fixed = CarDecoder::min_readable_fixed_extent(acting_version);
    let needed = acting_block_length.max(min_fixed);
    let available = buf.len().saturating_sub(body_pos);
    assert!(
        needed <= available,
        "message at offset {message_offset} needs {needed} body bytes but only {available} are \
         present — the timed region would wrap past the buffer"
    );
}

/// Prove every message start in a replicated stream, so a strided
/// `wrap_unchecked` loop is sound for all `count` iterations.
#[inline]
pub fn assert_stream_wrap_extent(
    buf: &[u8],
    msg_len: usize,
    count: usize,
    acting_block_length: usize,
    acting_version: u16,
) {
    assert!(msg_len > 0, "a zero-length message would never advance");
    assert!(
        buf.len() >= count.saturating_mul(msg_len),
        "stream holds {} bytes, too short for {count} messages of {msg_len}",
        buf.len()
    );
    for index in 0..count {
        assert_baseline_wrap_extent(buf, index * msg_len, acting_block_length, acting_version);
    }
}

/// Prove an encode buffer can hold a complete frame before a timed region
/// wraps it unchecked.
#[inline]
pub fn assert_encode_extent(buf: &[u8], needed_with_header: usize) {
    assert!(
        buf.len() >= needed_with_header,
        "encode buffer holds {} bytes but a complete frame needs {needed_with_header}",
        buf.len()
    );
}
