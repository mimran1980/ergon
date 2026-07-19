# ErgoSBE

> **Experimental.** This repository is an experimental umbrella project.
> APIs, directory layout, and crate names may change without notice.

An experimental Rust workspace for low-latency trading infrastructure, built
around four pillars: SBE code generation, ClickHouse persistence, an Aeron
Cluster client, and end-to-end samples.

## Start here

| Goal | Go to |
|------|--------|
| Generate codecs / wire API | [`sbe/docs/guide/getting-started.md`](sbe/docs/guide/getting-started.md) |
| Claim + nested AppMessage encode | [`sbe/docs/guide/claim-nested-encode.md`](sbe/docs/guide/claim-nested-encode.md) |
| Verified-open work only | [`docs/LIVING_BACKLOG.md`](docs/LIVING_BACKLOG.md) |
| Full local gate | `just check` |
| Cluster client | [`cluster/README.md`](cluster/README.md) |
| Samples map | [`samples/README.md`](samples/README.md) |

Path dependency in this monorepo (`ergosbe = { path = "sbe" }`); crates.io
version numbers in guides are illustrative.

## Project layout

| Directory | Crate | Purpose | Docs |
|-----------|-------|---------|------|
| `sbe/` | `ergosbe` | SBE XML → idiomatic Rust codec generator | [`sbe/README.md`](sbe/README.md) |
| `persist/` (+ `persist/derive/`) | `ergo-clickhouse-persist` | Auto-persist annotated structs to ClickHouse | [`persist/README.md`](persist/README.md), [`persist/derive/README.md`](persist/derive/README.md) |
| `cluster/` | `ergo-aeron-cluster` | Aeron Cluster client on `rusteron-client` 0.2 | [`cluster/README.md`](cluster/README.md) |
| `cluster-test-support/` (excluded) | `ergo-aeron-cluster-test-support` | Java test harness (Gradle Aeron jars) | [`cluster-test-support/README.md`](cluster-test-support/README.md) |
| `ergosbe-benchmarks/` | `ergosbe-benchmarks` | Criterion Aeron-parity matrix | [`ergosbe-benchmarks/README.md`](ergosbe-benchmarks/README.md) |
| `samples/` (excluded) | — | End-to-end demos (IPC + HA) | [`samples/README.md`](samples/README.md) |

| Sample | Transport | Recipe |
|--------|-----------|--------|
| [`advanced-bitget`](samples/advanced-bitget/) | Aeron **IPC** (+ multi-schema / Persist DTO) | `just samples-orderbook` / `just test-ipc` |
| [`cluster-ha-orderbook`](samples/cluster-ha-orderbook/) | Aeron **Cluster** | `just samples-cluster-ha` |

Two harnesses only (former `exchange-orderbook` merged into advanced-bitget).
All rusteron crates use **`0.2`** (^0.2 → latest 0.2.x).

```text
IPC:      AppMessage claims → Aeron IPC → typed/dynamic CH + Persist snapshot DTO
Cluster:  try_claim → Java cluster → never-stale book + feed_latency DynamicRow
```

Note: for the cluster pillar the **directory names differ from the crate
names** on purpose — the dirs are `cluster/` and `cluster-test-support/`, the
crates remain `ergo-aeron-cluster` and `ergo-aeron-cluster-test-support`.
"Excluded" crates are not workspace members; each builds standalone.

## Submodules

- `simple-binary-encoding/` — the official SBE reference implementation
  (wire-compatibility reference and Java tooling).
- `aeron/` — Aeron pinned at 1.52.2 (`5b62f21d91`): the cluster SBE schema
  source of truth and the Gradle build for test jars.

```sh
git submodule update --init --recursive
```

## Gates

| Command | What it proves | Needs |
|---------|----------------|-------|
| `just check` | Hygiene, fmt, clippy, workspace tests, IPC samples, cluster lib | Rust only |
| `just check-aeron-cluster` | Cluster fmt/clippy/lib tests | Rust only |
| `just build-aeron-jars` then `just test-aeron-cluster-harness` | Full cluster integration suite | Java 17+ |
| `just bench` | Aeron perf-parity matrix (ErgoSBE vs Aeron SBE) | Rust only |
| `just bench-cluster` | Cluster codec encode+decode head-to-head (ErgoSBE vs sbe-tool) | Rust only |
| `just samples-orderbook` | Live ClickHouse E2E (IPC sample) | Docker |
| `just samples-cluster-ha` | HA offline + feed_latency CH | Docker |
| `just samples-cluster-ha-kill-leader` | Multi-node never-stale book | Java + jars |
| `just check-aeron-cluster-codec-drift` | Residual sbe-tool trees match regenerate (benches only) | Java (sbe-tool) |

**Gotcha (`--all-features` + cluster):** `ergo-aeron-cluster` *is* a workspace
member. Optional feature `test-harness` pulls `cluster-test-support` (Java/Aeron
jars). So `just build` / `just check` run:

```text
cargo … --workspace --all-features --exclude ergo-aeron-cluster
cargo … -p ergo-aeron-cluster          # default features only (pure Rust)
```

Never bare `cargo … --workspace --all-features` if you want a Java-free gate.
Harness: `just build-aeron-jars` then `just test-aeron-cluster-harness`.

**Layout:** pillar directories `sbe/`, `persist/`, `cluster/`, `samples/` are
permanent names (never rename). Cluster dir names intentionally differ from
crate names (`ergo-aeron-cluster`).

### Release (crates.io)

Publish **product** crates one-by-one at **default features** (do not enable
`test-harness` for a publish build):

1. `ergosbe` (`sbe/`)
2. `ergo-clickhouse-persist-derive` then `ergo-clickhouse-persist`
3. `ergo-aeron-cluster` — default features only; keep harness optional

**Do not publish:** `ergosbe-benchmarks` (`publish = false`), samples,
`cluster-test-support`. Tag the monorepo after a coordinated release. Downstream
apps use crates.io versions; in-repo samples keep `path = …`. The Aeron submodule
pin is independent of crate version numbers.

**Verified-open backlog only:** [`docs/LIVING_BACKLOG.md`](docs/LIVING_BACKLOG.md).  
Umbrella orientation (historical):
[`docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md`](docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md).  
HA sample:
[`samples/cluster-ha-orderbook/`](samples/cluster-ha-orderbook/).

---

# Pillar: sbe (ErgoSBE codegen)

Opinionated, idiomatic Rust code generation for [Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/) (SBE).

ErgoSBE reads SBE XML schemas and produces safe, fast, version-aware Rust codecs.
The project goal is byte-for-byte compatibility with the official SBE reference
implementation, with an API shaped for Rust rather than translated from Java.

## Features

These are the implemented/generated capabilities. Release-quality claims such as
"full Aeron parity", "HFT-ready", and "safe by parse" are gated by
[`sbe/todos/123-release-quality-gates.md`](sbe/todos/123-release-quality-gates.md).

- **XML schema parsing** — parse SBE schemas with XInclude support, miette diagnostics
- **Encoder/Decoder generation** -- zero-allocation fixed-block views plus concrete consuming tail stages
- **Infallible field accessors** — scalar, enum, set, and composite accessors are plain `fn(&self) -> T`, no unwrapping
- **Flat enum generation** — enums are true Rust `enum`s with a `NullVal` variant for unknown wire values (no separate `Kind` type)
- **Buffer verification** — `Decoder::verify(&[u8])` validates an entire message buffer before decoding, reporting group/vardata bounds
- **Version-aware decoding** — all accessors respect the wire message version
- **Repeating groups** -- concrete ordered group/entry stages; runtime counts remain wire-validated
- **Variable-length data** — var-data with length-prefixed byte slices and optional UTF-8 accessors
- **AnyMessage dispatch** — `AnyMessage` enum with `Unknown` forwarding for external frames
- **FrameCursor** — iterate externally-framed SBE feed buffers (length-prefix or fixed-size)
- **Multi-schema** — `generate_multi` for projects with shared type definitions across schemas
- **Ordered tail stages** -- encoder and decoder enforce group/var-data order at compile time
- **Optional/null handling** — `Option<T>` return types for optional and version-gated fields
- **Feature-gated trusted-input path** -- `bound-check-disabled` keeps accessor names stable while routing validated/trusted inputs through unchecked internals
- **Compile-time constants** — `FIELD_NULL`, `FIELD_MIN`, `FIELD_MAX` on every decoded field

## Current Status

- **Maintained ErgoSBE/Aeron five-run matrix:** all 10 scenarios ≤ 1.00 as of
  2026-07-18 ([`ergosbe-performance-optimisation-goal.md`](ergosbe-performance-optimisation-goal.md)).
  Evidence for that set only — not a universal “HFT-ready” claim.
- Persist + IPC samples (`just samples-orderbook`) green when CH is up; live
  exchange WebSocket remains a **manual** recipe (not CI).
- **Cluster:** residual product complete; claim_shaped encode maintained;
  SessionConnectRequest encode and NewLeader decode are **not** ≤1.00 gates.
- Open work: [`docs/LIVING_BACKLOG.md`](docs/LIVING_BACKLOG.md) only
  (`sbe/todos/` is a historical inventory).

## Quick start

### 1. Add dependency (monorepo path)

```toml
[build-dependencies]
ergosbe = { path = "../sbe" }   # or crates.io "0.1" when published
```

### 2. Create `build.rs`

```rust
use ergosbe::{parse_file, Generator, GenerationConfig, Schema};

fn main() {
    // Parse an SBE XML schema file (with XInclude resolution)
    let ir = parse_file("schemas/my_schema.xml").unwrap();
    let schema = Schema::from_ir(ir);

    // Configure the generator
    let config = GenerationConfig::new("my_messages");
    let generator = Generator::new(config);

    // Generate Rust source
    let output = generator.generate(&schema);

    // Write to the output directory
    let out_dir = std::env::var("OUT_DIR").unwrap();
    for module in output.modules() {
        std::fs::write(
            format!("{}/{}", out_dir, module.path),
            &module.source,
        ).unwrap();
    }
}
```

### 3. Use generated code

Scalar, enum, set, and composite field accessors are **infallible** -- no `?`, no `unwrap`:

```rust
// Include the generated module
include!(concat!(env!("OUT_DIR"), "/my_messages.rs"));

fn decode_message(buf: &[u8]) -> Result<(), sbe_rt::DecodeError> {
    let car = CarDecoder::wrap_and_apply_header(buf, 0)?;
    let serial = car.serial_number();           // u64 -- infallible
    let year = car.model_year();                // u16 -- infallible
    let model = car.code();                     // Model (flat enum) -- infallible
    let extras = car.extras();                  // OptionalExtras (set) -- infallible
    let engine = car.engine();                  // Engine (composite) -- infallible
    println!("Car #{} ({})", serial, year);

    // Groups and var-data use concrete consuming stages. Later tail
    // components are unavailable until earlier ones are finished/skipped.
    let fuel = car.fuel_figures()?;
    let after_fuel = fuel.finish()?;
    let _next = after_fuel.performance_figures()?;
    Ok(())
}
```

## Architecture

| Layer | Module | Description |
|-------|--------|-------------|
| Schema Input | `xml`, `schema` | Parse SBE XML, resolve includes, validate |
| Intermediate Repr | `ir`, `resolve` | Token stream, offset/block-length pass |
| Generation Options | `config` | Module name, wire-compatibility policy |
| Code Generation | `codegen` | Rust source production |

## Related crates

- **[`ergo-clickhouse-persist`](persist/README.md)** — debugging persistence:
  auto-persist annotated Rust structs to ClickHouse with automatic schema
  management. Consumer-side only, never on the hot path.

## Design priorities

1. **Official-SBE wire compatibility is non-negotiable.**
2. **ErgoSBE must be equal to or faster than Aeron SBE in every maintained,
   measured scenario.**
3. **Prefer an easier or safer Rust API** when it is zero-cost or outside the
   hot path.
4. **Do not slow a benchmarked hot path** with a safety check, abstraction,
   branch, or ergonomic wrapper unless it is an explicit opt-in.
5. **Use simplicity as the tie-breaker** only when compatibility, performance,
   and safety are equal.

Generated ordered hot paths allocate no heap memory, and all decoding remains
acting-version/acting-block-length aware. Trusted-input fast paths are explicit
whole-path opt-ins rather than per-field unchecked API families.

See [`sbe/design/DECISIONS.md`](sbe/design/DECISIONS.md) for the complete design rationale.

## Development

Prefer `just check` (matches CI hygiene). Equivalent manual gates:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
cargo test -p ergo-aeron-cluster --lib
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
