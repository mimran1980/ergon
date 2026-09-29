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
ergo_sbe::sbe_mod!(pub versioned_l3_v0 = "versioned_l3_v0_bench");
ergo_sbe::sbe_mod!(pub versioned_l3_v1 = "versioned_l3_v1_bench");
ergo_sbe::sbe_mod!(pub versioned_l3_v2 = "versioned_l3_v2_bench");
ergo_sbe::sbe_mod!(pub versioned_l3 = "versioned_l3_bench");
// Shared dense/sparse/empty L3 book plus one encoder per acting version.
pub mod versioned_l3_fixture;
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

/// Bytes reserved per message in the gated 10k encode pair.
pub const THROUGHPUT_SLOT: usize = 64;
/// Model year both arms of that pair write.
pub const THROUGHPUT_YEAR: u16 = 2013;

/// One header-plus-scalars encode. Shared by the timed 10k loop and the
/// untimed byte check, so the assert describes the measured work.
#[inline(always)]
pub fn throughput_encode_ergo(buf: &mut [u8], serial: u64) -> u8 {
    use crate::ergo_car::CarEncoder;

    CarEncoder::wrap_and_apply_header(buf, 0)
        .serial_number(serial)
        .model_year(THROUGHPUT_YEAR);
    buf[8]
}

/// sbe-tool arm of [`throughput_encode_ergo`]. `header(0)` runs before the
/// body setters, matching the official order.
#[inline(always)]
pub fn throughput_encode_tool(buf: &mut [u8], serial: u64) -> u8 {
    use sbe_tool_car::sbe_tool::{WriteBuf, car_codec::encoder::CarEncoder};

    CarEncoder::default()
        .wrap(WriteBuf::new(buf), 8)
        .header(0)
        .parent()
        .unwrap()
        .serial_number(serial)
        .model_year(THROUGHPUT_YEAR);
    buf[8]
}

/// One unchecked scalar decode after an external extent proof.
///
/// # Safety
/// `off + CarDecoder::HEADER_LENGTH + max(block_length,
/// CarDecoder::min_readable_fixed_extent(version))` must not overflow and
/// must be at most `buf.len()`. [`assert_baseline_wrap_extent`] proves this.
///
/// Calling it without an unsafe block must not compile:
/// ```compile_fail,E0133
/// ergo_sbe_benchmarks::throughput_decode_ergo(&[], 0, 0, 0);
/// ```
#[inline(always)]
pub unsafe fn throughput_decode_ergo(
    buf: &[u8],
    off: usize,
    block_length: usize,
    version: u16,
) -> (u64, u64) {
    use crate::ergo_car::CarDecoder;

    // SAFETY: the caller guarantees the header and version-aware body extent.
    let car = unsafe { CarDecoder::wrap_unchecked(buf, off, block_length, version) };
    (car.serial_number(), car.model_year() as u64)
}

/// sbe-tool arm of [`throughput_decode_ergo`].
#[inline(always)]
pub fn throughput_decode_tool(
    buf: &[u8],
    off: usize,
    block_length: u16,
    version: u16,
) -> (u64, u64) {
    let car = sbe_tool_car_body_decoder(buf, off, block_length, version);
    (car.serial_number(), car.model_year() as u64)
}

/// Untimed encode sample: one message and `count` messages, both arms.
pub struct EncodeThroughputSample {
    pub one_ergo: [u8; THROUGHPUT_SLOT],
    pub one_tool: [u8; THROUGHPUT_SLOT],
    pub batch_ergo: Vec<u8>,
    pub batch_tool: Vec<u8>,
    pub one_year: u64,
    pub year_total: u64,
}

/// Build the encode preflight from the same per-message body the timed loop uses.
pub fn sample_encode_throughput(count: usize) -> EncodeThroughputSample {
    let mut one_ergo = [0u8; THROUGHPUT_SLOT];
    let mut one_tool = [0u8; THROUGHPUT_SLOT];
    throughput_encode_ergo(&mut one_ergo, 0);
    throughput_encode_tool(&mut one_tool, 0);
    let (block_length, version) = (
        u16::from_le_bytes(one_ergo[0..2].try_into().unwrap()) as usize,
        u16::from_le_bytes(one_ergo[6..8].try_into().unwrap()),
    );
    assert_baseline_wrap_extent(&one_ergo, 0, block_length, version);
    // SAFETY: the preceding assertion proves the frame's fixed extent.
    let (_, one_year) = unsafe { throughput_decode_ergo(&one_ergo, 0, block_length, version) };
    let mut batch_ergo = vec![0u8; count * THROUGHPUT_SLOT];
    let mut batch_tool = vec![0u8; count * THROUGHPUT_SLOT];
    let mut year_total = 0u64;
    for i in 0..count {
        let off = i * THROUGHPUT_SLOT;
        throughput_encode_ergo(&mut batch_ergo[off..off + THROUGHPUT_SLOT], i as u64);
        throughput_encode_tool(&mut batch_tool[off..off + THROUGHPUT_SLOT], i as u64);
        assert_baseline_wrap_extent(&batch_ergo, off, block_length, version);
        // SAFETY: the preceding assertion proves this frame's fixed extent.
        let (_, year) = unsafe { throughput_decode_ergo(&batch_ergo, off, block_length, version) };
        year_total += year;
    }
    EncodeThroughputSample {
        one_ergo,
        one_tool,
        batch_ergo,
        batch_tool,
        one_year,
        year_total,
    }
}

/// Totals from walking `count` identical frames with both decode arms.
pub struct DecodeThroughputSample {
    pub one_serial: u64,
    pub one_year: u64,
    pub ergo_serial: u64,
    pub ergo_year: u64,
    pub tool_serial: u64,
    pub tool_year: u64,
}

/// Build the decode preflight from the same per-message body the timed loop uses.
pub fn sample_decode_throughput(
    buf: &[u8],
    msg_len: usize,
    count: usize,
    block_length: usize,
    version: u16,
    tool_block_length: u16,
    tool_version: u16,
) -> DecodeThroughputSample {
    assert_stream_wrap_extent(buf, msg_len, count, block_length, version);
    // A baseline sample is read even when count is zero.
    assert_baseline_wrap_extent(buf, 0, block_length, version);
    // SAFETY: the baseline assertion proves the first frame's fixed extent.
    let (one_serial, one_year) = unsafe { throughput_decode_ergo(buf, 0, block_length, version) };
    let mut ergo_serial = 0u64;
    let mut ergo_year = 0u64;
    let mut tool_serial = 0u64;
    let mut tool_year = 0u64;
    let mut off = 0;
    for _ in 0..count {
        // SAFETY: the stream assertion proves every strided frame's extent.
        let (serial, year) = unsafe { throughput_decode_ergo(buf, off, block_length, version) };
        ergo_serial += serial;
        ergo_year += year;
        let (serial, year) = throughput_decode_tool(buf, off, tool_block_length, tool_version);
        tool_serial += serial;
        tool_year += year;
        off += msg_len;
    }
    DecodeThroughputSample {
        one_serial,
        one_year,
        ergo_serial,
        ergo_year,
        tool_serial,
        tool_year,
    }
}
