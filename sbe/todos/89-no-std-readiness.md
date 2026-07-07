# `no_std` readiness audit and feature flag

Audit generated code for `no_std` compatibility and add a `no_std` feature flag.
The design (DECISIONS.md §1, §4, §10) specifies that decode/encode is already
`no_std`-clean by construction — only allocating helpers need `alloc`. This todo
makes that promise concrete.

## Status: Not Started

## Acceptance Criteria

- [ ] Generated runtime (`sbe_rt`) compiles with `#![no_std]`
- [x] `DecodeError` and `EncodeError` use `core::error::Error` (stable since 1.81) — already done
- [x] No `std::` paths in generated decode/encode paths
- [ ] `alloc-convenience` feature (todo 83) gates all `String`/`Vec` usage
- [ ] `no_std` feature flag added: when enabled, generated code uses `core` only
- [ ] Test: compile generated code in a `#![no_std]` crate
- [ ] Document which features require `std` vs `core` vs `alloc`

## Dependencies

- 83-vardata-as-string-alloc (alloc gating)
- Existing `core::error::Error` impl

## Notes

- DECISIONS.md §1 parks `no_std` for post-v1 but says the generated code is already
  `no_std`-clean by construction. This todo verifies that claim and makes it testable.
- The `core::error::Error` stabilization (1.81) means this is now feasible.
