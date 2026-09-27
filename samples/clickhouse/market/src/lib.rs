//! The lab's SBE codecs, shared by the feed handlers, the engines and the
//! dummy exchanges: `market` (schema/market.xml) and `trading`
//! (schema/trading.xml).

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/market.rs"]
pub mod market;

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/trading.rs"]
pub mod trading;

/// schema/market.xml, for `Persist::connect`.
pub const MARKET_SCHEMA: &str = include_str!("../../schema/market.xml");
/// schema/trading.xml.
pub const TRADING_SCHEMA: &str = include_str!("../../schema/trading.xml");
