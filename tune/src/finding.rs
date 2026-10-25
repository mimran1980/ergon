//! What the checks take (the component, its hot CPUs) and what they return
//! (findings).

use std::fmt;

use crate::cpuset::CpuSet;

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
    /// [`unknown_stops_require`](crate::unknown_stops_require).
    Unknown(String),
}

/// One result of a check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The check's identifier in the low-latency specification (`T1`,
    /// `K2`, `M1`), or a caller's own for a check it adds.
    pub id: &'static str,
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
        id: &'static str,
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
        id: &'static str,
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
        id: &'static str,
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
