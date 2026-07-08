# u8/u16/u32 framing policy for groups, varData, and strings

**Blocked by:** none (codegen-only)
**Severity:** MEDIUM

## Problem

SBE schemas can specify custom `dimensionType` for repeating groups and custom
`length` field types for varData/strings. The current codegen hard-codes `u16`
for group dimension headers (2-byte blockLength + 2-byte numInGroup) and varData
length prefixes. This wastes 2 bytes per group/varData when messages are small
enough to fit in u8 ranges.

Aeron sbe-tool handles this correctly — the dimension type is resolved from the
schema's `composite` definition and codegen emits the correct read/write calls
for whatever unsigned type the schema declares.

### Typical use case

```xml
<!-- Compact u8 framing for small messages -->
<composite name="groupSize8">
  <type name="blockLength" primitiveType="uint8"/>
  <type name="numInGroup" primitiveType="uint8"/>
</composite>

<composite name="varData8">
  <type name="length" primitiveType="uint8"/>
  <type name="varData" primitiveType="uint8"/>
</composite>

<message name="CompactOrder" id="1">
  <group name="legs" id="2" dimensionType="groupSize8">
    <field name="price" id="3" type="uint32"/>
  </group>
  <data name="memo" id="4" type="varData8"/>
</message>
```

## Design

1. **Resolve dimension type at parse time** — already done (`dimensionType` is
   resolved to a composite with `blockLength`/`numInGroup` fields). Store the
   primitive type of each dimension field in the IR.

2. **Codegen changes:**
   - Group dim header write: `count as u8` instead of `count as u16` when framing is u8
   - Group dim header read: `read_bytes::<1>` instead of `read_bytes::<2>`
   - VarData length prefix: same conditional sizing
   - `GROUP_DIM_TEMPLATE` const: use correct byte size
   - `encoded_length()` calculations: account for variable dim size

3. **Public API:** Generated code uses correct integer type for group counts
   and varData lengths based on the schema's dimension type. `u8` framing
   means groups limited to 255 entries, varData to 255 bytes.

## Acceptance criteria

- [ ] Schema with `uint8` dimensionType generates group decoder using `u8` count
- [ ] Schema with `uint8` varData length generates correct length prefix read/write
- [ ] Schema with `uint32` dimensionType generates `u32` framing (for completeness)
- [ ] Default `uint16` framing unchanged for schemas without explicit dimensionType
- [ ] Roundtrip test: encode → decode with u8 framing produces correct values
- [ ] `encoded_length()` correctly accounts for non-u16 framing sizes
- [ ] Boundary values: group of 255 entries with u8 framing, group of 1 entry,
      0-entry group, varData of 0 bytes, varData of 255 bytes
- [ ] Golden file updated with u8-framed example schema
- [ ] Unit test: schema with `groupSize8` dimensionType parses correctly
- [ ] Unit test: encode/decode roundtrip with u8-framed group and varData
- [ ] Unit test: `encoded_length()` matches actual encoded length for u8 framing

## Existing schemas to verify

- `sbe/tests/fixtures/schemas/group-dimension-test-schema.xml` (if it exists)
- Upstream Aeron schemas with custom dimension types
- `fix_examples_v2rc3.xml` uses `groupSizeEncoding` with `uint16` fields plus
  `numGroups`/`numVarDataFields` extensions — verify this still works

## Ponytail

Simplest path: change the group/varData codegen to read the `primitiveType` of
the dimension/length fields from the IR and emit `u8`/`u16`/`u32`-specific code
paths via a match. No trait abstraction, no generic `ReadBuf` policy — just
three arms that emit the right byte count. Add when real schemas with non-u16
framing appear.
