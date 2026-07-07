# Dual composite access: eager copy + lazy flyweight

**Blocked by:** none (codegen only)
**Ref:** Aeron perf audit (todo 105, gap #7)

## Problem

ErgoSBE copies composite fields eagerly:

```rust
pub fn engine(&self) -> Engine {
    let offset = self.pos + 35;
    Engine(self.buf[offset..][..6].try_into().unwrap())
}
```

Aeron returns a flyweight decoder that reads from the buffer on each
field access:

```rust
pub fn engine_decoder(self) -> EngineDecoder<Self> {
    EngineDecoder::default().wrap(self, offset)
}
```

| Pattern | ErgoSBE | Aeron |
|---------|---------|-------|
| Single-field access (`engine.capacity()`) | 6-byte copy + 2-byte read | 2-byte read from buffer |
| Multi-field access (`engine.capacity()` + `engine.horsepower()`) | 6-byte copy + 4 bytes of reads | 4 bytes of reads (2 calls through parent) |

ErgoSBE wins for multi-field access (amortised copy). Aeron wins for
single-field access (no copy). The choice depends on usage pattern —
but ErgoSBE currently offers no choice.

## Design

Generate BOTH patterns on composite fields in decoders:

```rust
/// Eager copy — good for multi-field access.
/// Reads all bytes once, fields are stack-local reads.
#[inline]
pub fn engine(&self) -> Engine {
    let offset = self.pos + 35;
    Engine(self.buf[offset..][..6].try_into().unwrap())
}

/// Lazy flyweight — good for single-field access.
/// Reads from the buffer on each field access (zero copy).
#[inline]
pub fn engine_lazy(&self) -> EngineDecoder {
    let offset = self.pos + 35;
    EngineDecoder { buf: self.buf, pos: offset }
}
```

The `EngineDecoder` struct already exists (it's the composite's own
decoder). The lazy accessor just returns it directly instead of
copying the bytes into a value struct.

### Naming

Ponytail: `_lazy` suffix. Clear, obvious, one word. No `_decoder` /
`_flyweight` / `_ref` bikeshed.

### Encoder symmetry?

Encoders already work with the composite value type (`Engine`) —
there's no flyweight encoder because encoding needs the full value.
No change needed on the encoder side.

## Acceptance criteria

- [x] `{field}_lazy() -> {Type}Decoder` generated for composite fields
  on message decoders and group entry decoders
- [x] `{field}() -> {Type}` preserved (eager copy, unchanged)
- [x] Lazy variant compiles to a single `get_u16_at` (or equivalent)
  for single-field reads — zero copy from buffer
- [x] Golden file stability test passes
- [x] No regression in baseline test suite
