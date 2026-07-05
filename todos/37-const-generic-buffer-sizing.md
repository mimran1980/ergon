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
- [ ] Fixed message: `ENCODED_LENGTH` == `MAX_ENCODED_LENGTH`
- [ ] Variable message: `ENCODED_LENGTH` emits compile error directing user to `MAX_ENCODED_LENGTH`
- [ ] Doc example on each message: `let mut buf = [0u8; Msg::MAX_ENCODED_LENGTH];`
- [ ] `encoded_length(&self)` runtime method returns exact encoded size for the current message
