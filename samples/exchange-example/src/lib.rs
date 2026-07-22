//! ergon IPC sample — exercise codegen + Aeron IPC + ClickHouse (not a prod app).
//!
//! Demonstrates:
//! - Multi-schema generate (normalized AppMessage + Bitget/Binance spot fixtures)
//! - Nested AppMessage → L2Book/Trade, exact claim encode into Aeron IPC
//! - Typed + dynamic streams, foreground ClickHouse persistence
//! - Local L2 book + `#[derive(Persist)]` top-of-book snapshot DTO
//! - Three-thread architecture: producer, SHARED driver, consumer/persister
//!
//! Cluster / NewLeader / kill-leader live in `samples/cluster-ha-orderbook`.

/// Generated SBE codecs for the normalized application schema
/// (AppMessage/L2Book/Trade). Generated at build time into `OUT_DIR`.
// ponytail: generated code carries benign unused/mut warnings; suppress at
// the include boundary (same policy as persist/src/sbe.rs) until the
// templates themselves are warning-clean.
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

pub mod bitget;
pub mod config;
pub mod counters;
pub mod decimal;
pub mod market;
pub mod orderbook;
pub mod persistence;
pub mod publication;
pub mod snapshot_persist;

// Application-side TryFromSbe adapter for rust_decimal (generated code never
// mentions rust_decimal).
impl_sbe_decimal_for_rust_decimal!();
