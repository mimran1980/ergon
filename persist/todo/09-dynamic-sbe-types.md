# Dynamic SBE message types — schema + row encoding

**Blocked by:** none (pure ErgoSBE schema work)
**Blocks:** 10

Define the SBE schema and generate Rust types for the dynamic table protocol.
These are fixed, generic message types — no runtime codegen needed.

## SBE schema design

Three message types:

### DynamicSchema
Registers a table's column layout. Sent once (or on producer restart).

```
schema_id: uint32
table_name: varString
metadata: group (repeating)
  numEntries: uint8
  entries: MetadataEntry   (composite: key varString, value varString)
columns: group (repeating)
  numColumns: uint8
  entries: ColumnDef       (composite: field_id uint8, name varString, type_tag uint8)
```

### DynamicRow
A single row of values. Sent for every record() call.

```
schema_id: uint32
metadata: group (repeating)        → entries: (key varString, value varString)
int64Fields: group (repeating)     → entries: (field_id uint8, value int64)
uint64Fields: group (repeating)    → entries: (field_id uint8, value uint64)
float64Fields: group (repeating)   → entries: (field_id uint8, value double)
boolFields: group (repeating)      → entries: (field_id uint8, value uint8)
stringFields: group (repeating)    → entries: (field_id uint8, value varString)
nullFields: group (repeating)      → entries: (field_id uint8)
symbolTable: varData               → packed string data for string values
```

The metadata group sits at the front of the message — the consumer reads it
first and creates/updates metadata columns before processing data groups.
Metadata is the same group structure in both `DynamicSchema` and `DynamicRow`.
In `DynamicSchema` it declares which metadata keys exist; in `DynamicRow` it
provides their values. New metadata keys discovered in `DynamicRow` (not in
the schema) trigger `ADD COLUMN` on the consumer.

### Type tag mapping
```
0 = Int64
1 = UInt64
2 = Float64
3 = Bool
4 = String
5 = Null
```

## ErgoSBE integration

These schemas are standard SBE XML. Generate Rust types with ErgoSBE's existing
build.rs pipeline. The generated types live in `ergo-clickhouse-persist` (not in
the user's crate — these are internal infrastructure types).

## Acceptance criteria

- [ ] SBE XML schema for `DynamicSchema`, `DynamicRow` written
- [ ] Schema compiles through ErgoSBE's generator
- [ ] Generated Rust types compile with no warnings
- [ ] `DynamicRow` can encode + decode roundtrip with known values
- [ ] `DynamicSchema` encodes table_name, metadata, and column list correctly
- [ ] Metadata group in `DynamicSchema` → consumer can read metadata keys
- [ ] Metadata group in `DynamicRow` → consumer can read metadata values
- [ ] Metadata roundtrip: encode with metadata → decode → metadata values match
- [ ] All six field groups (int64, uint64, float64, bool, string, null) encode + decode correctly
- [ ] Symbol table works: string values roundtrip through varData
- [ ] Empty row (all groups including metadata have numInGroup=0) decodes correctly
- [ ] Row with metadata + one field of each type decodes correctly
- [ ] Schema + row roundtrip: encode DynamicSchema, encode DynamicRow with that schema_id, decode both
- [ ] Empty metadata (no metadata keys) → still produces valid roundtrip
