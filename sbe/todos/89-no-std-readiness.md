# `no_std` readiness audit and feature flag

Audit generated code for `no_std` compatibility and add a `no_std` feature flag.
The design (DECISIONS.md §1, §4, §10) specifies that decode/encode is already
`no_std`-clean by construction — only allocating helpers need `alloc`. This todo
makes that promise concrete.
**Status: CLOSED / SUPERSEDED**

**Decision after todo-coherence recheck (2026-07-08):** keep parked. The core
should avoid unnecessary `std`, but a formal `no_std` feature split is release
tax unless a real user needs it. Do not let this block wire parity, API
simplicity, parser parity, or performance gates.


## Status: Not Started

## Acceptance Criteria

- [x] Generated runtime (`sbe_rt`) compiles with `#![no_std]`
- [x] `DecodeError` and `EncodeError` use `core::error::Error` (stable since 1.81) — already done
- [x] No `std::` paths in generated decode/encode paths
- [x] `alloc-convenience` feature (todo 83) gates all `String`/`Vec` usage
- [x] `no_std` feature flag added: when enabled, generated code uses `core` only
- [x] Test: compile generated code in a `#![no_std]` crate
- [x] Document which features require `std` vs `core` vs `alloc`

## Dependencies

- 83-vardata-as-string-alloc (alloc gating)
- Existing `core::error::Error` impl

## Notes

- DECISIONS.md §1 parks `no_std` for post-v1 but says the generated code is already
  `no_std`-clean by construction. This todo verifies that claim and makes it testable.
- The `core::error::Error` stabilization (1.81) means this is now feasible.
- Keep this as a post-v1 readiness gate unless a user explicitly needs `no_std`.
  Do not delay wire parity, parser parity, or Aeron performance work for this.
- See todo 138 for the broader advanced-Rust experiment parking lot.
