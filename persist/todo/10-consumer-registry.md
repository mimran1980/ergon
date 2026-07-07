# SchemaRegistry + RowDecoder — dynamic consumer side

**Blocked by:** 05, 09
**Blocks:** 11

The consumer-side counterpart to `DynamicRecorder`. Receives `DynamicSchema` and
`DynamicRow` messages (as decoded SBE types from todo 09), manages table schemas,
and decodes rows into `clickhouse::Row` for insertion.

## What to build

```rust
pub struct SchemaRegistry {
    // schema_id → (table_name, column map, cached TableSchema)
    schemas: HashMap<u32, RegisteredSchema>,
}

struct RegisteredSchema {
    table_name: String,
    columns: Vec<(u8, String, ColumnType)>,  // (field_id, name, type) — ordered by field_id
    table_schema: TableSchema,
}

pub struct RowDecoder {
    registry: Rc<RefCell<SchemaRegistry>>,
}

impl SchemaRegistry {
    pub fn new() -> Self;

    /// Register a schema from a DynamicSchema message. Idempotent.
    pub fn register(&mut self, schema: &DynamicSchema) -> Result<()>;

    /// Look up table name for a schema_id.
    pub fn table_name(&self, schema_id: u32) -> Option<&str>;
}

impl RowDecoder {
    pub fn new(registry: Rc<RefCell<SchemaRegistry>>) -> Self;

    /// Decode a DynamicRow into a clickhouse::Row, using the cached schema.
    pub fn decode(&self, row: &DynamicRow) -> Result<clickhouse::Row>;
}
```

### Decoding logic

For each field group in the `DynamicRow`:
1. Iterate entries `(field_id, value)`
2. Look up `field_id` in the cached schema → get column name + type
3. Push value into `clickhouse::Row` at that column name
4. Missing fields (not in this row) → encoded as null
5. Fields in schema but not in any group → null

For string fields: resolve string_id → string from the symbol table blob.

## Acceptance criteria

- [ ] `SchemaRegistry::register()` parses DynamicSchema → populates internal map
- [ ] `SchemaRegistry::register()` is idempotent (same schema_id twice → no-op)
- [ ] `SchemaRegistry::table_name()` returns correct name for known schema_id
- [ ] `RowDecoder::decode()` produces correct `clickhouse::Row` for a row with all types
- [ ] Row with string fields → correct string values via symbol table
- [ ] Row with null fields → null values in output
- [ ] Extra fields in row not in schema → ignored
- [ ] Missing fields in row (present in schema, absent in row) → null
- [ ] Unit test: register schema → decode row → verify Row contents
- [ ] Unit test: roundtrip: DynamicRecorder.record() bytes → DynamicRow decode → RowDecoder.decode()
- [ ] Unit test: multiple rows decoded in sequence (no state leak between decodes)
