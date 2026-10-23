# Try the public chart

One throwaway kind cluster. It installs
`oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1` with the
hot-pod check's host mounts, applies the hot-pod admission policy, and
prints what the plugin did.

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
`ergon.io/shared-cpus=true`. The chart is installed with `sharedCpus: "0"`,
namespace allowlist `trading`, and `hostContract.kubeletConfig`, `sys` and
`proc` on. The script checks the DaemonSet passes `--kubelet-config`,
`--host-sys` and `--host-proc`.

A kind node shares the host kernel, so it has no `isolcpus`. A pod
labelled `ergon.io/latency: hot` needs its exclusive CPUs isolated, so no
hot pod can start here. The sample shows the grant on an unlabelled pod
and the refusal on a hot one. It does not switch the check off.

| file | pod | expected |
|---|---|---|
| `shared-pod.yaml` | Guaranteed, `cpu: "2"`, `ergon.io/shared-millicpus: "250"`, no hot label | Ready. Log: `ERGON_SHARED_CPUS=0`, non-empty `ERGON_ISOLATED_CPUS`, `Cpus_allowed_list: 0`, `nproc=1` |
| `hot-pod.yaml` | the same, labelled hot, with `priorityClassName: ergon-hot`. Applied after `trader` is deleted, so its two CPUs fit | admitted, then `CreateContainerError` with `exclusive CPU not isolated` from the plugin, never running |
| `bad-probe.yaml` | the hot pod with an exec `livenessProbe` | denied by the ValidatingAdmissionPolicy: `livenessProbe must not be exec` |

What the log lines mean:

| line | meaning |
|---|---|
| `ERGON_SHARED_CPUS=0` | the plugin added the housekeeping CPU from the chart |
| `ERGON_ISOLATED_CPUS` non-empty | the kubelet's two exclusive CPUs |
| `Cpus_allowed_list: 0` | PID 1 is bound to the shared CPU only |
| `nproc=1` | the process affinity is that one CPU |

## What this is, and what it skips

Kubernetes HFT tuning is the node and pod settings in the book chapter
`book/src/deploy/hft.md`. This sample runs two parts of that setup:

- the shared-CPU grant, from the public `ergon-shared-cpus` image;
- the admission policy in `k8s/nri-latency-pods/admission/`, which refuses
  an exec probe on a hot pod.

There is no HFT image. Hugepages, an isolated cpuset partition, io_uring
seccomp and the NIC PHC are grants in the same plugin. The chart writes
`seccomp`, `ptp` and `schedFifo` as `"off"`, and this kind node is not a
place those grants can be shown: no hugepage pool, no `/dev/ptp`, no
`cpuset.cpus.exclusive` partition. Latency is not measured. EKS is not
involved.

`helm install` prints NOTES that say the chart is not published. It is
published; ignore that line.

`test/e2e.sh` is the longer kind run. It builds `ergon-shared-cpus:dev`
from this tree and does not pull the public chart.

## Status

Not yet run on kind.
