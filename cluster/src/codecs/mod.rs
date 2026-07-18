//! Cluster SBE codecs.
//!
//! **Production path:** [`ergo_codecs`] / [`ergo_codecs_mark`] — ErgoSBE
//! `build.rs` generation from the pinned `aeron/` submodule schemas.
//!
//! **Residual sbe-tool trees:** [`cluster_codecs`] / [`cluster_codecs_mark`]
//! remain only for head-to-head Criterion benches and legacy examples.
//! [`rfq_codecs`] is frozen sbe-tool (no schema XML). Do not hand-edit.

pub mod cluster_codecs;
pub mod cluster_codecs_mark;
pub mod rfq_codecs;

/// ErgoSBE-generated production cluster codecs (`build.rs` → OUT_DIR).
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_comparisons,
    unused_assignments,
    non_camel_case_types,
    non_snake_case,
    unexpected_cfgs,
    clippy::all
)]
pub mod ergo_codecs {
    #![allow(
        dead_code,
        unused_imports,
        unused_variables,
        unused_mut,
        unused_comparisons,
        unused_assignments,
        non_camel_case_types,
        non_snake_case,
        unexpected_cfgs,
        clippy::all
    )]
    include!(concat!(env!("OUT_DIR"), "/aeron_cluster_codecs.rs"));
}
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_comparisons,
    unused_assignments,
    non_camel_case_types,
    non_snake_case,
    unexpected_cfgs,
    clippy::all
)]
pub mod ergo_codecs_mark {
    #![allow(
        dead_code,
        unused_imports,
        unused_variables,
        unused_mut,
        unused_comparisons,
        unused_assignments,
        non_camel_case_types,
        non_snake_case,
        unexpected_cfgs,
        clippy::all
    )]
    include!(concat!(env!("OUT_DIR"), "/aeron_cluster_codecs_mark.rs"));
}

// sbe-tool 1.39.0 omits `impl Writer for WriteBuf`; provided here so the
// generated `mod.rs` files stay pure generator output. See writer_impls.rs.
mod writer_impls;

#[cfg(test)]
mod tests;
