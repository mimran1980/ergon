# Getting started with ErgoSBE

This guide walks through adding ErgoSBE to a Rust project, writing or reusing an
SBE schema, generating code, and using the generated decoder and encoder.

## Prerequisites

- Rust 1.88 or later (edition 2024)
- An SBE XML schema (see [schema-authoring.md](schema-authoring.md) or use one
  from the [official SBE examples](../../simple-binary-encoding/sbe-samples/src/main/resources/))

## Adding ErgoSBE to your project

ErgoSBE runs as a build dependency. It generates Rust source at compile time.

```toml
[package]
name = "my-trading-app"
version = "0.1.0"
edition = "2024"

[build-dependencies]
ergosbe = "0.1"
```

## Writing build.rs

Create a `build.rs` file in your project root. This is the entry point for
code generation.

### Single schema

```rust
// build.rs
use ergosbe::{parse_file, Generator, GenerationConfig, Schema};

fn main() {
    // Parse the SBE XML schema
    let ir = parse_file("schemas/market_data.xml")
        .expect("failed to parse SBE schema");
    let schema = Schema::from_ir(ir);

    // Configure the generator
    let config = GenerationConfig::new("market_data");
    let generator = Generator::new(config);

    // Generate Rust source
    let output = generator.generate(&schema);

    // Write to OUT_DIR
    let out_dir = std::env::var("OUT_DIR").unwrap();
    for module in output.modules() {
        std::fs::write(
            format!("{}/{}", out_dir, module.path),
            &module.source,
        )
        .expect("failed to write generated module");
    }
}
```

### Multiple schemas with shared types

When multiple SBE schemas share types (common enums, sets, composites), use
`generate_multi` to avoid duplicating type definitions:

```rust
use ergosbe::{parse_file, Generator, GenerationConfig, Schema};

fn main() {
    let ir_a = parse_file("schemas/common_types.xml").unwrap();
    let schema_a = Schema::from_ir(ir_a);

    let ir_b = parse_file("schemas/market_data.xml").unwrap();
    let schema_b = Schema::from_ir(ir_b);

    let mut config = GenerationConfig::new("common_types");
    config.shared_module = Some("common_types".into());

    let generator = Generator::new(config);
    let output = generator.generate_multi(&[
        (&schema_a, "common_types"),
        (&schema_b, "market_data"),
    ]);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    for module in output.modules() {
        std::fs::write(
            format!("{}/{}", out_dir, module.path),
            &module.source,
        )
        .expect("failed to write generated module");
    }
}
```

## Using generated code

In your Rust source, include the generated module:

```rust
// src/main.rs
include!(concat!(env!("OUT_DIR"), "/market_data.rs"));

fn main() -> Result<(), sbe_rt::DecodeError> {
    let buf: &[u8] = &[
        // ... raw SBE message bytes ...
    ];

    // Decode a message -- wrap_and_apply_header reads the SBE header
    let quote = QuoteDecoder::wrap_and_apply_header(buf, 0)?;

    // Read fixed fields -- scalar, enum, set, and composite accessors
    // are ALL infallible (no Result wrapper):
    let price = quote.price();        // u64 -- no ?, no unwrap
    let qty = quote.quantity();       // u32 -- infallible
    let side = quote.side();          // Side (flat enum) -- infallible

    println!("Price: {}, Qty: {}", price, qty);

    // Read optional/version-gated fields -- return Option<T> directly
    if let Some(pegged_price) = quote.pegged_price() {
        println!("Pegged: {}", pegged_price);
    }

    // Groups and var-data still return Result:
    for entry in quote.orders()? {
        let id = entry.order_id();    // u64 -- infallible
        println!("Order: {}", id);
    }

    Ok(())
}
```

### Buffer verification

Before decoding, you can verify an entire message buffer. This validates the
header, block length, group dimensions, and var-data bounds in a single pass:

```rust
// Verify all group/vardata bounds before decoding
QuoteDecoder::verify(&buf)?;
let quote = QuoteDecoder::wrap_and_apply_header(buf, 0)?;
```

### Encoding messages

```rust
fn encode_example() -> Result<(), sbe_rt::EncodeError> {
    // Allocate a buffer (fixed-size messages have ENCODED_LENGTH)
    let mut buf = [0u8; QuoteEncoder::ENCODED_LENGTH];

    // Wrap and write the SBE header
    let mut encoder = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)?;

    // Set scalar fields -- returns &mut Self for chaining
    encoder
        .price(1234500)
        .quantity(100)
        .side(Side::from_raw(1));

    // Get the encoded bytes
    let encoded = encoder.as_ref();
    assert!(encoded.len() > 0);
    Ok(())
}
```

## Understanding the pipeline

1. **Parse**: XML schema text is parsed into a flat token stream (`Ir`)
2. **Resolve**: Offsets, block lengths, and default values are computed
3. **Generate**: The `Generator` walks the resolved IR and emits Rust source
4. **Format**: Generated source is formatted via `prettyplease`

## Next steps

- [Schema authoring](schema-authoring.md) — writing SBE XML schemas for ErgoSBE
- [Generated API](generated-api.md) — detailed reference for generated types
- [Advanced topics](advanced.md) — multi-schema, unsafe, HFT patterns
