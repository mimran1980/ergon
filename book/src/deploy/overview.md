# Deployment

A busy-spinning thread is only as fast as its host.

| page | for |
|---|---|
| [Kubernetes HFT tuning](hft.md) | **start here**: every setting that won an A/B, its measured effect, copy-paste blocks, three checks |
| [AWS and Azure hosts, x86 and Graviton](hosts.md) | every host setting: what it does, its measured effect, how to set it |
| [Kubernetes](kubernetes.md) | whole cores, hugepages and the node bootstrap; the EKS manifests |
| [Benchmarks: x86 and Graviton](benchmarks.md) | which platform: latency and cost per hour, side by side |
| [Java for Archive and Cluster](java.md) | the JVM and Aeron settings for Archive and Cluster |
| [Latency pods](latency-pods.md) | the five grants a normal pod cannot make |
| [Shared CPUs](shared-cpus.md) | `k8s/nri-shared-cpus`: exclusive CPUs plus the node's shared ones |

## Checklist

1. **A physical core per busy thread**, isolated from the scheduler, the tick
   and interrupts, plus one core for housekeeping. On x86 turn SMT off, so
   one CPU is one physical core, the same as on Graviton
   ([SMT off](kubernetes.md#smt-off)).
2. **No CPU quota on a busy-spinning container:** on Kubernetes, the static
   CPU manager with Guaranteed pods and whole CPUs on kubelet 1.33 or later
   (1.36 or later for a latency node), so the kubelet drops the quota
   itself. Do NOT set `cpuCFSQuota: false`: it removes every pod's CPU
   limit on the node
   ([checklist](kubernetes.md#latency-checklist)).
3. **Interrupts on housekeeping, and pinned again after every NIC reset:**
   ENA moves them back onto isolated CPUs whenever an interface comes up.
4. **Interrupt moderation off** (the NIC batching packets into fewer
   interrupts): the adaptive part for a busy-polling loop; all of it for an
   interrupt-driven path, but only with the NIC's interrupts on a core of
   their own.
5. **No page faults:** transparent hugepages off, Aeron's directory on a sized
   2 MiB hugetlbfs, non-sparse term buffers, clients that pre-touch.
6. **Quiet cores:** spin on every core for two minutes and keep busy threads
   off any that stalled. Azure vCPUs stall for milliseconds once a minute.
7. **A disciplined wall clock:** chrony, on PTP where you can; stamp with
   [ergon-clock](../clock/overview.md).
8. **Let the code check:** [ergon-tune](../tune/overview.md) runs at start-up
   in ergon-io's driver and the media driver, and names each missing setting
   with its fix.
9. **Compare only within one run:** two pairs of one instance type differed
   at the median.

The DPDK transport in this repository is a
[proof of concept](../md-dpdk/overview.md).

## Glossary

Network, memory, cloud and measuring terms:
[DPDK chapter's glossary](../md-dpdk/overview.md#glossary). Host,
Kubernetes and Java terms:

| term | what it is | pros / cons, when it matters |
|---|---|---|
| [C-state](hosts.md#c-states) | how deeply an idle CPU sleeps | deeper saves power but wakes later. A spinning core never idles |
| [timer tick, RCU callbacks](hosts.md#kernel-command-line) | kernel work on every CPU: a regular timer interrupt; deferred clean-up | `isolcpus` keeps tasks off, `nohz_full` stops the tick, `rcu_nocbs` moves the callbacks; that cut Azure DPDK's worst percentiles hard |
| [interrupt affinity, `irqbalance`](hosts.md#interrupts-and-the-nic) | the CPUs an interrupt may run on; `irqbalance` is a daemon that moves interrupts around | stop `irqbalance`, pin interrupts to housekeeping, pin again after every NIC reset |
| IOMMU passthrough | IOMMU: the unit that checks which memory a device may reach. Passthrough (`iommu=pt`; Graviton `iommu.passthrough=1`) skips it for the host's own drivers | lets `igb_uio` work on metal; harmless on a VM |
| NUMA node | a group of cores and the memory nearest them | keep a pod's CPUs and devices on one; every `c7i` below `metal-48xl` has only one |
| KSM, NUMA balancing | kernel background work: merging identical pages; moving pages nearer the CPU that uses them | either can stop a busy thread with a page fault; the EKS node turns both off |
| [spin test](hosts.md#cpu-layout) | spin on every CPU for two minutes; note any that stall 300 µs or more | keep busy threads off those: Azure vCPUs stall for milliseconds once a minute |
| [`-march`, `-mcpu`, `target-cpu`](hosts.md#build-for-the-cpu) | compiler options that build for one CPU: `-march`, `-mcpu` (C), `target-cpu` (Rust). On AArch64 only `-mcpu` sets the core | Graviton: without them, gcc calls a library helper for each atomic add |
| `clocksource=tsc`, `tsc=reliable` | boot options: the kernel keeps time with the CPU counter, and `tsc=reliable` turns its runtime checks off | only on hardware measured as the same TSC across cores against a PTP/PHC reference, and never on a VM. The EKS node takes it with `RUSTERON_FORCE_TSC=1` and records the choice ([ergon-clock](../clock/overview.md)) |
| [wall clock, NTP, PTP](../clock/wall-clock.md) | the time of day (UTC), kept right by chrony over the network (NTP) or from a hardware clock (PTP) | pro: matches other hosts and NIC stamps. Con: a correction can step it back; time intervals with the CPU counter |
| [kubelet](kubernetes.md#nodes) | the agent on each Kubernetes node that starts pods and gives them CPUs, memory and devices | it must be told to give whole cores |
| Guaranteed QoS | a pod whose requests equal its limits on every resource | only these, with whole CPUs, get cores of their own from the static CPU manager |
| [SMT (hyper-threading)](kubernetes.md#smt-off) | two logical CPUs on one physical core, sharing its execution resources | a pod with `cpu: 1` shares the core with its twin. Turn SMT off on x86 latency nodes: one CPU is then one core, as on Graviton |
| [CPU quota (CFS quota)](kubernetes.md#no-cpu-quota) | a container's CPU limit as the kernel applies it: at most the quota of CPU time in each 100 ms period | a busy-spinning thread runs at the cap; when it is used up the kernel stops the whole container until the period ends. On kubelet 1.33+ the quota goes away by itself for a container with exclusive CPUs; never turn it off for the whole node |
| static CPU manager, topology manager | kubelet policies: whole cores for Guaranteed pods; a pod's CPUs and devices on one NUMA node | with `full-pcpus-only`, no hyperthread is shared with another pod |
| cpuset | the CPUs a pod may run on | pin each busy thread to its own CPU from it (`Cpus_allowed_list`) |
| DaemonSet | one pod on every matching node | here, one media driver per DPDK node |
| device plugin | a node service that offers devices to pods as a resource | here, ENA pairs bound to `igb_uio` |
| taint, toleration, `nodeSelector` | a taint keeps pods off a node unless the pod tolerates it; a `nodeSelector` sends a pod to labelled nodes | together: only latency pods (and the DaemonSets that must run there) land on latency nodes |
| LimitRange, ResourceQuota | namespace defaults and caps: a `LimitRange` gives every container a default CPU request and limit; a `ResourceQuota` caps the namespace | an unlabelled pod cannot then ask for half a node ([keep other pods in check](kubernetes.md#keep-other-pods-in-check)) |
| PriorityClass | how important a pod is: a higher class preempts a lower one | give latency pods a high class and the rest a low one |
| `hostPath` | a directory of the node, mounted into a pod | here, the node's hugetlbfs for Aeron's directory |
| `hostNetwork` | a pod on the node's network, not the pod network's virtual interface | takes a kernel-UDP pod off that interface |
| [Archive, Cluster](java.md) | Aeron's Java services. Archive records and replays streams; Cluster runs your service on several nodes that agree on one log (the consensus module) | ergon is only a cluster client; these run on Java |
| Agrona | the Java library of buffers and agents that Aeron is built on | its use of `jdk.internal.misc.Unsafe` is why the JVM needs the first `--add-opens` |
| LTS | a Java release with long-term support (17, 21, 25) | run Archive and Cluster on 25 |
| JIT | the JVM's compiler: it turns hot code into machine code while the program runs | code-cache pressure can undo its work (deoptimise) |
| GC, barrier | GC: the garbage collector, which frees objects no longer used. Barrier: the extra work it adds to the program's memory accesses | `ParallelGC` is what every Aeron script uses: convention, not a measured collector A/B |
| safepoint | a pause in which every Java thread stops | every one is logged with its cause to tmpfs (`-Xlog:gc*,safepoint`, [Java](java.md)) |
| duty cycle | one pass of an agent's work loop | `*.cycle.threshold=1ms` counts each one over 1 ms, so stalls show |
| spy | a subscription that reads a publication on its own host, from its log buffer, not the network | see `aeron.spies.simulate.connection` ([Java](java.md#the-aeron-properties)) |
| sparse file, pre-touch | sparse: disk pages appear only on first write. Pre-touch: every page is faulted in at start | no first-write page faults, but pre-touched terms count against a container's memory limit |
