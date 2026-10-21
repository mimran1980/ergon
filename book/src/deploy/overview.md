# Deployment

A busy-spinning thread is only as fast as its host. Start with
[Kubernetes HFT tuning](hft.md): one table of every setting, where it goes
(node or pod) and its evidence.

| page | for |
|---|---|
| [Kubernetes HFT tuning](hft.md) | **start here**: the settings table, the measured runs, copy-paste blocks, three checks |
| [AWS and Azure hosts, x86 and Graviton](hosts.md) | how to apply each node setting on a VM or node |
| [Kubernetes](kubernetes.md) | whole cores, no CPU quota, SMT off, the node bootstrap; the EKS manifests |
| [Benchmarks: x86 and Graviton](benchmarks.md) | which platform: latency and cost per hour, side by side |
| [Java for Archive and Cluster](java.md) | the JVM and Aeron settings for Archive and Cluster |
| [Latency pods](latency-pods.md) | the five grants a normal pod cannot make |
| [Shared CPUs](shared-cpus.md) | `k8s/nri-shared-cpus`: exclusive CPUs plus the node's shared ones |
| [Clocks](clocks.md) | which clock to read, and time sync on the node |

The DPDK transport in this repository is a
[proof of concept](../md-dpdk/overview.md).

## Glossary

Network, memory, cloud and measuring terms:
[DPDK chapter's glossary](../md-dpdk/overview.md#glossary). Host,
Kubernetes and Java terms:

| term | what it is | pros / cons, when it matters |
|---|---|---|
| [C-state](hosts.md#c-states) | how deeply an idle CPU sleeps | deeper saves power but wakes later. A spinning core never idles; capping is mixed ([C-states](hft.md#c-states)) |
| [timer tick, RCU callbacks](hosts.md#kernel-command-line) | kernel work on every CPU: a regular timer interrupt; deferred clean-up | `isolcpus` keeps tasks off, `nohz_full` stops the tick, `rcu_nocbs` moves the callbacks ([isolation](hft.md#isolation)) |
| [interrupt affinity, `irqbalance`](hosts.md#interrupts-and-the-nic) | the CPUs an interrupt may run on; `irqbalance` is a daemon that moves interrupts around | stop `irqbalance`, pin interrupts to housekeeping, pin again after every NIC reset |
| IOMMU passthrough | IOMMU: the unit that checks which memory a device may reach. Passthrough (`iommu=pt`; Graviton `iommu.passthrough=1`) skips it for the host's own drivers | lets `igb_uio` work on metal; harmless on a VM |
| NUMA node | a group of cores and the memory nearest them | keep a pod's CPUs and devices on one; every lab host had only one |
| KSM, NUMA balancing | kernel background work: merging identical pages; moving pages nearer the CPU that uses them | KSM merges only memory marked mergeable (`MADV_MERGEABLE`), which Aeron and ergon-io do not mark; NUMA balancing matters only with several NUMA nodes. The EKS node turns both off. Not measured here |
| [spin test](hft.md#settings) | spin on every CPU for two minutes; note any that stall 300 µs or more | a node-level check: a pod cannot choose its cores. Not measured here |
| [`-march`, `-mcpu`, `target-cpu`](hosts.md#build-for-the-cpu) | compiler options that build for one CPU: `-march`, `-mcpu` (C), `target-cpu` (Rust). On AArch64 only `-mcpu` sets the core | Graviton: without them, gcc calls a library helper for each atomic add |
| `clocksource=tsc`, `tsc=reliable` | boot options: the kernel keeps time with the CPU counter, and `tsc=reliable` turns its runtime checks off | it turns the kernel's TSC checks off. The EKS node takes it only on bare metal with `RUSTERON_FORCE_TSC=1`, never on a VM, and records the choice (`node/provision-node.sh`) |
| [wall clock, NTP, PTP](clocks.md#set-up-time-sync) | the time of day (UTC), kept right by chrony over the network (NTP) or from a hardware clock (PTP) | pro: matches other hosts and NIC stamps. Con: a correction can step it back; time intervals with quanta or `Instant` |
| [kubelet](kubernetes.md#nodes) | the agent on each Kubernetes node that starts pods and gives them CPUs, memory and devices | it must be told to give whole cores |
| Guaranteed QoS | a pod whose requests equal its limits on every resource | only these, with whole CPUs, get cores of their own from the static CPU manager |
| [SMT (hyper-threading)](kubernetes.md#smt-off) | two logical CPUs on one physical core, sharing its execution resources | a pod with `cpu: 1` shares the core with its twin. Turn SMT off on x86 latency nodes: one CPU is then one core, as on Graviton |
| [CPU quota (CFS quota)](kubernetes.md#no-cpu-quota) | a container's CPU limit as the kernel applies it: at most the quota of CPU time in each 100 ms period | a busy-spinning thread runs at the cap; when it is used up the kernel stops the whole container until the period ends. On kubelet 1.33+ the quota goes away by itself for a container with exclusive CPUs; do not turn it off for the whole node |
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
| GC, barrier | GC: the garbage collector, which frees objects no longer used. Barrier: the extra work it adds to the program's memory accesses | `ParallelGC` is what Aeron's scripts use: convention, not a measured collector A/B |
| safepoint | a pause in which every Java thread stops | every one is logged with its cause to tmpfs (`-Xlog:gc*,safepoint`, [Java](java.md)) |
| duty cycle | one pass of an agent's work loop | `*.cycle.threshold=1ms` counts each one over 1 ms, so stalls show |
| spy | a subscription that reads a publication on its own host, from its log buffer, not the network | see `aeron.spies.simulate.connection` ([Java](java.md#the-aeron-properties)) |
| sparse file, pre-touch | sparse: disk pages appear only on first write. Pre-touch: every page is faulted in at start | no first-write page faults, but pre-touched terms count against a container's memory limit |
