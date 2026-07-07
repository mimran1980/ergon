# End-to-end exchange orderbook → ClickHouse sample

**Blocked by:** Wire parity completion (todos 0-3), multi-schema codegen (todo 32),
schemas compile cleanly (todo 103), persist feature completeness

Once both ErgoSBE and Ergo-ClickHouse-Persist are feature-complete, the
`samples/exchange-orderbook/` crate should become a full end-to-end demo:

- Connect to an exchange WebSocket (Bitget or Binance)
- Receive SBE binary Depth50 frames
- Decode them with ErgoSBE codegen
- Build an L2 orderbook (existing `LocalBook` in `orderbook.rs`)
- Persist the orderbook snapshot into ClickHouse with 24h TTL

## Sub-tasks

1. **Schema compilation** — `build.rs` must generate compilable Rust from
   Bitget and/or Binance Spot SBE schemas (≈140KB each). Blocked by multi-schema
   codegen and wire parity.

2. **Orderbook DTO** — a `#[derive(Persist)]` struct capturing exchange name,
   instrument, timestamp, best bid/ask, spread, and top-N level arrays.
   Columns: `exchange`, `instrument`, `timestamp`, `best_bid`, `best_ask`,
   `spread`, `bid_levels` (Array), `ask_levels` (Array), `bid_sizes`,
   `ask_sizes`.

3. **ClickHouse persistence** — use `ergo-clickhouse-persist` to auto-create
   the table and persist snapshots on each update. Table engine: `MergeTree`
   with `ORDER BY (exchange, instrument, timestamp)` and `TTL timestamp +
   INTERVAL 24 HOURS`.

4. **justfile recipe** — `just samples-orderbook` that:
   - Starts ClickHouse if not running (`docker run -d clickhouse/clickhouse-server`)
   - Waits for ClickHouse to be healthy
   - Sets `CLICKHOUSE_URL` env var
   - Runs `cargo run -p exchange-orderbook`

5. **Multiple exchange/instrument support** — the sample should handle at
   least BTCUSDT from one exchange, with the schema supporting multiple
   (exchange + instrument as composite key).

## Acceptance criteria

- [ ] `just samples-orderbook` starts ClickHouse, compiles, connects to
  exchange, builds orderbook, persists to ClickHouse
- [ ] Orderbook table has `exchange`, `instrument` (e.g. "BTCUSDT") columns
- [ ] Table has 24h TTL
- [ ] `cargo test -p exchange-orderbook` passes
- [ ] Works from a single `just` command

Ref: user request for end-to-end integration demo using both crates.
