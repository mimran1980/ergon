//! Where ergon-io's helper threads run: `Driver::connect_background`'s dial
//! helper, the warning thread, and `uring-sqpoll`'s `SQPOLL` kernel thread.
//!
//! `Driver::new` chooses once, on the thread creating it, and reports the
//! choice ([`crate::checks::HostReport::helpers`], and one start-up line).
//! The choice starts from the CPUs the process may use, never leaves them,
//! and never takes the polling CPU when it is known. When it is not (no
//! `Config::loop_cpu`, and the creating thread is not pinned to one CPU) it
//! cannot: the start-up line says the helpers may include the CPU the loop
//! ends up on.
//!
//! | Order | Helper CPUs | Typical case |
//! |---|---|---|
//! | 1 | `Config::helper_cpus`, else `ERGON_HELPER_CPUS` (a kernel CPU list: `3`, `3,7`, `2-3`) | CPUs a node-level mechanism adds to the pod's cpuset for shared work. A CPU outside the process's CPUs is an error ([`CpuError::Outside`]) |
//! | 2 | the polling CPU's SMT sibling | x86, `cpu: 2` with the kubelet's `full-pcpus-only`: one core's two hyperthreads |
//! | 3 | the process's other CPUs, those the host does not isolate first | Graviton (no SMT), `cpu: 2`: the other core |
//! | 4 | none | `cpu: 1`: the polling CPU only |
//!
//! - The process's CPUs: its cgroup's `cpuset.cpus.effective` (cgroup v1:
//!   `cpuset.effective_cpus`), which is a Kubernetes container's CPU set;
//!   else the main thread's affinity (`/proc/self/status`); else the online
//!   CPUs. Not the creating thread's affinity: that thread is usually pinned
//!   to the polling CPU already. The kernel refuses a thread mask or an
//!   `SQPOLL` CPU outside the cpuset (EINVAL), and the host's housekeeping
//!   CPUs are outside a Guaranteed pod's cpuset.
//! - `uring-sqpoll` puts `SQPOLL` on `Config::sqpoll_cpu` when it names one;
//!   else on the first helper CPU — except for an explicit helper list (row
//!   1: the shared-work CPUs a node plugin adds, never a spinner or `SQPOLL`
//!   there), where it takes the lowest CPU of the process's own outside that
//!   list, and refuses ([`CpuError::NoExclusiveCpu`]) when there is none.
//!   The dial helper and the warning thread take the other helper CPUs when
//!   there are any, else share that one.
//! - With none (a pod with one CPU), no helper thread is put on the polling
//!   CPU behind the application's back. `connect_background` returns
//!   [`CpuError::NoSpareCpu`]; with `Config::inline_connect_fallback` it
//!   connects as `Driver::connect` does instead (the handshake on the
//!   polling thread). `uring-sqpoll` fails at `Driver::new`. The warning
//!   thread shares the polling CPU, parked until there is work.
//!   `Driver::new` says all this in one warning line.

use std::fmt;
use std::time::Duration;

use ergon_tune::{CpuSet, CpuSetError, Host};

use crate::backend::{Backend, Config};

/// The environment variable naming the helper CPUs when
/// `Config::helper_cpus` is empty: a kernel CPU list (`3`, `3,7`, `2-3`).
pub const HELPER_CPUS_ENV: &str = "ERGON_HELPER_CPUS";

/// How often `Driver::connect_background`'s dial helper steps a dial that is
/// in flight, when it has a CPU of its own: low connect latency at 20,000
/// wakeups a second, paid on its own CPU.
pub const DIAL_STEP_OWN_CPU: Duration = Duration::from_micros(50);

/// [`DIAL_STEP_OWN_CPU`] when the dial helper shares `uring-sqpoll`'s
/// `SQPOLL` CPU: there every wakeup is a preemption of the spinner, so the
/// steps are rarer (a background connect is not the hot path).
pub const DIAL_STEP_SHARED_CPU: Duration = Duration::from_millis(1);

/// Where the process's CPUs ([`Placement::allowed`]) were read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Allowed {
    /// The process's cgroup cpuset: this file.
    Cgroup(String),
    /// No cgroup cpuset could be read: the main thread's affinity
    /// (`/proc/self/status`).
    MainThread,
    /// Neither could be read: the online CPUs.
    Online,
    /// Nothing could be read (not Linux, or a fixture tree without the
    /// files).
    Unknown,
}

impl fmt::Display for Allowed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cgroup(path) => write!(f, "its cgroup cpuset, {path}"),
            Self::MainThread => {
                f.write_str("its main thread's affinity: no cgroup cpuset could be read")
            }
            Self::Online => f.write_str(
                "the online CPUs: neither a cgroup cpuset nor the main thread's affinity could be \
                 read",
            ),
            Self::Unknown => f.write_str("unknown: none of its CPU files could be read"),
        }
    }
}

/// Why the helper CPUs are [`Placement::cpus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Why {
    /// `Config::helper_cpus`.
    Configured,
    /// [`HELPER_CPUS_ENV`].
    Environment,
    /// The polling CPU's SMT sibling.
    Sibling,
    /// The process's CPUs the host does not isolate (not the polling CPU's,
    /// when it is known).
    Housekeeping,
    /// The process's CPUs left after that, all isolated on the host (not
    /// the polling CPU's, when it is known).
    Isolated,
    /// The process may use the polling CPU only: none.
    NoSpareCpu,
}

impl fmt::Display for Why {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Configured => "Config::helper_cpus",
            Self::Environment => HELPER_CPUS_ENV,
            Self::Sibling => "the polling CPU's SMT sibling",
            Self::Housekeeping => "the process's CPUs the host does not isolate",
            Self::Isolated => "the process's CPUs, all isolated on the host",
            Self::NoSpareCpu => "none: the process may use the polling CPU only",
        })
    }
}

/// Where ergon-io's helper threads run, as `Driver::new` chose it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The CPUs the process may use.
    pub allowed: CpuSet,
    /// Where they were read.
    pub source: Allowed,
    /// The polling CPU, when known (`Config::loop_cpu`, or the one CPU the
    /// thread creating the driver is pinned to).
    pub polling: Option<u32>,
    /// The helper CPUs: a subset of `allowed` without `polling`; empty when
    /// there is none.
    pub cpus: CpuSet,
    /// Why these.
    pub why: Why,
}

/// A helper thread that needs a CPU of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HelperThread {
    /// `Driver::connect_background`'s dial helper.
    Dial,
    /// `uring-sqpoll`'s `SQPOLL` kernel thread.
    SqPoll,
}

/// Why a helper thread has no CPU: [`crate::Error::Cpu`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuError {
    /// The process may use the polling CPU only (a pod with one CPU), and
    /// `helper` would time-slice the polling thread there.
    NoSpareCpu {
        /// The thread that has no CPU.
        helper: HelperThread,
        /// The polling CPU, when known.
        polling: Option<u32>,
        /// The CPUs the process may use.
        allowed: CpuSet,
    },
    /// `setting` names `cpu`, which the process may not use: the kernel
    /// refuses a thread there (EINVAL).
    Outside {
        /// `loop_cpu` (or the creating thread's pinned CPU),
        /// `helper_cpus`, `ERGON_HELPER_CPUS`, `connect_cpu` or
        /// `sqpoll_cpu`.
        setting: &'static str,
        /// The CPU.
        cpu: u32,
        /// The CPUs the process may use.
        allowed: CpuSet,
    },
    /// `setting` puts a helper thread on the polling CPU, `cpu`.
    PollingCpu {
        /// `helper_cpus`, `ERGON_HELPER_CPUS` or `connect_cpu`.
        setting: &'static str,
        /// The polling CPU.
        cpu: u32,
    },
    /// Every CPU the process may use besides the polling CPU is in the
    /// explicit helper list `setting` names (the shared-work CPUs a node
    /// plugin adds): `helper` (a spinner) must not default onto them.
    NoExclusiveCpu {
        /// The thread that has no CPU ([`HelperThread::SqPoll`]).
        helper: HelperThread,
        /// `helper_cpus` or [`HELPER_CPUS_ENV`].
        setting: &'static str,
        /// The helper CPUs of that list.
        helpers: CpuSet,
        /// The polling CPU, when known.
        polling: Option<u32>,
        /// The CPUs the process may use.
        allowed: CpuSet,
    },
    /// `setting`'s value is not a CPU list.
    BadList {
        /// [`HELPER_CPUS_ENV`].
        setting: &'static str,
        /// The value.
        value: String,
        /// Why it did not parse.
        error: CpuSetError,
    },
}

/// `CPU 3`, `CPUs 2-3`, or `no CPU` for the empty set.
fn named(set: &CpuSet) -> String {
    match set.len() {
        0 => "no CPU".to_owned(),
        1 => format!("CPU {set}"),
        _ => format!("CPUs {set}"),
    }
}

const SECOND_CPU: &str = "give the pod a second CPU (Guaranteed QoS, the kubelet's static CPU \
                          manager policy, a whole-number CPU request of 2)";

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let polling = |p: &Option<u32>| p.map_or_else(String::new, |p| format!(" {p}"));
        match self {
            Self::NoSpareCpu {
                helper: HelperThread::Dial,
                polling: p,
                allowed,
            } => write!(
                f,
                "connect_background needs a CPU other than the polling CPU{} for its helper \
                 thread, and this process may use {} only (a pod with one CPU): the helper would \
                 time-slice the polling thread. Use Driver::connect, whose handshake runs on the \
                 polling thread (about 0.7 ms of CPU per TLS handshake, once per connect or \
                 reconnect), set inline_connect_fallback=true to have connect_background do \
                 that, or {SECOND_CPU}",
                polling(p),
                named(allowed)
            ),
            Self::NoSpareCpu {
                helper: HelperThread::SqPoll,
                polling: p,
                allowed,
            } => write!(
                f,
                "uring-sqpoll needs a CPU other than the polling CPU{} for its SQPOLL kernel \
                 thread, which spins, and this process may use {} only (a pod with one CPU). Use \
                 recv-busy, which polls the NIC from the polling thread, or {SECOND_CPU}",
                polling(p),
                named(allowed)
            ),
            Self::Outside {
                setting,
                cpu,
                allowed,
            } => write!(
                f,
                "{setting}: CPU {cpu} is not one this process may use ({}, its cpuset): the \
                 kernel refuses a thread there (EINVAL). Name one of those, or have the node add \
                 CPU {cpu} to the pod's cpuset",
                named(allowed)
            ),
            Self::PollingCpu { setting, cpu } => write!(
                f,
                "{setting}: CPU {cpu} is the polling CPU, where a helper thread would time-slice \
                 the polling thread. Name another of this process's CPUs"
            ),
            Self::NoExclusiveCpu {
                helper,
                setting,
                helpers,
                polling: p,
                allowed,
            } => {
                let who = match helper {
                    HelperThread::Dial => "connect_background's dial helper",
                    HelperThread::SqPoll => "uring-sqpoll's SQPOLL kernel thread (which spins)",
                };
                write!(
                    f,
                    "{who} needs a CPU of its own: {setting}'s {} ({setting}) are the shared-work \
                     CPUs that list names (never a spinner or SQPOLL there), while beside the \
                     polling CPU{} this process may use {} only. Set sqpoll_cpu to a CPU of the \
                     process outside that list, use recv-busy (which polls the NIC from the \
                     polling thread; per_queue for many sockets), or {SECOND_CPU}",
                    named(helpers),
                    polling(p),
                    named(allowed)
                )
            }
            Self::BadList {
                setting,
                value,
                error,
            } => write!(
                f,
                "{setting}={value:?} is not a CPU list ({error}): write it as the kernel does, \
                 such as 3, 3,7 or 2-3"
            ),
        }
    }
}

impl std::error::Error for CpuError {}

/// Chooses the helper CPUs on `host` for a driver configured with
/// `configured` (`Config::helper_cpus`) and the environment's
/// [`HELPER_CPUS_ENV`] value `env`, the polling CPU `polling`.
///
/// Reads files only (the process's cgroup, the polling CPU's
/// `thread_siblings_list`, the host's isolated CPUs): call it before the
/// polling loop starts.
///
/// # Errors
/// [`CpuError::BadList`] for an `env` that is not a CPU list;
/// [`CpuError::Outside`] for a `polling` CPU or an explicit-list CPU the
/// process may not use; [`CpuError::PollingCpu`] for an explicit list
/// with the polling CPU only.
pub fn of(
    host: &Host,
    configured: &[u32],
    env: Option<&str>,
    polling: Option<u32>,
) -> Result<Placement, CpuError> {
    let explicit = if configured.is_empty() {
        match env.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => {
                let list = CpuSet::parse_list(v).map_err(|error| CpuError::BadList {
                    setting: HELPER_CPUS_ENV,
                    value: v.to_owned(),
                    error,
                })?;
                Some((list, HELPER_CPUS_ENV, Why::Environment)).filter(|(l, _, _)| !l.is_empty())
            }
            None => None,
        }
    } else {
        Some((
            configured.iter().copied().collect(),
            "helper_cpus",
            Why::Configured,
        ))
    };
    let (allowed, source) = process_cpus(host);
    // The polling CPU must be one the process may use: `Config::loop_cpu`
    // outside the cpuset would make the choice believe the loop is
    // elsewhere and take its real CPU (the kernel runs the thread inside
    // the cpuset regardless).
    if let Some(cpu) = polling
        && !allowed.is_empty()
        && !allowed.contains(cpu)
    {
        return Err(CpuError::Outside {
            setting: "loop_cpu",
            cpu,
            allowed,
        });
    }
    let siblings = polling
        .and_then(|p| {
            read_set(
                host,
                &format!("/sys/devices/system/cpu/cpu{p}/topology/thread_siblings_list"),
            )
        })
        .unwrap_or_default();
    // `isolcpus`, and cgroup v2 isolated partitions (the root cgroup's file:
    // absent in a container).
    let isolated = read_set(host, "/sys/devices/system/cpu/isolated")
        .unwrap_or_default()
        .union(&read_set(host, "/sys/fs/cgroup/cpuset.cpus.isolated").unwrap_or_default());
    let (cpus, why) = choose(&allowed, polling, explicit, &siblings, &isolated)?;
    Ok(Placement {
        allowed,
        source,
        polling,
        cpus,
        why,
    })
}

/// The text of `abs` on `host`; `None` when it cannot be read (and for the
/// running host on a system other than Linux).
fn read(host: &Host, abs: &str) -> Option<String> {
    if host.is_live() && !cfg!(target_os = "linux") {
        return None;
    }
    std::fs::read_to_string(host.path(abs)).ok()
}

/// The CPU list in `abs`, when it can be read and parses.
fn read_set(host: &Host, abs: &str) -> Option<CpuSet> {
    read(host, abs).and_then(|t| CpuSet::parse_list(&t).ok())
}

/// The CPUs the process may use on `host`, and where they were read: the
/// cgroup cpuset, else the main thread's affinity, else the online CPUs.
#[must_use]
pub fn process_cpus(host: &Host) -> (CpuSet, Allowed) {
    if let Some(found) = read(host, "/proc/self/cgroup").and_then(|t| cgroup_cpus(host, &t)) {
        return found;
    }
    let main = read(host, "/proc/self/status").and_then(|t| {
        t.lines()
            .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
            .and_then(|l| CpuSet::parse_list(l).ok())
    });
    if let Some(cpus) = main.filter(|c| !c.is_empty()) {
        return (cpus, Allowed::MainThread);
    }
    read_set(host, "/sys/devices/system/cpu/online")
        .filter(|c| !c.is_empty())
        .map_or_else(
            || (CpuSet::new(), Allowed::Unknown),
            |cpus| (cpus, Allowed::Online),
        )
}

/// The cpuset of the cgroup `/proc/self/cgroup` (`text`) names, or of its
/// nearest parent that has one: cgroup v1's `cpuset` hierarchy first (a
/// hybrid host keeps the controller there), then cgroup v2's.
///
/// Its path is relative to the process's cgroup namespace, and
/// `/sys/fs/cgroup` is mounted at that namespace's root, so the same rule
/// finds a container's own cgroup (`0::/`, its own namespace) and one seen
/// from the host's.
fn cgroup_cpus(host: &Host, text: &str) -> Option<(CpuSet, Allowed)> {
    let v1 = text.lines().find_map(|l| {
        let mut f = l.splitn(3, ':');
        let (_, controllers, path) = (f.next()?, f.next()?, f.next()?);
        controllers
            .split(',')
            .any(|c| c == "cpuset")
            .then_some(path)
    });
    let v2 = text.lines().find_map(|l| l.strip_prefix("0::"));
    let files = v1
        .into_iter()
        .flat_map(|p| {
            ancestors(p).into_iter().flat_map(|d| {
                ["cpuset.effective_cpus", "cpuset.cpus"]
                    .map(|f| format!("/sys/fs/cgroup/cpuset{d}/{f}"))
            })
        })
        .chain(v2.into_iter().flat_map(|p| {
            ancestors(p)
                .into_iter()
                .map(|d| format!("/sys/fs/cgroup{d}/cpuset.cpus.effective"))
        }));
    for abs in files {
        if let Some(cpus) = read_set(host, &abs).filter(|c| !c.is_empty()) {
            return Some((cpus, Allowed::Cgroup(abs)));
        }
    }
    None
}

/// `path` and each of its parents, the root as the empty string: `/a/b`
/// gives `/a/b`, `/a` and the empty string.
fn ancestors(path: &str) -> Vec<String> {
    let mut dir = path.trim().trim_end_matches('/').to_owned();
    let mut out = vec![dir.clone()];
    while let Some(i) = dir.rfind('/') {
        dir.truncate(i);
        out.push(dir.clone());
    }
    out
}

/// The helper CPUs within `allowed` for polling CPU `polling`, given an
/// explicit list (and the setting it came from), the polling CPU's SMT
/// siblings and the host's isolated CPUs. An empty `allowed` (unknown)
/// takes an explicit list as it is.
pub(crate) fn choose(
    allowed: &CpuSet,
    polling: Option<u32>,
    explicit: Option<(CpuSet, &'static str, Why)>,
    siblings: &CpuSet,
    isolated: &CpuSet,
) -> Result<(CpuSet, Why), CpuError> {
    let me = polling.map_or_else(CpuSet::new, |p| CpuSet::of([p]));
    if let Some((list, setting, why)) = explicit {
        if let Some(cpu) = list
            .iter()
            .find(|&c| !allowed.is_empty() && !allowed.contains(c))
        {
            return Err(CpuError::Outside {
                setting,
                cpu,
                allowed: allowed.clone(),
            });
        }
        let rest = list.difference(&me);
        return match (rest.is_empty(), polling) {
            (true, Some(cpu)) => Err(CpuError::PollingCpu { setting, cpu }),
            _ => Ok((rest, why)),
        };
    }
    let sibling = siblings.intersection(allowed).difference(&me);
    if !sibling.is_empty() {
        return Ok((sibling, Why::Sibling));
    }
    let others = allowed.difference(&me);
    let housekeeping = others.difference(isolated);
    Ok(if !housekeeping.is_empty() {
        (housekeeping, Why::Housekeeping)
    } else if others.is_empty() {
        (others, Why::NoSpareCpu)
    } else {
        (others, Why::Isolated)
    })
}

/// `cfg`'s `SQPOLL` CPU, for `uring-sqpoll`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // the driver is Linux-only
pub(crate) const fn sqpoll_of(cfg: &Config) -> Option<u32> {
    match cfg.backend {
        Backend::UringSqpoll => cfg.sqpoll_cpu,
        _ => None,
    }
}

/// The CPUs for the dial helper and the warning thread: the helper CPUs,
/// without the `SQPOLL` CPU `sqpoll` when others remain.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn thread_cpus(p: &Placement, sqpoll: Option<u32>) -> Vec<usize> {
    let mut cpus = p.cpus.clone();
    if let Some(s) = sqpoll {
        let others = cpus.difference(&CpuSet::of([s]));
        if !others.is_empty() {
            cpus = others;
        }
    }
    cpus.iter()
        .filter_map(|c| usize::try_from(c).ok())
        .collect()
}

/// The dial helper's CPUs for a driver configured as `cfg` (its
/// `sqpoll_cpu` already chosen): `connect_cpu`, else [`thread_cpus`].
///
/// # Errors
/// [`CpuError::Outside`] or [`CpuError::PollingCpu`] for a `connect_cpu`
/// the process may not use, or that is the polling CPU;
/// [`CpuError::NoSpareCpu`] when there is no helper CPU.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn dial_cpus(p: &Placement, cfg: &Config) -> Result<Vec<usize>, CpuError> {
    if let Some(cpu) = cfg.connect_cpu {
        if !p.allowed.is_empty() && !p.allowed.contains(cpu) {
            return Err(CpuError::Outside {
                setting: "connect_cpu",
                cpu,
                allowed: p.allowed.clone(),
            });
        }
        if p.polling == Some(cpu) {
            return Err(CpuError::PollingCpu {
                setting: "connect_cpu",
                cpu,
            });
        }
        return Ok(usize::try_from(cpu).into_iter().collect());
    }
    let cpus = thread_cpus(p, sqpoll_of(cfg));
    if cpus.is_empty() {
        return Err(CpuError::NoSpareCpu {
            helper: HelperThread::Dial,
            polling: p.polling,
            allowed: p.allowed.clone(),
        });
    }
    Ok(cpus)
}

/// The dial helper's step interval for its CPU set `cpus`
/// ([`dial_cpus`]): [`DIAL_STEP_OWN_CPU`] on a CPU of its own,
/// [`DIAL_STEP_SHARED_CPU`] when its only CPU is `sqpoll` (the `SQPOLL`
/// kernel thread's spinning one).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))] // the driver is Linux-only
pub(crate) const fn dial_step(cpus: &[usize], sqpoll: Option<usize>) -> Duration {
    if let ([only], Some(sqpoll)) = (cpus, sqpoll)
        && *only == sqpoll
    {
        DIAL_STEP_SHARED_CPU
    } else {
        DIAL_STEP_OWN_CPU
    }
}

/// `uring-sqpoll`'s `SQPOLL` CPU: `configured` (`Config::sqpoll_cpu`), else
/// the default of [`default_sqpoll`]. A configured polling CPU is allowed
/// (and warned about by T8).
///
/// # Errors
/// [`CpuError::Outside`] for a `configured` CPU the process may not use;
/// [`CpuError::NoSpareCpu`] when there is no helper CPU;
/// [`CpuError::NoExclusiveCpu`] when an explicit helper list holds every
/// CPU but the polling one.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn sqpoll_cpu(p: &Placement, configured: Option<u32>) -> Result<u32, CpuError> {
    match configured {
        Some(cpu) if !p.allowed.is_empty() && !p.allowed.contains(cpu) => Err(CpuError::Outside {
            setting: "sqpoll_cpu",
            cpu,
            allowed: p.allowed.clone(),
        }),
        Some(cpu) => Ok(cpu),
        None => default_sqpoll(p),
    }
}

/// The `SQPOLL` CPU with no `Config::sqpoll_cpu`: the first helper CPU,
/// except for an explicit helper list ([`Why::Configured`] /
/// [`Why::Environment`], the shared-work CPUs a node plugin adds — never a
/// spinner or `SQPOLL` there), where it is the lowest CPU of the process's
/// own outside that list and the polling CPU.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn default_sqpoll(p: &Placement) -> Result<u32, CpuError> {
    let no_spare = || CpuError::NoSpareCpu {
        helper: HelperThread::SqPoll,
        polling: p.polling,
        allowed: p.allowed.clone(),
    };
    if p.cpus.is_empty() {
        return Err(no_spare());
    }
    if matches!(p.why, Why::Configured | Why::Environment) && !p.allowed.is_empty() {
        let me = p.polling.map_or_else(CpuSet::new, |cpu| CpuSet::of([cpu]));
        let own = p.allowed.difference(&p.cpus).difference(&me);
        if let Some(cpu) = own.iter().next() {
            return Ok(cpu);
        }
        return Err(CpuError::NoExclusiveCpu {
            helper: HelperThread::SqPoll,
            setting: match p.why {
                Why::Configured => "helper_cpus",
                _ => HELPER_CPUS_ENV,
            },
            helpers: p.cpus.clone(),
            polling: p.polling,
            allowed: p.allowed.clone(),
        });
    }
    p.cpus.iter().next().ok_or_else(no_spare)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ergon_tune::{CpuSet, Host};

    use super::{
        Allowed, CpuError, HELPER_CPUS_ENV, HelperThread, Placement, Why, choose, dial_cpus, of,
        process_cpus, sqpoll_cpu, thread_cpus,
    };
    use crate::backend::{Backend, Config};

    type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn set(cpus: &[u32]) -> CpuSet {
        CpuSet::of(cpus.iter().copied())
    }

    /// The helper CPUs with no explicit list and no SMT sibling.
    fn pick(isolated: &[u32], allowed: &[u32], polling: Option<u32>) -> R<CpuSet> {
        Ok(choose(&set(allowed), polling, None, &CpuSet::new(), &set(isolated))?.0)
    }

    const EIGHT: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    const TEN: [u32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    /// A node booted with `isolcpus=2-7` whose static CPU manager gives a
    /// Guaranteed pod CPUs 2 and 3, the loop on 2: background work must go
    /// to 3, the pod's other CPU. The host's housekeeping CPUs (0, 1) are
    /// outside the cpuset, where the helper's `pthread_create` and
    /// `io_uring_setup` fail with EINVAL.
    #[test]
    fn a_two_cpu_pod_on_isolated_cpus_gets_its_other_cpu() -> R {
        assert_eq!(pick(&[2, 3, 4, 5, 6, 7], &[2, 3], Some(2))?, set(&[3]));
        Ok(())
    }

    /// A pod given CPUs 2 and 3 on a node without `isolcpus` (Docker
    /// `--cpuset-cpus=2,3` on 10 CPUs): `SQPOLL` takes the first CPU alone,
    /// so it must be 3. CPU 0 made `io_uring_setup` fail with EINVAL (Linux
    /// 6.12); the helper only started because the kernel cut its mask down
    /// to the cpuset.
    #[test]
    fn a_two_cpu_pods_first_choice_is_inside_its_cpuset() -> R {
        assert_eq!(pick(&[], &[2, 3], Some(2))?, set(&[3]));
        Ok(())
    }

    /// A pod with one CPU, the loop's (Docker `--cpuset-cpus=0`): no CPU to
    /// spare, never one outside the cpuset (EINVAL) nor the loop's own.
    #[test]
    fn a_single_cpu_pod_has_no_cpu_to_spare() -> R {
        let (cpus, why) = choose(&set(&[0]), Some(0), None, &CpuSet::new(), &CpuSet::new())?;
        assert_eq!((cpus, why), (CpuSet::new(), Why::NoSpareCpu));
        Ok(())
    }

    /// A cpuset holding a housekeeping CPU (1) and isolated ones (2, 3):
    /// the housekeeping one inside it, not CPU 0 outside it.
    #[test]
    fn a_cpuset_with_a_housekeeping_cpu_keeps_to_it() -> R {
        assert_eq!(pick(&[2, 3, 4, 5, 6, 7], &[1, 2, 3], Some(2))?, set(&[1]));
        Ok(())
    }

    /// No cpuset limit (a process allowed every CPU): the host's
    /// housekeeping CPUs, as before.
    #[test]
    fn without_a_cpuset_limit_the_host_housekeeping_cpus_are_used() -> R {
        assert_eq!(pick(&[2, 3, 4, 5, 6, 7], &EIGHT, Some(2))?, set(&[0, 1]));
        assert_eq!(pick(&[], &TEN, None)?, set(&TEN), "no polling CPU known");
        Ok(())
    }

    /// x86, `cpu: 2` with `full-pcpus-only`: the pod holds one core's two
    /// hyperthreads, 2 and 6; the loop on 2 leaves its sibling.
    #[test]
    fn an_x86_pod_with_one_whole_core_gets_the_loops_sibling() -> R {
        let (cpus, why) = choose(&set(&[2, 6]), Some(2), None, &set(&[2, 6]), &set(&[2, 6]))?;
        assert_eq!((cpus, why), (set(&[6]), Why::Sibling));
        Ok(())
    }

    /// Two whole cores (2+6, 3+7), loops on the even siblings: each loop's
    /// helpers go on its own sibling, never on the other loop's CPU (3).
    #[test]
    fn with_two_cores_the_helpers_keep_to_the_loops_sibling() -> R {
        let pod = set(&[2, 3, 6, 7]);
        assert_eq!(
            choose(&pod, Some(2), None, &set(&[2, 6]), &CpuSet::new())?,
            (set(&[6]), Why::Sibling)
        );
        assert_eq!(
            choose(&pod, Some(3), None, &set(&[3, 7]), &CpuSet::new())?,
            (set(&[7]), Why::Sibling)
        );
        Ok(())
    }

    /// A sibling outside the pod (it holds CPUs 2 and 3 of different
    /// cores, or Graviton: no SMT) is not used: the pod's other CPU is.
    #[test]
    fn a_sibling_the_pod_does_not_hold_is_not_used() -> R {
        assert_eq!(
            choose(&set(&[2, 3]), Some(2), None, &set(&[2, 6]), &CpuSet::new())?,
            (set(&[3]), Why::Housekeeping)
        );
        assert_eq!(
            choose(&set(&[2, 3]), Some(2), None, &set(&[2]), &set(&[2, 3]))?,
            (set(&[3]), Why::Isolated),
            "Graviton: no sibling, the other core"
        );
        Ok(())
    }

    /// An explicit list wins over the sibling, must lie inside the
    /// process's CPUs, and loses the polling CPU.
    #[test]
    fn an_explicit_list_wins_and_must_be_inside_the_cpuset() -> R {
        let pod = set(&[2, 3, 4, 6]);
        let ask = |cpus: &[u32]| {
            choose(
                &pod,
                Some(2),
                Some((set(cpus), "helper_cpus", Why::Configured)),
                &set(&[2, 6]),
                &CpuSet::new(),
            )
        };
        assert_eq!(ask(&[4])?, (set(&[4]), Why::Configured));
        assert_eq!(ask(&[2, 4])?, (set(&[4]), Why::Configured));
        assert_eq!(
            ask(&[4, 9]),
            Err(CpuError::Outside {
                setting: "helper_cpus",
                cpu: 9,
                allowed: pod.clone()
            })
        );
        assert_eq!(
            ask(&[2]),
            Err(CpuError::PollingCpu {
                setting: "helper_cpus",
                cpu: 2
            })
        );
        Ok(())
    }

    /// A host tree under a temporary directory, removed when dropped.
    struct Tree(PathBuf);

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl Tree {
        fn new(name: &str) -> R<Self> {
            let root = std::env::temp_dir()
                .join(format!("ergon-io-placement-{name}-{}", std::process::id()));
            if root.exists() {
                std::fs::remove_dir_all(&root)?;
            }
            std::fs::create_dir_all(&root)?;
            Ok(Self(root))
        }

        fn file(&self, abs: &str, text: &str) -> R<&Self> {
            let p = self.0.join(abs.trim_start_matches('/'));
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(p, text)?;
            Ok(self)
        }

        fn host(&self) -> Host {
            Host::at(&self.0)
        }

        /// A 10-CPU host, CPU c's SMT sibling c ^ 1 (pairs 0+1, 2+3, ...).
        fn smt10(name: &str) -> R<Self> {
            let t = Self::new(name)?;
            t.file("/sys/devices/system/cpu/online", "0-9\n")?
                .file("/sys/devices/system/cpu/isolated", "\n")?;
            for c in 0..10u32 {
                let pair = c & !1;
                t.file(
                    &format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list"),
                    &format!("{pair}-{}\n", pair + 1),
                )?;
            }
            Ok(t)
        }
    }

    /// cgroup v2 with a private cgroup namespace (Docker, containerd on
    /// cgroup v2): the container's cgroup is `/`.
    #[test]
    fn the_cpuset_of_a_container_with_its_own_cgroup_namespace() -> R {
        let t = Tree::smt10("v2-private")?;
        t.file("/proc/self/cgroup", "0::/\n")?
            .file("/sys/fs/cgroup/cpuset.cpus.effective", "2-3\n")?
            .file("/proc/self/status", "Name:\tapp\nCpus_allowed_list:\t2\n")?;
        let (cpus, from) = process_cpus(&t.host());
        assert_eq!(cpus, set(&[2, 3]));
        assert_eq!(
            from,
            Allowed::Cgroup("/sys/fs/cgroup/cpuset.cpus.effective".to_owned())
        );
        Ok(())
    }

    /// cgroup v2 seen from the host's cgroup namespace: the container's
    /// path, and a parent's cpuset when the container's own has no cpuset
    /// files.
    #[test]
    fn the_cpuset_of_a_cgroup_path_or_its_nearest_parent() -> R {
        let t = Tree::smt10("v2-host")?;
        t.file("/proc/self/cgroup", "0::/kubepods.slice/pod1/cri-1\n")?
            .file("/sys/fs/cgroup/cpuset.cpus.effective", "0-9\n")?
            .file(
                "/sys/fs/cgroup/kubepods.slice/pod1/cpuset.cpus.effective",
                "4,5\n",
            )?
            .file(
                "/sys/fs/cgroup/kubepods.slice/pod1/cri-1/cgroup.procs",
                "1\n",
            )?;
        let (cpus, from) = process_cpus(&t.host());
        assert_eq!(cpus, set(&[4, 5]));
        assert_eq!(
            from,
            Allowed::Cgroup("/sys/fs/cgroup/kubepods.slice/pod1/cpuset.cpus.effective".to_owned())
        );
        Ok(())
    }

    /// cgroup v1 (a hybrid host keeps the cpuset controller there).
    #[test]
    fn the_cpuset_of_a_cgroup_v1_host() -> R {
        let t = Tree::smt10("v1")?;
        t.file(
            "/proc/self/cgroup",
            "12:memory:/kubepods/pod1/c1\n5:cpuset,foo:/kubepods/pod1/c1\n0::/\n",
        )?
        .file(
            "/sys/fs/cgroup/cpuset/kubepods/pod1/c1/cpuset.effective_cpus",
            "7\n",
        )?;
        let (cpus, from) = process_cpus(&t.host());
        assert_eq!(cpus, set(&[7]));
        assert_eq!(
            from,
            Allowed::Cgroup(
                "/sys/fs/cgroup/cpuset/kubepods/pod1/c1/cpuset.effective_cpus".to_owned()
            )
        );
        Ok(())
    }

    /// No cgroup cpuset: the main thread's affinity, then the online CPUs.
    #[test]
    fn without_a_cgroup_cpuset_the_main_thread_then_the_online_cpus() -> R {
        let t = Tree::smt10("fallbacks")?;
        assert_eq!(process_cpus(&t.host()), (set(&TEN), Allowed::Online));
        t.file("/proc/self/status", "Name:\tapp\nCpus_allowed_list:\t1-2\n")?;
        assert_eq!(process_cpus(&t.host()), (set(&[1, 2]), Allowed::MainThread));
        let none = Tree::new("nothing")?;
        assert_eq!(
            process_cpus(&none.host()),
            (CpuSet::new(), Allowed::Unknown)
        );
        Ok(())
    }

    /// A pod with CPUs 2 and 3, a core's two hyperthreads, the loop on 2:
    /// end to end, its sibling 3.
    #[test]
    fn a_two_cpu_x86_pod_end_to_end() -> R {
        let t = Tree::smt10("pod2")?;
        t.file("/proc/self/cgroup", "0::/\n")?
            .file("/sys/fs/cgroup/cpuset.cpus.effective", "2-3\n")?;
        let p = of(&t.host(), &[], None, Some(2))?;
        assert_eq!(
            p,
            Placement {
                allowed: set(&[2, 3]),
                source: Allowed::Cgroup("/sys/fs/cgroup/cpuset.cpus.effective".to_owned()),
                polling: Some(2),
                cpus: set(&[3]),
                why: Why::Sibling,
            }
        );
        Ok(())
    }

    /// A pod with one CPU: no helper CPU, and every helper's refusal says
    /// why and what to do.
    #[test]
    fn a_single_cpu_pod_end_to_end() -> R {
        let t = Tree::smt10("pod1")?;
        t.file("/proc/self/cgroup", "0::/\n")?
            .file("/sys/fs/cgroup/cpuset.cpus.effective", "0\n")?;
        let p = of(&t.host(), &[], None, Some(0))?;
        assert_eq!((p.cpus.clone(), p.why), (CpuSet::new(), Why::NoSpareCpu));
        let dial = dial_cpus(&p, &Config::new(Backend::EpollSpin));
        assert_eq!(
            dial,
            Err(CpuError::NoSpareCpu {
                helper: HelperThread::Dial,
                polling: Some(0),
                allowed: set(&[0])
            })
        );
        let text = dial.err().map(|e| e.to_string()).unwrap_or_default();
        for part in ["Driver::connect", "inline_connect_fallback", "second CPU"] {
            assert!(text.contains(part), "{part:?} not in {text:?}");
        }
        let sq = sqpoll_cpu(&p, None);
        assert_eq!(
            sq,
            Err(CpuError::NoSpareCpu {
                helper: HelperThread::SqPoll,
                polling: Some(0),
                allowed: set(&[0])
            })
        );
        let text = sq.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(text.contains("recv-busy"), "{text}");
        Ok(())
    }

    /// A `loop_cpu` outside the cpuset is refused: the polling thread runs
    /// inside the cpuset anyway (the kernel refuses to run outside it), so
    /// without this the helper choice would believe the loop is elsewhere
    /// and take its real CPU.
    #[test]
    fn a_loop_cpu_outside_the_cpuset_is_refused() -> R {
        let t = Tree::smt10("loopcpu")?;
        t.file("/proc/self/cgroup", "0::/\n")?
            .file("/sys/fs/cgroup/cpuset.cpus.effective", "2-3\n")?;
        assert_eq!(
            of(&t.host(), &[], None, Some(7)),
            Err(CpuError::Outside {
                setting: "loop_cpu",
                cpu: 7,
                allowed: set(&[2, 3])
            })
        );
        assert!(of(&t.host(), &[], None, Some(2)).is_ok(), "inside it");
        Ok(())
    }

    /// `Config::helper_cpus` wins over `ERGON_HELPER_CPUS`, which wins
    /// over the automatic choice; a malformed variable is an error.
    #[test]
    fn the_configured_list_then_the_environment() -> R {
        let t = Tree::smt10("explicit")?;
        t.file("/proc/self/cgroup", "0::/\n")?
            .file("/sys/fs/cgroup/cpuset.cpus.effective", "2-5\n")?;
        let h = t.host();
        let conf = of(&h, &[4], Some("5"), Some(2))?;
        assert_eq!((conf.cpus, conf.why), (set(&[4]), Why::Configured));
        let env = of(&h, &[], Some("4-5"), Some(2))?;
        assert_eq!((env.cpus, env.why), (set(&[4, 5]), Why::Environment));
        let blank = of(&h, &[], Some(" "), Some(2))?;
        assert_eq!(blank.why, Why::Sibling, "an empty variable is unset");
        assert!(
            matches!(
                of(&h, &[], Some("4,x"), Some(2)),
                Err(CpuError::BadList {
                    setting: HELPER_CPUS_ENV,
                    ..
                })
            ),
            "a malformed list"
        );
        assert_eq!(
            of(&h, &[], Some("8"), Some(2)),
            Err(CpuError::Outside {
                setting: HELPER_CPUS_ENV,
                cpu: 8,
                allowed: set(&[2, 3, 4, 5])
            })
        );
        Ok(())
    }

    fn placed(cpus: &[u32]) -> Placement {
        Placement {
            allowed: set(&[2, 3, 4, 5]),
            source: Allowed::MainThread,
            polling: Some(2),
            cpus: set(cpus),
            why: Why::Housekeeping,
        }
    }

    /// `connect_cpu` must be one of the process's CPUs and not the
    /// polling CPU; `sqpoll_cpu` must be one of the process's CPUs.
    #[test]
    fn explicit_helper_cpus_must_be_usable() -> R {
        let p = placed(&[3, 4, 5]);
        let mut cfg = Config::new(Backend::EpollSpin);
        cfg.connect_cpu = Some(7);
        assert_eq!(
            dial_cpus(&p, &cfg),
            Err(CpuError::Outside {
                setting: "connect_cpu",
                cpu: 7,
                allowed: set(&[2, 3, 4, 5])
            })
        );
        cfg.connect_cpu = Some(2);
        assert_eq!(
            dial_cpus(&p, &cfg),
            Err(CpuError::PollingCpu {
                setting: "connect_cpu",
                cpu: 2
            })
        );
        cfg.connect_cpu = Some(5);
        assert_eq!(dial_cpus(&p, &cfg)?, vec![5]);
        assert_eq!(
            sqpoll_cpu(&p, Some(9)),
            Err(CpuError::Outside {
                setting: "sqpoll_cpu",
                cpu: 9,
                allowed: set(&[2, 3, 4, 5])
            })
        );
        assert_eq!(sqpoll_cpu(&p, Some(4))?, 4);
        assert_eq!(sqpoll_cpu(&p, None)?, 3, "the first helper CPU");
        Ok(())
    }

    /// `SQPOLL` spins: the dial helper and the warning thread take the
    /// other helper CPUs when there are any, else share its CPU.
    #[test]
    fn the_other_helpers_leave_sqpolls_cpu_when_they_can() -> R {
        assert_eq!(thread_cpus(&placed(&[3, 4, 5]), Some(3)), vec![4, 5]);
        assert_eq!(thread_cpus(&placed(&[3]), Some(3)), vec![3]);
        assert_eq!(thread_cpus(&placed(&[3, 4]), None), vec![3, 4]);
        let mut cfg = Config::new(Backend::UringSqpoll);
        cfg.sqpoll_cpu = Some(3);
        assert_eq!(dial_cpus(&placed(&[3, 4]), &cfg)?, vec![4]);
        Ok(())
    }

    /// An explicit helper list is the shared-work CPUs a node plugin adds
    /// (never a spinner or `SQPOLL` there): with no `sqpoll_cpu`, `SQPOLL`
    /// takes the lowest CPU of the process's own outside that list.
    #[test]
    fn sqpoll_defaults_off_an_explicit_helper_list() -> R {
        let p = Placement {
            allowed: set(&[2, 3, 7]),
            source: Allowed::MainThread,
            polling: Some(2),
            cpus: set(&[7]),
            why: Why::Environment,
        };
        assert_eq!(sqpoll_cpu(&p, None)?, 3);
        let p = Placement {
            allowed: set(&[2, 3, 4, 7]),
            source: Allowed::MainThread,
            polling: Some(2),
            cpus: set(&[4, 7]),
            why: Why::Configured,
        };
        assert_eq!(sqpoll_cpu(&p, None)?, 3, "the lowest of the process's own");
        Ok(())
    }

    /// When the explicit helper list holds every CPU but the polling one,
    /// `SQPOLL` refuses rather than spin on a shared helper CPU. The
    /// message names `sqpoll_cpu` and the alternatives.
    #[test]
    fn sqpoll_refuses_rather_than_spin_on_the_shared_helpers() {
        let p = Placement {
            allowed: set(&[2, 7]),
            source: Allowed::MainThread,
            polling: Some(2),
            cpus: set(&[7]),
            why: Why::Configured,
        };
        assert_eq!(
            sqpoll_cpu(&p, None),
            Err(CpuError::NoExclusiveCpu {
                helper: HelperThread::SqPoll,
                setting: "helper_cpus",
                helpers: set(&[7]),
                polling: Some(2),
                allowed: set(&[2, 7])
            })
        );
        let text = sqpoll_cpu(&p, None)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        for part in ["helper_cpus", "sqpoll_cpu", "recv-busy"] {
            assert!(text.contains(part), "{part:?} not in {text:?}");
        }
        let p = Placement {
            why: Why::Environment,
            ..p
        };
        assert!(
            matches!(
                sqpoll_cpu(&p, None),
                Err(CpuError::NoExclusiveCpu {
                    setting: HELPER_CPUS_ENV,
                    ..
                })
            ),
            "{:?}",
            sqpoll_cpu(&p, None)
        );
    }

    /// An automatic choice still puts `SQPOLL` on the first helper CPU:
    /// the x86 layout is the loop's even SMT sibling taking `SQPOLL` and
    /// the dial helper.
    #[test]
    fn an_automatic_choice_keeps_sqpoll_on_the_first_helper_cpu() -> R {
        for why in [Why::Sibling, Why::Housekeeping, Why::Isolated] {
            let p = Placement {
                allowed: set(&[2, 3]),
                source: Allowed::MainThread,
                polling: Some(2),
                cpus: set(&[3]),
                why,
            };
            assert_eq!(sqpoll_cpu(&p, None)?, 3, "{why:?}");
        }
        Ok(())
    }
}
