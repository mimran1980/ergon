# Per-field FieldMeta const module, SEMANTIC_VERSION, SCHEMA_HASH

**Blocked by:** none (codegen-only)

Generated code should expose schema metadata for reflection/introspection:

- `FieldMeta` const module per message with field names, types, offsets
- `SEMANTIC_VERSION` const from schema semanticVersion attribute
- `SCHEMA_HASH` const for schema identity verification

These are zero-cost compile-time values useful for tooling, logging, and
schema validation at startup.

## Acceptance criteria

- [ ] Per-message `field_meta` module with struct-per-field (name, type, offset, sinceVersion, presence)
- [ ] `pub const FIELD_META: &[FieldInfo]` array for each message
- [ ] `pub const SEMANTIC_VERSION: &str` from schema attribute
- [ ] `pub const SCHEMA_HASH: u64` computed from canonical schema representation
- [ ] Compile-time values only (no runtime overhead)
- [ ] Tests verify metadata matches schema

Ref: gap analysis (todo 51), DECISIONS.md §5 field-level metadata.
