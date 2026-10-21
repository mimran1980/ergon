# AWS and Azure Hosts, x86 and Graviton

How to apply each node setting on a VM or a Kubernetes node. Which settings
have evidence, and the figures: [Kubernetes HFT tuning](hft.md). The labs
(`md-dpdk/benches/cloud/host.sh` and `vm.sh`, `io/benches/scripts/`) and the
EKS node bootstrap (`md-dpdk/deploy/aws-eks/node/`) apply these settings.

Terms: [deployment glossary](overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary).

## Instances

| platform | measured on | notes |
|---|---|---|
| AWS x86 | `c7i.xlarge`, `c7i.2xlarge`, `c7i.4xlarge`, `c7i.metal-24xl` | metal costs 6 times a `c7i.4xlarge` ([cost](benchmarks.md#cost-per-hour)). The shipped EKS driver DaemonSet asks for 6 exclusive CPUs plus 1 reserved: 7 cores, so a `c7i.4xlarge` (8 cores with SMT off) |
| AWS Graviton | `c8g.xlarge`, `c8g.2xlarge` (Graviton4), `c9g.xlarge`, `c9g.2xlarge` (Graviton5) | no SMT: a vCPU is a core |
| Azure x86 | `Standard_D8s_v6`, `D8ds_v6`, `D16s_v6` (Xeon Platinum 8573C), accelerated networking (MANA) | |

- **Put both hosts in one cluster placement group** (AWS) or **proximity
  placement group** (Azure). Not measured here.
- **Compare only within one run and one host pair:** two pairs of one
  instance type can differ.

## Kernel command line

Keep the scheduler, the timer tick, RCU callbacks and new interrupts off
every CPU but housekeeping. `<busy>` is the busy CPUs, `<hk>` housekeeping:

```text
isolcpus=managed_irq,domain,<busy> nohz_full=<busy> rcu_nocbs=<busy> irqaffinity=<hk>
```

**Effect:** [isolation](hft.md#isolation). Kernel reference:
[kernel-parameters](https://docs.kernel.org/admin-guide/kernel-parameters.html),
[NO_HZ](https://docs.kernel.org/timers/no_hz.html). On x86 isolate whole
cores (SMT off, so one CPU is one core).

| platform | also add | why |
|---|---|---|
| AWS x86, bare metal | `iommu=pt` | `igb_uio` (DPDK) does not program the IOMMU; a VM has none |
| AWS Graviton, bare metal | `iommu.passthrough=1` | the same, on Arm |
| EKS DPDK node (x86) | `nosmt transparent_hugepage=never hugepagesz=2M hugepages=<n>`; on a VM also `intel_idle.max_cstate=1 processor.max_cstate=1` | `node/provision-node.sh:310-349`. The C1 cap's evidence is mixed: [C-states](hft.md#c-states) |

`clocksource=tsc tsc=reliable` turns the kernel's TSC checks off. The EKS
node takes it only on bare metal with `RUSTERON_FORCE_TSC=1`, never on a VM
(`provision-node.sh`).

## Interrupts and the NIC

Interrupt moderation means the NIC waits to batch several packets into one
interrupt: less CPU, more latency
([ENA driver](https://docs.kernel.org/networking/device_drivers/ethernet/amazon/ena.html)).

| setting | how | evidence |
|---|---|---|
| every movable interrupt on housekeeping, `irqbalance` stopped | `/proc/irq/*/smp_affinity_list`, `/proc/irq/default_smp_affinity`; `systemctl disable --now irqbalance` | [interrupt placement](hft.md#interrupt-placement) |
| NIC queue interrupts pinned again after every NIC reset | ENA puts each queue's interrupt back on CPU *i* when the interface comes up (reset, `ethtool -G`/`-L`, a NIC back from DPDK). The EKS node re-pins on each link event (`node/irq-affinity.sh watch`) | as above |
| ENA moderation off | `ethtool -C <if> adaptive-rx off rx-usecs 0`; ENA turns adaptive moderation back on when the interface comes up, so re-apply it with the pins | [moderation](hft.md#interrupt-moderation), x86 polling backend only |
| receive ring | `ethtool -G <if> rx 4096` | not measured here |
| global busy polling | `net.core.busy_read`, `busy_poll` | not measured here; it reaches every blocking read on the host. Leave unset; ergon-io busy-polls per socket |

## C-states

A C-state is how deeply an idle CPU sleeps: deeper saves power but wakes
later. A spinning core never idles; the housekeeping core that takes the
interrupts does.

| host | setting | evidence |
|---|---|---|
| x86 VM | cap at C1 (`intel_idle.max_cstate=1`), or leave it | mixed: [C-states](hft.md#c-states) |
| x86 metal | leave it | not measured from committed data |
| Graviton | nothing to set | the OS has no C-states to pick: an idle core waits for an interrupt (`WFI`) |

## Memory

| setting | how | evidence |
|---|---|---|
| transparent hugepages off | `transparent_hugepage=never` on the boot line, or `never` into `/sys/kernel/mm/transparent_hugepage/enabled` | [transhuge](https://docs.kernel.org/admin-guide/mm/transhuge.html); not measured here |
| 2 MiB hugepage pool | `hugepagesz=2M hugepages=<n>` on the boot line, or `vm.nr_hugepages` | DPDK and Aeron's hugetlbfs draw from it. On Graviton use a 4 KiB-page kernel (Debian's cloud-arm64 is) so 2 MiB pages exist |
| Aeron's directory on a sized hugetlbfs | `mount -t hugetlbfs -o pagesize=2M,size=1G nodev /mnt/huge-aeron`; on the media driver `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false` | without `size=` Aeron sees no free space and refuses every log buffer ([hft](hft.md#settings)) |
| `net.core.rmem_max`, `wmem_max` | 16 MiB | needed only when Aeron's socket buffers are raised above the kernel's limit ([hft](hft.md#settings)) |
| automatic NUMA balancing off | `sysctl -w kernel.numa_balancing=0` | not measured here (one-node lab hosts) |

## Time

chrony on the Amazon Time Sync Service (`169.254.169.123`), or on the ENA's
PTP clock. Azure's image runs chrony on `ptp_hyperv`. Configuration:
[time sync](clocks.md#set-up-time-sync).

## CPU layout

One physical core per role. SMT is off on x86 latency nodes
([SMT off](kubernetes.md#smt-off)), so one CPU is one core:

| core | x86 (SMT off) | Graviton (no SMT) |
|---|---|---|
| housekeeping: interrupts, Aeron's conductor, the EAL's main thread, client conductors | CPU 0 | CPU 0 (CPUs 0–1 on 8 cores for ergon-io) |
| application, busy-spinning | one CPU | one CPU |
| media driver sender | one CPU | one CPU |
| media driver receiver | one CPU | one CPU |

- **`SHARED_NETWORK`** (sender and receiver on one thread) saves a core. Its
  latency is not measured from committed data.
- **ergon-io `uring-sqpoll`:** its kernel thread on a CPU of its own beat
  the loop's SMT sibling ([SMT sibling](hft.md#smt-sibling)). With SMT off,
  ergon-io takes another CPU of the pod's own.
- **Name one CPU per core.** `thread_siblings_list` still lists the offline
  twin; use the lowest id of each core. AWS numbers x86 siblings N and
  N + cores, Azure adjacently (0–1, 2–3).

## Build for the CPU

- **x86:** rusteron builds Aeron's C with `-march=native` unless
  `RUSTERON_C_MARCH` names another, so build the media driver on, or for, the
  instance type it runs on. ergon-io's measurements used generic x86-64
  builds.
- **Graviton:** Rust with `RUSTFLAGS=-Ctarget-cpu=neoverse-v2` (Graviton4),
  `neoverse-v3` (Graviton5) or `native`; C with `CFLAGS=-mcpu=...`. Without
  it, Ubuntu's and Debian's gcc call a library helper for each atomic
  (gcc's `-moutline-atomics` default on AArch64). On AArch64 `-march` sets
  only the architecture; `-mcpu` sets the core. Latency effect not measured
  here.

## Check it

[ergon-tune](../tune/overview.md) runs these checks at start-up in ergon-io's
`Driver::new` and the media driver, and prints each finding with its fix.
