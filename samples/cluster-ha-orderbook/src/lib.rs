//! HA cluster orderbook sample: try_claim publish + never-stale book.
//!
//! Recipes: `just samples-cluster-ha`, `just samples-cluster-ha-kill-leader`.

/// Generated AppMessage / L2Book codecs (ergon `build.rs` → OUT_DIR).
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_assignments,
    unused_must_use,
    non_camel_case_types,
    non_snake_case,
    clippy::all,
    clippy::pedantic
)]
pub mod normalized_app {
    #![allow(
        dead_code,
        unused_imports,
        unused_variables,
        unused_mut,
        unused_assignments,
        unused_must_use,
        non_camel_case_types,
        non_snake_case,
        clippy::all,
        clippy::pedantic
    )]
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

pub mod follower;
pub mod ha_book;
pub mod market;
pub mod publish;
