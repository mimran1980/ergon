# Replace manual byte loops with `copy_from_slice`

**Blocked by:** `01-scalar-wire-parity`

Every field read in decoder and encoder templates uses:
```rust
let mut bytes = [0u8; N];
let mut j = 0;
while j < N {
    bytes[j] = self.buf[self.pos + offset + j];
    j += 1;
}
```

Replace with idiomatic `copy_from_slice` or `try_into`:
```rust
let bytes: [u8; N] = self.buf[self.pos + offset..][..N].try_into().unwrap();
```

Benefits:
- LLVM vectorises `copy_from_slice` better than manual byte loops
- Generated output is ~3× shorter (less code bloat in the hot path)
- More readable generated code (auditability)
- Eliminates `while j < N` boilerplate from ~50 template locations

## Acceptance criteria

- [ ] Replace ALL manual `while j < N { bytes[j] = ... }` patterns in:
  - [ ] Decoder primitive field reads
  - [ ] Decoder composite field reads
  - [ ] Encoder scalar writes
  - [ ] Encoder composite writes
  - [ ] Fixed-size array reads
  - [ ] Group entry field reads
- [ ] Generated code compiles and passes all existing tests
- [ ] Wire output is byte-identical to before (regen-stability test catches regressions)
- [ ] Benchmark: encode/decode throughput unchanged or improved

Discovered by: generated code review agent (todos/11-generated-code-review).
