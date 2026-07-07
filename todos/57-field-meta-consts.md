# Per-field FieldMeta const module, SEMANTIC_VERSION, SCHEMA_HASH

**Blocked by:** none (codegen-only)

Generated code should expose schema metadata for reflection/introspection:

- `FieldMeta` const module per message with field names, types, offsets
- `SEMANTIC_VERSION` const from schema semanticVersion attribute
- `SCHEMA_HASH` const for schema identity verification

These are zero-cost compile-time values useful for tooling, logging, and
schema validation at startup.

## Acceptance criteria

- [x] Per-message `field_meta` module with struct-per-field (name, type, offset, sinceVersion, presence)
- [x] `pub const FIELDS: &[FieldInfo]` array for each message
- [x] `pub const SEMANTIC_VERSION: &str` from schema attribute
- [x] `pub const SCHEMA_HASH: u64` computed from canonical schema representation
- [x] Compile-time values only (no runtime overhead)
- [x] Tests verify metadata matches schema

Ref: gap analysis (todo 51), DECISIONS.md §5 field-level metadata.

## Verification / Unit Testing
- [x] Write a test `test_field_meta_consts` in `sbe/tests/integration_tests.rs` that:
  1. Inspects the generated `car_field_meta` (or similar) module.
  2. Asserts that every field metadata constant (e.g. `car_field_meta::SERIAL_NUMBER`) has the expected `id`, `since_version`, and `offset`.
  3. Asserts that the hash `SCHEMA_HASH` matches the expected value, and `SEMANTIC_VERSION` is correct.
