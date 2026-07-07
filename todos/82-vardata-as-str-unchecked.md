# `unsafe fn as_str_unchecked()` on var-data fields

Generate `unsafe fn {field}_as_str_unchecked() -> &'a str` on var-data fields with
`characterEncoding`. This is the zero-cost UTF-8 skip for HFT hot loops, distinct from
bounds-checking (`bound-check-disabled`). Specified in DECISIONS.md §3.

**Status:** not started

## Acceptance criteria

- [x] `unsafe fn {field}_as_str_unchecked(&self) -> &'a str` generated for var-data with `characterEncoding`
- [x] Uses `core::str::from_utf8_unchecked()` internally — zero-cost
- [x] Safety contract documented in rustdoc: caller must ensure bytes are valid UTF-8
- [x] Distinct from `_unchecked()` bounds-checking methods (different escape hatch)
- [x] Test: valid UTF-8 → correct string
- [x] Test: compile-time check that it requires `unsafe` block
- [x] Golden file updated
- [x] Benchmark comparing `as_str()` vs `as_str_unchecked()` latency

## Dependencies

- `03-group-vardata-wire-parity` — var-data foundation
- `16-varstring-encoding-fix`

## Notes

- DECISIONS.md §3 and trap 7 explicitly specify this.
- `as_str_unchecked` is `unsafe fn` (zero-cost via `str::from_utf8_unchecked`), distinct from
  `bound-check-disabled` (UTF-8 validity vs array bounds).
