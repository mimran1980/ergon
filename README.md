# ergon

[![Crates.io](https://img.shields.io/crates/v/ergo-sbe)](https://crates.io/crates/ergo-sbe)
[![CI](https://github.com/mimran1980/ergon/actions/workflows/ci.yml/badge.svg)](https://github.com/mimran1980/ergon/actions/workflows/ci.yml)
[![API Docs](https://docs.rs/ergo-sbe/badge.svg)](https://docs.rs/ergo-sbe/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/)

Rust crates for low-latency trading: an SBE codec generator, an Aeron Cluster
client, busy-spin network I/O, host checks and a DPDK transport for the Aeron
media driver. 0.x alpha, not production-ready: any release may break the API,
with no deprecation period; [CHANGELOG.md](CHANGELOG.md) lists each break.
Exit criteria: [Road to 1.0](https://mimran1980.github.io/ergon/project/road-to-1.0.html).

> **AI assistance.** This repository was developed with heavy AI assistance.
> Process notes for `ergo-sbe`:
> [AI assistance disclosure](https://mimran1980.github.io/ergon/project/ai-assistance.html).

## Kubernetes: shared CPUs for latency pods

| Package | Get it |
|---|---|
| Helm chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1` |
| Image (amd64, arm64) | `docker pull ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` |

```sh
helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
  --version 0.1.1 -n ergon-system --create-namespace \
  --set-string config.sharedCpus=0 \
  --set hostContract.kubeletConfig.enabled=true \
  --set hostContract.sys.enabled=true \
  --set hostContract.proc.enabled=true
```

A Guaranteed pod gets its exclusive CPUs plus the node's shared housekeeping
CPUs, and every new thread starts on the shared ones, so the busy-spin loop
keeps its core. The `hostContract` flags let the plugin check a pod
labelled `ergon.io/latency: hot` against the node (isolated exclusive CPUs,
static CPU manager, NIC IRQs on the IRQ CPUs only). Run on kind on one Azure
Cobalt VM, with THP `never` and the fair server off on the isolated CPUs:
the hot thread with helpers on the shared CPU held p50 0.02 µs, p99.99
0.03 µs, max 4.81 to 5.52 µs, 0 involuntary switches a minute; 3.5 to
5.5 µs is the hypervisor's floor. Node events (systemd timers, pod starts,
logins) still reach isolated CPUs
([benchmarks](k8s/nri-shared-cpus/BENCHMARKS.md)). Not verified on EKS; a
helper's wake-up wait not measured.

- Node setup, install, configuration: [k8s/nri-shared-cpus](k8s/nri-shared-cpus/README.md).
- Node and pod tuning, each setting marked measured or not, with copy-paste
  blocks: [Kubernetes HFT tuning](https://mimran1980.github.io/ergon/deploy/hft.html).

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
