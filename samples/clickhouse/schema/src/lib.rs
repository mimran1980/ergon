//! SBE schemas and the codecs generated from them.
//!
//! `market.xml` is public market data. `trading.xml` is the engine's EMAs,
//! aggregated books, orders, and fills. Each module has its own `AnyMessage`.
//! [`AnySchemaMessage`] reads the header's schema id and decodes with that
//! module's enum. Any other schema id is `Other`, not an error. The ingester
//! does not call this enum: it loads the XML and routes on the same header
//! fields.

mod any;

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

pub use any::{AnySchemaMessage, SchemaDecodeError};
