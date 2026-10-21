# ergon-tune

[![Crates.io](https://img.shields.io/crates/v/ergon-tune)](https://crates.io/crates/ergon-tune)
[![API Docs](https://docs.rs/ergon-tune/badge.svg)](https://docs.rs/ergon-tune/)

Checks of a Linux host's low-latency settings, for start-up and housekeeping.

> **AI assistance.** Like the rest of ergon, this crate was written **with heavy
> AI assistance**. See
> [AI-ASSISTANCE.md](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md).
> It is not production-ready.

## What it does

- A busy-spinning thread (one that loops on its CPU without sleeping) loses
  latency to whatever takes that CPU: a timer tick, an interrupt,
  `khugepaged`, a busy hyperthread on its core, or a container's CPU quota
  running out.
- Each of those is a host setting, and a reboot, a new image or a NIC reset
  can undo it.
- ergon-tune reads the host and returns findings: what is wrong, what it cost
  where it was measured, and the exact fix. It changes nothing.
- ergon-io and the ergon-md-dpdk media driver run these checks themselves.
  Call them yourself only from your own busy loop.

| Check | What it looks at |
|---|---|
| T1 | busy-spinning CPUs isolated, `nohz_full`, outside the default IRQ affinity |
| T2 | NIC interrupts off those CPUs |
| T3 | transparent hugepages off |
| T4 | the Aeron directory on tmpfs or hugetlbfs |
| T5, T6 | NUMA balancing off, and the NIC on the hot CPUs' node (multi-node hosts) |
| T7 | no hot thread under `SCHED_FIFO` or `SCHED_RR` |
| T8 | no two busy threads on one physical core |
| T9 | no CPU quota on the process's cgroup (a Kubernetes CPU limit, `docker run --cpus`); `ThrottleWatch` reads its throttling counters while the loop runs. Fix on Kubernetes: kubelet 1.33+, `cpuManagerPolicy: static`, a Guaranteed pod and a whole-number CPU request, so the kubelet drops the quota itself. Never `cpuCFSQuota: false`: it removes every pod's CPU limit on the node |
| T10 | SMT (hyperthreading) off on x86 (Graviton and arm64 pass: one vCPU is one physical core). On x86 a Kubernetes `cpu: 2` is one physical core, and a busy sibling slows the loop (+2.3–2.7 µs p50, +9–12 µs p99 at 10–100 connections; [data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#smt-sibling-cost)). The fix is on the node: EKS/EC2 `CpuOptions: ThreadsPerCore: 1`, self-managed `nosmt`, or an Azure size without SMT (Fasv6, Falsv6, Famsv6, Dpsv6, Dplsv6) |
| K1-K7 | inside a Kubernetes pod only, with T7 and T9: Guaranteed `QoS`, exclusive CPUs (the static CPU manager, not the shared pool), the loop's SMT sibling inside the pod's cpuset, a CPU for the helper threads (`ERGON_SHARED_CPUS`/`ERGON_HELPER_CPUS`), the capabilities the caller says it uses (`Needs`: busy polling, `SCHED_FIFO`, `mlockall`), no `memory.high` and no CPU/memory pressure, and the NIC's NUMA node. Each finding says whether the fix goes in the pod spec or on the node, with the exact setting |
| T11 | how `CLOCK_REALTIME` is kept: PTP-grade (chronyd locked to a PTP hardware clock, or phc2sys) is ok; NTP-disciplined or unsynchronised warns, with the chrony `refclock PHC` line for the host's PHC (a node setting); a correction still being slewed out, an error bound wider than 1 ms or a pending leap second warns. What the process cannot see (in a pod: the daemons and chronyd's port) is unknown, never a warning |
| T12 | the kernel's clock source: the counter (`tsc`, `arch_sys_counter`) is ok; a hypervisor's view of it (`kvm-clock`, Hyper-V's TSC page) is info; anything else (`hpet`, `acpi_pm`) on x86-64 or 64-bit Arm warns: every `CLOCK_REALTIME` and `CLOCK_MONOTONIC` read is a system call. On another architecture a source other than `tsc` or `arch_sys_counter` is unknown |
| N1-N4 (`nic`) | ENA adaptive RX moderation, deferred interrupts without a busy poller, the Azure accelerated-networking VF, the kernel RX ring |

A finding names its fix. T1 on a stock host with its loop on CPU 2 of 8
gives `isolcpus=nohz,domain,managed_irq,2 nohz_full=2 rcu_nocbs=2 irqaffinity=0-1,3-7`.

## The switch

`ERGON_HOST_CHECKS` is the one switch over every host-check warning:

| Value | What it does |
|---|---|
| `on` (default; `warn` is the same) | checks run and the warning lines print |
| `off` | no warning line prints; the results are still returned, and info lines still print |
| `require` | a warning is an error at start, and so is a check that could not prove a property it must prove (see below) |

`require` is a readiness gate, not a blanket refusal. It refuses on a warning
(a check fired) and on an **unproven** check whose job is to prove a property
of this pod or process: its CPUs are its own, they are isolated, there is no
CPU quota, its helpers have a core. Starting on an unproven claim is the false
pass those checks exist to stop. An unknown of a check whose subsystem the
node may not have — PSI and `memory.high`, a NUMA node, a NIC knob, THP — is
printed and allowed through: that is a real host, and refusing there would
stop every start on it. Which is which is `CheckId::unknown_stops_require`.

Every printed warning line ends with `(ERGON_HOST_CHECKS=off silences host-check
warnings)`. The media driver takes `RUSTERON_MEDIA_DRIVER_HOST_CHECK` as its
alias (`ERGON_HOST_CHECKS` wins); there `off` is stronger and runs no check at
all. ergon-io's `Driver::new` and the media driver read the switch; the checks
stay pure functions over a `Host`.

## When to use it

| Use it for | Not for |
|---|---|
| start-up, before a latency-sensitive loop | the hot path: checks read files and allocate; N1 and N4 make a netlink query that can wait 100 ms |
| housekeeping: an ENA reset turns adaptive moderation back on and moves queue interrupts; `ThrottleWatch::check` sees a CPU quota stopping the container; `ClockWatch::check` sees time sync or the clock source change | tuning a host: it only reads |
| CI or review, against a captured or hand-made `/sys` and `/proc` tree | hosts other than Linux: every reader reports unknown |

## Guarantees and limits

- **Read-only.** No check writes a file or changes a setting. T11 asks the
  kernel with `adjtimex` `modes` 0 (unprivileged, changes nothing) and chronyd
  with its `tracking` monitoring request (UDP to 127.0.0.1:323, an address, no
  name lookup).
- **Unknown is never a warning.** Unreadable data (a pod's network namespace,
  an interface with no device, `hidepid`) gives an unknown result with the
  reason.
- **Safe before a busy loop.** No threads, statics, thread-locals, atomics
  or DNS, and it logs nothing.
- **One `unsafe` call.** T11's `adjtimex` (`src/time/kernel.rs`): rustix has
  no wrapper for it yet, so the crate declares the C library's function over
  a hand-written `struct timex` (64-bit x86 and Arm Linux only; its size and
  field offsets, from the kernel's `include/uapi/linux/timex.h`, are asserted
  at compile time). It goes when rustix wraps `adjtimex`. Everything else is
  safe Rust.
- **Runs on fixtures.** Every read goes through a `Host`: the running host
  (`Host::system()`) or a tree standing for its `/` (`Host::at`), on any OS.
  Kernel queries (T4's `statfs`, T11's `adjtimex`, N1's and N4's ethtool)
  reach only the running host; on a tree they report unknown.
- **Measured.** Each finding quotes this repository's AWS and Azure
  measurements, or the kernel's or vendor's documentation where nothing was
  measured, and names its source. Hosts differ: measure yours.
- **Only what the table lists.** CPU idle states and frequency are not
  checked.

## How to use it

```toml
[dependencies]
ergon-tune = "0.1"   # features = ["nic"] for N1-N4
```

At start-up, name the CPUs that busy-spin and log what the checks find.
`Component` is the caller: the media driver (`Md`), ergon-io (`Io`) or an
application's busy-spinning runtime (`Rt`). Transparent hugepages warn for
`Md` and `Rt` and inform for `Io`.

```rust
use ergon_tune::{Component, Host, HotSet, Role, t1_isolation, t3_thp};

fn main() {
    let host = Host::system();
    let hot = HotSet::new().with(3, Role::Loop); // the loop runs on CPU 3
    for f in t1_isolation(&host, &hot).iter().chain(&t3_thp(&host, Component::Rt)) {
        // One line each: `T1 warn: <what, its measured effect, the fix> (evidence: <where>)`.
        if f.is_finding() {
            eprintln!("{f}");
        }
    }
}
```

In a container, check for a CPU quota before the loop, then watch the
throttling counters from housekeeping. `check` reads `cpu.stat.local`,
opened by `open`, and `cpu.stat` only after that grew (printing `cpu.stat`
takes a kernel lock every cgroup shares). It allocates only when it reports:

```rust
use ergon_tune::{Host, ThrottleWatch, t9_cpu_quota};

fn main() {
    let host = Host::system();
    let t9 = t9_cpu_quota(&host); // before the loop
    if t9.is_finding() {
        eprintln!("{t9}");
    }
    let mut watch = ThrottleWatch::open(&host).ok(); // before the loop
    // From housekeeping, every few seconds; never inside the hot path:
    if let Some(f) = watch.as_mut().and_then(ThrottleWatch::check) {
        eprintln!("{f}"); // `T9 warn: this process's cgroup (...) was throttled ...`
    }
}
```

How `CLOCK_REALTIME` is kept (T11) and the clock source (T12), before the
loop and then from housekeeping. `open` scans `/proc` and asks chronyd,
waiting at most 50 ms; `check` never waits and allocates only when a finding
changes. chronyd's answer is read one check late:

```rust
use ergon_tune::{ClockWatch, Host, TimeSyncProbe};

fn main() {
    let host = Host::system();
    let (mut watch, findings) = ClockWatch::open(&host, &TimeSyncProbe::default()); // before the loop
    for f in &findings {
        eprintln!("{f}"); // `T11 warn: CLOCK_REALTIME is not disciplined to a PTP hardware clock ...`
    }
    // From housekeeping, every few seconds; never inside the hot path:
    for f in watch.check() {
        eprintln!("{f}"); // none unless T11's judgement or the clock source changed
    }
    let _ = (watch.verdict(), watch.bound(), watch.leap_pending());
}
```

The same checks read a tree that stands for a host, on any OS:

```rust
use ergon_tune::{Component, Host, Status, t3_thp};

fn main() -> std::io::Result<()> {
    // A host's `/`, cut down to the one file T3 reads.
    let root = std::env::temp_dir().join(format!("ergon-tune-readme-{}", std::process::id()));
    let thp = root.join("sys/kernel/mm/transparent_hugepage");
    std::fs::create_dir_all(&thp)?;
    std::fs::write(thp.join("enabled"), "always [madvise] never\n")?;

    let found = t3_thp(&Host::at(&root), Component::Md);
    assert_eq!(found[0].status, Status::Finding);
    assert!(found[0].is_warning());
    assert!(found[0].message.contains("echo never > /sys/kernel/mm/transparent_hugepage/enabled"));

    std::fs::remove_dir_all(&root)
}
```

- Feature `nic` adds N1-N4. N1 and N4 ask the kernel over ethtool's generic
  netlink, unprivileged.
- The only dependency is [rustix](https://crates.io/crates/rustix), on Linux.

## Further reading

- The book chapter: each setting in plain English, measured effects, and a
  symptom-to-fix table: [ergon-tune](https://mimran1980.github.io/ergon/tune/overview.html).
- What the terms mean (IRQ, `nohz_full`, interrupt moderation, busy polling,
  THP, …), with pros and cons: the chapter's
  [glossary](https://mimran1980.github.io/ergon/tune/overview.html#glossary).
- API reference: [docs.rs/ergon-tune](https://docs.rs/ergon-tune/).

Licensed under Apache-2.0.
