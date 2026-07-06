# Infallible field accessors — trust message-level validation

**Blocked by:** none (codegen only)

Per-field bounds checks and byte-copy loops are redundant when message-level
validation has already proven the buffer is large enough. The upstream Aeron
Rust SBE returns `T` directly from field accessors, not `Result<T, Error>`.

## What to change

1. **Remove per-field bounds checks from `pub fn foo()`** — return `T` directly.
   The message-level `wrap_and_apply_header` already validates the buffer.

2. **Replace byte-copy loops with direct reads**:
   ```rust
   // Before (current):
   let mut bytes = [0u8; 1];
   let mut j = 0;
   while j < 1 { bytes[j] = self.buf[offset + j]; j += 1; }
   Ok(i8::from_le_bytes(bytes))
   
   // After:
   i8::from_le_bytes(self.buf[offset..offset + 1].try_into().unwrap())
   ```

3. **Keep `try_foo() -> Result<T, DecodeError>` variants** for callers who want
   per-field validation on untrusted buffers

4. **Rename `raw_foo()` → `foo()`** (the infallible variant becomes the default)

## Acceptance criteria

- [ ] Every field accessor returns `T`, not `Result<T, DecodeError>`
- [ ] No byte-copy loops for fixed-size fields (use `from_le_bytes` directly)
- [ ] `try_foo()` checked variants exist for untrusted buffers
- [ ] All existing tests pass with updated API
- [ ] Golden file regenerated
- [ ] Compare generated output against upstream Aeron Rust SBE

Ref: user observation that upstream returns `u64` not `Result<u64, Error>`.
Our `while j < 1` byte-copy loop is embarrassing compared to upstream's
clean `from_le_bytes` pattern.
