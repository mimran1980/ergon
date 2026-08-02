# ergo-sbe

[![Crates.io](https://img.shields.io/crates/v/ergo-sbe)](https://crates.io/crates/ergo-sbe)
[![CI](https://github.com/mimran1980/ergon/actions/workflows/ci.yml/badge.svg)](https://github.com/mimran1980/ergon/actions/workflows/ci.yml)
[![API Docs](https://docs.rs/ergo-sbe/badge.svg)](https://docs.rs/ergo-sbe/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/)

`ergo-sbe` generates **binary-compatible** Rust SBE codecs with compile-time
wire-order enforcement, closure-based groups, exact buffer sizing, and zero
heap allocation on hot paths.

> **AI assistance.** Large parts of this project were written **with heavy AI
> assistance**. Humans directed the work, approved designs, and ran verification.
> Details: [AI-ASSISTANCE.md](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md).

## Full documentation

**[ergo-sbe book](https://mimran1980.github.io/ergon/)** is the comprehensive
guide for ergo-sbe (also linked from this crate on
[docs.rs](https://docs.rs/ergo-sbe/)):

- [Getting Started](https://mimran1980.github.io/ergon/sbe/getting-started.html) — depend, generate, encode, decode
- [Feature Tour](https://mimran1980.github.io/ergon/sbe/feature-tour.html) — exact sizing, bulk arrays, decode stages, DTOs, trust boundaries
- [Core Concepts](https://mimran1980.github.io/ergon/sbe/core-concepts.html) — wire order, buffer sizing, composites, flyweight vs struct
- [Configuration](https://mimran1980.github.io/ergon/sbe/configuration.html) — `with_conversion` vs `with_domain_type`, hooks
- [Recipes](https://mimran1980.github.io/ergon/sbe/recipes.html) — Display/Debug, schema→rustdoc, domain DTOs, timestamps
- [Benchmarks](https://mimran1980.github.io/ergon/sbe/benchmarks.html) — parity methodology and gates

**Compatibility profile (normative):**
[`docs/SBE_COMPATIBILITY.md`](https://github.com/mimran1980/ergon/blob/main/docs/SBE_COMPATIBILITY.md)
— do not claim unqualified “SBE binary compatibility.”

**0.1 → 0.1.10 migration:**
[`docs/MIGRATION_0_1_TO_0_1_10.md`](https://github.com/mimran1980/ergon/blob/main/docs/MIGRATION_0_1_TO_0_1_10.md)
— fallible `wrap` / `decode`, `try_wrap*` removed; private `*_unchecked` cores until HFT-008 keep.

## Quick Example

```rust
use ergo_sbe::{parse, Schema, Generator, GenerationConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"<?xml version="1.0"?>
    <messageSchema package="demo" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Ping" id="1" blockLength="4">
        <field name="seq" id="1" type="uint32" offset="0"/>
      </message>
    </messageSchema>"#;

    let ir = parse(xml)?;
    let schema = Schema::from_ir(ir);
    let modules = Generator::new(GenerationConfig::new("demo_msgs"))
        .generate(&schema)?;
    // In a real project you'd use a build script.
    // Full guide: https://mimran1980.github.io/ergon/sbe/getting-started.html
    // Checked encode (0.1.10): MessageEncoder::wrap_and_apply_header(buf, 0)?
    // Public zero-check twins ship only after HFT-008 keep=true evidence.
    let _ = modules;
    Ok(())
}
```

## API Reference

[docs.rs/ergo-sbe](https://docs.rs/ergo-sbe/) — generated Rustdoc for the published crate.

## License

Apache-2.0.
