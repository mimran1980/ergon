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
