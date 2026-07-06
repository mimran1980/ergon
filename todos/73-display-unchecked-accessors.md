# todo 73: Display impl uses unchecked raw_ accessors inconsistently

## Status: IN PROGRESS

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

- [ ] Write a test `test_display_impl_safety` in `sbe/tests/integration_tests.rs` that formats a decoder containing invalid/out-of-bounds data using `format!("{}", decoder)` and verifies that it does not panic and outputs safely, handling formatting errors or missing fields gracefully without using unsafe/unchecked accessors internally.
