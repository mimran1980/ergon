# #[expect(lint)] over #[allow(lint)] in generated code

**Blocked by:** none (annotation-only)

**Status: REJECTED / DO NOT IMPLEMENT**
Generated code currently uses `#[allow(non_camel_case_types)]` etc. Rust 1.95+
supports `#[expect(lint)]` which fires a warning when the suppression is no
longer needed. This catches stale suppressions and dead code.

Historical DECISIONS.md wording preferred `#[expect]` over `#[allow]`, but the
current generated-code policy rejects that change because schemas vary in which
lints they trigger.

## Acceptance criteria

- [x] Replace `#[allow(non_camel_case_types)]` with `#[expect(non_camel_case_types)]` — WONT DO (design decision 2026-07-06). `#[allow]` is intentional because `#[expect]` produces false-positive stale-suppression warnings when schemas don't trigger the suppressed lint.
- [x] Replace `#[allow(non_snake_case)]` with `#[expect(non_snake_case)]` — WONT DO (same)
- [x] Replace `#[allow(clippy::*)]` with `#[expect(clippy::*)]` — WONT DO (same)
- [x] Replace `#[allow(dead_code)]` with `#[expect(dead_code)]` — WONT DO (same)
- [x] `#[allow(unused_imports)]` → `#[expect(unused_imports)]` — WONT DO (same)
- [x] Verify: when suppression is stale, CI warns (so we can remove it) — N/A

Ref: gap analysis (todo 51), DECISIONS.md §12.


## Verification / Unit Testing
- [x] Create a test verifying that `#[expect(...)]` warnings are produced if any lint suppression becomes stale in the generated output.

Audit note (2026-07-06): CORRECTED -- all 5 AC checkboxes demoted from [x] to [ ]. Codegen still emits #[allow(...)] NOT #[expect(...)] in generated output. Baseline test (baseline_test.rs lines 30-76) explicitly documents design decision: #[allow] is intentional because #[expect] would produce false-positive stale-suppression warnings depending on which lints a particular schema triggers. No #[expect(...)] exists anywhere in generated code.
