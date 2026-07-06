# #[expect(lint)] over #[allow(lint)] in generated code

**Blocked by:** none (annotation-only)

Generated code currently uses `#[allow(non_camel_case_types)]` etc. Rust 1.95+
supports `#[expect(lint)]` which fires a warning when the suppression is no
longer needed. This catches stale suppressions and dead code.

DECISIONS.md §12 explicitly prefers `#[expect]` over `#[allow]`.

## Acceptance criteria

- [x] Replace `#[allow(non_camel_case_types)]` with `#[expect(non_camel_case_types)]`
- [x] Replace `#[allow(non_snake_case)]` with `#[expect(non_snake_case)]`
- [x] Replace `#[allow(clippy::*)]` with `#[expect(clippy::*)]`
- [x] Replace `#[allow(dead_code)]` with `#[expect(dead_code)]`
- [x] `#[allow(unused_imports)]` → `#[expect(unused_imports)]`
- [ ] Verify: when suppression is stale, CI warns (so we can remove it)

Ref: gap analysis (todo 51), DECISIONS.md §12.


## Verification / Unit Testing
- [ ] Create a test verifying that `#[expect(...)]` warnings are produced if any lint suppression becomes stale in the generated output.
