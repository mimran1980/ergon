# Introduction

`ergo-sbe` generates zero-allocation Rust codecs from
[Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/) schemas.
Wire layout matches official SBE for the
[published profile](https://github.com/mimran1980/ergon/blob/main/docs/SBE_COMPATIBILITY.md).
Tail order is a **type error**, not a runtime surprise.

## Quick start

```toml
[build-dependencies]
ergo-sbe = "0.1"
```

```rust,no_run
{{#include ../examples/heartbeat-encode.rs:staged_chaining}}
```

*(From `book/examples/heartbeat-encode.rs` — compiled against the feature-tour codec.)*

**[Set up →](sbe/getting-started/depend.md)** ·
**[Feature tour →](sbe/feature-tour.md)** ·
**[Coming from sbe-tool →](sbe/getting-started/from-sbe-tool.md)**

| | |
|---|---|
| **Wire order** | Named stages — `asks` before `bids` does not compile |
| **Exact sizing** | `compute_length_with_header` / `*EncodedLength` before you write; works with Aeron `try_claim` |
| **Groups** | Closures nest like the schema; no `.parent()` |
| **Trust boundary** | `try_*` → `Result`; bare `wrap` panics if short; bare `decode` panics if short and `Err` on wrong template/schema; `unsafe *_unchecked` skips the proof |
| **Composites** | `#[repr(transparent)] Engine([u8; N])` — the value is the on-wire bytes |
| **Domain types** | Map wire `Decimal` to `rust_decimal::Decimal` with one config line |
| **Bulk groups** | `bulk_add(&[Entry])` / `bulk_decode()` for fixed-stride leaf groups |
| **Runtime deps** | Generated codecs embed `sbe_rt` — no `ergo-sbe` on the hot path |

0.x experimental. Pin versions. Exit criteria: [Road to 1.0](project/road-to-1.0.md).

It is **not** a line-for-line port of sbe-tool. The generated API is meant to be
easier under Rust's borrow checker, safer at untrusted boundaries, and still
official-SBE on the wire. Nested groups use **scoped closures** so you do not
hand encoder ownership back with `.parent()`:

```rust,no_run
{{#include ../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(Compiled and run in CI.)*

Wire parity is the dual-encode suite (`sbe_tool_wire_parity_test`,
`sbe_tool_multi_schema_wire_parity_test`) plus the maintained benchmark gate.
Quote a bench result by run id, not a number copied here. See
[Benchmarks](sbe/benchmarks.md).

## Workspace

| Path | Package | Role |
|---|---|---|
| `sbe/` | `ergo-sbe` | Schema parser and codec generator |
| `sbe/benchmarks/` | `ergo-sbe-benchmarks` | Unpublished parity benches |
| `cluster/` | `ergo-aeron-cluster` | Experimental Aeron Cluster client |
| `samples/` | seven crates | Unpublished playgrounds (`publish = false`) |

Rust 1.88+. Clone setup, `just` recipes, and project boundaries:
[repository README](https://github.com/mimran1980/ergon/blob/main/README.md).
