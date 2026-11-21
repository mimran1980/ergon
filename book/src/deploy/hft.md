# Kubernetes HFT tuning

Every latency setting in this guide, where it goes, and its evidence.
**Traced** links to the nri-shared-cpus lab in this repository. **Not
measured here** means the setting rests on kernel, vendor or Kubernetes
documentation only.
**Node** needs node-level access (boot line, kubelet, root on the host); a
pod cannot set it. **Pod** works from the pod spec or the app.

Terms: [deployment glossary](overview.md#glossary).

This chapter is the list. It is not an image and it is not a second chart.
The shared-CPU grant is [Shared CPUs](shared-cpus.md): public image
`ghcr.io/mimran1980/ergon-shared-cpus:0.1.4` and chart
`oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.4`
(EKS AL2023 hot pods need `kubeletConfigDir`:
[EKS install](shared-cpus.md#use-the-public-chart)).
The refusal of an exec probe on a pod labelled `ergon.io/latency: hot` is
the admission policy in [Latency Pods](latency-pods.md). Hugepages, the
isolated cpuset partition, io_uring seccomp and the NIC PHC are opt-in
grants in that same image. The chart writes `seccomp` and `ptp` as
`"off"`. The kind sample
`k8s/nri-shared-cpus/examples/try` installs the public chart, prints the
shared-CPU grant on an unlabelled pod, and shows the two hot-pod refusals:
the plugin's (no `isolcpus` on kind) and the admission policy's. Not yet
run on kind. It does not measure the rows below.

## Settings

| setting | what it does | where | evidence | apply |
|---|---|---|---|---|
| CPU isolation | keeps tasks, the timer tick, RCU callbacks and new interrupts off the busy CPUs | node | [NO_HZ](https://docs.kernel.org/timers/no_hz.html), `kernel-parameters.txt`; traced: with the quiet-CPU settings below an isolated CPU's max was 4.81 to 5.52 µs ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md)) | boot line `isolcpus=managed_irq,domain,<busy> nohz_full=<busy> rcu_nocbs=<busy> irqaffinity=<housekeeping>` ([hosts](hosts.md#kernel-command-line)) |
| NIC queue interrupts off the loop's CPU, `irqbalance` off, re-pinned after every NIC reset | the interrupt handler does not run on the loop's CPU | node | ENA's driver source (`ena_setup_io_intr`); not measured here | [hosts](hosts.md#interrupts-and-the-nic) |
| ENA moderation off: `adaptive-rx off rx-usecs 0` | one interrupt per packet; no adaptive-moderation work on the polling CPU | node | [ENA driver](https://docs.kernel.org/networking/device_drivers/ethernet/amazon/ena.html), AWS Graviton performance runbook; not measured here | `ethtool -C <if> adaptive-rx off rx-usecs 0`, again after every NIC reset |
| SMT off (x86) | one Kubernetes CPU is one physical core | node | siblings share one core's execution units; not measured here | EC2 `CpuOptions` `ThreadsPerCore: 1`, or `nosmt` ([SMT off](kubernetes.md#smt-off)) |
| static CPU manager, `full-pcpus-only` | Guaranteed whole-CPU containers get CPUs of their own | node | [Kubernetes CPU management](https://kubernetes.io/docs/tasks/administer-cluster/cpu-management-policies/); not measured here | [kubelet block](#copy-paste) |
| `reservedSystemCPUs` = the housekeeping CPUs | system daemons stay off the exclusive CPUs | node | same Kubernetes page; not measured here | [kubelet block](#copy-paste) |
| no CPU quota on a busy container | the kernel never stops the container mid-period | node + pod | [sched-bwc](https://docs.kernel.org/scheduler/sched-bwc.html), Kubernetes PR #127525 (kubelet 1.33+); not measured here (`tune/src/quota.rs` T9) | kubelet 1.33+, static CPU manager, Guaranteed pod, whole-number `cpu`. Not `cpuCFSQuota: false` ([why](kubernetes.md#no-cpu-quota)) |
| Guaranteed QoS, whole-number `cpu` | the only pods the static CPU manager gives exclusive CPUs | pod | Kubernetes CPU management page; not measured here | [pod block](#copy-paste) |
| each busy thread pinned to one CPU of the pod's cpuset | the thread stays on its exclusive CPU | pod | not measured here | `sched_setaffinity` from `Cpus_allowed_list` ([busy-spin loop](busy-spin-loop.md#start-up-order)) |
| an `io_uring` SQPOLL thread on its own CPU | the kernel's polling thread does not share the loop's core | pod | not measured here | pin it to a CPU of the pod's own outside the shared CPUs ([shared CPUs](shared-cpus.md#pinning-by-index)) |
| transparent hugepages `never` | no `khugepaged` (it runs under `madvise` too), whose `lru_add_drain_all` runs a kworker on every CPU with queued pages | node | traced: it was an isolated CPU's only context switch and its 16.25 µs max ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#what-reached-the-exclusive-cpu-and-the-fix)) | `transparent_hugepage=never` on the boot line, or `never` into `/sys/kernel/mm/transparent_hugepage/enabled` at boot |
| fair server off on the isolated CPUs | no 1 s deadline-server timer kicking them (a nohz kick and a reschedule IPI) and keeping their tick on | node | traced, kernel 6.12: 56 kicks a minute with gaps to 14.1 µs, then 0 after ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#what-reached-the-exclusive-cpu-and-the-fix)) | `echo 0 > /sys/kernel/debug/sched/fair_server/cpu<N>/runtime` for each isolated N, at every boot after debugfs is mounted (not persistent; kernel 6.12+). Only where nothing real-time runs on those CPUs: not with `SCHED_FIFO` |
| node events out of trading hours | `lru_add_drain_all` callers (block-device ioctls, `systemctl daemon-reload` and generators, `mlock`, page migration) and BPF loads (systemd units, container starts with seccomp, SSH logins) reach every isolated CPU | node (operations) | traced: a systemd daily timer gave a 40.60 µs max and one context switch ([nri-shared-cpus](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md#remaining-risk-node-events)) | systemd timers moved off trading hours, no pod churn and no SSH logins on the node during trading |
| Aeron directory on a sized 2 MiB hugetlbfs, `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false` | no 4 KiB page faults on the log buffers | node (pool, mount) + driver pod | [hugetlbpage](https://docs.kernel.org/admin-guide/mm/hugetlbpage.html); not measured here. An unsized hugetlbfs reports no free space, which Aeron's storage check refuses (`aeron_fileutil.c:952-959` in rusteron-media-driver 0.2.11) | [hosts](hosts.md#memory); the three variables in the driver pod's env |
| `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true` | a client faults its mapped terms in at start | pod (client) | `aeronc.h:170` (rusteron-client 0.2.11); not measured here | [pod block](#copy-paste) |
| `net.core.rmem_max`, `wmem_max` ≥ Aeron's socket buffers | lets the kernel grant `AERON_SOCKET_SO_RCVBUF` | node | functional, not latency: the driver refuses a window larger than the granted `SO_RCVBUF` (`aeron_driver.c:407-456`, rusteron-media-driver 0.2.11). Aeron's default is 128 KiB (`aeron_driver_context.c:191`); only a raised buffer, such as 2 MiB, needs it | `sysctl -w net.core.rmem_max=16777216 net.core.wmem_max=16777216` |
| automatic NUMA balancing off | no balancing page faults on pinned threads | node | not measured here | `sysctl -w kernel.numa_balancing=0` |
| topology manager `single-numa-node` | a pod's CPUs and devices on one NUMA node | node | Kubernetes topology manager docs; not measured here | [kubelet block](#copy-paste) |
| the CPU that takes the NIC interrupts capped at C1 (`intel_idle`) | an interrupt does not wait for that CPU to wake from C6 | node | [cpuidle](https://docs.kernel.org/admin-guide/pm/cpuidle.html); not measured here. `acpi_idle` (AWS c7a): nothing deeper than C1 to cap. ergon-tune T20 | per CPU `echo 1 > /sys/devices/system/cpu/cpu<N>/cpuidle/state<i>/disable` for every state past C1, or `pm_qos_resume_latency_us` 1; `intel_idle.max_cstate=1` caps every CPU. Write it at every boot |
| C-states capped at C1 on every CPU (x86) | an idle housekeeping core wakes faster | node | not measured here | optional; a spinning core never idles, so it reaches only the housekeeping cores |
| TCP autocorking off | a small send goes out at once, not after the previous one's transmit completes (`TCP_NODELAY` does not stop autocorking) | pod netns, or node for `hostNetwork` | `net/ipv4/tcp.c` `tcp_should_autocork`; not measured here. ergon-tune T21 | `sysctl -w net.ipv4.tcp_autocorking=0`; in a pod `securityContext.sysctls`, allowed by the kubelet's `--allowed-unsafe-sysctls` |
| `SCHED_FIFO` on the loop | real-time priority | pod | [sched-rt-group](https://docs.kernel.org/scheduler/sched-rt-group.html): RT throttling can take the CPU away; not measured here. Not built into the Kubernetes plugins | the app keeps `SCHED_OTHER` ([latency pods](latency-pods.md#deliberately-not-built)) |
| ENA receive ring `rx 4096` | room for bursts | node | not measured here | `ethtool -G <if> rx 4096` |
| global busy polling (`net.core.busy_read`, `busy_poll`) | every socket read on the host spins | node | [NAPI](https://docs.kernel.org/networking/napi.html); not measured here | leave unset: it reaches every socket on the host; busy-poll per socket (`SO_BUSY_POLL`) |
| quiet-core spin test | finds CPUs the platform stalls | node | not measured here. A pod cannot choose its cores | run it on new nodes; cordon or replace noisy ones |
| no exec probes, no `restartPolicy: Always` sidecars on hot pods | no probe process lands on the loop's CPU | pod | an exec starts on any CPU of the cpuset (`k8s/nri-shared-cpus/README.md`, Limits); not measured here | `httpGet`/`tcpSocket`/`grpc` probes; the [admission policy](latency-pods.md#4-admission) enforces it |
| both hosts in one cluster placement group | shorter network path between the pair | node (cloud) | AWS docs; not measured here | the node group's placement group |
| build for the CPU (`-mcpu`, `target-cpu`) | Graviton: inline atomics instead of gcc's outline helpers | build | not measured here | [hosts](hosts.md#build-for-the-cpu) |
| chrony on a PTP clock | `CLOCK_REALTIME` stays close to UTC | node | vendor docs; not a latency setting ([Clocks](clocks.md)) | [time sync](clocks.md#set-up-time-sync) |

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
([busy-spin loop](busy-spin-loop.md#start-up-order)).

## Check it

```sh
# 1. no CPU quota on the busy container
cat /sys/fs/cgroup/cpu.max               # expect: max 100000
grep throttled /sys/fs/cgroup/cpu.stat   # nr_throttled stays flat

# 2. the pod got whole cores of its own
grep Cpus_allowed_list /proc/self/status

# 3. the host: ergon-tune names each missing setting and its fix
ergon-tune --hot <busy CPUs>         # on the node
```

Under `ERGON_HOST_CHECKS=require` a caller of ergon-tune refuses to start
while any check warns or a proof check stays unproven
([ergon-tune](../tune/overview.md#where-the-checks-run)). In a stock
static-CPU-manager pod K2 stays unproven, so `require` refuses it
([checks inside a pod](kubernetes.md#checks-inside-a-pod)).

## Open

- Nothing here measures hugepages, the RX ring, NUMA settings or the
  CPU quota on latency.
- The nri-shared-cpus lab traced its own maxima: after THP `never` and the fair server off, an isolated
  CPU on one Azure Cobalt VM took max 4.81 to 5.52 µs and no context
  switch in 60 s reps, the rest below the guest kernel (the hypervisor,
  3.5 to 5.5 µs) ([benchmarks](https://github.com/mimran1980/ergon/blob/main/k8s/nri-shared-cpus/BENCHMARKS.md)).

## More

| page | for |
|---|---|
| [Kubernetes](kubernetes.md) | the EKS node config, `No CPU quota`, `SMT off`, keeping other pods off the node |
| [Hosts](hosts.md) | the same settings on a VM, and how to apply each |
| [Shared CPUs](shared-cpus.md) | helper threads on the node's housekeeping CPUs (`k8s/nri-shared-cpus`) |
| [Latency pods](latency-pods.md) | hugepages, io_uring, PHC devices on a pod (`k8s/nri-latency-pods`) |
