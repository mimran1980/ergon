//! ClickHouse persistence for orderbook snapshots.
//!
//! Demonstrates `#[derive(Persist)]` alongside ErgoSBE-generated `SbeMessage`
//! types. The pattern:
//! 1. Decode SBE bytes → generated type (implements `SbeMessage`)
//! 2. Populate `OrderbookSnapshot` from decoded fields
//! 3. `sender.persist(&snapshot)` → ClickHouse with auto-batching

use chrono::{DateTime, Utc};
use ergo_clickhouse_persist_derive::Persist;
use rust_decimal::Decimal;
use serde::Serialize;

/// A point-in-time snapshot of an exchange orderbook for ClickHouse persistence.
///
/// Captures the top-of-book state. For full depth, extend with `Vec<Decimal>` array
/// columns.
///
/// Table DDL (auto-generated):
/// ```sql
/// CREATE TABLE orderbook_snapshots (
///     exchange String,
///     instrument String,
///     timestamp DateTime64(9),
///     best_bid Decimal(18, 8),
///     best_ask Decimal(18, 8),
///     spread Decimal(18, 8),
///     _persist_time DateTime64(9)
/// ) ENGINE = MergeTree
/// ORDER BY (exchange, instrument, timestamp)
/// TTL timestamp + INTERVAL 24 HOURS
/// ```
#[derive(Persist, Serialize, Clone, Debug)]
#[persist(order_by = "exchange, instrument, timestamp")]
#[persist(ttl = "timestamp, 24 HOURS")]
pub struct OrderbookSnapshot {
    /// Exchange name (e.g. "bitget", "binance").
    pub exchange: String,
    /// Trading pair (e.g. "BTCUSDT").
    pub instrument: String,
    /// When the snapshot was captured (exchange time, if available).
    pub timestamp: DateTime<Utc>,
    /// Best bid price.
    #[persist(type = "Decimal(18, 8)")]
    pub best_bid: Decimal,
    /// Best ask price.
    #[persist(type = "Decimal(18, 8)")]
    pub best_ask: Decimal,
    /// ask - bid spread.
    #[persist(type = "Decimal(18, 8)")]
    pub spread: Decimal,
}

/// Snapshot the top of an orderbook.
///
/// # Panics
///
/// Panics if the book is empty (no bids or asks). Callers should guard against
/// empty books before calling this.
pub fn snapshot_book(
    exchange: &str,
    instrument: &str,
    timestamp: DateTime<Utc>,
    best_bid: Decimal,
    best_ask: Decimal,
) -> OrderbookSnapshot {
    assert!(!best_bid.is_zero() && !best_ask.is_zero(), "empty book");
    let spread = best_ask - best_bid;
    OrderbookSnapshot {
        exchange: exchange.to_string(),
        instrument: instrument.to_string(),
        timestamp,
        best_bid,
        best_ask,
        spread,
    }
}
