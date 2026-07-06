# Migration guide from other SBE libraries

Write a migration guide for users coming from other SBE implementations (official
Java-generated Rust, sbe-tool, etc.). This was identified as missing in the
documentation todo (34).

**Status:** Done

## Acceptance criteria

- [x] `docs/guide/migration.md` created
- [x] Section: migrating from the official SBE Java-generated Rust code
- [x] Section: migrating from sbe-tool C++ codegen (covered alongside Java-generated Rust)
- [x] Side-by-side API comparison table (official vs ErgoSBE)
- [x] Key differences highlighted: E3 enums, type-state encoding, version-aware decoding, raw accessors
- [x] Common gotchas and breaking changes documented
- [x] Links to relevant generated-api.md and advanced.md sections

## Dependencies

- `34-documentation` — documentation framework

## Notes

Todo 34 identified the migration guide as a remaining item. Users switching from
the official Java-generated Rust bindings need clear guidance on API differences.
