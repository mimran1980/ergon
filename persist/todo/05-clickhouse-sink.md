# ClickhouseSink — connection, schema management, batching, INSERT

**Blocked by:** 03
**Blocks:** 10, 11

The main entry point for the consumer. Manages the ClickHouse connection, caches
table schemas, auto-batches rows, handles schema migration on first-seen table
names, and runs DDL.

Also includes `PersistSender` — a wrapper that injects producer metadata into
every row. A single `ClickhouseSink` can produce multiple senders, one per
table, each with its own metadata. This is how multiple producers sharing one
consumer get identified.

## What to build

```rust
pub struct ClickhouseSinkBuilder { ... }
pub struct ClickhouseSink { ... }
pub struct PersistSenderBuilder { ... }
pub struct PersistSender { ... }

impl ClickhouseSinkBuilder {
    pub fn new() -> Self;
    pub fn url(mut self, url: &str) -> Self;
    pub fn user(mut self, user: &str) -> Self;
    pub fn password(mut self, password: &str) -> Self;
    pub fn database(mut self, db: &str) -> Self;
    pub fn batch_size(mut self, n: usize) -> Self;
    pub fn flush_interval(mut self, d: Duration) -> Self;
    pub fn build(self) -> Result<ClickhouseSink>;
}

impl ClickhouseSink {
    /// Create a sender bound to a table with optional metadata.
    pub fn sender(&self, table_name: &str) -> PersistSenderBuilder;

    /// Flush any pending batch immediately.
    pub fn flush(&self) -> Result<()>;

    /// Drop empty tables. Tables with rows are left untouched.
    pub fn cleanup(&self) -> Result<()>;
}

impl PersistSenderBuilder {
    /// Static metadata — set once, applies to every row from this sender.
    pub fn metadata(mut self, key: &str, value: impl Into<String>) -> Self;
    pub fn build(self) -> Result<PersistSender>;
}

impl PersistSender {
    /// Persist a row. Metadata columns are auto-merged into the schema + row.
    pub fn persist(&self, dto: &impl Persist) -> Result<()>;
}
```

### Internal behaviour

1. **Schema caching:** `HashMap<String, TableSchema>` — on first persist for a table name,
   generates DDL, creates table, caches schema. Metadata keys are folded into the schema
   alongside struct columns.
2. **Metadata injection:** `PersistSender` wraps a `Persist` type. On `persist()`:
   - Table schema = struct's `TableSchema` + metadata keys as `String` columns
   - Row = struct's `encode_row()` + metadata values
3. **Auto-batch:** Rows accumulate in a `Vec<Row>`. Flush when either `batch_size` rows
   or `flush_interval` elapsed. Default: 1000 rows / 100ms.
4. **Background flush:** A tokio task or thread that periodically flushes. On `Drop`,
   flushes remaining rows.
5. **Error handling:** If ClickHouse is unreachable, drop data and log warning. Never
   propagate errors to the caller (debug data, not worth blocking).
6. **Connection:** Uses `CLICKHOUSE_URL` env var as default URL.

## Acceptance criteria

- [ ] Builder pattern with all options, sensible defaults
- [ ] `sender("table").build()` succeeds
- [ ] `sender.persist(&dto)` → schema created on first call (struct columns + metadata columns), rows inserted
- [ ] Metadata columns appear in table: `app`, `host`, `pid`, etc.
- [ ] Metadata values are consistent across rows from the same sender
- [ ] Schema cached: second call with same table name skips DDL
- [ ] Schema migration: adding a data field triggers ALTER TABLE ADD COLUMN
- [ ] Schema migration: new metadata key on sender rebuild triggers ALTER TABLE ADD COLUMN
- [ ] Type conflict: incompatible change is logged and skipped
- [ ] Auto-batch: rows don't appear until flush interval or batch size hit
- [ ] Manual `flush()` sends pending rows immediately
- [ ] `cleanup()` drops empty tables, leaves non-empty tables alone
- [ ] `Drop` on sink flushes remaining rows
- [ ] ClickHouse unreachable → drop data, log warning, don't panic
- [ ] Default URL from `CLICKHOUSE_URL` env var
- [ ] Multiple senders on one sink (same table, different metadata) produce correct rows
- [ ] Multiple senders on one sink (different tables) produce isolated tables
- [ ] Unit test: schema caching logic with metadata columns (no ClickHouse needed)
- [ ] Unit test: batch accumulation + flush trigger
- [ ] Unit test: diff logic triggers correct DDL
- [ ] Integration test: docker ClickHouse, create table + insert + query back + verify metadata
- [ ] Integration test: schema migration (add column, type change, conflict)
- [ ] Integration test: cleanup drops empty table
- [ ] Integration test: multiple tables from same struct via different names
