//! Cluster SBE codecs.
//!
//! # Production (ErgoSBE)
//!
//! - [`ergo_codecs`] — Aeron cluster session schema 111 (`build.rs` → `OUT_DIR`)
//! - [`ergo_codecs_mark`] — cluster mark schema
//! - [`ergo_rfq_codecs`] — cookbook RFQ schema 101 from `schemas/protocol-codecs.xml`
//!
//! # Residual (sbe-tool 1.39.0)
//!
//! [`cluster_codecs`], [`cluster_codecs_mark`], and [`rfq_codecs`] remain only
//! for head-to-head Criterion benches and wire-parity tests. Prefer ErgoSBE
//! modules for all production call sites. Do not hand-edit residual trees.

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

/// ErgoSBE-generated RFQ cookbook codecs (schema 101).
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
pub mod ergo_rfq_codecs {
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
    include!(concat!(env!("OUT_DIR"), "/aeron_rfq_codecs.rs"));
}

/// Production session codecs (schema 111). Prefer this alias in new code
/// (`use ergo_aeron_cluster::codecs::session::…`).
pub use ergo_codecs as session;

/// Production RFQ codecs (schema 101). Prefer this alias in new code
/// (`use ergo_aeron_cluster::codecs::rfq::…`).
pub use ergo_rfq_codecs as rfq;

// sbe-tool 1.39.0 omits `impl Writer for WriteBuf`; provided here so the
// generated `mod.rs` files stay pure generator output. See writer_impls.rs.
mod writer_impls;

#[cfg(test)]
mod tests;
