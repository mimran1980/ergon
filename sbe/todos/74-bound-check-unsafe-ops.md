# bound-check-disabled: also enable unsafe operations for extra speed

**Blocked by:** `07-bound-check-disabled`

When `bound-check-disabled` feature is active, generate code that uses unsafe
primitives for maximum throughput. The feature is opt-in and explicitly for
HFT users who accept the safety trade-off.
**Status: ACTIVE / FEATURE-GATED INTERNALS**

**Decision after todo-coherence recheck (2026-07-08):** keep the fast-path
internals active, but do not use this todo to add broad public `_unchecked`
methods. The feature should route through typed buffer helpers or localized
unsafe internals while preserving the public API.


## Current verification status (2026-07-08)

The full default workspace command must pass, and feature-enabled tests must be
rerun after helper changes. If generated `E0015` const-helper errors recur,
remove constness from runtime buffer accessors; do not slow the unsafe helper
path to preserve const evaluation. Do not claim the unsafe feature path is
verified until:

```sh
RUSTC_WRAPPER="" cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1
```

passes again.

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
- [x] Safe defaults unchanged when feature off
- [ ] Benchmarks: measure speed difference for both paths
- [ ] Tests pass with and without feature
- [ ] Undefined Behavior & Memory Safety Audit: Run the test suite under Miri (`cargo miri test`) with the `bound-check-disabled` feature enabled to verify that no pointer arithmetic, slicing, or raw pointer casts violate Rust's memory alignment, dereferenceability, or aliasing rules.

Ref: user request — bound-check-disabled should go all-in on unsafe for HFT.

Audit note (2026-07-06): safe defaults were confirmed unchanged at that time.
The statement that tests pass both with and without the feature is now stale as
of 2026-07-08 because todo 122 introduced non-const helpers that generated
const callsites still use. Items 1-2 (`copy_nonoverlapping`, `get_unchecked`)
remain NOT implemented.
