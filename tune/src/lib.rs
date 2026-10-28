#![doc = include_str!("../README.md")]
//!
//! ## The checks
//!
//! | Check | Function | Called by |
//! |---|---|---|
//! | T1 | [`t1_isolation`] | io, md, rt |
//! | T2 | [`t2_nic_irqs`] | io (egress NIC), md (data NICs, DPDK ports) |
//! | T3 | [`t3_thp`] | md, rt (warn); io (info) |
//! | T4 | [`t4_aeron_dir`] | md, rt |
//! | T5 | [`t5_numa_balancing`] | md, rt |
//! | T6 | [`t6_nic_node`] | md |
//! | T7 | [`t7_sched_policy`] | io, md, rt |
//! | T8 | [`t8_smt_siblings`] | io, md, rt |
//! | T9 | [`t9_cpu_quota`] at start; [`ThrottleWatch`] while running | io, md (start); `ThrottleWatch`: no caller in this repository |
//! | T10 | [`t10_smt_on`] | io, md |
//! | T11 | [`t11_time_sync`] at start; [`ClockWatch`] while running | rt (start, housekeeping) |
//! | T12 | [`t12_clocksource`] at start; [`ClockWatch`] while running | rt (start, housekeeping) |
//! | T13 | [`t13_workqueue_cpumask`] | no caller in this repository |
//! | T14 | [`t14_compaction`] (info) | no caller in this repository |
//! | T15 | [`t15_host_tasks`] | no caller in this repository |
//! | T16 | [`t16_instrumentation`] (info) | no caller in this repository |
//! | T17 | [`t17_ksm`] | no caller in this repository |
//! | T18 | [`t18_steal`] (info) | no caller in this repository |
//! | T19 | [`FaultWatch`] while running | io (the polling thread, every 5 s from the warning thread) |
//! | T20 | [`t20_irq_idle`] | no caller in this repository (the lab's transport suite caps the IRQ CPU itself) |
//! | K1-K6 | [`k8s_checks`] (with T7 and T9), inside a pod only | io |
//! | K7 | [`k8s_checks`] with [`Needs::with_nic`] | no caller in this repository names a NIC |
//! | N1 (`nic`) | `n1_adaptive_rx` | io, md (kernel sockets) |
//! | N2 (`nic`) | `n2_deferred_irqs` | io, md |
//! | N3 (`nic`) | `n3_azure_vf` | io |
//! | N4 (`nic`) | `n4_rx_ring` | md (socket transport with busy polling) |
//!
//! - **Callers** ([`Component`]): io is ergon-io, md the ergon-md-dpdk media
//!   driver, rt an application's busy-spinning runtime (outside this
//!   repository; it calls what it chooses).
//! - **Results**: each is a [`Finding`]. Its [`Status`] says whether the check
//!   fired; its [`Severity`] says how to log it.
//! - **Cost**: file reads and allocations. T7 reads `/proc/thread-self` (no
//!   system call); T2 reads one directory and two files per interrupt; N1 and
//!   N4 make one netlink exchange per interface per reset (plus one family
//!   lookup on first use), each with 100 ms send and receive timeouts.
//!   [`ThrottleWatch::check`] reads one open file (`cpu.stat` too once
//!   throttling shows) and allocates only when it reports. T11 scans
//!   `/proc` and waits at most 50 ms for chronyd; [`ClockWatch::check`]
//!   never waits and allocates only when a finding changes. T15 reads two
//!   files per task on the host; [`FaultWatch::check`] reads one open file
//!   and allocates only when it reports.
//! - **Evidence**: each finding's `evidence` names its source: a
//!   settings-matrix table in git history (`git show
//!   128747b9^:io/benches/results/<run>/tables.md`, with lines), or the
//!   kernel's, Kubernetes' or a vendor's documentation, marked "not
//!   measured here".
//! - **The switch**: every host-check warning line ends with how to silence
//!   it. [`HOST_CHECKS_ENV`] is `off` (no warning line, the results are
//!   still returned), `on` (the default) or `require` (a warning is an error
//!   at start). See [`HostCheckPolicy`] and [`host_check_line`].

mod cpu;
mod cpuset;
mod faults;
mod finding;
mod host;
mod idle;
mod k8s;
mod mem;
mod net;
#[cfg(feature = "nic")]
mod nic;
mod noise;
mod platform;
mod quota;
mod report;
mod time;

pub use cpu::{
    SchedPolicy, Thread, t1_isolation, t7_sched_policy, t8_smt_siblings, t10_smt_on,
    thread_affinity, thread_policy,
};
pub use cpuset::{CPU_LIMIT, CpuSet, CpuSetError};
pub use faults::{FaultWatch, Faults};
pub use finding::{Component, Finding, HotCpu, HotSet, Role, Severity, Status};
pub use host::{Host, ReadError};
pub use idle::{
    IdleState, ResumeLimit, T20_MAX_EXIT_LATENCY_US, cpu_idle_states, cpuidle_driver, resume_limit,
    t20_irq_idle,
};
pub use k8s::{Needs, in_pod, k8s_checks};
pub use mem::{FsMagic, fs_magic, t3_thp, t4_aeron_dir, t4_from, t5_numa_balancing};
pub use net::{
    BusyPoll, Nic, NicIrq, busy_poll_sysctls, carrier_changes, nic_irqs, t2_nic_irqs, t6_nic_node,
};
#[cfg(feature = "nic")]
pub use nic::{
    Coalesce, Ethtool, EthtoolCache, EthtoolError, Netlink, Rings, n1_adaptive_rx,
    n2_deferred_irqs, n3_azure_vf, n4_rx_ring,
};
pub use noise::{
    CpuSteal, cpu_steal, t13_workqueue_cpumask, t14_compaction, t15_host_tasks,
    t16_instrumentation, t17_ksm, t18_steal,
};
pub use platform::{Arch, Cloud, Platform};
pub use quota::{
    CgroupVersion, CpuCgroup, CpuQuota, ThrottleWatch, Throttling, cpu_cgroup, t9_cpu_quota,
};
pub use report::{
    HOST_CHECKS_ENV, HostCheckError, HostCheckPolicy, SILENCE, host_check_line,
    unknown_stops_require, warn_line,
};
pub use time::{
    Bound, BoundSource, ChronyAnswer, ChronyError, ChronyReference, ChronyTracking,
    ClockBoundAnswer, ClockBoundError, ClockBoundReading, ClockBoundStatus, ClockSample,
    ClockWatch, Clocksource, ClocksourceKind, ClocksourceName, KernelSync, Observed, PtpClock,
    RefclockId, SyncVerdict, TimeDaemon, TimeSync, TimeSyncLimits, TimeSyncProbe, t11_time_sync,
    t12_clocksource, time_sync,
};
