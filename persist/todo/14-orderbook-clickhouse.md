# Orderbook ClickHouse persistence schema

**Blocked by:** `samples/todo/00-e2e-orderbook-persist.md`, persist
feature completeness (all outstanding persist todos)

Provide the DTO and table schema for persisting exchange orderbook snapshots
to ClickHouse via `ergo-clickhouse-persist`.

## table: `orderbook_snapshots`

```sql
CREATE TABLE orderbook_snapshots (
    exchange           LowCardinality(String),
    instrument         LowCardinality(String),
    timestamp          DateTime64(9),
    best_bid           Decimal(18, 8),
    best_ask           Decimal(18, 8),
    spread             Decimal(18, 8),
    bid_levels         Array(Decimal(18, 8)),
    ask_levels         Array(Decimal(18, 8)),
    bid_sizes          Array(Decimal(18, 8)),
    ask_sizes          Array(Decimal(18, 8)),
    _persist_time      DateTime64(9)
) ENGINE = MergeTree
ORDER BY (exchange, instrument, timestamp)
TTL timestamp + INTERVAL 24 HOURS;
```

## Rust DTO

```rust
#[derive(Persist, Clone)]
#[persist(order_by = "exchange, instrument, timestamp")]
struct OrderbookSnapshot {
    exchange: String,           // LowCardinality(String)
    instrument: String,         // LowCardinality(String)
    timestamp: DateTime<Utc>,   // DateTime64(9)
    #[persist(type = "Decimal(18, 8)")]
    best_bid: u64,
    #[persist(type = "Decimal(18, 8)")]
    best_ask: u64,
    #[persist(type = "Decimal(18, 8)")]
    spread: u64,
    #[persist(array)]
    #[persist(type = "Decimal(18, 8)")]
    bid_levels: Vec<u64>,
    #[persist(array)]
    #[persist(type = "Decimal(18, 8)")]
    ask_levels: Vec<u64>,
    #[persist(array)]
    #[persist(type = "Decimal(18, 8)")]
    bid_sizes: Vec<u64>,
    #[persist(array)]
    #[persist(type = "Decimal(18, 8)")]
    ask_sizes: Vec<u64>,
}
```

## Tasks

- [ ] Define `OrderbookSnapshot` in `samples/exchange-orderbook/src/persist.rs`
- [ ] Add `ergo-clickhouse-persist` + `chrono` deps to sample Cargo.toml
- [ ] On each orderbook update, encode snapshot and send to ClickHouse sink
- [ ] Verify table auto-created with correct schema and 24h TTL
- [ ] Verify queries work: `SELECT * FROM orderbook_snapshots ORDER BY timestamp DESC LIMIT 10`

## Notes

- `LowCardinality(String)` for exchange/instrument — need to check if
  `PersistAs` or derive macro supports type annotations for this
- Decimal values use `u64` mantissa with 8 decimal places — price/size
  conversions need a helper in the sample
- TTL: ClickHouse drops expired rows automatically, no manual cleanup needed
