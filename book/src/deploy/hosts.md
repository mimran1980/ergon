# AWS and Azure Hosts, x86 and Graviton

The labs (`md-dpdk/benches/cloud/host.sh` and `vm.sh`,
`io/benches/scripts/`) and the EKS node bootstrap
(`md-dpdk/deploy/aws-eks/node/`) apply every setting below. Each was chosen
by an A/B. One copy of every figure:
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)
and [`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).

Terms: [deployment glossary](overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary).

## Instances

| platform | measured on | notes |
|---|---|---|
| AWS x86 | `c7i.2xlarge` (4 cores × 2), `c7i.4xlarge` (8 × 2), `c7i.metal-24xl` | Metal has the tightest tails, at 6 times a `c7i.4xlarge`'s price ([cost](benchmarks.md#cost-per-hour)). An EKS DPDK node needs 5 physical cores: `c7i.4xlarge`. Other x86 types were not published |
| AWS Graviton | `c8g.xlarge` (Graviton4), `c9g.xlarge`, `c9g.2xlarge` (Graviton5) | no SMT: a vCPU is a core. No C-states for the OS to manage |
| Azure x86 | `Standard_D8s_v6`, `D8ds_v6`, `D16s_v6` (Xeon Platinum 8573C), accelerated networking (MANA) | some vCPUs stall once a minute, for milliseconds |

- **Put both hosts in one cluster placement group** (AWS) or **proximity
  placement group** (Azure). Even then the pair matters: placement alone
  moved DPDK's idle p50 by about **7 µs** between two rounds on the same
  instance type ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)).
- **Pick the smallest size with enough cores** for the layout below. A
  bigger size buys quieter cores, not a faster network path.

## Kernel command line

Keep the kernel's own work (the scheduler, the timer tick, RCU callbacks,
interrupts) off every CPU except housekeeping (`1-N` here are the busy
CPUs):

```text
isolcpus=nohz,domain,managed_irq,1-N nohz_full=1-N rcu_nocbs=1-N irqaffinity=0
```

**Effect:** on Azure this cut DPDK's worst percentiles hard. Plain kernel UDP
did not change: its tail is interrupts, not scheduling. On x86, isolate whole
cores (SMT is off on latency nodes, so one CPU is one core).

| platform | also add |
|---|---|
| AWS x86 | `iommu=pt`, so `igb_uio` works on metal; harmless on a VM |
| AWS Graviton | `iommu.passthrough=1` instead (metal) |
| Azure | nothing |
| EKS DPDK node (x86) | `transparent_hugepage=never hugepagesz=2M hugepages=<n>`; on a VM also `intel_idle.max_cstate=1 processor.max_cstate=1`. `clocksource=tsc tsc=reliable` only on bare metal with `RUSTERON_FORCE_TSC=1` |

## Interrupts and the NIC

Interrupt moderation means the NIC waits to batch several packets into one
interrupt: less CPU, more latency.

| setting | what it does | measured effect |
|---|---|---|
| every movable interrupt on housekeeping, `irqbalance` stopped, `/proc/irq/default_smp_affinity` the housekeeping mask | keeps interrupts off the busy cores | AWS: a lab left over a million ENA interrupts on isolated CPUs before it re-pinned them |
| pin again after every NIC reset | ENA puts each queue's interrupt back on CPU *i* (mostly isolated) whenever the interface comes up: reset, `ethtool -G`/`-L`, a NIC back from DPDK | the EKS node re-pins within milliseconds of a link event (`node/irq-affinity.sh watch`) |
| ENA, busy-polling loop: `ethtool -C <if> adaptive-rx off` | stops ENA's moderation work running on the polling CPU about 40 times a second | lowers ergon-io's worst percentiles |
| ENA, interrupt path: `ethtool -C <if> adaptive-rx off rx-usecs 0 tx-usecs 0`, **with the NIC's interrupts on a core of their own** | one interrupt per packet, no waiting | the best kernel path on both. Interrupts left on housekeeping: lost frames under load. Graviton: the best kernel path, no lossy rep at 1 M msgs/s in rounds G1 and G3 (one in G2) |
| kernel busy polling: `net.core.busy_read=50`, `busy_poll=50` (Azure also `napi_defer_hard_irqs=2`, `gro_flush_timeout=200000` on the VF) | a socket read spins on the NIC queue instead of sleeping | helps Azure's idle path a lot. Graviton: collapsed at 1 M msgs/s of 288 B, so not there. It reaches every blocking read on the host: dedicated hosts only |
| receive ring: `ethtool -G <if> rx 4096` | room for bursts while the reader catches up | Azure: no NIC drops or NAKs, where the default ring dropped at 1 M msgs/s of 288 B |

ENA turns adaptive moderation back on whenever the interface comes up:
re-apply it together with the interrupt pins.

## C-states

A C-state is how deeply an idle CPU sleeps: deeper saves power but wakes
later. The wake delay is the vendor's C-state exit latency, not measured here.

| host | setting | measured effect |
|---|---|---|
| x86 VM | cap at C1 (`intel_idle.max_cstate=1`), or leave it | mixed. Plain kernel UDP waits for the housekeeping CPU to wake. Capping helped in one round and cost the 288 B loss-free rate in another. The EKS node caps VMs at C1 |
| x86 metal | no cap | C1 made kernel UDP a little slower at the median, with single-rep collapses under load |
| Graviton | nothing to set | the OS has no C-states to pick: an idle core just waits for an interrupt (`WFI`). `nohlt` (idle cores poll instead) is not measured |
| any, busy cores | nothing to set | a spinning core never idles; capping cost ergon-io's receiver a little at the median |

## Memory

| setting | what it does | measured effect |
|---|---|---|
| transparent hugepages off (`/sys/kernel/mm/transparent_hugepage/enabled` = `never`) | stops khugepaged pausing busy threads to merge pages | Azure idle max falls hard |
| 2 MiB hugepages, `vm.nr_hugepages=2048` | backs DPDK's packet buffers and Aeron's directory | required by DPDK. On Graviton use a 4 KiB-page kernel (Debian's cloud-arm64 is) so 2 MiB pages exist |
| Aeron's directory on a sized hugetlbfs: `mount -t hugetlbfs -o pagesize=2M,size=1G nodev /mnt/huge-aeron`, `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false` | no page faults on the log buffers | without `size=`, Aeron sees no free space and refuses every log buffer |
| `net.core.rmem_max`, `wmem_max` 16 MiB | lets the kernel grant Aeron's 2 MiB socket buffers, which the 2 MiB window needs | the EKS lab's first attempts failed without it |

## Time

chrony on the Amazon Time Sync Service (`169.254.169.123`), or on the ENA's
PTP clock in a `precision-time` placement group. Azure's image already runs
chrony on `ptp_hyperv`. Configuration: [time sync](../clock/wall-clock.md).

## CPU layout

One physical core per role, skipping any core a 120-second spin on every CPU
saw stall for 300 µs or more. SMT is off on x86 latency nodes
([SMT off](kubernetes.md#smt-off)), so one CPU is one core there too:

| core | x86 (SMT off) | Graviton (no SMT) |
|---|---|---|
| housekeeping: interrupts, Aeron's conductor, the EAL's main thread, client conductors | CPU 0 | CPU 0 (CPUs 0–1 on 8 cores for ergon-io) |
| application, busy-spinning | one CPU | one CPU |
| media driver sender | one CPU | one CPU |
| media driver receiver | one CPU | one CPU |

- **`SHARED_NETWORK`** puts sender and receiver on one thread and saves a
  core. Not recommended: on Graviton it more than doubled 1,344 B burst
  latency.
- **ergon-io with many sockets:** `uring-sqpoll`, its kernel thread on a
  CPU the pod already has — the loop's SMT sibling or another CPU. Never a
  second isolated full core just for SQPOLL.
- **Name one CPU per core.** `thread_siblings_list` still lists the offline
  twin; the lowest id of each core is the one to use. AWS numbers x86
  siblings N and N + cores, Azure adjacently (0–1, 2–3).

## Build for the CPU

- **x86:** rusteron builds Aeron's C with `-march=native` unless
  `RUSTERON_C_MARCH` names another, so build the media driver on, or for, the
  instance type it runs on. ergon-io's measurements used generic x86-64
  builds.
- **Graviton:** Rust with `RUSTFLAGS=-Ctarget-cpu=neoverse-v2` (Graviton4),
  `neoverse-v3` (Graviton5) or `native`; C with `CFLAGS=-mcpu=...`. Without
  it, Ubuntu's and Debian's gcc call a library helper for every atomic add.
  On AArch64 `-march` sets only the architecture; `-mcpu` sets the core.

## Check it

[ergon-tune](../tune/overview.md) runs these checks at start-up in ergon-io's
`Driver::new` and the media driver, and prints each finding with its fix.
`RUSTERON_MEDIA_DRIVER_HOST_CHECK=require` refuses to start the driver while
any check warns.
