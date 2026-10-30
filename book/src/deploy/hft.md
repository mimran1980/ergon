# Kubernetes HFT tuning

Every latency setting in this guide, where it goes, and its evidence.
**Measured** links to a committed run below. **Not measured here** means
the setting rests on kernel, vendor or Kubernetes documentation only.
**Node** needs node-level access (boot line, kubelet, root on the host); a
pod cannot set it. **Pod** works from the pod spec or the app.

Terms: [deployment glossary](overview.md#glossary).

This chapter is the list. It is not an image and it is not a second chart.
The shared-CPU grant is [Shared CPUs](shared-cpus.md): public image
`ghcr.io/mimran1980/ergon-shared-cpus:0.1.3` and chart
`oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.3`
(EKS AL2023 hot pods need `kubeletConfigDir`:
[EKS install](shared-cpus.md#use-the-public-chart)).
The refusal of an exec probe on a pod labelled `ergon.io/latency: hot` is
the admission policy in [Latency Pods](latency-pods.md). Hugepages, the
isolated cpuset partition, io_uring seccomp and the NIC PHC are opt-in
grants in that same image. The chart writes `seccomp`, `ptp` and
`schedFifo` as `"off"`. The kind sample
`k8s/nri-shared-cpus/examples/try` installs the public chart, prints the
shared-CPU grant on an unlabelled pod, and shows the two hot-pod refusals:
the plugin's (no `isolcpus` on kind) and the admission policy's. Not yet
run on kind. It does not measure the rows below.

## Settings

| setting | what it does | where | evidence | apply |
|---|---|---|---|---|
| CPU isolation | keeps tasks, the timer tick, RCU callbacks and new interrupts off the busy CPUs | node | measured: [isolation](#isolation) | boot line `isolcpus=managed_irq,domain,<busy> nohz_full=<busy> rcu_nocbs=<busy> irqaffinity=<housekeeping>` ([hosts](hosts.md#kernel-command-line)) |
| NIC queue interrupts off the loop's CPU, `irqbalance` off, re-pinned after every NIC reset | the interrupt handler does not run on the loop's CPU | node | measured: [interrupt placement](#interrupt-placement) | [hosts](hosts.md#interrupts-and-the-nic); EKS: `node/irq-affinity.sh watch` |
| ENA moderation off: `adaptive-rx off rx-usecs 0` | one interrupt per packet; no adaptive-moderation work on the polling CPU | node | measured, x86, polling backend: [moderation](#interrupt-moderation) | `ethtool -C <if> adaptive-rx off rx-usecs 0`, again after every NIC reset |
| SMT off (x86) | one Kubernetes CPU is one physical core | node | measured, one case (SQPOLL on the loop's sibling): [SMT sibling](#smt-sibling) | EC2 `CpuOptions` `ThreadsPerCore: 1`, or `nosmt` ([SMT off](kubernetes.md#smt-off)) |
| static CPU manager, `full-pcpus-only` | Guaranteed whole-CPU containers get CPUs of their own | node | [Kubernetes CPU management](https://kubernetes.io/docs/tasks/administer-cluster/cpu-management-policies/); not measured here | [kubelet block](#copy-paste) |
| `reservedSystemCPUs` = the housekeeping CPUs | system daemons stay off the exclusive CPUs | node | same Kubernetes page; not measured here | [kubelet block](#copy-paste) |
| no CPU quota on a busy container | the kernel never stops the container mid-period | node + pod | [sched-bwc](https://docs.kernel.org/scheduler/sched-bwc.html), Kubernetes PR #127525 (kubelet 1.33+); not measured here (`tune/src/quota.rs` T9) | kubelet 1.33+, static CPU manager, Guaranteed pod, whole-number `cpu`. Not `cpuCFSQuota: false` ([why](kubernetes.md#no-cpu-quota)) |
| Guaranteed QoS, whole-number `cpu` | the only pods the static CPU manager gives exclusive CPUs | pod | Kubernetes CPU management page; not measured here | [pod block](#copy-paste) |
| each busy thread pinned to one CPU of the pod's cpuset | the thread stays on its exclusive CPU | pod | the isolation runs pinned every loop; pinning alone is not measured | `sched_setaffinity` from `Cpus_allowed_list` ([io: Kubernetes](../io/kubernetes.md#pin-the-loop-thread-inside-the-pod)) |
| `uring-sqpoll`: the SQPOLL thread on its own CPU | the kernel's polling thread does not share the loop's core | pod | measured at 10 and 100 connections: [SMT sibling](#smt-sibling) | ergon-io picks a CPU of the pod's own outside the shared CPUs ([shared CPUs](shared-cpus.md#pinning-by-index)) |
| transparent hugepages `never` | no `khugepaged` (it runs under `madvise` too), whose `lru_add_drain_all` runs a kworker on every CPU with queued pages | node | traced: it was an isolated CPU's only context switch and its 16.25 µs max ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#what-reached-the-exclusive-cpu-and-the-fix)) | `transparent_hugepage=never` on the boot line, or `never` into `/sys/kernel/mm/transparent_hugepage/enabled` at boot |
| fair server off on the isolated CPUs | no 1 s deadline-server timer kicking them (a nohz kick and a reschedule IPI) and keeping their tick on | node | traced, kernel 6.12: 56 kicks a minute with gaps to 14.1 µs, then 0 after ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#what-reached-the-exclusive-cpu-and-the-fix)) | `echo 0 > /sys/kernel/debug/sched/fair_server/cpu<N>/runtime` for each isolated N, at every boot after debugfs is mounted (not persistent; kernel 6.12+). Only where nothing real-time runs on those CPUs: not with `SCHED_FIFO` |
| node events out of trading hours | `lru_add_drain_all` callers (block-device ioctls, `systemctl daemon-reload` and generators, `mlock`, page migration) and BPF loads (systemd units, container starts with seccomp, SSH logins) reach every isolated CPU | node (operations) | traced: a systemd daily timer gave a 40.60 µs max and one context switch ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#remaining-risk-node-events)) | systemd timers moved off trading hours, no pod churn and no SSH logins on the node during trading |
| Aeron directory on a sized 2 MiB hugetlbfs, `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false` | no 4 KiB page faults on the log buffers | node (pool, mount) + driver pod | [hugetlbpage](https://docs.kernel.org/admin-guide/mm/hugetlbpage.html); not measured here. An unsized hugetlbfs reports no free space, which Aeron's storage check refuses (`aeron_fileutil.c:952-959` in rusteron-media-driver 0.2.11) | [hosts](hosts.md#memory); driver env as in `md-dpdk/deploy/aws-eks/media-driver-config.yaml` |
| `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true` | a client faults its mapped terms in at start | pod (client) | `aeronc.h:170` (rusteron-client 0.2.11); not measured here | [pod block](#copy-paste) |
| `net.core.rmem_max`, `wmem_max` ≥ Aeron's socket buffers | lets the kernel grant `AERON_SOCKET_SO_RCVBUF` | node | functional, not latency: the driver refuses a window larger than the granted `SO_RCVBUF` (`aeron_driver.c:407-456`, rusteron-media-driver 0.2.11). Aeron's default is 128 KiB (`aeron_driver_context.c:191`); only a raised buffer, such as the EKS ConfigMap's 2 MiB, needs it | `sysctl -w net.core.rmem_max=16777216 net.core.wmem_max=16777216` |
| automatic NUMA balancing off | no balancing page faults on pinned threads | node | not measured here (every lab host had one NUMA node) | `sysctl -w kernel.numa_balancing=0` |
| topology manager `single-numa-node` | a pod's CPUs and devices on one NUMA node | node | Kubernetes topology manager docs; not measured here (one-node lab hosts) | [kubelet block](#copy-paste) |
| the CPU that takes the NIC interrupts capped at C1 (`intel_idle`) | an interrupt does not wait for that CPU to wake from C6 | node | measured, AWS c7i, interrupt-driven TCP at 10k msg/s: p50 43–160 → 37–39 µs, p99 315–559 → 44–49 µs ([C-states](#c-states)). `acpi_idle` (AWS c7a): nothing deeper than C1 to cap. ergon-tune T20 | per CPU `echo 1 > /sys/devices/system/cpu/cpu<N>/cpuidle/state<i>/disable` for every state past C1, or `pm_qos_resume_latency_us` 1; `intel_idle.max_cstate=1` caps every CPU |
| C-states capped at C1 on every CPU (x86) | an idle housekeeping core wakes faster | node | VM, polling loop: measured, **mixed** ([C-states](#c-states)); metal: no gain ([hosts](hosts.md#c-states)). Not recommended on this evidence | leave unset beyond the IRQ CPU |
| TCP autocorking off | a small send goes out at once, not after the previous one's transmit completes (`TCP_NODELAY` does not stop autocorking) | pod netns, or node for `hostNetwork` | counted, not timed: c7i corked 49k–164k of ~600k sends a host per point with the IRQ CPU in C6, 42–149 at C1 ([C-states](#c-states)). ergon-tune T21 | `sysctl -w net.ipv4.tcp_autocorking=0`; in a pod `securityContext.sysctls`, allowed by the kubelet's `--allowed-unsafe-sysctls` |
| `SCHED_FIFO` on the loop | real-time priority | pod | measured, **mixed**: [SCHED_FIFO](#sched_fifo). Off by default | `ergon-cpus --fifo` ([latency pods](latency-pods.md#real-time-scheduling-built-default-off)) |
| ENA receive ring `rx 4096` | room for bursts | node | not measured here | `ethtool -G <if> rx 4096` |
| global busy polling (`net.core.busy_read`, `busy_poll`) | every socket read on the host spins | node | ergon-io: not measured as a global sysctl; `recv-busy` sets `SO_BUSY_POLL` per socket instead ([fair run](#interrupt-placement)). Aeron's media driver, kernel UDP: measured, polling not isolated. x86 round 7, with a 4,096 ring, lower p50 than plain kernel UDP (two settings differ); Graviton G1 lost at 1 M msgs/s in 5 of 5 reps (four settings differ) ([BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#measured-but-no-setting-recommended)) | not recommended on this evidence: leave unset. `ergon-md-dpdk` check M4 reports the state as information |
| quiet-core spin test | finds CPUs the platform stalls | node | not measured here: the cloud lab skips CPUs that stalled 300 µs in a 120 s spin (`md-dpdk/benches/cloud/vm.sh:2022-2029`). A pod cannot choose its cores | run it on new nodes; cordon or replace noisy ones |
| no exec probes, no `restartPolicy: Always` sidecars on hot pods | no probe process lands on the loop's CPU | pod | an exec starts on any CPU of the cpuset (`k8s/nri-shared-cpus/README.md`, Limits); not measured here | `httpGet`/`tcpSocket`/`grpc` probes; the [admission policy](latency-pods.md#4-admission) enforces it |
| both hosts in one cluster placement group | shorter network path between the pair | node (cloud) | AWS docs; not measured here | the node group's placement group |
| build for the CPU (`-mcpu`, `target-cpu`) | Graviton: inline atomics instead of gcc's outline helpers | build | not measured here | [hosts](hosts.md#build-for-the-cpu) |
| chrony on a PTP clock | `CLOCK_REALTIME` stays close to UTC | node | vendor docs; not a latency setting ([Clocks](clocks.md)) | [time sync](clocks.md#set-up-time-sync) |

## Evidence

Every row: one run, arms compared only with the same run's baseline. µs.
Raw tables: `git show <commit>:<path>` as named. Hosts: AWS `c7i.2xlarge`
receiver (SMT on), `c7i.xlarge` sender, kernel 7.0; Azure
`Standard_D8ds_v6`, kernel 6.17; Graviton `c8g.2xlarge`
([io/benches/BENCHMARK.md](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md)).
Boot lines and NIC settings per run are in the committed `env-rcv*.txt`
next to each run id under `io/benches/results/`.

### Isolation

A loop that only reads the CPU counter, on the CPU the busy loop uses,
records every gap. One sample per boot line; the loop sends nothing.
Sources: `git show 2517309d:io/benches/results/flags-aws-20261014T163114Z/tables.md`
and `…/flags-azure-20261014T144146Z/tables.md`, "Platform noise".

| host | boot line | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap (max) |
|---|---|---|---|---|---|
| AWS c7i | none | 60,120 | 120.0 | 34.5 | 146.95 |
| AWS c7i | `isolcpus=managed_irq,domain` + `irqaffinity` | 60,063.5 | 4.0 | 0.0 | 52.30 |
| AWS c7i | + `nohz_full`, `rcu_nocbs` | 50.0 | 4.5 | 1.0 | 133.90 |
| Azure | none | 17,189 | 120.0 | 3.0 | 612.94 |
| Azure | `isolcpus=managed_irq,domain` + `irqaffinity` | 10,656 | 47.0 | 0.0 | 58.94 |
| Azure | + `nohz_full`, `rcu_nocbs` | 239.5 | 37.0 | 0.0 | 67.82 |

`isolcpus` cut gaps over 20 µs. Adding `nohz_full` cut gaps over 5 µs
(about 1,000 a second before it, the tick rate it stops:
[NO_HZ](https://docs.kernel.org/timers/no_hz.html)) by 97.7 % (Azure) and
99.9 % (AWS). The longest gap is one event per sample and does not rank the
two isolated lines.

### Interrupt placement

**Graviton** (fair run, `io/benches/results/flags-aws-20261019T175248Z/fair.md`,
committed): WebSocket round trip, interrupt-driven backends (`epoll-spin`,
`mio`, and `uring-spin`, since removed), moderation off everywhere, two
rounds. Six points' senders fell behind; the file's top note names them.

| connections | queue IRQs on the loop's CPU: p99 / max | on a core of their own: p99 / max |
|---|---|---|
| 1 | 45.87–47.17 / 134.98–161.30 | 46.97–48.37 / 107.72–145.15 |
| 4–64 | 52.31–86.46 / 128.56–640.75 | 49.15–53.85 / 135.64–425.45 |

Same file, own-core IRQs, p50: at 1 connection the interrupt backends
read 38.78–39.33 against 36.01–37.04 for NIC polling; at 4 connections the
two families overlap. From 16 connections `recv-busy` (48.98–49.63 at 16,
78.14–78.29 at 64, from-due p99 9.7 s) and `recv-busy-pq` (44.26–44.73) sit
above the interrupt backends (39.27–40.42); `uring-sqpoll` stays level.

**x86** (`git show 2517309d:io/benches/results/flags-aws-20261015T062428Z/tables.md`,
`irqlocal` arm: every queue IRQ on the loop's CPU), `recv-busy` market data,
kernel arrival → decoded:

| arm | p50 | p99 | max |
|---|---|---|---|
| baseline, rounds 1 / 2 | 3.42 / 2.86 | 7.87 / 8.05 | 36.31 / 47.96 |
| IRQs on the loop's CPU, rounds 1 / 2 | 6.49 / 4.68 | 26.09 / 20.11 | 98.61 / 73.64 |

The WebSocket round trip in that run showed no consistent change.

### Interrupt moderation

`git show 2517309d:io/benches/results/flags-aws-20261015T070339Z/tables.md`,
AWS c7i, `recv-busy`, 10,000 round trips a second. Baseline: adaptive-rx on
(`io/benches/scripts/vm-setup.sh` `nodim`). Rounds 1 / 2.

| arm | WebSocket p50 | p99.99 | max | HTTP p99.99 | max |
|---|---|---|---|---|---|
| baseline | 43.90 / 40.65 | 118.85 / 116.53 | 1,023.80 / 259.23 | 118.88 / 77.06 | 445.88 / 432.70 |
| `adaptive-rx off` | 40.88 / 40.93 | 71.89 / 76.02 | 354.05 / 970.07 | 116.06 / 83.44 | 980.65 / 201.50 |
| `adaptive-rx off rx-usecs 0` | 41.90 / 41.55 | 78.50 / 80.41 | 233.02 / 343.68 | 80.99 / 73.78 | 201.65 / 226.79 |

WebSocket p99.99 fell 36–47 µs in both rounds. HTTP p99.99 moved no more
than the 42 µs gap between its own two baselines. One run. The interrupt path with
moderation on is not measured.

### SMT sibling

`git show 2517309d:io/benches/results/flags-aws-20261015T122127Z/tables.md`,
AWS c7i, `uring-sqpoll` market data, kernel arrival → decoded, three rounds:
the SQPOLL kernel thread on its own core against the loop's SMT sibling.

| connections | own core: p50 / p99 / max | loop's sibling: p50 / p99 / max |
|---|---|---|
| 1 | 3.46–3.79 / 8.42–12.92 / 31.57–118.34 | 4.01–4.30 / 6.99–11.69 / 34.72–45.83 |
| 10 | 5.18–6.04 / 20.41–23.81 / 77.03–109.42 | 7.53–8.37 / 29.46–32.77 / 95.85–349.01 |
| 100 | 5.00–5.28 / 20.56–21.98 / 76.01–301.08 | 7.46–7.81 / 32.46–34.17 / 94.51–111.56 |

At 1 connection the own core is lower at p50 and the sibling at p99. At 10
and 100 connections the own core is
lower at p50 and p99 in every round. This is one busy sibling (SQPOLL);
other sibling loads are not measured.

TCP round trips at 1 connection (`c7i.4xlarge`, 2026-10-28): the loop and
SQPOLL on two hyperthreads of one core are 2.3–7.3 µs lower at p50 than mio
on one core of the same hosts (3.8–8.4 against mio with SMT off), and level with SQPOLL on a core of its own to
50,000 msg/s ([`uring-sqpoll` on the loop's SMT sibling](../io/backends.md#uring-sqpoll-on-the-loops-smt-sibling-x86)).
SMT off still keeps one pod's sibling from another pod's work.

### C-states

`…/flags-aws-20261015T062428Z/tables.md`, `cstate` arm: every idle state
deeper than C1 off, AWS c7i VM. Rounds 1 / 2.

| arm | WebSocket p50 | p99.99 | max | HTTP p99.99 | max |
|---|---|---|---|---|---|
| baseline | 36.92 / 37.69 | 115.88 / 104.24 | 454.17 / 967.90 | 112.84 / 106.21 | 1,009.61 / 1,049.79 |
| C1 cap | 39.86 / 39.78 | 133.30 / 128.17 | 464.79 / 1,035.66 | 106.05 / 90.25 | 492.76 / 227.01 |

WebSocket worse at p50 and p99.99, HTTP better at p99.99: no consistent
gain.

The CPU that takes the NIC interrupts is a different case: with
interrupt-driven receive every packet waits for that CPU to wake. Lab
session `aws-20261028-092158` (two `c7i.4xlarge`, `intel_idle`, kernel 7.2,
SMT off, CPU 1 takes every data-NIC interrupt): mio TCP round trips, 1
stream, 10k msg/s, 3 reps, each arm mio-tcp but for one setting
([BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#c7i-the-irq-cpus-idle-depth)).

| arm | p50 µs | p99 µs | CPU 1's idle per 60 s point |
|---|---|---|---|
| every state enabled | 43–160 | 315–559 | C1E 25–52 s, C6 4–28 s |
| no busy poll | 44–165 | 438–545 | C1E 23–49 s, C6 6–30 s |
| capped at C1 | 37.3–39.0 | 44.3–48.9 | C1 55–56 s, nothing deeper |
| never idle (poll) | 37.0–38.2 | 43.6–44.5 | poll 61 s |
| SQPOLL (control) | 34.3–35.6 | 41.7–46.3 | C1E 43–50 s, C6 7–12 s |

p50 follows CPU 1's time in C6 (exit 190 µs). Busy polling off made no
difference, so the cost is the wake, not the poll. ergon-tune T20 reports
an IRQ CPU that may sleep past C1. Autocorking compounds it: a small send
waits for the previous one's transmit completion, which needs the same
CPU awake; those points corked 15k–143k sends a host, the capped ones
42–149. ergon-tune T21 reports autocorking on.

### SCHED_FIFO

`…/flags-aws-20261015T070339Z/tables.md`, `rt` arm: the loop under
`chrt -f 50`, `kernel.sched_rt_runtime_us=-1`. Rounds 1 / 2.

| arm | WebSocket p99.99 | max | HTTP p99.99 | max | loop CPU timer interrupts /s |
|---|---|---|---|---|---|
| baseline | 118.85 / 116.53 | 1,023.80 / 259.23 | 118.88 / 77.06 | 445.88 / 432.70 | 11–23 |
| `SCHED_FIFO` | 119.81 / 73.87 | 579.59 / 1,008.92 | 75.62 / 75.47 | 982.27 / 229.95 | 533–947 |

Mixed. Under the policy the loop's `nohz_full` CPU took 533–947 timer
interrupts a second, against 11–23 without it.

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

**App pod**: Guaranteed, whole cores, Aeron's directory on the node's
hugetlbfs. `AERON_FILE_PAGE_SIZE` and `AERON_TERM_BUFFER_SPARSE_FILE` are
read only by the media driver: set them on the driver's pod.

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
cat /sys/fs/cgroup/cpu.max               # expect: max 100000
grep throttled /sys/fs/cgroup/cpu.stat   # nr_throttled stays flat

# 2. the pod got whole cores of its own
grep Cpus_allowed_list /proc/self/status

# 3. the host: ergon-tune names each missing setting and its fix
#    (runs at start-up in ergon-io's Driver::new and the media driver)
```

`ERGON_HOST_CHECKS=require` (ergon-io and the media driver) refuses to
start while any check warns
or a proof check stays unproven
([ergon-tune](../tune/overview.md#where-the-checks-run)). In a stock
static-CPU-manager pod ergon-io's K2 stays unproven, so `require` refuses
it ([checks inside a pod](kubernetes.md#checks-inside-a-pod)).

## Open

- No committed run measures hugepages, the RX ring, NUMA settings or the
  CPU quota on latency.
- Most maxima above are not traced to a cause. The nri-shared-cpus lab
  traced its own: after THP `never` and the fair server off, an isolated
  CPU on one Azure Cobalt VM took max 4.81 to 5.52 µs and no context
  switch in 60 s reps, the rest below the guest kernel (the hypervisor,
  3.5 to 5.5 µs) ([benchmarks](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md)).

## More

| page | for |
|---|---|
| [Kubernetes](kubernetes.md) | the EKS manifests, `No CPU quota`, `SMT off`, keeping other pods off the node |
| [Hosts](hosts.md) | the same settings on a VM, and how to apply each |
| [Benchmarks](benchmarks.md) | platform and cost side by side |
| [Shared CPUs](shared-cpus.md) | helper threads on the node's housekeeping CPUs (`k8s/nri-shared-cpus`) |
| [Latency pods](latency-pods.md) | hugepages, io_uring, PHC devices on a pod (`k8s/nri-latency-pods`) |
