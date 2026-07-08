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
**Status: DONE**


## Acceptance criteria

- [x] Replace all `if buf.len() < ... { return Err(...) }` patterns in non-`const fn` decode methods with `.ok_or_else()?`
- [x] Pure `const fn` methods left unchanged; runtime buffer-reading methods
      can drop constness to use `.get()`/`?`/slice fast paths
- [x] Templates updated:
  - `wrap_and_apply_header` (decoder)
  - `decode_frame`
  - Group decoder `wrap`
- [x] Works with `copy_from_slice` (todo 29) — one bounds check, then `copy_from_slice`
- [x] Generated code uses `?` operator in decode entry points (no manual early returns)
- [x] No performance regression — verified by benchmark
- [x] All existing tests pass
