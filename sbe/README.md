# ergo-sbe

`ergo-sbe` parses Simple Binary Encoding schemas and generates Rust codecs. The
project targets low-latency messaging while preserving official SBE wire
semantics.

> Experimental software. The crate has no production-safety or long-term API
> compatibility guarantee.

## Rust version and edition

| | |
|---|---|
| **Edition** | **2024** |
| **MSRV** | **1.88** (keeps let-chains and other 1.88+ language features; edition 2024 floor is 1.85) |

If **edition 2021** (and an older MSRV) would unblock you, open an issue or say
so — I am happy to maintain a 2021-compatible packaging path if there is real
demand. Prefer saying what toolchain your shop is stuck on (e.g. 1.75 / 1.80).

## What it generates

Generated modules can include:

- borrowed, version-aware decoder flyweights;
- fixed-field encoder phases and consuming tail stages;
- repeating groups, nested groups, composites, enums, sets, constants, and
  variable-length data;
- exact encoded-length helpers for fixed, directly sized, and structurally
  dynamic messages;
- checked decoder and encoder entry points plus explicit trusted-buffer paths;
- `AnyMessage` and `FrameCursor` dispatch for multi-message schemas;
- strict text accessors when the schema declares a character encoding;
- optional conversion methods and owned domain objects.

The generated API follows the schema's wire order. Concrete type and method
names therefore depend on the input schema.

## Generate a module

Add the generator as a build dependency:

```toml
[build-dependencies]
ergo-sbe = "0.1"
```

A minimal `build.rs` parses one schema and writes the generated module to
Cargo's output directory:

```rust
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prefer parse_file("schemas/messages.xml") when the schema lives on disk.
    let schema_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <messageSchema package="example" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Heartbeat" id="1">
        <field name="seq" id="1" type="uint32" offset="0"/>
      </message>
    </messageSchema>"#;

    let ir = ergo_sbe::parse(schema_xml)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let generated = ergo_sbe::Generator::new(
        ergo_sbe::GenerationConfig::new("messages"),
    )
    .generate(&schema)?;

    let module = generated.modules().next().ok_or_else(|| {
        std::io::Error::other("schema generated no Rust module")
    })?;
    // In a real build.rs, OUT_DIR is set by Cargo:
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|_| ".".into()));
    std::fs::write(out_dir.join(&module.path), &module.source)?;
    assert!(module.source.contains("HeartbeatEncoder"));
    Ok(())
}
```

Include the result from application code:

```rust,ignore
// Shape only — needs OUT_DIR from a real build.rs
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
```

### Samples (schema + runnable demos)

Samples live in the **ergon monorepo** (not in this crates.io package). Links are
absolute so they work on [docs.rs](https://docs.rs/ergo-sbe) and crates.io.

**Samples** (monorepo only — not on crates.io). Each picks a conversion style on purpose:

| Sample | Conversion style | What you learn |
|--------|------------------|----------------|
| [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) | **Both** (domain types for bool/timestamp; `with_conversion` for Decimal) | Full API map + `demo_conversion_only` |
| [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) | **`with_domain_type` only** | Nested books; concrete `price() -> Decimal` |
| [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) | **`with_conversion` only** | App-side `TryFromSbe` / `price_as` / `price_from` |

```sh
git clone https://github.com/mimran1980/ergon.git && cd ergon
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo run  --manifest-path samples/l3-book/Cargo.toml
```

## Buffer sizing

Size every encode buffer from generated metadata:

| Message shape | Generated approach |
|---|---|
| Fixed block only | `Encoder::ENCODED_LENGTH` |
| Directly sized tails | `compute_encoded_length_with_message_header(...)` |
| Groups or nested dynamic tails | `{Message}EncodedLength` staged builder |

For ragged groups, declare each entry before its tail contribution
(names follow your schema — shape only):

```rust,ignore
let complete = MessageEncodedLength::new().entries_ragged(2, |entries| {
    entries.add()?.payload(first_payload.len())?;
    entries.add()?.payload(second_payload.len())?;
    Ok(())
})?;
let len = complete.encoded_length_with_header();
let mut buffer = vec![0u8; len];
```

Generated names vary with the schema, but the rule is stable: the number of
`add()` calls must match a declared ragged count. Unknown-size builders derive
the final group count from completed entries. The integration test
`docs_validation_test` exercises this pattern against a real generated module.

## Decode and encode safely

Use generated `try_*` constructors for buffers received from outside the
process or not already validated. They check the message header and fixed block
before exposing scalar accessors. Use a generated `verify` method when the
entire dynamic tail must be validated before traversal.

Fast `wrap` methods are trusted-buffer APIs. They avoid repeated checks and
must only receive storage whose header, fixed block, and tail bounds have
already been established.

Encoders use a fixed-field phase followed by consuming tail stages. A typical
generated flow is (shape only — names follow your schema):

```rust,ignore
let complete = MessageEncoder::try_wrap_and_apply_header(&mut buffer, 0)?
    .fixed(&MessageFixedFields { sequence: 42 })
    .entries(entry_count, |entries| {
        for value in values {
            entries.add(|entry| {
                entry.value(*value);
                Ok(())
            })?;
        }
        Ok(())
    })?
    .description(description)?;
```

Prefer `?` inside group closures so count, bounds, and variable-data errors
remain observable. Validated live in `docs_validation_test`.

## Configuration

### `with_conversion` vs `with_domain_type` (pick one per field)

These are **different APIs**. `with_domain_type` already enables conversion for
that selector — do **not** also call `with_conversion` for the same selector.

| | `with_conversion(sel)` | `with_domain_type(sel, "path::Type")` |
|--|------------------------|--------------------------------------|
| **When** | You want a pluggable adapter, or no forced crate dep | One canonical Rust type for the field |
| **Decode** | `dec.price_as::<T>()?` | `dec.price()` → `path::Type` |
| **Encode** | `enc.price_from(&value)?` | `enc.price(value)` |
| **Wire still there** | Yes (`price_value` / `price_wire`) | Yes (`price_value` / `price_wire` when converted) |
| **Who implements traits** | **You** (`TryFromSbe` / `TryToSbe`) | Generator (well-known paths: bool, rust_decimal, chrono) |
| **Sample** | [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) |

#### Option A — generic conversion (`with_conversion`)

```rust
use ergo_sbe::{ConversionSelector, GenerationConfig};

// build.rs
let config = GenerationConfig::new("msgs")
    .with_conversion(ConversionSelector::named_type("Decimal"));
let _ = config;
// Generator::new(config).generate(&schema)?;
// app: enc.price_from(&app_price)?; let app: MyT = dec.price_as()?;
```

#### Option B — concrete domain type (`with_domain_type`)

```rust
use ergo_sbe::{ConversionSelector, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );
let _ = config;
// app: enc.price(rust_decimal::Decimal::new(12345, 2));
// let p: rust_decimal::Decimal = dec.price();
```

Side-by-side teaching sample (uses **both**, on different selectors):
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
— `demo_conversion_only` is pure Option A for Decimal.

### Other options

- `enable_domain_objects` — owned message structs (`CarDomain`, …)
- `with_external_sbe_rt` / `with_shared_module` — multi-schema packaging
- `enable_error_from_impls` — `?` into your error type
- `with_unchecked_companions` — benchmark-only fast paths
- `with_keyword_append_token` — rename Rust-keyword field names (default `"_"`)

Text fields remain bytes unless their schema declares a supported character
encoding. Text accessors validate UTF-8 or ASCII and return errors rather than
lossy replacements.

Generated `Display` and `Debug` output is diagnostic and is not a stable
serialization format.

## Verify the crate

```sh
# unit + integration + doctests
cargo test -p ergo-sbe --all-features -- --test-threads=1
# rustdoc examples only
cargo test -p ergo-sbe --doc --all-features
# docs must build without warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
```

`just test` runs workspace tests (including doctests) plus an explicit
`ergo-sbe` doctest/rustdoc gate and `docs_validation_test` (README fences +
generated-API smoke).

Performance methodology and commands live in the monorepo
[BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
(not shipped in the crates.io package).

## Package scope

The crates.io package contains the generator source, manifest, and this README.
Repository tests, fixtures, samples, benchmarks, and internal planning material
are **not** part of the package — open them on GitHub via the links above.

## License

Apache-2.0. Repository: [mimran1980/ergon](https://github.com/mimran1980/ergon).
