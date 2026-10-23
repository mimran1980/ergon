//! What the checks take (the component, its hot CPUs) and what they return
//! (findings).

use std::fmt;

use crate::cpuset::CpuSet;

/// A check, by its identifier in the low-latency specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CheckId {
    /// Hot CPUs isolated and tickless.
    T1,
    /// NIC interrupts off the hot CPUs.
    T2,
    /// Transparent hugepages.
    T3,
    /// The Aeron directory is memory-backed.
    T4,
    /// Automatic NUMA balancing.
    T5,
    /// The NIC on the hot CPUs' NUMA node.
    T6,
    /// A hot thread under `SCHED_FIFO` or `SCHED_RR`.
    T7,
    /// Two busy threads on one physical core.
    T8,
    /// A CPU quota (CFS bandwidth limit) on the process's cgroup.
    T9,
    /// SMT (hyperthreading) is active on this x86 host.
    T10,
    /// The pod is not Guaranteed `QoS`.
    K1,
    /// The pod's CPUs are not exclusive (the shared pool).
    K2,
    /// A hot CPU's SMT sibling is outside the pod's cpuset.
    K3,
    /// The pod has no CPU for helpers.
    K4,
    /// A capability the caller uses is missing.
    K5,
    /// `memory.high` or CPU/memory pressure on the pod's cgroup.
    K6,
    /// The pod's CPUs are not on the NIC's NUMA node.
    K7,
    /// ENA adaptive RX interrupt moderation (`nic`).
    N1,
    /// Deferred interrupts without a busy poller (`nic`).
    N2,
    /// An Azure accelerated-networking VF (`nic`).
    N3,
    /// The kernel RX ring (`nic`).
    N4,
    /// The media driver's agents busy-spin on pinned cores of their own
    /// (checked by `ergon-md-dpdk`, which builds the finding).
    M1,
    /// The media driver's term buffers are not sparse (`ergon-md-dpdk`).
    M2,
    /// The media driver's socket buffers are granted as asked
    /// (`ergon-md-dpdk`).
    M3,
    /// Kernel busy polling for the media driver's socket transport
    /// (`ergon-md-dpdk`).
    M4,
    // Added after 0.1.30: last, so the variants before keep their order.
    /// How `CLOCK_REALTIME` is kept: the time daemon, PTP-grade or NTP, its
    /// error bound, an unsettled correction, a pending leap second.
    T11,
    /// The kernel's clock source: the hardware counter, a hypervisor's view
    /// of it, or one read by a system call (hpet, `acpi_pm`).
    T12,
    // Added after 0.1.33: last, so the variants before keep their order.
    /// Unbound workqueues may run on a busy-spinning CPU
    /// (`/sys/devices/virtual/workqueue/cpumask`).
    T13,
    /// Memory compaction may move a hot thread's pages: proactive
    /// compaction, or compaction of locked pages (vm sysctls).
    T14,
    /// Another process's task may run on a busy-spinning CPU.
    T15,
    /// Kernel instrumentation: kprobes, ftrace hooks, BPF programs.
    T16,
    /// Kernel same-page merging (KSM) runs.
    T17,
    /// Steal time on a busy-spinning CPU: the hypervisor ran something else.
    T18,
    /// A hot thread took page faults while it ran (`FaultWatch`).
    T19,
}

impl CheckId {
    /// The identifier (`T1`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
            Self::T4 => "T4",
            Self::T5 => "T5",
            Self::T6 => "T6",
            Self::T7 => "T7",
            Self::T8 => "T8",
            Self::T9 => "T9",
            Self::T10 => "T10",
            Self::T11 => "T11",
            Self::T12 => "T12",
            Self::T13 => "T13",
            Self::T14 => "T14",
            Self::T15 => "T15",
            Self::T16 => "T16",
            Self::T17 => "T17",
            Self::T18 => "T18",
            Self::T19 => "T19",
            Self::K1 => "K1",
            Self::K2 => "K2",
            Self::K3 => "K3",
            Self::K4 => "K4",
            Self::K5 => "K5",
            Self::K6 => "K6",
            Self::K7 => "K7",
            Self::N1 => "N1",
            Self::N2 => "N2",
            Self::N3 => "N3",
            Self::N4 => "N4",
            Self::M1 => "M1",
            Self::M2 => "M2",
            Self::M3 => "M3",
            Self::M4 => "M4",
        }
    }

    /// Whether an unknown result of this check stops a
    /// [`HostCheckPolicy::Require`](crate::HostCheckPolicy::Require) start.
    ///
    /// A check that proves a property of this pod or process — its CPUs are
    /// its own, they are isolated, there is no CPU quota, its helpers have a
    /// core — **blocks** when it cannot be answered: `require` must not start
    /// on an unproven property, and the file is one this node always has.
    /// Unproven is not "fine".
    ///
    /// A check of a subsystem the node may not have — PSI and `memory.high`,
    /// a NUMA node, a NIC knob, THP — does **not** block: that is a real
    /// host, and refusing to start there is not a readiness gate but a
    /// refusal to run at all.
    ///
    /// The match is exhaustive on purpose: a new check has to be classified
    /// here, and until it is, the build says so.
    #[must_use]
    pub const fn unknown_stops_require(self) -> bool {
        match self {
            // Proof about this pod or process. The evidence is node state
            // this deployment provides, and a host that can run a busy loop
            // always has the file.
            Self::T1
            | Self::T2
            | Self::T7
            | Self::T8
            | Self::T9
            | Self::T10
            | Self::T12
            | Self::K1
            | Self::K2
            | Self::K3
            | Self::K4
            | Self::K5
            | Self::M1
            | Self::M2
            | Self::M3 => true,
            // A subsystem the node may not have. Absent evidence is not a
            // gap in the proof; it is a host without that subsystem. T11:
            // a pod normally sees neither the time daemons (another PID
            // namespace) nor chronyd's port (another network namespace).
            // T13-T19: node-wide noise, read where the node lets a process
            // read it (debugfs, other PID namespaces and steal accounting
            // may be out of sight); none proves a property of this pod.
            Self::T3
            | Self::T11
            | Self::T13
            | Self::T14
            | Self::T15
            | Self::T16
            | Self::T17
            | Self::T18
            | Self::T19
            | Self::T4
            | Self::T5
            | Self::T6
            | Self::K6
            | Self::K7
            | Self::N1
            | Self::N2
            | Self::N3
            | Self::N4
            | Self::M4 => false,
        }
    }
}

impl fmt::Display for CheckId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a caller reports a [`Status::Finding`]: a warning line, or an
/// informational one. Opt-in strict modes (refusing to start) are the
/// caller's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    /// One warning line.
    Warn,
    /// One informational line.
    Info,
}

impl Severity {
    /// `warn` or `info`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }
}

/// What a check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The check fired: the message gives the effect and the fix.
    Finding,
    /// Nothing to report, or the check does not apply here (the message
    /// says which).
    Ok,
    /// The data could not be read (the reason): never a warning. It stops a
    /// `require` start only where the check must prove something —
    /// [`CheckId::unknown_stops_require`].
    Unknown(String),
}

/// One result of a check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The check.
    pub id: CheckId,
    /// How to report it when it is a [`Status::Finding`].
    pub severity: Severity,
    /// Finding, ok or unknown.
    pub status: Status,
    /// What was found, its measured effect and the exact fix (for a
    /// finding); what was checked (ok); what could not be told (unknown).
    pub message: String,
    /// Where the evidence for the effect is (a write-up and its section,
    /// or a document).
    pub evidence: &'static str,
}

impl Finding {
    pub(crate) const fn found(
        id: CheckId,
        severity: Severity,
        evidence: &'static str,
        message: String,
    ) -> Self {
        Self {
            id,
            severity,
            status: Status::Finding,
            message,
            evidence,
        }
    }

    pub(crate) const fn ok(
        id: CheckId,
        severity: Severity,
        evidence: &'static str,
        message: String,
    ) -> Self {
        Self {
            id,
            severity,
            status: Status::Ok,
            message,
            evidence,
        }
    }

    pub(crate) fn unknown(
        id: CheckId,
        severity: Severity,
        evidence: &'static str,
        message: String,
        reason: impl fmt::Display,
    ) -> Self {
        Self {
            id,
            severity,
            status: Status::Unknown(reason.to_string()),
            message,
            evidence,
        }
    }

    /// Whether the check fired.
    #[must_use]
    pub const fn is_finding(&self) -> bool {
        matches!(self.status, Status::Finding)
    }

    /// Whether the check fired at [`Severity::Warn`].
    #[must_use]
    pub const fn is_warning(&self) -> bool {
        self.is_finding() && matches!(self.severity, Severity::Warn)
    }

    /// Whether the data could not be read.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self.status, Status::Unknown(_))
    }
}

/// One line: `T1 warn: <message> (evidence: <where>)`, `T1 ok: <message>`
/// or `T1 unknown: <message>: <reason>`.
impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.status {
            Status::Finding => write!(
                f,
                "{} {}: {} (evidence: {})",
                self.id,
                self.severity.as_str(),
                self.message,
                self.evidence
            ),
            Status::Ok => write!(f, "{} ok: {}", self.id, self.message),
            Status::Unknown(reason) => write!(f, "{} unknown: {}: {reason}", self.id, self.message),
        }
    }
}

/// The component calling the checks. Some checks weigh differently per
/// component (THP: a warning for the processes that run Aeron, information
/// for ergon-io alone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Component {
    /// The media driver (`ergon-md-dpdk`).
    Md,
    /// ergon-io.
    Io,
    /// An application's busy-spinning runtime: a loop that runs Aeron.
    Rt,
}

/// What a hot CPU is used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    /// A busy-spinning event loop (ergon-io's polling thread, the runtime's
    /// loop).
    Loop,
    /// The media driver's sender agent.
    Sender,
    /// The media driver's receiver agent.
    Receiver,
    /// The media driver's shared or shared-network agent.
    SharedAgent,
    /// ergon-io's `io_uring` SQPOLL kernel thread (`sqpoll_cpu`).
    SqPoll,
    /// The media driver's conductor: one of the component's threads, not a
    /// busy-spinner, so not checked for isolation itself.
    Conductor,
    /// A DPDK EAL lcore: one of the component's threads, not a busy-spinner.
    EalLcore,
    /// A CPU the component keeps for background work (the runtime's
    /// background CPUs): never checked, only noted when it is a busy
    /// thread's SMT sibling.
    Housekeeping,
}

impl Role {
    /// The role's name in messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Sender => "sender",
            Self::Receiver => "receiver",
            Self::SharedAgent => "shared agent",
            Self::SqPoll => "SQPOLL thread",
            Self::Conductor => "conductor",
            Self::EalLcore => "EAL lcore",
            Self::Housekeeping => "housekeeping",
        }
    }

    /// Whether a thread in this role busy-spins: the CPUs T1, T2 and T8
    /// check.
    #[must_use]
    pub const fn spins(self) -> bool {
        matches!(
            self,
            Self::Loop | Self::Sender | Self::Receiver | Self::SharedAgent | Self::SqPoll
        )
    }
}

/// A CPU and what the component uses it for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HotCpu {
    /// The CPU number.
    pub cpu: u32,
    /// What runs there.
    pub role: Role,
}

/// The CPUs a component uses, with their roles. A CPU may appear with more
/// than one role.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HotSet {
    cpus: Vec<HotCpu>,
}

impl HotSet {
    /// No CPUs.
    #[must_use]
    pub const fn new() -> Self {
        Self { cpus: Vec::new() }
    }

    /// This set with `cpu` in `role` added.
    #[must_use]
    pub fn with(mut self, cpu: u32, role: Role) -> Self {
        self.push(cpu, role);
        self
    }

    /// Adds `cpu` in `role` (once).
    pub fn push(&mut self, cpu: u32, role: Role) {
        let h = HotCpu { cpu, role };
        if !self.cpus.contains(&h) {
            self.cpus.push(h);
        }
    }

    /// The CPUs and roles, in the order added.
    pub fn iter(&self) -> impl Iterator<Item = &HotCpu> {
        self.cpus.iter()
    }

    /// The CPUs of busy-spinning roles ([`Role::spins`]).
    #[must_use]
    pub fn spinning(&self) -> CpuSet {
        self.cpus
            .iter()
            .filter(|h| h.role.spins())
            .map(|h| h.cpu)
            .collect()
    }

    /// Every CPU of the set.
    #[must_use]
    pub fn all(&self) -> CpuSet {
        self.cpus.iter().map(|h| h.cpu).collect()
    }

    /// The roles `cpu` has here, in the order added.
    #[must_use]
    pub fn roles(&self, cpu: u32) -> Vec<Role> {
        self.cpus
            .iter()
            .filter(|h| h.cpu == cpu)
            .map(|h| h.role)
            .collect()
    }

    /// `CPU 4 (sender)`, `CPU 4 (sender, receiver)`, or `CPU 4`.
    #[must_use]
    pub fn describe(&self, cpu: u32) -> String {
        let roles: Vec<&str> = self.roles(cpu).iter().map(|r| r.name()).collect();
        if roles.is_empty() {
            format!("CPU {cpu}")
        } else {
            format!("CPU {cpu} ({})", roles.join(", "))
        }
    }

    /// `describe` for every CPU of `cpus`, joined with commas.
    pub(crate) fn describe_all(&self, cpus: &CpuSet) -> String {
        cpus.iter()
            .map(|c| self.describe(c))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
