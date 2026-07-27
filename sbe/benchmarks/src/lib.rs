//! ergon benchmarks — on-the-fly generated codecs.
//!
//! The ergon Car codec is generated at build time by `build.rs` from the
//! example schema. This ensures benchmarks always measure the latest codegen,
//! never stale checked-in generated code.
//!
//! sbe-tool reference code is checked in (stable, generated once from upstream).

#![allow(unsafe_code)]
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
/// sbe-tool-generated Car codec (checked in, stable reference).
pub mod sbe_tool_car {
    include!("sbe_tool_car_patched.rs");
}
