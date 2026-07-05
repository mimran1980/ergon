# Constant-value fields (`presence="constant"`)

**Blocked by:** `01-scalar-wire-parity`

Fields with `presence="constant"` don't consume wire space — their value is
baked from the schema. The accessor should return a `const fn` with the
compile-time value. String constants return `&'static str`; numeric constants
return the typed value. DECISIONS.md §4.

## Acceptance criteria

- [ ] Parse `presence="constant"` and `constantValue` from XML
- [ ] Constant string fields: `pub const fn foo(&self) -> &'static str { "value" }`
- [ ] Constant numeric fields: `pub const fn foo(&self) -> T { value }`
- [ ] Constant char fields > 1 byte return `&'static str`
- [ ] Constant-value field does not advance the offset counter
- [ ] Test: `constant-enum-fields.xml` schema
- [ ] Test: `basic-schema-constant-header-field.xml` schema
- [ ] Test: `group-with-constant-fields.xml` schema (constant fields in groups)

Ref: `design/DECISIONS.md` §4 constant-value fields.
