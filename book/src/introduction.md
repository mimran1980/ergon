# Introduction

## What to use

One line per module: what it is for, and the call you make.

| module | what it does | how you use it |
|---|---|---|
| `ergo-sbe` | schema XML → Rust SBE codecs | in `build.rs`: `ergo_sbe::generate_to_out_dir(...)`, then `ergo_sbe::sbe_mod!(name)` ([chapter](sbe/getting-started/depend.md)) |
| `ergon-clock` | UTC nanoseconds from the CPU counter | `calibrate()` once at start-up, then `now_ns()` ([chapter](clock/overview.md)) |
| `ergon-tune` | reads the host and names what will wreck your tail latency, and the fix | call at start-up; it changes nothing ([chapter](tune/overview.md)) |
| `ergon-io` | WebSocket and HTTP/TLS on one busy-spinning thread | `poll()` from your loop; epoll or `io_uring` is configuration ([chapter](io/overview.md)) |
| `ergo-aeron-cluster` | Aeron Cluster client | only if you need cluster; experimental, not a production substitute ([chapter](cluster/overview.md)) |
| `ergon-md-dpdk` | DPDK transport for the Aeron media driver | Linux only; proof of concept — use [Aeron Premium](https://aeron.io/) for kernel bypass ([chapter](md-dpdk/overview.md)) |

Kubernetes, if you deploy there:

| module | what it does | how you use it |
|---|---|---|
| `k8s/nri-shared-cpus` | gives a pod its exclusive CPUs **and** the node's shared housekeeping CPUs, and starts every thread on the shared ones so the busy-spin loop keeps its core | deploy its Helm chart on a containerd node with NRI on; ask for `ergon.io/shared-millicpus` ([chapter](deploy/shared-cpus.md)) |
| `k8s/nri-latency-pods` | the latency-pod setup: node tuning, an admission policy that refuses exec probes and always-restart sidecars, and the pod annotations for hugepages, CPU isolation, io_uring and PHC devices | apply the node and admission halves on **EKS AL2023** only. A pod author sets annotations and labels, nothing more. Not a crate — nothing to import ([chapter](deploy/latency-pods.md)) |

Start with `ergo-sbe` + `ergon-clock` + `ergon-tune`. Add `ergon-io` if you talk
to an exchange. The rest is optional.

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
| `io/` | `ergon-io` | Busy-spin network I/O: epoll / `io_uring` backends, in-place TLS 1.3, WebSocket, HTTP ([chapter](io/overview.md)) |
| `clock/` | `ergon-clock` | Calibrated hardware-counter clock ([chapter](clock/overview.md)) |
| `tune/` | `ergon-tune` | Start-up and housekeeping checks of a host's low-latency settings ([chapter](tune/overview.md)) |
| `md-dpdk/` | `ergon-md-dpdk` | DPDK transport for the Aeron media driver: a proof of concept, not for production ([chapter](md-dpdk/overview.md)) |
| `samples/` | seven teaching crates | Unpublished playgrounds (`publish = false`) |

How to set up hosts and Kubernetes for these crates on AWS and Azure, x86 and
Graviton, with the measurements behind it: [Deployment](deploy/overview.md).

Rust 1.99+. Clone setup, `just` recipes, and project boundaries:
[repository README](https://github.com/mimran1980/ergon/blob/main/README.md).
