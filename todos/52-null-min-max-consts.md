# NULL, MIN, MAX constants on fields

**Blocked by:** none (codegen-only)

Every field in generated code should expose `FOO_NULL`, `FOO_MIN`, `FOO_MAX`
consts alongside the accessor. These are compile-time values from the schema.

## Acceptance criteria

- [x] `*_NULL` const for every field with a nullValue in schema
- [x] `*_MIN` const for every field with a minValue in schema
- [x] `*_MAX` const for every field with a maxValue in schema
- [ ] `const fn` for all consts (allow inline in const contexts) — emitted as `pub const`, not `pub const fn`
- [ ] Tests: verify consts match schema values

Ref: gap analysis (todo 51), DECISIONS.md §2.
