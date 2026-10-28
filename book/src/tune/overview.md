# ergon-tune

ergon-tune checks a Linux host's low-latency settings. Each finding says what
is wrong, the exact fix, and its evidence: an archived A/B where one exists
([evidence](#evidence-per-check)), otherwise the kernel's, Kubernetes', Aeron's
or a vendor's documentation and "not measured here".
It changes nothing. ergon-io and the ergon-md-dpdk media driver run the checks
for you ([where](#where-the-checks-run)).

> **AI assistance.** Like the rest of ergon, this crate was written with heavy
> AI assistance ([disclosure](../project/ai-assistance.md)). It is not
> production-ready.

```toml
[dependencies]
ergon-tune = { version = "0.1", features = ["nic"] }   # `nic` adds N1–N4
```

## Glossary

In brackets: the check (T1–N4). Times are in µs.

| term | what it is | pros / cons, or when it matters |
|---|---|---|
| [check](#what-it-checks), [finding](#read-a-finding) | A check tests one host (T1–T19, K1–K7) or NIC (N1–N4) setting. A finding is a check that fired, with its fix. | Data it cannot read gives *unknown*, never a warning. |
| busy-spinning thread | A thread that loops on one CPU and never sleeps. | Never waits to wake up, but takes a whole CPU. Anything else that runs there pauses it. |
| [hot, housekeeping CPU](../deploy/hosts.md#cpu-layout) | Hot: a CPU a thread busy-spins on. Housekeeping: any other CPU. | Move the kernel's work and interrupts to housekeeping CPUs. |
| p50, p99, p99.9, p99.99, max | The median; the slowest 1 in 100, 1,000 or 10,000; the slowest of all. The high ones are the *tail*. | Host noise shows in the tail and max; the p50 often hardly moves. |
| loop gap | A pause seen by a loop that only reads the clock (the TSC, the CPU's time-stamp counter). | Shows how often something took the CPU, and for how long. |
| interrupt (IRQ) | A device or timer stops the CPU to run a kernel handler. | On a hot CPU, the loop waits for the handler. |
| timer tick | The kernel's regular timer interrupt, `CONFIG_HZ` times a second: 1,000 on the AWS lab's Ubuntu kernel. | `nohz_full` stops nearly all of it on hot CPUs (T1). |
| [`isolcpus`](../deploy/hosts.md#kernel-command-line) | A boot option that keeps other tasks off the listed CPUs. | Alone, the tick stays on. |
| [`nohz_full`](../deploy/hosts.md#kernel-command-line) | A boot option that stops nearly all timer ticks on the listed CPUs while one task runs there. | Use it with `isolcpus` (T1). |
| [`rcu_nocbs`](../deploy/hosts.md#kernel-command-line) | A boot option that moves RCU callbacks (the kernel's deferred clean-up work) off the listed CPUs. | Part of T1's boot line. Not measured alone. |
| [IRQ affinity](../deploy/hosts.md#interrupts-and-the-nic) | The CPUs an interrupt may run on. `irqaffinity` sets the boot default; `/proc/irq/<n>/smp_affinity_list` sets one interrupt's. | NIC interrupts on the loop's CPU cost the tail (T2). |
| [SMT siblings](../deploy/hosts.md#cpu-layout) (hyperthreads) | Two hardware threads that share one core. | A busy sibling slows the loop: measured for ergon-io's SQPOLL thread on the sibling (T10). Graviton, AWS's Arm CPU, has no SMT. |
| real-time scheduling (`SCHED_FIFO`, `SCHED_RR`) | Runs a thread ahead of all normal ones. RT throttling caps it at 0.95 s a second by default. | Its CPU's tick stays on; the latency A/B was inconclusive (T7). |
| [CPU quota (CFS quota)](../deploy/kubernetes.md#latency-checklist) | A cgroup's CPU limit: its threads may run for the quota in each period (100 ms by default). Kubernetes sets one from a container's CPU limit, `docker run --cpus` too. | A busy-spinning thread runs at the cap: when the cgroup's threads use the quota up, the kernel stops all of them until the period ends (T9). |
| throttled, `nr_throttled` | A cgroup stopped because its quota ran out. `nr_throttled` in its `cpu.stat` counts the periods it happened in. | Should not grow for a busy-spinning container. `cpu.stat.local` also counts a parent's (a pod's) quota (T9). |
| [CPU idle states (C-states), frequency](../deploy/hosts.md#c-states) | How deeply an idle CPU sleeps, and how fast it runs. | Deeper sleep saves power but wakes later. Not checked. |
| [transparent hugepages (THP)](../deploy/hosts.md#memory) | The kernel's own 2 MiB pages: `always`, or where the app asks (`madvise`). `khugepaged` builds them in the background. | `khugepaged` works in the background, under `madvise` too (on the regions that asked). Not measured with archived data (T3). |
| Aeron directory | Where Aeron, the messaging library under ergon's transports, keeps its buffers as memory-mapped files. | On a disk filesystem, the kernel writes them back to disk (T4). |
| media driver | Aeron's process that moves messages between the network and the applications, through the Aeron directory. | The ergon-md-dpdk one runs these checks before it starts. |
| tmpfs, [hugetlbfs](../deploy/hosts.md#memory) | Filesystems held in memory. `/dev/shm` is tmpfs; hugetlbfs uses reserved 2 MiB pages. | T4 wants the Aeron directory on one: nothing is written back to disk. |
| NUMA node | A group of CPUs and the memory nearest them. Large hosts have several. | A NIC on another node makes every packet cross between nodes (T6). Every lab host had one: not measured. |
| automatic NUMA balancing | The kernel unmaps pages to see which CPU uses them, and moves them closer. | Each unmap gives a pinned thread a page fault: a stop to map the page back (T5). Not measured. |
| kernel UDP | UDP through the kernel's network stack and sockets. *Plain*: interrupt-driven, no busy polling. | No setup. Against DPDK: [ergon-md-dpdk](../md-dpdk/overview.md). |
| interrupt-driven | Packets reach the socket through the NIC's interrupt, not busy polling. | Each packet waits for an interrupt (and any moderation delay: N1). |
| [NIC receive queue](../deploy/hosts.md#interrupts-and-the-nic) | A lane the NIC puts arriving packets on. Each has its own interrupt. | After every reset, ENA puts queue *i*'s interrupt on CPU *i*, isolated CPUs included (T2). |
| [interrupt moderation](../deploy/hosts.md#interrupts-and-the-nic) (`rx-usecs`) | The NIC waits up to `rx-usecs` µs so one interrupt covers several packets. | Fewer interrupts, later packets (N1). |
| adaptive RX moderation | The driver re-tunes that wait by itself, from a kernel worker. | On AWS that worker pre-empted a busy-polling loop about 18 times a second, and not at all with it off (N1). |
| [busy polling](../deploy/hosts.md#interrupts-and-the-nic) | A socket read polls the NIC queue itself instead of sleeping until an interrupt. | ergon-io's backends: [which backend](../io/backends.md). |
| deferred interrupts (`napi_defer_hard_irqs`, `gro_flush_timeout`) | The kernel keeps the NIC's interrupts off and polls from a timer instead, so a busy poller can take the packets. | With no busy poller, packets wait for the timer (N2). |
| [RX ring](../deploy/hosts.md#interrupts-and-the-nic) | The NIC's buffer for arriving packets: how many it can hold. | Too small drops bursts (N4). |
| NIC reset | The driver restarts the NIC: a device reset, `ethtool -G` or `-L`, a NIC back from DPDK. | On ENA it undoes T2 and N1: re-apply at each link-up. Checks re-run when `carrier_changes` moves. |
| ENA | AWS's virtual NIC (Elastic Network Adapter) and its Linux driver. | N1 checks its adaptive moderation. |
| `ethtool` | Linux's tool for NIC settings: `-C` moderation, `-G` ring sizes, `-L` queue counts. | N1 and N4 read the same settings over netlink (a kernel socket API), without root. |
| accelerated networking (VF) | Azure gives the VM a virtual function (VF): a slice of the real NIC (SR-IOV). | Without it, traffic takes the hypervisor's (Hyper-V's) slower software path (N3). |
| [DPDK](../md-dpdk/overview.md) | A library that drives the NIC from user space, skipping the kernel. | Its gains and costs: [ergon-md-dpdk](../md-dpdk/overview.md#why-bypass-is-faster-and-what-it-costs). |
| [SQPOLL thread](../io/backends.md) | An io_uring kernel thread that runs the ring's requests, so the loop makes no system calls. `sqpoll_cpu` pins it. | It spins, so its CPU is hot: T1 and T8 check it. |
| sysfs, `/proc` | Files under `/sys` and `/proc` where the kernel shows its settings. | Most checks only read these, so they also run on a copied tree (`Host::at`). |
| NAK | An Aeron receiver's request to resend lost data. | NAKs in bursts with kernel busy polling: check the RX ring (N4). |
| open loop | Each message is timed from when it was due, not when it was sent. | A stall counts against every message it delays. |
| `CLOCK_REALTIME` | The kernel's wall clock: UTC stamps, kernel and NIC timestamps. A time daemon disciplines it. | A pod shares the node's: fix time sync on the node (T11). |
| PTP hardware clock (PHC) | A NIC's or hypervisor's clock (`/dev/ptp0`; ENA's `ptp_ena`, Hyper-V's `ptp_hyperv`) that chrony can lock `CLOCK_REALTIME` to (`refclock PHC`). | Tighter than NTP over the network; needs the device on the node (T11). |
| unbound workqueue | Kernel work not tied to one CPU (writeback, reclaim, driver work), run by `kworker/u` threads on the CPUs in `/sys/devices/virtual/workqueue/cpumask`. | On a hot CPU it pauses the loop (T13). |
| memory compaction | The kernel moves pages to make large free blocks: on demand, or in the background (`kcompactd`, `vm.compaction_proactiveness`). | A moved page faults on its next touch, locked pages too unless `vm.compact_unevictable_allowed` is 0 (T14). |
| kprobe, ftrace hook, BPF program | Code the kernel runs at a function or event: a probe's handler, a tracer, a loaded BPF program. | Runs on whichever CPU calls the function, hot ones included (T16). |
| KSM | Kernel same-page merging: `ksmd` merges identical pages of memory that asked for it. | A write to a merged page takes a copy-on-write fault (T17). |
| steal | Time the hypervisor ran something else while a vCPU had work (`/proc/stat`). | The loop stands still; the fix is outside the guest (T18). |
| page fault | The thread touched a page the kernel has not mapped: *minor* (no I/O) or *major* (waits for I/O). | Each stops the thread; a warmed, locked loop takes none (T19, `FaultWatch`). |
| clock source | The counter the kernel keeps time with (`current_clocksource`): `tsc`, `arch_sys_counter`, or `hpet`, `acpi_pm`. | Only the counter (or a hypervisor's view of it) is read without a system call (T12). |

## Why check the host

- A busy-spinning thread loops on one CPU and never sleeps. It loses time
  only when something takes that CPU: a timer tick, an interrupt (IRQ), a
  kernel thread, a busy hyperthread on its core.
- Those pauses set the tail: p99.99 (the slowest 1 in 10,000) and the max.
- Each comes from a host setting, and a reboot, a new image or a NIC reset can
  undo it. Check at start-up and again later.

## What it checks

| check | setting | why it matters |
|---|---|---|
| T1 | CPU isolation: `isolcpus`, `nohz_full`, `irqaffinity` | `isolcpus` keeps other tasks off the loop's CPU. `nohz_full` stops its timer tick, an interrupt `CONFIG_HZ` times a second (1,000 on the AWS lab's Ubuntu kernel). `irqaffinity` sends new interrupts elsewhere. `isolcpus` alone leaves the tick on. |
| T2 | NIC interrupt placement | Each NIC receive queue raises interrupts. On the loop's CPU, the kernel runs the handler instead of the loop. |
| T3 | transparent hugepages (THP) | The kernel thread `khugepaged` merges small pages into 2 MiB pages in the background. Its `lru_add_drain_all` ran a kworker on an isolated spin CPU ([nri-shared-cpus lab](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#what-reached-the-exclusive-cpu-and-the-fix)). |
| T4 | the Aeron directory's filesystem | Aeron, the messaging library under ergon's transports, keeps its buffers in memory-mapped files in one directory. On a disk filesystem the kernel writes them back to disk. |
| T5 | automatic NUMA balancing | On a host with several memory nodes, the kernel unmaps pages to see which CPU uses them, so pinned threads take page faults. |
| T6 | the NIC's NUMA node | A NIC on another node makes every packet cross between the sockets. |
| T7 | real-time scheduling | `SCHED_FIFO` or `SCHED_RR` keeps the CPU's tick on. RT throttling (the kernel's cap on real-time threads, 0.95 s of each second by default) can take the CPU away. |
| T8 | SMT siblings, shared CPUs | Two busy threads on one CPU take turns. Two hyperthreads share one core's execution units. |
| T9 | CPU quota (a container's CPU limit) | A cgroup with a quota may use only that much CPU time per period. A busy-spinning thread uses its CPU all the time, so it runs at the cap, and the kernel stops the whole container when the cap is reached. On Kubernetes 1.33+ the kubelet drops the quota itself for a container with exclusive CPUs; never turn it off for the whole node. |
| T10 | SMT (hyperthreading) on, x86 | One Kubernetes `cpu: 2` is one physical core, not two. A busy sibling slows the loop. Turn SMT off on the node: EKS/EC2 `"CpuOptions": {"CoreCount": <the type's cores>, "ThreadsPerCore": 1}` (not on bare metal, or on T2, C7a, M7a, R7a, Apple silicon Mac or Graviton), self-managed `nosmt` or `echo off > /sys/devices/system/cpu/smt/control` before the kubelet (then delete its `cpu_manager_state`), or an Azure size without SMT (Fasv6, Falsv6, Famsv6, Dpsv6, Dplsv6). Graviton and arm64 pass: one vCPU is one physical core. |
| T11 | how `CLOCK_REALTIME` is kept | UTC stamps are only as good as `CLOCK_REALTIME`, and `CLOCK_MONOTONIC` is slewed with it. PTP-grade (chronyd locked to a PHC, or phc2sys) is ok. NTP-disciplined or unsynchronised warns with the `refclock PHC` line for the host's PHC; so do a correction still being slewed out, an error bound over 1 ms and a pending leap second. In a pod the daemons and chronyd's port are usually out of sight: what cannot be seen is unknown, never a warning, but the kernel's own state (`adjtimex`) is read there too. |
| T12 | the kernel's clock source | `tsc` or `arch_sys_counter` is read through the vDSO; `kvm-clock` and Hyper-V's TSC page are too, where the hypervisor marks them stable (info). `hpet` or `acpi_pm` has no vDSO read: every UTC stamp and Aeron clock read is a system call. |
| T13 | the unbound workqueue cpumask | `kworker/u` threads run unbound kernel work on any CPU in `/sys/devices/virtual/workqueue/cpumask`. `nohz_full` keeps hot CPUs out of it at boot; a boot without it, or a later write, puts them back. Warn. |
| T14 | memory compaction | `vm.compaction_proactiveness` above 0 compacts in the background; `vm.compact_unevictable_allowed` 1 moves locked pages too, so `mlockall` does not pin them. A moved page faults on its next touch. Info: compaction on demand still runs. |
| T15 | other processes' tasks on hot CPUs | A task of another process whose `Cpus_allowed_list` includes a hot CPU can be woken there. This process's threads, zombies and kernel threads are not asked. In a pod's own PID namespace the node's tasks cannot be seen: unknown, unless one it can see fires. Warn. |
| T16 | kprobes, ftrace hooks, BPF programs | Each runs inline on the CPU that calls what it is attached to. Read from debugfs, tracefs and `/proc/kallsyms`; unprivileged, or with `bpf_jit_harden` on, these are unreadable or hide BPF programs: unknown. Info. |
| T17 | KSM | `/sys/kernel/mm/ksm/run` 1: `ksmd` merges pages, and a write to a merged page takes a copy-on-write fault. Warn. |
| T18 | steal on hot CPUs | `/proc/stat`'s steal since boot: the hypervisor ran something else while the vCPU had work. 0 also when the hypervisor does not report steal. Info. |
| T19 | page faults on a hot thread | `FaultWatch` reads the thread's `minflt` and `majflt` again while it runs. Any fault since the last check warns. |
| K1 | the pod's `QoS` class | Anything but Guaranteed gets a CPU limit and shares CPUs; the loop runs at the cap or with a neighbour. Fix (pod spec): `resources.requests` = `resources.limits`. |
| K2 | exclusive CPUs | The shared pool means other pods land on the loop's CPU. Fix (node): kubelet `cpuManagerPolicy: static` (with `full-pcpus-only` on x86); fix (pod spec): whole-CPU requests in a Guaranteed pod. |
| K3 | the loop's SMT sibling | A sibling in another pod is a busy neighbour (the T10 numbers). Fix (node): turn SMT off; fix (pod spec): ask for whole cores so both siblings come together. |
| K4 | a CPU for the helper threads | One CPU means the handshake and the warning thread share the loop's core. Fix (pod spec): `cpu: 2`, or name `ERGON_SHARED_CPUS`/`ERGON_HELPER_CPUS`. |
| K5 | capabilities | Busy polling needs `CAP_NET_ADMIN`, `SCHED_FIFO` `CAP_SYS_NICE`, `mlockall` `CAP_IPC_LOCK` (or `RLIMIT_MEMLOCK`). Warns only for what the caller says it uses (`Needs`). Fix (pod spec): `securityContext.capabilities.add`. |
| K6 | `memory.high`, CPU/memory pressure | At `memory.high` the kernel reclaims and throttles the cgroup. Any non-zero PSI `avg10` warns: no threshold is measured. Fix (pod spec): drop `memory.high` (set requests = limits); fix (node): less load on the node. |
| K7 | the NIC's NUMA node | Same as T6, for a pod. Runs only when the caller names the NIC (`Needs::with_nic`). Fix (node): kubelet `topologyManagerPolicy: single-numa-node`; fix (pod spec): ask for CPU and device together. |
| N1 | ENA adaptive RX moderation | Moderation makes the NIC wait so one interrupt covers several packets. ENA re-tunes that wait from a kernel worker on the CPU that polls the queue: a busy-polling loop's own CPU. |
| N2 | deferred interrupts | `napi_defer_hard_irqs` and `gro_flush_timeout` hold interrupts back so a busy-polling socket (one whose reads poll the NIC queue) can take the packets. With no busy poller, packets wait for the timer. |
| N3 | Azure accelerated networking | Without its VF (the SR-IOV virtual NIC), traffic takes Hyper-V's slower software path. |
| N4 | NIC receive (RX) ring | How many packets the NIC can hold. Too small, and it drops packets in bursts. |

Not checked: CPU idle states and frequency.

## Symptom, check, fix

| symptom | check | fix |
|---|---|---|
| tens of thousands of loop gaps a minute; about 1,000 timer interrupts a second on the loop's CPU | T1 | boot with `isolcpus=nohz,domain,managed_irq,<hot> nohz_full=<hot> rcu_nocbs=<hot> irqaffinity=<housekeeping>` |
| hundreds of loop gaps a minute; burst p99 up | T2 | `echo <housekeeping> > /proc/irq/<n>/smp_affinity_list`, again after every NIC reset |
| THP `always` or `madvise` | T3 | `echo never > /sys/kernel/mm/transparent_hugepage/enabled`, or boot with `transparent_hugepage=never` |
| Aeron directory on ext4, xfs or overlayfs | T4 | `/dev/shm` (in a pod: a memory-backed `emptyDir` sized for Aeron's files), or a sized 2 MiB hugetlbfs with `AERON_FILE_PAGE_SIZE=2097152`: on the node `mount -t hugetlbfs -o pagesize=2M,size=1G none /mnt/huge`, in a pod the [nri-latency-pods](../deploy/latency-pods.md) plugin's `$ERGON_HUGEPAGES_DIR` |
| page faults on pinned threads; several NUMA nodes | T5 | on the node: `sysctl -w kernel.numa_balancing=0` |
| NIC and hot CPUs on different nodes | T6 | pin the threads to the NIC's node, or use a NIC on theirs |
| a real-time thread on the loop's CPU | T7 | `chrt --other -p 0 <tid>`, on an isolated CPU |
| two busy threads on one CPU or core | T8 | pin them to separate cores; leave the sibling idle |
| stalls of milliseconds in a container; `nr_throttled` in `/sys/fs/cgroup/cpu.stat` grows | T9 | Kubernetes 1.33+: `cpuManagerPolicy: static`, a Guaranteed pod and whole-number CPU requests, so the kubelet drops the quota itself. Never `cpuCFSQuota: false`: it removes every pod's CPU limit on the node ([checklist](../deploy/kubernetes.md#latency-checklist)) |
| a `cpu: 2` pod runs like one core; p50 up with work on the loop's sibling | T10, K3 | turn SMT off on the node (EKS `CpuOptions: ThreadsPerCore: 1`, `nosmt`, or an Azure size without SMT) |
| a Burstable pod, the shared CPU pool, or a CPU for helpers missing | K1, K2, K4 | Guaranteed with requests = limits; the kubelet's static CPU manager; `cpu: 2` |
| AWS: p99.99 high with a busy-polling loop | N1 | `ethtool -C <if> adaptive-rx off rx-usecs 0`, again after every reset |
| packets on sockets that do not busy-poll held for `gro_flush_timeout` | N2 | `echo 0 > /sys/class/net/<if>/napi_defer_hard_irqs; echo 0 > /sys/class/net/<if>/gro_flush_timeout` |
| Azure: VF missing or down | N3 | node-level. AKS: a node-pool VM size with accelerated networking. One VM: `az vm deallocate`, then `az network nic update --name <NIC> --resource-group <group> --accelerated-networking true`, then `az vm start` |
| NIC drops and NAKs in bursts with kernel busy polling | N4 | `ethtool -G <if> rx 4096` (on ENA this resets the NIC: re-pin, turn adaptive-rx off again) |
| fine after boot, worse later | T2, N1 | an ENA reset undid them: re-apply from a link-event hook |
| cross-host one-way latencies drift or jump | T11 | on the node: chrony `refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer` (AWS, ENA PHC enabled) or `refclock PHC /dev/ptp_hyperv poll 3 dpoll -2 offset 0` (Azure) |
| clock reads slow; `current_clocksource` is `hpet` | T12 | find why the kernel left the counter: `dmesg \| grep -i clocksource`. Force `tsc=reliable` only on hardware qualified across cores, never on a VM |
| `kworker/u` threads on the loop's CPU | T13 | on the node: `echo <housekeeping mask> > /sys/devices/virtual/workqueue/cpumask`, or boot with `nohz_full` |
| page faults on a pinned, locked thread | T14, T17, T19 | on the node: `sysctl -w vm.compaction_proactiveness=0 vm.compact_unevictable_allowed=0`; `echo 2 > /sys/kernel/mm/ksm/run`; in the process: `mlockall` and touch every buffer before the loop |
| another program's threads on the loop's CPU | T15 | boot with `isolcpus=domain,<hot>`; systemd `CPUAffinity=<housekeeping>`; `taskset -apc <housekeeping> <pid>`; kubelet `reservedSystemCPUs` |
| stalls no host setting explains | T16, T18 | `bpftool prog show` and `/sys/kernel/debug/kprobes/list` (root) for the owners of instrumentation; for steal, an instance whose vCPUs are not shared, or bare metal |

Each finding carries the fix for that host, with its real CPU lists and IRQ
numbers. T1 with the loop on CPU 2 of 8:
`isolcpus=nohz,domain,managed_irq,2 nohz_full=2 rcu_nocbs=2 irqaffinity=0-1,3-7`.

## Evidence per check

Each finding's `evidence` field names its source. The A/B tables are in
`io/benches/results/<run>/tables.md` (or `tables-local.md`). All are AWS
`c7i` in Tokyo; each figure in a finding keeps its max.

| check | evidence | status |
|---|---|---|
| T1 | runs `flags-aws-20261014T163114Z`, `flags-aws-20261015T045527Z` (boot profiles, one 120 s gap-spinner sample each) | measured |
| T2 | run `flags-aws-20261015T062428Z` (`irqlocal`, two reps); ENA's driver source for the IRQ placement | measured |
| T7 | run `flags-aws-20261015T070339Z` (`rt`, two reps): tick measured; latency inconclusive | measured |
| T8 | two threads on one CPU: not measured here; a busy sibling: T10's run | partly measured |
| T10, K3 | run `flags-aws-20261015T122127Z` (SQPOLL on the loop's sibling, three rounds); other sibling loads not measured | measured |
| N1 | runs `flags-aws-20261015T070339Z` (`base`, `nodim`, `nocoalesce`, two reps each) | measured, x86 busy polling only |
| T3, T4, T5, T6, T9, T11–T19, K1, K2, K4–K7, N2, N3, N4 | kernel, Kubernetes, Aeron, chrony, AWS or Azure documentation | not measured here |

## Where the checks run

| component | checks | when | switch |
|---|---|---|---|
| ergon-io's `Driver` (`Component::Io`) | T1 and T8 on the loop's CPU (and `sqpoll_cpu`); T7 on the calling thread; T9, T10; T3 as info; K1–K6 inside a pod (K7 needs a named NIC, which ergon-io does not pass) | `Driver::new`; read them back with `Driver::host_report()` | `ERGON_HOST_CHECKS`: `on` (default), `off` (no warning line; the results stay) or `require` (a warning, or an unproven proof check, stops the start) |
| ergon-io's `Driver` | T2, N1, N2 (interrupt-driven backends only), N3 on each connection's egress NIC | after the first route lookup to it; again after a NIC reset | `quickack_check=off` skips them |
| ergon-io's `Driver` | T19 on the thread creating the driver (taken to be the polling thread, once per thread) | every 5 s, from its warning thread | `ERGON_HOST_CHECKS=off` drops the warning lines |
| the ergon-md-dpdk media driver (`Component::Md`) | T1, T3, T4, T5, T7, T8, T9, T10; T2 and T6 on the data NICs; on the socket transport also N1, N2, and N4 with busy polling | before the driver starts; N1 and T2 again after a NIC reset, on its `md-host-checks` thread | `ERGON_HOST_CHECKS`: `on`/`warn` (default), `off` (no check runs at all) or `require` (any warning or unproven proof stops the start) |
| your own busy loop (`Component::Rt`) | any of them: T1, T3, T4 (the directory it connects to), T5, T7, T8, T9, T10, T11, T12, T13–T18; K1–K7 inside a pod; on a kernel NIC also T2, N1, N2, N3; `ThrottleWatch`, `ClockWatch`, `FaultWatch` | before the loop; the NIC checks again when the NIC's `carrier_changes` moves; `ThrottleWatch::check`, `ClockWatch::check` and `FaultWatch::check` every few seconds from housekeeping | none: skip a check by not calling it |

Neither ergon-io nor the media driver runs T13–T18 yet: call them yourself.

Every printed warning line ends with `(ERGON_HOST_CHECKS=off silences host-check
warnings)`. The K checks run only inside a pod (the `kubepods` cgroup path, the
service-account mount, or `KUBERNETES_SERVICE_HOST`) and each says whether its
fix is in the pod spec or on the node.

`require` is a readiness gate, not a blanket refusal. It refuses on a warning
(a check fired) and on a check that could not **prove** a property of this pod
or process — its CPUs are its own, they are isolated, there is no CPU quota,
its helpers have a core — because starting on an unproven claim is the false
pass those checks exist to stop. An unknown of a check whose subsystem the
node may not have (PSI and `memory.high`, a NUMA node, a NIC knob, THP) is
printed and allowed through: that is a real host, and refusing there would
stop every start on it. `unknown_stops_require(id)` says which: T3, T4, T5,
T6, T11, T13-T21, K6, K7, N1-N4 and M4 do not block; every other id does, a
caller's own included.

Two consequences under `require`:

- **T10** warns on every x86 host with SMT on, so `require` refuses it,
  including the SQPOLL-on-sibling layout T8 accepts.
- **K2** proves exclusive CPUs only from an isolated or root cpuset
  partition, or a matching `cpuset.cpus.exclusive`. The kubelet's static CPU
  manager writes neither, so a stock Guaranteed pod is K2 unknown and
  refused. The [nri-latency-pods](../deploy/latency-pods.md) plugin's
  isolated partition (Linux 6.7+) passes it.

Limits:

- **ergon-io** needs the loop's CPU: pin the thread that calls `Driver::new`
  to one CPU, or set `loop_cpu`. Otherwise it skips T1, T8 and T2, and says so.
- **The media driver** checks the socket transport's NICs only if
  `RUSTERON_MEDIA_DRIVER_DATA_NICS` names them. It adds its own checks, M1–M4
  ([ergon-md-dpdk](../md-dpdk/overview.md)).
- Both print each finding once on stderr, never from a busy thread.
- **In a pod** the NIC checks report unknown: run them on the node
  ([Kubernetes](../deploy/kubernetes.md)). T9 reads the container's own
  cgroup; the pod's quota shows only in `cpu.stat.local` (Linux 6.6+), which
  `ThrottleWatch` reads.
- **In CI**, run them on a captured `/sys` and `/proc` tree: `Host::at(dir)`.

## Call them yourself

The checks are plain functions. They read files and allocate: call them before
the loop or from housekeeping, never inside the loop.

- `Host::system()` is the running host. `Host::at(dir)` is a directory that
  stands for a host's `/`, on any OS.
- `HotSet` lists the CPUs your threads busy-spin on, each with a `Role`
  (`Loop`, `Sender`, `SqPoll`, …), so a message names the thread.
- `Component` (`Io`, `Md`, `Rt`) is the caller. It sets T3's severity: warn
  for `Md` and `Rt`, info for `Io`.

```rust,ignore
{{#include ../../../tune/tests/live.rs:run_all}}
```

The NIC checks need the `nic` feature. Keep one `EthtoolCache::new()` for the
process: it asks the kernel once per interface, and again only after a reset
(when `carrier_changes` moves). Here `i` is an interface name:

```rust,ignore
{{#include ../../../tune/tests/live.rs:nic_checks}}
```
*(From `tune/tests/live.rs`, which runs every check against the host it runs
on: `cargo test -p ergon-tune --all-features`.)*

| function | returns | cost |
|---|---|---|
| `t1_isolation(&host, &hot)` | one finding per missing setting per CPU | a few sysfs reads |
| `t2_nic_irqs(&host, &nic, &hot)` | one finding per IRQ that may fire on a hot CPU | one directory and two files per IRQ |
| `t3_thp(&host, component)` | one finding | a few sysfs reads |
| `t4_aeron_dir(&host, path)` | one finding | one `statfs`; unknown on a tree |
| `t5_numa_balancing(&host)` | one finding | a few sysfs reads |
| `t6_nic_node(&host, &nic, &hot)` | one finding per PCI device | a few sysfs reads |
| `t7_sched_policy(&host, thread, role)` | one finding | one `/proc` read; `Thread::Current` from the thread that will spin, or `Thread::Task(tid)` |
| `t8_smt_siblings(&host, &hot)` | one finding per shared core | a few sysfs reads |
| `t9_cpu_quota(&host)` | one finding | `/proc/self/cgroup` and a few cgroup files |
| `ThrottleWatch::open(&host)`, then `.check()` | `open`: the watch, before the loop; `check`: a finding when the cgroup was throttled since the last check | `check`: one read of `cpu.stat.local`, which `open` opened, and of `cpu.stat` only after that grew (its read takes a kernel lock); no allocation unless it reports; call it from housekeeping |
| `n1_adaptive_rx`, `n4_rx_ring(&host, ifname, &mut cache)` | one finding | one netlink exchange per interface per reset (and one family lookup on first use), each with 100 ms send and receive timeouts; unprivileged |
| `n2_deferred_irqs(&host, ifname, busy_polls)` | one finding per deferring device | sysfs; `busy_polls` is `Some(true)` if your sockets busy-poll, `None` if not known |
| `n3_azure_vf(&host, ifname)` | one finding | sysfs |
| `t11_time_sync(&host, &probe)` | the verdict's finding, plus one each for an unsettled clock, a wide bound, a pending leap second | `adjtimex`, a `/proc` scan, `/sys/class/ptp`, ClockBound's segment, and a chronyd query of at most 50 ms; the pure step is `TimeSync::from_observed` |
| `t12_clocksource(&host)` | one finding | two file reads |
| `ClockWatch::open(&host, &probe)`, then `.check()` | `open`: T11 and T12, and the watch, before the loop; `check`: T11's findings again when its judgement changed, T12's when the clock source did, else none | `check`: one `adjtimex`, one non-blocking receive and send to chronyd (its answer used one check late), ClockBound's segment and the clock source read again; never waits; no allocation unless a finding changed; `update(sample)` judges an injected sample |
| `t13_workqueue_cpumask(&host, &hot)` | one finding | one sysfs read |
| `t14_compaction(&host)` | one finding | two `/proc/sys` reads |
| `t15_host_tasks(&host, &hot)` | one finding, naming up to six processes | a `/proc` scan: two files per task on the host |
| `t16_instrumentation(&host)` | one finding | debugfs, tracefs and `/proc/kallsyms` (a few MB) |
| `t17_ksm(&host)` | one finding | one or two sysfs reads |
| `t18_steal(&host, &hot)` | one finding | one read of `/proc/stat`; `cpu_steal(&host)` gives each CPU's counts, to diff two readings |
| `FaultWatch::open(&host, thread, role)` (or `open_task(&host, pid, tid, role)`), then `.check()` | `open`: the watch, before the loop; `check`: a warning when the thread faulted since the last check | `check`: one `pread` of the thread's `stat`, which `open` opened; no allocation unless it reports. Open it with `Thread::Current` on the hot thread itself, or by thread id from anywhere |

A `Nic` is `Nic::Interface("ens5")` or a DPDK port, `Nic::Pci("0000:00:06.0")`.
The readers underneath (`thread_affinity`, `carrier_changes`, `nic_irqs`,
`busy_poll_sysctls`, `fs_magic`, `cpu_cgroup`, `Platform::detect`) are public
too.

## Read a finding

| field | meaning |
|---|---|
| `id` | the check, a string: `"T4"`, `"K2"`, `"M1"`, or a caller's own |
| `status` | `Finding`: it fired. `Ok`: fine, or it does not apply here. `Unknown(reason)`: the data could not be read. |
| `severity` | `Warn` or `Info`: how to log a `Finding` |
| `message` | what was found, its measured effect, and the fix |
| `evidence` | where the numbers come from |

`Display` prints one line, `T4 warn: <message> (evidence: <where>)` or
`T4 unknown: <message>: <reason>`:

```rust,ignore
{{#include ../../../tune/tests/host_checks.rs:one_line}}
```

- **Warn**: fix it.
- **Info**: real but minor, such as THP for ergon-io or an IRQ whose new
  affinity takes effect at its next interrupt.
- **Unknown** is never a warning: a pod's network namespace, `hidepid`, an
  interface with no device, not Linux. Log it as info.
- **To make warnings fatal**, use the media driver's `require`, or
  `is_warning()` in your own start-up.

## Alternatives

| tool | good at | short of | use it |
|---|---|---|---|
| `tuned` profiles (`cpu-partitioning`, `latency-performance`) | applying isolation, IRQ and power settings; `tuned-adm verify` checks they stuck | does not know which threads spin, which NIC they use, or what a setting costs; not measured here | to apply settings, with ergon-tune checking the result |
| a gap spinner, or the kernel's `osnoise` and `hwlat` tracers (`rtla`) | measuring noise on each CPU, whatever its cause | says how much, not which setting | to choose quiet cores: on Azure `D8ds_v6` the lab's spinner saw 47 of its 48 stalls of 300 µs or more at one second of the minute (the longest of those 47: 1,083 µs; run `flags-azure-20261014T144146Z`, `tables-local.md` line 519) |
| provisioning scripts alone | setting everything once | drift: ENA re-places queue IRQs whenever an interface comes up (`ena_setup_io_intr` in its driver) | with ergon-tune re-checking |

## Best practices and pitfalls

- **Pin first, then check.** ergon-io takes the loop's CPU from the calling
  thread's affinity.
- **Re-check after every NIC reset.** Whenever ENA brings an interface up (a
  device reset, `ethtool -G` or `-L`, a NIC handed back from DPDK), it puts
  queue *i*'s interrupt on CPU *i*, isolated CPUs included, and turns adaptive
  moderation back on. Re-pin from a link-event hook.
- **N1, busy polling: `adaptive-rx off rx-usecs 0`.** Against `adaptive-rx off`
  alone (one run, two reps): WebSocket p99.99 78–80 against 72–76 µs (max
  233–344 against 354–970), HTTP p99.99 74–81 against 83–116 µs (max 202–227
  against 202–981), market-data p99 7.9–8.6 against 9.3–12.9 µs (max 34–41
  against 25–29). The interrupt-driven path is not A/B'd here.
- **Use THP `never`, not `madvise`.** khugepaged still runs under `madvise`
  (kernel `transhuge.rst`); the cost is not measured here.
- **No CPU quota on a busy-spinning container.** `/sys/fs/cgroup/cpu.max`
  should read `max 100000`, and `nr_throttled` should not grow while it runs
  (T9, `ThrottleWatch`). Set-up: [Kubernetes](../deploy/kubernetes.md#latency-checklist).
- **Graviton** has no SMT, so T8 fires there only when two busy threads share
  one CPU. N2 does not suggest busy polling there.
- **Test your host image.** Build a tree with the files your provisioning
  writes and assert on the findings, as `tune/tests/common` does for five
  hosts.
- **Measure your own host.** The measured effects come from AWS `c7i` runs.

How to apply each setting on AWS and Azure, x86 and Graviton:
[Hosts](../deploy/hosts.md).
