# raw_ accessors for composite/enum/set types inside group entries

> **Superseded public-surface decision (2026-07-10):** preserve the raw-value
> semantics record, but do not generate per-field unchecked variants. Select
> trusted-input internals through one stable accessor surface.

**Blocked by:** `02-composite-enum-set-wire-parity`

Group entry decoders only offer high-level `foo()` accessors that collapse
optional null and version absence into `Option<T>`. HFT hot loops inside
groups need `raw_foo()` accessors that return the raw wire value.
**Status: DONE**


## Acceptance criteria

- [x] `raw_foo()` on group entry decoders for composite fields
- [x] `raw_foo()` on group entry decoders for enum fields
- [x] `raw_foo()` on group entry decoders for set fields
- [x] `unsafe fn raw_foo_unchecked()` variants where bounds-check-disabled
- [x] Consistent with message-level raw_ accessor naming and semantics

Ref: gap analysis (todo 51), DECISIONS.md §8 (raw accessors in HFT hot loops).

## Verification / Unit Testing
- [x] Write a test `test_raw_accessors_group_entries` in `sbe/tests/integration_tests.rs` that:
  1. Encodes a message with a repeating group where the entries have enum, set, and composite fields.
  2. Verifies that the decoder entry has `raw_` accessors for these fields (e.g. `raw_speed()`, `raw_mph()`, etc.) and that they return the raw unmapped integers.
  3. Asserts that the return value matches the expected bytes exactly without mapping optional null sentinel values.
