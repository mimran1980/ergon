# ergon

[![Crates.io](https://img.shields.io/crates/v/ergo-sbe)](https://crates.io/crates/ergo-sbe)
[![CI](https://github.com/mimran1980/ergon/actions/workflows/ci.yml/badge.svg)](https://github.com/mimran1980/ergon/actions/workflows/ci.yml)
[![API Docs](https://docs.rs/ergo-sbe/badge.svg)](https://docs.rs/ergo-sbe/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/)

> **AI assistance.** This monorepo was developed **with heavy AI assistance**.
> Humans directed the problems, approved designs, and verified results. Process
> notes: [AI-ASSISTANCE.md](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md).

## What to use

One line per module: what it is for, and the call you make.

| module | what it does | how you use it |
|---|---|---|
| [`ergo-sbe`](sbe/) | schema XML → Rust SBE codecs | in `build.rs`: `ergo_sbe::generate_to_out_dir(...)`, then `ergo_sbe::sbe_mod!(name)` |
| [`ergon-clock`](clock/) | UTC nanoseconds from the CPU counter | `calibrate()` once at start-up, then `now_ns()` |
| [`ergon-tune`](tune/) | reads the host and names what will wreck your tail latency, and the fix | call at start-up; it changes nothing |
| [`ergon-io`](io/) | WebSocket and HTTP/TLS on one busy-spinning thread | `poll()` from your loop; epoll or `io_uring` is configuration |
| [`ergo-aeron-cluster`](cluster/) | Aeron Cluster client | only if you need cluster; experimental, not a production substitute |
| [`ergon-md-dpdk`](md-dpdk/) | DPDK transport for the Aeron media driver | Linux only; proof of concept — use [Aeron Premium](https://aeron.io/) for kernel bypass |
| `md-dpdk/harness`, `md-dpdk/device-plugin`, `sbe/benchmarks`, `clock/bench`, `cluster-test-harness` | benchmarks and test support | you do not call these from an app |

Kubernetes, if you deploy there:

**[Kubernetes HFT tuning](https://mimran1980.github.io/ergon/deploy/hft.html)** —
every setting that won an A/B, its measured effect, and the two copy-paste
blocks. Start there.

| module | what it does | how you use it |
|---|---|---|
| [`k8s/nri-shared-cpus`](k8s/nri-shared-cpus/) | gives a pod its exclusive CPUs **and** the node's shared housekeeping CPUs, and starts every thread on the shared ones so the busy-spin loop keeps its core | deploy its Helm chart on a containerd node with NRI on; ask for `ergon.io/shared-millicpus` |
| [`k8s/nri-latency-pods`](k8s/nri-latency-pods/) | the latency-pod setup: node tuning, an admission policy that refuses exec probes and always-restart sidecars, and the pod annotations for hugepages, CPU isolation, io_uring and PHC devices | apply the node and admission halves on **EKS AL2023** only. A pod author sets annotations and labels, nothing more. Not a crate — nothing to import |

Start with `ergo-sbe` + `ergon-clock` + `ergon-tune`. Add `ergon-io` if you talk
to an exchange. The rest is optional.

Not production-ready: APIs may change. Verify wire compatibility, failure
handling and performance for your own schemas. Criteria for lifting that:
**[Road to 1.0](https://mimran1980.github.io/ergon/project/road-to-1.0.html)**.

## Workspace

| Path | Package | Role |
|---|---|---|
| [`sbe/`](sbe/) | `ergo-sbe` | SBE schema parser and Rust codec generator |
| [`sbe/benchmarks/`](sbe/benchmarks/) | `ergo-sbe-benchmarks` | Unpublished parity benchmarks |
| [`cluster/`](cluster/) | `ergo-aeron-cluster` | Experimental Aeron Cluster client |
| [`io/`](io/) | `ergon-io` | Busy-spin network I/O: switchable epoll / `io_uring` backends, in-place TLS 1.3, WebSocket, HTTP/1.1 with request signing, optional libcurl fallback |
| [`io/benches/`](io/benches/) | `ergon-io-bench` | Standalone latency benchmark for the `ergon-io` backends; write-up in `io/benches/BENCHMARK.md` |
| [`clock/`](clock/) | `ergon-clock` | Hardware-counter clock for low-latency threads: `rdtsc` or `CNTVCT_EL0` scaled to UTC nanoseconds by one multiply, calibrated once at start-up, kept on `CLOCK_REALTIME` by one cheap update a second (rate-capped, never back); unordered and ordered reads; warns at start-up when it falls back to `CLOCK_MONOTONIC` or the host's clock is not on UTC |
| [`clock/bench/`](clock/bench/) | `ergon-clock-bench` | Unpublished benchmarks of `ergon-clock` against quanta, minstant, std and the kernel's clocks (`clockbench`, Criterion); results in `clock/BENCHMARKS.md` |
| [`tune/`](tune/) | `ergon-tune` | Start-up and housekeeping checks of a host's low-latency settings (CPU isolation, NIC interrupts, THP, the Aeron directory, NUMA, scheduling policy, SMT siblings; with `nic`, ENA moderation, NAPI deferral, the Azure VF and the RX ring): pure functions returning findings, read through a root path so they run against fixture trees |
| [`md-dpdk/`](md-dpdk/) | `ergon-md-dpdk` | DPDK ENA kernel-bypass transport for the media driver (Linux x86_64) |
| [`md-dpdk/harness/`](md-dpdk/harness/), [`md-dpdk/device-plugin/`](md-dpdk/device-plugin/) | unpublished | DPDK interop harness and kubelet device plugin |
| [`samples/`](samples/) | seven teaching crates | Unpublished integration playgrounds |

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

Pull-request CI also enforces the non-decreasing coverage baseline and runs the
LE/BE/nested fixture crate as ordinary tests. A weekly schedule (Sundays) runs
every fuzz target for ten minutes and the same fixtures under Miri. Mutation
testing is manual (`just check-mutation`); CI enforces only its configuration.
Missing or empty results fail closed. 32-bit x86 and big-endian s390x codec
tests are not run in CI.

See [`samples/README.md`](samples/README.md) for standalone sample commands and
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

## Documentation

- **[ergo-sbe book](https://mimran1980.github.io/ergon/)** — guides for ergo-sbe and the cluster client (feature tour, recipes, design notes)
- [docs.rs/ergo-sbe](https://docs.rs/ergo-sbe/) — API reference for the generator
- [docs.rs/ergo-aeron-cluster](https://docs.rs/ergo-aeron-cluster/) — API reference for the cluster client
- [`sbe/README.md`](sbe/README.md) — generation and generated-code workflow
- [`cluster/README.md`](cluster/README.md) — Cluster client scope and usage
- [`md-dpdk/README.md`](md-dpdk/README.md) — DPDK transport, its requirements and deployment
- [`samples/README.md`](samples/README.md) — sample inventory and commands
- [`sbe/BENCHMARKS.md`](sbe/BENCHMARKS.md) — benchmark method and gates
- [`AI-ASSISTANCE.md`](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md) — AI-assisted development process
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — contribution and verification rules
- [`CHANGELOG.md`](CHANGELOG.md) — user-facing changes

Git history is the archive for completed plans and superseded design notes.

## License

Apache-2.0.
