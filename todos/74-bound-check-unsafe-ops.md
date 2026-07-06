# bound-check-disabled: also enable unsafe operations for extra speed

**Blocked by:** `07-bound-check-disabled`

When `bound-check-disabled` feature is active, generate code that uses unsafe
primitives for maximum throughput. The feature is opt-in and explicitly for
HFT users who accept the safety trade-off.

## What changes when `bound-check-disabled` is active

Currently the feature only skips `if offset > buf.len()` checks. It should
also replace safe-but-checked stdlib calls with their unchecked equivalents:

| Safe (default) | Unsafe (feature on) | Speed gain |
|---|---|---|
| `copy_from_slice` | `ptr::copy_nonoverlapping` | Skips internal bounds check + panic path |
| `buf[offset..offset+N]` | `buf.get_unchecked(offset..offset+N)` | Skips bounds check |
| `bytes.try_into().unwrap()` | `*(bytes.as_ptr() as *const [u8; N])` | Direct pointer read, no conversion |
| `from_le_bytes(bytes)` | `u64::from_le_bytes(*(&bytes as *const [u8; 8] as *const [u8; 8]))` | Already optimal on LE; identical |

The unsafe variants only trigger when `#[cfg(feature = "bound-check-disabled")]`
— the safe defaults are unchanged.

## Implementation approach

Generate both paths in codegen:

```rust
#[cfg(not(feature = "bound-check-disabled"))]
{
    self.buf[offset..offset + N].copy_from_slice(&val_bytes);
}
#[cfg(feature = "bound-check-disabled")]
{
    unsafe {
        core::ptr::copy_nonoverlapping(
            val_bytes.as_ptr(),
            self.buf.as_mut_ptr().add(offset),
            N,
        );
    }
}
```

Use a helper macro or function to keep the templates DRY.

## Acceptance criteria

- [ ] `copy_from_slice` → `ptr::copy_nonoverlapping` when feature on
- [ ] Array indexing → `get_unchecked` when feature on
- [ ] All unsafe gated behind `#[cfg(feature = "bound-check-disabled")]`
- [ ] Safe defaults unchanged when feature off
- [ ] Benchmarks: measure speed difference for both paths
- [ ] Tests pass with and without feature

Ref: user request — bound-check-disabled should go all-in on unsafe for HFT.
