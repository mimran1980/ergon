//! HA cluster orderbook sample: try_claim publish + never-stale book.
//!
//! Recipes: `just samples-cluster-ha`, `just samples-cluster-ha-kill-leader`.

// Generated under `src/generated/normalized_app.rs` (gitignored).
pub mod follower;
pub mod ha_book;
pub mod market;
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_must_use,
    non_camel_case_types,
    non_snake_case,
    clippy::all,
    warnings
)]
#[path = "generated/normalized_app.rs"]
pub mod normalized_app;
pub mod publish;
