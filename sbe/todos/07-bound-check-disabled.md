# Bound-check-disabled + unchecked fast paths

**Blocked by:** `03-group-vardata-wire-parity`

Feature gate `bound-check-disabled` flips default paths to `_unchecked`
internally. Per-call `unsafe fn …_unchecked` entrypoints. API surface identical
across the feature; only subtracts branches. Crate stays safe-by-default.
**Status: DESIGN / ROADMAP**


## Current verification status (2026-07-08)

Earlier verification runs failed at generated-code stability and the
feature-enabled test command:

```sh
RUSTC_WRAPPER="" cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1
```

Historical root cause observed in generated test crates: non-const
`read_bytes` / `write_bytes` helpers were called from generated `const fn`
paths. The policy is now to remove constness from runtime buffer accessors
rather than constrain the fast read/write helpers.

Current release-gate docs record the default and feature-enabled `ergosbe`
test commands as passing. Keep this todo open for the remaining unchecked-path
routing and generated lint policy work below.

## Acceptance criteria

- [x] `bound-check-disabled` feature flag in generated code
- [x] `_unchecked` variants on all structural entrypoints
- [ ] Default `Iterator` impls route through checked → `_unchecked` path
- [ ] Generated module `unsafe_code = "forbid"` only when unchecked disabled
- [ ] Tests: both feature states produce identical field values
- [x] Docs: safety contracts on every `_unchecked` function

Ref: `design/DECISIONS.md` §11 slice 10.


## Verification / Unit Testing
- [ ] Create or repair a unit test `test_bounds_checking_switch` that compiles a schema twice (with and without `bound-check-disabled` feature enabled) and verifies that bounds checks are compiled out when active.
