//! T20: how deep the CPUs that take the data NICs' interrupts may sleep
//! (cpuidle), and the hot CPUs that may sleep at all.
//!
//! Reads `/sys/devices/system/cpu` only (unprivileged, visible from a pod).
//! `/dev/cpu_dma_latency` is never opened: opening it registers a
//! `pm_qos` request, which a read-only check must not do.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::cpuset::CpuSet;
use crate::finding::{Finding, HotSet, Severity};
use crate::host::{Host, ReadError, first_line};
use crate::net::{Nic, nic_irqs};

/// The deepest exit latency (µs) T20 accepts on a CPU that takes a data
/// NIC's interrupts: C1's. On `intel_idle` C1E (2 µs) is the first state
/// past it, and the first one the lab's cap disabled.
pub const T20_MAX_EXIT_LATENCY_US: u64 = 1;

const CPUIDLE: &str = "/sys/devices/system/cpu/cpuidle";

const T20_EVIDENCE: &str = "md-dpdk/BENCHMARKS.md, \"c7i: the IRQ CPU's idle depth\" (lab session \
     aws-20261028-092158: AWS c7i.4xlarge, intel_idle, kernel 7.2, mio TCP round trips at 10k \
     msg/s, 3 reps, each point's cpuidle usage read before and after); Linux \
     Documentation/admin-guide/pm/cpuidle.rst (state<N>/disable, pm_qos_resume_latency_us, \
     /dev/cpu_dma_latency) and intel_idle.rst (max_cstate)";

/// One cpuidle state of a CPU (`/sys/devices/system/cpu/cpu<N>/cpuidle/state<i>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdleState {
    /// `i` in `state<i>`: 0 is the shallowest (`POLL` on x86).
    pub index: u32,
    /// Its name (`POLL`, `C1`, `C1E`, `C6`).
    pub name: String,
    /// Its exit latency (µs): the longest a wake from it takes.
    pub exit_latency_us: u64,
    /// Whether it is disabled (`disable` 1): the governor never picks it.
    pub disabled: bool,
}

/// A CPU's resume latency limit (`pm_qos`)
/// (`/sys/devices/system/cpu/cpu<N>/power/pm_qos_resume_latency_us`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeLimit {
    /// `0`, or the file absent: no limit.
    None,
    /// `n/a`: no latency tolerated, polling only.
    PollOnly,
    /// The governor keeps to states whose exit latency is at most this (µs).
    Us(u64),
}

impl ResumeLimit {
    /// Whether a state with exit latency `us` may be entered under it.
    #[must_use]
    pub const fn allows(self, us: u64) -> bool {
        match self {
            Self::None => true,
            Self::PollOnly => false,
            Self::Us(limit) => us <= limit,
        }
    }
}

/// The cpuidle driver (`intel_idle`, `acpi_idle`, `haltpoll`), or `None`
/// when there is none (`none`, or a kernel without cpuidle: the hypervisor's
/// or the architecture's idle, such as Graviton's WFI).
///
/// # Errors
/// `/sys/devices/system/cpu` cannot be read, or the driver file cannot.
pub fn cpuidle_driver(host: &Host) -> Result<Option<String>, ReadError> {
    match host.read_optional(&format!("{CPUIDLE}/current_driver")) {
        Ok(text) => Ok(text
            .map(|t| first_line(&t))
            .filter(|d| !d.is_empty() && d != "none")),
        Err(_) if host.is_dir("/sys/devices/system/cpu") && !host.exists(CPUIDLE) => Ok(None),
        Err(e) => Err(e),
    }
}

fn number(host: &Host, abs: &str) -> Result<u64, ReadError> {
    host.read_line(abs)?
        .parse()
        .map_err(|_| ReadError::Malformed {
            path: abs.to_owned(),
            detail: "not a number",
        })
}

/// CPU `cpu`'s cpuidle states, shallowest first; none when it has no
/// `cpuidle` directory.
///
/// # Errors
/// A state's files cannot be read or do not parse.
pub fn cpu_idle_states(host: &Host, cpu: u32) -> Result<Vec<IdleState>, ReadError> {
    let dir = format!("/sys/devices/system/cpu/cpu{cpu}/cpuidle");
    if !host.is_dir(&dir) {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in host.list(&dir)? {
        let Some(index) = entry
            .strip_prefix("state")
            .and_then(|i| i.parse::<u32>().ok())
        else {
            continue;
        };
        let d = format!("{dir}/{entry}");
        out.push(IdleState {
            index,
            name: host.read_line(&format!("{d}/name"))?,
            exit_latency_us: number(host, &format!("{d}/latency"))?,
            disabled: number(host, &format!("{d}/disable"))? != 0,
        });
    }
    out.sort_by_key(|s| s.index);
    Ok(out)
}

/// CPU `cpu`'s resume latency limit (`pm_qos`).
///
/// # Errors
/// The file is there but cannot be read or does not parse.
pub fn resume_limit(host: &Host, cpu: u32) -> Result<ResumeLimit, ReadError> {
    let path = format!("/sys/devices/system/cpu/cpu{cpu}/power/pm_qos_resume_latency_us");
    let text = match host.read_optional(&path) {
        Ok(Some(t)) => first_line(&t),
        Ok(None) | Err(ReadError::Io { .. }) => return Ok(ResumeLimit::None),
        Err(e) => return Err(e),
    };
    if text == "n/a" {
        return Ok(ResumeLimit::PollOnly);
    }
    match text.parse::<u64>() {
        Ok(0) => Ok(ResumeLimit::None),
        Ok(us) => Ok(ResumeLimit::Us(us)),
        Err(_) => Err(ReadError::Malformed {
            path,
            detail: "not a number or n/a",
        }),
    }
}

/// The CPUs the interrupts of `nics` land on (each IRQ's effective
/// affinity, else its `smp_affinity_list`), each with its IRQs' names; one
/// unknown finding a NIC that cannot be mapped.
fn irq_cpus(host: &Host, nics: &[Nic], out: &mut Vec<Finding>) -> BTreeMap<u32, Vec<String>> {
    let mut cpus: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for nic in nics {
        let irqs = match nic_irqs(host, nic) {
            Ok(v) => v,
            Err(e) => {
                out.push(Finding::unknown(
                    "T20",
                    Severity::Warn,
                    T20_EVIDENCE,
                    format!("cannot tell which CPUs take {nic}'s interrupts"),
                    e,
                ));
                continue;
            }
        };
        for irq in irqs {
            let lands = host
                .read_cpu_list(&format!("/proc/irq/{}/effective_affinity_list", irq.irq))
                .ok()
                .filter(|c| !c.is_empty())
                .or_else(|| {
                    host.read_cpu_list(&format!("/proc/irq/{}/smp_affinity_list", irq.irq))
                        .ok()
                })
                .unwrap_or_default();
            let name = irq.action.clone().map_or_else(
                || format!("IRQ {}", irq.irq),
                |a| format!("{a} (IRQ {})", irq.irq),
            );
            for c in lands.iter() {
                cpus.entry(c).or_default().push(name.clone());
            }
        }
    }
    cpus
}

/// `C1E (exit 2 µs), C6 (exit 190 µs)`.
fn states_text(states: &[&IdleState]) -> String {
    states
        .iter()
        .map(|s| format!("{} (exit {} µs)", s.name, s.exit_latency_us))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The node-side fixes for CPU `cpu` whose states `deep` are too deep.
fn fixes(cpu: u32, deep: &[&IdleState], driver: &str) -> String {
    let mut per_cpu = deep
        .iter()
        .map(|s| {
            format!(
                "echo 1 > /sys/devices/system/cpu/cpu{cpu}/cpuidle/state{}/disable",
                s.index
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    per_cpu = format!("`{per_cpu}`");
    let boot = match driver {
        "intel_idle" => " at boot `intel_idle.max_cstate=1` (every CPU);",
        "acpi_idle" => " at boot `processor.max_cstate=1` (every CPU);",
        _ => "",
    };
    format!(
        "Fix (node, root; one of): {per_cpu} (until reboot); `echo {T20_MAX_EXIT_LATENCY_US} > \
         /sys/devices/system/cpu/cpu{cpu}/power/pm_qos_resume_latency_us` (this CPU, until \
         reboot);{boot} or a process holding /dev/cpu_dma_latency open with \
         {T20_MAX_EXIT_LATENCY_US} written to it (every CPU, while held). Kubernetes: a node \
         setting, never the pod's (/sys is read-only in a pod): the boot line, or a privileged \
         DaemonSet that writes it; a pod can hold /dev/cpu_dma_latency only privileged, with the \
         device mounted. A process already holding /dev/cpu_dma_latency is not seen here \
         (opening it registers a request, so it is not read)"
    )
}

/// What T20 found on one CPU.
enum CpuVerdict {
    /// States deeper than C1 it may enter.
    Deep(Vec<IdleState>),
    /// None: its deepest usable state, and the `pm_qos` limit if that is what
    /// keeps it there.
    Shallow(Option<IdleState>, ResumeLimit),
}

fn judge(host: &Host, cpu: u32) -> Result<CpuVerdict, ReadError> {
    let states = cpu_idle_states(host, cpu)?;
    let limit = resume_limit(host, cpu)?;
    let usable: Vec<&IdleState> = states
        .iter()
        .filter(|s| !s.disabled && limit.allows(s.exit_latency_us))
        .collect();
    let deep: Vec<IdleState> = usable
        .iter()
        .filter(|s| s.exit_latency_us > T20_MAX_EXIT_LATENCY_US)
        .map(|s| (*s).clone())
        .collect();
    if deep.is_empty() {
        Ok(CpuVerdict::Shallow(
            usable.last().map(|s| (*s).clone()),
            limit,
        ))
    } else {
        Ok(CpuVerdict::Deep(deep))
    }
}

/// `C1 (exit 1 µs)`, `polling only`, with the `pm_qos` limit when set.
fn shallow_text(deepest: Option<&IdleState>, limit: ResumeLimit) -> String {
    let mut s = deepest.map_or_else(
        || "polling only".to_owned(),
        |d| {
            if d.exit_latency_us == 0 {
                "polling only".to_owned()
            } else {
                format!("at most {} (exit {} µs)", d.name, d.exit_latency_us)
            }
        },
    );
    match limit {
        ResumeLimit::Us(us) => {
            let _ = write!(s, ", pm_qos_resume_latency_us {us}");
        }
        ResumeLimit::PollOnly => s.push_str(", pm_qos_resume_latency_us n/a"),
        ResumeLimit::None => {}
    }
    s
}

/// T20 on one CPU: `Ok(Some)` its finding (a warning, information, or
/// unknown); `Err(line)` its ok line (a CPU that sleeps no deeper than C1),
/// kept for the summary.
fn one_cpu(
    host: &Host,
    cpu: u32,
    cpus: &HotSet,
    takes: Option<&[String]>,
    driver: &str,
) -> Result<Option<Finding>, String> {
    let id = "T20";
    let who = takes.map_or_else(
        || cpus.describe(cpu),
        |names| {
            let mut list = names.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            if names.len() > 3 {
                list.push_str(", …");
            }
            format!("{} takes {list} and", cpus.describe(cpu))
        },
    );
    let verdict = match judge(host, cpu) {
        Ok(v) => v,
        Err(e) => {
            return Ok(Some(Finding::unknown(
                id,
                Severity::Warn,
                T20_EVIDENCE,
                format!("cannot tell how deep {} may sleep", cpus.describe(cpu)),
                e,
            )));
        }
    };
    let (deepest, limit) = match verdict {
        CpuVerdict::Deep(deep) => {
            let deep: Vec<&IdleState> = deep.iter().collect();
            let fix = fixes(cpu, &deep, driver);
            let states = states_text(&deep);
            return Ok(Some(if takes.is_some() {
                Finding::found(
                    id,
                    Severity::Warn,
                    T20_EVIDENCE,
                    format!(
                        "{who} may sleep in {states} ({driver}): an interrupt that finds it there \
                         waits for the wake before its handler and softirq run. On AWS c7i \
                         (intel_idle, Sapphire Rapids) interrupt-driven TCP round trips at 10k \
                         msg/s had p50 43–160 µs and p99 315–559 µs with every state enabled \
                         (the IRQ CPU in C6, exit 190 µs, for 4–28 s of each 60 s point), and \
                         p50 37–39 µs, p99 44–49 µs capped at C1 (3 reps). {fix}"
                    ),
                )
            } else {
                Finding::found(
                    id,
                    Severity::Info,
                    T20_EVIDENCE,
                    format!(
                        "{who} may sleep in {states} ({driver}): it matters only when the thread \
                         there sleeps (a backoff idle strategy, a blocking wait), which then \
                         wakes up to the exit latency late. {fix}"
                    ),
                )
            }));
        }
        CpuVerdict::Shallow(deepest, limit) => (deepest, limit),
    };
    let halts = deepest.as_ref().is_some_and(|d| d.exit_latency_us > 0);
    if takes.is_some() && halts && driver == "acpi_idle" {
        let name = deepest.as_ref().map_or("C1", |d| d.name.as_str());
        let index = deepest.as_ref().map_or(1, |d| d.index);
        return Ok(Some(Finding::found(
            id,
            Severity::Info,
            T20_EVIDENCE,
            format!(
                "{who} sleeps at most in {name} (acpi_idle): nothing deeper to cap. Under a \
                 hypervisor C1 is a halt the hypervisor wakes; on AWS c7a (acpi_idle) the lab saw \
                 no wake-up penalty (idle ping round trip min 101–114 µs, against 223–258 µs on \
                 c7i with intel_idle uncapped). Polling only removes the halt as well, at a \
                 whole core's power: `echo 1 > \
                 /sys/devices/system/cpu/cpu{cpu}/cpuidle/state{index}/disable`"
            ),
        )));
    }
    Err(format!(
        "{}: {}",
        cpus.describe(cpu),
        shallow_text(deepest.as_ref(), limit)
    ))
}

/// T20: the CPUs that take the data NICs' interrupts may not sleep deeper
/// than C1 ([`T20_MAX_EXIT_LATENCY_US`]), and a hot CPU that is not
/// busy-spinning is reported (information) when it may.
///
/// An interrupt that finds its CPU in a deep idle state waits for the wake
/// before its handler and softirq run: interrupt-driven receive (no busy
/// polling, or busy polling that misses) pays that on every packet that
/// arrives while the CPU sleeps. A CPU's usable states are the enabled ones
/// within its `pm_qos_resume_latency_us`. Per CPU: a warning for an IRQ CPU
/// with a deeper usable state; information for an IRQ CPU on `acpi_idle`
/// whose deepest is C1 (nothing to cap) and for a hot CPU of a role that
/// sleeps ([`Role::spins`](crate::Role::spins) false: the conductor, an EAL
/// lcore) with a deeper state; busy-spinning CPUs are skipped, since they
/// never idle while they spin. No cpuidle driver (Graviton, a hypervisor
/// that owns idle) is ok.
#[must_use]
pub fn t20_irq_idle(host: &Host, nics: &[Nic], cpus: &HotSet) -> Vec<Finding> {
    let (id, sev) = ("T20", Severity::Warn);
    let driver = match cpuidle_driver(host) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return vec![Finding::ok(
                id,
                sev,
                T20_EVIDENCE,
                "no cpuidle driver (cpuidle/current_driver none or absent): idle is the \
                 hypervisor's or the architecture's (Graviton: WFI), nothing the OS caps"
                    .to_owned(),
            )];
        }
        Err(e) => {
            return vec![Finding::unknown(
                id,
                sev,
                T20_EVIDENCE,
                "cannot tell the cpuidle driver".to_owned(),
                e,
            )];
        }
    };
    let mut out = Vec::new();
    let irq = irq_cpus(host, nics, &mut out);
    let spinning = cpus.spinning();
    let sleepers: CpuSet = cpus.all().difference(&spinning);
    let checked: CpuSet = irq.keys().copied().collect::<CpuSet>().union(&sleepers);
    let mut ok = Vec::new();
    for cpu in checked.iter() {
        match one_cpu(host, cpu, cpus, irq.get(&cpu).map(Vec::as_slice), &driver) {
            Ok(Some(f)) => out.push(f),
            Ok(None) => {}
            Err(line) => ok.push(line),
        }
    }
    if out.iter().all(|f| !f.is_finding() && !f.is_unknown()) {
        let message = if ok.is_empty() {
            format!(
                "no CPU to check: no interrupt of the data NICs found, and every hot CPU \
                 busy-spins ({driver})"
            )
        } else {
            format!("{} ({driver})", ok.join("; "))
        };
        out.push(Finding::ok(id, sev, T20_EVIDENCE, message));
    }
    out
}
