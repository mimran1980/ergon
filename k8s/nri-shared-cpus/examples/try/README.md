# Try the public chart

One throwaway kind cluster. It installs
`oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.0`, applies
the hot-pod admission policy, and prints what the plugin did.

```sh
k8s/nri-shared-cpus/examples/try/run.sh
```

Needs Docker with at least 6 CPUs, helm, kubectl, and kind v0.33.0.
`KIND=/path/to/kind` if `kind` is not on `PATH`. The script writes
`KUBECONFIG` to `/tmp/ergon-shared-cpus-try.kubeconfig` and refuses any
other value. It deletes cluster `ergon-shared-cpus-try` only after this
run's `kind create` succeeds, including on a later failure or interrupt.
If that cluster already exists, the script stops and leaves it alone. It
does not read `~/.kube/config`. No cloud.

Docker Desktop can crash-loop containerd in this kind node with
`failed to get metadata for stored sandbox`. The script wipes that node's
containerd state once and keeps waiting. It does not wipe a node whose
containerd is answering, and it does not touch any other cluster.

The kind node is `kindest/node:v1.33.0` with the static CPU manager,
`reservedSystemCPUs: "0,1"`, NRI on, and the label
`ergon.io/shared-cpus=true`. The chart is installed with `sharedCpus: "0"`
and namespace allowlist `trading`.

`hot-pod.yaml` is a Guaranteed pod labelled `ergon.io/latency: hot`. It
asks for `cpu: "2"` and `ergon.io/shared-millicpus: "250"`. The script
waits until the log contains:

| line | meaning |
|---|---|
| `ERGON_SHARED_CPUS=0` | the plugin added the housekeeping CPU from the chart |
| `ERGON_ISOLATED_CPUS` non-empty | the kubelet's two exclusive CPUs |
| `Cpus_allowed_list: 0` | PID 1 is bound to the shared CPU only |
| `nproc=1` | the process affinity is that one CPU |

`bad-probe.yaml` is the same pod with an exec `livenessProbe`. The
ValidatingAdmissionPolicy must deny it. The denial text contains
`livenessProbe must not be exec`.

## What this is, and what it skips

Kubernetes HFT tuning is the node and pod settings in the book chapter
`book/src/deploy/hft.md`. This sample runs two parts of that setup:

- the shared-CPU grant, from the public `ergon-shared-cpus` image;
- the admission policy in `k8s/nri-latency-pods/admission/`, which refuses
  an exec probe on a hot pod.

There is no HFT image. Hugepages, an isolated cpuset partition, io_uring
seccomp and the NIC PHC are grants in the same plugin. Chart 0.1.0 does
not write `seccomp`, `ptp` or `schedFifo`, and this kind node is not a
place those grants can be shown: no hugepage pool, no `/dev/ptp`, no
`cpuset.cpus.exclusive` partition. Latency is not measured. EKS is not
involved.

`helm install` of chart 0.1.0 prints "This chart is NOT released". That
note was packed before the image was pushed. The script still installs
that tag. The log says the sentence is stale.

`test/e2e.sh` is the longer kind run. It builds `ergon-shared-cpus:dev`
from this tree and does not pull the public chart.

## Run on 2026-10-22

`run.sh` against chart `0.1.0` on `kindest/node:v1.33.0`. Docker Desktop,
linux aarch64, 10 CPUs. The cluster was deleted at the end. No kind
clusters remained.

```
allocatable ergon.io/shared-millicpus=1k
ERGON_ISOLATED_CPUS=2,3
ERGON_SHARED_CPUS=0
ERGON_SHARED_MILLICPUS=250
ERGON_HELPER_CPUS=0
ERGON_ISOLATED_CORES=2;3
Cpus_allowed_list:	0
cpuset.cpus.effective=0,2-3
nproc=1
```

The exec-probe pod was refused:

```
Error from server (Forbidden): ... ValidatingAdmissionPolicy 'ergon-latency-pods' with binding 'ergon-latency-pods' denied request: livenessProbe must not be exec/command on a hot pod.
```

`1k` is 1000 millicpus: one shared CPU, oversubscription 1. CPUs 2 and 3
were the kubelet's exclusive pair on that node. CPU 0 was the shared CPU
from the chart. This is one kind node. It is not an EKS result and it is
not a latency measurement.
