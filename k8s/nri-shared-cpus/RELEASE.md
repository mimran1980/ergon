# Release gate

## Current release: 0.1.2

| what | value |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus:0.1.2` (and `:latest`) |
| image digest | index `sha256:33ddd06f6b2e58bae6a2bf13092366482b5a292bc50a01223b0b383b66026da6` (amd64 `sha256:63946a9a9c11ec98be252778c16f9e7020186b1a5400ee711251b6510fb748a9`, arm64 `sha256:0d341b3cc0f9c125708120e216e12b61ed7827c74d24851c51afec10c728e102`); the index carries the `org.opencontainers.image.description` annotation |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.2`, appVersion `0.1.2` |
| chart digest | `sha256:145054055d604e8a866bf86556cd3a1b46b92b1b6e3347183b8033dd7112b14d` |
| source | `85098f8b`, Actions run [40173324072](https://github.com/mimran1980/ergon/actions/runs/40173324072) (unit, vet, static build, helm lint and template, push) |
| `--version` (amd64, arm64) | `ergon-shared-cpus 0.1.2 85098f8b16e17d80747c73d6069c1ec07d70496a nri=v0.12.3` |
| checked | `helm template` of the OCI chart renders image `:0.1.2` with `--kubelet-config`; the kind e2e (`test/e2e.sh`) at that source: 18 steps, 0 failed (2026-10-25) |
| not verified | EKS |
| measured | the hot thread with helpers on the shared CPU (the checkout, on kind on one Azure Cobalt VM, THP `never` and the fair server off on the isolated CPUs): p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs, 0 involuntary switches a 60 s rep; the hypervisor's floor 3.5 to 5.5 µs. Shared-CPU scaling as designed ([BENCHMARKS.md](BENCHMARKS.md)) |
| not measured | a helper's wake-up wait on a shared CPU, the pin's cost |

The EKS lane and the open benchmarks below are open for 0.1.2. Do not
invent their figures.

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
| EKS | — | **not verified.** The NRI socket, nodeadm TOML merge, the kubelet drop-in merge, `/opt` writable and `system-node-critical` outside `kube-system` are all unproven on a real node |
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
| the CPU env and the pin do not cost the hot path | cost of `pin_isolated` and of reading `ERGON_*` | not measured |
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
| README `## Prior art` names CRI-O, `mixed-cpu-node-plugin`, `dra-driver-cpu` #103/#149/#305, `nri-plugins` and `nri-cpuset-partition`, and does not claim novelty | done |
| README `## Did the isolation actually hold?` is the IRQ-leak check (probe and socket on one exclusive CPU) | done |
| book install runbook (`book/src/deploy/shared-cpus.md`) names the released tag | done |
| contract (resource name, annotation keys, env names) | stated in the README; changes go in the repo `CHANGELOG.md` |
| `--version` prints version, git describe and NRI module version | done |

## 5. Claims

Nothing published may claim this idea is new. The README `## Prior art`
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

The EKS row and the benchmark rows are open. Order for a tag:

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
