//! T13-T18: the node's background work that can take a busy-spinning CPU or
//! move a hot thread's pages: unbound workqueues (T13), memory compaction
//! (T14), other processes' tasks (T15), kernel instrumentation (T16), KSM
//! (T17) and hypervisor steal (T18).
//!
//! Each reads `/sys` and `/proc` only, so it runs unprivileged and from a
//! pod; every fix is on the node.

use std::fmt::Write as _;

use crate::cpu::spare;
use crate::cpuset::CpuSet;
use crate::finding::{Finding, HotSet, Severity};
use crate::host::{Host, ReadError, first_line};

/// `n thing` or `n things`.
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Up to four names, then `…` for the rest.
fn some_names(names: &[String]) -> String {
    let mut s = names.iter().take(4).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > 4 {
        s.push_str(", …");
    }
    s
}

// ---------------------------------------------------------------- T13

const WQ_MASK: &str = "/sys/devices/virtual/workqueue/cpumask";

const T13_EVIDENCE: &str = "Linux Documentation/core-api/workqueue.rst (unbound workqueues, \
     /sys/devices/virtual/workqueue/cpumask) and admin-guide/kernel-per-CPU-kthreads.rst; not \
     measured here";

/// T13: the unbound workqueue cpumask
/// (`/sys/devices/virtual/workqueue/cpumask`) leaves out every
/// busy-spinning CPU of `cpus` ([`Role::spins`](crate::Role::spins)).
#[must_use]
pub fn t13_workqueue_cpumask(host: &Host, cpus: &HotSet) -> Finding {
    let (id, sev) = ("T13", Severity::Warn);
    let spinning = cpus.spinning();
    if spinning.is_empty() {
        return Finding::ok(
            id,
            sev,
            T13_EVIDENCE,
            "no busy-spinning CPU to check".to_owned(),
        );
    }
    let all = cpus.describe_all(&spinning);
    let mask = match host
        .read(WQ_MASK)
        .and_then(|t| CpuSet::parse_mask(&t).map_err(|e| ReadError::cpus(WQ_MASK, e)))
    {
        Ok(m) => m,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                T13_EVIDENCE,
                format!("cannot tell whether unbound workqueues may run on {all}"),
                e,
            );
        }
    };
    let on = spinning.intersection(&mask);
    if on.is_empty() {
        return Finding::ok(
            id,
            sev,
            T13_EVIDENCE,
            format!(
                "unbound workqueues are kept off {all} (workqueue cpumask {})",
                mask.to_mask()
            ),
        );
    }
    let hk =
        spare(host, &spinning).map_or_else(|| "<housekeeping mask>".to_owned(), |s| s.to_mask());
    Finding::found(
        id,
        sev,
        T13_EVIDENCE,
        format!(
            "{} {} in the unbound workqueue cpumask ({WQ_MASK} is {}): the kernel's unbound work \
             items (writeback, reclaim and driver work, kworker/u threads) may run there, and the \
             busy-spinning thread stands still while they do (not measured here). `nohz_full` \
             keeps those CPUs out of it at boot, so this fires on a boot without it, or after \
             something wrote the file. Fix (node): `echo {hk} > {WQ_MASK}`",
            cpus.describe_all(&on),
            if on.len() == 1 { "is" } else { "are" },
            mask.to_mask(),
        ),
    )
}

// ---------------------------------------------------------------- T14

const PROACTIVENESS: &str = "/proc/sys/vm/compaction_proactiveness";
const UNEVICTABLE: &str = "/proc/sys/vm/compact_unevictable_allowed";

const T14_EVIDENCE: &str = "Linux Documentation/admin-guide/sysctl/vm.rst \
     (compaction_proactiveness: default 20, 0 turns proactive compaction off; \
     compact_unevictable_allowed: default 1, 0 on PREEMPT_RT \"to avoid a page fault, due to \
     compaction\"); not measured here";

/// A number sysctl, or `None` when this kernel does not have it.
fn sysctl_number(host: &Host, path: &str) -> Result<Option<i64>, ReadError> {
    host.read_optional(path)?
        .map(|t| {
            first_line(&t).parse().map_err(|_| ReadError::Malformed {
                path: path.to_owned(),
                detail: "not a number",
            })
        })
        .transpose()
}

/// T14: memory compaction leaves a hot thread's pages in place.
///
/// Proactive compaction off (`vm.compaction_proactiveness` 0, Linux 5.9 and
/// later) and locked pages not compacted (`vm.compact_unevictable_allowed`
/// 0). Information: compaction on demand still runs.
#[must_use]
pub fn t14_compaction(host: &Host) -> Finding {
    let (id, sev) = ("T14", Severity::Info);
    let read = sysctl_number(host, PROACTIVENESS)
        .and_then(|p| sysctl_number(host, UNEVICTABLE).map(|u| (p, u)));
    let (proactive, unevictable) = match read {
        Ok(v) => v,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                T14_EVIDENCE,
                "cannot tell whether memory compaction may move a hot thread's pages".to_owned(),
                e,
            );
        }
    };
    let mut what = Vec::new();
    let mut fix = Vec::new();
    if let Some(p) = proactive.filter(|p| *p != 0) {
        what.push(format!(
            "vm.compaction_proactiveness {p} (kcompactd compacts memory in the background)"
        ));
        fix.push("vm.compaction_proactiveness=0");
    }
    if let Some(u) = unevictable.filter(|u| *u != 0) {
        what.push(format!(
            "vm.compact_unevictable_allowed {u} (compaction moves locked pages too, so \
             `mlockall` does not keep them in place)"
        ));
        fix.push("vm.compact_unevictable_allowed=0");
    }
    if what.is_empty() {
        let message = if proactive.is_none() && unevictable.is_none() {
            "the kernel has no compaction sysctls".to_owned()
        } else {
            "proactive compaction is off and locked pages are not compacted".to_owned()
        };
        return Finding::ok(id, sev, T14_EVIDENCE, message);
    }
    Finding::found(
        id,
        sev,
        T14_EVIDENCE,
        format!(
            "memory compaction may move a hot thread's pages: {}. Moving a page unmaps it: the \
             thread's next touch of it faults, and the unmap flushes the TLB of every CPU running \
             the process, busy-spinning ones included (not measured here). Fix (node; vm.* \
             sysctls are not set from a pod): `sysctl -w {}`",
            what.join("; "),
            fix.join(" ")
        ),
    )
}

// ---------------------------------------------------------------- T15

/// `PF_KTHREAD`, a kernel thread, in a task's `stat` flags (field 9).
const PF_KTHREAD: u64 = 0x0020_0000;

const T15_EVIDENCE: &str = "Linux proc(5) (/proc/<pid>/task/<tid>/status Cpus_allowed_list) and \
     kernel/sched/core.c sched_init_smp (\"Move init over to a non-isolated CPU\": init's affinity \
     is housekeeping_cpumask(HK_TYPE_DOMAIN), set by isolcpus=domain); not measured here";

/// A task's state (field 3) and flags (field 9) from its `stat`.
fn state_and_flags(text: &str) -> Option<(char, u64)> {
    let (_, after_comm) = text.rsplit_once(')')?;
    let mut fields = after_comm.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let flags = fields.nth(9 - 4)?.parse().ok()?;
    Some((state, flags))
}

/// A task's name and the CPUs it may run on, from its `status`.
fn name_and_cpus(text: &str) -> Option<(String, CpuSet)> {
    let mut name = None;
    let mut cpus = None;
    for line in text.lines() {
        if let Some(n) = line.strip_prefix("Name:") {
            name = Some(n.trim().to_owned());
        } else if let Some(l) = line.strip_prefix("Cpus_allowed_list:") {
            cpus = CpuSet::parse_list(l).ok();
        }
    }
    Some((name?, cpus?))
}

/// This process's id: `/proc/self`'s target, or, on the running host,
/// the process's own.
fn own_pid(host: &Host) -> Option<u32> {
    match host.link_name("/proc/self") {
        Ok(Some(n)) => n.parse().ok(),
        _ => None,
    }
    .or_else(|| host.is_live().then(std::process::id))
}

/// What T15 saw in `/proc`.
#[derive(Default)]
struct Tasks {
    /// Each other process with a task allowed on a spinning CPU: its pid,
    /// its name and those CPUs.
    offenders: Vec<(u32, String, CpuSet)>,
    /// Whether a kernel thread was visible (the host's PID namespace).
    kernel: bool,
    /// User tasks read.
    checked: usize,
    /// Tasks that could not be read.
    unreadable: usize,
}

/// Is a read error a task that ended between listing and reading?
const fn gone(e: &ReadError) -> bool {
    matches!(
        e,
        ReadError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }
    )
}

fn scan_tasks(host: &Host, spinning: &CpuSet) -> Result<Tasks, ReadError> {
    let own = own_pid(host);
    let mut seen = Tasks::default();
    let pids = host.list("/proc")?;
    for pid in pids.iter().filter_map(|p| p.parse::<u32>().ok()) {
        if Some(pid) == own {
            continue;
        }
        let dir = format!("/proc/{pid}/task");
        let tids = match host.list(&dir) {
            Ok(t) => t,
            Err(e) if gone(&e) => continue,
            Err(_) => {
                seen.unreadable += 1;
                continue;
            }
        };
        let (mut main, mut first, mut hit) = (None, None, CpuSet::new());
        for tid in tids.iter().filter_map(|t| t.parse::<u32>().ok()) {
            let read = host
                .read(&format!("{dir}/{tid}/stat"))
                .and_then(|s| Ok((s, host.read(&format!("{dir}/{tid}/status"))?)));
            let (stat, status) = match read {
                Ok(r) => r,
                Err(e) if gone(&e) => continue,
                Err(_) => {
                    seen.unreadable += 1;
                    continue;
                }
            };
            let Some((state, flags)) = state_and_flags(&stat) else {
                seen.unreadable += 1;
                continue;
            };
            if flags & PF_KTHREAD != 0 {
                seen.kernel = true;
                continue;
            }
            // A zombie or dead task never runs again.
            if matches!(state, 'Z' | 'X') {
                continue;
            }
            let Some((name, allowed)) = name_and_cpus(&status) else {
                seen.unreadable += 1;
                continue;
            };
            seen.checked += 1;
            let on = allowed.intersection(spinning);
            if !on.is_empty() {
                hit = hit.union(&on);
                first.get_or_insert_with(|| name.clone());
            }
            if tid == pid {
                main = Some(name);
            }
        }
        if !hit.is_empty() {
            let name = main.or(first).unwrap_or_default();
            seen.offenders.push((pid, name, hit));
        }
    }
    Ok(seen)
}

/// T15: no task of another process may run on a busy-spinning CPU of
/// `cpus` (its `Cpus_allowed_list`, from `/proc/<pid>/task/<tid>/status`).
///
/// This process's own threads are not asked, nor are zombies or kernel
/// threads: a per-CPU kernel thread belongs to its CPU, and the others are
/// T1's (`isolcpus`, `nohz_full`) and T13's. In a PID namespace of its own
/// (a pod), the node's other tasks cannot be seen: unknown unless one that
/// can be seen fires.
#[must_use]
pub fn t15_host_tasks(host: &Host, cpus: &HotSet) -> Finding {
    let (id, sev) = ("T15", Severity::Warn);
    let spinning = cpus.spinning();
    if spinning.is_empty() {
        return Finding::ok(
            id,
            sev,
            T15_EVIDENCE,
            "no busy-spinning CPU to check".to_owned(),
        );
    }
    let all = cpus.describe_all(&spinning);
    let cannot = format!("cannot tell whether other processes' tasks may run on {all}");
    let seen = match scan_tasks(host, &spinning) {
        Ok(s) => s,
        Err(e) => return Finding::unknown(id, sev, T15_EVIDENCE, cannot, e),
    };
    if seen.offenders.is_empty() {
        if !seen.kernel {
            return Finding::unknown(
                id,
                sev,
                T15_EVIDENCE,
                cannot,
                "only this PID namespace's tasks are visible (no kernel thread in /proc): the \
                 node's other tasks cannot be seen",
            );
        }
        if seen.unreadable > 0 {
            return Finding::unknown(
                id,
                sev,
                T15_EVIDENCE,
                cannot,
                format!(
                    "{} could not be read (hidepid, or permission)",
                    count(seen.unreadable, "task", "tasks")
                ),
            );
        }
        return Finding::ok(
            id,
            sev,
            T15_EVIDENCE,
            format!(
                "no task of another process may run on {all} ({} checked; kernel threads are \
                 T1's and T13's)",
                count(seen.checked, "user task", "user tasks")
            ),
        );
    }
    let mut list = String::new();
    for (i, (pid, name, on)) in seen.offenders.iter().take(6).enumerate() {
        let sep = if i == 0 { "" } else { "; " };
        let _ = write!(list, "{sep}{name} (pid {pid}) on {}", cpus.describe_all(on));
    }
    if seen.offenders.len() > 6 {
        let _ = write!(list, "; and {} more", seen.offenders.len() - 6);
    }
    let unread = if seen.unreadable > 0 {
        format!(
            " ({} more could not be read)",
            count(seen.unreadable, "task", "tasks")
        )
    } else {
        String::new()
    };
    let hk =
        spare(host, &spinning).map_or_else(|| "<housekeeping CPUs>".to_owned(), |s| s.to_string());
    Finding::found(
        id,
        sev,
        T15_EVIDENCE,
        format!(
            "{} outside this one may run on the busy-spinning CPUs: {list}{unread}. The \
             scheduler may wake their threads there, and the busy-spinning thread stands still \
             while they run (not measured here). Fix (node): boot with `isolcpus=domain,<hot>` \
             (T1), which starts init, and so every task it forks, on the housekeeping CPUs; give \
             systemd's services `CPUAffinity={hk}` (/etc/systemd/system.conf); move a running \
             one with `taskset -apc {hk} <pid>`. In Kubernetes the kubelet's static CPU manager \
             with `reservedSystemCPUs={hk}` keeps other pods off, not the node's own services",
            count(seen.offenders.len(), "process", "processes"),
        ),
    )
}

// ---------------------------------------------------------------- T16

const KPROBES: &str = "/sys/kernel/debug/kprobes/list";
const KPROBES_ENABLED: &str = "/sys/kernel/debug/kprobes/enabled";
const FTRACE: [&str; 2] = [
    "/sys/kernel/tracing/enabled_functions",
    "/sys/kernel/debug/tracing/enabled_functions",
];
const KALLSYMS: &str = "/proc/kallsyms";
const JIT_ENABLE: &str = "/proc/sys/net/core/bpf_jit_enable";
const JIT_KALLSYMS: &str = "/proc/sys/net/core/bpf_jit_kallsyms";
const JIT_HARDEN: &str = "/proc/sys/net/core/bpf_jit_harden";

const T16_EVIDENCE: &str = "Linux Documentation/trace/kprobes.rst (debugfs kprobes/list, \
     kprobes/enabled), trace/ftrace.rst (enabled_functions) and admin-guide/sysctl/net.rst \
     (bpf_jit_kallsyms: JIT-ed programs in /proc/kallsyms, for privileged readers, and not with \
     bpf_jit_harden on); not measured here";

/// The armed kprobes, by function: debugfs's list, less the disabled and
/// gone ones; none when kprobes are disarmed (`enabled` 0).
fn kprobes(host: &Host) -> Result<Vec<String>, ReadError> {
    let text = host.read(KPROBES)?;
    if host.read_line(KPROBES_ENABLED).is_ok_and(|l| l == "0") {
        return Ok(Vec::new());
    }
    Ok(text
        .lines()
        .filter(|l| !l.contains("[DISABLED]") && !l.contains("[GONE]"))
        .filter_map(|l| l.split_whitespace().nth(2))
        .map(|f| f.split('+').next().unwrap_or(f).to_owned())
        .collect())
}

/// The functions ftrace has hooked (function tracer, fentry/fexit BPF,
/// live patches), from tracefs at either mount point.
fn ftrace_hooks(host: &Host) -> Result<Vec<String>, ReadError> {
    let text = host
        .read(FTRACE[0])
        .or_else(|e| host.read(FTRACE[1]).map_err(|_| e))?;
    Ok(text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_owned)
        .collect())
}

/// A sysctl that must be at least 1 for an empty BPF list to prove anything.
fn at_least_one(host: &Host, path: &str) -> Result<(), ReadError> {
    match host.read_i64(path)? {
        n if n >= 1 => Ok(()),
        _ => Err(ReadError::Malformed {
            path: path.to_owned(),
            detail: "0: JIT-ed BPF programs are not listed in /proc/kallsyms",
        }),
    }
}

/// The BPF programs `/proc/kallsyms` lists (`bpf_prog_<tag>_<name>
/// [bpf]`), by name. None listed proves none loaded only for a privileged
/// reader (real addresses) with the JIT and its symbols on, and JIT
/// hardening off (hardening stops the export).
fn bpf_programs(host: &Host) -> Result<Vec<String>, ReadError> {
    let text = host.read(KALLSYMS)?;
    let mut privileged = false;
    let mut names = Vec::new();
    for line in text.lines() {
        let mut f = line.split_whitespace();
        let (Some(addr), Some(_), Some(sym)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        privileged |= addr.bytes().any(|b| b != b'0');
        if line.trim_end().ends_with("[bpf]")
            && let Some(rest) = sym.strip_prefix("bpf_prog_")
        {
            // `<16 hex digits of tag>_<name>`, or the tag alone.
            let name = rest.split_once('_').map_or(rest, |(_, n)| n);
            names.push(name.to_owned());
        }
    }
    if names.is_empty() {
        if !privileged {
            return Err(ReadError::Malformed {
                path: KALLSYMS.to_owned(),
                detail: "addresses hidden: an unprivileged reader is not shown BPF programs",
            });
        }
        at_least_one(host, JIT_ENABLE)?;
        at_least_one(host, JIT_KALLSYMS)?;
        if host.read_i64(JIT_HARDEN)? != 0 {
            return Err(ReadError::Malformed {
                path: JIT_HARDEN.to_owned(),
                detail: "not 0: JIT hardening stops BPF programs being listed in /proc/kallsyms",
            });
        }
    }
    Ok(names)
}

/// T16: no kernel instrumentation is loaded.
///
/// It reads kprobes (debugfs), ftrace hooks (tracefs) and BPF programs
/// (`/proc/kallsyms`). Information: a finding names what it saw and what it
/// could not read. Unprivileged, the sources are usually unreadable:
/// unknown, never a warning.
#[must_use]
pub fn t16_instrumentation(host: &Host) -> Finding {
    let (id, sev) = ("T16", Severity::Info);
    let sources = [
        (kprobes(host), "kprobe", "kprobes"),
        (
            ftrace_hooks(host),
            "ftrace-hooked function",
            "ftrace-hooked functions",
        ),
        (bpf_programs(host), "BPF program", "BPF programs"),
    ];
    let mut seen = Vec::new();
    let mut unread = Vec::new();
    for (r, one, many) in &sources {
        match r {
            Ok(names) if names.is_empty() => {}
            Ok(names) => seen.push(format!(
                "{} ({})",
                count(names.len(), one, many),
                some_names(names)
            )),
            Err(e) => unread.push(e.to_string()),
        }
    }
    if seen.is_empty() {
        if unread.is_empty() {
            return Finding::ok(
                id,
                sev,
                T16_EVIDENCE,
                "no kprobe, ftrace hook or BPF program is loaded".to_owned(),
            );
        }
        return Finding::unknown(
            id,
            sev,
            T16_EVIDENCE,
            "cannot tell whether kprobes, ftrace hooks or BPF programs are loaded".to_owned(),
            unread.join("; "),
        );
    }
    let unread = if unread.is_empty() {
        String::new()
    } else {
        format!("; not readable here: {}", unread.join("; "))
    };
    Finding::found(
        id,
        sev,
        T16_EVIDENCE,
        format!(
            "kernel instrumentation is loaded: {}{unread}. Each runs on whichever CPU calls what \
             it is attached to, busy-spinning ones included: a kprobe traps into its handler, an \
             ftrace hook or a BPF program runs inline (systemd's cgroup filters and monitoring \
             agents load them; not measured here). Find the owners (root): `bpftool prog show`, \
             `cat {KPROBES}`; remove what the node does not need",
            seen.join(", ")
        ),
    )
}

// ---------------------------------------------------------------- T17

const KSM: &str = "/sys/kernel/mm/ksm";

const T17_EVIDENCE: &str = "Linux Documentation/admin-guide/mm/ksm.rst (run: 0 stop, 1 run, 2 \
     unmerge; pages_sharing); not measured here";

/// T17: kernel same-page merging is not running (`/sys/kernel/mm/ksm/run`
/// 0 or 2).
#[must_use]
pub fn t17_ksm(host: &Host) -> Finding {
    let (id, sev) = ("T17", Severity::Warn);
    if !host.exists(KSM) && host.exists("/sys/kernel/mm") {
        return Finding::ok(id, sev, T17_EVIDENCE, "the kernel has no KSM".to_owned());
    }
    let run = format!("{KSM}/run");
    let cannot = "cannot tell whether kernel same-page merging (KSM) runs".to_owned();
    match host.read_i64(&run) {
        Ok(0) => Finding::ok(id, sev, T17_EVIDENCE, "KSM is stopped (run 0)".to_owned()),
        Ok(2) => Finding::ok(
            id,
            sev,
            T17_EVIDENCE,
            "KSM is stopped and its pages unmerged (run 2)".to_owned(),
        ),
        Ok(1) => {
            let sharing = host.read_i64(&format!("{KSM}/pages_sharing")).map_or_else(
                |_| String::new(),
                |n| format!("; {n} pages are merged (pages_sharing)"),
            );
            Finding::found(
                id,
                sev,
                T17_EVIDENCE,
                format!(
                    "kernel same-page merging runs ({run} is 1{sharing}): ksmd scans the memory \
                     that asked for merging (madvise MADV_MERGEABLE, or a whole process with \
                     prctl PR_SET_MEMORY_MERGE), and a write to a merged page takes a \
                     copy-on-write fault on the writing thread (not measured here). Fix (node): \
                     `echo 2 > {run}` (unmerge and stop), or `echo 0 > {run}` (stop)"
                ),
            )
        }
        Ok(_) => Finding::unknown(
            id,
            sev,
            T17_EVIDENCE,
            cannot,
            ReadError::Malformed {
                path: run,
                detail: "not 0, 1 or 2",
            },
        ),
        Err(e) => Finding::unknown(id, sev, T17_EVIDENCE, cannot, e),
    }
}

// ---------------------------------------------------------------- T18

const PROC_STAT: &str = "/proc/stat";

/// Linux's `USER_HZ`: `/proc/stat` counts in hundredths of a second.
const MS_PER_TICK: u64 = 10;

const T18_EVIDENCE: &str = "Linux proc(5) (/proc/stat: steal, the eighth value of a cpuN line, \
     in USER_HZ ticks); not measured here";

/// One CPU's time since boot, from its `/proc/stat` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuSteal {
    /// The CPU number.
    pub cpu: u32,
    /// Time the hypervisor ran something else while this vCPU was
    /// runnable, in ticks of 10 ms (`USER_HZ`).
    pub steal_ticks: u64,
    /// All its time, in the same ticks: user, nice, system, idle, iowait,
    /// irq, softirq and steal (guest time is already in user and nice).
    pub total_ticks: u64,
}

/// Each CPU's steal and total time since boot, from `/proc/stat`. Read it
/// twice and subtract to see the steal between the two reads.
///
/// # Errors
/// `/proc/stat` cannot be read, or a CPU line has no steal column or a
/// value that is not a number.
pub fn cpu_steal(host: &Host) -> Result<Vec<CpuSteal>, ReadError> {
    let text = host.read(PROC_STAT)?;
    let malformed = |detail| ReadError::Malformed {
        path: PROC_STAT.to_owned(),
        detail,
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line
            .strip_prefix("cpu")
            .filter(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        else {
            continue;
        };
        let mut f = rest.split_whitespace();
        let cpu = f
            .next()
            .and_then(|c| c.parse().ok())
            .ok_or_else(|| malformed("a cpu line without a CPU number"))?;
        let values = f
            .take(8)
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| malformed("a cpu line value that is not a number"))?;
        if values.len() < 8 {
            return Err(malformed("a cpu line without the steal column"));
        }
        out.push(CpuSteal {
            cpu,
            steal_ticks: values[7],
            total_ticks: values.iter().fold(0, |a: u64, v| a.saturating_add(*v)),
        });
    }
    Ok(out)
}

/// T18: no time stolen from a busy-spinning CPU of `cpus` since boot.
///
/// It reads `/proc/stat`'s steal. Information: steal is the hypervisor's,
/// and a count since boot says it happened, not that it happens now
/// ([`cpu_steal`] twice says that).
#[must_use]
pub fn t18_steal(host: &Host, cpus: &HotSet) -> Finding {
    let (id, sev) = ("T18", Severity::Info);
    let spinning = cpus.spinning();
    if spinning.is_empty() {
        return Finding::ok(
            id,
            sev,
            T18_EVIDENCE,
            "no busy-spinning CPU to check".to_owned(),
        );
    }
    let all = cpus.describe_all(&spinning);
    let times = match cpu_steal(host) {
        Ok(t) => t,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                T18_EVIDENCE,
                format!("cannot tell whether time was stolen from {all}"),
                e,
            );
        }
    };
    let mut stolen = Vec::new();
    let mut missing = CpuSet::new();
    for cpu in spinning.iter() {
        match times.iter().find(|t| t.cpu == cpu) {
            None => missing.insert(cpu),
            Some(t) if t.steal_ticks > 0 => {
                let total = t.total_ticks.max(1);
                // Tenths of a percent, rounded.
                let permille = (t.steal_ticks.saturating_mul(1000) + total / 2) / total;
                stolen.push(format!(
                    "{}: {} ms stolen since boot ({}.{}% of its time)",
                    cpus.describe(cpu),
                    t.steal_ticks.saturating_mul(MS_PER_TICK),
                    permille / 10,
                    permille % 10
                ));
            }
            Some(_) => {}
        }
    }
    let not_listed = || {
        format!(
            "{} not in {PROC_STAT} (offline)",
            cpus.describe_all(&missing)
        )
    };
    if stolen.is_empty() {
        if !missing.is_empty() {
            return Finding::unknown(
                id,
                sev,
                T18_EVIDENCE,
                format!("cannot tell whether time was stolen from {all}"),
                not_listed(),
            );
        }
        return Finding::ok(
            id,
            sev,
            T18_EVIDENCE,
            format!(
                "no time stolen from {all} since boot (also 0 when the hypervisor does not \
                 report steal)"
            ),
        );
    }
    let missing = if missing.is_empty() {
        String::new()
    } else {
        format!("; {}", not_listed())
    };
    Finding::found(
        id,
        sev,
        T18_EVIDENCE,
        format!(
            "{}{missing}: the hypervisor ran another guest or its own work while the vCPU had \
             work to run, and a busy-spinning thread there stood still (not measured here). The \
             count is since boot: read `cpu_steal` twice to see it now. Fix (outside the guest): \
             an instance whose vCPUs are not shared (not a burstable type), or bare metal",
            stolen.join("; ")
        ),
    )
}
