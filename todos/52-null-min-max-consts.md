# NULL, MIN, MAX constants on fields

**Blocked by:** none (codegen-only)

Every field in generated code should expose `FOO_NULL`, `FOO_MIN`, `FOO_MAX`
consts alongside the accessor. These are compile-time values from the schema.

## Acceptance criteria

- [ ] `*_NULL` const for every field with a nullValue in schema
- [ ] `*_MIN` const for every field with a minValue in schema
- [ ] `*_MAX` const for every field with a maxValue in schema
- [ ] `const fn` for all consts (allow inline in const contexts)
- [ ] Tests: verify consts match schema values

Ref: gap analysis (todo 51), DECISIONS.md §2.
