# Kubernetes

A busy-spinning pod needs what a busy-spinning process on a VM needs: a whole
core per busy thread, interrupts elsewhere, no page faults. Kubelet has to be
told to give it that. The worked example is the EKS deployment of the DPDK
media driver,
[`md-dpdk/deploy/aws-eks`](https://github.com/mimran1980/ergon/tree/main/md-dpdk/deploy/aws-eks).
Its node and pod settings apply to any low-latency pod; its DPDK parts are a
[proof of concept](../md-dpdk/overview.md), not for production.

Terms: [deployment glossary](overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary).

| | x86 (`c7i`) | Graviton |
|---|---|---|
| manifests and node bootstrap | yes: Amazon Linux 2023, self-managed nodes | none: the bootstrap refuses non-Intel CPUs and the images are `linux/amd64` |
| measured | DPDK pods on two `c7i.4xlarge`, with its max ([benchmarks](benchmarks.md#kubernetes-eks-on-c7i4xlarge)) | not measured |

The kubelet and pod settings are not x86-specific, but no Graviton node has
been built with them. For Graviton's kernel arguments see [hosts](hosts.md).

## Latency checklist

Use a dedicated node pool, tainted so only latency pods land there
([keep other pods in check](#keep-other-pods-in-check)).

**Node (kubelet and host):**

1. SMT off on x86, so one Kubernetes CPU is one physical core, the same as
   on Graviton ([SMT off](#smt-off)).
2. `cpuManagerPolicy: static` with `full-pcpus-only: "true"`: Guaranteed pods
   get whole cores of their own.
3. `reservedSystemCPUs`: the housekeeping cores, the same ones the kernel's
   boot line leaves out of isolation.
4. `topologyManagerPolicy: single-numa-node`: a pod's CPUs and devices on one
   NUMA node.
5. No CPU quota on the latency containers: kubelet 1.36 or later (1.33 is
   the minimum for this), `cpuCFSQuota` left at its default `true`. Do NOT
   set `cpuCFSQuota: false`: it removes every pod's CPU limit on the node
   ([why](#no-cpu-quota)).
6. The host settings under the kubelet: isolated cores, interrupts on
   housekeeping, transparent hugepages off, hugepages ([hosts](hosts.md)).

**Pod:**

7. Guaranteed QoS: requests equal limits for every resource, in every
   container.
8. A whole-number CPU request on every busy-spinning container. With SMT
   off, one CPU is one core.
9. Each busy thread pinned to its own core from the pod's cpuset, and not
   `SCHED_FIFO` (T7).
10. On CRI-O only: the annotations in [No CPU quota](#no-cpu-quota).
    containerd (EKS, AKS, k3s) ignores them.

**Check:**

11. At start: ergon-io and the media driver run
    [ergon-tune](../tune/overview.md), and T9 warns about a CPU quota. In
    your own loop, call `t9_cpu_quota` before it.
12. While it runs: `nr_throttled` must not grow ([verify](#verify)). In your
    own loop, call `ThrottleWatch::check` from housekeeping.

## Nodes

**Kubelet** gives Guaranteed pods whole cores of their own (the static CPU
manager) and keeps a pod's CPUs and devices on one NUMA node (the topology
manager). `node/userdata.sh` writes this, with `$reserved` the housekeeping
CPUs (`0` on a `c7i.4xlarge`: one CPU is one physical core, because SMT is
off):

```yaml
{{#include ../../../md-dpdk/deploy/aws-eks/node/userdata.sh:130:144}}
```

- `full-pcpus-only`: whole physical cores, never one hyperthread shared with
  another pod. With SMT off it is a safety net.
- `kubeReservedCgroup` and `systemReservedCgroup` are emptied because kubelet
  rejects `reservedSystemCPUs` while they are set.
- `memoryManagerPolicy: None`: every `c7i` below `metal-48xl` has one NUMA
  node.
- `cpuCFSQuota` is not set: it stays `true`, so every pod keeps its CPU
  limit. The kubelet drops the quota itself on a container with exclusive
  CPUs ([No CPU quota](#no-cpu-quota)).

The node scripts (tested against fixtures by `just test-dpdk-bootstrap`):

| script | when | what it does |
|---|---|---|
| `node/provision-node.sh` | once, then one reboot before kubelet | kernel command line: every CPU but the reserved core isolated, interrupts on that core, C-states C1 on a VM, `iommu=pt`, `transparent_hugepage=never`, 2 MiB hugepages, so kubelet's first start advertises `hugepages-2Mi` |
| `node/bootstrap-dpdk-ena.sh` | every boot, before kubelet | interrupts on housekeeping with a watcher that re-pins on each link event; `irqbalance`, KSM, NUMA balancing off; THP off; `rmem_max`/`wmem_max` 16 MiB; Aeron's directory on a sized hugetlbfs at `/mnt/huge-aeron`; for DPDK the two spare ENAs on `igb_uio`. On failure the node still runs kernel UDP and advertises no DPDK pair |

Clocksource is left to the kernel. A forced `clocksource=tsc tsc=reliable`
turns the kernel's TSC checks off, so bare metal takes it only with
`RUSTERON_FORCE_TSC=1`, after the TSC has been measured as the same across
cores against a PTP/PHC reference. A VM never gets it.

Kubelet's reserved CPUs and the kernel's housekeeping CPUs are the same set:
interrupts and system daemons stay off the pods' cores. Dedicated nodes are
labelled `rusteron-dpdk/ena=true` and tainted `rusteron-dpdk:NoSchedule`.

## SMT off

SMT (hyper-threading) runs two logical CPUs on one physical core. They share
the core's execution resources: a pod with `cpu: 1` shares a core with its
twin, and its latency follows whatever the twin does. Turn SMT off on every
x86 latency node, so one Kubernetes CPU is one physical core — the same as
on Graviton, which has no SMT.

| where | how |
|---|---|
| EC2, virtual machine | the node group's launch template: `CpuOptions` with `CoreCount` (the type's default core count) and `ThreadsPerCore: 1`. Both fields are required; there is no extra charge. Not on bare metal, and not on T2, C7a, M7a, R7a or Apple silicon Macs (EC2 will not change them; C7a, M7a and R7a already run one thread per core) |
| EC2 bare metal, self-managed | the kernel's `nosmt` on the boot line. Before the kubelet's first start, writing `off` to `/sys/devices/system/cpu/smt/control` works too: write it, then delete `/var/lib/kubelet/cpu_manager_state` (the CPU manager state lists the old CPUs, and the kubelet crash-loops without the delete) |
| Azure | a size without SMT: `Fasv6`, `Falsv6`, `Famsv6` (AMD) or `Dpsv6`, `Dplsv6` (Cobalt) |
| self-managed, k3s | `nosmt` on the kernel command line |

- With `ThreadsPerCore: 1` the instance runs half its usual vCPU count (a
  `c7i.4xlarge`: 8 cores, 8 vCPUs, not 16). Size pods and quotas for cores.
- `thread_siblings_list` still lists the offline twin. Name one CPU per core
  (the lowest id) in `reservedSystemCPUs` and in kernel arguments. The node
  scripts do.
- The launch template that sets it: `node/launch-template.py`. The live lab
  fails a node whose `/sys/devices/system/cpu/smt/active` is `1`
  (`md-dpdk/benches/cloud/aws/eks.sh`).

## Pods

- **Guaranteed QoS with whole cores:** requests equal to limits on every
  resource, and a whole-number CPU request.
- **Pin each busy thread to its own core from the pod's cpuset.** Read it from
  `Cpus_allowed_list` in `/proc/self/status`; in a privileged container
  `/sys/fs/cgroup/cpuset.cpus.effective` lists every host CPU. One logical
  CPU per physical core, and Aeron's client conductor off the busy cores.
- **Hugepages as a resource** (`hugepages-2Mi`), Aeron's directory from the
  node's hugetlbfs as a `hostPath`, and
  `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true` on clients.
- **Handle SIGTERM.** Kubelet stops a pod with it; a PID 1 with no handler
  ignores it until SIGKILL, closing nothing.
- **Privileged only where needed:** the DPDK driver pod (it maps the NIC and
  locks hugepages). A client that only reaches the driver through Aeron's
  directory is not.
- **`hostNetwork: true`** takes a kernel-UDP pod off the pod network's virtual
  interface. Not measured here.

## Keep other pods in check

A dedicated node pool only works if nothing else lands there.

- **Taint the latency nodes** (`NoSchedule`) and let only latency pods and
  the DaemonSets that must run there tolerate it: the CNI, kube-proxy, the
  node agent, a log collector, the media driver. Everything else stays off.
- **A `LimitRange` per namespace** with a default CPU request and limit for
  containers, and a `ResourceQuota` for the namespace. An unlabelled pod
  then cannot ask for half a node.
- **Small CPU requests and limits on the DaemonSets that stay** (log
  collectors, a metrics agent): a limit keeps a burst off the latency
  cores. Give both; a limit with no request still lets them burst up to it.
- **`PriorityClass`:** one high class for latency pods, a low one for the
  rest. A latency pod then preempts, never the other way round.
- **Kubelet 1.36 or later** with the static CPU manager: only a Guaranteed
  whole-CPU container loses its quota; everything else keeps its CPU limit
  ([No CPU quota](#no-cpu-quota)).

**`strict-cpu-reservation`** (kubelet policy option, GA in 1.35): by default
the shared CPU pool includes `reservedSystemCPUs`, so a Burstable pod or a
fractional sidecar can run on the reserved CPUs. `strict-cpu-reservation:
"true"` keeps everything that was not given those CPUs off them. Use it only
when nothing but the system (and threads you pin there yourself) may use
them: it breaks anything that needs the shared pool by default, such as a
fractional sidecar in a latency pod or ergon-stack's host-stats thread. The
shared-CPU NRI plugin (`k8s/nri-shared-cpus`; kind e2e green 2026-10-18, EKS
and the latency benchmarks open — `RELEASE.md`) lends shared
CPUs from `reservedSystemCPUs` to a pod's helper threads and expects the
default, so leave `strict-cpu-reservation` off while using it.

## No CPU quota

- A CPU quota (Linux's CFS bandwidth control) lets a container's threads run
  for at most the quota in each 100 ms period. Kubernetes sets it from the
  container's CPU limit.
- A busy-spinning thread uses its core all the time, so it runs at the cap.
  When the container's other threads push it over, the kernel stops every
  thread in it until the period ends: a stall of milliseconds.
- A pod has a quota of its own too: the sum of its containers' limits. From
  inside a container, only `cpu.stat.local` shows it.

What the kubelet writes:

| case | container `cpu.max` | pod `cpu.max` |
|---|---|---|
| default (`cpuCFSQuota: true`), no exclusive CPUs | limit × period: `200000 100000` for 2 CPUs | the sum of the containers' limits |
| kubelet 1.33+, a container with exclusive CPUs | `max` | `max` when any container has exclusive CPUs |
| a sidecar with a fractional CPU in that pod | still capped: `50000 100000` | `max` |
| `cpuCFSQuota: false` (the whole node: **do not**) | `max` | `max` |

- **Exclusive CPUs** need all three: `cpuManagerPolicy: static`, a
  Guaranteed pod, and a whole-number CPU request on the container. Kubelet
  1.33 is the first that then drops the quota; use 1.36 or later on a
  latency node.
- **Do NOT set `cpuCFSQuota: false`.** It is node-wide: it removes the CPU
  limit of every pod there, log collectors and system pods too, and they
  then run unlimited on the shared CPUs. The kubelet's own behaviour above
  is enough, and it only touches the containers with exclusive CPUs.
- **Kubelet 1.31 and 1.32** (EKS extended support, AKS) always set the
  quota. Use 1.33 or later, 1.36 or later for a latency node (the lab's k3s
  and the EKS deployment are 1.36).
- **Changing `cpuManagerPolicy`:** drain the node, stop the kubelet, delete
  `/var/lib/kubelet/cpu_manager_state`, start the kubelet. Without the
  delete, the kubelet crash-loops.

The kubelet config file:

```yaml
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cpuManagerPolicy: static
cpuManagerPolicyOptions:
  full-pcpus-only: "true"
reservedSystemCPUs: "0"     # the housekeeping cores (one CPU is one core)
topologyManagerPolicy: single-numa-node
# cpuCFSQuota is not set: it stays true, so every pod keeps its CPU limit.
```

**EKS, Amazon Linux 2023:** nodeadm merges `spec.kubelet.config` into its
defaults. Put the `NodeConfig` in the launch template's user data, as a MIME
part with `Content-Type: application/node.eks.aws`. The full one, with the
cluster fields, is in [Nodes](#nodes).

```yaml
apiVersion: node.eks.aws/v1alpha1
kind: NodeConfig
spec:
  kubelet:
    config:
      cpuManagerPolicy: static
      cpuManagerPolicyOptions:
        full-pcpus-only: "true"
      reservedSystemCPUs: "0"
```

**EKS, Bottlerocket:** in the user data (TOML). Reboot after changing the
policy. It has `reserved-cpus` and `cpu-manager-policy-options` too, but no
feature gates. Leave `cpu-cfs-quota-enforced` at its default `true`.

```toml
[settings.kubernetes]
cpu-manager-policy = "static"
```

**AKS:** per node pool. AKS has no `reservedSystemCPUs` field.

```sh
echo '{"cpuManagerPolicy": "static"}' > kubelet.json
az aks nodepool add --resource-group <group> --cluster-name <cluster> \
  --name latency --kubelet-config kubelet.json
```

**k3s:** in `/etc/rancher/k3s/config.yaml`, or as a `KubeletConfiguration`
drop-in under `/var/lib/rancher/k3s/agent/etc/kubelet.conf.d/`.

```yaml
kubelet-arg:
  - "cpu-manager-policy=static"
  - "reserved-cpus=0"
```

**CRI-O (OpenShift):** the kubelet settings above are enough. CRI-O can also
drop the quota per pod, but only with the high-performance runtime class
(OpenShift's Node Tuning Operator makes `performance-<profile>`), a
Guaranteed pod and whole CPUs. containerd ignores these annotations, so they
are safe in a shared manifest.

```yaml
metadata:
  annotations:
    cpu-quota.crio.io: "disable"
    cpu-load-balancing.crio.io: "disable"
    irq-load-balancing.crio.io: "disable"
spec:
  runtimeClassName: performance-<profile>
```

### Verify

On the node:

```sh
kubectl get node <node> -o jsonpath='{.status.nodeInfo.kubeletVersion}'   # 1.36 or later (1.33 minimum)
kubectl get --raw /api/v1/nodes/<node>/proxy/configz \
  | jq '.kubeletconfig | {cpuCFSQuota, cpuManagerPolicy, cpuManagerPolicyOptions, reservedSystemCPUs}'
```

`cpuCFSQuota` must read `true` (or be absent: that is the default).

In the pod:

```sh
cat /sys/fs/cgroup/cpu.max            # max 100000
grep throttled /sys/fs/cgroup/cpu.stat
cat /sys/fs/cgroup/cpu.stat.local     # Linux 6.6+: counts the pod's quota too
```

- `nr_throttled` and `throttled_usec` must not grow while the pod runs. Read
  them twice, a minute apart.
- In Prometheus, cAdvisor's `container_cpu_cfs_throttled_periods_total` must
  stay flat.
- [ergon-tune](../tune/overview.md) does both: T9 at start, and
  `ThrottleWatch` while the loop runs.

## The media driver DaemonSet

One driver pod per DPDK node: one ENA pair from the device plugin, six CPUs
(housekeeping, sender, receiver and three spare; one CPU is one core), memory
and hugepages, requests equal to limits.

```yaml
{{#include ../../../md-dpdk/deploy/aws-eks/media-driver-daemonset.yaml}}
```

Its ConfigMap, with the reason for each value:

```yaml
{{#include ../../../md-dpdk/deploy/aws-eks/media-driver-config.yaml}}
```
*(`md-dpdk/deploy/aws-eks/`; the node scripts and the entrypoint are tested
against fixtures by `just test-dpdk-bootstrap`.)*

The entrypoint reads the pod's cpuset and hands out cores in order:
housekeeping, sender, receiver, then the rest for a client. Node
requirements, the hugepage budget and the device plugin: the deployment's
[README](https://github.com/mimran1980/ergon/blob/main/md-dpdk/deploy/aws-eks/README.md).

## Checks inside a pod

A pod sees the host's CPU, interrupt and memory files but its own network
namespace. [ergon-tune](../tune/overview.md) reports the network settings it
cannot read there as unknown, never as a warning. Run the NIC checks on the
node, or in a pod on the host's network. T9 reads the container's own cgroup;
the pod's quota shows only in `cpu.stat.local` ([verify](#verify)).
