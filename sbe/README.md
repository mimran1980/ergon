# ergosbe (`sbe/`)

SBE XML → idiomatic Rust codec generator. Core pillar of the ErgoSBE umbrella.

## Status

**Experimental product crate.** Maintained ErgoSBE vs Aeron SBE matrix is green
(10/10 ≤ 1.00 as of 2026-07-18). Not a universal “HFT-ready” claim beyond that set.

## Depends on

- Rust MSRV **1.95** (workspace)
- Official SBE semantics / wire shape (see design authority below)

## Build / test

```sh
cargo test -p ergosbe --lib
cargo test -p ergosbe --test baseline_test
cargo bench -p ergosbe-benchmarks --no-run   # from repo root
just bench                                   # Aeron parity matrix
```

## Layout

| Path | Role |
|------|------|
| `src/xml.rs`, `schema.rs` | Parse / validate SBE XML |
| `src/ir.rs`, `resolve.rs` | Intermediate representation + offsets |
| `src/config.rs` | Generation options |
| `src/codegen.rs` | Rust source generation (`syn` / `quote` / `prettyplease`) |
| `design/DECISIONS.md` | Canonical design authority |
| `docs/guide/` | Getting started, schema authoring, generated API, migration |
| `tests/` | Wire, golden, compile-fail, allocation proofs |

## Public entry points

- `parse` / `parse_file` → IR
- `Schema::from_ir`
- `GenerationConfig` + `Generator::try_generate` / `generate`
- Typical consumer: call from **your** `build.rs`, `include!` from `OUT_DIR`

## Where truth lives

- Design: [`design/DECISIONS.md`](design/DECISIONS.md)
- Guide: [`docs/guide/getting-started.md`](docs/guide/getting-started.md)
- Perf ledger: [`../ergosbe-performance-optimisation-goal.md`](../ergosbe-performance-optimisation-goal.md)
- Crate rustdoc: `cargo doc -p ergosbe --open`

## Non-goals

- Nightly-only APIs, speculative SIMD bulk copy, broad per-field unchecked families
- Transmute / native-endian casts from wire buffers
- Hand-editing generated sample codecs instead of regenerating from XML
