# Introduction

## What to use

One line per module: what it is for, and the call you make.

| module | what it does | how you use it |
|---|---|---|
| `ergo-sbe` | schema XML → Rust SBE codecs | `ergo_sbe::generate_to_out_dir(...)` in `build.rs`; then plain `include!`, or `ergo_sbe::sbe_mod!(name)` in your crate, which also needs `ergo-sbe` in `[dependencies]` ([chapter](sbe/getting-started/depend.md)) |
| `ergon-tune` | reads the host and names what will wreck your tail latency, and the fix | call at start-up; it changes nothing ([chapter](tune/overview.md)) |
| `ergon-io` | WebSocket and HTTP/TLS on one busy-spinning thread | `poll()` from your loop; epoll or `io_uring` is configuration ([chapter](io/overview.md)) |
| `ergo-aeron-cluster` | Aeron Cluster client | only if you need cluster; experimental, not a production substitute ([chapter](cluster/overview.md)) |
| `ergon-md-dpdk` | DPDK transport for the Aeron media driver | Linux x86_64 and aarch64; proof of concept — use [Aeron Premium](https://aeron.io/) for kernel bypass ([chapter](md-dpdk/overview.md)) |

Kubernetes, if you deploy there. Tuning a node and pod for HFT:
[Kubernetes HFT tuning](deploy/hft.md) — start there.

| module | what it does | how you use it |
|---|---|---|
| `k8s/nri-shared-cpus` | gives a pod its exclusive CPUs **and** the node's shared housekeeping CPUs, and starts every thread on the shared ones so the busy-spin loop keeps its core | chart `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1`, installed with the flags in the [chapter](deploy/shared-cpus.md#use-the-public-chart). Image `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1`. Prepare the node first. Sample: `k8s/nri-shared-cpus/examples/try` ([chapter](deploy/shared-cpus.md)) |
| `k8s/nri-latency-pods` | node tuning and the hot-pod admission policy (no exec probes, no always-restart sidecars). The grants live in the shared-cpus image. This directory is the YAML and the node units | apply `admission/priority-class.yaml` and `admission/validating-admission-policy.yaml` after the chart. Pod authors set annotations and the `ergon.io/latency: hot` label ([chapter](deploy/latency-pods.md)) |

Start with `ergo-sbe`. Add another crate only for a need you have:
`ergon-io` for exchange I/O, `ergon-tune` to check a latency host's settings.
Which clock to read for timestamps and durations: [Clocks](deploy/clocks.md).

`ergo-sbe` generates Rust codecs with no heap allocation on the encode and
decode hot paths ([allocation_count_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/allocation_count_test.rs)) from
[Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/) schemas.
Wire layout matches official SBE for the
[published profile](https://github.com/mimran1980/ergon/blob/main/docs/SBE_COMPATIBILITY.md).
Tail order is a **type error**, not a runtime surprise.

## Quick start

```toml
[build-dependencies]
ergo-sbe = "0.1"

[dependencies]
ergo-sbe = "0.1"   # only for sbe_mod! / include_sbe!
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

0.x alpha: any release may break the API, with no deprecation period. Pin
exact versions and read the changelog before upgrading. Exit criteria:
[Road to 1.0](project/road-to-1.0.md).

It is **not** a line-for-line port of sbe-tool. The generated API is meant to be
easier under Rust's borrow checker, safer at untrusted boundaries, and still
official-SBE on the wire. Nested groups use **scoped closures** so you do not
hand encoder ownership back with `.parent()`:

```rust,no_run
{{#include ../../samples/sbe-feature-tour/src/lib.rs:encode_sample_car}}
```
*(Compiled and run in CI.)*

Wire parity is tested by the dual-encode suite (`sbe_tool_wire_parity_test`,
`sbe_tool_multi_schema_wire_parity_test`). The benchmarks also assert byte
parity before timing: [Benchmarks](sbe/benchmarks.md).

## Deployment

How to set up hosts and Kubernetes for these crates on AWS and Azure, x86 and
Graviton: [Deployment](deploy/overview.md).

Rust 1.99+. Clone setup and checks:
[repository README](https://github.com/mimran1980/ergon/blob/main/README.md).
