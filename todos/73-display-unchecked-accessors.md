# todo 73: Display impl uses unchecked raw_ accessors inconsistently

## Status: DONE

## Problem

The generated `Display` impl in `generate_decoder_display()` mixed `raw_*`
(unchecked, unsafe) accessors with checked accessors inconsistently. Required
primitive fields (e.g. `serial_number`, `model_year`) used the unsafe
`self.raw_field_name()` path, while optional/since-version fields used the
checked `self.field_name()` path.

Display is a debug/logging path, not a hot path. Safety and consistency
matter more than performance here.

## Fix

1. **`ergosbe/src/codegen.rs`** (`generate_decoder_display`): Replaced the
   `self.raw_{}())?;` call with `if let Ok(v) = self.{}() { write!(f, ...) }`
   for required primitive fields, matching the pattern already used for
   optional fields, groups, and var data elsewhere in the same function.

   The generated code now uses:
   ```rust
   if let Ok(v) = self.serial_number() { write!(f, "serial_number: {}", v)?; }
   ```

2. **`ergosbe/tests/golden/car_example.rs`**: Regenerated golden file.

## Verification

All tests pass: 14 unit, 16 integration + 29 issue regression, 5 property-based,
1 benchmark, 1 stability/golden match, 7 baseline parity tests.
