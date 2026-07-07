# Var-data tail offset caching (avoid re-walking)

**Blocked by:** none (codegen only)
**Ref:** Aeron perf audit (todo 105, gap #3)

## Problem

ErgoSBE's var-data accessors on message decoders call `tail_offset_N()`
which walks ALL previous tail sections:

```rust
fn tail_offset_2(&self) -> Result<usize, DecodeError> {
    let start = self.tail_offset_1()?;  // walks group dimensions
    // ... read var-data header, compute extent
}

fn tail_offset_1(&self) -> Result<usize, DecodeError> {
    let start = self.tail_offset_0()?;  // base offset
    // ... read group dimension header, walk entries
}
```

Each call is O(N) where N is the number of preceding tail sections.
If the user reads 3 var-data fields, the 1st walks 0 sections, the
2nd walks 1, the 3rd walks 2 — total O(N²).

Aeron uses a mutable `limit` that advances naturally. No re-walking.

This is related to todo 109 (group iteration lazy tail) but distinct:
even without group iteration, reading multiple var-data fields on a
message decoder incurs repeated tail walking.

## Design

### Option A: Compute and cache all tail offsets at wrap time

In `wrap_and_apply_header()`, pre-compute all tail offsets and store
them in the decoder struct:

```rust
pub struct CarDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    tail_offsets: [usize; 3],  // pre-computed
}
```

**Pros**: O(1) for every accessor. Simple.
**Cons**: Larger struct. Tail offsets computed even if never used.
Number of tail sections varies per message — need const generic or
fixed upper bound.

### Option B: Lazy cache (first access computes, subsequent reuse)

Use `Option<usize>` cache fields, initialized to `None`, filled on
first access:

```rust
fn tail_offset_2(&self) -> Result<usize, DecodeError> {
    if let Some(cached) = self.tail_2_cache {
        return Ok(cached);
    }
    let val = /* compute */;
    self.tail_2_cache = Some(val);  // needs &mut self
    Ok(val)
}
```

**Pros**: Pay only for what you use. No struct size blowup.
**Cons**: Needs `&mut self` or interior mutability (`Cell`).

### Option C: `Cell<Option<usize>>` for interior mutability

Use `Cell` to cache without `&mut self`:

```rust
pub struct CarDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    tail_2_cache: Cell<Option<usize>>,
}
```

**Pros**: No API break. `&self` accessors still work. `Copy` preserved.
**Cons**: `Cell` import. One field per tail section. Struct size grows.

### Recommendation: Option C (ponytail)

`Cell<Option<usize>>` adds zero overhead on the happy path (the `Cell`
is just a `usize` underneath, and `Option<usize>` niche-optimises to
the same size). The first access computes; subsequent accesses are a
single branch on the cached value. `Copy` is preserved because `Cell`
is `Copy`.

For messages with few tail sections (typical: 0-5), the struct size
increase is negligible (0-5 words = 0-40 bytes).

## Acceptance criteria

- [x] Var-data tail offsets cached via `Cell<Option<usize>>` on
  message decoders
- [x] First access computes and stores; subsequent accesses return
  cached value
- [x] `Copy` trait preserved on decoder structs
- [x] No `&mut self` on accessor methods
- [x] Benchmark: var-data accessor latency O(1) regardless of
  position in the tail sequence
- [x] Golden file stability test passes
- [x] No regression in baseline test suite
