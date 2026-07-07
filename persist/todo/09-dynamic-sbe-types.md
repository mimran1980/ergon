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
columns: group (repeating)
  numColumns: uint8
  entries: ColumnDef   (composite: field_id uint8, name varString, type_tag uint8)
```

### DynamicRow
A single row of values. Sent for every record() call.

```
schema_id: uint32
int64Fields: group (repeating)    → entries: (field_id uint8, value int64)
uint64Fields: group (repeating)   → entries: (field_id uint8, value uint64)
float64Fields: group (repeating)  → entries: (field_id uint8, value double)
boolFields: group (repeating)     → entries: (field_id uint8, value uint8)
stringFields: group (repeating)   → entries: (field_id uint8, value varString)
nullFields: group (repeating)     → entries: (field_id uint8)
symbolTable: varData              → packed string data for string values
```

One group per value type. Each group entry is just `(field_id, value)`.
The "variant" is implicit in which group a field appears in.

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
- [ ] `DynamicSchema` encodes table_name and column list correctly
- [ ] All six field groups (int64, uint64, float64, bool, string, null) encode + decode correctly
- [ ] Symbol table works: string values roundtrip through varData
- [ ] Empty row (all groups have numInGroup=0) decodes correctly
- [ ] Row with one field of each type decodes correctly
- [ ] Schema + row roundtrip: encode DynamicSchema, encode DynamicRow with that schema_id, decode both
