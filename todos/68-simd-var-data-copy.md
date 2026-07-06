# SIMD-accelerated var-data copy on encode/decode

**Blocked by:** `03-group-vardata-wire-parity`

No SBE implementation auto-generates SIMD paths. ErgoSBE could be first.

Var-data fields (symbols, strings) are common in crypto feeds — a 24-byte
symbol repeated in every message. Copying 24 bytes with a byte loop is 24
instructions; AVX2 can do it in 2 (32-byte load + 32-byte store).

## What to generate

```rust
#[cfg(target_feature = "avx2")]
#[inline]
unsafe fn copy_var_data_avx2(dst: &mut [u8], src: &[u8]) {
    // Use 256-bit SIMD loads/stores for chunks >= 32 bytes
    // Fall back to copy_from_slice for remainder
}

#[cfg(not(target_feature = "avx2"))]
#[inline]
fn copy_var_data_fallback(dst: &mut [u8], src: &[u8]) {
    dst[..src.len()].copy_from_slice(src);
}
```

Generated encode/decode methods use `copy_var_data` which dispatches to
the fastest available implementation at compile time via `#[cfg]`.

## Acceptance criteria

- [ ] Emit `cfg(target_feature = "avx2")` guarded SIMD copy helpers
- [ ] Generated var-data setters use SIMD path when available
- [ ] Generated var-data decoders use SIMD for `as_slice()` extraction
- [ ] Fallback to `copy_from_slice` when SIMD not available
- [ ] No `unsafe` in user-facing API — SIMD is internal detail
- [ ] Benchmark: var-data encode/decode with and without SIMD

Ref: competitive analysis — nobody auto-generates SIMD for serialization.
