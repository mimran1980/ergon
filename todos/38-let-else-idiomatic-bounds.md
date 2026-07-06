# `let-else` for idiomatic bounds checks

**Blocked by:** `01-scalar-wire-parity`

Replace manual `if buf.len() < needed { return Err(...) }` with `let-else`
and `?` throughout generated decode methods. Same machine code, reads like
idiomatic Rust.

```rust
// Before:
if self.buf.len() < self.pos + offset + 2 {
    return Err(DecodeError::BufferTooShort { needed, available });
}
let mut bytes = [0u8; 2];
// ... manual copy loop ...

// After:
let rest = self.buf.get(self.pos + offset..).ok_or_else(|| {
    DecodeError::BufferTooShort { field: "modelYear", needed: 2, available: self.buf.len() - self.pos - offset }
})?;
let bytes: [u8; 2] = rest[..2].try_into().unwrap();
```

Benefits: `?` propagates errors naturally, `unwrap()` here is safe (we just
proved the slice is big enough), and the generated code reads like hand-written
Rust. Stable since Rust 1.65.

## Acceptance criteria

- [ ] Replace all `if buf.len() < ... { return Err(...) }` patterns with `.ok_or_else()?`
- [ ] Works with `copy_from_slice` (todo 29) — one bounds check, then `copy_from_slice`
- [ ] Generated code uses `?` operator throughout (no manual early returns)
- [ ] No performance regression — same machine code, verified by benchmark
- [ ] All existing tests pass
