# `is_empty()` inherent method on group decoders

Add `pub fn is_empty(&self) -> bool` as an inherent method on group decoders.
DECISIONS.md §3 specifies this should be inherent (not from `ExactSizeIterator`)
because `ExactSizeIterator::is_empty` is still unstable on stable Rust.

## Status

✅ Done — already implemented with `#[inline]` in `generate_group_decoder()` at line 2523.
Added test `group_decoder_is_empty()` in `baseline_test.rs`.

## Acceptance criteria

- [x] `pub fn is_empty(&self) -> bool` generated on every group decoder
- [x] Returns `self.count == 0` (or equivalent)
- [x] Inherent method — does NOT rely on `ExactSizeIterator::is_empty()`
- [x] Consistent with `len()` from `ExactSizeIterator`
- [x] Test: empty group → `is_empty() == true`
- [x] Test: non-empty group → `is_empty() == false`
- [x] Golden file updated

## Dependencies

- `03-group-vardata-wire-parity` — group decoder foundation

## Notes

DECISIONS.md §3 explicitly calls this out: "is_empty() is an inherent method on
the group decoder, not an ExactSizeIterator override, because
ExactSizeIterator::is_empty is still unstable on stable Rust."
