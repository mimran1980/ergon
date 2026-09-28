//! SBE schemas and the codecs generated from them.
//!
//! `market.xml` is public market data. `trading.xml` is the engine's EMAs,
//! aggregated books, orders, and fills.

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/market.rs"]
pub mod market;

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/trading.rs"]
pub mod trading;

/// `market.xml`, for `Persist::connect`.
pub const MARKET_SCHEMA: &str = include_str!("../market.xml");
/// `trading.xml`.
pub const TRADING_SCHEMA: &str = include_str!("../trading.xml");
