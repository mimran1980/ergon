//! HA cluster orderbook sample: try_claim publish + never-stale book + latency.
//!
//! **Rusteron pin:** `ergo-aeron-cluster` / rusteron-client **0.2.4**. The IPC
//! baseline `advanced-bitget` stays on **0.2.1**. Separate binaries avoid
//! dual-pin conflict.
//!
//! Recipes: `just samples-cluster-ha`, `just samples-cluster-ha-kill-leader`.
//! See `samples/cluster-ha-orderbook/README.md`.

#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_assignments,
    unused_must_use,
    clippy::all,
    clippy::pedantic
)]
pub mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

pub mod follower;
pub mod ha_book;
pub mod latency;
pub mod market;
pub mod publish;
