//! The Kubernetes checks (K1–K7): what a pod's settings do to a
//! busy-spinning thread. They run only inside a pod.
//!
//! Each finding says where the fix goes: the pod spec, or the node (the
//! kubelet). Everything is read at start-up; nothing reads a file while the
//! loop runs.

use crate::cpu::{Thread, t7_sched_policy};
use crate::cpuset::CpuSet;
use crate::finding::{CheckId, Finding, HotSet, Role, Severity};
use crate::host::{Host, ReadError};
use crate::net::Nic;
use crate::quota::{CpuCgroup, cpu_cgroup, t9_cpu_quota};

const K_EVIDENCE: &str = "Kubernetes documentation (Pod QoS, CPU Management Policies, Topology \
     Management, Linux capabilities); Linux Documentation/admin-guide/cgroup-v2.rst (memory.high, \
     cpuset partitions) and accounting/psi.rst; K3's figures are T10's; otherwise not measured \
     here";

/// What the caller does in the pod, so the checks warn only about the
/// privileges and the layout it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Needs {
    /// The thread that will busy-spin (T7 checks its policy).
    pub thread: Thread,
    /// What it does, for the messages.
    pub role: Role,
    /// Sockets busy-poll or prefer busy polling: `CAP_NET_ADMIN`.
    pub busy_polls: bool,
    /// A thread runs `SCHED_FIFO` or `SCHED_RR`: `CAP_SYS_NICE`.
    pub sched_fifo: bool,
    /// The process locks its memory (`mlockall`): `CAP_IPC_LOCK`, or
    /// `RLIMIT_MEMLOCK`.
    pub mlock: bool,
    /// The data NIC, for the NUMA check (K7).
    pub nic: Option<Nic>,
}

impl Needs {
    /// One busy-spinning `thread` in `role`, with no other needs.
    #[must_use]
    pub const fn new(thread: Thread, role: Role) -> Self {
        Self {
            thread,
            role,
            busy_polls: false,
            sched_fifo: false,
            mlock: false,
            nic: None,
        }
    }

    /// This, with the NIC for K7.
    #[must_use]
    pub fn with_nic(mut self, nic: Nic) -> Self {
        self.nic = Some(nic);
        self
    }

    /// This, with busy polling needing `CAP_NET_ADMIN`.
    #[must_use]
    pub const fn busy_polls(mut self) -> Self {
        self.busy_polls = true;
        self
    }

    /// This, with real-time threads needing `CAP_SYS_NICE`.
    #[must_use]
    pub const fn sched_fifo(mut self) -> Self {
        self.sched_fifo = true;
        self
    }

    /// This, with `mlockall` needing `CAP_IPC_LOCK` or `RLIMIT_MEMLOCK`.
    #[must_use]
    pub const fn mlock(mut self) -> Self {
        self.mlock = true;
        self
    }
}

/// Whether this runs inside a Kubernetes pod: the environment of the running
/// host, the service account mount, or a `kubepods` cgroup path.
#[must_use]
pub fn in_pod(host: &Host) -> bool {
    if host.is_live() && std::env::var_os("KUBERNETES_SERVICE_HOST").is_some() {
        return true;
    }
    if host.exists("/var/run/secrets/kubernetes.io/serviceaccount") {
        return true;
    }
    cgroup_path(host).is_some_and(|p| p.contains("kubepods"))
}

/// The checks that apply here: K1–K7, with T9 and T7 reused. Empty when this
/// does not run inside a pod. Call this once, before the loop.
#[must_use]
pub fn k8s_checks(host: &Host, cpus: &HotSet, needs: &Needs) -> Vec<Finding> {
    if !in_pod(host) {
        return Vec::new();
    }
    let cg = cpu_cgroup(host);
    vec![
        k1_qos(host),
        k2_exclusive(host, cg.as_ref().ok()),
        t9_cpu_quota(host),
        t7_sched_policy(host, needs.thread, needs.role),
        k3_smt_sibling(host, cpus, cg.as_ref().ok()),
        k4_helpers(host, cpus, cg.as_ref().ok()),
        k5_capabilities(host, needs),
        k6_pressure(host, cg.as_ref().ok()),
        k7_numa(host, cg.as_ref().ok(), needs.nic.as_ref()),
    ]
}

/// The cgroup path of `/proc/self/cgroup`'s unified or cpu line: the pod's
/// path on the node (`/kubepods/burstable/pod…`), or the container's own
/// (`/`) when a cgroup namespace hides the pod.
fn cgroup_path(host: &Host) -> Option<String> {
    let text = host.read("/proc/self/cgroup").ok()?;
    let mut unified = None;
    for line in text.lines() {
        let mut parts = line.splitn(3, ':');
        let (Some(_), Some(controllers), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if controllers.split(',').any(|c| c == "cpu") {
            return Some(path.to_owned());
        }
        if controllers.is_empty() {
            unified = Some(path.to_owned());
        }
    }
    unified
}

/// The `QoS` class the cgroup path shows.
fn qos_of(path: &str) -> Option<&'static str> {
    if path.contains("besteffort") {
        Some("BestEffort")
    } else if path.contains("burstable") {
        Some("Burstable")
    } else if path.contains("kubepods") {
        Some("Guaranteed")
    } else {
        None
    }
}

/// K1: the pod is Guaranteed `QoS`.
fn k1_qos(host: &Host) -> Finding {
    let id = CheckId::K1;
    let sev = Severity::Warn;
    let Some(path) = cgroup_path(host) else {
        return Finding::unknown(
            id,
            sev,
            K_EVIDENCE,
            "cannot tell this pod's QoS class".to_owned(),
            "no cgroup path in /proc/self/cgroup",
        );
    };
    match qos_of(&path) {
        Some("Guaranteed") => Finding::ok(
            id,
            sev,
            K_EVIDENCE,
            format!("Guaranteed QoS (cgroup {path})"),
        ),
        Some(qos) => Finding::found(
            id,
            sev,
            K_EVIDENCE,
            format!(
                "this pod is {qos} (cgroup {path}): not Guaranteed, so it keeps a CPU limit and \
                 gets no exclusive CPUs. Fix (pod spec): set this container's \
                 `resources.requests` equal to its `resources.limits` for cpu and memory, and ask \
                 for a whole number of CPUs (cpu: 2): that is Guaranteed QoS"
            ),
        ),
        None => Finding::unknown(
            id,
            sev,
            K_EVIDENCE,
            "cannot tell this pod's QoS class".to_owned(),
            format!("cgroup path {path} has no kubepods QoS segment"),
        ),
    }
}

/// The pod's cpuset, or the container's own: `cpuset.cpus.effective` on
/// cgroup v2, `cpuset.cpus` on v1.
fn pod_cpuset(host: &Host, cg: Option<&CpuCgroup>) -> Result<CpuSet, ReadError> {
    let mut dirs: Vec<String> = cg.map(|c| vec![c.dir.clone()]).unwrap_or_default();
    dirs.push("/sys/fs/cgroup".to_owned());
    let mut last = ReadError::Malformed {
        path: "/sys/fs/cgroup".to_owned(),
        detail: "no cpuset.cpus.effective",
    };
    for dir in dirs {
        for name in ["cpuset.cpus.effective", "cpuset.cpus"] {
            let path = format!("{dir}/{name}");
            match host.read_cpu_list(&path) {
                Ok(set) => return Ok(set),
                Err(ReadError::Io { .. }) => {}
                Err(e) => last = e,
            }
        }
    }
    Err(last)
}

/// The cgroup directories `pod_cpuset` reads from: the container's own first,
/// then the cgroup root.
fn pod_cgroup_dirs(cg: Option<&CpuCgroup>) -> Vec<String> {
    let mut dirs: Vec<String> = cg.map(|c| vec![c.dir.clone()]).unwrap_or_default();
    dirs.push("/sys/fs/cgroup".to_owned());
    dirs
}

/// Why this pod's CPUs are its own, if the kernel says so. `cpuset.cpus` alone
/// cannot: a restricted shared pool looks the same from inside the pod.
/// `cpuset.cpus.partition` is `isolated`/`root` for a partition this cgroup
/// owns; `cpuset.cpus.exclusive` naming exactly this cpuset is the other.
/// `Ok(None)` means the kernel exposed no answer here.
fn exclusivity_evidence(host: &Host, cg: Option<&CpuCgroup>, set: &CpuSet) -> Option<String> {
    for dir in pod_cgroup_dirs(cg) {
        if let Ok(partition) = host.read(&format!("{dir}/cpuset.cpus.partition")) {
            let p = partition.trim();
            if p == "isolated" || p == "root" {
                return Some(format!("cpuset.cpus.partition={p}"));
            }
            if p == "member" {
                return None;
            }
        }
        if let Ok(exclusive) = host.read_cpu_list(&format!("{dir}/cpuset.cpus.exclusive"))
            && !exclusive.is_empty()
            && &exclusive == set
        {
            return Some("cpuset.cpus.exclusive names exactly this cpuset".to_owned());
        }
    }
    None
}

/// K2: the pod's CPUs are its own, not the shared pool.
fn k2_exclusive(host: &Host, cg: Option<&CpuCgroup>) -> Finding {
    let id = CheckId::K2;
    let sev = Severity::Warn;
    let set = match pod_cpuset(host, cg) {
        Ok(s) => s,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot tell this pod's cpuset".to_owned(),
                e,
            );
        }
    };
    let online = match host.read_cpu_list("/sys/devices/system/cpu/online") {
        Ok(o) => o,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot tell the node's online CPUs".to_owned(),
                e,
            );
        }
    };
    if set.is_empty() {
        return Finding::unknown(
            id,
            sev,
            K_EVIDENCE,
            "cannot tell this pod's cpuset".to_owned(),
            "the cpuset is empty",
        );
    }
    if set == online {
        return Finding::found(
            id,
            sev,
            K_EVIDENCE,
            format!(
                "this pod's cpuset is every online CPU of the node ({set}): it runs in the shared \
                 pool, so anything else on those CPUs takes the loop's time. Fix (node): kubelet \
                 `cpuManagerPolicy: static` with `cpuManagerPolicyOptions: full-pcpus-only`. Fix \
                 (pod spec): Guaranteed QoS and a whole-number CPU request, so the CPU manager \
                 gives this container exclusive CPUs"
            ),
        );
    }
    if let Some(why) = exclusivity_evidence(host, cg, &set) {
        return Finding::ok(
            id,
            sev,
            K_EVIDENCE,
            format!(
                "this pod's cpuset ({set}) is a proper subset of the node's online CPUs \
                 ({online}): exclusive CPUs ({why})"
            ),
        );
    }
    Finding::unknown(
        id,
        sev,
        K_EVIDENCE,
        format!(
            "this pod's cpuset ({set}) is a proper subset of the node's online CPUs ({online}), \
             which is what exclusive CPUs look like — and also what a restricted shared pool \
             looks like from inside the pod. Nothing here says which"
        ),
        "no cpuset.cpus.partition (isolated/root) and no cpuset.cpus.exclusive matching this \
         cpuset; exclusivity is node-side state (the kubelet CPU manager) that this pod cannot \
         read",
    )
}

/// K3: a busy-spinning CPU's SMT sibling is outside this pod's cpuset.
fn k3_smt_sibling(host: &Host, cpus: &HotSet, cg: Option<&CpuCgroup>) -> Finding {
    let id = CheckId::K3;
    let sev = Severity::Warn;
    let set = match pod_cpuset(host, cg) {
        Ok(s) => s,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot tell this pod's cpuset".to_owned(),
                e,
            );
        }
    };
    let online = host.read_cpu_list("/sys/devices/system/cpu/online");
    let mut lost: Vec<(u32, u32)> = Vec::new();
    let mut smt = false;
    // CPUs whose topology is missing. A missing thread_siblings_list is not
    // "SMT is off": it is evidence this check does not have.
    let mut unreadable: Vec<String> = Vec::new();
    for cpu in cpus.spinning().iter() {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list");
        match host.read_cpu_list(&path) {
            Ok(siblings) => {
                for s in siblings.iter().filter(|s| *s != cpu) {
                    smt = true;
                    let on = online.as_ref().map_or(true, |o| o.contains(s));
                    if on && !set.contains(s) {
                        lost.push((cpu, s));
                    }
                }
            }
            Err(ReadError::Io { .. }) => {
                unreadable.push(cpus.describe(cpu));
            }
            Err(e) => {
                return Finding::unknown(
                    id,
                    sev,
                    K_EVIDENCE,
                    format!("cannot tell {}'s SMT siblings", cpus.describe(cpu)),
                    e,
                );
            }
        }
    }
    if lost.is_empty() {
        if !unreadable.is_empty() {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                format!(
                    "cannot tell whether {} has an SMT sibling in this pod",
                    unreadable.join(", ")
                ),
                "no thread_siblings_list for the busy-spinning CPUs; SMT off and unreadable \
                 topology look the same without it",
            );
        }
        let message = if smt {
            "every hot CPU's SMT siblings are in this pod's cpuset".to_owned()
        } else {
            "no hot CPU has an SMT sibling".to_owned()
        };
        return Finding::ok(id, sev, K_EVIDENCE, message);
    }
    let pairs: Vec<String> = lost
        .iter()
        .map(|(c, s)| format!("{}: CPU {s}", cpus.describe(*c)))
        .collect();
    Finding::found(
        id,
        sev,
        K_EVIDENCE,
        format!(
            "outside this pod's cpuset are the SMT siblings of the busy CPUs ({}): whatever runs \
             there (another pod, the kubelet) takes the core's execution units from the loop \
             (measured only for ergon-io's SQPOLL thread on the sibling: T10's figures). Fix (node): turn SMT off (T10). Fix (pod spec): ask for both threads of the \
             core (cpu: 2 with `full-pcpus-only`), so this pod owns its siblings",
            pairs.join(", ")
        ),
    )
}

/// Whether either helper-CPU variable is named (the running host only).
fn named_helpers(host: &Host) -> bool {
    host.is_live()
        && ["ERGON_SHARED_CPUS", "ERGON_HELPER_CPUS"]
            .iter()
            .any(|name| std::env::var_os(name).is_some_and(|v| !v.is_empty()))
}

/// K4: the pod has a CPU for helpers.
///
/// A CPU counts only when no busy-spinning role is pinned to it: two CPUs
/// that both spin leave the helpers nowhere to go, whatever the cpuset's
/// size. A spare that is a busy CPU's SMT sibling still counts — the book's
/// `x86 with SMT, cpu: 2` layout puts the helpers there — and the message
/// says so, because those helpers take that core's execution units.
fn k4_helpers(host: &Host, cpus: &HotSet, cg: Option<&CpuCgroup>) -> Finding {
    let id = CheckId::K4;
    let sev = Severity::Warn;
    let set = match pod_cpuset(host, cg) {
        Ok(s) => s,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot tell this pod's cpuset".to_owned(),
                e,
            );
        }
    };
    let spinning = cpus.spinning();
    let spare: CpuSet = set.iter().filter(|c| !spinning.contains(*c)).collect();
    if spare.is_empty() {
        if named_helpers(host) {
            return Finding::ok(
                id,
                sev,
                K_EVIDENCE,
                "every CPU in this pod's cpuset holds a busy-spinning role, and helper CPUs are \
                 named (ERGON_SHARED_CPUS or ERGON_HELPER_CPUS)"
                    .to_owned(),
            );
        }
        return Finding::found(
            id,
            sev,
            K_EVIDENCE,
            if set.len() == 1 {
                format!(
                    "this pod has one CPU ({set}) and names no helper CPUs: the helpers share the \
                     loop's CPU and take its time. Fix (pod spec): ask for one more CPU (cpu: 2), \
                     or run the shared-CPU plugin and set ERGON_SHARED_CPUS or ERGON_HELPER_CPUS"
                )
            } else {
                format!(
                    "every CPU in this pod's cpuset ({}) holds a busy-spinning role: the helpers \
                     share one of those CPUs and take its time. Fix (pod spec): ask for one more \
                     CPU (cpu: {}), or run the shared-CPU plugin and set ERGON_SHARED_CPUS or \
                     ERGON_HELPER_CPUS",
                    cpus.describe_all(&set),
                    set.len() + 1
                )
            },
        );
    }
    // Of the spares, which are a busy CPU's other hyperthread: a real CPU for
    // the helpers, but on the loop's core.
    let mut sibling: Vec<u32> = Vec::new();
    let mut own_core: Vec<u32> = Vec::new();
    let mut unproven: Vec<u32> = Vec::new();
    for c in spare.iter() {
        let path = format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list");
        match host.read_cpu_list(&path) {
            Ok(s) if s.iter().any(|x| x != c && spinning.contains(x)) => sibling.push(c),
            Ok(_) => own_core.push(c),
            // A missing thread_siblings_list is not "a core of its own".
            Err(_) => unproven.push(c),
        }
    }
    // The helpers get a core of their own when any spare is; otherwise the
    // spares are the busy CPUs' other hyperthreads (or the topology is
    // missing), and the message has to say which.
    let shared = own_core.is_empty() && !sibling.is_empty();
    let free = if !own_core.is_empty() {
        own_core
    } else if !sibling.is_empty() {
        sibling
    } else {
        unproven
    };
    let note = if shared {
        ", and it is a busy CPU's SMT sibling: the helpers share that core's execution units"
    } else {
        ""
    };
    let names = free
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let (cpu, is) = if free.len() == 1 {
        ("CPU", "is")
    } else {
        ("CPUs", "are")
    };
    Finding::ok(
        id,
        sev,
        K_EVIDENCE,
        format!(
            "{} CPUs in this pod's cpuset: {cpu} {names} {is} free of busy-spinning roles: one is \
             for helpers{note}",
            set.len()
        ),
    )
}

/// The capabilities bitmask of `/proc/self/status` (`CapEff`).
fn cap_eff(host: &Host) -> Result<u64, ReadError> {
    let text = host.read("/proc/self/status")?;
    for line in text.lines() {
        if let Some(hex) = line.strip_prefix("CapEff:") {
            return u64::from_str_radix(hex.trim(), 16).map_err(|_| ReadError::Malformed {
                path: "/proc/self/status".to_owned(),
                detail: "CapEff is not hex",
            });
        }
    }
    Err(ReadError::Malformed {
        path: "/proc/self/status".to_owned(),
        detail: "no CapEff line",
    })
}

/// Whether `RLIMIT_MEMLOCK` is unlimited: `/proc/self/limits`.
fn memlock_unlimited(host: &Host) -> bool {
    host.read("/proc/self/limits").is_ok_and(|text| {
        text.lines().any(|line| {
            line.starts_with("Max locked memory")
                && line.split_whitespace().any(|w| w == "unlimited")
        })
    })
}

/// K5: the capabilities the caller's needs require.
fn k5_capabilities(host: &Host, needs: &Needs) -> Finding {
    let id = CheckId::K5;
    let sev = Severity::Warn;
    let caps = match cap_eff(host) {
        Ok(c) => c,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot tell this pod's capabilities".to_owned(),
                e,
            );
        }
    };
    let has = |bit: u32| caps & (1u64 << bit) != 0;
    let mut missing: Vec<&str> = Vec::new();
    let mut add: Vec<&str> = Vec::new();
    if needs.busy_polls && !has(12) {
        missing.push("NET_ADMIN (busy polling)");
        add.push("NET_ADMIN");
    }
    if needs.mlock && !has(14) && !memlock_unlimited(host) {
        missing.push("IPC_LOCK (mlockall: the capability, or an unlimited RLIMIT_MEMLOCK)");
        add.push("IPC_LOCK");
    }
    if needs.sched_fifo && !has(23) {
        missing.push("SYS_NICE (SCHED_FIFO)");
        add.push("SYS_NICE");
    }
    if missing.is_empty() {
        return Finding::ok(
            id,
            sev,
            K_EVIDENCE,
            format!("CapEff has what this caller needs ({caps:#x})"),
        );
    }
    Finding::found(
        id,
        sev,
        K_EVIDENCE,
        format!(
            "this pod lacks {} (CapEff {caps:#x}): the call fails at start, or runs without it. \
             Fix (pod spec): `securityContext.capabilities.add: [\"{}\"]`",
            missing.join(", "),
            add.join("\", \"")
        ),
    )
}

/// `some avg10` of a PSI file (`cpu.pressure`, `memory.pressure`).
fn psi_avg10(host: &Host, path: &str) -> Option<f64> {
    let text = host.read(path).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("some ") {
            for field in rest.split_whitespace() {
                if let Some(v) = field.strip_prefix("avg10=") {
                    return v.parse::<f64>().ok();
                }
            }
        }
    }
    None
}

/// K6: no `memory.high`, no CPU or memory pressure.
fn k6_pressure(host: &Host, cg: Option<&CpuCgroup>) -> Finding {
    let id = CheckId::K6;
    let sev = Severity::Warn;
    let mut dirs: Vec<String> = cg.map(|c| vec![c.dir.clone()]).unwrap_or_default();
    dirs.push("/sys/fs/cgroup".to_owned());
    let mut high: Option<String> = None;
    let mut pressure: Vec<String> = Vec::new();
    let mut saw_file = false;
    for dir in &dirs {
        let path = format!("{dir}/memory.high");
        if let Ok(line) = host.read_line(&path) {
            saw_file = true;
            let v = line.trim();
            if v != "max" && !v.is_empty() {
                high = Some(format!("{path} is `{v}`"));
            }
        }
        for name in ["cpu.pressure", "memory.pressure"] {
            let path = format!("{dir}/{name}");
            if host.exists(&path) {
                saw_file = true;
            }
            if let Some(avg) = psi_avg10(host, &path)
                && avg > 0.0
            {
                pressure.push(format!("{path} some avg10 {avg}"));
            }
        }
    }
    let mut problems: Vec<String> = Vec::new();
    if let Some(h) = high {
        problems.push(format!(
            "{h}: the kernel reclaims this cgroup's memory once its working set reaches it, and \
             throttles the cgroup's threads while it does (cgroup-v2.rst; not measured here)"
        ));
    }
    for p in pressure {
        problems.push(format!(
            "{p}: the CPU or the memory was under pressure recently (any non-zero reading \
             warns; no threshold is measured)"
        ));
    }
    if problems.is_empty() {
        if !saw_file {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "no cgroup v2 memory.high or pressure file here to read".to_owned(),
                "no memory.high and no PSI file is a kernel that will not say, not a quiet cgroup",
            );
        }
        return Finding::ok(
            id,
            sev,
            K_EVIDENCE,
            "no memory.high and no CPU or memory pressure".to_owned(),
        );
    }
    Finding::found(
        id,
        sev,
        K_EVIDENCE,
        format!(
            "{}. Fix (pod spec): give this container a memory limit far above its working set (or \
             none), and its own exclusive CPUs. Fix (node): taint the latency nodes so nothing \
             else runs there",
            problems.join("; ")
        ),
    )
}

/// The NIC's NUMA node, or `None` when it has no PCI device or no node
/// (`numa_node -1`).
fn nic_numa_node(host: &Host, nic: &Nic) -> Result<Option<u32>, ReadError> {
    let devices = crate::net::pci_devices(host, nic)?;
    let Some(bdf) = devices.first() else {
        return Ok(None);
    };
    let n = host.read_i64(&format!("/sys/bus/pci/devices/{bdf}/numa_node"))?;
    if n < 0 {
        return Ok(None);
    }
    let node = u32::try_from(n).map_err(|_| ReadError::Malformed {
        path: format!("/sys/bus/pci/devices/{bdf}/numa_node"),
        detail: "the node number does not fit in u32",
    })?;
    Ok(Some(node))
}

/// The NUMA nodes the cpuset spans.
fn cpuset_nodes(host: &Host, set: &CpuSet) -> Result<Vec<u32>, ReadError> {
    let entries = host.list("/sys/devices/system/node")?;
    let mut on: Vec<u32> = Vec::new();
    let mut found = false;
    for entry in entries {
        let Some(name) = entry.strip_prefix("node") else {
            continue;
        };
        let Ok(n) = name.parse::<u32>() else {
            continue;
        };
        let path = format!("/sys/devices/system/node/{entry}/cpulist");
        if let Ok(cpus) = host.read_cpu_list(&path) {
            found = true;
            if !set.intersection(&cpus).is_empty() {
                on.push(n);
            }
        }
    }
    if found {
        Ok(on)
    } else {
        Err(ReadError::Malformed {
            path: "/sys/devices/system/node".to_owned(),
            detail: "no nodeN/cpulist",
        })
    }
}

/// K7: the pod's CPUs are on the NIC's NUMA node.
fn k7_numa(host: &Host, cg: Option<&CpuCgroup>, nic: Option<&Nic>) -> Finding {
    let id = CheckId::K7;
    let sev = Severity::Warn;
    let Some(nic) = nic else {
        return Finding::ok(
            id,
            sev,
            K_EVIDENCE,
            "no NIC named: the NUMA check is off".to_owned(),
        );
    };
    let node = match nic_numa_node(host, nic) {
        Ok(None) => {
            return Finding::ok(
                id,
                sev,
                K_EVIDENCE,
                format!("the NIC {nic} has no NUMA node: nothing to align"),
            );
        }
        Ok(Some(n)) => n,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                format!("cannot tell the NIC {nic}'s NUMA node"),
                e,
            );
        }
    };
    let set = match pod_cpuset(host, cg) {
        Ok(s) => s,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot tell this pod's cpuset".to_owned(),
                e,
            );
        }
    };
    let on = match cpuset_nodes(host, &set) {
        Ok(n) => n,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                K_EVIDENCE,
                "cannot read the node's NUMA topology".to_owned(),
                e,
            );
        }
    };
    let nodes = on.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    if on.contains(&node) {
        return Finding::ok(
            id,
            sev,
            K_EVIDENCE,
            format!("this pod's CPUs are on NUMA node {nodes}, with the NIC {nic} (node {node})"),
        );
    }
    Finding::found(
        id,
        sev,
        K_EVIDENCE,
        format!(
            "this pod's CPUs are on NUMA node {nodes}, the NIC {nic} is on node {node}: every \
             packet crosses between nodes and takes the interconnect. Fix (node): kubelet \
             `topologyManagerPolicy: single-numa-node`, so a pod gets CPUs and devices of one node"
        ),
    )
}
