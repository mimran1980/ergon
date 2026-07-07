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
}

pub struct DynamicRecorder {
    // Pre-computed: schema_id, field→group mapping, wire layout, buffer template
    schema_id: u32,
    field_map: Vec<FieldDescriptor>,
    buffer: Vec<u8>,   // pre-sized
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
    pub fn build(self) -> DynamicRecorder;
}

impl DynamicRecorder {
    /// Hot path — positional values matching registration order. No allocation.
    pub fn record(&mut self, values: &[DynamicValue]) -> Result<&[u8]>;
}
```

### Internals

- `build()`: registers the schema, computes a schema_id, maps field names to
  group indices, pre-allocates a buffer large enough for the maximum row size.
- `record()`: writes values into the pre-allocated buffer by type group, returns
  the encoded SBE bytes. Every call reuses the same buffer (overwrites).
- `schema_id` is deterministic: hash of table_name + sorted(field_names + types).
  No consumer coordination needed — consumer discovers schema on first sight.

### Type inference from first row

The builder requires explicit `ColumnType` per field. No inference from values —
that's too fragile. The user must declare types.

## Acceptance criteria

- [ ] `DynamicRecorderBuilder::new()` + `.field()` + `.build()` compiles
- [ ] `record()` with correct value types produces buffer bytes
- [ ] `record()` with wrong number of values → error (panic or Result, up to implementor)
- [ ] `record()` 100k times in a loop with no allocation (verify with a simple allocation counter or benchmark)
- [ ] Two recorders with identical field sets → same `schema_id`
- [ ] Two recorders with different field sets → different `schema_id`
- [ ] Unit test: build + record + inspect buffer (verify it's valid SBE)
- [ ] Unit test: schema_id determinism
- [ ] Unit test: String values work (symbol table interning)
- [ ] Unit test: Null values work
