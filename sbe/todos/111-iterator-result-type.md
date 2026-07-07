# Iterator error handling — Result-bearing Iterator

**Blocked by:** 109 (group iteration fast path)
**Ref:** Aeron perf audit (todo 105, gap #5)

## Problem

ErgoSBE's `Iterator::next()` for group decoders swallows errors:

```rust
fn next(&mut self) -> Option<Self::Item> {
    let entry = EntryDecoder::wrap(self.buf, self.pos, ...);
    let size = match entry.encoded_length() {
        Ok(s) => s,
        Err(_) => {
            self.count = 0;         // terminate iteration
            return Some(entry);     // return potentially corrupt entry
        }
    };
    ...
}
```

On `encoded_length()` failure (buffer too short), the error is silently
dropped, the iterator terminates on the next call, and the current entry
has a wrong position — accessing its fields reads garbage.

Aeron's `advance()` returns `Result<Option<usize>>` — the error propagates.

## Design: `Iterator<Item = Result<T, E>>`

The idiomatic Rust pattern for fallible iterators is `Item = Result<T, E>`.
This is used by `std::io::Lines`, `std::fs::ReadDir`, etc.

```rust
impl<'a> Iterator for FuelFiguresDecoder<'a> {
    type Item = Result<FuelFiguresEntryDecoder<'a>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = FuelFiguresEntryDecoder::wrap(self.buf, self.pos, self.acting_version);
        match entry.encoded_length() {
            Ok(size) => {
                self.pos += size;
                self.count -= 1;
                Some(Ok(entry))
            }
            Err(e) => {
                self.count = 0;  // terminate
                Some(Err(e))     // propagate error
            }
        }
    }
}
```

### User experience

```rust
// Collect all entries, fail on first error
let entries: Vec<EntryDecoder> = decoder.collect::<Result<Vec<_>, _>>()?;

// Skip errors (trust the buffer)
for entry in decoder.filter_map(|r| r.ok()) { ... }

// Trust the buffer entirely (production HFT)
for entry in decoder.map(|r| r.unwrap()) { ... }

// Handle errors properly
for result in decoder {
    match result {
        Ok(entry) => process(entry),
        Err(e) => log::error!("corrupt entry: {e}"),
    }
}
```

### Interaction with `iter_fast()` (todo 109)

`iter_fast()` returns `Item = EntryDecoder` (infallible). It assumes the
buffer is valid and skips tail scanning entirely. Users choose:

| Method | Item type | Tail scan | Error handling |
|--------|-----------|-----------|----------------|
| `for entry in decoder` | `Result<EntryDecoder, DecodeError>` | Yes | Explicit |
| `for entry in decoder.iter_fast()` | `EntryDecoder` | No | None (trusts buffer) |

This is a BREAKING API CHANGE for groups with tails. Currently
`Iterator::Item = EntryDecoder` for all groups. After this change:
- Groups with `total_tail == 0`: `Item = EntryDecoder` (unchanged — infallible)
- Groups with `total_tail > 0`: `Item = Result<EntryDecoder, DecodeError>` (changed)

Users of fixed-entry groups see no difference. Users of tail-having
groups must handle `Result` — but they were already getting silent
error swallowing before, so this is a correctness improvement.

## Acceptance criteria

- [x] `Iterator::Item = Result<EntryDecoder, DecodeError>` for groups
  with `total_tail > 0`
- [x] `Iterator::Item = EntryDecoder` preserved for `total_tail == 0`
  (infallible fast path)
- [x] `iter_fast()` method with `Item = EntryDecoder` for all groups
  (trusts buffer, from todo 109)
- [x] Error path in `next()` returns `Some(Err(e))` instead of
  swallowing the error
- [x] Golden file stability test passes
- [x] No regression in baseline test suite
