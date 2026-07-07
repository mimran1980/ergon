# SchemaRegistry + RowDecoder — dynamic consumer side

**Blocked by:** 05, 09
**Blocks:** 11
**Status: DONE**

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
    metadata_keys: Vec<String>,               // metadata column names (all String type)
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

1. **Metadata first:** Read the metadata group from DynamicRow. For each
   `(key, value)` pair: push into `clickhouse::Row` at that column name.
   If the key is new (not in the registered schema), trigger `ADD COLUMN`
   and add it to the schema cache.
2. **Data fields:** For each field group in the `DynamicRow`:
   - Iterate entries `(field_id, value)`
   - Look up `field_id` in the cached schema → get column name + type
   - Push value into `clickhouse::Row` at that column name
3. Missing fields (in schema but not in this row) → encoded as null.
4. Extra fields in row (not in schema) → ignored (data fields only — new
   metadata keys are always added).
5. For string fields: resolve string_id → string from the symbol table blob.

## Acceptance criteria

- [x] `SchemaRegistry::register()` parses DynamicSchema → populates internal map with columns + metadata_keys
- [x] `SchemaRegistry::register()` is idempotent (same schema_id twice → no-op)
- [x] `SchemaRegistry::table_name()` returns correct name for known schema_id
- [x] `RowDecoder::decode()` produces correct `clickhouse::Row` with data + metadata columns
- [x] Metadata values from row decoded into correct `clickhouse::Row` columns
- [x] New metadata key in row (not in schema) → ADD COLUMN + added to cache
- [x] Row with string fields → correct string values via symbol table
- [x] Row with null fields → null values in output
- [x] Extra data fields in row not in schema → ignored
- [x] Missing data fields in row (present in schema, absent in row) → null
- [x] Unit test: register schema with metadata → decode row → verify Row contents including metadata
- [x] Unit test: roundtrip: DynamicRecorder.record() bytes → DynamicRow decode → RowDecoder.decode() — metadata intact
- [x] Unit test: multiple rows decoded in sequence (no state leak between decodes)
- [x] Unit test: dynamic metadata discovery (new metadata key appears mid-stream)
