# Cap MAX_ENCODED_LENGTH at reasonable stack size

The generated `MAX_ENCODED_LENGTH` for the Car message is ~3GB because three
var-data fields each have `max_length=2^30`. The doc comment says
"stack-allocate with `[0u8; MAX_ENCODED_LENGTH]`" but 3GB won't fit on the
stack.

## Changes

- [x] In `codegen.rs`, add cap logic after computing `max_encoded_length`
- [x] Cap at 65536 (64KB) with warning doc comment when exceeded
- [x] Update doc to say "MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use Vec for heap"
- [x] `ENCODED_LENGTH` for fixed-size messages (already in place, no var-data)
- [x] Regenerate golden file (`cargo test update_golden -- --ignored`)
- [x] All tests pass

## Verification / Unit Testing
- [x] Write a test `test_max_encoded_length_cap` in `sbe/tests/integration_tests.rs` that:
  1. Asserts `CarDecoder::MAX_ENCODED_LENGTH` is capped at exactly 65536 bytes.
  2. Verifies that a message with a very large possible layout is capped at 65536 and doesn't overflow or generate an invalid/huge stack allocation size.
