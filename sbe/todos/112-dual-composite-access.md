# Dual composite access: eager copy + lazy flyweight

**Blocked by:** none (codegen only)
**Ref:** Aeron perf audit (todo 105, gap #7)

## Problem

ErgoSBE copies composite fields eagerly — all bytes at once into a value struct:

```rust
pub fn engine(&self) -> Engine {
    let offset = self.pos + 35;
    Engine(self.buf[offset..][..6].try_into().unwrap()) // copies 6 bytes
}
```

Aeron returns a flyweight decoder that reads from the buffer on each field access:

```rust
pub fn engine_decoder(self) -> EngineDecoder<Self> {
    EngineDecoder::default().wrap(self, offset)
}
// engine.capacity() → get_u16_at(self.offset) — reads 2 bytes directly
```

Benchmark (todo 105 audit):

| Pattern | ErgoSBE | Aeron |
|---------|---------|-------|
| Single-field (`engine.capacity()`) | 6-byte copy + 2-byte stack read = 8 bytes | 2-byte buffer read |
| Multi-field (`engine.capacity()` + `.horsepower()`) | 6-byte copy + 4 bytes stack reads | 4 bytes buffer reads (2 calls through parent) |

**Aeron wins for single-field access by 4×.** For multi-field, ErgoSBE's eager copy
amortises — the 6-byte copy is shared across all field reads. But HFT users who only
read one field from a composite are paying a 4× penalty on every access.

## Design

**The default must be the FAST path.** Flip the current API:

```rust
/// Flyweight decoder — DEFAULT. Zero-copy from buffer, reads per-field.
/// Use this for single-field access (the common HFT case).
#[inline]
pub fn engine(&self) -> EngineDecoder<'a> {
    let offset = self.pos + 35;
    EngineDecoder { buf: self.buf, pos: offset }
}

/// Eager copy to a value struct. Copy all bytes once, then stack reads.
/// Use this for multi-field access to amortise the copy.
#[inline]
pub fn engine_as_struct(&self) -> Engine {
    let offset = self.pos + 35;
    Engine(self.buf[offset..][..6].try_into().unwrap())
}
```

The flyweight `EngineDecoder` is a lightweight struct wrapping `buf` + `pos`
that reads directly from the wire on each field access — zero-copy:

```rust
pub struct EngineDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> EngineDecoder<'a> {
    pub fn capacity(&self) -> u16 {
        u16::from_le_bytes(self.buf[self.pos..][..2].try_into().unwrap())
    }
    pub fn num_cylinders(&self) -> u8 {
        self.buf[self.pos + 2]
    }
}
```

This matches Aeron's approach — direct buffer reads per field.

### Naming
- `engine()` — flyweight decoder (FAST, zero-copy, DEFAULT)
- `engine_as_struct()` — eager value struct copy (for multi-field access)

This IS a breaking API change — `engine()` currently returns `Engine`.
But performance is the #1 requirement, and the default should be fast.

### Scope
- Message decoder composite fields
- Group entry decoder composite fields
- Encoder: no change (encoding always needs the full value)

## Acceptance criteria

- [ ] `{field}() -> {Type}Decoder` flyweight (DEFAULT, zero-copy, fast)
- [ ] `{field}_as_struct() -> {Type}` eager copy (for multi-field access)
- [ ] Flyweight decoder struct: `buf: &'a [u8], pos: usize`
- [ ] Each field on flyweight reads directly from buffer (zero-copy)
- [ ] This IS a breaking API change — documented in migration guide
- [ ] Golden file stability test passes
- [ ] Baseline tests pass
- [ ] Benchmark: `engine().capacity()` reads ≤ 2 bytes from buffer (not 6+2)
