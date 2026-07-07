# Fixed-entry group fast path with `as_chunks()`

When a group entry has no nested groups or var-data tail, expose indexed access
and chunk-backed iteration over `&[[u8; BLOCK_LENGTH]]` via `slice::as_chunks`.
This is the common order-book shape. DECISIONS.md §9.

## Status

🔲 Not started

## Acceptance criteria

- [x] Detect groups where entry has no nested groups and no var-data ("fixed-entry")
- [x] Generate `fn as_chunks(&self) -> Result<&[[u8; BLOCK_LENGTH]], DecodeError>` on fixed-entry group decoders
- [x] Generate `fn entry_at(&self, index: usize) -> Result<EntryDecoder<'a>, DecodeError>` for random access (generated as `nth()`)
- [x] Chunk-backed iteration removes repeated stride/bounds arithmetic
- [x] The typed entry decoder still reads field-by-field (chunk is just a fixed-size backing window)
- [x] Test: decode order-book-shaped group via `as_chunks()`
- [x] Test: `entry_at(0)` and `entry_at(len-1)` return correct entries
- [x] Test: `entry_at(len)` returns error (out of bounds)
- [ ] Benchmark: `as_chunks()` vs standard iterator for fixed-entry groups (deferred — C criterion, needs todo 06 benchmark infra)
- [x] Golden file updated

## Dependencies

- `03-group-vardata-wire-parity` — group decoder foundation
- `44-group-skip-and-index`

## Notes

DECISIONS.md §9 specifies this as a core helper. "This is the common order-book
shape and removes repeated stride/bounds arithmetic." `slice::as_chunks` is
stable in the MSRV.
