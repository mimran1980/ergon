//! ergon IPC sample — exercise codegen + Aeron IPC + domain objects.
//!
//! Demonstrates:
//! - Multi-schema generate (normalized AppMessage + Bitget/Binance spot fixtures)
//! - Nested AppMessage → L2Book/Trade, exact claim encode into Aeron IPC
//! - Local L2 book orderbook maintenance
//! - Decimal adapter and domain-object mapping
//! - Three-thread architecture: producer, SHARED driver, consumer

// Generated SBE codecs for the normalized application schema
// (AppMessage/L2Book/Trade). Built via `generate_to_out_dir` into `OUT_DIR`.
ergo_sbe::sbe_mod!(pub normalized_app);

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
