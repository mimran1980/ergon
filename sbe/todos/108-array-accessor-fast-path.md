# Array accessor non-const fast path

**Blocked by:** none
**Ref:** Aeron perf audit (todo 105, gap #1)

## Problem

Array field accessors (fixed-length arrays like `some_numbers: [u32; 4]`,
`vehicle_code: [u8; 6]`) are generated as `const fn` using while-loop byte copies:

```rust
pub const fn some_numbers(&self) -> Result<[u32; 4], DecodeError> {
    let mut res = [0u32; 4];
    let mut idx = 0;
    while idx < 4 {
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 { bytes[j] = self.buf[offset + j]; j += 1; }
        res[idx] = u32::from_le_bytes(bytes);
        idx += 1;
    }
    Ok(res)
}
```

This is because `const fn` cannot use slice indexing or `try_into()`. But
this `const fn` pattern leaks into the **hot path** — every array field read
does 4 × 4 = 16 iterations of byte-by-byte copying.

Aeron Rust SBE generates 4 unrolled `get_u32_at()` calls with zero bounds
checks:
```rust
pub fn some_numbers(&self) -> [u32; 4] {
    let buf = self.get_buf();
    [buf.get_u32_at(self.offset + 12),
     buf.get_u32_at(self.offset + 12 + 4), ...]
}
```

**Impact**: ErgoSBE array accessors compile to 2-4× more instructions than
Aeron's. For a hot decode loop reading arrays, this is a measurable perf hit.

## Design

Generate TWO accessors per array field:

1. **Safe `fn foo() -> [T; N]`** — no `Result`, no bounds check on individual
   elements (the message was already verified, or the user accepts the risk).
   Uses `copy_from_slice` + `from_le_bytes` (const-stable since Rust 1.88)
   for the copy, but is NOT `const fn`. This is the hot-path variant.

2. **`const fn raw_foo() -> [T; N]`** — the existing while-loop pattern,
   `const fn` only. Used in const contexts, `build.rs` const assertions, etc.

Actually, `copy_from_slice` + `from_le_bytes` became const-stable in Rust 1.88,
so the safe path CAN be `const fn` now. If the MSRV permits, we can just
replace the while-loop with `copy_from_slice` universally.

Check: `const fn` with `copy_from_slice`:
```rust
pub const fn some_numbers(&self) -> Result<[u32; 4], DecodeError> {
    let offset = self.pos + 12;
    if offset + 16 > self.buf.len() {
        return Err(DecodeError::BufferTooShort { ... });
    }
    let bytes: [u8; 16] = /* copy_from_slice at offset */;
    Ok([
        u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
    ])
}
```

Wait — `try_into()` on slices is NOT const-stable. The subslice pattern is
the blocker. The while-loop is the only `const fn`-compatible approach for
subslicing.

**Revised design**: Generate THREE variants:
1. **`fn foo() -> [T; N]`** — non-const, uses `copy_from_slice` + subslicing.
   Bounds check before the copy. HOT PATH.
2. **`const fn raw_foo() -> [T; N]`** — const fn, while-loop pattern. No
   bounds check (trust the buffer). CONST contexts only.
3. **`const unsafe fn foo_unchecked() -> [T; N]`** — const fn, direct
   `copy_from_slice` without bounds check. UNSAFE.

Actually, ponytail: the simplest thing that works. Just generate the
non-const fast path alongside the existing const raw_. The const raw_
already exists. Just add:

```rust
#[inline]
pub fn some_numbers(&self) -> [u32; 4] {
    let offset = self.pos + 12;
    let bytes: [u8; 16] = self.buf[offset..offset + 16].try_into().unwrap();
    [
        u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
    ]
}
```

The bounds check is the `self.buf[offset..offset+16]` which panics on OOB.
Users who verify their buffer first (which `wrap()` already does for the
message body) get infallible array reads.

## Acceptance criteria

- [ ] `_unchecked` array accessors use bulk `copy_from_slice` + unrolled element
  parsing instead of per-element while-loop (both group entry and message decoder)
- [x] The existing `const fn raw_` accessors are preserved for const contexts
- [ ] Benchmarks show array accessor latency within 10% of Aeron's
  unrolled reads
- [x] Golden file stability test passes
- [x] No regression in baseline test suite

## Status

The `_unchecked` variant now does one bulk `copy_from_slice` of the full
array, then unrolled element-by-element `from_{le,ne,be}_bytes` calls on the
copied buffer. This removes the inner while-loop per element. The safe
`const fn` remains byte-by-byte (const fn limitation — slice indexing not
const-stable).
