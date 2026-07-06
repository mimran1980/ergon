# raw_ accessors for composite/enum/set types inside group entries

**Blocked by:** `02-composite-enum-set-wire-parity`

Group entry decoders only offer high-level `foo()` accessors that collapse
optional null and version absence into `Option<T>`. HFT hot loops inside
groups need `raw_foo()` accessors that return the raw wire value.

## Acceptance criteria

- [ ] `raw_foo()` on group entry decoders for composite fields
- [ ] `raw_foo()` on group entry decoders for enum fields
- [ ] `raw_foo()` on group entry decoders for set fields
- [ ] `unsafe fn raw_foo_unchecked()` variants where bounds-check-disabled
- [ ] Consistent with message-level raw_ accessor naming and semantics

Ref: gap analysis (todo 51), DECISIONS.md §8 (raw accessors in HFT hot loops).
