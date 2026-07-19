//! ErgoSBE advanced sample — pure SBE end-to-end.
//!
//! Demonstrates:
//! - SBE message generation (AppMessage, L2Book, Trade) via generated encoders
//! - Direct-claim encoding into Aeron IPC via Rusteron 0.2
//! - SBE message consumption, decoding, and dispatch via AnyMessage
//! - Typed stream 1001 and dynamic stream 1002 over Aeron IPC
//! - Foreground ClickHouse persistence with Decimal(38,18) arrays
//! - Three-thread architecture: producer, SHARED driver, consumer/persister
//!
//! No JSON. No REST. No external protocol translation. Pure SBE.

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
pub mod ha_book;
pub mod latency;
pub mod market;
pub mod persistence;
pub mod publication;

// Application-side SbeDecimal adapter for rust_decimal (generated code never
// mentions rust_decimal).
impl_sbe_decimal_for_rust_decimal!();
