# ErgoSBE Guide

## Quick start

```rust
use ergo_sbe::{parse, Generator, GenerationConfig, Schema};

let xml = std::fs::read_to_string("my_schema.xml")?;
let ir = parse(&xml)?;
let schema = Schema::from_ir(ir);

let modules = Generator::new(GenerationConfig::new("my_codec"))
    .generate(&schema)?;

for m in modules.modules() {
    std::fs::write(&m.path, &m.source)?;
}
```

## Generated API

- **Flyweight.** Decoders borrow `&[u8]`; encoders borrow `&mut [u8]`. No heap.
- **Consuming stages.** Groups and var-data are sequential on the wire. Each tail
  position is a distinct struct. `into_<group>(self)` consumes the current stage.
- **Version-aware.** Decoders read the wire `blockLength` and `version`. Accessors
  return `Option<T>` for versioned fields.
- **Domain objects.** Enable with `.enable_domain_objects()`. Each message gets an
  owned `MsgDomain` struct with `From<MsgDecoder>`.

## Builder reference

| Builder | Effect |
|---------|--------|
| `enable_domain_objects()` | Generate owned domain structs |
| `with_shared_module("name")` | Share types across multi-schema crates |
| `with_external_sbe_rt("path")` | Use shared `sbe_rt` module |
| `enable_decimal_converters("Decimal")` | Emit generic `SbeDecimal` trait |
| `enable_error_from_impls("path")` | Emit `From` impls for custom error types |
| `with_unchecked_companions()` | Benchmark-only unchecked methods |

## See also

- [`design/DECISIONS.md`](design/DECISIONS.md) — canonical design authority
- Crate [`README.md`](README.md) — setup and verification commands
- [`examples/`](examples/) — flyweight and domain-object examples
