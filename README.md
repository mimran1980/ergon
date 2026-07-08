# ErgoSBE

Opinionated, idiomatic Rust code generation for [Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/) (SBE).

ErgoSBE reads SBE XML schemas and produces safe, fast, version-aware Rust codecs.
The project goal is byte-for-byte compatibility with the official SBE reference
implementation, with an API shaped for Rust rather than translated from Java.

## Features

These are the implemented/generated capabilities. Release-quality claims such as
"full Aeron parity", "HFT-ready", and "safe by parse" are gated by
[`sbe/todos/123-release-quality-gates.md`](sbe/todos/123-release-quality-gates.md).

- **XML schema parsing** — parse SBE schemas with XInclude support, miette diagnostics
- **Encoder/Decoder generation** — zero-allocation `Copy` decoders, fluent encoder API
- **Infallible field accessors** — scalar, enum, set, and composite accessors are plain `fn(&self) -> T`, no unwrapping
- **Flat enum generation** — enums are true Rust `enum`s with a `NullVal` variant for unknown wire values (no separate `Kind` type)
- **Buffer verification** — `Decoder::verify(&[u8])` validates an entire message buffer before decoding, reporting group/vardata bounds
- **Version-aware decoding** — all accessors respect the wire message version
- **Repeating groups** — `ExactSizeIterator`-based group access with entry decoders
- **Variable-length data** — var-data with length-prefixed byte slices and optional UTF-8 accessors
- **AnyMessage dispatch** — `AnyMessage` enum with `Unknown` forwarding for external frames
- **FrameCursor** — iterate externally-framed SBE feed buffers (length-prefix or fixed-size)
- **Multi-schema** — `generate_multi` for projects with shared type definitions across schemas
- **Type-state tail encoding** — encoder enforces tail element ordering at compile time
- **Optional/null handling** — `Option<T>` return types for optional and version-gated fields
- **Unchecked accessors** — `unsafe fn foo_unchecked()` for HFT hot loops (no bounds check)
- **Compile-time constants** — `FIELD_NULL`, `FIELD_MIN`, `FIELD_MAX` on every decoded field

## Current Status

- Local `ergosbe` tests, formatting, clippy, and generated-code stability checks
  are tracked in [`sbe/todos/TESTING_PLAN.md`](sbe/todos/TESTING_PLAN.md).
- Head-to-head Aeron performance parity is not claimed until todo 105 and the
  release gates have benchmark evidence for every hot-path scenario.
- Advanced Rust proof APIs such as verified frames, typed frame policies,
  scoped callbacks, and required-field proofs are roadmap items until their
  runtime, compile-fail, and benchmark gates pass.
- The exchange-orderbook sample currently compiles, but generated warning volume
  and live exchange/ClickHouse E2E verification remain tracked work.

## Stable Rust Advantage Roadmap

ErgoSBE should beat standard Aeron Rust bindings by leaning into stable Rust
features that reduce the public interface while keeping the generated
implementation zero-cost:

- **Sealed proof tokens and marker types** for checked/verified/unchecked
  decoder modes, schema identity, and frame policy.
- **Associated codec types** on `SbeMessage` for monomorphised generic helpers.
- **HRTB-scoped callbacks** so borrowed decoder views cannot escape a feed frame.
- **Return-position `impl Trait`** to hide generated iterator/helper type names.
- **Const/static templates** for header and group dimension setup.
- **Optional `#[repr(transparent)]` semantic newtypes** for domain safety without
  changing the wire representation.

The stable-Rust roadmap is tracked in
[`sbe/todos/144-stable-rust-advantage-roadmap.md`](sbe/todos/144-stable-rust-advantage-roadmap.md).

## Quick start

### 1. Add dependency

```toml
[build-dependencies]
ergosbe = "0.1"
```

### 2. Create `build.rs`

```rust
use ergosbe::{parse_file, Generator, GenerationConfig, Schema};

fn main() {
    // Parse an SBE XML schema file (with XInclude resolution)
    let ir = parse_file("schemas/my_schema.xml").unwrap();
    let schema = Schema::from_ir(ir);

    // Configure the generator
    let config = GenerationConfig::new("my_messages");
    let generator = Generator::new(config);

    // Generate Rust source
    let output = generator.generate(&schema);

    // Write to the output directory
    let out_dir = std::env::var("OUT_DIR").unwrap();
    for module in output.modules() {
        std::fs::write(
            format!("{}/{}", out_dir, module.path),
            &module.source,
        ).unwrap();
    }
}
```

### 3. Use generated code

Scalar, enum, set, and composite field accessors are **infallible** -- no `?`, no `unwrap`:

```rust
// Include the generated module
include!(concat!(env!("OUT_DIR"), "/my_messages.rs"));

fn decode_message(buf: &[u8]) -> Result<(), sbe_rt::DecodeError> {
    let car = CarDecoder::wrap_and_apply_header(buf, 0)?;
    let serial = car.serial_number();           // u64 -- infallible
    let year = car.model_year();                // u16 -- infallible
    let model = car.code();                     // Model (flat enum) -- infallible
    let extras = car.extras();                  // OptionalExtras (set) -- infallible
    let engine = car.engine();                  // Engine (composite) -- infallible
    println!("Car #{} ({})", serial, year);

    // Groups and var-data still return Result:
    for entry in car.fuel_figures()? {
        let speed = entry.speed();              // u16 -- infallible
        println!("Speed: {}", speed);
    }
    Ok(())
}
```

## Architecture

| Layer | Module | Description |
|-------|--------|-------------|
| Schema Input | `xml`, `schema` | Parse SBE XML, resolve includes, validate |
| Intermediate Repr | `ir`, `resolve` | Token stream, offset/block-length pass |
| Generation Options | `config` | Module name, wire-compatibility policy |
| Code Generation | `codegen` | Rust source production |

## Related crates

- **[`ergo-clickhouse-persist`](persist/README.md)** — debugging persistence:
  auto-persist annotated Rust structs to ClickHouse with automatic schema
  management. Consumer-side only, never on the hot path.

## Design philosophy

1. **Wire compatible** — generated bytes match official SBE byte-for-byte.
2. **Idiomatic Rust** — not Java-in-Rust. Decoders are `Copy` flyweights; encoders use
   type-state for tail fields.
3. **Zero allocation by default** — decoders borrow the input buffer; no heap allocation
   on the hot path.
4. **Version-aware** — every accessor gates on the wire `actingVersion` and
   `actingBlockLength`.
5. **No `unsafe` by default** — `unsafe` is opt-in via `_unchecked()` methods or the
   `bound-check-disabled` feature.

See [`sbe/design/DECISIONS.md`](sbe/design/DECISIONS.md) for the complete design rationale.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
