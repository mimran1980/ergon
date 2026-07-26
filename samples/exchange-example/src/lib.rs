//! ergon IPC sample — exercise codegen + Aeron IPC + domain objects.
//!
//! Demonstrates:
//! - Multi-schema generate (normalized AppMessage + Bitget/Binance spot fixtures)
//! - Nested AppMessage → L2Book/Trade, exact claim encode into Aeron IPC
//! - Local L2 book orderbook maintenance
//! - Decimal adapter and domain-object mapping
//! - Three-thread architecture: producer, SHARED driver, consumer

// Generated codecs under `src/generated/` (gitignored) — IDE can go-to-def.
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
#[path = "generated/bitget_spot.rs"]
pub mod bitget_spot;

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
#[path = "generated/binance_spot.rs"]
pub mod binance_spot;

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
