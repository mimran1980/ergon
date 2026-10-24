# ergon-shared-cpus benchmarks

What the [Azure lab](README.md#latency-lab-one-azure-vm) measured, and what
it is ready to measure but has not yet run. Raw files of the measured run:
[`lab/results/azure-20261024212712/`](lab/results/azure-20261024212712/).

## Hot thread with helpers on the shared CPU (run 20261024212712)

A hot pod's spin thread on its isolated CPU, its helpers on the node's one
shared CPU, the plugin on. Five reps of 60 s.

| item | value |
|---|---|
| VM | Azure `Standard_D8ps_v6` (8 vCPUs, Cobalt 100, Neoverse-N2, no SMT), Debian 13, kernel `6.12.113+deb13-cloud-arm64` |
| node | kind (kindest/node v1.35.1, containerd 2.2.1), static CPU manager, `reservedSystemCPUs: 0-1` |
| layout | CPU 0: every NIC IRQ and VMBus channel. CPU 1: `sharedCpus`. CPUs 2-7: `isolcpus=managed_irq,domain` + `nohz_full` + `rcu_nocbs`, exclusive |
| pod | Guaranteed, `cpu: 2`, `ergon.io/shared-millicpus: 1000`; the spin on CPU 2 |
| helper load | `stress-ng --cpu 1 --vm 1 --vm-bytes 128M` and a loopback UDP reader fed 20,000 datagrams/s by a sender, threads of the probe's process |
| probe | [`lab/gapprobe`](lab/gapprobe): every loop iteration into a histogram (exact max), every gap of 1 µs or more logged with its `CLOCK_REALTIME` end; the counter is `CNTVCT_EL0` at 1 GHz |
| sampler | `/proc/interrupts` and `/proc/softirqs` of the spin's CPU every 10 ms, from CPU 0 |

### Result

| rep | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | overlapping the max (growth on CPU 2, 10 ms windows) | involuntary switches |
|---|---|---|---|---|---|---|---|---|
| 1 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 17.40 | arch_timer +1, IPI0 Rescheduling +1, IPI1 Function call +1 | 6 |
| 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 23.47 | arch_timer +1, IPI0 +1, IPI1 +1 | 6 |
| 3 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 18.44 | arch_timer +1, IPI0 +1, IPI1 +1 | 6 |
| 4 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 25.34 | IPI0 +1, IPI1 +1 | 6 |
| 5 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 23.18 | IPI0 +1, IPI1 +1 | 6 |

- Floor: p50 0.02 µs, p99.99 0.03 µs, max 17.40 to 25.34 µs.
- Every max overlaps one rescheduling IPI and one function-call IPI on the
  isolated CPU; in 3 of 5 also an `arch_timer` tick (`nohz_full` keeps a
  residual tick).
- 6 involuntary context switches of the spin thread in every 60 s rep
  (`nonvoluntary_ctxt_switches`). Something preempted it 6 times a minute.
- Gaps of 5 µs or more: 59 to 60 a rep, about one a second. In each rep 58
  or 59 of them overlap an IPI0 and an IPI1 window, 43 to 47 an
  `arch_timer` one too.
- Gaps of 9 µs or more: a pair in the spin's first second (17.4 to 19.6
  µs, then 11.9 to 13.4 µs 2 ms later), then about one every 10.240 s (9
  to 25 µs; rep 4 has one more, 9.3 µs, 1 s after the pair). The max is
  one of these.

What sent those IPIs and what ran in those 6 switches is **not known from
this run**: the sampler counts interrupts, it does not name their sender.
The lab now has a traced rep for that (below).

### e2e placement in the same run

From [`e2e.txt`](lab/results/azure-20261024212712/e2e.txt): the hot pod was
admitted and started with `ERGON_SHARED_CPUS=1` and
`ERGON_ISOLATED_CPUS=2,3`; in three `ps -L` snapshots its spin thread
(`gp-hot`) was on CPU 2 with `Cpus_allowed_list` 2, and every other thread
(`sh`, `gapprobe`, `gp-udp-sink`, `gp-udp-src`, `stress-ng` and its
workers) on CPU 1 with `Cpus_allowed_list` 1. An exec-probe hot pod was
denied.

## Zero context switches on the exclusive CPU: lab ready, not yet run

The target is 0 involuntary switches of the spin thread in a 60 s rep, and
a named cause for every remaining max. The lab changed since run
20261024212712; **none of these changes is measured yet**.

| change | why | where |
|---|---|---|
| a traced rep (`LAB_TRACE=1`, default): one more 60 s plugin-on rep under kernel ftrace | names every switch away from the spin thread (what ran, who woke it, the waker's kernel stack), every IPI and cross-call sent to its CPU (sender CPU, task, callsite, stack), every timer, tick, interrupt and work item there, and ties each to the gap log by `CLOCK_REALTIME`. Output `results/trace-cpu<N>.md`. Its helpers run 25 s before the spin so the tracefs setup (each event enabled patches kernel text, which interrupts every CPU) is done first; a rep armed after its spin started is a failure | `vm.sh` `trace_start`/`trace_stop`, `lib.sh` `lab_trace_events`, `gapprobe trace-report` |
| the probe's gap log and histogram are written before the spin thread pins itself | the old log grew on the hot CPU: one first-touch page fault per 4 KiB (256 gaps), which leaves per-CPU memory state (charge cache, LRU batch, vmstat counters) the kernel settles with work queued on that CPU. The summary now counts the loop's page faults (`minflt`, `majflt`) | `gapprobe` `GapLog`, `Histogram::new` (a Linux unit test fails on 4,096 faults without it) |
| the probe's UDP helpers run in a child process (`--helpers process`, default) | helpers that map or unmap memory in the spin thread's process can send it TLB-flush or membarrier IPIs; a separate process has its own address space | `gapprobe spin --helpers`; `LAB_HELPERS=thread` repeats the old layout |
| node settings (`LAB_QUIET=1`, default) | `vm.stat_interval` 120, `kernel.watchdog_cpumask` the housekeeping CPUs, `kernel.nmi_watchdog` 0, `kernel.numa_balancing` 0, `vm.compaction_proactiveness` 0, `kernel.timer_migration` 1, the writeback workqueue on the housekeeping CPUs: each removes a per-CPU kernel worker or timer of `Documentation/admin-guide/kernel-per-CPU-kthreads.rst`. Which of them mattered is for the trace to show | `lib.sh` `lab_quiet_settings`, recorded in `results/quiet.txt` |
| layout: exclusive CPUs 3-7, `reservedSystemCPUs: 0-2` | one boot line serves one shared CPU (bench) and two (scale); the spin now runs on CPU 3 | `lib.sh` `lab_layout` |

Rule for apps, from the mechanism above (not yet measured): allocate and
touch the hot thread's memory before it pins itself, and keep threads that
map and unmap memory out of the hot thread's process, or have them stop
unmapping once the hot thread runs.

The `nohz_full` residual tick (about 1 Hz with one runnable task) is a
kernel limit; if the trace shows it, it stays and is reported as remaining.

The traced pipeline (tracefs setup, filters, stacks, `trace-report`) has
been run on a Linux 7.0 kernel in a privileged container; not yet on the
lab VM's 6.12.

## Shared-CPU scaling: lab ready, not yet run

The design, from the code (`internal/assign`): a container asking for M
`ergon.io/shared-millicpus` gets `ceil(M/1000)` shared CPUs (at least one),
the least-loaded ones; its cpuset is its exclusive CPUs plus those. M is
scheduler accounting only: no CPU quota is set (a container with one is
refused), so its helpers may use all of their shared CPUs when they are
free. When pods share a shared CPU, the kernel splits it by the containers'
cgroup `cpu.weight`, which comes from their `cpu` request, not from M.

`vm.sh scale` (`LAB_SCALE=1`, default) checks that on two shared CPUs
(`sharedCpus: 1-2`), 30 s a case, 4 helper threads a pod:

| case | pods | expected |
|---|---|---|
| accounting | none, then 2,100 and 2,000 millicpus | allocatable `ergon.io/shared-millicpus` 2000 at oversubscription 1; a pod asking 2,100 stays Pending (Unschedulable); one asking 2,000 schedules |
| a-burst-100m | cpu 1, 100 m, busy helpers | helpers on 1 of the 2 shared CPUs, up to all of it (about 1 core); the other idle; `nr_throttled` 0 |
| b-burst-1001m | cpu 1, 1,001 m, busy helpers | helpers on both shared CPUs: about 2 cores |
| c-idle | cpu 1, 100 m, helpers busy 1 % | near 0.04 cores |
| d-contention | A cpu 1, B cpu 1, C cpu 2, 100 m each, started in turn | A and B on different shared CPUs; C on A's; A and C split it by `cpu.weight` (C's about twice A's), B alone |

Each case records per-CPU busy time of every shared and exclusive CPU
(`/proc/stat`, every task on the VM), each pod's `cpu.stat` (`usage_usec`,
`nr_periods`, `nr_throttled`, `throttled_usec`), `cpu.weight` and `cpu.max`,
of the container's cgroup and of the pod's (a pod-level quota throttles
there, not in the container's), the shared CPUs each pod got, every
thread's CPU and `Cpus_allowed_list` each second with a verdict (the spin
thread on its CPU alone, every other thread on that pod's shared CPUs),
and the spin thread's max. A pod's helper time is its container's usage
less the spin thread's. The step's own sampling runs on the IRQ CPU; the
contention pods spin 30 s longer each, so the first one's helpers run
through the whole window. Output `results/scale.md`.

## Not measured

- How long a helper's wake-up waits on a shared CPU under node load.
- The cost of `pin_isolated` and of reading `ERGON_*`.
- Everything in the two "lab ready" sections above, until a run.

## Reproduce

```sh
DRY_RUN=1 k8s/nri-shared-cpus/lab/azure-vm.sh   # plan, time, price; no cloud call
k8s/nri-shared-cpus/lab/azure-vm.sh             # about 75 minutes, about $0.44
```

`LAB_ARMS="plugin-on plugin-off"` adds the plain static CPU manager arm.
