# ergon-shared-cpus benchmarks

What one Azure Cobalt VM (8 vCPUs, `isolcpus` boot line, kind on the VM) and
the EKS lane measured. Raw files: [`lab/results/`](lab/results/), one directory per run.

| run | what | files |
|---|---|---|
| 20261024234841 | **current best**: 3 bench reps and a traced rep, after the THP and fair-server fixes | [`azure-20261024234841/`](lab/results/azure-20261024234841/) |
| 20261024233129 | the first working trace (2 bench reps and a traced rep), before the fixes | [`azure-20261024233129/`](lab/results/azure-20261024233129/) |
| 20261024230400 | shared-CPU scaling, then 5 bench reps, before the fixes (its trace failed: tracefs is root-only, fixed since) | [`azure-20261024230400/`](lab/results/azure-20261024230400/) |
| 20261024212712 | the first bench, an older layout (exclusive CPUs 2-7), superseded | [`azure-20261024212712/`](lab/results/azure-20261024212712/) |

## Setup

| item | value |
|---|---|
| VM | Azure `Standard_D8ps_v6` (8 vCPUs, Cobalt 100, Neoverse-N2, no SMT), Debian 13, kernel `6.12.113+deb13-cloud-arm64` |
| node | kind (kindest/node v1.35.1), static CPU manager, `reservedSystemCPUs: 0-2` |
| layout | CPU 0: every NIC IRQ and VMBus channel. CPU 1: `sharedCpus` (bench); 1-2 (scale). CPUs 3-7: `isolcpus=managed_irq,domain` + `nohz_full` + `rcu_nocbs`, exclusive. The spin on CPU 3 |
| node settings | `quiet.txt` of each run; the THP and fair-server lines only in 20261024234841 |
| pod | Guaranteed, `cpu: 2`, `ergon.io/shared-millicpus: 1000` |
| helper load | `stress-ng --cpu 1 --vm 1 --vm-bytes 128M` and a loopback UDP reader fed 20,000 datagrams/s, in a child process of the probe, on the shared CPU |
| probe | [`lab/gapprobe`](lab/gapprobe): every loop iteration into a histogram (exact max), every gap of 1 µs or more logged with its `CLOCK_REALTIME` end; counter `CNTVCT_EL0` at 1 GHz |
| sampler | `/proc/interrupts` and `/proc/softirqs` of the spin's CPU every 10 ms, from CPU 0 |
| trace | one more rep under ftrace filtered to CPU 3: every switch, IPI, cross-call, timer and work item, with the sender's stack, matched to the gap log (`trace-cpu3.md`) |

## Hot thread with helpers on the shared CPU (run 20261024234841)

60 s reps, plugin on. From [`report.md`](lab/results/azure-20261024234841/report.md):

| rep | p50 µs | p99.99 µs | max µs | involuntary switches | page faults | gaps ≥ 3 µs | interrupts on CPU 3 during the spin |
|---|---|---|---|---|---|---|---|
| 1 | 0.02 | 0.03 | 4.81 | 0 | 0 | 2 | 0 |
| 2 | 0.02 | 0.03 | 5.24 | 0 | 0 | 2 | 0 |
| 3 | 0.02 | 0.03 | 5.52 | 0 | 0 | 2 | 0 |
| trace | 0.02 | 0.03 | 40.60 | 1 | 0 | 6 | 1 rescheduling IPI, 2 ticks, 37 function-call IPIs (`trace-cpu3.md`) |

- Gaps ≥ 3 µs counted from `plugin-on-<n>.gaps.csv`; interrupts from
  `plugin-on-<n>.irq.tsv` within each summary's `start_ns`..`end_ns`.
- Before the fixes, same layout: max 15.14 to 22.09 µs, 1 involuntary
  switch a rep, 106 to 116 gaps ≥ 3 µs a rep (run 20261024230400,
  `report.md` and `plugin-on-<n>.gaps.csv`).
- The traced rep's max is a node event, not the steady state: see the
  next two sections.

## What reached the exclusive CPU, and the fix

| cause | evidence | fix | after (run 20261024234841) |
|---|---|---|---|
| `khugepaged` on CPU 0 called `lru_add_drain_all`, which ran `kworker/3` (`lru_add_drain_per_cpu`) for 6 µs: the rep's only involuntary switch and its 16.25 µs max | 20261024233129 `trace-cpu3.md`, "Switches away" and "Work items" | `transparent_hugepage/enabled` `never` | no `khugepaged` work on CPU 3 in the trace; 0 switches in the 3 untraced reps |
| the fair server (kernel 6.12's deadline server, `dl_task_timer` every 1.024 s on a housekeeping CPU): `enqueue_dl_entity` sent CPU 3 a nohz kick and a rescheduling IPI 56 times in 60 s, gaps 6.3 to 14.1 µs, and kept the tick on (`tick_stop dependency=SCHED` 115, `arch_timer` 61) | 20261024233129 `trace-cpu3.md`, "IPIs" and "Timers" | `/sys/kernel/debug/sched/fair_server/cpu<N>/runtime` 0 on CPUs 3-7 (`quiet.txt`: 50000000 -> 0) | 1 rescheduling IPI and 2 ticks in the traced minute, both at the node event below |
| `kick_all_cpus_sync` from the BPF JIT (`__text_poke`): `sshd-auth` installing its seccomp filter at each SSH login of the lab's own polling (6 in the minute), and `bpf_jit_free` kworkers: 18 cross-calls, gaps up to 6.2 µs | 20261024233129 `trace-cpu3.md` | one multiplexed SSH connection (`ControlMaster`) | no `sshd-auth` cross-call in the trace |
| run 20261024212712: 6 involuntary switches a rep, cause not traced (no trace then) | that run's `report.md` | the probe's log and histogram touched before it pins, helpers in a child process, the first node settings | 1 a rep, 0 page faults (run 20261024230400 `report.md`) |

## Remaining risk: node events

The traced rep of run 20261024234841 fell at 00:00:09 UTC, when systemd's
daily timers ran (`trace-cpu3.md`, `trace-1.trace.txt.gz`):

| event | path to CPU 3 | cost |
|---|---|---|
| systemd ran its generators; `systemd-gpt-auto-generator`'s block-device ioctl (`blkdev_common_ioctl` → `invalidate_bdev`) called `lru_add_drain_all` | `kworker/3` ran `lru_add_drain_per_cpu` for 20 µs | the 40.60 µs max, the rep's 1 involuntary switch |
| systemd (pid 1) loaded 12 BPF programs (`bpf_prog_load` → JIT → `kick_all_cpus_sync`), and kworkers freed old ones | 36 cross-calls | gaps up to 3.6 µs |

Anything on the node that calls `lru_add_drain_all` (block-device ioctls,
`systemctl daemon-reload` and generators, `mlock`, page migration) or
loads a BPF program (systemd units, a container start with seccomp, an
SSH login) still reaches every isolated CPU. Keep them out of trading
hours:

| source | rule |
|---|---|
| systemd daily timers | move or disable them on latency nodes (`systemctl list-timers`) |
| pod churn | no pod starts or restarts on the node during trading |
| logins | no SSH logins to the node during trading (`kubectl exec` also starts a seccomp-filtered process through runc; not traced) |

How often these fire on a production node over a day is not measured.

## The floor: below the guest kernel

Gaps the trace finds no guest event for: 5.13 and 3.48 µs in run
20261024234841, 5.87, 3.54 and 3.68 µs in run 20261024233129
(`trace-cpu3.md`, "Gaps of 3.00 µs or more"). The untraced reps' maxima,
4.81 to 5.52 µs with no interrupt counted on CPU 3, are the same kind.
That is the hypervisor (or the hardware): a 3.5 to 5.5 µs floor on this VM
that no guest setting removes.

## Shared-CPU scaling (run 20261024230400)

From [`scale.md`](lab/results/azure-20261024230400/scale.md): two shared
CPUs (`sharedCpus: 1-2`), 30 s windows, 4 helper threads a pod. Before the
THP and fair-server fixes.

| case | pods | shared CPUs given | helpers' CPU, cores | `nr_throttled` (container, pod) | spin max µs, involuntary switches |
|---|---|---|---|---|---|
| a-burst-100m | cpu 1, 100 m, busy | 1 | 0.99 (CPU 1 1.00 busy, CPU 2 0.05) | 0, 0 | 15.08, 0 |
| b-burst-1001m | cpu 1, 1,001 m, busy | 1, 2 | 1.99 | 0, 0 | 16.53, 0 |
| c-idle | cpu 1, 100 m, helpers busy 1 % | 1 | 0.04 | 0, 0 | 16.44, 0 |
| d-contention A | cpu 1, 100 m, busy | 1 | 0.33 | 0, 0 | 14.96, 0 |
| d-contention B | cpu 1, 100 m, busy | 2 | 0.99 | 0, 0 | 14.16, 0 |
| d-contention C | cpu 2, 100 m, busy | 1 | 0.66 | 0, 0 | 11.73, 0 |

- A container gets `ceil(millicpus/1000)` shared CPUs and, when busy, the
  whole of them; idle, near zero. No quota (`cpu.max` `max` on container
  and pod).
- A and C share CPU 1 and split it 0.33 : 0.66. That follows cgroup
  `cpu.weight` (container 100 vs 174, pod 39 vs 79), which comes from their
  integer `cpu` request, not their equal millicpus. B has CPU 2 alone.
- Every placement verdict passed: the spin thread on its exclusive CPU
  alone, every other thread on its pod's shared CPUs.
- Accounting ([`scale/accounting.txt`](lab/results/azure-20261024230400/scale/accounting.txt)):
  allocatable `ergon.io/shared-millicpus` 2000; a pod asking 2,100 stays
  Pending (`Insufficient ergon.io/shared-millicpus`); one asking 2,000
  schedules.

## e2e placement (run 20261024234841)

From [`e2e.txt`](lab/results/azure-20261024234841/e2e.txt): the hot pod was
admitted with `ERGON_SHARED_CPUS=1` and `ERGON_ISOLATED_CPUS=3,4`; in three
`ps -L` snapshots its spin thread (`gp-hot`) was on CPU 3, allowed 3, and
every other thread on CPU 1, allowed 1. An exec-probe hot pod was denied.

## EKS c8g.xlarge (run eks-nri-20261025-082023)

The cloud lab's EKS runner: EKS 1.36, AL2023, 2 x `c8g.xlarge`
spot (4 vCPUs, Graviton4, no SMT), kernel 6.18.54, containerd 2.3.5, this
tree's chart and image 0.1.2. Files:
[`lab/results/eks-nri-20261025-082023/`](lab/results/eks-nri-20261025-082023/)
(`phase-x/` holds the probes' CSV and the script that ran them by hand on
the lab's cluster, `phasex-manual.sh`). One run of 15 s a cell, no repeat
run: read the A/A pairs before any comparison.

| item | value |
|---|---|
| layout | CPU 0: system and `sharedCpus`. CPU 1: every IRQ. CPUs 2-3: `isolcpus=managed_irq,domain` + `nohz_full` + `rcu_nocbs`, THP `never`, fair server 0 (the quiet-CPU unit) |
| pod | Guaranteed `cpu: 1` (exclusive CPU 2), `ergon.io/shared-millicpus: 250`, `seccomp.ergon.io: io-uring` |
| placement | PID 1 `Cpus_allowed_list: 0` (the hook), container cpuset `0,2`, an exec'd shell `0,2`, `ERGON_ISOLATED_CPUS=2`, `ERGON_SHARED_CPUS=0` (`phase-x/placement-*.txt`) |
| load | one `while :` spinner pinned to CPU 0 in a Burstable pod outside the plugin's namespaces: CPU 0 100 % busy (`busy.txt`); idle cells 0 to 1 % |

**A helper's wake-up wait** (`wakelat`): a thread on the shared CPU sleeps to
a 1 ms absolute deadline and records how late it woke. µs, 15,000 wake-ups a
cell.

| cell | slack | p50 | p99 | p99.9 | p99.99 | max |
|---|---|---|---|---|---|---|
| idle A | 50 µs | 52.6 | 56.8 | 299 | 1,390 | 2,447 |
| idle B (A/A) | 50 µs | 52.6 | 57.8 | 281 | 2,907 | 3,265 |
| idle | 1 ns | 2.6 | 8.5 | 219 | 1,343 | 3,369 |
| CPU 0 at 100 % | 50 µs | 54.5 | 56.7 | 57.5 | 97 | 117 |
| CPU 0 at 100 % | 1 ns | 4.6 | 7.0 | 8.9 | 76 | 77 |

- **The 1 ns slack** (the hook's) takes 50 µs off every wake-up: p50 52.6
  to 2.6 µs idle, 54.5 to 4.6 µs loaded. That is the kernel's default slack,
  removed. It does not move the tail.
- **Under one spinner** the wait stayed under 117 µs (one run, no A/A
  repeat of the loaded cells). That the woken helper preempts the spinner
  (EEVDF wake-up preemption) is the likely reason, not traced. Milliseconds
  were not seen under this load.
- **Idle maxima of 2.4 to 3.4 ms, unexplained.** Both idle A/A cells have
  them, the loaded cells do not. Ruled out: the slack (1 ns has it too), the
  probe's CPU (every wake-up was on CPU 0, `offshared` 0). Not ruled out:
  the node's own daemons on CPU 0 (systemd `CPUAffinity=0,1`: kubelet,
  containerd, the VPC CNI) and the idle CPU's exit latency. No trace was
  taken.

**The pin's cost** (`pincost`, 10,000 times): read and parse
`ERGON_ISOLATED_CPUS`, then `sched_setaffinity` from the shared CPU to
CPU 2, until `sched_getcpu()` reports it. µs.

| cell | step | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|
| idle | read `ERGON_*` | 0.06 | 0.10 | 0.19 | 0.24 |
| idle | pin (move included) | 5.3 | 6.0 | 8.2 | 50 |
| CPU 0 at 100 % | read `ERGON_*` | 0.16 | 0.18 | 0.19 | 0.23 |
| CPU 0 at 100 % | pin (move included) | 6.6 | 8.3 | 9.5 | 26 |

The call returns once the thread runs on the target (pin and "moved" agree
within 0.1 µs). Both are start-up costs, paid once a thread.

The SQPOLL placement measured on the same nodes is in the book
([shared CPUs](https://mimran1980.github.io/ergon/deploy/shared-cpus.html)).

## Plugin outage timings (kind)

`test/e2e.sh` step 18, run with `E2E_OUTAGE_CYCLES=20` on 2026-11-05.
These are functional timings: kind on Docker Desktop on an Apple-silicon
Mac (linuxkit 7.0.14 arm64, 10 vCPUs), Kubernetes 1.37.0, containerd
2.3.4. The host was not quiet: its load average was 65 at the start of
step 18 and 140 at the end, from other jobs. They show how the contract
behaves, not latency.

Each cycle runs with pod A up: `cpu: 2`, 250 shared millicpus, a helper
that burns 5 ms of every 10 ms round, and `ergon-cpus --ready-addr` with
an `httpGet` readiness probe (`periodSeconds: 1`, `failureThreshold: 1`).
The cycle parks the plugin and restarts the kubelet, which shrinks the
cpuset to the exclusive CPUs. It waits for the probe to fail, holds the
outage 5 s, then brings the plugin back. The marks come from three places:
- the node reads the container's `cpuset.cpus.effective` every 20 ms
  (shrink and whole);
- the probe logs a line on each change (503 and 200);
- the helper logs each round that ends off the shared CPU, with its CPU
  time.

ms, 20 cycles, one run. At n = 20, p99 and p99.99 are the max. Raw files:
[`lab/results/kind-outage-20261105/`](lab/results/kind-outage-20261105/)
(`outage.tsv`, its `columns.tsv`, the node's cpuset traces, step 18's log).

| metric | n | min | p50 | p99 | p99.99 | max | cause of the max |
|---|---|---|---|---|---|---|---|
| detect: shrink to probe 503 | 20 | -17.6 | 186.6 | 989.8 | 989.8 | 989.8 | probe phase: the shrink came just after a probe, so it waited about one 1 s period. The negative min is the node poll's 20 ms resolution |
| harm: shrink to cpuset whole | 20 | 5893.6 | 6796.2 | 7951.9 | 7951.9 | 7951.9 | the 5 s hold the test sets, plus that cycle's slow detection (948 ms) and recovery (1149 ms) |
| recover: plugin back to cpuset whole | 20 | 560.4 | 855.3 | 1150.3 | 1150.3 | 1150.3 | plugin pod start and `Synchronize` on the loaded host; the two were not timed apart (unexplained) |
| verify: cpuset whole to probe 200 | 20 | 1.4 | 553.1 | 981.1 | 981.1 | 981.1 | probe phase, as for detect |
| helper back: cpuset whole to last helper round off S | 20 | -35.2 | -22.1 | -2.9 | -2.9 | -2.9 | always negative: the helper was back on the shared CPU before the node poll saw the whole cpuset, so no lag shows at this resolution |
| helper CPU time on exclusive CPUs | 20 | 1817.5 | 2102.4 | 2453.7 | 2453.7 | 2453.7 | the longest outage (7777 ms); the helper's CPU time is 31 % of each outage in every cycle (30.6 to 31.7 %) |
| helper rounds off S (count) | 20 | 362 | 420 | 486 | 486 | 486 | the longest outage |

In all 20 cycles, the probe reported verified only after the node saw the
whole cpuset, and the helper ran on an exclusive CPU (the harm is real and
lasts as long as the outage). Detection and verification are bounded by
the probe period. Interference grows with the outage, and only the plugin's
return ends it: readiness reports the problem, it does not fix it.

### Second run, quieter host

The same step 18 and 20 cycles, run again later the same day on the same
cluster setup (kind on Docker Desktop, 10 vCPUs, kernel 7.0.14, a fresh
cluster) after the merge of the outage work. The host load average was 9.9
at the start of step 18 and 4.6 at the end (65 and 140 in the first run);
the Mac was still running other jobs, so this is quieter, not quiet. The
first run's table above is unchanged. Raw files:
[`lab/results/kind-outage-20261105-run2/`](lab/results/kind-outage-20261105-run2/).

ms, 20 cycles, one run. At n = 20, p99 and p99.99 are the max.

| metric | n | min | p50 | p99 | p99.99 | max | cause of the max |
|---|---|---|---|---|---|---|---|
| detect: shrink to probe 503 | 20 | 107.4 | 186.3 | 983.7 | 983.7 | 983.7 | cycle 2: the shrink came just after a probe, so it waited about one 1 s period |
| harm: shrink to cpuset whole | 20 | 5761.4 | 5876.8 | 7094.3 | 7094.3 | 7094.3 | cycle 2: the 5 s hold, plus its slow detection (984 ms) and recovery (627 ms); the remaining 483 ms (the wait for the NotReady status before the hold starts) is unexplained |
| recover: plugin back to cpuset whole | 20 | 508.1 | 532.7 | 678.7 | 678.7 | 678.7 | cycle 6: plugin pod start and `Synchronize`, not timed apart (unexplained) |
| verify: cpuset whole to probe 200 | 20 | 10.4 | 72.2 | 898.4 | 898.4 | 898.4 | cycle 2: probe phase, as for detect |
| helper back: cpuset whole to last helper round off S | 20 | -36.5 | -18.1 | -8.2 | -8.2 | -8.2 | always negative: the helper was back on the shared CPU before the node poll saw the whole cpuset; no lag shows at the poll's 20 ms resolution |
| helper CPU time on exclusive CPUs | 20 | 1771.3 | 1815.7 | 2227.2 | 2227.2 | 2227.2 | cycle 2, the longest outage (7094 ms); the helper's CPU time is 30.6 to 31.6 % of each outage |
| helper rounds off S (count) | 20 | 354 | 363 | 445 | 445 | 445 | cycle 2, the longest outage |

As in the first run, the probe reported verified only after the node saw the
whole cpuset in all 20 cycles, and the helper ran on an exclusive CPU in all
20. The quieter host shortened recovery (p50 533 ms against 855 ms) and the
tail of verification; detection is bounded by the probe period in both.

## Not measured

- After a plugin outage, the API `Ready` condition and the Endpoints
  removal. The kubelet's status sync comes after the probe result timed
  above. Plugin pod start and `Synchronize` were also not timed apart.

- A helper's wake-up wait under more than one spinner, and the cause of
  the idle 2.4 to 3.4 ms maxima (EKS, above).
- The scale step with the THP and fair-server fixes (its spin maxima above
  are from before them).
- Another VM size, x86, bare metal. The hot-thread spin probe on EKS.
- How often node events (above) occur on a production node.
