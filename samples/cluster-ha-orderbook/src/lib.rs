//! HA cluster orderbook sample: try_claim publish of complete L2 images and a
//! follower book that serves nothing from a session loss until a valid
//! recovery snapshot (rules in [`ha_book`]).
//!
//! Recipes: `just samples-cluster-ha`, `just samples-cluster-ha-kill-leader`.

// Generated under `src/generated/normalized_app.rs` (gitignored). rustfmt
// skips it: the build script rewrites it on every regeneration.
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
#[rustfmt::skip]
pub mod normalized_app;
pub mod publish;
