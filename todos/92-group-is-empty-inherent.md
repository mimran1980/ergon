# `is_empty()` inherent method on group decoders

Add `pub fn is_empty(&self) -> bool` as an inherent method on group decoders.
DECISIONS.md §3 specifies this should be inherent (not from `ExactSizeIterator`)
because `ExactSizeIterator::is_empty` is still unstable on stable Rust.

## Status

🔲 Not started

## Acceptance criteria

- [ ] `pub fn is_empty(&self) -> bool` generated on every group decoder
- [ ] Returns `self.count == 0` (or equivalent)
- [ ] Inherent method — does NOT rely on `ExactSizeIterator::is_empty()`
- [ ] Consistent with `len()` from `ExactSizeIterator`
- [ ] Test: empty group → `is_empty() == true`
- [ ] Test: non-empty group → `is_empty() == false`
- [ ] Golden file updated

## Dependencies

- `03-group-vardata-wire-parity` — group decoder foundation

## Notes

DECISIONS.md §3 explicitly calls this out: "is_empty() is an inherent method on
the group decoder, not an ExactSizeIterator override, because
ExactSizeIterator::is_empty is still unstable on stable Rust."
