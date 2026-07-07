# ClickhouseSink — connection, schema management, batching, INSERT

**Blocked by:** 03
**Blocks:** 10, 11

The main entry point for the consumer. Manages the ClickHouse connection, caches
table schemas, auto-batches rows, handles schema migration on first-seen table
names, and runs DDL.

## What to build

```rust
pub struct ClickhouseSinkBuilder { ... }
pub struct ClickhouseSink { ... }

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
    /// Persist a single row. First call for a table name creates the table.
    /// Subsequent calls with changed schema trigger ALTER TABLE.
    pub fn persist(&self, dto: &impl Persist, table_name: &str) -> Result<()>;

    /// Flush any pending batch immediately.
    pub fn flush(&self) -> Result<()>;

    /// Drop empty tables. Tables with rows are left untouched.
    pub fn cleanup(&self) -> Result<()>;
}
```

### Internal behaviour

1. **Schema caching:** `HashMap<String, TableSchema>` — on first `persist("my_table", ...)`,
   generates DDL, creates table, caches schema. Subsequent calls diff against cached.
2. **Auto-batch:** Rows accumulate in a `Vec<Row>`. Flush when either `batch_size` rows
   or `flush_interval` elapsed. Default: 1000 rows / 100ms.
3. **Background flush:** A tokio task or thread that periodically flushes. On `Drop`,
   flushes remaining rows.
4. **Error handling:** If ClickHouse is unreachable, drop data and log warning. Never
   propagate errors to the caller (debug data, not worth blocking).
5. **Connection:** Uses `CLICKHOUSE_URL` env var as default URL.

## Acceptance criteria

- [ ] Builder pattern with all options, sensible defaults
- [ ] `persist()` → schema created on first call, rows inserted
- [ ] Schema cached: second call with same table name skips DDL
- [ ] Schema migration: adding a field triggers ALTER TABLE ADD COLUMN
- [ ] Type conflict: incompatible change is logged and skipped
- [ ] Auto-batch: rows don't appear until flush interval or batch size hit
- [ ] Manual `flush()` sends pending rows immediately
- [ ] `cleanup()` drops empty tables, leaves non-empty tables alone
- [ ] `Drop` on sink flushes remaining rows
- [ ] ClickHouse unreachable → drop data, log warning, don't panic
- [ ] Default URL from `CLICKHOUSE_URL` env var
- [ ] Unit test: schema caching logic (no ClickHouse needed)
- [ ] Unit test: batch accumulation + flush trigger
- [ ] Unit test: diff logic triggers correct DDL
- [ ] Integration test: docker ClickHouse, create table + insert + query back
- [ ] Integration test: schema migration (add column, type change, conflict)
- [ ] Integration test: cleanup drops empty table
- [ ] Integration test: multiple tables from same struct via different names
