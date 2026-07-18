# ErgoSBE

> **Experimental.** This repository is an experimental umbrella project.
> APIs, directory layout, and crate names may change without notice.

An experimental Rust workspace for low-latency trading infrastructure, built
around four pillars: SBE code generation, ClickHouse persistence, an Aeron
Cluster client, and end-to-end samples.

## Project layout

| Directory | Crate | Purpose |
|-----------|-------|---------|
| `sbe/` | `ergosbe` | SBE XML → idiomatic Rust codec generator (the core pillar) |
| `persist/` (+ `persist/derive/`) | `ergo-clickhouse-persist` | Auto-persist annotated structs to ClickHouse |
| `cluster/` | `ergo-aeron-cluster` | Aeron Cluster client prototype on `rusteron-client` |
| `cluster-test-support/` (excluded) | `ergo-aeron-cluster-test-support` | Java test harness for the cluster crate (Gradle-built Aeron jars) |
| `ergosbe-benchmarks/` | `ergosbe-benchmarks` | Criterion Aeron-parity benchmark matrix |
| `samples/` (excluded) | — | End-to-end demos (`advanced-bitget`, `exchange-orderbook`) |

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
| `just check` | Hygiene, fmt, clippy, workspace tests, both samples, cluster lib | Rust only |
| `just check-aeron-cluster` | Cluster fmt/clippy/53 lib tests | Rust only |
| `just build-aeron-jars` then `just test-aeron-cluster-harness` | Full cluster integration suite (connect/auth/failover/restart/archive) | Java 17+ |
| `just bench` | Aeron perf-parity matrix (ErgoSBE vs Aeron SBE) | Rust only |
| `just bench-cluster` | Cluster codec encode head-to-head (ErgoSBE vs sbe-tool) | Rust only |
| `just test-clickhouse-live` / `just test-exchange-orderbook-live` / `just samples-orderbook` | Live ClickHouse E2E (IPC samples) | Docker |
| `just check-aeron-cluster-codec-drift` | Residual committed sbe-tool codecs match regenerate (transitional) | Java (sbe-tool) |

**Gotcha:** never run `cargo … --workspace --all-features` without
`--exclude ergo-aeron-cluster` — the cluster's `test-harness` feature pulls the
Java/Gradle-building test-support crate.

**Layout:** pillar directories `sbe/`, `persist/`, `cluster/`, `samples/` are
permanent names (never rename). Cluster dir names intentionally differ from
crate names (`ergo-aeron-cluster`).

Living plan:
[`docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md`](docs/superpowers/plans/2026-07-18-ergosbe-experimental-master-plan.md).  
Residual goal prompt:
[`docs/superpowers/plans/2026-07-18-completion-goal-prompt.md`](docs/superpowers/plans/2026-07-18-completion-goal-prompt.md).  
Planned HA sample (cluster feed, never-stale book, dynamic latency → CH):
[`docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md`](docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md).

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

- Local `ergosbe` tests, formatting, clippy, and generated-code stability checks
  are tracked in [`sbe/todos/TESTING_PLAN.md`](sbe/todos/TESTING_PLAN.md).
- **Maintained ErgoSBE/Aeron five-run matrix:** all 10 scenarios ≤ 1.00 as of
  2026-07-18 (see
  [`ergosbe-performance-optimisation-goal.md`](ergosbe-performance-optimisation-goal.md)).
  That is evidence for the maintained set — not a universal “HFT-ready” claim.
- Persist live ClickHouse and both IPC samples (`just samples-orderbook`) are
  green for current scope; live exchange WebSocket remains a manual recipe.
- **Cluster:** production codecs are ErgoSBE-generated; SessionConnectRequest
  encode vs sbe-tool is still **OPEN** at ~1.003 on a first Criterion run;
  reliability gaps and HA sample track remain (master plan §4–5).
- Advanced Rust proof APIs such as verified frames, typed frame policies,
  scoped callbacks, and required-field proofs stay roadmap until their
  runtime, compile-fail, and benchmark gates pass.

## Stable Rust Advantage Roadmap

ErgoSBE uses stable Rust features to reduce the public interface while keeping
the generated implementation zero-cost; every performance conclusion still
requires the maintained Aeron comparison:

- **Sealed proof tokens and marker types** for checked/verified/unchecked
  decoder modes, schema identity, and frame policy.
- **Associated codec types** on `SbeMessage` for monomorphised generic helpers.
- **HRTB-scoped callbacks** so borrowed decoder views cannot escape a feed frame.
- **Return-position `impl Trait`** to hide generated iterator/helper type names.
- **Const/static templates** for header and group dimension setup.
- **Optional `#[repr(transparent)]` semantic newtypes** for domain safety without
  changing the wire representation.

The stable-Rust roadmap is tracked in
[`sbe/todos/144-stable-rust-advantage-roadmap.md`](sbe/todos/144-stable-rust-advantage-roadmap.md).

## Quick start

### 1. Add dependency

```toml
[build-dependencies]
ergosbe = "0.1"
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

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
