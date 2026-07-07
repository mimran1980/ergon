# Table TTL support in TableSchema

**Blocked by:** none
**Severity:** MEDIUM

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

## Acceptance criteria

- [ ] `TtlConfig` struct with `column` + `interval` fields
- [ ] `TableSchema::ttl: Option<TtlConfig>`
- [ ] `create_table_ddl()` emits `TTL` clause when configured
- [ ] `SchemaDiff` ignores TTL (no migration needed)
- [ ] `DynamicRecorderBuilder` gains `ttl()` method
- [ ] `#[derive(Persist)]` gets `#[persist(ttl = "...")]` container attribute
- [ ] Integration test: table created with TTL, verify via `SHOW CREATE TABLE`
