# Buffer verification function for production readiness

**Blocked by:** `01-scalar-wire-parity`

FlatBuffers generates `Verify*Buffer()` that validates the entire buffer
structure before any field access. This catches malformed, truncated, or
corrupt frames early — before they cause panics or silent data corruption.

## What to generate

For each message, emit a `verify` function:

```rust
impl CarDecoder<'_> {
    pub fn verify(buf: &[u8]) -> Result<(), VerifyError> {
        // 1. Check header present
        // 2. Check block_length is reasonable
        // 3. Verify all group dimension headers
        // 4. Verify all var-data length prefixes
        // 5. Verify no group/var-data extends past buffer end
    }
}
```

Returns a structured `VerifyError` with the exact field/offset that failed.

## Acceptance criteria

- [ ] `pub fn verify(buf: &[u8]) -> Result<(), VerifyError>` per message
- [ ] Validates header size, block_length bounds, group dim headers,
  var-data length prefixes
- [ ] Checks that no field extends past buffer end
- [ ] `VerifyError` carries field name and offset context
- [ ] Zero allocation — all checks are bounds reads
- [ ] Tests: valid fixture passes verify, truncated fixture fails with
  expected error field

Ref: FlatBuffers `Verify*Buffer()` pattern, production feed validation.

## Verification / Unit Testing
- [ ] Write a test `test_buffer_verify_function` in `sbe/tests/integration_tests.rs` that:
  1. Calls `CarDecoder::verify` with a valid encoded message buffer and asserts it returns `Ok(())`.
  2. Calls `CarDecoder::verify` with a truncated buffer (e.g., body truncated, or group count claiming entries that aren't there) and asserts it returns `Err(VerifyError::MessageTooShort)` or `Err(VerifyError::GroupDimOutOfBounds)`.
