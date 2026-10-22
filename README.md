# ergon

[![Crates.io](https://img.shields.io/crates/v/ergo-sbe)](https://crates.io/crates/ergo-sbe)
[![CI](https://github.com/mimran1980/ergon/actions/workflows/ci.yml/badge.svg)](https://github.com/mimran1980/ergon/actions/workflows/ci.yml)
[![API Docs](https://docs.rs/ergo-sbe/badge.svg)](https://docs.rs/ergo-sbe/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/)

Rust crates for low-latency trading: an SBE codec generator, an Aeron Cluster
client, busy-spin network I/O, host checks and a DPDK transport for the Aeron
media driver. 0.x and not production-ready: APIs may change. Exit criteria:
[Road to 1.0](https://mimran1980.github.io/ergon/project/road-to-1.0.html).

> **AI assistance.** This repository was developed with heavy AI assistance.
> Process notes for `ergo-sbe`:
> [AI assistance disclosure](https://mimran1980.github.io/ergon/project/ai-assistance.html).

**Tuning Kubernetes for HFT?
[Kubernetes HFT tuning](https://mimran1980.github.io/ergon/deploy/hft.html)
— start there.** It lists each setting, marks which ones were measured, and
has the copy-paste blocks.

The [NRI plugin README](k8s/nri-shared-cpus/README.md) covers installation and configuration for Kubernetes shared CPUs.

## Crates

| Path | Crate | What it is |
|---|---|---|
| [`sbe/`](sbe/) | `ergo-sbe` | SBE schema XML → Rust codecs, generated in `build.rs` |
| [`cluster/`](cluster/) | `ergo-aeron-cluster` | Aeron Cluster client. Experimental, not a production substitute |
| [`io/`](io/) | `ergon-io` | WebSocket and HTTP/1.1 over TLS 1.3 on one busy-spin thread; epoll or `io_uring` |
| [`tune/`](tune/) | `ergon-tune` | Reads the host's low-latency settings and reports findings; changes nothing |
| [`md-dpdk/`](md-dpdk/) | `ergon-md-dpdk` | DPDK transport for the Aeron media driver, Linux x86_64 and aarch64. Proof of concept |
| [`k8s/nri-shared-cpus`](k8s/nri-shared-cpus/README.md) | Helm chart | Gives a pod its exclusive CPUs plus the node's shared CPUs. |
| [`k8s/nri-latency-pods`](k8s/nri-latency-pods/) | node and admission setup | Node tuning, an admission policy and pod annotations for latency pods (EKS AL2023) |

Unpublished support: `sbe/benchmarks`, `cluster-test-harness`,
`io/benches`, `md-dpdk/harness`, `md-dpdk/device-plugin` and seven sample
crates in [`samples/`](samples/README.md).

Start with `ergo-sbe`. Add another crate only for a need you have. Which one
does what, and how to call it: the
[book introduction](https://mimran1980.github.io/ergon/).

## Set up

Rust 1.99 or newer. The Aeron and SBE upstreams are submodules:

```sh
git submodule update --init --recursive
just policy
just check-products
just test        # full suite; builds the Aeron Java jars
```

Every check and what CI runs: [CONTRIBUTING.md](CONTRIBUTING.md).

## Project rules

- Official SBE wire compatibility takes priority over API convenience.
- Maintained hot paths are compared with sbe-tool output on equal work
  ([method](sbe/BENCHMARKS.md)). Treat a surprising ratio as a benchmark
  defect first.
- Checked entry points report malformed input rather than return default,
  empty or lossy values.
- The cluster crate is a client only: no consensus module, service container,
  archive or admin tool.

## Documentation

- [Book](https://mimran1980.github.io/ergon/): guides for every crate and
  deployment
- API: [ergo-sbe](https://docs.rs/ergo-sbe/),
  [ergo-aeron-cluster](https://docs.rs/ergo-aeron-cluster/)
- [CHANGELOG.md](CHANGELOG.md), [SECURITY.md](SECURITY.md)

Apache-2.0.
