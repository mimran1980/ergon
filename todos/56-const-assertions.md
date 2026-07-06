# const assertions for MessageHeader size invariants

**Blocked by:** none (codegen-only)

Generated code should include compile-time `const` assertions that verify
critical size invariants from the schema:

- `HEADER_TEMPLATE.len() == 8` (or whatever the schema headerType size is)
- `GROUP_DIM_TEMPLATE.len()` matches dimensionType
- `ENCODED_LENGTH` matches hand-computed value
- Message block_length matches sum of field sizes

Catches codegen bugs at compile time rather than at runtime.

## Acceptance criteria

- [x] `const.*HEADER_TEMPLATE.len() == N);` in generated output
- [x] `const.*GROUP_DIM_TEMPLATE.len() == N);`
- [x] `const.*ENCODED_LENGTH == sum_of_field_sizes);`
- [ ] `const _: () = assert!(BLOCK_LENGTH == N);` for each message
- [ ] All existing tests pass

Ref: gap analysis (todo 51), DECISIONS.md §12 "const assertions in generated code".
