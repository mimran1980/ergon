# Typed iterator for as_chunks: return EntryDecoder items instead of raw bytes

**Blocked by:** none
**Ref:** user request

## Problem

`as_chunks()` on fixed-entry groups returns raw bytes:
```rust
pub fn as_chunks(&self) -> Result<&'a [[u8; 6]], DecodeError> { ... }
```

The caller gets `&[[u8; 6]]` and has to manually parse each chunk. There's no
way to iterate entries with `&self` (current `Iterator` impl needs `&mut self`
because it decrements `count`).

## Design

Add an `entries(&self)` method that returns a borrowing iterator yielding
`EntryDecoder<'a>` items without mutating the decoder:

```rust
/// Borrowing iterator over entries. Does not consume or mutate the decoder.
/// Each entry is a lightweight view — zero-copy from the buffer.
pub fn entries(&self) -> EntriesIter<'a, '_> {
    EntriesIter {
        buf: self.buf,
        pos: self.pos,
        remaining: self.count,
        _phantom: PhantomData,
    }
}

pub struct EntriesIter<'a, 'd> {
    buf: &'a [u8],
    pos: usize,
    remaining: usize,
    _phantom: PhantomData<&'d ()>,
}

impl<'a, 'd> Iterator for EntriesIter<'a, 'd> {
    type Item = EntryDecoder<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let entry = EntryDecoder::wrap(self.buf, self.pos, 0);
        self.pos += ENTRY_BLOCK_LENGTH;
        self.remaining -= 1;
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, 'd> ExactSizeIterator for EntriesIter<'a, 'd> {
    fn len(&self) -> usize { self.remaining }
}
```

This only works for `total_tail == 0` (fixed-entry groups). For groups with
tails, position correctness requires tail scanning. `iter_fast()` (todo 109)
already covers the mutable-borrow case.

### Naming

`entries()` — clear, one word, doesn't conflict with anything. Not `iter()`
because that would shadow/conflict with the `Iterator` impl. Not `as_entries()`
because it returns an iterator, not a slice.

## Acceptance criteria

- [ ] `entries()` method on group decoders with `total_tail == 0`
- [ ] `EntriesIter` borrows `&self`, returns `EntryDecoder<'a>` items
- [ ] `ExactSizeIterator` impl with correct `len()`
- [x] `as_chunks()` preserved as the raw-bytes escape hatch
- [x] Golden file regenerated and stability test passes
- [x] Baseline tests pass
