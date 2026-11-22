# Release gate

## Current release: 0.1.5

| what | value |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus:0.1.5` (and `:latest`) |
| image digest | index `sha256:e52d38f8ffa87627a52d72307a25211b67f243fbf7e61c4b55a18e6512e853e2` (amd64 `sha256:ec3116cc1def1c5a0c8b2290bf96c41de98cceccd23f8aeb5ec896f56f10b0e7`, arm64 `sha256:7a0de9430a9fd3476dbd90daf1cbb39d971a43578f0a9f288f71bc76699786e6`); the index carries the description, source and licence annotations |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.5`, appVersion `0.1.5` |
| chart digest | `sha256:0e38d621cb87c0428fefe201c0d9e8657bbd684c709f245134de26558a51f66b` |
| source | `df9c4dc7`, Actions run [45031159176](https://github.com/mimran1980/ergon/actions/runs/45031159176) (unit, vet, static build, helm lint and template, push) |
| `--version` (amd64, arm64) | `ergon-shared-cpus 0.1.5 df9c4dc77e3c489c36aabcc9f089c4ed0d3eee18 nri=v0.12.3` |
| checked | `helm template` of the OCI chart renders image `:0.1.5`; `--version` pulled on amd64 and arm64 (2026-11-22); the publish run's unit, vet and static-build job passed at that source. The Go lane on an x86 host, the cgroup kernel lane and the kind e2e (18 steps, 0 failed) last ran at the 0.1.4 source, 2026-11-11 |
| changed since 0.1.2 | 0.1.3: the `dpdk.ergon.io` NIC pair exports, Aeron's hugepage variables, and Aeron 1.54's cpuset-offsets name in `ergon-cpus`. 0.1.4: `ergon-cpus --ready-addr` (a readiness probe that also requires a `cpu-partition` pod's partition to be valid) and the opt-in SCHED_FIFO grant removed. 0.1.5: the NIC-pair variables are `ERGON_NIC_PAIR_ADDR_GATEWAY` and `ERGON_NIC_PAIR_PORT_IP_EAL`, from `ERGON_IO_FSTACK` and `ERGON_IO_SMOLTCP`. CPU placement and the hooks are unchanged, so the EKS lane and the benchmarks below, done at 0.1.2, still describe it; EKS was not re-run at 0.1.5 |
| verified on EKS (0.1.2) | AL2023 `c8g.xlarge` spot, Kubernetes 1.36, kernel 6.18.54, containerd 2.3.5, runc 1.4.3, the same plugin and chart source as published 0.1.2 (`85098f8b`), built from that source and pushed to the run's ECR (not the GHCR image), runs `eks-nri-20261025-070515` and `eks-nri-20261025-082023` ([`lab/results/`](lab/results/)): the NRI socket, nodeadm's TOML merge, the kubelet drop-in merge, `/opt` writable, `system-node-critical` in `ergon-system`, the device plugin, the quiet-CPU boot unit. Hot pods were refused while a late ENI's IRQs sat on the shared CPU (fail closed, as designed; `create refused ... reason node-contract` in the plugin log) |
| measured | the hot thread with helpers on the shared CPU (the checkout, on kind on one Azure Cobalt VM, THP `never` and the fair server off on the isolated CPUs): p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs, 0 involuntary switches a 60 s rep; the hypervisor's floor 3.5 to 5.5 µs. Shared-CPU scaling as designed ([BENCHMARKS.md](BENCHMARKS.md)) |

The EKS lane is verified for 0.1.2; 0.1.3 to 0.1.5 leave CPU placement unchanged. Do not invent the figures of
any benchmark row still open below.

## 1. Code defects

All four are fixed. Each one could put a container on the wrong CPUs or
refuse one it should take.

| # | defect | where | state |
|---|---|---|---|
| 1 | `NicNumaNode` defaults to `0` | `internal/config/config.go` | **fixed.** `Parse()` sets `-1` (unknown). Four tests in `config_numa_test.go`: `TestParseNicNumaNodeUnknown`, `TestNicNumaNodeUnknownRefusesOnMultiNode`, `TestNicNumaNodeUnknownStillAnswersOnOneNode`, `TestNicNumaNodeKnownAnswersItsNode` |
| 2 | `CheckCoreLayout` only tests an exclusive CPU's siblings against the shared set | `internal/cpuset/cpuset.go` | **fixed.** Also refuses when a core's siblings sit outside exclusive ∪ shared, naming the unmanaged CPU. Doc comment says only exclusive cores are held to this: a shared core with an unmanaged sibling is fine. Tests `TestCheckCoreLayout` (3 new cases), `TestCheckCoreLayoutNamesTheUnmanagedSibling`, `TestCheckCoreLayoutLeavesSharedAlone` |
| 3 | `UpdateContainer` does not reconcile partition/reservation, and `Synchronize` can replay a stale widen | `internal/nriplugin/plugin.go` | **fixed.** Three things: the reservation follows the container (`hold` on a managed update, `release` when it stops being an exclusive integer-CPU container, `holdPresent` when its shared set is outside today's `sharedCpus`); a resize that makes the `cpu-partition` hooks stale is reported (`stale_partition`), because NRI cannot re-run a createRuntime hook; and the delayed re-widen copy is dropped for a container that has since been updated, stopped or removed (`touched` / `markTouched`). Five tests: `TestUpdateIneligibleReleasesTheReservation`, `TestUpdateRewidenKeepsTheReservation`, `TestUpdateResizeReportsStalePartition`, `TestSynchronizeResendIsNotReplayedAfterUpdate`, `TestSynchronizeResendIsNotReplayedAfterStop` |
| 4 | DaemonSet does not mount the host cgroup tree | `deploy/daemonset.yaml`, `charts/.../templates/daemonset.yaml` | **fixed.** Host cgroup v2 mounted read-only at `/sys/fs/cgroup` (the node's own path; the binary derives that cgroup root and the createRuntime hook matches it — it is not a YAML field). Read-only because the plugin never writes the host tree from inside the pod: the `cpu-partition` writes are `ergon-cpus`'s, and runc runs that hook on the node. Same mount in both manifests; `helm lint` and `helm template` pass |

Sign-off: each defect has a failing test first, then a fix, then the cgroup
kernel lane (`scripts/check-cgroup.sh`) and the kind e2e (`test/e2e.sh`)
green. All four have their unit evidence. On 2026-10-18 the whole tree passed
`scripts/check.sh` (gofmt, vet, `go test -race`, static amd64 and arm64, then
the cgroup kernel lane) and `test/e2e.sh` (18 steps, `fails: 0`, teardown
left no cluster, container, image or volume this run created).

Tests that place containers take whole cores: the defect-2 rule refuses an
exclusive core whose other SMT thread is unmanaged. The defect-3 tests call
`UpdateContainer` on the live container (`applyAdjustment` folds the create
adjustment in, as the runtime does), not on the create request.

## 2. Test coverage

| lane | command | state |
|---|---|---|
| unit, vet, static amd64+arm64 | `scripts/check.sh --local` | green 2026-10-18 over the whole tree |
| cgroup kernel lane (real cgroup v2, partition a real kernel will accept) | `scripts/check-cgroup.sh` | green 2026-10-18 over the whole tree |
| kind e2e, 18 steps | `test/e2e.sh` | green 2026-10-18, `fails: 0` |
| EKS | a two-node EKS run | **verified** on both nodes of runs `eks-nri-20261025-070515` and `eks-nri-20261025-082023` ([`lab/results/`](lab/results/)): NRI socket (`ergon_shared_cpus_nri_connected 1`), nodeadm TOML merge (`containerd config dump` shows `default_validator enable = true`), kubelet drop-in merge (`--config-dir` on the kubelet; `ergon_shared_cpus_contract_ok 1` on node b, and on node a, where it was 0, a node-contract warning naming only the late ENI's IRQs, which the plugin checks only after the merged kubelet config passes), `/opt/ergon/bin/ergon-cpus` installed, `system-node-critical` Running in `ergon-system`, device plugin registered, `ergon-quiet-cpus.service` active with `--check` rc 0 |
| AKS | — | not supported (no NRI enablement, no `reservedSystemCPUs`) |
| CRI-O | — | not supported; use its own `shared_cpuset` |
| Bottlerocket | — | not verified. runc exec of a hook binary from a pod-written host path is unproven on a read-only root with SELinux |

## 3. Benchmarks

The README lists what is **not measured**. Every one of these is a release
blocker, because the claim of this plugin is that the exclusive CPUs stay
quiet. A figure without its max is not a result.

| claim | what must be measured | state |
|---|---|---|
| helpers on shared CPUs do not disturb the hot thread | hot-thread latency p50 / p90 / p99 / p99.9 / p99.99 / **max** with the plugin, helpers loaded | measured (run 20261024234841, [BENCHMARKS.md](BENCHMARKS.md)): p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs over 3 reps, no interrupt on the spin's CPU during the spin; the gaps left have no guest event (the hypervisor) |
| the exclusive CPU takes no context switch | 0 involuntary switches of the hot thread a 60 s rep, every remaining max named | reached in steady state: 0 in each of 3 reps (run 20261024234841), with THP `never` and the fair server off. A traced rep at a systemd daily timer took 1 (40.60 µs, a block-device ioctl's `lru_add_drain_all`), named in its `trace-cpu3.md`: node events stay a risk, kept out of trading hours |
| a pod's helpers scale to its shared CPUs and idle near zero | per-CPU busy time, cgroup usage and throttling at 100 and 1,001 millicpus, idle, and two pods on one shared CPU | measured (run 20261024230400 `scale.md`): 0.99 cores at 100 millicpus, 1.99 at 1,001, 0.04 idle, 0.33 : 0.66 by `cpu.weight` on one shared CPU, `nr_throttled` 0 |
| a wake-up on a shared CPU can wait milliseconds under node load | distribution of that wait under a load the node is meant to carry | not measured |
| the CPU env and the pin do not cost the hot path | cost of `pin_isolated` and of reading `ERGON_*` | not measured; a start-up cost, once a thread |
| capacity accounting is honest | scheduler sees `len(sharedCpus) x 1000 x oversubscription`, and a pod that asks for more is refused | covered by unit tests; on the lab node allocatable 2000 and a pod asking 2,100 stayed Pending (run 20261024230400 `scale/accounting.txt`) |
| the layout refusal fires | a split core is named and refused, with the core id | covered by unit tests, not measured on hardware |

"Covered by all scenarios" means: a split core, an unknown NUMA node, a
missing `thread_siblings_list`, an incomplete topology, a kubelet restart
with the plugin up and with it down, a hand-set `ERGON_*` env, an ineligible
quota or shares, an in-place resize, an exec probe, a `cpu-partition`
opt-in on a kernel without `cpuset.cpus.exclusive`, and a pod outside the
namespace allowlist. Each has a test or a documented refusal.

## 4. Docs

| item | state |
|---|---|
| README status names the released tag and this gate | done |
| README `## Related projects` credits CRI-O, `mixed-cpu-node-plugin`, `dra-driver-cpu` #103/#149/#305, `nri-plugins` and `nri-cpuset-partition`, does not claim novelty and does not criticise their work | done |
| README `## Did the isolation actually hold?` is the IRQ-leak check (probe and socket on one exclusive CPU) | done |
| book install runbook (`book/src/deploy/shared-cpus.md`) names the released tag | done |
| contract (resource name, annotation keys, env names) | stated in the README; changes go in the repo `CHANGELOG.md` |
| `--version` prints version, git describe and NRI module version | done |

## 5. Claims

Nothing published may claim this idea is new or criticise another project's
work: credit them and let readers choose. The README `## Related projects`
section is the standing wording; keep it in step with these projects:

| project | watch |
|---|---|
| [kubernetes-sigs/dra-driver-cpu](https://github.com/kubernetes-sigs/dra-driver-cpu) | [#103](https://github.com/kubernetes-sigs/dra-driver-cpu/issues/103), [#149](https://github.com/kubernetes-sigs/dra-driver-cpu/pull/149), [#305](https://github.com/kubernetes-sigs/dra-driver-cpu/issues/305). When mixed allocation merges, this tree's reason to exist narrows to the containerd-now path and the quant-facing API |
| [cri-o/cri-o](https://github.com/cri-o/cri-o) | `shared_cpuset` is the production reference. Ours must not be described as more mature than it |
| [containers/nri-plugins](https://github.com/containers/nri-plugins) | the heavyweight alternative. Do not grow this tree towards it (IRQ, governors, pool trees) |

Do not claim SBE wire compatibility, latency numbers or "loss-free" in this
tree without a measured `BENCHMARKS.md` beside this file. Unmeasured is
written "not measured", never invented.

## 6. How to publish

A benchmark row still open blocks a tag. Order for a tag:

1. Keep sections 1 and 2 green.
2. Run the benchmarks in section 3 and put the numbers in a
   `BENCHMARKS.md` next to this file, each latency with its max and each max
   traced to a cause.
3. Rewrite the README `## Status` table and the book runbook to name the
   released tag.
4. Run `.github/workflows/nri-shared-cpus-publish.yml` with the tag you want
   and `publish: publish`. That one run builds both platforms, lints the
   chart, and pushes the image and the chart. Its first step refuses to
   continue unless the input reads exactly `publish`, so a typo fails the
   run instead of quietly doing nothing.
5. Verify the push: pull `ghcr.io/mimran1980/ergon-shared-cpus:<tag>` on
   both amd64 and arm64, run `--version`, and `helm install` the pushed
   chart on a test cluster.

The workflow packages the chart at the `tag` input as both version and
appVersion and pushes the image at that tag.

Image and chart names, in one place:

| what | value |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus` |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` |
| platforms | `linux/amd64`, `linux/arm64` |
| tags on release | the version tag, and `latest` |
| licence | Apache-2.0 |
