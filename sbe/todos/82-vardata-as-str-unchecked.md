# `unsafe fn as_str_unchecked()` on var-data fields

Generate `unsafe fn {field}_as_str_unchecked() -> &'a str` on var-data fields with
`characterEncoding`. This is the zero-cost UTF-8 skip for HFT hot loops, distinct from
bounds-checking (`bound-check-disabled`). Specified in DECISIONS.md §3.

**Status:** PARKED / REJECTED FOR DEFAULT API

**Decision after todo-coherence recheck (2026-07-08):** keep this parked unless
a benchmark shows UTF-8 validation is material on real feed shapes. The current
API-simplicity direction deliberately avoids broad unchecked helpers. Users who
need this escape hatch can write
`unsafe { core::str::from_utf8_unchecked(field_bytes) }` at the call site.

## Acceptance criteria

- [ ] `unsafe fn {field}_as_str_unchecked(&self) -> &'a str` generated for var-data with `characterEncoding`
- [ ] Uses `core::str::from_utf8_unchecked()` internally — zero-cost
- [ ] Safety contract documented in rustdoc: caller must ensure bytes are valid UTF-8
- [ ] Distinct from `_unchecked()` bounds-checking methods (different escape hatch)
- [ ] Test: valid UTF-8 → correct string
- [ ] Test: compile-time check that it requires `unsafe` block
- [ ] Golden file updated
- [ ] Benchmark comparing `as_str()` vs `as_str_unchecked()` latency

## Dependencies

- `03-group-vardata-wire-parity` — var-data foundation
- `16-varstring-encoding-fix`

## Notes

- Historical DECISIONS.md wording required this, but the current design has
  been updated: default generated var-data APIs stay safe and small.
- If this returns, it must remain distinct from `bound-check-disabled` because
  UTF-8 validity and buffer bounds are different safety contracts.
