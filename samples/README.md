# Samples

Standalone crates that exercise repository APIs. They are **excluded from the
workspace**, set `publish = false`, and are **not** production reference
implementations — they move with experimental APIs on purpose.

## Start here (product teaching path)

| Step | Sample | Why |
|------|--------|-----|
| **1** | [`sbe-feature-tour/`](sbe-feature-tour/) | **Golden path.** Full feature map: stages, EncodedLength, checked constructors + verify, Display, DTO with `DomainVarData::Strings`, all three conversion styles |
| **2a** | [`l3-book/`](l3-book/) | Nested/ragged books; **`with_domain_type` only**; **build-dep only** (plain `include!`) |
| **2b** | [`exchange-example/`](exchange-example/) | Multi-schema; **`with_conversion` only**; IPC + app `TryFromSbe` |
| **3** | [`sbe-codegen-examples/`](sbe-codegen-examples/) | Generator **as a library** (no `build.rs`) |
| Later | [`cluster-tutorial/`](cluster-tutorial/) | Connect, offer, poll, keep-alive, close |
| Later | [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Claim-based Cluster publishing + HA-shaped book |
| Later | [`cluster-rfq/`](cluster-rfq/) | RFQ / auction codecs over Cluster |

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
| **Build only** (**product / samples default**) | `ergo-sbe` | — | `generate_to_dir` → `src/generated/` (gitignored) + `#[path = "generated/….rs"]` |
| **OUT_DIR only** | `ergo-sbe` | — | `generate_to_out_dir` + `include!(concat!(env!("OUT_DIR"), …))` — fine for apps; **poor IDE go-to-def** |
| **Build + runtime** | `ergo-sbe` | `ergo-sbe` | Macros such as `sbe_mod!` plus build-time generation |
| **Runtime only** | — | `ergo-sbe` | Call `parse` / `Generator` as a library (no `build.rs`) |

| Sample | Pattern | Purpose | External requirements |
|---|---|---|---|
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Build only** | **Teaching / feature map** — EncodedLength, stages, DTO, AnyMessage, **all three** conversion styles | None |
| [`l3-book/`](l3-book/) | **Build only** | Nested/ragged L3 books; **`with_domain_type` only** | None for local tests |
| [`exchange-example/`](exchange-example/) | **Build only** | Multi-schema + **`with_conversion` only** + Aeron IPC | Network only for live exchange paths |
| [`cluster-rfq/`](cluster-rfq/) | **Build only** | RFQ / auction protocol codecs + cluster examples | Java harness for live examples |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | **Build only** (may still dep cluster) | Claim-based Cluster publishing + HA-shaped book | Java harness only for leader-kill coverage |
| [`sbe-codegen-examples/`](sbe-codegen-examples/) | **Runtime only** | Generator API as a library (no `build.rs`) | None |
| [`cluster-tutorial/`](cluster-tutorial/) | **Neither** (uses `ergo-aeron-cluster`) | Connect, offer, poll, keep-alive, close walkthrough | Java 17+ and built Aeron artifacts |

### Seeing generated code (without committing it)

`include!(concat!(env!("OUT_DIR"), …))` and `sbe_mod!` put files under a
hashed path like `target/debug/build/<crate>-<hash>/out/….rs` — hard to find
and rust-analyzer usually **cannot** jump into them.

Samples instead write to a **stable, local path**:

```text
samples/<name>/src/generated/*.rs   # created on cargo build, gitignored
```

1. `cargo build --manifest-path samples/sbe-feature-tour/Cargo.toml`
2. Open `samples/sbe-feature-tour/src/generated/feature_tour.rs`
3. From app code, **Go to definition** on `CarEncoder` / etc. should land there

Root `.gitignore` has `**/src/generated/`. Do **not** commit those trees
(Binance alone is multi‑MB). Rebuild after a clean clone.

```rust
// build.rs
let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
ergo_sbe::generate_to_dir("schemas/messages.xml", config, &out)?;

// src/lib.rs — real path → IDE go-to-definition works
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all, warnings)]
#[path = "generated/messages.rs"]
mod messages;
```

## Buffer sizing (samples & tests)

- **Const-sized messages:** stack `[0u8; MsgEncoder::compute_length()]`
- **Dynamic / ragged:** size with `*EncodedLength` / `compute_length_with_header(…)`,
  then encode into a claim/slot of that exact length — avoid oversize
  `vec![0u8; 4096]` “guess” buffers

See the main README [buffer sizing](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#buffer-sizing)
section and feature-tour `demo_car_size_and_encode`.

## Domain DTOs & var-data

```rust
.with_domain_objects(DomainVarData::Strings) // String; bad UTF-8 → InvalidUtf8 error
.with_domain_objects(DomainVarData::Bytes)   // Vec<u8>; byte-exact
```

`Strings` is strict UTF-8 (no empty-string fallback). Feature-tour uses
`Strings`; l3-book uses `Bytes` where tails are byte-oriented.

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
| [`l3-book/`](l3-book/) | **`with_domain_type` only** | `dec.try_price()?` → `Decimal`; `enc.try_price(d)?` |
| [`exchange-example/`](exchange-example/) | **`with_conversion` only** | `dec.price_as::<T>()?`; `enc.price_from(&t)?` (+ app `TryFromSbe`) |
| [`sbe-feature-tour/`](sbe-feature-tour/) | **All three** (different selectors) | bool/timestamp concrete (`Generated`); Decimal generic (`demo_conversion_only`); ManualDecimal concrete + app impl (`demo_domain_type_manual_impl`) |

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
