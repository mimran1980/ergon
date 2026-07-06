# Struct-of-Arrays (SoA) columnar access for repeating groups

**Blocked by:** `44-group-skip-and-index`

FlatBuffers doesn't auto-generate SoA but the pattern is proven 2-3x faster
for market data feeds. When iterating 50 price levels, reading all prices
contiguously (one cache line) is much faster than striding through interleaved
price/size structs.

## What to generate

For repeating groups where all entries have the same fixed-size layout (no
var-data tail), generate columnar access methods:

```rust
// Group: asks (price: int64, size: int64)
// Entry layout: [price: 8 bytes][size: 8 bytes] = 16 bytes per entry

impl AsksGroupDecoder<'_> {
    /// View all prices as a contiguous slice — one memcpy per cache line.
    pub fn prices_as_slice(&self) -> &[i64] { ... }

    /// View all sizes as a contiguous slice.
    pub fn sizes_as_slice(&self) -> &[i64] { ... }
}
```

Implementation: when a group has N entries of ENTRY_BLOCK_LENGTH bytes each,
the memory at `buf[start..start + N * ENTRY_BLOCK_LENGTH]` is interleaved.
Use `slice::as_chunks` (stabilised in Rust 1.77) to view every-other-8-byte
chunk as `&[i64]`.

## Acceptance criteria

- [ ] Detect fixed-size groups (no var-data tail on entries)
- [ ] Generate `{field}_as_slice() -> &[T]` for each primitive field
- [ ] Generate `{field}_as_slice() -> &[[u8; N]]` for fixed-size composite fields
- [ ] Zero-copy — returns a slice pointing into the original buffer
- [ ] Benchmark: SoA access vs AoS iteration for 50-entry group
- [ ] Non-fixed-size groups: return `None` or fall back to iterator

## Performance rationale

Market data feeds at 50 levels × 2 sides × 2 fields (price, size) = 200
scalar reads per update. SoA access loads 6 cache lines (3 per side)
instead of 50+ cache lines for interleaved access.

Ref: competitive analysis of FlatBuffers/Cap'n Proto patterns.
