# todo 72: group encoder add() silently exceeding count

## Status: DONE

## Problem

The group encoder's `add()` method returned `Ok(())` when the caller exceeded the
declared `count`, silently truncating data. The caller had no way to detect that
entries were being dropped.

## Fix

1. **`crates/ergosbe/src/codegen.rs`**: Added a `GroupFull` variant to `EncodeError` in
   `generate_sbe_rt_src()`:

   ```rust
   pub enum EncodeError {
       BufferTooShort { needed: usize, available: usize },
       VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
       GroupFull { declared: u16, attempted: u16 },
   }
   ```

2. **`crates/ergosbe/src/codegen.rs`**: Changed the `add()` method in
   `generate_group_encoder` to return `Err` instead of `Ok(())` when the
   declared count is exceeded:

   ```rust
   // before
   if self.written >= self.count {
       return Ok(());
   }

   // after
   if self.written >= self.count {
       return Err(sbe_rt::EncodeError::GroupFull {
           declared: self.count,
           attempted: self.written + 1,
       });
   }
   ```

3. **`crates/ergosbe/tests/golden/car_example.rs`**: Regenerated golden file.

## Verification

All core tests pass: 7 baseline (decode, encode, display, byte-exact, constants),
16 integration (block length, extension, group, since, variable, edge cases),
1 stability/golden match. Proptest failures are pre-existing infrastructure issues
(sccache/temp dir CWD), unrelated to this change.
