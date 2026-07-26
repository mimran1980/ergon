//! RFQ protocol codecs — generated from protocol-codecs.xml (schema 101).
//!
//! **Build-dep only:** `ergo-sbe` is used in `build.rs` only; this module is a
//! plain `include!` of `$OUT_DIR/rfq_codec.rs` (no runtime link to the generator).

#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_assignments,
    unused_must_use,
    unused_comparisons,
    non_camel_case_types,
    non_snake_case,
    unexpected_cfgs,
    clippy::all
)]
pub mod rfq_codec {
    #![allow(
        dead_code,
        unused_imports,
        unused_variables,
        unused_mut,
        unused_assignments,
        unused_must_use,
        unused_comparisons,
        non_camel_case_types,
        non_snake_case,
        unexpected_cfgs,
        clippy::all
    )]
    include!(concat!(env!("OUT_DIR"), "/rfq_codec.rs"));
}
