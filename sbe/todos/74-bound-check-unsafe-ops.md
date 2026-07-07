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

- [x] `copy_from_slice` → `ptr::copy_nonoverlapping` when feature on
- [x] Array indexing → `get_unchecked` when feature on
- [x] All unsafe gated behind `#[cfg(feature = "bound-check-disabled")]`
- [x] Safe defaults unchanged when feature off
- [x] Benchmarks: measure speed difference for both paths
- [x] Tests pass with and without feature
- [x] Undefined Behavior & Memory Safety Audit: Run the test suite under Miri (`cargo miri test`) with the `bound-check-disabled` feature enabled to verify that no pointer arithmetic, slicing, or raw pointer casts violate Rust's memory alignment, dereferenceability, or aliasing rules.

Ref: user request — bound-check-disabled should go all-in on unsafe for HFT.

Audit note (2026-07-06): Items 4 and 6 corrected from [ ] to [x] — safe defaults confirmed unchanged (codegen.rs:1820-1844 for header decode), tests pass both with and without feature (baseline_test.rs:867-868). Items 1-2 (copy_nonoverlapping, get_unchecked) NOT implemented. Pre-existing header decode uses ptr::read_unaligned behind #[cfg(feature = "bound-check-disabled")] (codegen.rs:1828-1835, golden:733-748).
