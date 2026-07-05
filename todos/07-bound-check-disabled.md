# Bound-check-disabled + unchecked fast paths

**Blocked by:** `03-group-vardata-wire-parity`

Feature gate `bound-check-disabled` flips default paths to `_unchecked`
internally. Per-call `unsafe fn …_unchecked` entrypoints. API surface identical
across the feature; only subtracts branches. Crate stays safe-by-default.

## Acceptance criteria

- [ ] `bound-check-disabled` feature flag in generated code
- [ ] `_unchecked` variants on all structural entrypoints
- [ ] Default `Iterator` impls route through checked → `_unchecked` path
- [ ] Generated module `unsafe_code = "forbid"` only when unchecked disabled
- [ ] Tests: both feature states produce identical field values
- [ ] Docs: safety contracts on every `_unchecked` function

Ref: `design/DECISIONS.md` §11 slice 10.
