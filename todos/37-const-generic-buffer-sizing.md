# Const-generic buffer sizing

**Blocked by:** `01-scalar-wire-parity`

Every message with a known maximum size should be stack-allocatable at compile
time. Generate `MAX_ENCODED_LENGTH` and `ENCODED_LENGTH` consts so users never
guess buffer sizes.

```rust
// Stack-allocate a fixed message:
let mut buf = [0u8; Car::ENCODED_LENGTH];

// Stack-allocate a variable message (worst-case):
let mut buf = [0u8; Order::MAX_ENCODED_LENGTH];
```

For fixed messages (no groups, no var-data), `ENCODED_LENGTH` is exact. For
variable messages, it's a compile error to use `ENCODED_LENGTH` — you must use
`MAX_ENCODED_LENGTH` with worst-case sizing or `encoded_length()` at runtime.

## Acceptance criteria

- [ ] `const ENCODED_LENGTH: usize` — exact for fixed messages
- [ ] `const MAX_ENCODED_LENGTH: usize` — worst-case (header + block + max_groups * entry + max_var_data)
- [ ] Fixed message: `ENCODED_LENGTH` == `MAX_ENCODED_LENGTH` (only `ENCODED_LENGTH` emitted per design — value is identical)
- [ ] Variable message: `ENCODED_LENGTH` omitted (compile error by absence, not custom message per design)
- [ ] Doc example on each message: `let mut buf = [0u8; Msg::MAX_ENCODED_LENGTH];`
- [ ] `encoded_length(&self)` runtime method returns exact encoded size for the current message

## Verification / Unit Testing
- [ ] Create a unit test `test_const_generic_buffer_sizing` in `sbe/tests/integration_tests.rs` or `smoke_test.rs` that:
  1. Asserts `CarDecoder::ENCODED_LENGTH` (if fixed) is exactly 32 bytes (or matches the expected size).
  2. Asserts `CarDecoder::MAX_ENCODED_LENGTH` is 65536 (or the cap size/worst-case size).
  3. Verifies that `CarEncoder::wrap_and_apply_header` returns `Err(EncodeError::BufferTooShort)` when passed a buffer smaller than `ENCODED_LENGTH`.
