//! The media driver's low-latency host checks, built on [`ergon_tune`].
//!
//! [`ConfiguredDriver::from_env`](crate::driver::ConfiguredDriver::from_env)
//! runs them after the agents' pins are set and the transport is installed,
//! before the driver starts, and prints one line on stderr for each that is
//! not fine (`ergon-md-dpdk warning: ...`, or `ergon-md-dpdk info: ...` for
//! information and for what could not be read), with its measured effect, the
//! maximum where one was recorded, and the fix:
//!
//! | Check | What | Severity |
//! |---|---|---|
//! | **M1** | The network agents busy-spin, are pinned, and have cores of their own (off the conductor's CPU outside `SHARED` and `INVOKER`) | socket transport: warn; DPDK: the install refuses to start otherwise |
//! | **M2** | Term buffers are not sparse | warn |
//! | **M3** | `AERON_SOCKET_SO_RCVBUF`/`SNDBUF` granted as asked (a probe socket) | socket transport: warn; DPDK (host-local channels only): info |
//! | **M4** | Kernel busy polling for the socket transport: its state and what was measured, no setting recommended | info |
//! | **M-host** | T1 and T8 on the agents' CPUs (and the conductor's and EAL main lcore's for T8), T3, T4 (the driver's own directory), T5, T7, T9 (no CPU quota on the driver's cgroup) | as ergon-tune gives them |
//! | **M-nic** | T2 and T6 on the data NICs; for the socket transport also N1, N2, and N4 with busy polling | as ergon-tune gives them |
//! | **M-run** | N1 and T2 again for a socket-transport data NIC once its `carrier_changes` moved and settled | warn |
//!
//! Every check runs for every driver, at its own severity: a driver left at
//! Aeron's defaults (agents floating and backing off, sparse term buffers)
//! is warned about like any other.
//!
//! The data NICs are the DPDK ports, by PCI address, under `dpdk-ena`. The
//! socket transport does not know its own at start-up, and checking every
//! NIC would flag a management NIC left at its defaults on purpose, so they
//! are the interfaces `RUSTERON_MEDIA_DRIVER_DATA_NICS` names (comma- or
//! space-separated); without it one information line says what is skipped.
//! Named, they are watched by md's re-check thread (`md-host-checks`, Linux
//! only), which reads each NIC's `carrier_changes` once a second on a CPU off
//! every busy-spinning agent's core and re-runs N1 and T2 once the counter
//! moved and then held still for [`SETTLE_READS`] reads (an ENA reset
//! re-places queue interrupts and turns adaptive moderation back on). No
//! check runs on an agent's thread. Under DPDK the ports have no kernel
//! interface and nothing is re-checked.
//!
//! `ERGON_HOST_CHECKS` is the one switch (every ergon component's): `on`/`warn`
//! (the default), `off` (no check and no thread, said in one information line,
//! [`OFF_NOTE`]) or `require`, the strict opt-in: any warning then refuses to
//! start the driver ([`DpdkError::HostCheck`]). Any other value refuses to
//! start, naming the variable. Here `off` is the stronger one:
//! no check runs at all, rather than only the warning lines being dropped.
//! Strict by default is only what already was:
//! the DPDK transport's agent rules. Information lines and checks that cannot
//! read their data (`unknown`: a pod's network namespace, a system other than
//! Linux) never fail a start. Every warning line ends with how to turn it
//! off.
//!
//! The checks themselves are pure functions of a [`Host`] and their inputs,
//! so they run against fixture trees on any OS.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use ergon_tune::{
    BusyPoll, Component, CpuSet, Ethtool, EthtoolCache, Finding, HOST_CHECKS_ENV, Host, HotSet, Netlink, Nic, Role,
    Severity, Status, Thread, busy_poll_sysctls, carrier_changes, host_check_line, n1_adaptive_rx, n2_deferred_irqs,
    n4_rx_ring, t1_isolation, t2_nic_irqs, t3_thp, t4_aeron_dir, t5_numa_balancing, t6_nic_node, t7_sched_policy,
    t8_smt_siblings, t9_cpu_quota, t10_smt_on, t22_aeron_affinity_names, thread_affinity,
};
use rusteron_media_driver::AeronDriverContext;
use rusteron_media_driver::bindings::aeron_threading_mode_t;

use crate::error::DpdkError;
use crate::ffi::{effective_idle_strategy, is_busy_idle_strategy};

/// The variable naming the socket transport's data NICs, the kernel
/// interfaces its channels use (comma- or space-separated).
pub const DATA_NICS_ENV: &str = "RUSTERON_MEDIA_DRIVER_DATA_NICS";

/// The one line printed under [`HostCheckPolicy::Off`].
pub const OFF_NOTE: &str = "ergon-md-dpdk info: ERGON_HOST_CHECKS=off: no host check runs (the \
     agents' placement and idling, term buffers, socket buffers, busy polling, CPU isolation, SMT siblings, \
     hugepages, the Aeron directory, the CPU quota, the data NICs) and nothing is re-checked; warn (the default) prints what is not \
     set up for low latency";

/// How many further reads (one a second) a data NIC's `carrier_changes` must
/// hold a new value before N1 and T2 are checked again.
///
/// That is 2–3 s after it last moved. The counter moves as the carrier goes down, before the driver has
/// come back up and before a link-event hook (the re-pin T2 recommends) has
/// heard of it, so a check at the first move would warn about a host that is
/// about to fix itself.
pub const SETTLE_READS: u8 = 2;

/// How the host checks report ([`ergon_tune::HOST_CHECKS_ENV`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostCheckPolicy {
    /// No check runs, and no re-check thread starts.
    Off,
    /// Each line is printed; the driver starts whatever they say.
    #[default]
    Warn,
    /// As `Warn`, and any warning refuses to start the driver.
    Require,
}

impl HostCheckPolicy {
    /// Reads `ERGON_HOST_CHECKS` (every ergon component's one switch): `Warn`
    /// when it is unset. `off`, `on`/`warn` or `require`, any case.
    ///
    /// Here `off` is the stronger one: no check runs at all ([`OFF_NOTE`]),
    /// rather than only the warning lines being dropped.
    ///
    /// # Errors
    ///
    /// [`DpdkError::InvalidEnvironment`] for a value other than `off`,
    /// `on`, `warn` or `require`, naming the variable.
    pub fn from_env() -> Result<Self, DpdkError> {
        let policy =
            ergon_tune::HostCheckPolicy::from_env().map_err(|e| DpdkError::InvalidEnvironment(e.to_string()))?;
        Ok(match policy {
            ergon_tune::HostCheckPolicy::Off => Self::Off,
            ergon_tune::HostCheckPolicy::On => Self::Warn,
            ergon_tune::HostCheckPolicy::Require => Self::Require,
        })
    }
}

/// Reads [`DATA_NICS_ENV`]: the interfaces it names, in order, each once;
/// none when unset or empty.
///
/// # Errors
///
/// [`DpdkError::InvalidEnvironment`] for a name the kernel could not have
/// made (empty, over 15 bytes, `.` or `..`, or holding `/` or `:`).
pub fn data_nics_from_env() -> Result<Vec<String>, DpdkError> {
    let Some(value) = std::env::var_os(DATA_NICS_ENV) else {
        return Ok(Vec::new());
    };
    let value = value
        .into_string()
        .map_err(|v| DpdkError::InvalidEnvironment(format!("{DATA_NICS_ENV}={} is not UTF-8", v.display())))?;
    let mut nics: Vec<String> = Vec::new();
    for name in value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|n| !n.is_empty())
    {
        if !interface_name(name) {
            return Err(DpdkError::InvalidEnvironment(format!(
                "{DATA_NICS_ENV}={value:?}: {name:?} is not a network interface name"
            )));
        }
        if !nics.iter().any(|n| n == name) {
            nics.push(name.to_owned());
        }
    }
    Ok(nics)
}

/// A name the kernel could have given an interface (`dev_valid_name`): 1 to
/// 15 bytes, not `.` or `..`, no `/`, `:` or whitespace.
fn interface_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() < 16
        && name != "."
        && name != ".."
        && !name.bytes().any(|b| b == b'/' || b == b':' || b.is_ascii_whitespace())
}

/// Where the driver's agents run and how they idle: what M1 and the CPU
/// checks read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agents {
    /// The threading mode.
    pub mode: aeron_threading_mode_t,
    /// `AERON_CONDUCTOR_CPU_AFFINITY`: under `SHARED` and `INVOKER` the one
    /// agent's CPU.
    pub conductor_cpu: Option<u32>,
    /// `AERON_SENDER_CPU_AFFINITY`: under `SHARED_NETWORK` the network
    /// agent's CPU.
    pub sender_cpu: Option<u32>,
    /// `AERON_RECEIVER_CPU_AFFINITY`.
    pub receiver_cpu: Option<u32>,
    /// The conductor's idle strategy, as it runs.
    pub conductor_idle: String,
    /// The sender's idle strategy, as it runs.
    pub sender_idle: String,
    /// The receiver's idle strategy, as it runs.
    pub receiver_idle: String,
    /// The shared network agent's idle strategy, as it runs.
    pub shared_network_idle: String,
    /// The shared agent's (and an `INVOKER` driver's) idle strategy, as it
    /// runs.
    pub shared_idle: String,
}

/// One network agent of a threading mode.
struct NetworkAgent {
    name: &'static str,
    cpu: Option<u32>,
    idle: String,
    cpu_var: &'static str,
    idle_var: &'static str,
}

impl Agents {
    /// The layout `context` gives: its threading mode and CPUs, and each
    /// idle strategy as the agent runs it (an `AERON_*_IDLE_STRATEGY` from
    /// the environment, which Aeron loads while keeping the stored name at
    /// `backoff`).
    #[must_use]
    pub fn from_context(context: &AeronDriverContext) -> Self {
        let cpu = |c: i32| u32::try_from(c).ok();
        let [conductor, sender, receiver] = crate::ffi::agent_cpus(context);
        Self {
            mode: context.get_threading_mode(),
            conductor_cpu: cpu(conductor),
            sender_cpu: cpu(sender),
            receiver_cpu: cpu(receiver),
            conductor_idle: effective_idle_strategy(
                context.get_conductor_idle_strategy(),
                "AERON_CONDUCTOR_IDLE_STRATEGY",
            ),
            sender_idle: effective_idle_strategy(context.get_sender_idle_strategy(), "AERON_SENDER_IDLE_STRATEGY"),
            receiver_idle: effective_idle_strategy(
                context.get_receiver_idle_strategy(),
                "AERON_RECEIVER_IDLE_STRATEGY",
            ),
            shared_network_idle: effective_idle_strategy(
                context.get_sharednetwork_idle_strategy(),
                "AERON_SHAREDNETWORK_IDLE_STRATEGY",
            ),
            shared_idle: effective_idle_strategy(context.get_shared_idle_strategy(), "AERON_SHARED_IDLE_STRATEGY"),
        }
    }

    /// The CPUs the checks treat as the driver's: each pinned network agent
    /// (busy-spinning roles: T1, T2 and T8 check them), the conductor's
    /// outside `SHARED` and `INVOKER` (where it is the one agent), and the
    /// EAL main lcore's under DPDK (the component's other threads, for T8).
    #[must_use]
    pub fn hot(&self, eal_main_cpu: Option<u32>) -> HotSet {
        let mut hot = HotSet::new();
        match self.mode {
            aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED => {
                push(&mut hot, self.sender_cpu, Role::Sender);
                push(&mut hot, self.receiver_cpu, Role::Receiver);
                push(&mut hot, self.conductor_cpu, Role::Conductor);
            }
            aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => {
                push(&mut hot, self.sender_cpu, Role::SharedAgent);
                push(&mut hot, self.conductor_cpu, Role::Conductor);
            }
            aeron_threading_mode_t::AERON_THREADING_MODE_SHARED
            | aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => {
                push(&mut hot, self.conductor_cpu, Role::SharedAgent);
            }
        }
        push(&mut hot, eal_main_cpu, Role::EalLcore);
        hot
    }

    /// The agents carrying the mode's network work.
    fn network(&self) -> Vec<NetworkAgent> {
        match self.mode {
            aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED => vec![
                NetworkAgent {
                    name: "sender",
                    cpu: self.sender_cpu,
                    idle: self.sender_idle.clone(),
                    cpu_var: "AERON_SENDER_CPU_AFFINITY",
                    idle_var: "AERON_SENDER_IDLE_STRATEGY",
                },
                NetworkAgent {
                    name: "receiver",
                    cpu: self.receiver_cpu,
                    idle: self.receiver_idle.clone(),
                    cpu_var: "AERON_RECEIVER_CPU_AFFINITY",
                    idle_var: "AERON_RECEIVER_IDLE_STRATEGY",
                },
            ],
            aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => vec![NetworkAgent {
                name: "network agent (sender and receiver)",
                cpu: self.sender_cpu,
                idle: self.shared_network_idle.clone(),
                cpu_var: "AERON_SENDER_CPU_AFFINITY",
                idle_var: "AERON_SHAREDNETWORK_IDLE_STRATEGY",
            }],
            aeron_threading_mode_t::AERON_THREADING_MODE_SHARED => vec![NetworkAgent {
                name: "shared agent (conductor, sender and receiver)",
                cpu: self.conductor_cpu,
                idle: self.shared_idle.clone(),
                cpu_var: "AERON_CONDUCTOR_CPU_AFFINITY",
                idle_var: "AERON_SHARED_IDLE_STRATEGY",
            }],
            aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => vec![NetworkAgent {
                name: "application's thread running the driver (INVOKER)",
                cpu: self.conductor_cpu,
                idle: self.shared_idle.clone(),
                cpu_var: "AERON_CONDUCTOR_CPU_AFFINITY",
                idle_var: "AERON_SHARED_IDLE_STRATEGY",
            }],
        }
    }

    /// The role T7 names the thread creating the driver by: the one agent
    /// under `SHARED` and `INVOKER`, else the conductor. The agents it
    /// starts inherit its policy.
    const fn creating_role(&self) -> Role {
        match self.mode {
            aeron_threading_mode_t::AERON_THREADING_MODE_SHARED
            | aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => Role::SharedAgent,
            aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED
            | aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => Role::Conductor,
        }
    }
}

fn push(hot: &mut HotSet, cpu: Option<u32>, role: Role) {
    if let Some(c) = cpu {
        hot.push(c, role);
    }
}

const M1_EVIDENCE: &str = "Aeron's C driver (aeron_driver_context.c: every agent's idle strategy defaults to backoff; \
     aeron_agent.h: backoff parks from 1 µs to 1 ms); prctl(2) PR_SET_TIMERSLACK (default slack 50 µs); \
     md-dpdk/BENCHMARKS.md, Runs and their data (every lab run pins noop agents); the cost of a park or of \
     an unpinned agent is not measured here";

/// What a park costs, and the limit of what is known.
const PARK_COST: &str = "a sleep can overrun by the thread's timer slack, 50 µs by default (prctl(2) \
     PR_SET_TIMERSLACK), so a 1 µs park can take tens of µs; not measured here";

/// What sharing a core costs, and the limit of what is known.
const SHARED_CORE_COST: &str =
    "the agent then waits for the other thread's time slice before it polls again; not measured here";

const fn m1(status: Status, message: String) -> Finding {
    Finding {
        id: "M1",
        severity: Severity::Warn,
        status,
        message,
        evidence: M1_EVIDENCE,
    }
}

/// M1 for the socket transport: each network agent busy-spins on a pinned
/// CPU of its own.
///
/// Outside `SHARED` and `INVOKER` that CPU is not the conductor's. One
/// warning per agent that does not; one ok when every one does. (Two agents
/// on one CPU or one core are T8's.)
#[must_use]
pub fn m1_agents(agents: &Agents) -> Vec<Finding> {
    let shares_with_conductor = matches!(
        agents.mode,
        aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED
            | aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK
    );
    let network = agents.network();
    let mut out = Vec::new();
    for a in &network {
        let spins = is_busy_idle_strategy(&a.idle);
        let message = match (a.cpu, spins) {
            (Some(c), false) => Some(format!(
                "the {} is pinned to CPU {c} but idles with `{}`, which gives the CPU up once idle (Aeron's \
                 `backoff` parks from 1 µs up to 1 ms): {PARK_COST}. Fix: `{}=noop`",
                a.name, a.idle, a.idle_var
            )),
            (None, true) => Some(format!(
                "the {} busy-spins (`{}`) but is not pinned ({} unset): it floats, so the scheduler can put \
                 it on a core another thread uses; {SHARED_CORE_COST}. Fix: `{}=<an isolated CPU of its own>`",
                a.name, a.idle, a.cpu_var, a.cpu_var
            )),
            (None, false) => Some(format!(
                "the {} is neither pinned ({} unset) nor busy-spinning (`{}`). Floating, the scheduler can put \
                 it on a core another thread uses: {SHARED_CORE_COST}. Idle, it gives its CPU up: {PARK_COST}. \
                 Fix: `{}=<an isolated CPU of its own> {}=noop`",
                a.name, a.cpu_var, a.idle, a.cpu_var, a.idle_var
            )),
            (Some(c), true) if shares_with_conductor && agents.conductor_cpu == Some(c) => Some(format!(
                "the {} and the conductor are both on CPU {c} ({} and AERON_CONDUCTOR_CPU_AFFINITY {c}): the \
                 conductor's work (log buffers, name resolution) takes turns with a busy-spinning agent there; \
                 {SHARED_CORE_COST}. Fix: pin the conductor to a housekeeping CPU \
                 (`AERON_CONDUCTOR_CPU_AFFINITY=<housekeeping CPU>`)",
                a.name, a.cpu_var
            )),
            (Some(_), true) => None,
        };
        if let Some(m) = message {
            out.push(m1(Status::Finding, m));
        }
    }
    if out.is_empty() {
        let names: Vec<&str> = network.iter().map(|a| a.name).collect();
        out.push(m1(
            Status::Ok,
            format!("the {} busy-spin on pinned CPUs of their own", names.join(" and ")),
        ));
    }
    out
}

/// M1 under DPDK: the install enforces it (it refuses to start otherwise).
fn m1_dpdk() -> Finding {
    m1(
        Status::Ok,
        "the DPDK transport's install requires it and would have refused to start otherwise: the network \
         agents busy-spin on pinned CPUs, off the conductor's CPU and the EAL main lcore's core"
            .to_owned(),
    )
}

const M2_EVIDENCE: &str = "Aeron's C driver (aeron_driver_context.c: AERON_TERM_BUFFER_SPARSE_FILE_DEFAULT true); \
     Aeron's ChannelUriStringBuilder.sparse (\"at the potential expense of latency\"); md-dpdk/BENCHMARKS.md \
     (Method: non-sparse term buffers throughout); sparse was not A/B'd";

/// M2: the term buffers are not sparse, under either transport.
#[must_use]
pub fn m2_term_buffers(sparse: bool) -> Finding {
    let (status, message) = if sparse {
        (
            Status::Finding,
            "the term buffers are sparse (AERON_TERM_BUFFER_SPARSE_FILE unset or true: the C driver's \
             default, unlike the Java driver's): their pages fault in at first touch, an image's in the \
             receiver agent as it writes, each fault zeroing a 2 MiB page on hugetlbfs (4 KiB on tmpfs) while \
             the agent waits; Aeron's documentation says sparse saves space \"at the potential expense of \
             latency\". Every lab measurement used non-sparse buffers (no figure: not A/B'd). Fix: \
             `AERON_TERM_BUFFER_SPARSE_FILE=false`"
                .to_owned(),
        )
    } else {
        (
            Status::Ok,
            "the term buffers are not sparse: written in full when created".to_owned(),
        )
    };
    Finding {
        id: "M2",
        severity: Severity::Warn,
        status,
        message,
        evidence: M2_EVIDENCE,
    }
}

/// What a probe socket was granted for one buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    /// The context asks for none (0: the kernel's default).
    NotSet,
    /// Asked `asked` bytes; `getsockopt` reported `granted`, twice what
    /// the kernel keeps (socket(7)).
    Granted {
        /// The bytes asked for.
        asked: usize,
        /// What `getsockopt` reported.
        granted: usize,
    },
    /// The probe could not be made (the reason).
    Unknown(String),
}

/// The probe socket's two buffers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketBuffers {
    /// `SO_RCVBUF` (`AERON_SOCKET_SO_RCVBUF`).
    pub rcvbuf: Probe,
    /// `SO_SNDBUF` (`AERON_SOCKET_SO_SNDBUF`).
    pub sndbuf: Probe,
}

/// Asks the running kernel for `rcvbuf` and `sndbuf` bytes, and reads back
/// what it granted.
///
/// One UDP probe socket, each buffer skipped when 0. Works in any network
/// namespace. Linux only: elsewhere each is unknown.
#[must_use]
pub fn probe_socket_buffers(rcvbuf: usize, sndbuf: usize) -> SocketBuffers {
    probe::buffers(rcvbuf, sndbuf)
}

#[cfg(target_os = "linux")]
mod probe {
    use super::{Probe, SocketBuffers};
    use rustix::net::{AddressFamily, SocketType, socket, sockopt};

    pub fn buffers(rcvbuf: usize, sndbuf: usize) -> SocketBuffers {
        if rcvbuf == 0 && sndbuf == 0 {
            return SocketBuffers {
                rcvbuf: Probe::NotSet,
                sndbuf: Probe::NotSet,
            };
        }
        let fd = match socket(AddressFamily::INET, SocketType::DGRAM, None) {
            Ok(fd) => fd,
            Err(e) => {
                let why = format!("the probe socket could not be created: {e}");
                return SocketBuffers {
                    rcvbuf: unknown_unless_unset(rcvbuf, &why),
                    sndbuf: unknown_unless_unset(sndbuf, &why),
                };
            }
        };
        let rcv = if rcvbuf == 0 {
            Probe::NotSet
        } else {
            granted(
                rcvbuf,
                sockopt::set_socket_recv_buffer_size(&fd, rcvbuf).and_then(|()| sockopt::socket_recv_buffer_size(&fd)),
            )
        };
        let snd = if sndbuf == 0 {
            Probe::NotSet
        } else {
            granted(
                sndbuf,
                sockopt::set_socket_send_buffer_size(&fd, sndbuf).and_then(|()| sockopt::socket_send_buffer_size(&fd)),
            )
        };
        SocketBuffers {
            rcvbuf: rcv,
            sndbuf: snd,
        }
    }

    fn unknown_unless_unset(asked: usize, why: &str) -> Probe {
        if asked == 0 {
            Probe::NotSet
        } else {
            Probe::Unknown(why.to_owned())
        }
    }

    fn granted(asked: usize, got: rustix::io::Result<usize>) -> Probe {
        match got {
            Ok(granted) => Probe::Granted { asked, granted },
            Err(e) => Probe::Unknown(format!("the probe socket's buffer could not be set or read: {e}")),
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod probe {
    use super::{Probe, SocketBuffers};

    pub fn buffers(rcvbuf: usize, sndbuf: usize) -> SocketBuffers {
        let one = |asked: usize| {
            if asked == 0 {
                Probe::NotSet
            } else {
                Probe::Unknown("not Linux: the doubling the check reads is Linux's (socket(7))".to_owned())
            }
        };
        SocketBuffers {
            rcvbuf: one(rcvbuf),
            sndbuf: one(sndbuf),
        }
    }
}

const M3_EVIDENCE: &str = "Aeron's C driver (aeron_driver.c, aeron_driver_validate_sufficient_socket_buffer_lengths: \
     it warns only when getsockopt reports less than the request); socket(7); md-dpdk/BENCHMARKS.md (Method: \
     2 MiB with rmem_max and wmem_max at 16 MiB); not A/B'd";

/// M3: each socket buffer the context asks for was granted in full.
///
/// In full is the probe's `getsockopt` reporting at least twice the request.
/// Under the socket transport a warning; under DPDK, where only host-local
/// channels use kernel sockets, information.
#[must_use]
pub fn m3_socket_buffers(buffers: &SocketBuffers, dpdk: bool) -> Vec<Finding> {
    let severity = if dpdk { Severity::Info } else { Severity::Warn };
    let only = if dpdk {
        " (under dpdk-ena only host-local channels use kernel sockets)"
    } else {
        ""
    };
    [
        ("AERON_SOCKET_SO_RCVBUF", "net.core.rmem_max", &buffers.rcvbuf),
        ("AERON_SOCKET_SO_SNDBUF", "net.core.wmem_max", &buffers.sndbuf),
    ]
    .into_iter()
    .map(|(var, sysctl, probe)| {
        let (status, message) = match probe {
            Probe::NotSet => (Status::Ok, format!("{var} is 0: the kernel's default buffer")),
            Probe::Granted { asked, granted } if *granted >= asked.saturating_mul(2) => (
                Status::Ok,
                format!("{var} {asked} bytes granted in full (getsockopt {granted})"),
            ),
            Probe::Granted { asked, granted } => (
                Status::Finding,
                format!(
                    "{var} asked {asked} bytes and the kernel kept {} (getsockopt reports double: {granted}): \
                     {sysctl} caps it (socket(7)), and Aeron's own check warns only below half of the request, \
                     so it passes this silently{only}. A buffer smaller than asked overflows sooner under a \
                     burst, and each datagram it drops costs a NAK and a retransmit (not A/B'd: the lab ran \
                     2 MiB with the cap at 16 MiB). Fix: `sysctl -w {sysctl}=16777216` (in the host's network \
                     namespace), or ask for no more than it allows",
                    granted / 2
                ),
            ),
            Probe::Unknown(why) => (
                Status::Unknown(why.clone()),
                format!("cannot tell whether {var} is granted as asked"),
            ),
        };
        Finding {
            id: "M3",
            severity,
            status,
            message,
            evidence: M3_EVIDENCE,
        }
    })
    .collect()
}

const M4_EVIDENCE: &str = "md-dpdk/BENCHMARKS.md (Measured, but no setting recommended): c9g.2xlarge G1 \
     (benches/cloud/results/aws-20261016-072005.txt, summary.md lines 10, 14, 86, 118, 141) and c7i.4xlarge \
     round 7 (aws-20261015-185813.txt, summary.md lines 5, 8, 137, 173)";

/// What was measured with kernel busy polling, and why no setting follows.
const M4_MEASURED: &str = "measured, but no setting is recommended (md-dpdk/BENCHMARKS.md): on c9g.2xlarge (G1) \
     busy polling with a 4,096-entry ring lost at 288 B 1 M msgs/s in 5 of 5 reps (61,488 NAKs; p50 4,973 µs, \
     max 9,175 (8,544–102,826) µs) against 0 of 5 for interrupt moderation off with the data NICs' interrupts on a \
     core of their own, the arms differing in four settings; on c7i.4xlarge (round 7) it had p50 27.79 against \
     94.02 µs at 288 B 100 k msgs/s against plain kernel UDP (max 382 (372–389) against 469 (448–578) µs), the \
     arms differing in two settings. Neither isolates busy polling";

/// M4: whether kernel busy polling is on for the socket transport.
///
/// On is `net.core.busy_read` or `busy_poll` above 0. Information either
/// way, with what was measured: the committed runs do not isolate busy
/// polling, so no setting is recommended or warned about. Unknown where the
/// sysctls cannot be read (they exist only in the host's network namespace).
#[must_use]
pub fn m4_busy_polling(host: &Host) -> Finding {
    let (status, message) = match busy_poll_sysctls(host) {
        Ok(BusyPoll { busy_read, busy_poll }) => {
            let state = if busy_read > 0 || busy_poll > 0 { "on" } else { "off" };
            (
                Status::Finding,
                format!(
                    "kernel busy polling is {state} (net.core.busy_read {busy_read}, busy_poll {busy_poll}): \
                     {M4_MEASURED}"
                ),
            )
        }
        Err(e) => (
            Status::Unknown(e.to_string()),
            "cannot tell whether kernel busy polling is on (net.core.busy_read and busy_poll exist only in \
             the host's network namespace)"
                .to_owned(),
        ),
    };
    Finding {
        id: "M4",
        severity: Severity::Info,
        status,
        message,
        evidence: M4_EVIDENCE,
    }
}

/// The data NICs the checks look at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataNics {
    /// The socket transport's, by interface name ([`DATA_NICS_ENV`]); none
    /// named is one information line.
    Socket(Vec<String>),
    /// The DPDK ports.
    Dpdk {
        /// The sender's and the receiver's PCI addresses (or vdev names).
        ports: Vec<String>,
        /// Whether they are test vdevs, which have no PCI device.
        vdev: bool,
        /// Interfaces [`DATA_NICS_ENV`] named all the same: ignored, with a
        /// note.
        named: Vec<String>,
    },
}

/// What the checks read besides the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inputs {
    /// The agents' layout.
    pub agents: Agents,
    /// Whether the DPDK transport is installed.
    pub dpdk: bool,
    /// The EAL main lcore's CPU, under DPDK.
    pub eal_main_cpu: Option<u32>,
    /// `AERON_TERM_BUFFER_SPARSE_FILE` as the context has it.
    pub term_buffer_sparse: bool,
    /// The probe socket's buffers ([`probe_socket_buffers`]).
    pub buffers: SocketBuffers,
    /// The driver's Aeron directory.
    pub aeron_dir: PathBuf,
    /// The data NICs.
    pub nics: DataNics,
}

impl Inputs {
    /// The inputs of a driver configured as `context`, with the DPDK
    /// `transport` when installed and the data NICs `named`: the probe
    /// socket asks the running kernel.
    #[must_use]
    pub fn from_context(
        context: &AeronDriverContext,
        transport: Option<&crate::ffi::DpdkTransport>,
        named: Vec<String>,
    ) -> Self {
        let (dpdk, eal_main_cpu, nics) = match transport {
            Some(t) => {
                let (ports, vdev) = t.ports();
                (
                    true,
                    t.eal_main_cpu(),
                    DataNics::Dpdk {
                        ports: ports.to_vec(),
                        vdev,
                        named,
                    },
                )
            }
            None => (false, None, DataNics::Socket(named)),
        };
        Self {
            agents: Agents::from_context(context),
            dpdk,
            eal_main_cpu,
            term_buffer_sparse: context.get_term_buffer_sparse_file(),
            buffers: probe_socket_buffers(context.get_socket_so_rcvbuf(), context.get_socket_so_sndbuf()),
            aeron_dir: PathBuf::from(context.get_dir()),
            nics,
        }
    }
}

/// What the checks found about a driver and its host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostReport {
    /// The CPUs the checks treated as the driver's ([`Agents::hot`]).
    pub hot: HotSet,
    /// Information lines about what was not checked, and why.
    pub notes: Vec<String>,
    /// Every finding, those that are fine included.
    pub findings: Vec<Finding>,
}

impl HostReport {
    /// The lines to print: the notes, then a line per finding that is not
    /// fine ([`line()`]).
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.notes
            .iter()
            .cloned()
            .chain(self.findings.iter().filter_map(line))
            .collect()
    }

    /// The findings at warning severity: what `require` refuses.
    #[must_use]
    pub fn warnings(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.is_warning()).collect()
    }

    /// What [`HostCheckPolicy::Require`] decides: the driver may start
    /// unless a finding warns ([`Self::warnings`]). Information lines, and
    /// checks that could not read their data, never refuse.
    ///
    /// # Errors
    ///
    /// [`DpdkError::HostCheck`] naming each warning's check.
    pub fn require(&self) -> Result<(), DpdkError> {
        let warnings: Vec<String> = self.warnings().iter().map(|f| f.id.to_string()).collect();
        if warnings.is_empty() {
            return Ok(());
        }
        Err(DpdkError::HostCheck(format!(
            "{HOST_CHECKS_ENV}=require and {} host check warning(s) ({}), printed above: fix them, or set \
             {HOST_CHECKS_ENV}=warn or off to start anyway",
            warnings.len(),
            warnings.join(", ")
        )))
    }
}

/// The line printed for `f`, `None` for what is fine.
///
/// `ergon-md-dpdk warning: <finding> (ERGON_HOST_CHECKS=off silences
/// host-check warnings)` at [`Severity::Warn`], `ergon-md-dpdk info:
/// <finding>` at [`Severity::Info`] and for what could not be read
/// ([`host_check_line`]).
#[must_use]
pub fn line(f: &Finding) -> Option<String> {
    host_check_line("ergon-md-dpdk", f)
}

/// Every check for a driver with `inputs` on `host` (the ethtool answers
/// from `cache`).
#[must_use]
pub fn host_report<E: Ethtool>(host: &Host, inputs: &Inputs, cache: &mut EthtoolCache<E>) -> HostReport {
    let agents = &inputs.agents;
    let cpus = agents.hot(inputs.eal_main_cpu);
    let mut notes = Vec::new();
    let mut findings = Vec::new();
    if inputs.dpdk {
        findings.push(m1_dpdk());
    } else {
        findings.extend(m1_agents(agents));
    }
    findings.push(m2_term_buffers(inputs.term_buffer_sparse));
    findings.extend(m3_socket_buffers(&inputs.buffers, inputs.dpdk));
    let busy = if inputs.dpdk {
        None
    } else {
        findings.push(m4_busy_polling(host));
        busy_poll_sysctls(host).ok()
    };
    findings.extend(t1_isolation(host, &cpus));
    findings.extend(t8_smt_siblings(host, &cpus));
    findings.push(t10_smt_on(host));
    findings.push(t7_sched_policy(host, Thread::Current, agents.creating_role()));
    findings.extend(t3_thp(host, Component::Md));
    findings.push(t4_aeron_dir(host, &inputs.aeron_dir));
    // this process's environment, which only the running host has
    findings.push(t22_aeron_affinity_names(|name| {
        host.is_live().then(|| std::env::var(name).ok()).flatten()
    }));
    findings.push(t5_numa_balancing(host));
    findings.push(t9_cpu_quota(host));
    match &inputs.nics {
        DataNics::Socket(names) if names.is_empty() => notes.push(format!(
            "ergon-md-dpdk info: {DATA_NICS_ENV} is not set, so the socket transport's data NICs are not \
             checked (T2 interrupts, T6 NUMA node, N1 adaptive moderation, N2 deferred interrupts, N4 RX ring) \
             nor re-checked after a reset: name the interfaces its channels use, comma-separated (never the \
             management NIC, left at its defaults on purpose)"
        )),
        DataNics::Socket(names) => {
            let busy_polls = busy.map(|b| b.busy_read > 0);
            for nic in names {
                let iface = Nic::Interface(nic.clone());
                findings.extend(t2_nic_irqs(host, &iface, &cpus));
                findings.extend(t6_nic_node(host, &iface, &cpus));
                findings.push(n1_adaptive_rx(host, nic, cache));
                findings.extend(n2_deferred_irqs(host, nic, busy_polls));
                if busy_polls == Some(true) {
                    findings.push(n4_rx_ring(host, nic, cache));
                }
            }
        }
        DataNics::Dpdk { ports, vdev, named } => {
            if !named.is_empty() {
                notes.push(format!(
                    "ergon-md-dpdk info: {DATA_NICS_ENV} ({}) is ignored under dpdk-ena: the data NICs are the \
                     DPDK ports, checked by PCI address",
                    named.join(", ")
                ));
            }
            if *vdev {
                notes.push(
                    "ergon-md-dpdk info: the DPDK ports are test virtual devices (RUSTERON_DPDK_TEST_VDEV), \
                     which have no PCI device: their interrupts (T2) and NUMA node (T6) are not checked"
                        .to_owned(),
                );
            } else {
                let mut seen: Vec<&String> = Vec::new();
                for bdf in ports {
                    if seen.contains(&bdf) {
                        continue;
                    }
                    seen.push(bdf);
                    let port = Nic::Pci(bdf.clone());
                    findings.extend(t2_nic_irqs(host, &port, &cpus));
                    findings.extend(t6_nic_node(host, &port, &cpus));
                }
            }
        }
    }
    HostReport {
        hot: cpus,
        notes,
        findings,
    }
}

/// A data NIC re-checked after a reset: N1 and T2 again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recheck {
    /// The interface.
    pub nic: String,
    /// Its `carrier_changes` when last checked.
    pub from: u64,
    /// Its `carrier_changes` now.
    pub to: u64,
    /// T2 (when the driver has busy-spinning CPUs) and N1.
    pub findings: Vec<Finding>,
}

/// One watched NIC.
#[derive(Debug)]
struct Watch {
    nic: String,
    carrier: u64,
    /// A new value of the counter, and how many reads after the first have
    /// seen it unchanged.
    moved: Option<(u64, u8)>,
}

/// M-run: the data NICs watched for a reset, with the ethtool answers
/// (until the interface resets). md's re-check thread owns one.
#[derive(Debug)]
pub struct Rechecks<E: Ethtool = Netlink> {
    watches: Vec<Watch>,
    hot: HotSet,
    cache: EthtoolCache<E>,
}

impl<E: Ethtool> Rechecks<E> {
    /// Watches each of `nics` whose `carrier_changes` can be read on `host`,
    /// against the driver's CPUs `driver_cpus`.
    ///
    /// A NIC whose counter cannot be read (a pod's veth, an interface that is
    /// not there) has nothing to re-check.
    #[must_use]
    pub fn new(host: &Host, nics: &[String], driver_cpus: HotSet, cache: EthtoolCache<E>) -> Self {
        let watches = nics
            .iter()
            .filter_map(|nic| {
                carrier_changes(host, nic).ok().map(|carrier| Watch {
                    nic: nic.clone(),
                    carrier,
                    moved: None,
                })
            })
            .collect();
        Self {
            watches,
            hot: driver_cpus,
            cache,
        }
    }

    /// The interfaces watched.
    #[must_use]
    pub fn watched(&self) -> Vec<String> {
        self.watches.iter().map(|w| w.nic.clone()).collect()
    }

    /// Whether any interface is watched.
    #[must_use]
    pub const fn watching(&self) -> bool {
        !self.watches.is_empty()
    }

    /// One read of each watched NIC's `carrier_changes`, and N1 and T2 again
    /// for each whose counter moved since it was last checked and has then
    /// held still for [`SETTLE_READS`] further reads.
    pub fn tick(&mut self, host: &Host) -> Vec<Recheck> {
        let mut due = Vec::new();
        for w in &mut self.watches {
            let Ok(now) = carrier_changes(host, &w.nic) else {
                continue;
            };
            if now == w.carrier {
                w.moved = None;
                continue;
            }
            match w.moved {
                Some((seen, held)) if seen == now => {
                    if held + 1 >= SETTLE_READS {
                        due.push((w.nic.clone(), w.carrier, now));
                        w.carrier = now;
                        w.moved = None;
                    } else {
                        w.moved = Some((seen, held + 1));
                    }
                }
                _ => w.moved = Some((now, 0)),
            }
        }
        due.into_iter()
            .map(|(nic, from, to)| {
                let mut findings = Vec::new();
                if !self.hot.spinning().is_empty() {
                    findings.extend(t2_nic_irqs(host, &Nic::Interface(nic.clone()), &self.hot));
                }
                findings.push(n1_adaptive_rx(host, &nic, &mut self.cache));
                Recheck {
                    nic,
                    from,
                    to,
                    findings,
                }
            })
            .collect()
    }
}

/// The CPUs md's re-check thread may run on, off every busy-spinning core.
///
/// Those the calling thread may use, less every busy-spinning agent's core
/// (SMT siblings included), the conductor's too when it busy-spins; the
/// conductor's CPU alone when it is pinned there and does not spin (with
/// Aeron's housekeeping). `None` when nothing is left, or the calling
/// thread's CPUs cannot be read.
#[must_use]
pub fn housekeeping_cpus(host: &Host, agents: &Agents) -> Option<CpuSet> {
    let allowed = thread_affinity(host, Thread::Current).ok()?;
    let mut spinning = agents.hot(None).spinning();
    let conductor_spins = is_busy_idle_strategy(&agents.conductor_idle);
    if let (Some(c), true) = (agents.conductor_cpu, conductor_spins) {
        spinning.insert(c);
    }
    let mut busy = CpuSet::new();
    for cpu in spinning.iter() {
        busy.insert(cpu);
        let path = host.path(&format!(
            "/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list"
        ));
        if let Some(siblings) = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| CpuSet::parse_list(&t).ok())
        {
            busy = busy.union(&siblings);
        }
    }
    let free = allowed.difference(&busy);
    if let Some(c) = agents.conductor_cpu.filter(|c| free.contains(*c)) {
        return Some(CpuSet::of([c]));
    }
    (!free.is_empty()).then_some(free)
}

/// md's re-check thread (`md-host-checks`): stopped and joined when dropped.
#[derive(Debug)]
pub(crate) struct RecheckThread {
    stop: Option<mpsc::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for RecheckThread {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// How often the re-check thread reads the data NICs' counters.
const RECHECK_PERIOD: Duration = Duration::from_secs(1);

/// Starts md's re-check thread for the socket transport's data NICs
/// `nics`, or says why it does not start.
pub(crate) fn start_rechecks(agents: &Agents, nics: &[String]) -> Result<RecheckThread, String> {
    let host = Host::system();
    if !cfg!(target_os = "linux") {
        return Err("ergon-md-dpdk info: not Linux: the data NICs are not re-checked after a reset (M-run)".to_owned());
    }
    let rechecks = Rechecks::new(&host, nics, agents.hot(None), EthtoolCache::new());
    if !rechecks.watching() {
        return Err(format!(
            "ergon-md-dpdk info: no carrier_changes of {} can be read here (an interface not in this network \
             namespace): N1 and T2 are not re-checked after a reset (M-run)",
            nics.join(", ")
        ));
    }
    let Some(cpus) = housekeeping_cpus(&host, agents) else {
        return Err(
            "ergon-md-dpdk info: the thread creating the driver has no CPU off the busy-spinning agents' cores \
             for md's re-check thread: N1 and T2 are not re-checked after a reset (M-run)"
                .to_owned(),
        );
    };
    let (stop, stopped) = mpsc::channel::<()>();
    let (pinned_tx, pinned) = mpsc::channel::<Result<(), String>>();
    let on = cpus.to_string();
    let handle = std::thread::Builder::new()
        .name("md-host-checks".to_owned())
        .spawn(move || run_rechecks(&host, rechecks, &cpus, &pinned_tx, &stopped))
        .map_err(|e| format!("ergon-md-dpdk info: md's re-check thread could not start ({e}): M-run is off"))?;
    // It pins itself before anything else and says so: back from here, it
    // runs only off the agents' cores (which run nothing yet).
    match pinned.recv_timeout(PIN_WAIT) {
        Ok(Ok(())) => Ok(RecheckThread {
            stop: Some(stop),
            handle: Some(handle),
        }),
        Ok(Err(why)) => {
            let _ = handle.join(); // it returned after saying so
            Err(format!(
                "ergon-md-dpdk info: md's re-check thread could not be pinned to CPUs {on} ({why}), so it \
                 stopped rather than run on an agent's core: the data NICs are not re-checked after a reset \
                 (M-run)"
            ))
        }
        Err(_) => {
            // Not joined: it stops at its next read, `stop` gone.
            drop(stop);
            drop(handle);
            Err(format!(
                "ergon-md-dpdk info: md's re-check thread did not pin itself to CPUs {on} within {} ms: it was \
                 stopped, and the data NICs are not re-checked after a reset (M-run)",
                PIN_WAIT.as_millis()
            ))
        }
    }
}

/// How long `start_rechecks` waits for the thread to have pinned itself.
const PIN_WAIT: Duration = Duration::from_secs(5);

/// The re-check thread: pinned off the agents' cores first (the result sent
/// on `pinned`), then a read a second until stopped.
fn run_rechecks(
    host: &Host,
    mut rechecks: Rechecks,
    cpus: &CpuSet,
    pinned: &mpsc::Sender<Result<(), String>>,
    stopped: &mpsc::Receiver<()>,
) {
    let pin = pin_current_thread(cpus);
    let ok = pin.is_ok();
    if pinned.send(pin).is_err() || !ok {
        return;
    }
    while stopped.recv_timeout(RECHECK_PERIOD) == Err(mpsc::RecvTimeoutError::Timeout) {
        for r in rechecks.tick(host) {
            let lines: Vec<String> = r.findings.iter().filter_map(line).collect();
            eprintln!(
                "ergon-md-dpdk info: {} reset (carrier_changes {} → {}): N1 and T2 checked again{}",
                r.nic,
                r.from,
                r.to,
                if lines.is_empty() { ", nothing to fix" } else { ":" }
            );
            for l in lines {
                eprintln!("{l}");
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn pin_current_thread(cpus: &CpuSet) -> Result<(), String> {
    let mut set = rustix::thread::CpuSet::new();
    for cpu in cpus.iter() {
        let cpu = usize::try_from(cpu).map_err(|e| e.to_string())?;
        if cpu >= rustix::thread::CpuSet::MAX_CPU {
            return Err(format!(
                "CPU {cpu} is beyond the affinity mask's {}",
                rustix::thread::CpuSet::MAX_CPU
            ));
        }
        set.set(cpu);
    }
    rustix::thread::sched_setaffinity(None, &set).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "linux"))]
fn pin_current_thread(cpus: &CpuSet) -> Result<(), String> {
    let _ = cpus;
    Err("not Linux".to_owned())
}
