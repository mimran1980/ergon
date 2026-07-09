# Group iteration: lazy tail scanning for entries with var-data

**Blocked by:** none (pure codegen)
**Ref:** Aeron perf audit (todo 105, gap #2)
**Status: ACTIVE / PERFORMANCE FIX CANDIDATE**


## Problem

When a group entry has var-data tails (var-string fields, nested groups),
ErgoSBE's `Iterator::next()` eagerly computes the full encoded length of
every entry:

```rust
fn next(&mut self) -> Option<Self::Item> {
    let entry = EntryDecoder::wrap(self.buf, self.pos, self.acting_version);
    let size = entry.encoded_length()?;  // ← scans ALL tail sections
    self.pos += size;
    Some(entry)
}
```

`encoded_length()` calls `tail_offset_N()` which walks through every
var-data and nested-group tail section, reading dimension headers,
bounds-checking, and computing offsets. For a group with 50 entries each
having a var-string field, this does 50 extra var-data header reads and
bounds checks — even if the user never reads the var-string.

Aeron's `advance()` simply bumps by `block_length`:
```rust
pub fn advance(&mut self) -> SbeResult<Option<usize>> {
    self.offset = parent.get_limit();
    parent.set_limit(self.offset + self.block_length as usize);
    Ok(Some(index))
}
```

No per-entry tail scanning. Var-data is accessed lazily via separate
decoders that move the parent limit.

**Impact**: For groups with var-data entries, iteration is O(N × M) where
M is the number of tail sections per entry. Aeron is O(N).

## Design

### Option A: Mutable limit (Aeron's approach)

Add a mutable `limit` field to the group decoder. Entries inherit it.
When a var-data accessor on an entry is called, it advances the limit.
The next `next()` call picks up from the new limit.

```rust
pub struct FuelFiguresDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    limit: usize,   // ← NEW: mutable limit, advanced by var-data reads
    ...
}
```

**Pros**: Matches Aeron's proven approach. Zero-overhead iteration.
**Cons**: `&mut self` on var-data accessors (API break). Entry decoder
loses `Copy`. More complex state management.

### Option B: Cached tail lengths

Compute the tail length once per entry and cache it. The first `next()`
call scans tails; subsequent calls reuse cached values for position
tracking.

```rust
fn next(&mut self) -> Option<Self::Item> {
    let entry = EntryDecoder::wrap(self.buf, self.pos, ...);
    let size = self.tail_cache[self.count - 1].unwrap_or_else(|| {
        let s = entry.encoded_length().unwrap_or(0);
        self.tail_cache[self.count - 1] = Some(s);
        s
    });
    self.pos += size;
    Some(entry)
}
```

**Pros**: No API change. Iterator stays `&mut self`. Entry stays `Copy`.
**Cons**: Allocation for cache (N entries). Still scans on first access.
Complexity for non-trivial.

### Option C: Fast iterator (ponytail)

The simplest fix: provide a separate `iter_fast()` method on the GROUP
decoder (not the entry decoder) that advances by `ENTRY_BLOCK_LENGTH`
only, trusting the buffer:

```rust
pub fn iter_fast(&mut self) -> FastIter<'a, '_> {
    FastIter { decoder: self }
}

impl Iterator for FastIter<'a, '_> {
    type Item = EntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.decoder.count == 0 { return None; }
        let entry = EntryDecoder::wrap(self.decoder.buf, self.decoder.pos, ...);
        self.decoder.pos += Self::ENTRY_BLOCK_LENGTH;
        self.decoder.count -= 1;
        Some(entry)
    }
}
```

Users who verify their buffer (or don't care about var-data) use
`iter_fast()`. The standard `Iterator` impl stays safe.

**Pros**: Zero-overhead. No API break. User explicitly opts into speed.
**Cons**: Two iterator paths. User must know which to use.

### Recommendation: Option C (ponytail)

It's the minimal change: one new method, no API break, no state
management. Users who verified their message get the fast path by
calling `.iter_fast()` instead of relying on the `Iterator` impl.

Actually, even simpler: we already HAVE this for `total_tail == 0`
groups. The `Iterator::next()` already uses `ENTRY_BLOCK_LENGTH` for
fixed-entry groups. The gap is ONLY for groups where entries have tails.

**Even simpler fix**: For groups with tails, keep the safe `Iterator`
impl, but ALSO generate `iter_fast()` that skips tail scanning.
Document that `iter_fast()` is correct when the user has verified
the buffer or doesn't access var-data fields on entries.

### Stronger follow-up: ordered tail cursor

Todo 130 adds a type-state cursor for ordered tail traversal. That can be the
safe, schema-order path for users who need entry-level var-data or nested groups
without repeated rescans. `iter_fast()` remains the minimal trusted-buffer path;
`TailCursor` is the safer API for production code that wants compile-time order.

## Acceptance criteria

- [ ] `iter_fast()` method generated on group decoders where
  `total_tail > 0`
- [ ] `iter_fast()` advances by `ENTRY_BLOCK_LENGTH` without tail
  scanning
- [ ] Existing `Iterator` impl preserved (safe, scans tails)
- [ ] Benchmark: `iter_fast()` within 10% of Aeron's `advance()` loop
- [ ] Compare with todo 130 ordered tail cursor; document which API should be
      preferred for trusted fixed-entry scans versus schema-order tail reads
- [x] Golden file stability test passes
- [x] No regression in baseline test suite
