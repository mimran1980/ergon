# ergo-sbe

`ergo-sbe` parses Simple Binary Encoding schemas and generates Rust codecs. The
project targets low-latency messaging while preserving official SBE wire
semantics.

> Experimental software. The crate has no production-safety or long-term API
> compatibility guarantee.

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
    let schema_path = "schemas/messages.xml";
    let ir = ergo_sbe::parse_file(schema_path)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let generated = ergo_sbe::Generator::new(
        ergo_sbe::GenerationConfig::new("messages"),
    )
    .generate(&schema)?;

    let module = generated.modules().next().ok_or_else(|| {
        std::io::Error::other("schema generated no Rust module")
    })?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::write(out_dir.join(&module.path), &module.source)?;

    println!("cargo::rerun-if-changed={schema_path}");
    Ok(())
}
```

Include the result from application code:

```rust
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
```

### Samples (schema + runnable demos)

Samples live in the **ergon monorepo** (not in this crates.io package). Links are
absolute so they work on [docs.rs](https://docs.rs/ergo-sbe) and crates.io.

**Primary tour** — fixed `ENCODED_LENGTH`, staged `*EncodedLength`, encode chain,
consuming decoder stages, owned DTOs, `AnyMessage`, `try_*` vs trusted wrap,
Display/Debug, and **both** `with_domain_type` + `with_conversion` (see
`demo_conversion_only`):

| Resource | GitHub |
|----------|--------|
| Sample root | [samples/sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) |
| Schema | [schemas/feature-tour.xml](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/schemas/feature-tour.xml) |
| Named demos | [src/lib.rs](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| `build.rs` (domain objects + both conversion styles) | [build.rs](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/build.rs) |
| Sample README (feature → function map) | [README.md](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/README.md) |

**Deeper nested/ragged sample** — L3 books with **`with_domain_type` only**
(concrete `price() -> rust_decimal::Decimal`; domain type already implies
conversion, so bare `with_conversion` is not used):

| Resource | GitHub |
|----------|--------|
| Sample root | [samples/l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) |
| Sample README (`with_domain_type` rationale) | [README.md](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/README.md) |
| Schema | [schemas/l3-book.xml](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/schemas/l3-book.xml) |
| Encode / EncodedLength helpers | [src/lib.rs](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/lib.rs) |
| Runnable demos | [src/main.rs](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/main.rs) |
| `build.rs` | [build.rs](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/build.rs) |

Clone and run from the monorepo:

```sh
git clone https://github.com/mimran1980/ergon.git
cd ergon
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo run  --manifest-path samples/l3-book/Cargo.toml
```

## Buffer sizing

Size every encode buffer from generated metadata:

| Message shape | Generated approach |
|---|---|
| Fixed block only | `Encoder::ENCODED_LENGTH` |
| Directly sized tails | `compute_encoded_length_with_message_header(...)` |
| Groups or nested dynamic tails | `{Message}EncodedLength` staged builder |

For ragged groups, declare each entry before its tail contribution:

```rust
let complete = MessageEncodedLength::new().entries_ragged(2, |entries| {
    entries.add()?.payload(first_payload.len())?;
    entries.add()?.payload(second_payload.len())?;
    Ok(())
})?;
let len = complete.encoded_length_with_header();
let mut buffer = vec![0_u8; len];
```

Generated names vary with the schema, but the rule is stable: the number of
`add()` calls must match a declared ragged count. Unknown-size builders derive
the final group count from completed entries.

## Decode and encode safely

Use generated `try_*` constructors for buffers received from outside the
process or not already validated. They check the message header and fixed block
before exposing scalar accessors. Use a generated `verify` method when the
entire dynamic tail must be validated before traversal.

Fast `wrap` methods are trusted-buffer APIs. They avoid repeated checks and
must only receive storage whose header, fixed block, and tail bounds have
already been established.

Encoders use a fixed-field phase followed by consuming tail stages. A typical
generated flow is:

```rust
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

This is a shape example; use the names generated for your schema. Prefer `?`
inside group closures so count, bounds, and variable-data errors remain
observable.

## Configuration

`GenerationConfig` supports:

- `with_conversion` for **generic** `*_as` / `*_from` methods (caller supplies
  `TryFromSbe` / `TryToSbe`; no forced rust_decimal dependency) — see
  [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example)
  and feature-tour `demo_conversion_only`;
- `with_domain_type` for **concrete** app-type methods (implies conversion and
  well-known bool / rust_decimal / chrono impls when those paths are used) — see
  [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book);
  you do not also need `with_conversion` for the same selectors;
- `enable_domain_objects` for owned message structures;
- `with_external_sbe_rt` to share a generated runtime;
- `with_shared_module` for multi-schema shared types;
- `enable_error_from_impls` for application error conversion;
- `with_unchecked_companions` for explicit benchmark-only companions.

Text fields remain bytes unless their schema declares a supported character
encoding. Text accessors validate UTF-8 or ASCII and return errors rather than
lossy replacements.

Generated `Display` and `Debug` output is diagnostic and is not a stable
serialization format.

## Verify the crate

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
cargo package -p ergo-sbe --list --allow-dirty
```

Performance methodology and commands live in the monorepo
[BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
(not shipped in the crates.io package).

## Package scope

The crates.io package contains the generator source, manifest, and this README.
Repository tests, fixtures, samples, benchmarks, and internal planning material
are **not** part of the package — open them on GitHub via the links above.

## License

Apache-2.0. Repository: [mimran1980/ergon](https://github.com/mimran1980/ergon).
