# Migration guide from other SBE libraries

Write a migration guide for users coming from other SBE implementations (official
Java-generated Rust, sbe-tool, etc.). This was identified as missing in the
documentation todo (34).

**Status:** Not started

## Acceptance criteria

- [ ] `docs/guide/migration.md` created
- [ ] Section: migrating from the official SBE Java-generated Rust code
- [ ] Section: migrating from sbe-tool C++ codegen
- [ ] Side-by-side API comparison table (official vs ErgoSBE)
- [ ] Key differences highlighted: E3 enums, type-state encoding, version-aware decoding, raw accessors
- [ ] Common gotchas and breaking changes documented
- [ ] Links to relevant generated-api.md and advanced.md sections

## Dependencies

- `34-documentation` — documentation framework

## Notes

Todo 34 identified the migration guide as a remaining item. Users switching from
the official Java-generated Rust bindings need clear guidance on API differences.
