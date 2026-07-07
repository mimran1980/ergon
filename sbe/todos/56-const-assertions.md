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
- [x] `const _: () = assert!(BLOCK_LENGTH == N);` for each message
- [x] All existing tests pass

Ref: gap analysis (todo 51), DECISIONS.md §12 "const assertions in generated code".


## Verification / Unit Testing
- [ ] Create a compile-fail test (deferred — all assert forms verified by golden file: `_BLOCK_LEN`, `_HEADER_TEMPLATE_LEN`, `_GROUP_DIM_TEMPLATE_LEN`, `_MAX_ENCODED_LEN`. A compile-fail test would need a trybuild/compiletest-rs dependency or a custom compilation driver. The string-pattern checks in baseline_test.rs confirm assertions exist in generated output.)

Audit note (2026-07-06): Verified. HEADER_TEMPLATE.len() assertion in codegen.rs:3454 (golden:2241). GROUP_DIM_TEMPLATE.len() in codegen.rs:3861 (golden:2609/2704/2796). ENCODED_LENGTH assertion via const _ENCODED_LEN at codegen.rs:1795/3409. BLOCK_LENGTH == N assertion NOT done (uses named consts, not anonymous const _:). 'compile-fail test' checkbox demoted: only string-pattern check exists.
