# Kubernetes HFT tuning

Bold µs rows are measured; the rest are required for whole cores or fail
closed. `X (a–b)` = median of five per-rep maxima
([how to read a number](benchmarks.md#how-to-read-a-number)).
Scan the tables, copy the two blocks, run the three checks.

Terms: [deployment glossary](overview.md#glossary).

## Do this

| where | setting | value | effect |
|---|---|---|---|
| node | CPU isolation | `isolcpus=nohz,domain,managed_irq,1-N nohz_full=1-N rcu_nocbs=1-N irqaffinity=0` | Azure DPDK idle median of rep maxima **216 (69–663) → 71 (65–447)** µs. Biggest single win |
| node | transparent hugepages | `never` | Azure idle p99.99 **67.01 (51.84–87.23) → 53.15 (51.81–82.24)** (round 4); idle median of rep maxima **545.79 (82.37–632.32) → 106.50 (98.11–286.21)** (round 6), absolute idle max **632 → 286** |
| node | SMT off (x86) | `ThreadsPerCore: 1`, or `nosmt` | one Kubernetes CPU is one physical core, as on Graviton. A busy sibling of the loop costs **+0.5 µs p50 at 1 connection, +2.3–2.7 µs p50 and +9–12 µs p99 at 10–100** ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#smt-sibling-cost)) |
| node | static CPU manager | `cpuManagerPolicy: static`, `full-pcpus-only: "true"` | a Guaranteed pod gets whole cores, never one hyperthread shared with another pod |
| node | topology manager | `topologyManagerPolicy: single-numa-node` | a pod's CPUs and devices stay on one NUMA node |
| node | reserved CPUs | `reservedSystemCPUs` = the housekeeping cores | interrupts and system daemons stay off the pods' cores |
| node | hugepages | `hugepagesz=2M hugepages=<n>` on the boot line | no 4 KiB page faults. An **unsized** hugetlbfs fails Aeron's storage check |
| node | `rmem_max`, `wmem_max` | 16 MiB | the kernel will not grant Aeron's 2 MiB socket buffers without it |
| node | KSM, NUMA balancing, `irqbalance` | off | each can stop a busy thread with a page fault, or move interrupts onto a busy core |
| pod | QoS | requests = limits, whole-number `cpu` | only these get cores of their own |
| pod | each busy thread | pinned to one CPU of the pod's cpuset | a thread outside the cpuset never starts (`EINVAL`) |
| pod | Aeron directory | hugetlbfs `hostPath`, `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false` | no first-write page faults on the log buffers |
| pod | SIGTERM | handled | a PID 1 with no handler is SIGKILLed, closing nothing |
| app | ENA adaptive moderation | `ethtool -C <if> adaptive-rx off rx-usecs 0` | **Polling backends:** `recv-busy` p99.99 falls **43–47 µs** (runs `062428Z`, `070339Z`). **Interrupt backends:** with this and the queue IRQs on a core of their own, the interrupt family is within **1.7–3.3 µs** of NIC polling at 1 connection and level with it from 4 sockets, and unlike plain `recv-busy` it does not collapse at 64 (fair run `20261019T175248Z`). Cost of leaving it on, x86: not measured |
| app | NIC queue IRQ placement | the queues' IRQs on a spare core, never on the loop's CPU | Graviton WebSocket round trip, p99 / max (fair run `20261019T175248Z`): IRQs on the loop's CPU **45.9–47.2 / 135–161 µs** at 1 connection, **52.3–86.5 / 129–641 µs** at 4–64; IRQs on a spare core **47.0–53.9 / 108–425 µs** throughout. Placement costs tail, not median |
| app | ENA RX ring | `ethtool -G <if> rx 4096` | Azure: no NIC drop or NAK, where the default ring dropped at 1 M msgs/s of 288 B |
| app | keep-alive | about 1 kHz on sparse, latency-critical streams | a lost frame is noticed only at Aeron's next heartbeat: one lost ping cost **134 ms** |
| app | quiet cores only | two-minute spin at pod start; pin to cores that never stalled 300 µs | some Azure vCPUs stop for up to **3.7 ms** once a minute |

Figures: [md-dpdk/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)
(SMT: [io/benches/BENCHMARK.md](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md);
the two `app` rows: fair run
[`20261019T175248Z`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/flags-aws-20261019T175248Z/fair.md),
whose top note names the six points whose sender fell behind).

## Never do this

| setting | why not | measured |
|---|---|---|
| `cpuCFSQuota: false` | node-wide: every pod loses its CPU limit, including log collectors | a busy-spinning container that hits the quota is stopped for the rest of the 100 ms period |
| plain kernel UDP with no isolation | interrupts and the tick land on the busy cores | Azure idle median of rep maxima **354.81 (272.13–2,203.65)** (absolute 2,203.65), 39 spikes ≥200 µs a rep — against **57.89 (49.47–64.22)** (absolute 64.22) isolated DPDK |
| `SHARED_NETWORK` (driver sender+receiver on one thread) | saves a core, costs the tail | Graviton 1,344 B bursts p50 **522 vs 234**, median of rep maxima **748 (723–886) vs 493 (465–745)**, absolute **886 vs 745** |
| busy polling on Graviton at 288 B, 1 M/s | collapses | p50 **4,973 µs**, median of rep maxima **9,175 (8,544–102,826)** (absolute 102,826), **61,488 NAKs** |
| `mlockall`, pacing, an RX burst of 64, a 1 MiB receive window, C-states at C1 on metal | each lost its A/B | see [not recommended](../md-dpdk/overview.md) |
| `AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD` = the conductor's park | the counter includes the park, so it fires on every idle `nanosleep` | the lab now derives it from the park and refuses a bad value |
| `rcv-wnd=80k` on a 288 B channel | a rate cap, not a loss fix | flat-out falls **6.5–6.8×**, and the 288 B loss-free ceiling **drops** 157 k → 121 k msgs/s |

Figures in this table: [md-dpdk/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

## Copy-paste

**Kubelet** (EKS `NodeConfig`; k3s and AKS: [Kubernetes](kubernetes.md#no-cpu-quota)):

```yaml
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cpuManagerPolicy: static
cpuManagerPolicyOptions:
  full-pcpus-only: "true"
reservedSystemCPUs: "0"          # the housekeeping cores
topologyManagerPolicy: single-numa-node
# cpuCFSQuota stays true: the kubelet drops it for exclusive CPUs (1.33+)
```

**Pod** — Guaranteed, whole cores, pinned:

```yaml
apiVersion: v1
kind: Pod
spec:
  priorityClassName: <your latency class>
  containers:
  - name: app
    resources:
      requests: { cpu: "2", memory: 1Gi, hugepages-2Mi: 1Gi }
      limits:   { cpu: "2", memory: 1Gi, hugepages-2Mi: 1Gi }
    env:
    - { name: AERON_FILE_PAGE_SIZE, value: "2097152" }
    - { name: AERON_TERM_BUFFER_SPARSE_FILE, value: "false" }
    - { name: AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY, value: "true" }
    volumeMounts:
    - { name: huge, mountPath: /mnt/huge-aeron }
  volumes:
  - name: huge
    hostPath: { path: /mnt/huge-aeron, type: Directory }
```

Pin each busy thread to one CPU of the pod's cpuset before the loop starts
([io: Kubernetes](../io/kubernetes.md#pin-the-loop-thread-inside-the-pod)).

## Check it

```sh
# 1. no CPU quota on the busy container
cat /sys/fs/cgroup/cpu.max            # must read: max 100000
grep throttled /sys/fs/cgroup/cpu.stat   # nr_throttled must not grow

# 2. the pod got whole cores of its own
grep Cpus_allowed_list /proc/self/status

# 3. the host is set up — ergon-tune names every missing setting and its fix
#    (runs at start-up in ergon-io's Driver::new and the media driver)
```

`RUSTERON_MEDIA_DRIVER_HOST_CHECK=require` refuses to start the driver
while any check warns.

## Platform notes

| platform | pick | note |
|---|---|---|
| Graviton5 (`c9g`) | kernel UDP: moderation off, data NIC interrupts on a spare core, **no** busy polling | lowest latency measured. Idle p50 **18.93**, median of rep maxima **79 (76–168)**; 288 B at 1 M/s p50 **44.83**, median of rep maxima **204 (122–211)** |
| AWS x86 (`c7i`) | kernel UDP: busy polling, 4,096-entry ring | idle p50 **28.22**, median of rep maxima **1,938 (349–4,846)**. Moderation off with its interrupts on a core of their own is not measured on x86 |
| Azure | kernel UDP: busy polling, 4,096-entry ring | idle p50 **21.15**, median of rep maxima **99.52 (73.86–209.41)**. Some vCPUs stall once a minute — spin-test first |
| any, both ends on one host | localhost | isolated DPDK idle p50 **9.81**, median of rep maxima **31.60 (29.92–32.80)**: a 3 µs spread across five reps |

Figures in this table: [md-dpdk/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

- **One AWS flow tops out at 7.8–8.0 Gbit/s.** A channel is one flow; spread
  a heavier stream over several channels.
- **Run below the loss-free rate,** never at flat-out. Flat-out always loses.
- **Latency figures come with their max.** Every absolute maximum above (a
  bracket's upper end) was investigated; the ones still open are listed in
  [`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

## More

| page | for |
|---|---|
| [Kubernetes](kubernetes.md) | the EKS manifests, `No CPU quota`, `SMT off`, keeping other pods off the node |
| [Hosts](hosts.md) | the same settings on a VM outside Kubernetes |
| [Benchmarks](benchmarks.md) | platform and cost side by side |
| [Shared CPUs](shared-cpus.md) | helper threads on the node's housekeeping CPUs (`k8s/nri-shared-cpus`) |
| [Latency pods](latency-pods.md) | hugepages, io_uring, PHC devices on a pod (`k8s/nri-latency-pods`) |
