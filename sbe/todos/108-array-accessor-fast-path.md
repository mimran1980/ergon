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

This is because `const fn` cannot use the same slice/`try_into()` patterns as
the runtime fast path. That `const fn` pattern leaks into the **hot path** —
every array field read does 4 × 4 = 16 iterations of byte-by-byte copying.

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

Generate the hot-path accessor as a normal `fn`, not a `const fn`. Do not keep
separate const buffer-read variants unless a real user need appears; the codec's
primary job is runtime feed decode.

Use:

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

- [x] `_unchecked` array accessors use bulk `copy_from_slice` + unrolled element
  parsing instead of per-element while-loop (both group entry and message decoder)
- [ ] Safe/runtime array accessors do not use const-only byte loops
- [ ] `raw_` array accessors are normal fast runtime methods unless a const-only
      use case is explicitly justified
- [ ] Benchmarks show array accessor latency within 10% of Aeron's
  unrolled reads
- [x] Golden file stability test passes
- [x] No regression in baseline test suite

## Status

The `_unchecked` variant now does one bulk `copy_from_slice` of the full array,
then unrolled element-by-element `from_{le,ne,be}_bytes` calls on the copied
buffer. The remaining policy is to remove const-only byte loops from safe
runtime array accessors rather than preserve `const fn`.
