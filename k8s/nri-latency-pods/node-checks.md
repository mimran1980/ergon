# Node checks

Every setting a latency node needs, what it is for, and the one line to check
it. A node service writes `/run/ergon/node.json` with these facts so the NRI
plugin and the pod can read them (the NIC's node and PHC name among them). The table's own "how to check" column is how to verify a node.
**A normal pod cannot change any of these** — they are node settings.

## Checklist

| check | value | why | how to check |
|---|---|---|---|
| CPU manager | `static` | exclusive CPUs for the loop | `cat /var/lib/kubelet/config.yaml \| grep -i cpumanager` or the nodeadm `NodeConfig` |
| reserved CPUs | `0,1` on c8g (0 = system/shared, 1 = NIC IRQs) | kubelet, daemons and NIC IRQs stay off the hot cores | `cat /var/lib/kubelet/config.yaml \| grep reservedSystemCPUs` |
| `strict-cpu-reservation` | **off** | on, it empties the shared pool of the reserved CPUs and starves the ingester's host-stats thread and fractional sidecars | `grep -r strict-cpu-reservation /etc/kubernetes/ /var/lib/kubelet/` |
| topology manager | `single-numa-node`, scope `pod` | memory lands on the NIC's NUMA node | `grep -i topology /var/lib/kubelet/config.yaml` |
| `full-pcpus-only` | x86 with SMT only | c8g has no SMT; the option is a no-op there | `grep full-pcpus /var/lib/kubelet/config.yaml` |
| kernel | `>= 6.2` (cgroup v2) | keeps a thread's requested affinity across cpuset changes | `uname -r`; `stat -fc %T /sys/fs/cgroup` |
| cgroup v2 | mounted | the plugin refuses cgroup v1 | `test -f /sys/fs/cgroup/cgroup.controllers` |
| `net.core.rmem_max` / `wmem_max` | 4–16 MiB | Aeron's 2 MiB receive window needs them; without them the window caps and frames drop | `sysctl net.core.rmem_max net.core.wmem_max` |
| `net.core.busy_poll` / `busy_read` | **off** on Graviton | busy polling on Graviton was catastrophic: p50 **4,973 µs**, max **9,175 (8,544–102,826)**, **61,488 NAKs** ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)) | `sysctl net.core.busy_poll net.core.busy_read` |
| `kernel.numa_balancing` | `0` | no automatic page migration under a latency pod | `sysctl kernel.numa_balancing` |
| hugepages | boot `hugepagesz=2M hugepages=256` | `AERON_DIR` on hugetlbfs removes 4 KiB page faults | `grep HugePages /proc/meminfo` |
| `transparent_hugepage` | `never` | THP off cut idle p99.99 **67 → 53** and idle max **546 → 106** µs ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)) | `cat /sys/kernel/mm/transparent_hugepage/enabled` |
| `nohz_full` | every non-reserved CPU | keeps the tick off the loop's core | `cat /sys/devices/system/cpu/nohz_full` |
| `rcu_nocbs` | every non-reserved CPU | RCU callbacks off the hot cores | `cat /sys/devices/system/cpu/rcu_nocbs 2>/dev/null` or `grep rcu_nocbs /proc/cmdline` |
| `isolcpus=managed_irq` | every non-reserved CPU | no stray IRQ on the loop's core | `grep isolcpus /proc/cmdline` |
| `irqaffinity` | the reserved CPUs | NIC IRQs on the kept CPU, never the loop's | `grep irqaffinity /proc/cmdline`; `grep eth0 /proc/interrupts` |
| NIC IRQ CPU | CPU 1 (kept), **not** a shared CPU | IRQs on a busy housekeeping CPU lose frames at 1 M/s, max **10.8 ms** ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md), Graviton only) | `md-dpdk/deploy/aws-eks/node/irq-affinity.sh` status |
| `gro_flush_timeout`, `napi_defer_hard_irqs` | node unit writes `/sys/class/net/<dev>/…` | per-netdev sysfs, not a sysctl | `cat /sys/class/net/eth0/gro_flush_timeout` |
| `/dev/ptp*` | present or not | a clock source only; ENA has no hardware packet stamps | `ls /dev/ptp*` |
| chrony | running, synced | pods read the node's `CLOCK_REALTIME`; chrony on the node keeps it on the PHC ([Clocks](https://mimran1980.github.io/ergon/deploy/clocks.html)) | `chronyc tracking` |
| containerd | `>= 2.2`, NRI on | the plugin's socket | `containerd --version`; `ls /var/run/nri` |
| runc | `>= 1.3.1` | earlier runc passes systemd `CPUAffinity` on to containers | `runc --version` |
| `CONFIG_RT_GROUP_SCHED` | `n` if possible | `y` gates container real-time scheduling; we take no real-time policy (ergon-tune T7 checks nothing is under one) | `zcat /proc/config.gz \| grep RT_GROUP_SCHED` |
| NUMA nodes | record `numactl -H` | c8g.xlarge is expected single-NUMA; not measured, so confirm with the command | `numactl -H` |

## Why the named netns sysctls are node-only

The kernel blocks them in a pod (checked in `net/core/sysctl_net_core.c`):

- `net.core.rmem_max`, `wmem_max`, `rmem_default`, `wmem_default` are per-netns
  but **read-only in a non-init netns** (`netns_core_table_dup` clears the
  write bits). A pod write fails; an NRI `SetLinuxSysctl` on them makes runc
  fail the create.
- `net.core.busy_read`, `busy_poll` live in `net_core_table`, registered
  **only for `init_net`** — host-wide.
- `gro_flush_timeout`, `napi_defer_hard_irqs` are **per-netdev sysfs
  attributes**, not sysctls.

So: the node writes them (`node/netns-sysctls.sh`), the plugin **refuses** a
pod that asks for them and names the node sysctl. `hostNetwork` latency pods
share `init_net` with the host, so a "pod" write there is a host write: still
a node setting.

## The facts file

A systemd unit (EKS: from user data, next to the IRQ watcher) writes
`/run/ergon/node.json`:

```json
{
  "kernel": "6.18.0-...",
  "containerd": "2.3.5",
  "runc": "1.4.3",
  "reservedSystemCPUs": "0,1",
  "sharedCpus": "0",
  "nic_irq_cpu": 1,
  "rmem_max": 16777216,
  "wmem_max": 16777216,
  "numa_nodes": [0],
  "nic_numa_node": 0,
  "ptp": ["/dev/ptp0"],
  "hugepages_2Mi": 256,
  "rt_group_sched": false,
  "strict_cpu_reservation": false,
  "topology_manager": "single-numa-node"
}
```

Pods mount it read-only (`hostPath`, which is not allowed under PSS Baseline —
put the latency namespace under a policy that allows exactly this path).
