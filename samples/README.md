# Samples

Standalone crates that exercise repository APIs. They are **excluded from the
workspace**, set `publish = false`, and are **not** production reference
implementations — they move with experimental APIs on purpose.

## Start here (product teaching path)

| Step | Sample | Why |
|------|--------|-----|
| **1** | [`sbe-feature-tour/`](sbe-feature-tour/) | **Golden path.** Full feature map: stages, EncodedLength, try/trusted, Display, DTO with `DomainVarData::LossyStrings`, both conversion styles |
| **2a** | [`l3-book/`](l3-book/) | Nested/ragged books; **`with_domain_type` only**; **build-dep only** (plain `include!`) |
| **2b** | [`exchange-example/`](exchange-example/) | Multi-schema; **`with_conversion` only**; IPC + app `TryFromSbe` |
| **3** | [`sbe-codegen-examples/`](sbe-codegen-examples/) | Generator **as a library** (no `build.rs`) |
| Later | [`cluster-*`](cluster-ha-orderbook/) | Aeron cluster integration (Java harness for some paths) |

```sh
# 1 — always start here
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml

# 2 — pick the conversion style you want in product code
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
```

**Rule of thumb:** one conversion style per schema type
(`with_domain_type` *or* `with_conversion`, not both for the same selector).
See the main [ergo-sbe README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md)
configuration section.

## `ergo-sbe`: build dependency vs application dependency

Generated codecs ship their own embedded `sbe_rt` module. Linking the **app**
does not require `ergo-sbe` unless you use its macros or call the generator
library at runtime.

| Pattern | `build-dependencies` | `dependencies` | Typical use |
|---------|----------------------|----------------|-------------|
| **Build only** (**product default**) | `ergo-sbe` | — | `generate_to_out_dir` in `build.rs`, then plain `include!(concat!(env!("OUT_DIR"), "/….rs"))` |
| **Build + runtime** (convenience) | `ergo-sbe` | `ergo-sbe` | Same codegen, plus `ergo_sbe::sbe_mod!` / `include_sbe!` |
| **Runtime only** | — | `ergo-sbe` | Call `parse` / `Generator` as a library (no `build.rs`) |

| Sample | Pattern | Purpose | External requirements |
|---|---|---|---|
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Build + runtime** | **Teaching / feature map** — EncodedLength, stages, DTO, AnyMessage, **both** conversion styles | None |
| [`l3-book/`](l3-book/) | **Build only** | Nested/ragged L3 books; **`with_domain_type` only** | None for local tests |
| [`exchange-example/`](exchange-example/) | **Build + runtime** | Multi-schema + **`with_conversion` only** + Aeron IPC | Network only for live exchange paths |
| [`cluster-rfq/`](cluster-rfq/) | **Build only** | RFQ / auction protocol codecs + cluster examples | Java harness for live examples |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | **Build + runtime** | Claim-based Cluster publishing + HA-shaped book | Java harness only for leader-kill coverage |
| [`sbe-codegen-examples/`](sbe-codegen-examples/) | **Runtime only** | Generator API as a library (no `build.rs`) | None |
| [`cluster-tutorial/`](cluster-tutorial/) | **Neither** (uses `ergo-aeron-cluster`) | Connect, offer, poll, keep-alive, close walkthrough | Java 17+ and built Aeron artifacts |

**Build-only include shape** (preferred for products — no runtime `ergo-sbe`):

```rust
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
```

**Build + runtime** (macro convenience):

```rust
ergo_sbe::sbe_mod!(messages);
```

## Buffer sizing (samples & tests)

- **Const-sized messages:** stack `[0u8; MsgEncoder::ENCODED_LENGTH]`
- **Dynamic / ragged:** size with `*EncodedLength` / `compute_encoded_length_*`,
  then encode into a claim/slot of that exact length — avoid oversize
  `vec![0u8; 4096]` “guess” buffers

See the main README [buffer sizing](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#buffer-sizing)
section and feature-tour `demo_car_size_and_encode`.

## Domain DTOs & var-data

```rust
.enable_domain_objects(DomainVarData::LossyStrings) // String; bad UTF-8 → ""
.enable_domain_objects(DomainVarData::Bytes)        // Vec<u8>; byte-exact
```

`LossyStrings` is **not** lossless on re-encode of invalid UTF-8 (field becomes
empty). Feature-tour uses `LossyStrings`; l3-book uses `Bytes` where tails are
byte-oriented.

## Check each sample

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

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs (prefer stack when const).
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
