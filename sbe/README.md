# ergo-sbe

> **Experimental prototype. Do not use in production.** The generated interface
> is intentionally opinionated and will change while its safety, ergonomics, wire
> compatibility, and performance are evaluated.

ErgoSBE parses Simple Binary Encoding XML schemas and generates Rust encoders and
decoders. It is the primary project in this repository and the most thoroughly
tested, but it is still research software rather than a supported replacement for
the official SBE toolchain.

The goal is to prototype generated Rust interfaces that are easier and safer to
use without giving up low-latency performance. Useful ideas may eventually be
adapted for the official Java SBE Rust generator.

## Current capabilities

- SBE XML parsing, validation, includes, and schema normalization.
- Rust generation for messages, primitives, enums, sets, composites, repeating
  groups, and variable data.
- Acting-version-aware decoding and configurable byte order.
- Borrowed flyweight decoding and mutable-buffer encoding.
- Concrete consuming stages for ordered group and variable-data tails.
- Message dispatch, framing helpers, diagnostics, schema fixtures, allocation
  tests, and Aeron reference comparisons.
- Optional `serde` derives and an experimental domain-object layer.

These capabilities do not mean the planned interface is complete. The current
review found partial converter emission, bypassable fixed-field completion,
incomplete recursive domain mapping, incomplete composite symmetry, and
inconsistent handling of schema-declared text. See the single
[`implementation plan`](../docs/IMPLEMENTATION_PLAN.md) for the exact design and
open acceptance criteria.

## Build and test

From the repository root:

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cargo doc -p ergo-sbe --all-features --no-deps
cargo bench -p ergosbe-benchmarks --no-run
```

Generator and hot-path changes must also run the equal-work performance suite:

```sh
just bench
```

Passing tests establish the covered baseline only. They do not close an item in
the implementation plan unless the item's behavioural, compile-fail, allocation,
and performance criteria are present and pass.

## Minimal generation example

Add ErgoSBE as a build dependency while working in this repository:

```toml
[build-dependencies]
ergo-sbe = { path = "../sbe" }
```

A build script can parse a schema and write the generated module to `OUT_DIR`:

```rust,no_run
use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse_file("schemas/messages.xml")?;
    let schema = Schema::from_ir(ir);
    let generated = Generator::new(GenerationConfig::new("messages"))
        .generate(&schema)?;
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    for module in generated.modules() {
        std::fs::write(out.join(&module.path), &module.source)?;
    }

    Ok(())
}
```

Include the generated file from the consuming crate:

```rust,ignore
include!(concat!(env!("OUT_DIR"), "/messages.rs"));
```

Generated surface details are deliberately not duplicated here while the
pre-release redesign is unfinished. Generated code and tests are the truth for
current behaviour; the implementation plan is the truth for intended behaviour.

## Design direction

- Official SBE wire compatibility is non-negotiable.
- Encoders target the latest schema version; decoders honour the acting version.
- Required fixed fields will be proven as a phase, while groups and variable data
  retain consuming wire-order stages.
- One generic, statically dispatched converter seam will serve boolean, decimal,
  timestamp, newtype, composite, and domain conversions.
- Variable data remains bytes at the low-level wire interface. When the schema
  declares UTF-8 or ASCII, the decoder will additionally provide a fallible,
  zero-copy `&str` view.
- Malformed data must remain an error. Lossy or default substitution is not part
  of the generated protocol interface.
- Owned domain objects are optional convenience. Flyweights remain the hot-path
  interface.

## Layout

| Path | Purpose |
|---|---|
| `src/xml.rs` | Parse and validate SBE XML |
| `src/ir.rs` and `src/resolve.rs` | Intermediate representation and layout resolution |
| `src/config.rs` | Generator configuration |
| `src/codegen.rs` | Rust source generation |
| `tests/` | Behavioural, parity, regression, allocation, and stability coverage |

## Publication

`ergo-sbe` is intended for an eventual crates.io `0.x` prototype release. It is
not ready to publish until every release item in the implementation plan passes,
the package contents are minimal, and the public documentation describes only
verified behaviour.
