//! ergon benchmarks — on-the-fly generated codecs.
//!
//! The ergon Car codec is generated at build time by `build.rs` from the
//! example schema. This ensures benchmarks always measure the latest codegen,
//! never stale checked-in generated code.
//!
//! Aeron reference code is checked in (stable, generated once from upstream).

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

/// ergon-generated Car codec (from build.rs).
pub mod ergo_car {
    include!(concat!(env!("OUT_DIR"), "/car_bench.rs"));
}

/// Aeron-generated Car codec (checked in, stable reference).
pub mod aeron_car {
    include!("aeron_car_patched.rs");
}
