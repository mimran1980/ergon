# Native protocol inserts (replace SQL-string VALUES)

**Blocked by:** none
**Severity:** PARKED — ponytail

**Status: PARKED (2026-07-08)**

SQL-string VALUES works correctly. Native protocol would be 3-5x faster but
this crate is debugging persistence — never the hot path. The `clickhouse`
crate dep is already in Cargo.toml, so users who need native inserts can call
`client.insert()` directly. Add when SQL VALUES is measurably the bottleneck.

## Problem

`PersistSender::persist()` serializes rows to SQL strings:
```sql
INSERT INTO trades (price, qty, side) VALUES (10050, 100, 'B')
```

This is:
1. **Slow** — SQL parsing overhead on the ClickHouse server. Native protocol is 3-5× faster for bulk inserts
2. **String-heavy** — every value goes through `serde_json` → string → SQL escaping
3. **Brittle** — SQL injection risk if escaping is wrong (currently mitigated by JSON serialization)

The `clickhouse` crate already supports native protocol inserts via
`client.insert("table")?.write(&rows).await`. The dependency is already in
`Cargo.toml`.

## Design

Replace the `INSERT INTO ... VALUES (...)` SQL path with ClickHouse native
protocol via `clickhouse::Client::insert()`.

The native `insert()` accepts any type implementing `clickhouse::Row`, which
is derived by the `clickhouse::Row` derive macro. Two options:

### Option A: Require `clickhouse::Row` derive on DTOs

User derives both `Persist` and `clickhouse::Row`:
```rust
#[derive(Persist, clickhouse::Row)]
struct Trade { price: u64, qty: u32 }
```

The sink calls `client.insert(&table_name)?.write(&rows).await`.

**Pros**: Uses existing `clickhouse` ecosystem. No new code for row encoding.
**Cons**: Requires an extra derive on every DTO. Two separate schema sources (Persist + Row).

### Option B: Generate native blocks from Persist schema

Use `clickhouse::Block` API to construct columnar blocks from `Persist::encode_row()`
output, then send via native protocol.

**Pros**: Single derive, single schema source.
**Cons**: More code in the persist crate. Must construct ClickHouse blocks manually.

### Recommendation: Option A (ponytail)

`clickhouse::Row` already handles all the type mapping correctly. Users who
want native inserts derive both. The sink detects whether `T: clickhouse::Row`
and uses native protocol when available, falling back to SQL VALUES otherwise.

## Acceptance criteria

- [ ] `PersistSender::persist()` uses ClickHouse native protocol when `T: clickhouse::Row`
- [ ] Fallback to SQL VALUES for types without `clickhouse::Row`
- [ ] Batch insert: all accumulated rows sent in a single native block
- [ ] `PersistSenderBuilder` gains `native()` / `sql()` mode toggle
- [ ] Benchmark: native inserts ≥ 3× throughput of SQL VALUES for 1000-row batches
- [ ] Integration tests pass with both native and SQL modes
- [ ] No regression in existing tests
