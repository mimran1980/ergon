# DynamicRecorder — runtime table builder (producer side)

**Blocked by:** none (standalone, but integrates with 05 + 09)
**Blocks:** 11

The "no struct needed" path. Register fields at runtime, pre-compute the wire
layout, then call `.record()` on the hot path with positional values.

The recorder generates SBE messages under the hood (see todo 09 for the schema).
The record() call encodes a DynamicRow message into a buffer and sends it to a
channel (the channel/transport is out of scope — recorder just produces SBE bytes).

## What to build

```rust
pub struct DynamicRecorderBuilder {
    table_name: String,
    fields: Vec<(String, ColumnType)>,
    metadata: Vec<(String, String)>,   // static metadata key-value pairs
}

pub struct DynamicRecorder {
    // Pre-computed: schema_id, field→group mapping, wire layout, buffer template
    schema_id: u32,
    field_map: Vec<FieldDescriptor>,
    metadata_template: Vec<u8>,  // pre-encoded metadata block, copied into buffer each record()
    buffer: Vec<u8>,             // pre-sized
}

pub enum DynamicValue {
    Int64(i64),
    UInt64(u64),
    Float64(f64),
    Bool(bool),
    String(String),
    Null,
}

impl DynamicRecorderBuilder {
    pub fn new(table_name: impl Into<String>) -> Self;
    pub fn field(mut self, name: impl Into<String>, ty: ColumnType) -> Self;
    /// Set static metadata — same value on every row.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn build(self) -> DynamicRecorder;
}

impl DynamicRecorder {
    /// Hot path — positional values matching registration order. No allocation.
    pub fn record(&mut self, values: &[DynamicValue]) -> Result<&[u8]>;
}
```

### Internals

- `build()`: registers the schema, computes a schema_id, maps field names to
  group indices, pre-encodes the metadata block into a reusable template,
  pre-allocates a buffer large enough for the maximum row size.
- `record()`: copies the metadata template, then writes values into the
  pre-allocated buffer by type group, returns the encoded SBE bytes. Every call
  reuses the same buffer (overwrites). Metadata is constant — zero overhead on
  the hot path.
- `schema_id` is deterministic: hash of table_name + sorted(field_names + types) +
  sorted(metadata keys). No consumer coordination needed — consumer discovers
  schema on first sight.

### Type inference from first row

The builder requires explicit `ColumnType` per field. No inference from values —
that's too fragile. The user must declare types.

## Acceptance criteria

- [ ] `DynamicRecorderBuilder::new()` + `.field()` + `.metadata()` + `.build()` compiles
- [ ] `record()` with correct value types produces buffer bytes
- [ ] `record()` with wrong number of values → error (panic or Result, up to implementor)
- [ ] `record()` 100k times in a loop with no allocation (verify with a simple allocation counter or benchmark)
- [ ] Metadata values are consistent across all `record()` calls from the same recorder
- [ ] Change metadata → different `schema_id` (new schema registered on the consumer)
- [ ] Change data fields → different `schema_id`
- [ ] Same fields + same metadata → same `schema_id`
- [ ] Unit test: build + record + inspect buffer (verify it's valid SBE with metadata group)
- [ ] Unit test: schema_id determinism with metadata
- [ ] Unit test: String values work (symbol table interning)
- [ ] Unit test: Null values work
- [ ] Unit test: empty metadata (no metadata keys set) → still produces valid SBE
- [ ] Unit test: multiple metadata keys → all present in buffer
