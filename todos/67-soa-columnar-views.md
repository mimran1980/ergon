# Struct-of-Arrays (SoA) columnar access for repeating groups

**Status:** Implemented for single-field fixed-size groups.

## Design constraint

Rust's `core::slice::from_raw_parts(ptr, N)` reads elements at stride
`sizeof(T)`, which is fixed at compile time.  For multi-field entries like
`[price: i64(offset 0), size: i64(offset 8)]` in a 16-byte entry, the price data
is at byte positions 0, 16, 32, ... (stride 16), not 0, 8, 16, ... (stride 8).
`from_raw_parts` cannot express a non-unit stride, so **`{field}_as_slice() ->
&[T]` is only correct when the field occupies the entire entry** (single-field
groups).

For multi-field entries, the existing `as_chunks() -> &[[u8; ENTRY_BLOCK_LENGTH]]`
provides bulk entry access.  The caller decodes individual fields per entry in
a tight loop.

## What was generated

For fixed-size repeating groups where the entry has exactly one non-constant
field (i.e. one scalar primitive fills the entire entry block), generate:

```rust
impl FooGroupDecoder<'_> {
    pub fn field_as_slice(&self) -> Result<&[T], DecodeError> { ... }
}
```

Generated `_as_slice()` methods are:
- Zero-copy -- `core::slice::from_raw_parts` on the buffer
- Bounds-checked against the decoded entry count
- `#[inline]` for the hot path
- `unsafe` block internally with a safety comment (alignment assumed from SBE
  buffer allocation)

## Not generated (use `as_chunks()` instead)

- Multi-field entries (the field data is strided, not contiguous)
- Fixed-size primitive arrays (`[T; N]`)
- Composite fields
- Enum and Set fields

## Acceptance criteria

- [x] Detect fixed-size groups (no var-data tail on entries) -- pre-existing
- [x] Generate `{field}_as_slice() -> &[T]` for scalar primitive fields in
      single-field entries
- [x] Zero-copy -- `from_raw_parts` on the original buffer
- [x] All existing tests pass
- [ ] Generate `{field}_as_slice() -> &[[u8; N]]` for composite fields
      (deferred -- `as_chunks()` covers this case)
- [ ] Benchmark: SoA access vs AoS iteration (deferred)
- [ ] Non-fixed-size groups: iterator fallback (deferred)
