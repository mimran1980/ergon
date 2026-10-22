# Release gate

`ergon-shared-cpus` **0.1.1** was pushed on 2026-10-22 from `feat/0.1.33`
at `515846b0`, Actions run
[39853440732](https://github.com/mimran1980/ergon/actions/runs/39853440732).
The image is `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` (and `:latest`),
digest `sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0`.
`--version` on linux/amd64 and linux/arm64 printed
`ergon-shared-cpus 0.1.1 515846b0201b5fd70a7a6cf3a88839b6180fa9c1`.
The chart is `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version
`0.1.1`, digest `sha256:77a65305a45ea1b376ce80f9b1d5234b9061f4066a8c5baf5efdb892854fc0d8`.
`helm template` rendered that image. A kind install of 0.1.1 was not run.
The pushed NOTES still say the chart is not published. That tag is not
republished to change the note.

Tag **0.1.0** remains. Its image digest is
`sha256:d28fea2f95ebce3e53d33f8770f3ba69be75c76ec7437ea2b92c1329759e07e9`.
Its chart digest is
`sha256:7347131d7d87caee5cf6de11b2a11e2b62454da59c41092b3805bc2834795dbd`.

The rows below that are still open were open on that push. The EKS lane is
not verified. The latency figures are not measured. Do not invent them.

## 1. Code defects

These were known and open. All four are fixed; each one could put a container
on the wrong CPUs or
refuse one it should take.

| # | defect | where | state |
|---|---|---|---|
| 1 | `NicNumaNode` defaults to `0` | `internal/config/config.go` | **fixed.** `Parse()` sets `-1` (unknown). Four tests in `config_numa_test.go`: `TestParseNicNumaNodeUnknown`, `TestNicNumaNodeUnknownRefusesOnMultiNode`, `TestNicNumaNodeUnknownStillAnswersOnOneNode`, `TestNicNumaNodeKnownAnswersItsNode`. Before: `NicNumaNode = 0, want -1`, and `PickMems("", [0 1], 0) = "0"` (silent node 0). After: all pass |
| 2 | `CheckCoreLayout` only tests an exclusive CPU's siblings against the shared set | `internal/cpuset/cpuset.go` | **fixed.** Also refuses when a core's siblings sit outside exclusive ∪ shared, naming the unmanaged CPU. Doc comment says only exclusive cores are held to this: a shared core with an unmanaged sibling is fine. Tests `TestCheckCoreLayout` (3 new cases), `TestCheckCoreLayoutNamesTheUnmanagedSibling`, `TestCheckCoreLayoutLeavesSharedAlone` |
| 3 | `UpdateContainer` does not reconcile partition/reservation, and `Synchronize` can replay a stale widen | `internal/nriplugin/plugin.go` | **fixed.** Three things: the reservation now follows the container (`hold` on a managed update, `release` when it stops being an exclusive integer-CPU container, `holdPresent` when its shared set is outside today's `sharedCpus`); a resize that makes the `cpu-partition` hooks stale is reported (`stale_partition`), because NRI cannot re-run a createRuntime hook; and the delayed re-widen copy is dropped for a container that has since been updated, stopped or removed (`touched` / `markTouched`). Five tests: `TestUpdateIneligibleReleasesTheReservation`, `TestUpdateRewidenKeepsTheReservation`, `TestUpdateResizeReportsStalePartition`, `TestSynchronizeResendIsNotReplayedAfterUpdate`, `TestSynchronizeResendIsNotReplayedAfterStop` |
| 4 | DaemonSet does not mount the host cgroup tree | `deploy/daemonset.yaml`, `charts/.../templates/daemonset.yaml` | **fixed.** Host cgroup v2 mounted read-only at `/sys/fs/cgroup` (the node's own path; the binary derives that cgroup root and the createRuntime hook matches it — it is not a YAML field). Read-only because the plugin never writes the host tree from inside the pod: the `cpu-partition` writes are `ergon-cpus`'s, and runc runs that hook on the node. Same mount in both manifests; `helm lint` and `helm template` pass |

Sign-off: each defect has a failing test first, then a fix, then the cgroup
kernel lane (`scripts/check-cgroup.sh`) and the kind e2e (`test/e2e.sh`)
green. All four have their unit evidence. On 2026-10-18 the whole tree passed
`scripts/check.sh` (gofmt, vet, `go test -race`, static amd64 and arm64, then
the cgroup kernel lane) and `test/e2e.sh` (18 steps, `fails: 0`, teardown
left no cluster, container, image or volume this run created).

Two test fixtures were wrong rather than the product, and are worth knowing
about. Defect 2's core-layout rule now refuses an exclusive core whose other
SMT thread is unmanaged, which is the rule working: `TestBalanceOverSharedCPUs`
put three containers on CPUs 2, 3 and 6 and so split the cores 2,6 and 3,7
across containers, and `TestCreateCpuPartitionWithoutSharedOptIn` asked for
2,3 with their siblings 6,7 unmanaged. Both now take whole cores. Separately,
the defect-3 tests called `UpdateContainer` on the create *request* rather
than the live container; `applyAdjustment` folds the create adjustment in the
way the runtime does, so the update sees the widened cpuset and the
`ERGON_ISOLATED_CGROUP` env.

## 2. Test coverage

| lane | command | state |
|---|---|---|
| unit, vet, static amd64+arm64 | `scripts/check.sh --local` | green 2026-10-18 over the whole tree |
| cgroup kernel lane (real cgroup v2, partition a real kernel will accept) | `scripts/check-cgroup.sh` | green 2026-10-18 over the whole tree |
| kind e2e, 18 steps | `test/e2e.sh` | green 2026-10-18, `fails: 0` |
| EKS | — | **not verified.** The NRI socket, nodeadm TOML merge, `/opt` writable and `system-node-critical` outside `kube-system` are all unproven on a real node |
| AKS | — | not supported (no NRI enablement, no `reservedSystemCPUs`) |
| CRI-O | — | not supported; use its own `shared_cpuset` |
| Bottlerocket | — | not verified. runc exec of a hook binary from a pod-written host path is unproven on a read-only root with SELinux |

## 3. Benchmarks

The README lists what is **not measured**. Every one of these is a release
blocker, because the claim of this plugin is that the exclusive CPUs stay
quiet. A figure without its max is not a result.

| claim | what must be measured | state |
|---|---|---|
| helpers on shared CPUs do not disturb the hot thread | hot-thread latency p50 / p90 / p99 / p99.9 / p99.99 / **max** with and without the plugin, same load, same host | not measured |
| a wake-up on a shared CPU can wait milliseconds under node load | distribution of that wait under a load the node is meant to carry | not measured |
| the CPU env and the pin do not cost the hot path | cost of `pin_isolated` and of reading `ERGON_*` | not measured |
| capacity accounting is honest | scheduler sees `len(sharedCpus) x 1000 x oversubscription`, and a pod that asks for more is refused | covered by unit tests, not measured |
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
| README status names tag 0.1.0 and this gate | done 2026-10-22 |
| README `## Prior art` names CRI-O, `mixed-cpu-node-plugin`, `dra-driver-cpu` #103/#149/#305, `nri-plugins` and `nri-cpuset-partition`, and does not claim novelty | done |
| README `## Verify` includes the IRQ-leak check (probe and socket on one exclusive CPU) | done |
| book install runbook (`book/src/deploy/shared-cpus.md`) | present; must name the released image tag |
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

The first push ran on 2026-10-22 from `feat/0.1.33` at `e1e97ef6`,
Actions run
[39729861655](https://github.com/mimran1980/ergon/actions/runs/39729861655).
The image push and the chart push succeeded. A later step called
`PATCH /user/packages/container/{name}` to set visibility and got HTTP 404.
That step is not how these packages became pullable: an anonymous token
from `ghcr.io/token` then fetched both manifests with HTTP 200. The job is
red because of the 404. The tags are on GHCR.

0.1.0 did not wait for the open EKS row or the benchmark rows. Order for a later tag:

1. Fix the four defects and the test lanes in sections 1 and 2.
2. Run the benchmarks in section 3 and put the numbers in a
   `BENCHMARKS.md` next to this file, each latency with its max and each max
   traced to a cause.
3. Rewrite the README status paragraph and the book runbook to name the
   released tag.
4. Run `.github/workflows/nri-shared-cpus-publish.yml` with the tag you want
   and `publish: publish`. That one run builds both platforms, lints the
   chart, and pushes the image and the chart. Its first step refuses to
   continue unless the input reads exactly `publish`, so a typo fails the
   run instead of quietly doing nothing.
5. Verify the push: pull `ghcr.io/mimran1980/ergon-shared-cpus:<tag>` on
   both amd64 and arm64, run `--version`, and `helm install` the pushed
   chart on a test cluster.

Image and chart names, in one place:

| what | value |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus` |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` |
| platforms | `linux/amd64`, `linux/arm64` |
| tags on release | the version tag, and `latest` |
| licence | Apache-2.0 |
