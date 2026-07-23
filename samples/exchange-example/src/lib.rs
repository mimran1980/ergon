//! ergon IPC sample — exercise codegen + Aeron IPC + domain objects.
//!
//! Demonstrates:
//! - Multi-schema generate (normalized AppMessage + Bitget/Binance spot fixtures)
//! - Nested AppMessage → L2Book/Trade, exact claim encode into Aeron IPC
//! - Local L2 book orderbook maintenance
//! - Decimal adapter and domain-object mapping
//! - Three-thread architecture: producer, SHARED driver, consumer

/// Generated SBE codecs for the normalized application schema
/// (AppMessage/L2Book/Trade). Generated at build time into `OUT_DIR`.
// generated code carries benign unused/mut warnings; suppress at
// the include boundary until the templates themselves are warning-clean.
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
pub mod decimal;
pub mod market;
pub mod orderbook;
pub mod publication;

// Application-side TryFromSbe adapter for rust_decimal — a domain-mapping
// example showing how generated conversion traits can be implemented for
// external types without modifying the generator.
impl_sbe_decimal_for_rust_decimal!();
