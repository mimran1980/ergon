# Samples

These five standalone crates exercise repository APIs in larger flows. They are
excluded from the workspace, set `publish = false`, and are not reference
implementations.

| Sample | Purpose | External requirements |
|---|---|---|
| [`exchange-example/`](exchange-example/) | Multi-schema generation, domain objects, market-data state, and Aeron IPC | Network only for live exchange paths |
| [`l3-book/`](l3-book/) | Exact sizing, nested and ragged groups, variable data, conversions, and domain-object round trips | None for local tests |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Claim-based Cluster publishing and an HA-shaped order-book flow | Java harness only for leader-kill coverage |
| [`cluster-rfq/`](cluster-rfq/) | RFQ and auction application-protocol experiments | Java harness for live examples |
| [`cluster-tutorial/`](cluster-tutorial/) | Connect, offer, poll, keep-alive, and close walkthrough | Java 17+ and built Aeron artifacts |

## Check each sample

Run standalone packages through their manifest paths. These are diagnostic
entry points: samples intentionally move with experimental APIs, so a failing
command identifies migration drift rather than a supported product regression.

```sh
cargo check --manifest-path samples/exchange-example/Cargo.toml --all-targets
cargo check --manifest-path samples/l3-book/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-ha-orderbook/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-rfq/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-tutorial/Cargo.toml --all-targets
```

Useful service-free tests:

```sh
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

## L3 sample

The L3 sample is the main generated-code migration target. Its schema contains:

- fixed fields with `chrono`, `bool`, and `rust_decimal` mappings;
- nested bid/ask and order groups;
- ragged entry shapes;
- variable-length order identifiers;
- a three-level group-to-variable-data tail.

Its helpers compute the complete header-inclusive wire length before allocating,
encode into exactly that buffer, decode through generated flyweights, and check
owned domain-object round trips. Read
[`l3-book/src/main.rs`](l3-book/src/main.rs) and
[`l3-book/src/lib.rs`](l3-book/src/lib.rs) alongside the schema.

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs.
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
