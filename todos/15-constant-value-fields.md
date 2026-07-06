# Constant-value fields (`presence="constant"`)

**Blocked by:** `01-scalar-wire-parity`

Fields with `presence="constant"` don't consume wire space — their value is
baked from the schema. The accessor should return a `const fn` with the
compile-time value. String constants return `&'static str`; numeric constants
return the typed value. DECISIONS.md §4.

## Acceptance criteria

- [ ] Parse `presence="constant"` and `constantValue` from XML (needed for enum/set constant fields via `valueRef`)
- [ ] Constant string fields: `pub const fn foo(&self) -> &'static str { "value" }`
- [ ] Constant numeric fields: `pub const fn foo(&self) -> T { value }`
- [ ] Constant char fields > 1 byte return `&'static str`
- [x] Constant-value field does not advance the offset counter
- [x] Codegen: Enum decoder arm emits `const fn` for constant fields
- [x] Codegen: Set decoder arm emits `const fn` for constant fields
- [ ] Test: `constant-enum-fields.xml` schema (blocked by XML parsing of `valueRef` + `presence="constant"` on field elements)
- [ ] Test: `basic-schema-constant-header-field.xml` schema
- [ ] Test: `group-with-constant-fields.xml` schema (constant fields in groups)

Ref: `design/DECISIONS.md` §4 constant-value fields.
