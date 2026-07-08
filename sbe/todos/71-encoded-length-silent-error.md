# todo 71: encoded_length() silently swallowing errors
**Status: DONE**


**Status: DONE**

## Problem

The generated `EntryDecoder::encoded_length()` method used `unwrap_or()` when
computing group entry tail offsets, silently swallowing decode errors. If the
buffer was too short or the data was corrupt, the error was silently converted
to `self.pos + Self::ENTRY_BLOCK_LENGTH`, causing subsequent group entries to be
read from the wrong position.

## Fix

1. **`ergosbe/src/codegen.rs`**: Changed `EntryDecoder::encoded_length()` to
   return `Result<usize, sbe_rt::DecodeError>` instead of `usize`. The body now
   uses `?` to propagate errors:

   ```rust
   // before
   pub fn encoded_length(&self) -> usize {
       self.tail_offset_0().unwrap_or(self.pos + Self::ENTRY_BLOCK_LENGTH) - self.pos
   }

   // after
   pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
       Ok(self.tail_offset_0()? - self.pos)
   }
   ```

2. **`Iterator::next`**: Updated to handle the `Result`. On error, iteration
   stops (count set to 0) and the current entry is returned, preventing silent
   position corruption.

3. **`ergosbe/tests/golden/car_example.rs`**: Regenerated golden file.

## Verification

- [x] Write a test `test_encoded_length_error_propagation` in `sbe/tests/integration_tests.rs` that passes an incomplete/truncated buffer to `encoded_length()` and asserts it returns `Err(DecodeError::BufferTooShort)` instead of returning a garbage size.
