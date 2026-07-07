# Remove _unchecked methods — feature flag makes them redundant

**Ref:** user request. Bound-check-disabled feature gates the internal path.

## Problem

Every field accessor generates three methods:
1. `serial_number() -> u64` — safe, with `.try_into().unwrap()` on slice
2. `serial_number_unchecked() -> u64` — unsafe, `from_raw_parts`
3. `raw_serial_number() -> u64` — calls _unchecked

The `_unchecked` variant exists so users can opt into speed per-call. But the
`bound-check-disabled` feature flag already provides this — enable the feature
and ALL accessors become fast. Having both is API bloat.

## Design

Remove `_unchecked` methods. The feature flag gates the internal implementation:

```rust
pub fn serial_number(&self) -> u64 {
    let offset = self.pos + 0;
    #[cfg(not(feature = "bound-check-disabled"))]
    { u64::from_le_bytes(self.buf[offset..][..8].try_into().unwrap()) }
    #[cfg(feature = "bound-check-disabled")]
    unsafe {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 8));
        u64::from_le_bytes(bytes)
    }
}
```

`raw_` is also removed: with `bound-check-disabled`, the main accessor IS the
raw path. `raw_` can stay for array/enum/set types (where it has different
semantics — returns the underlying integer type).

## Scope

### Remove
- All `_unchecked` methods on message decoders (scalar, array, composite, enum, set)
- All `_unchecked` methods on group entry decoders
- The `raw_` aliases that just delegate to `_unchecked`

### Keep
- `raw_` on enum/set/composite types (semantic difference — returns underlying repr)
- `_unchecked` on array types (different return type: `[T;N]` vs `Result<[T;N], E>`)
  Actually, arrays: if feature-gated, the safe path also returns `[T;N]`. Remove.

## Acceptance criteria
- [ ] `_unchecked` methods removed from scalars, composites, enums, sets
- [ ] Feature flag gates internal bounds check, API stays identical
- [ ] Golden file stability test passes
- [ ] Baseline tests pass
- [ ] `cargo test --features bound-check-disabled` passes
- [ ] No `.unwrap()` in safe path body — signatures already return `T` not `Result<T>`
