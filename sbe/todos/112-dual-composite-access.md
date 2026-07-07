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

Generate BOTH patterns — let the user choose based on their access pattern:

```rust
/// Eager copy — good for multi-field access (copy once, read many).
#[inline]
pub fn engine(&self) -> Engine {
    let offset = self.pos + 35;
    Engine(self.buf[offset..][..6].try_into().unwrap())
}

/// Lazy flyweight — good for single-field access (zero-copy from buffer).
/// Reads directly from the wire on each field access.
#[inline]
pub fn engine_lazy(&self) -> EngineDecoder {
    let offset = self.pos + 35;
    EngineDecoder { buf: self.buf, pos: offset }
}
```

The `EngineDecoder` is a lightweight struct that wraps `buf` + `pos` and
provides individual field accessors that read directly from the buffer:

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
    // ...
}
```

This is what Aeron does — zero-copy, direct buffer reads per field.

### Scope

- Message decoder composite fields
- Group entry decoder composite fields
- Encoder side: no change needed (encoding always needs the full value)

### Naming

`_lazy` suffix — one word, obvious. `engine()` = eager (current), `engine_lazy()` = flyweight.

## Acceptance criteria

- [ ] `{field}_lazy() -> {Type}Decoder` generated for composite fields on message decoders
- [ ] `{field}_lazy() -> {Type}Decoder` generated for composite fields on group entry decoders
- [ ] `{field}() -> {Type}` preserved (eager copy, unchanged — current behavior)
- [ ] Lazy decoder struct generated with `buf: &'a [u8], pos: usize`
- [ ] Each field on the lazy decoder reads directly from buffer (zero-copy)
- [ ] Golden file stability test passes
- [ ] Baseline tests pass
- [ ] Benchmark: `engine_lazy().capacity()` reads ≤ 2 bytes from buffer (not 6+2)
