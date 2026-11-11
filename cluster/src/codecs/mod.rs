//! Cluster SBE codecs that ergo-sbe generates from the Aeron schemas into `OUT_DIR`.
//!
//! - `session`: the Aeron cluster session schema 111
//! - `mark`: the cluster mark schema
//!
//! The generated surface is unstable and hidden from docs.rs by
//! `#[doc(hidden)]` on the re-export in `lib.rs`. Applications should use the
//! high-level `AeronCluster` API. This module serves repository tests and
//! low-level protocol experiments.
//!
//! # Lint scope
//!
//! `missing_docs` and `unsafe_code` are allowed here because the generated
//! codec is not consumer-facing API. The documented surface is in the parent
//! crate's hand-written modules.
#![allow(unused_unsafe, missing_docs, unsafe_code)]
//! `build.rs` generates these modules at compile time with `ergo_sbe::generate`
//! and `write_to_out_dir`. The sbe-tool reference runtime for benchmarks is in
//! `cluster/benches/reference_sbe/`, private to Criterion.

ergo_sbe::sbe_mod!(pub session);
ergo_sbe::sbe_mod!(pub mark);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
