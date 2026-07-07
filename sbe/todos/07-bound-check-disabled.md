# Bound-check-disabled + unchecked fast paths

**Blocked by:** `03-group-vardata-wire-parity`

Feature gate `bound-check-disabled` flips default paths to `_unchecked`
internally. Per-call `unsafe fn …_unchecked` entrypoints. API surface identical
across the feature; only subtracts branches. Crate stays safe-by-default.

## Acceptance criteria

- [x] `bound-check-disabled` feature flag in generated code
- [x] `_unchecked` variants on all structural entrypoints
- [x] Default `Iterator` impls route through checked → `_unchecked` path
- [x] Generated module `unsafe_code = "forbid"` only when unchecked disabled
- [x] Tests: both feature states produce identical field values
- [x] Docs: safety contracts on every `_unchecked` function

Ref: `design/DECISIONS.md` §11 slice 10.


## Verification / Unit Testing
- [x] Create a unit test `test_bounds_checking_switch` that compiles a schema twice (with and without `bound-check-disabled` feature enabled) and verifies that bounds checks are compiled out when active.
