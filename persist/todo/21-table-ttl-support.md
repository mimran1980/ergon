# Table TTL support in TableSchema

**Blocked by:** none
**Severity:** MEDIUM
**Status: DONE (2026-07-09)** — all AC met: TtlConfig, TableSchema.ttl, create_table_ddl() emits TTL, SchemaDiff ignores TTL, DynamicRecorder ttl(), #[persist(ttl)] attribute. Integration test verified via Docker ClickHouse.

**Decision after deferred recheck (2026-07-08):** do not keep this blanket
deferred. TTL is already implemented at the DDL/schema level and is required by
the orderbook sample. Only ClickHouse `SHOW CREATE TABLE` verification remains
environment-gated.

## Problem

`TableSchema` has no TTL configuration. ClickHouse TTL (`TTL timestamp + INTERVAL 24 HOURS`)
is essential for time-series data — it automatically drops old rows, keeping
table size bounded without manual cleanup jobs.

Currently users must manually run `ALTER TABLE ... MODIFY TTL ...` after table
creation, or rely on external cleanup.

The `samples/exchange-orderbook` todo (14) explicitly requires 24h TTL on
the orderbook table. `DynamicRecorder` also has a TTL concept but it's not
wired into the schema DDL.

## Design

Add optional TTL to `TableSchema`:

```rust
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
    pub order_by: Vec<String>,
    pub ttl: Option<TtlConfig>,
    // ...
}

pub struct TtlConfig {
    /// Column to base TTL on (usually `timestamp` or `_persist_time`).
    pub column: String,
    /// TTL interval as a ClickHouse INTERVAL expression.
    pub interval: String,  // e.g. "24 HOURS", "7 DAYS"
}
```

`create_table_ddl()` generates:
```sql
CREATE TABLE ... (
    ...
) ENGINE = MergeTree
ORDER BY (...)
TTL timestamp + INTERVAL 24 HOURS
```

`SchemaDiff` ignores TTL changes (TTL is set once, not migrated on ALTER TABLE).

## Current verification status (2026-07-08)

DDL/unit coverage exists, but ClickHouse `SHOW CREATE TABLE` verification is
still open and requires Docker.

## Acceptance criteria

- [x] `TtlConfig` struct with `column` + `interval` fields
- [x] `TableSchema::ttl: Option<TtlConfig>`
- [x] `create_table_ddl()` emits `TTL` clause when configured
- [x] `SchemaDiff` ignores TTL (no migration needed)
- [x] `DynamicRecorderBuilder` gains `ttl()` method
- [x] `#[derive(Persist)]` gets `#[persist(ttl = "...")]` container attribute
- [x] Integration test: TTL verified via Docker ClickHouse `cargo test -- --ignored` (2026-07-09). Unit test `schema_with_ttl_default_column` covers DDL generation.
