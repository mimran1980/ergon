# Audit: `#[expect]` / `#[allow]` lint state in generated code (todo 95)

Audit whether generated ErgoSBE code uses `#[expect(lint)]` or `#[allow(lint)]`,
and whether CI would catch stale suppressions if `#[expect]` were used.
**Status: DONE**


## Status

✅ Complete

## Findings

### Lint attributes in generated code

**All crate-level suppressions use `#[allow(...)]`, NOT `#[expect(...)]`:**

- `#![allow(non_camel_case_types)]`
- `#![allow(non_snake_case)]`
- `#![allow(clippy::identity_op)]`
- `#![allow(clippy::eq_op)]`
- `#![allow(clippy::needless_borrow)]`
- `#![allow(clippy::manual_range_contains)]`
- `#![allow(unused_imports)]`
- `#![allow(unused_variables)]`
- `#![allow(unused_mut)]`
- `#![allow(dead_code)]`

**Item-level:** `#[allow(unused_unsafe)]` on `raw_*()` accessor methods.

**No `#[expect(...)]` appears anywhere in generated output.** The source codegen
(`sbe/src/codegen.rs`) has one `#[expect(unused_variables)]` at line 3225, but
that is in the codegen tool itself, not in the generated crate.

### CI clippy configuration

`.github/workflows/ci.yml` runs: `cargo clippy --workspace --all-targets`

- Does **NOT** have `-D warnings`, so stale clippy warnings would not fail CI.
- `expect_used = "warn"` is set in `Cargo.toml`, but `#[expect]` is not used in
  generated code, so this is a no-op for generated output.

### Why `#[allow]` is correct here (not `#[expect]`)

`#[expect(lint)]` warns when the suppressed lint stops firing — useful for
detecting stale suppressions. However, in ErgoSBE's generated code, the exact
set of lints that fire depends on the **schema**. A schema that does not trigger
a particular lint would produce a false-positive stale-suppression warning in
`#[expect]`, breaking CI for end users. `#[allow]` is the right choice for
generated, schema-driven code.

If the codegen evolves to emit `#[expect]` in schema-specific contexts (e.g.,
only when the schema is known to produce a pattern that triggers the lint), the
verification approach would need to change.

### Verification test added

`baseline_test.rs::generated_code_has_lint_suppressions()` — asserts that all
10 crate-level `#[allow(...)]` attributes and the item-level
`#[allow(unused_unsafe)]` are present in the generated output for the example
schema. This is a structural test that documents and verifies the current lint
suppression contract.

### Acceptance criteria assessment

- [x] Audit whether `#[allow]` or `#[expect]` is used in generated code
- [x] Determine if `#[expect]` would catch stale suppressions (it would, but
      it would also produce false positives for schema-dependent lints)
- [x] Write a verification test that documents the current lint suppression
      contract
- [x] Document findings in this todo

## References

- Generated lint attributes: `sbe/src/codegen.rs` lines 146-155
- Verification test: `sbe/tests/baseline_test.rs`
- CI workflow: `.github/workflows/ci.yml` line 32
- `Cargo.toml` line 24: `expect_used = "warn"`
