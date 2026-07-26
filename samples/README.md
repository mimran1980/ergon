# Samples

These standalone crates exercise repository APIs in larger flows. They are
excluded from the workspace, set `publish = false`, and are not reference
implementations.

## `ergo-sbe`: build dependency vs application dependency

Generated codecs ship their own embedded `sbe_rt` module. Linking the **app**
does not require `ergo-sbe` unless you use its macros or call the generator
library at runtime.

| Pattern | `build-dependencies` | `dependencies` | Typical use |
|---------|----------------------|----------------|-------------|
| **Build only** | `ergo-sbe` | — | Product path: `generate_to_out_dir` in `build.rs`, then plain `include!(concat!(env!("OUT_DIR"), "/….rs"))` |
| **Build + runtime** | `ergo-sbe` | `ergo-sbe` | Same codegen, plus `ergo_sbe::sbe_mod!` / `include_sbe!` convenience macros |
| **Runtime only** | — | `ergo-sbe` | Call `parse` / `Generator` as a library (no `build.rs`) |

| Sample | Pattern | Purpose | External requirements |
|---|---|---|---|
| [`l3-book/`](l3-book/) | **Build only** | Nested/ragged L3 books; **`with_domain_type` only** | None for local tests |
| [`cluster-rfq/`](cluster-rfq/) | **Build only** | RFQ / auction protocol codecs + cluster examples | Java harness for live examples |
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Build + runtime** | **ErgoSBE feature map** — EncodedLength, stages, DTO, AnyMessage, **both** conversion styles; uses `sbe_mod!` | None |
| [`exchange-example/`](exchange-example/) | **Build + runtime** | Multi-schema + **`with_conversion` only** + Aeron IPC; uses `sbe_mod!` | Network only for live exchange paths |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | **Build + runtime** | Claim-based Cluster publishing + HA-shaped book; uses `sbe_mod!` | Java harness only for leader-kill coverage |
| [`sbe-codegen-examples/`](sbe-codegen-examples/) | **Runtime only** | Generator API as a library (no `build.rs`) | None |
| [`cluster-tutorial/`](cluster-tutorial/) | **Neither** (uses `ergo-aeron-cluster`) | Connect, offer, poll, keep-alive, close walkthrough | Java 17+ and built Aeron artifacts |

**Build-only include shape** (no runtime `ergo-sbe`):

```rust
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
```

**Build + runtime** (macro):

```rust
ergo_sbe::sbe_mod!(messages);
```

## Check each sample

Run standalone packages through their manifest paths. These are diagnostic
entry points: samples intentionally move with experimental APIs, so a failing
command identifies migration drift rather than a supported product regression.

```sh
cargo check --manifest-path samples/sbe-feature-tour/Cargo.toml --all-targets
cargo check --manifest-path samples/exchange-example/Cargo.toml --all-targets
cargo check --manifest-path samples/l3-book/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-ha-orderbook/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-rfq/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-tutorial/Cargo.toml --all-targets
```

Useful service-free tests:

```sh
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline
```

Java-backed samples require:

```sh
just build-aeron-jars
cargo run --manifest-path samples/cluster-tutorial/Cargo.toml
```

## Conversion: which sample uses what

| Sample | Config | Decode / encode surface |
|--------|--------|-------------------------|
| [`l3-book/`](l3-book/) | **`with_domain_type` only** | `dec.price()` → `Decimal`; `enc.price(d)` |
| [`exchange-example/`](exchange-example/) | **`with_conversion` only** | `dec.price_as::<T>()?`; `enc.price_from(&t)?` (+ app `TryFromSbe`) |
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Both** (different selectors) | bool/timestamp concrete; Decimal generic (`demo_conversion_only`) |

Rule: **one style per selector**. `with_domain_type` already enables conversion;
do not stack `with_conversion` on the same selector.

```rust
// A — pluggable
.with_conversion(ConversionSelector::named_type("Decimal"))
// B — concrete (implies conversion)
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
```

## ErgoSBE feature tour

[`sbe-feature-tour/`](sbe-feature-tour/) maps product README claims → runnable demos.

```sh
cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml
```

## L3 sample

Deep nested/ragged books; **`with_domain_type` only** (concrete decimals /
chrono / bool). **Build-dep only** — no application `ergo-sbe` link. See
[`l3-book/README.md`](l3-book/README.md).

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs.
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
