# Introduction

`ergo-sbe` generates zero-allocation Rust codecs from
[Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/) (SBE)
schemas with **official-SBE wire fidelity** inside the published profile
([compatibility](https://github.com/mimran1980/ergon/blob/main/docs/SBE_COMPATIBILITY.md)).
Wire-order safety is enforced at **compile time** — calling methods out of schema
order is a type error. All 10 maintained SBE parity benchmarks pass at or below the
`1.00×` sbe-tool ceiling under both LTO profiles (0.1.10).

## Quick start

```toml
# Cargo.toml
[build-dependencies]
ergo-sbe = "0.1"
```

```rust,no_run
{{#include ../examples/heartbeat-encode.rs:staged_chaining}}
```

**[Set up →](sbe/getting-started/depend.md)** a 3-minute path from zero to
working codec. All API surfaces are covered in the
**[Feature Tour →](sbe/feature-tour.md)** with compilable examples.

## Feature overview

| | ergo-sbe |
|---|----------|
| **Wire-order safety** | Compile-time type-state stages — calling `asks` before `bids` is a type error, not a runtime bug |
| **Exact buffer sizing** | `compute_length_with_header(…)` gives the exact byte count before you encode — no oversize scratch buffers, works directly with Aeron `try_claim` |
| **Closure-based groups** | `bids(n, \|g\| g.add(\|e\| { … }))` — nests like the schema, no `.parent()` hopscotch |
| **Trust boundary** | `decode` / `try_from` / `wrap` return `Result` and validate extents; private zero-check cores only after HFT-008 keep |
| **Composite wire images** | `#[repr(transparent)] Engine([u8; N])` — the value IS the on-wire bytes, zero-copy with portable LE/BE accessors |
| **Domain types** | Map wire `Decimal` to `rust_decimal::Decimal` at the codec boundary — one line of config, no hand-rolled converters |
| **Bulk group ops** | `bulk_add(&[Entry])` / `bulk_decode()` — measured about 22-23% lower encode latency than `add()` for 1,000-entry flat groups on the audited Apple M4 profiles |
| **Zero dependencies at runtime** | Generated codecs embed their own `sbe_rt` — no `ergo-sbe` on your critical path |

ergon is an experimental Rust workspace. **ergo-sbe** generates the codecs;
**ergo-aeron-cluster** is a client-only Aeron Cluster experiment built on
rusteron. Neither crate is production-ready today. APIs may change. Verify
wire compatibility, failure handling, and performance for your own schemas.
Exit criteria: [Road to 1.0](project/road-to-1.0.md).

ergo-sbe parses [Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/)
(SBE) schemas and generates Rust codecs that match the **official SBE wire
layout** for the features listed in the compatibility profile (header, field
layout, groups, var-data, byte order — see `docs/SBE_COMPATIBILITY.md`).

It is **not** a line-for-line port of the java/rust sbe-tool stubs. The
goals for the generated API are:

1. **Easier to use** — especially nested groups and var-data under Rust’s
   borrow checker
2. **Safer** — wire order and trust boundaries enforced by types / `Result`
3. **Easier to read** — nested structure looks like the schema, not a pile of
   temporary handles

Still built for **low-latency** official-SBE wire work. The style uses
**named stage structs** (not `Encoder<State>` generics), **closures + method
chaining** for groups, checked entry points, version-aware accessors, and
optional domain/conversion helpers.

### Why not “Java-style” parent hopping?

In sbe-tool you often juggle flyweights and call something like `.parent()` to
hand ownership back up the tree. In Rust that fight becomes **borrow-checker
pain**: move a group encoder in, get stuck returning the parent, lose the
thread of the code.

ergo-sbe leans on **scoped closures** and **chaining** so nested schemas stay
readable and you rarely pass encoder ownership field-to-field by hand:

```text
  // Nested shape mirrors the schema — no .parent() hopscotch.
  enc.fixed(&fields)
      .bids(n, |bids| {
          bids.add(|level| {
              level.price(p).size(s);
              level.orders(m, |ords| {
                  ords.add(|o| { o.order_id(id); Ok(()) })?;
                  Ok(())
              })?;
              Ok(())
          })?;
          Ok(())
      })?
      .asks(0, |_| Ok(()))?
      .symbol(b"EURUSD")?;
```

Wire parity is exercised three ways: official Java `.sbe` fixtures, **live
dual-encode** suites that require ergo-sbe and sbe-tool Rust bytes to be
identical (`sbe_tool_wire_parity_test` for deep Car matrices;
`sbe_tool_multi_schema_wire_parity_test` across example/unit schemas with
checked-in sbe-tool reference crates under `sbe/tests/sbe_tool_reference/`),
and a maintained benchmark gate versus sbe-tool-generated codecs (see
[BENCHMARKS.md](sbe/benchmarks.md)).

> **Early release (0.x).** Experimental APIs may change. Binary wire
> compatibility is covered by an automated suite (golden bytes, schema edge
> cases, parity benches). Pin versions and report production use — real-world
> feedback is how the experimental banner goes away.

## Workspace

| Path | Package | Role |
|---|---|---|
| [`sbe/`](sbe/getting-started.md) | `ergo-sbe` | SBE schema parser and Rust codec generator |
| [`sbe/benchmarks/`](sbe/benchmarks.md) | `ergo-sbe-benchmarks` | Unpublished parity benchmarks |
| [`cluster/`](https://github.com/mimran1980/ergon/tree/main/cluster) | `ergo-aeron-cluster` | Experimental Aeron Cluster client |
| [`samples/`](https://github.com/mimran1980/ergon/tree/main/samples) | seven standalone crates | Unpublished integration playgrounds |

The workspace requires Rust 1.88 or newer. The sample crates are intentionally
excluded from the Cargo workspace and remain `publish = false`.

## Set up

The repository pins the upstream Aeron and SBE repositories as submodules:

```sh
git submodule update --init --recursive
```

Common local checks:

```sh
just policy
just check-products
just test
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
```

`just test` is intentionally not a partial/offline green path: it builds and
runs the Java Cluster lifecycle/recovery lane and the HA sample. Use
`just test-all` to add Miri and deterministic fuzz replay.

Pull-request CI also enforces the non-decreasing coverage baseline. Scheduled
lanes run every fuzz target for ten minutes, Miri fixtures weekly, and
critical-path mutation testing weekly. Missing or empty results fail closed.

See the [samples README](https://github.com/mimran1980/ergon/blob/main/samples/README.md) for standalone sample commands and
Java harness requirements. Run `just --list` for the repository's available
build, test, interoperability, and benchmark recipes.

## Project boundaries

- Official SBE wire compatibility takes priority over API convenience.
- Maintained hot paths are compared with the official SBE generator output.
- Benchmark claims are parity-checked and profile-specific. The corrected suite
  treats a repeatable sbe-tool win as a blocking benchmark/codegen defect and
  publishes results with LTO both enabled and disabled.
- Codec microbenchmarking is notoriously easy to get wrong. Benchmark results
  are explicitly reviewable evidence, not product claims; surprising ratios
  should be reported and treated as suspected benchmark defects first.
- Checked entry points must report malformed input rather than manufacture
  default, empty, or lossy values.
- The Cluster crate implements a client, not a consensus module, service
  container, archive, backup node, or Cluster administration tool.
- Samples, benchmarks, Java harness code, and upstream reference sources are
  repository support material, not publication targets.
