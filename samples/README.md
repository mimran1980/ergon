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

## Conversion config: which sample uses what

| Sample | Config | Why |
|--------|--------|-----|
| [`l3-book/`](l3-book/) | **`with_domain_type` only** | One canonical app type per field (`rust_decimal`, `bool`, `chrono`). That API **implies** conversion — you do **not** also call `with_conversion` for the same selectors. You get concrete methods like `price() -> rust_decimal::Decimal`. |
| [`exchange-example/`](exchange-example/) | **`with_conversion` only** | Pluggable seam: generated `price_as` / `price_from`; app implements `TryFromSbe` (see its decimal adapter). |
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Both** | Teaching sample: domain types for bool/timestamp; conversion-only Decimal on `Quote` (`demo_conversion_only`). |

## ErgoSBE feature tour

[`sbe-feature-tour/`](sbe-feature-tour/) is the map from **product README claims →
runnable code + schema**. Prefer it when documenting or teaching:

- fixed `ENCODED_LENGTH`
- staged `*EncodedLength` (including ragged groups)
- encoder `fixed` + consuming tails
- decoder consuming stages + strict text
- `CarDomain` DTO + byte-identical re-encode
- multi-message `AnyMessage`
- `try_*` / `verify` vs trusted `wrap`
- `with_conversion` vs `with_domain_type` (see sample README + `demo_conversion_only`)

```sh
cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml
```

## L3 sample

The L3 sample is the deep nested/ragged migration target. It intentionally uses
only `with_domain_type` in [`l3-book/build.rs`](l3-book/build.rs) — not bare
`with_conversion` — because the book always speaks `rust_decimal` / `chrono` /
`bool`. Domain type already enables the conversion machinery; adding
`with_conversion(Decimal)` on top would be a no-op for the same selector.

Schema highlights:

- fixed fields with `chrono`, `bool`, and `rust_decimal` mappings;
- nested bid/ask and order groups;
- ragged entry shapes;
- variable-length order identifiers;
- a three-level group-to-variable-data tail.

Its helpers compute the complete header-inclusive wire length before allocating,
encode into exactly that buffer, decode through generated flyweights, and check
owned domain-object round trips. Read
[`l3-book/README.md`](l3-book/README.md),
[`l3-book/src/main.rs`](l3-book/src/main.rs) and
[`l3-book/src/lib.rs`](l3-book/src/lib.rs) alongside the schema.

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs.
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
