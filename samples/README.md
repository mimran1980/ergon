# Samples

These five standalone crates exercise repository APIs in larger flows. They are
excluded from the workspace, set `publish = false`, and are not reference
implementations.

| Sample | Purpose | External requirements |
|---|---|---|
| [`sbe-feature-tour/`](sbe-feature-tour/) | **ErgoSBE feature map** — EncodedLength, encode/decode, DTO, AnyMessage, **and both** `with_domain_type` + `with_conversion` | None |
| [`exchange-example/`](exchange-example/) | Multi-schema + **`with_conversion` only** (app-side `TryFromSbe` for rust_decimal) + Aeron IPC | Network only for live exchange paths |
| [`l3-book/`](l3-book/) | Nested/ragged L3 books; **`with_domain_type` only** (concrete `price() -> Decimal`, etc.) | None for local tests |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Claim-based Cluster publishing and an HA-shaped order-book flow | Java harness only for leader-kill coverage |
| [`cluster-rfq/`](cluster-rfq/) | RFQ and auction application-protocol experiments | Java harness for live examples |
| [`cluster-tutorial/`](cluster-tutorial/) | Connect, offer, poll, keep-alive, and close walkthrough | Java 17+ and built Aeron artifacts |

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
chrono / bool). See [`l3-book/README.md`](l3-book/README.md).

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs.
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
