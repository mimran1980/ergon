# Constant-value fields (`presence="constant"`)

**Blocked by:** `01-scalar-wire-parity`

Fields with `presence="constant"` don't consume wire space — their value is
baked from the schema. The accessor should return a `const fn` with the
compile-time value. String constants return `&'static str`; numeric constants
return the typed value. DECISIONS.md §4.

## Acceptance criteria

- [x] Parse `presence="constant"` and `constantValue` from XML (needed for enum/set constant fields via `valueRef`)
- [x] Constant string fields: `pub const fn foo(&self) -> &'static str { "value" }`
- [x] Constant numeric fields: `pub const fn foo(&self) -> T { value }`
- [x] Constant char fields > 1 byte return `&'static str`
- [x] Constant-value field does not advance the offset counter
- [x] Codegen: Enum decoder arm emits `const fn` for constant fields
- [x] Codegen: Set decoder arm emits `const fn` for constant fields
- [x] Test: `constant-enum-fields.xml` schema
- [ ] Test: `basic-schema-constant-header-field.xml` schema (codegen header_size bug — constant in messageHeader composite produces wrong offset)
- [x] Test: `group-with-constant-fields.xml` schema (constant fields in groups)

Ref: `design/DECISIONS.md` §4 constant-value fields.


## Verification / Unit Testing
- [x] Create a unit test `test_constant_fields` that decodes messages with presence="constant" fields and verifies the returned constant values match the schema without reading from the buffer.
  - `proptest_roundtrip.rs` verifies maxRpm=9000, fuel="Petrol", discountedModel=Model::C round-trip correctly
  - `group_with_constant_fields_types_exist` verifies constant fields in groups
  - `constant_enum_fields_types_exist` verifies constant enum valueRef fields
