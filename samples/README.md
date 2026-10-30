# Samples

Standalone crates that exercise repository APIs. They are **excluded from the
workspace**, set `publish = false`, and are **not** production reference
implementations — they move with experimental APIs on purpose.

## Start here (product teaching path)

| Step | Sample | Why |
|------|--------|-----|
| **1** | [`sbe-feature-tour/`](sbe-feature-tour/) | **Golden path.** Full feature map: stages, EncodedLength, checked constructors + verify, Display, DTO with `DomainVarData::Strings`, all three conversion styles |
| **2a** | [`l3-book/`](l3-book/) | Nested/ragged books; **`with_domain_type` only**; **build-dep only** (`#[path]` into `src/generated/`) |
| **2b** | [`exchange-example/`](exchange-example/) | Multi-schema; **`with_conversion` only**; IPC + app `TryFromSbe` |
| **3** | [`sbe-codegen-examples/`](sbe-codegen-examples/) | Generator **as a library** (no `build.rs`) |
| Later | [`cluster-tutorial/`](cluster-tutorial/) | Connect, offer, poll, keep-alive, close |
| Later | [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Claim-based Cluster publishing + HA-shaped book |
| Later | [`cluster-rfq/`](cluster-rfq/) | RFQ / auction codecs over Cluster |

```sh
# 1 — always start here
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml

# 2 — pick the conversion style you want in product code
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
```

**Rule of thumb:** one conversion style per schema type
(`with_domain_type` *or* `with_conversion`, not both for the same selector).
See the book's
[with_conversion vs with_domain_type](https://mimran1980.github.io/ergon/sbe/configuration/conversion-vs-domain.html).

## `ergo-sbe`: build dependency vs application dependency

Generated codecs ship their own embedded `sbe_rt` module. Linking the **app**
does not require `ergo-sbe` unless you use its macros or call the generator
library at runtime.

| Pattern | `build-dependencies` | `dependencies` | Typical use |
|---------|----------------------|----------------|-------------|
| **Build only** (**samples default**) | `ergo-sbe` | — | `generate(..)?.write_to("src/generated")` → `src/generated/` (gitignored, unpublished) + `#[path = "generated/….rs"]` |
| **OUT_DIR only** | `ergo-sbe` | — | `generate(..)?.write_to_out_dir()` + `include!(concat!(env!("OUT_DIR"), …))` — the product shape without macros |
| **Build + runtime** (**product default**) | `ergo-sbe` | `ergo-sbe` | `generate(..)?.write_to_out_dir()` + `sbe_mod!` (expands to that `include!`) — what `cluster/build.rs` and the introduction do |
| **Runtime only** | — | `ergo-sbe` | Call `generate`, or `parse` + `Generator`, as a library (no `build.rs`) |

| Sample | Pattern | Purpose | External requirements |
|---|---|---|---|
| [`sbe-feature-tour/`](sbe-feature-tour/) | **Build only** | **Teaching / feature map** — EncodedLength, stages, DTO, AnyMessage, **all three** conversion styles | None |
| [`l3-book/`](l3-book/) | **Build only** | Nested/ragged L3 books; **`with_domain_type` only** | None for local tests |
| [`exchange-example/`](exchange-example/) | **Build only** | Multi-schema + **`with_conversion` only** + Aeron IPC | Network only for live exchange paths |
| [`cluster-rfq/`](cluster-rfq/) | **Build only** | RFQ / auction protocol codecs + cluster examples | Java harness for live examples |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | **Build only** (may still dep cluster) | Claim-based Cluster publishing + HA-shaped book | Java harness only for leader-kill coverage |
| [`sbe-codegen-examples/`](sbe-codegen-examples/) | **Runtime only** | Generator API as a library (no `build.rs`) | None |
| [`cluster-tutorial/`](cluster-tutorial/) | **Neither** (uses `ergo-aeron-cluster`) | Connect, offer, poll, keep-alive, close walkthrough | JDK 25 (17+) and built Aeron artifacts |
| [`low-latency-k8s/`](#low-latency-k8s) | **Neither** (rusteron-client, ergon-tune) | Kubernetes reference: DPDK media driver and Aeron ping/pong on exclusive CPUs, helpers and `tracing` on the shared CPU, a verifier that proves it | Linux; Docker for its tests and e2e |

### Seeing generated code (without committing it)

`include!(concat!(env!("OUT_DIR"), …))` and `sbe_mod!` put files under a
hashed path like `target/debug/build/<crate>-<hash>/out/….rs`, which is
awkward to open by hand.

Samples instead write to a **stable, local path**:

```text
samples/<name>/src/generated/*.rs   # created on cargo build, gitignored
```

1. `cargo build --manifest-path samples/sbe-feature-tour/Cargo.toml`
2. Open `samples/sbe-feature-tour/src/generated/feature_tour.rs`
3. From app code, **Go to definition** on `CarEncoder` / etc. should land there

Root `.gitignore` has `**/src/generated/`. Do **not** commit those trees
(Binance alone is multi‑MB). Rebuild after a clean clone.

```rust
// build.rs
let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
ergo_sbe::generate(ergo_sbe::SchemaSource::file("schemas/messages.xml"), config)?
    .write_to(&out)?;

// src/lib.rs — real path → IDE go-to-definition works
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all, warnings)]
#[path = "generated/messages.rs"]
mod messages;
```

## Buffer sizing (samples & tests)

- **Const-sized messages:** stack `[0u8; MsgEncoder::compute_length_with_header()]`
- **Dynamic / ragged:** size with `*EncodedLength` / `compute_length_with_header(…)`,
  then encode into a claim/slot of that exact length — avoid oversize
  `vec![0u8; 4096]` “guess” buffers

See the book's
[Buffer Sizing](https://mimran1980.github.io/ergon/sbe/core-concepts/buffer-sizing.html)
and feature-tour `demo_car_size_and_encode`.

## Domain DTOs & var-data

```rust
.with_domain_objects(DomainVarData::Strings) // String; bad UTF-8 → InvalidUtf8 error
.with_domain_objects(DomainVarData::Bytes)   // Vec<u8>; byte-exact
```

`Strings` is strict UTF-8 (no empty-string fallback). Feature-tour uses
`Strings`; l3-book uses `Bytes` where tails are byte-oriented.

## Check each sample

```sh
cargo check --manifest-path samples/sbe-feature-tour/Cargo.toml --all-targets
cargo check --manifest-path samples/exchange-example/Cargo.toml --all-targets
cargo check --manifest-path samples/l3-book/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-ha-orderbook/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-rfq/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-tutorial/Cargo.toml --all-targets
```

Useful service-free tests:

```sh
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline
```

Java-backed samples require:

```sh
just build-aeron-jars
cargo run --manifest-path samples/cluster-tutorial/Cargo.toml
```

## Conversion: which sample uses what

| Sample | Config | Decode / encode surface |
|--------|--------|-------------------------|
| [`l3-book/`](l3-book/) | **`with_domain_type` only** | `dec.price()?` → `Decimal`; `enc.price(d)?` |
| [`exchange-example/`](exchange-example/) | **`with_conversion` only** | `dec.price_as::<T>()?`; `enc.price_from(&t)?` (+ app `TryFromSbe`) |
| [`sbe-feature-tour/`](sbe-feature-tour/) | **All three** (different selectors) | bool/timestamp concrete (`Generated`); Decimal generic (`demo_conversion_only`); ManualDecimal concrete + app impl (`demo_domain_type_manual_impl`) |

Rule: **one style per selector**. `with_domain_type` already enables conversion;
do not stack `with_conversion` on the same selector.

```rust
// A — pluggable
.with_conversion(ConversionSelector::named_type("Decimal"))
// B — concrete (implies conversion)
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
```


<a id="low-latency-k8s"></a>
## low-latency-k8s: the Kubernetes reference

[`low-latency-k8s/`](low-latency-k8s/) puts the whole low-latency stack on
Kubernetes:

- the `ergon-shared-cpus` NRI plugin gives each Guaranteed pod exclusive CPUs
  plus one shared CPU;
- the DPDK media driver (`ergon-md-dpdk`, Aeron `DEDICATED`) runs its sender
  and receiver on two exclusive CPUs and everything else on the shared CPU;
- an Aeron ping and pong (`rusteron-client`) each busy-spin on one exclusive
  CPU, with 100 shared millicpus for the client conductor, the logger and the
  stats thread;
- `ll-probe` proves where every thread ran.

md-dpdk is a lab proof of concept, and its BENCHMARKS recommend kernel UDP
in production (`md-dpdk/BENCHMARKS.md`, the opening note and
[Recommended configuration](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#recommended-configuration)).
Every DPDK setting below is lab only.

<a id="aws-k8s"></a>
### AWS Kubernetes

The book's lowest 1-stream path, when DPDK is allowed, is Aeron on DPDK.
On EKS that layout is `eks-ena-x86` (`c7i.4xlarge`) or `eks-ena-arm64`
(`c9g.2xlarge`). The book and the READMEs link here. Both overlays render
and lint only: they have not been run on EKS, so the latency figures stay
the VM sessions.

```sh
kubectl apply -k samples/low-latency-k8s/k8s/overlays/eks-ena-x86
# eks-ena-arm64 is the Graviton overlay
```

Fill `k8s/overlays/eks-ena-<arch>/endpoints.env` and replace `REGISTRY/`
before applying. The quick start below is the node, the plugin and the
images.

| measured need | where to run it |
|---|---|
| 1 stream between hosts, lowest latency, DPDK allowed (AWS `c7a`: p99 35.4, 4 busy threads) | `eks-ena-x86` or `eks-ena-arm64`, this section |
| kernel UDP in production | no overlay. [BENCHMARKS](../md-dpdk/BENCHMARKS.md#recommended-configuration) recommend kernel UDP; those runs were VMs |
| 32 or more streams, up to 50,000 msg/s | no overlay. On Azure, mio or ergon-io kernel UDP, 1 busy thread, p50 3.7 to 10.5 µs below Aeron |
| WebSocket, ergon-io kernel TCP, smoltcp TCP, ergon-io on DPDK | no overlay. The remeasure is one rep, and its tails are loss, faults and unexplained maxima |

The overlays are the Aeron-on-DPDK lab layout. On AWS `c7a`, 1 stream,
50,000 msg/s, same session, that path's p99 is 35.4 (max 169 to 178),
against kernel Aeron p99 48.6, mio 60.8 (60.6 to 61.1) and ergon-io
60.9 (58.4 to 61.8)
(`book/src/transports/comparison.md`, unreliable UDP). It uses 4 busy
threads a host. `eks-ena-arm64` and `eks-ena-x86` render and lint only:
they have not been run on EKS, so that table is not an EKS measurement.

Settings already in the manifests, and the evidence for each:

| setting | where | evidence |
|---|---|---|
| `AERON_DIR=/mnt/huge-aeron/aeron`, `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false`, driver `hugepages-2Mi: 2Gi` | `k8s/base/driver.env`, `k8s/base/app.env`, the driver DaemonSet. A pod that opts in with `hugepages.ergon.io/<container>: "2M"` gets the page size and `SPARSE_FILE=false` from the plugin; it still sets `AERON_DIR` | required for Aeron to start. The IPC 2 MiB directory against tmpfs was not separated at any load ([hugepages](../book/src/transports/comparison.md#hugepages)). ergon-tune T23 warns when a hugetlbfs directory is missing either property |
| `transparent_hugepage=never`, `hugepagesz=2M hugepages=2048` | the node prep row below | a tail result: Azure idle p99.99 67 to 53 µs and idle max 546 to 106 µs; one AWS run overlaps; a traced `khugepaged` drain was a 16.25 µs max. No p50 change is published |
| `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true` | `k8s/base/app.env` | no latency figure is published |
| `RUSTERON_DPDK_HUGE_DIR=/dev/hugepages` | `k8s/base/driver.env` | DPDK packet buffers, about 37 MiB a NIC. `RUSTERON_DPDK_EAL_IN_MEMORY` has no published latency figure |
| `AERON_RECEIVER_IO_VECTOR_CAPACITY=16` | `k8s/base/driver.env` | one AWS DPDK run: no NAK against 16,247, max 142 against 1,130 µs |
| dedicated driver threads, exclusive CPUs | this sample's pod layout | once there are many streams, every thread taken from the driver costs latency (`book/src/transports/comparison.md`) |

`vdev-local` strips hugepages (`--no-huge`). It is the kind layout, not
the measured one.

At 32 and 128 streams on Azure, mio and ergon-io kernel UDP are 3.7 to
10.5 µs lower at p50 than Aeron, on 1 busy thread. That is set 2a, not a
deployment on EKS, so there is no mio overlay here. The production line
in BENCHMARKS stays kernel UDP.

No overlay for WebSocket, ergon-io kernel TCP, smoltcp TCP, or
ergon-io on DPDK (`fstack-spin`, `smoltcp-spin`). The remeasure is in
(`aws-20261029-174643` on `c7i.4xlarge`, `aws-20261029-190425` on
`c8g.2xlarge`): one rep, no A/A arm, and the tails are loss, page
faults and unexplained maxima. F-Stack TCP in the `c7a` table is an
md-dpdk bench arm, not this sample's driver.

The ENA device plugin is what fills PCI, address and gateway for the
Aeron driver: the `ena-driver` component requests
`rusteron.io/dpdk-ena-pair`, which injects
`RUSTERON_DPDK_SENDER_PCI`, `RUSTERON_DPDK_SENDER_IPV4_CIDR` and
`RUSTERON_DPDK_SENDER_GATEWAY` into the driver container. The minimum
ergon-io lines built from that pair are:

```text
fstack-spin,fstack_addr=10.0.1.20,fstack_gateway=10.0.1.1,fstack_pci=0000:00:06.0
port=0000:00:06.0,ip=10.0.1.20/24,eal=-a 0000:00:06.0
```

A container granted that pair needs neither line: ergon-io reads the
three variables itself when its line names no NIC, so `fstack-spin` (or
an empty smoltcp stack line) is enough. The NRI plugin's
`dpdk.ergon.io/<container>: "ena-pair"` opt-in (default off,
`../k8s/nri-latency-pods/README.md`) still exports the lines as
`ERGON_IO_FSTACK` and `ERGON_IO_SMOLTCP` for apps that log or pass them
on. These overlays are still render-and-lint only.

### What has run, and where

| where | what ran | proves |
|---|---|---|
| macOS (this repo's host) | unit, property, logger, manifest tests; entrypoint and `verify.sh` fixtures (bash 5 and macOS's bash 3.2); `node/eks-userdata.sh` output; `node/quiet-cpus.sh` on a fake `/sys`; bench smoke | code, renders, schemas, the verifier's pod discovery |
| Docker linux/arm64 and linux/amd64 (`scripts/test-docker.sh`) | the same Rust tests, plus the placement tests against an embedded driver and `hot_stdout` | the worker and the Aeron conductor stay on their creator's CPU, the loops allocate nothing but one allocation per warning line, a pinned thread never sleeps on a wedged stdout |
| Docker linux/arm64, Apple silicon (`scripts/e2e-vdev.sh --docker`, image `baseline`) | the driver on two DPDK taps, ping and pong, `ll-probe` for 30 s | the thread layout over a real DPDK data path. Not isolation: the Docker VM has no `isolcpus`. The script plays the plugin's part (sets `ERGON_*`, pins each process's first thread) |
| Docker linux/amd64 emulated on Apple silicon (`scripts/e2e-inside.sh` in the amd64 half of `baseline`) | the same e2e: `RESULT PASS checks=54` (`tests/fixtures/e2e-docker-baseline-amd64-emulated.txt`) | the amd64 image builds (x86-64-v2, DPDK, Aeron) and the layout holds; its latency is the emulator's |
| kind (`e2e-vdev.sh --kind`, overlay `vdev-local`) | written, **not run** (needs kind, 8 Docker CPUs, containerd ≥ 2.2 in the node image) | the plugin's grant |
| a Linux VM with `isolcpus` (overlay `vdev-isolated`, `node/linux-vm-prep.sh`) | **not run** (Azure Cobalt `D8ps_v6` and `F8as_v6` later) | the `full` contract |
| EKS ENA (overlays `eks-ena-arm64`, `eks-ena-x86`) | render and lint only | arm64: blocked until AL2023 has containerd ≥ 2.2 (`node/eks-nodeconfig.yaml`); x86: c7i.4xlarge, which needs the user's approval to run |

### Quick start

1. **Prepare the node.** Linux ≥ 6.2 with cgroup v2, Kubernetes ≥ 1.33,
   containerd ≥ 2.2, 8 CPUs, SMT off, 4 KiB pages.

   | setting | value | source |
   |---|---|---|
   | boot line | `isolcpus=nohz,domain,managed_irq,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0` | `md-dpdk/deploy/aws-eks/node/provision-node.sh`; measured by `book/src/deploy/hft.md:57-87` (ergon-io's loop gaps, not the md-dpdk runs) |
   | IOMMU | x86 `iommu=pt`, arm64 `iommu.passthrough=1` | `provision-node.sh` |
   | hugepages, THP | `hugepagesz=2M hugepages=2048`, `transparent_hugepage=never`; a 1 GiB hugetlbfs at `/mnt/huge-aeron` | `provision-node.sh`; `md-dpdk/deploy/aws-eks/README.md` |
   | IRQs | irqbalance off, every IRQ on CPU 0 | `hft.md` |
   | kubelet | `cpuManagerPolicy: static`, `full-pcpus-only`, `reservedSystemCPUs: "0-1"`, topology `single-numa-node` per pod, CFS quota left on | `node/kind-ll.yaml`, `node/eks-nodeconfig.yaml` |
   | containerd | NRI on, `default_validator` on | `k8s/nri-shared-cpus/README.md` |
   | sysctl | `kernel.numa_balancing=0`, `rmem_max`/`wmem_max` 16 MiB | `hft.md` |
   | at every boot | THP `never` and `/sys/kernel/debug/sched/fair_server/cpu<N>/runtime` 0 on each isolated CPU (kernel 6.12+; debugfs, so not persistent; only where nothing real-time runs there): `node/quiet-cpus.sh`, run by `node/ergon-quiet-cpus.service` | `k8s/nri-shared-cpus/BENCHMARKS.md` (run 20261024234841) |

   On a plain VM, `sudo node/linux-vm-prep.sh` sets these; after the reboot,
   `node/linux-vm-prep.sh --check` prints PASS/FAIL per row.

2. **Label the node.**

   ```sh
   kubectl label node <node> ergon.io/shared-cpus=true ergon.io/ll-driver=true ergon.io/ll-app=true
   ```

   On EKS the nodes come up labelled: `node/eks-userdata.sh <cluster>
   <endpoint> <ca-b64> <service-cidr> ping` (and `pong` for the second node)
   prints the user data. It runs `md-dpdk/deploy/aws-eks/node/userdata.sh`
   with reserved CPUs `0-1` and IRQ CPU `0`, so the kernel never isolates
   the shared CPU 1, swaps in `node/eks-nodeconfig.yaml` (NRI on, the
   `ergon.io` labels, `ergon.io/ll-role`), and installs and enables
   `ergon-quiet-cpus.service` before the node's reboot.

3. **Install the plugin.** CPU 0 takes the IRQs, CPU 1 is shared, CPUs 2-7
   are exclusive.

   ```sh
   # kubeadm, kind, a plain VM: values-kind.yaml; EKS AL2023: values-eks.yaml
   helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus --version 0.1.2 \
     -n ergon-system --create-namespace -f samples/low-latency-k8s/helm/values-kind.yaml
   kubectl apply -f k8s/nri-latency-pods/admission/priority-class.yaml \
                 -f k8s/nri-latency-pods/admission/validating-admission-policy.yaml
   ```

4. **On EKS only, the DPDK device plugin.** It comes with the `eks-ena-*`
   overlays (component `dpdk-ena-device-plugin` pulls in
   `md-dpdk/deploy/aws-eks` without its own driver). Fill in
   `k8s/overlays/eks-ena-<arch>/endpoints.env` with each node's receiver ENA
   IP first.

5. **Build the images and put them where the nodes pull.**

   ```sh
   # kind or a VM with the image in the local Docker: baseline holds linux/amd64 and linux/arm64
   samples/low-latency-k8s/docker/build.sh baseline
   kind load docker-image ll:baseline --name <cluster>   # kind only
   # EKS: push the CPU tag and md-dpdk's device plugin, then point the overlay at the registry
   samples/low-latency-k8s/docker/build.sh --push $REG arm64-v3          # amd64-v3 for eks-ena-x86
   docker buildx build --platform linux/arm64 --push \
     -f md-dpdk/deploy/aws-eks/Dockerfile.device-plugin -t $REG/rusteron-dpdk-device-plugin:0.1.34-arm64 .
   sed -i.orig "s|REGISTRY/|$REG/|" samples/low-latency-k8s/k8s/overlays/eks-ena-arm64/kustomization.yaml
   ```

6. **Deploy an overlay.**

   ```sh
   kubectl apply -k samples/low-latency-k8s/k8s/overlays/vdev-local   # vdev-isolated, eks-ena-arm64, eks-ena-x86
   ```

   | overlay | driver | hot pods | image |
   |---|---|---|---|
   | `vdev-local` | two DPDK taps, `--no-huge` | no (kind has no `isolcpus`) | `ll:baseline` |
   | `vdev-isolated` | two DPDK taps, hugepages | yes | `ll:baseline`; `e2e-vdev.sh --tag arm64-n2` (Cobalt) or `--tag amd64-v3` (F8as_v6) sets a CPU tag and its `kubernetes.io/arch` |
   | `eks-ena-arm64` | ENA pair | yes | `$REG/ll:arm64-v3`, c9g.2xlarge, `kubernetes.io/arch: arm64` |
   | `eks-ena-x86` | ENA pair | yes | `$REG/ll:amd64-v3`, c7i.4xlarge, `kubernetes.io/arch: amd64` |

   The ping runs for `LL_DURATION_S=90` (`k8s/base/app.env`), so it is
   still measuring through the verify window.

7. **Verify.**

   ```sh
   samples/low-latency-k8s/scripts/verify.sh --contract placement --seconds 30   # full on an isolcpus node
   kubectl -n trading logs job/ll-ping | grep '^ll-latency'
   ```

   `verify.sh` probes every node that runs a ping or a pong, with that
   node's driver (it fails when a node lacks its driver), using the image
   the `ll-ping` pod runs. Or run all of it in Docker, with no cluster:
   `just samples-ll-e2e`.

### Pinning a thread

The plugin starts every thread of the container on the shared CPU and sets:

| variable | holds |
|---|---|
| `ERGON_ISOLATED_CPUS` | the pod's exclusive CPUs, sorted |
| `ERGON_SHARED_CPUS` | this container's shared CPUs |
| `ERGON_HELPER_CPUS` | the same as `ERGON_SHARED_CPUS`; helpers read this first |

For a C binary, use `ergon-cpus` (from the plugin's image), with specs
`isolated:<i>` and `shared:<i>`:

```yaml
command: [ergon-cpus, --env, AERON_SENDER_CPU_AFFINITY=isolated:0,
          --env, AERON_RECEIVER_CPU_AFFINITY=isolated:1,
          --env, RUSTERON_DPDK_HOUSEKEEPING_CPU=shared:0, --, driver-entrypoint.sh, media_driver]
```

In Rust, the hot thread pins itself, and only after it has created every
helper (a new thread inherits its creator's mask):

```rust
use rustix::thread::{CpuSet, sched_setaffinity};
let list = std::env::var("ERGON_ISOLATED_CPUS")?;           // "3"
let cpu: usize = list.split(',').next().ok_or("empty")?.trim().parse()?;
let mut set = CpuSet::new();
set.set(cpu);
sched_setaffinity(None, &set)?;                                // this thread only
assert_eq!(rustix::thread::sched_getcpu(), cpu);
```

The sample does this in `src/cpus.rs` (`pin_and_check`). `ll-hot` starts
Aeron, so that `aeron-client` is created on the shared CPU, connects, and
only then pins. `tests/placement_aeron.rs` checks the result: `aeron-client`
keeps its creator's CPU and `ll-hot` ends on its own.

### Logging

The sample uses the standard `tracing` crates: `tracing-subscriber`'s `fmt`
writes to stdout (so `kubectl logs` works) through
`tracing-appender::non_blocking` (lossy, 4096 lines). The `WorkerGuard` lives
until the report. `ll-log` is spawned from `main` while `main` is on the
shared CPU, and is re-pinned there if its mask is wider (`src/logging.rs`).

| on the calling thread | on `ll-log` |
|---|---|
| level check; formatting into a reused thread-local `String` (timestamp, fields) | `write(2)` to stdout |
| a clone and drop of the writer (atomics on shared cache lines) | |
| **1 allocation** per event (the line copied into a `Vec`, dropped or not); 2 on a thread's first event; 0 for a disabled level (`tests/logger_alloc.rs`) | |
| `try_send`; a full queue drops the line and counts it (`dropped_lines`); a futex wake when `ll-log` sleeps | a failed write (EPIPE, EIO) loses its line: tracing-appender 0.2.5 does not count it, the sample's `CountingWriter` does (`write_errors`) |

A wedged stdout never stalls the caller. `tests/hot_stdout.rs` (Linux) pins
`ll-hot` to one CPU and `ll-log` to another, swaps the real fd 1 for a pipe
nobody reads, and asserts: `ll-log` asleep in `write(2)`, no voluntary
context switch on `ll-hot` over 20,000 events, every event dropped and
counted. It prints the cost of one event p50 to max (quanta's counter) and
what explains the max: the window's page faults, context switches, steal,
interrupts and softirqs on the CPU, and the CPU's own longest stall with no
event at all (`src/costs.rs`); with nothing that explains it, it says
"unexplained". `tests/logger_nonblocking.rs` and `benches/log_event.rs` print
the same for an unpinned caller. In Docker on Apple silicon (arm64, and
amd64 emulated) the idle CPU stalled as long as the max in most windows
(the VM takes the vCPU); the rest print their candidates (page faults of
the event's own allocation, softirqs). So the hot
loop logs only rate-limited warnings (one line a second per site, with
`suppressed=`): echo timeout, back-pressure, not connected. Each warning
line costs one allocation in the loop (`tests/placement_aeron.rs` runs the
loop with the app's subscriber and checks that). Per-second progress comes from `ll-stats` on the shared CPU.
Shutdown: dropping the guard waits up to 1.1 s and does not join `ll-log`;
when the queue is full it `println!`s, which blocks on a wedged stdout. So
`ll-app` writes its report to `/dev/termination-log` first, then drops the
guard, then prints the report. Late log lines can be lost; the report
cannot.

### Evidence: `ll-selfcheck` and `ll-verify`

At start, after `ll-hot` pins, each app checks its own threads and exits 2
on a FAIL:

```text
ll-selfcheck PASS thread=ll-hot tid=73 allowed=4 ran_on=4 expected=4
ll-selfcheck PASS thread=aeron-client tid=74 allowed=1 ran_on=1 expected=shared(1)
ll-selfcheck RESULT PASS threads=6 failed=0
```

`ll-probe` (run by `verify.sh` in one `hostPID` pod per node, pinned to the
IRQ CPU) samples every thread of the three pods every 100 ms: its name, its
`Cpus_allowed_list` and the CPU it last ran on (`stat` field 39). It reads
`/proc/interrupts`, `/proc/softirqs` and each hot thread's context switches
at both ends of the window. Each check prints one tab-separated line; any
FAIL, missing thread or zero samples fails the run.

| id | contract | PASS when |
|---|---|---|
| P1 | placement | `ERGON_*` set and disjoint; the container's `cpuset.cpus.effective` is isolated ∪ shared |
| P2 | placement | `ll-hot`, `sender` and `receiver` (or `aeron-md-snd`/`-rcv`) exist |
| P3 | placement | a hot thread's mask is exactly its CPU; the sender and receiver are on different cores |
| P4 | placement | every sample of a hot thread is on its CPU |
| P5 | placement | every other thread's mask is within the shared CPUs: `aeron-client`, `ll-log`, `ll-stats`, the driver's conductor (its main thread), `aeron-md-nra`, `dpdk-*`, unknown names |
| P6 | placement | every sample of every other thread is on a shared CPU |
| F0 | full | `/sys/devices/system/cpu/isolated` and `nohz_full` hold every exclusive CPU |
| F1 | full | no task but the occupant (kernel threads aside) ran on an exclusive CPU |
| F2 | full | no device interrupt on an exclusive CPU (the timer row aside: `LOC`, `HVS`, arm64's `arch_timer`) |
| F3 | full | the timer fires at most `--tick-limit` (2) times a second there; IPIs and softirqs printed as INFO |
| F4 | full | the hot threads made no context switch |
| F5 | full | no CPU quota (`cpu.max` is `max`), never throttled |

A check outside the contract prints `NOT-IN-CONTRACT` with its values. On a
vdev driver the sender and receiver run the kernel's tap and bridge code in
their own system calls (visible in `/proc/softirqs`), so a vdev `full` PASS
proves placement and the absence of other tasks and IRQs, not a kernel-free
path. The F3 and F4 limits are provisional.

From the Docker e2e on `ll:baseline` (`tests/fixtures/e2e-docker-baseline.txt`
holds all 108 lines; tabs shown as spaces):

```text
ll-verify  v1  contract=placement  node=docker-vdev  window_s=30.1  samples=294
ll-verify  pod=e2e/ll-media-driver  P3.cores  sender,receiver  PASS  sender=2 receiver=3 sender_core=2
ll-verify  pod=e2e/ll-media-driver  P3.affinity  sender/55  PASS  allowed=2 expected=2
ll-verify  pod=e2e/ll-media-driver  P4.ran_on  sender/55  PASS  ran_on=2 samples=294 bad=0 expected=2
ll-verify  pod=e2e/ll-media-driver  P5.affinity  aeron-md-nra/57  PASS  allowed=1 expected=shared(1)
ll-verify  pod=e2e/ll-ping  P3.affinity  ll-hot/73  PASS  allowed=4 expected=4
ll-verify  pod=e2e/ll-ping  P4.ran_on  ll-hot/73  PASS  ran_on=4 samples=294 bad=0 expected=4
ll-verify  pod=e2e/ll-ping  P5.affinity  aeron-client/74  PASS  allowed=1 expected=shared(1)
ll-verify  pod=e2e/ll-ping  P6.ran_on  aeron-client/74  PASS  ran_on=1 samples=294 bad=0 expected=shared(1)
ll-verify  node=docker-vdev  F2.device_irqs  cpu=4  NOT-IN-CONTRACT  delta=0 rows=-
ll-verify  RESULT  PASS  checks=54  failed=0
```

### Latency report

The ping measures closed-loop round trips (one 32-byte message in flight, as
the lab's `rtt xping`), stamped with quanta's counter after a `SeqCst`
fence. The pong prints the same lines without the two `rtt_*` ones. A run
proves the tuning only together with an `ll-verify` PASS from the same
window. Docs carry no figures; real runs are in `tests/fixtures/`.

```text
ll-latency  v2  role=ping  arch=…  cpu_build=…  counter_hz=…  transport=dpdk-ena  nic=vdev-tap  overlay=…  msg_bytes=32  mode=closed  warmup=…  samples=…  lost=…
ll-latency  rtt_ns  p50=…  p90=…  p99=…  p99.9=…  p99.99=…  max=…  mean=…
ll-latency  rtt_max_top10  <realtime_ns>:<rtt_ns>,…
ll-latency  loop_gap_ns  max=…  over_5us=…  over_20us=…  over_100us=…
ll-latency  hot  cpu=…  ran_on=…  voluntary_ctxt_switches=…  nonvoluntary_ctxt_switches=…
ll-latency  log  dropped_lines=…  write_errors=…  hot_warn_events=…  suppressed=…
```

`max` is the largest measured round trip, exact; the percentiles come from
an HDR histogram at 3 significant figures. `transport` is the driver's
(`dpdk-ena` on the ENA and the vdev overlays alike); `nic` tells them apart
(`ena` or `vdev-tap`, from `LL_NIC`). A message that stays "not connected"
past `LL_ECHO_TIMEOUT_MS` counts as lost, and an offer stops when the peer's
image closes, so a dead peer ends the run (`ended=peer closed`) instead of
holding the hot loop.

`counter_hz` shows how coarse the clock is (24 MHz on Apple silicon: 42 ns
a tick). In the Docker e2e capture, the largest round trips match the ping's
and pong's own loop gaps and their involuntary context switches: the hot
threads were descheduled on Docker's VM, which has no isolated CPUs. vdev
numbers go through a kernel tap and say nothing about ENA (`nic=vdev-tap`).
One earlier `e2e-vdev.sh --docker` run on the same image failed before
measuring: neither app's publication connected within
`LL_CONNECT_WAIT_S` (60 s), the next two passed; the cause was not found.

### Driver and Aeron settings (lab only)

| setting | value | source |
|---|---|---|
| threading | `DEDICATED`; sender and receiver idle `noop` | `md-dpdk/README.md:119-124` |
| conductor, native resource agent | affinity unset: on the shared CPU | `k8s/nri-shared-cpus/README.md` ("ergon-cpus") |
| EAL | one lcore, on the shared CPU (`-l <shared:0>`) | `md-dpdk/deploy/aws-eks/entrypoint.sh` |
| `AERON_RECEIVER_IO_VECTOR_CAPACITY` | 16 | `md-dpdk/BENCHMARKS.md:214` |
| ENA LLQ | the transport's default (`llq_policy=3`) | `BENCHMARKS.md:215` |
| MTU | 1408 | `md-dpdk/deploy/aws-eks/media-driver-config.yaml` |
| Aeron dir | `/mnt/huge-aeron/aeron`, 2 MiB pages, not sparse; window and socket buffers 2 MiB | `media-driver-config.yaml` |
| app pods | no hugepage request; `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true` | `md-dpdk/deploy/aws-eks/README.md` |
| `ERGON_HOST_CHECKS` | `on` (`require` refuses a stock static-CPU pod: K2) | `tune/README.md` |

### Image tags

`docker/build.sh` builds them on Amazon Linux 2023 (gcc 11.5), DPDK 24.11.4.

| tag | platforms | `-Ctarget-cpu` (Rust, md-dpdk C) | Aeron C `-march` | nodes | built |
|---|---|---|---|---|---|
| `baseline` | linux/amd64, linux/arm64 | `x86-64-v2` / `neoverse-n1` | `x86-64-v2` / `armv8.2-a` | any x86-64-v2 or Neoverse N1-or-later arm64 node | each half built and run (e2e) on its own (`LL_PLATFORMS=`); the one-call two-platform build and the pushed manifest list not run |
| `amd64-v3` | linux/amd64 | `x86-64-v3` | `x86-64-v3` | c7i, AMD Zen 4 (F8as_v6) | no |
| `arm64-n2` | linux/arm64 | `neoverse-n2` | `armv8.5-a` | Azure Cobalt 100 | no |
| `arm64-v3` | linux/arm64 | `neoverse-v3` | `armv8.6-a` | c9g (Graviton5) | no |

AL2023's gcc 11.5 accepts `-mcpu=neoverse-n1`, `-n2` and `-v3`,
`-march=armv8.5-a`, `armv8.6-a` and `x86-64-v3`, and refuses `armv9-a`
(checked in an AL2023 container), so the Aeron C levels are the highest Armv8 ones the CPU
implements. A CPU tag runs only on that CPU family (SIGILL elsewhere). A CPU
tag's latency effect is not measured.

### Not verified

- kind, the Linux VM lane and EKS: chart 0.1.2 on kind; chart 0.1.2 and
  nodeadm's containerd block on AL2023; AL2023's containerd version;
  `node/eks-userdata.sh`'s user data on EC2 (checked offline only).
- The image's `ergon-cpus` comes from the published 0.1.1 image; its source
  is unchanged in 0.1.2 (`docker/Dockerfile`), but it has not run under the
  0.1.2 hook.
- The CPU tags `amd64-v3`, `arm64-n2`, `arm64-v3`: not built.
- A tap PMD and bridge inside a pod's network namespace (the Docker e2e ran
  them in a container's).
- That a client pod with no hugepage request never faults a new hugetlb
  page (a SIGBUS if it does).
- The F3 and F4 limits; `counter_hz` on Cobalt.
- How fast a helper is on a shared CPU that also runs the kubelet.
- Any latency figure from this sample on real hardware.

### Commands

```sh
just samples-ll             # fmt, clippy, unit, logger and manifest tests, entrypoint and verify.sh fixtures (host)
just samples-ll-docker      # the same plus placement tests, linux/arm64 and linux/amd64
just samples-ll-e2e         # driver + ping + pong + ll-probe in Docker over DPDK taps
```

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs (prefer stack when const).
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.
