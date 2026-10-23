//! T1 (isolation and `nohz_full`), T7 (real-time policy), T8 (SMT siblings)
//! and T10 (SMT on, x86), and a thread's policy and affinity from
//! `/proc/thread-self`.

use crate::cpuset::CpuSet;
use crate::finding::{CheckId, Finding, HotSet, Role, Severity};
use crate::host::{Host, ReadError};
use crate::platform::{Arch, Platform};

/// A thread whose policy or affinity is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thread {
    /// The calling thread (`/proc/thread-self`).
    Current,
    /// A thread of this process by its kernel thread id
    /// (`/proc/self/task/<tid>`).
    Task(u32),
}

impl Thread {
    pub(crate) fn file(self, name: &str) -> String {
        match self {
            Self::Current => format!("/proc/thread-self/{name}"),
            Self::Task(tid) => format!("/proc/self/task/{tid}/{name}"),
        }
    }
}

/// A scheduling policy, as `/proc/<tid>/stat` gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedPolicy {
    /// `SCHED_OTHER` (0).
    Other,
    /// `SCHED_FIFO` (1).
    Fifo,
    /// `SCHED_RR` (2).
    RoundRobin,
    /// `SCHED_BATCH` (3).
    Batch,
    /// `SCHED_IDLE` (5).
    Idle,
    /// `SCHED_DEADLINE` (6).
    Deadline,
    /// A number this crate does not know.
    Unrecognised(u32),
}

impl SchedPolicy {
    const fn from_number(n: u32) -> Self {
        match n {
            0 => Self::Other,
            1 => Self::Fifo,
            2 => Self::RoundRobin,
            3 => Self::Batch,
            5 => Self::Idle,
            6 => Self::Deadline,
            other => Self::Unrecognised(other),
        }
    }

    /// The policy's name (`SCHED_FIFO`).
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Other => "SCHED_OTHER".to_owned(),
            Self::Fifo => "SCHED_FIFO".to_owned(),
            Self::RoundRobin => "SCHED_RR".to_owned(),
            Self::Batch => "SCHED_BATCH".to_owned(),
            Self::Idle => "SCHED_IDLE".to_owned(),
            Self::Deadline => "SCHED_DEADLINE".to_owned(),
            Self::Unrecognised(n) => format!("policy {n}"),
        }
    }
}

/// The policy of `thread`: the 41st field of its `stat`, counted after the
/// command name (which may hold spaces and parentheses).
///
/// # Errors
/// The stat file cannot be read or does not parse.
pub fn thread_policy(host: &Host, thread: Thread) -> Result<SchedPolicy, ReadError> {
    let path = thread.file("stat");
    let text = host.read(&path)?;
    let malformed = |detail| ReadError::Malformed {
        path: path.clone(),
        detail,
    };
    let (_, after_comm) = text
        .rsplit_once(')')
        .ok_or_else(|| malformed("no command name"))?;
    // Fields after the command start at field 3 (state); the policy is 41.
    let policy = after_comm
        .split_whitespace()
        .nth(41 - 3)
        .ok_or_else(|| malformed("fewer fields than a stat line has"))?;
    let n: u32 = policy
        .parse()
        .map_err(|_| malformed("policy not a number"))?;
    Ok(SchedPolicy::from_number(n))
}

/// The CPUs `thread` may run on (`Cpus_allowed_list` of its `status`).
///
/// # Errors
/// The status file cannot be read or does not parse.
pub fn thread_affinity(host: &Host, thread: Thread) -> Result<CpuSet, ReadError> {
    let path = thread.file("status");
    let text = host.read(&path)?;
    let list = text
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .ok_or_else(|| ReadError::Malformed {
            path: path.clone(),
            detail: "no Cpus_allowed_list line",
        })?;
    CpuSet::parse_list(list).map_err(|e| ReadError::cpus(&path, e))
}

/// The CPUs isolated from the scheduler's load balancing: `isolcpus`
/// (`/sys/devices/system/cpu/isolated`) and cgroup v2 isolated partitions
/// (`cpuset.cpus.isolated`, Linux 6.7 and later, in the root cgroup only:
/// absent in a container, which is not an error).
pub fn isolated_cpus(host: &Host) -> Result<CpuSet, ReadError> {
    let boot = host.read_cpu_list("/sys/devices/system/cpu/isolated")?;
    let partitions = host
        .read_cpu_list("/sys/fs/cgroup/cpuset.cpus.isolated")
        .unwrap_or_default();
    Ok(boot.union(&partitions))
}

/// The online CPUs outside `busy`, if any.
pub fn spare(host: &Host, busy: &CpuSet) -> Option<CpuSet> {
    host.read_cpu_list("/sys/devices/system/cpu/online")
        .map(|online| online.difference(busy))
        .ok()
        .filter(|s| !s.is_empty())
}

/// The CPUs to give housekeeping in a fix: online, minus `busy`; a
/// placeholder when that is nothing or unknown.
pub fn housekeeping(host: &Host, busy: &CpuSet) -> String {
    spare(host, busy).map_or_else(|| "<housekeeping CPUs>".to_owned(), |s| s.to_string())
}

const T1_EVIDENCE: &str = "git show \
     128747b9^:io/benches/results/flags-aws-20261014T163114Z/tables-local.md (noise table, \
     lines 27-30; interrupts table, lines 332-335) and \
     128747b9^:io/benches/results/flags-aws-20261015T045527Z/tables-local.md (lines 146-149, \
     403-406): AWS c7i, one 120 s gap-spinner sample per boot profile; Linux \
     Documentation/timers/no_hz.rst and admin-guide/kernel-parameters.txt (isolcpus, nohz_full, \
     rcu_nocbs, irqaffinity)";

/// What T1 reads and the boot line it recommends.
struct T1<'a> {
    cpus: &'a HotSet,
    spinning: CpuSet,
    /// The spinning CPUs asked to be `nohz_full`: all but those that hold
    /// only ergon-io's SQPOLL thread (`nohz_full` there is not measured
    /// here).
    tickless: CpuSet,
    /// The spinning CPUs, described.
    all: String,
    /// The CPUs the boot line isolates: the spinning ones and those already
    /// isolated or `nohz_full`.
    target: CpuSet,
    hk: String,
    boot: String,
}

impl T1<'_> {
    const fn finding(message: String) -> Finding {
        Finding::found(CheckId::T1, Severity::Warn, T1_EVIDENCE, message)
    }

    fn unknown(message: String, e: &ReadError) -> Finding {
        Finding::unknown(CheckId::T1, Severity::Warn, T1_EVIDENCE, message, e)
    }

    fn domain(&self, isolated: &Result<CpuSet, ReadError>, out: &mut Vec<Finding>) {
        let isolated = match isolated {
            Ok(i) => i,
            Err(e) => {
                out.push(Self::unknown(
                    format!("cannot tell whether {} are isolated", self.all),
                    e,
                ));
                return;
            }
        };
        for cpu in self.spinning.difference(isolated).iter() {
            out.push(Self::finding(format!(
                "{} is not isolated: the scheduler balances other tasks onto it. On AWS c7i, `isolcpus` \
                 took a gap-spinner's gaps over 20 µs from 120 and 162 a minute to 4 and 0.5 (longest \
                 gap 147 → 52 µs and 150 → 30 µs; two runs, one 120 s sample per boot). Fix: {}",
                self.cpus.describe(cpu),
                self.boot
            )));
        }
    }

    fn nohz(&self, nohz: &Result<Option<CpuSet>, ReadError>, out: &mut Vec<Finding>) {
        let nohz = match nohz {
            Ok(n) => n,
            Err(e) => {
                out.push(Self::unknown(
                    format!("cannot tell whether {} are nohz_full", self.all),
                    e,
                ));
                return;
            }
        };
        let (missing, kernel, fix) = nohz.as_ref().map_or_else(
            || {
                (
                    self.tickless.clone(),
                    "this kernel has no NO_HZ_FULL (no /sys/devices/system/cpu/nohz_full), so ",
                    format!(
                        "use a kernel built with CONFIG_NO_HZ_FULL=y and {}",
                        self.boot
                    ),
                )
            },
            |n| (self.tickless.difference(n), "", self.boot.clone()),
        );
        for cpu in missing.iter() {
            out.push(Self::finding(format!(
                "{} is not nohz_full: {kernel}its scheduler tick keeps firing, CONFIG_HZ times a \
                 second. On AWS c7i a gap-spinner's CPU took 1,002 local-timer interrupts a second \
                 without nohz_full and 0 with it, and had 60,120 and 60,130 gaps over 5 µs a minute \
                 on the stock boot, 60,063 and 60,046 with isolcpus alone, 50 and 21 with nohz_full \
                 too (longest gap 147/150, 52/30 and 134/30 µs; two runs, one 120 s sample per \
                 boot). Fix: {fix}",
                self.cpus.describe(cpu)
            )));
        }
    }

    /// The `Ok` line: what was found to be in place.
    fn ok(&self, nohz: &Result<Option<CpuSet>, ReadError>) -> Finding {
        let ticking = match nohz {
            Ok(Some(n)) => self.spinning.difference(n),
            Ok(None) => self.spinning.clone(),
            Err(_) => CpuSet::new(),
        };
        let message = if ticking.is_empty() {
            format!(
                "{}: isolated, nohz_full and outside default_smp_affinity",
                self.all
            )
        } else {
            format!(
                "{}: isolated and outside default_smp_affinity; nohz_full but for {}, which \
                 holds only the SQPOLL thread (not asked there; not measured)",
                self.all,
                self.cpus.describe_all(&ticking)
            )
        };
        Finding::ok(CheckId::T1, Severity::Warn, T1_EVIDENCE, message)
    }

    fn default_affinity(&self, host: &Host, out: &mut Vec<Finding>) {
        let path = "/proc/irq/default_smp_affinity";
        let mask = match host
            .read(path)
            .and_then(|t| CpuSet::parse_mask(&t).map_err(|e| ReadError::cpus(path, e)))
        {
            Ok(m) => m,
            Err(e) => {
                out.push(Self::unknown(
                    format!(
                        "cannot tell whether {} are outside the default IRQ affinity",
                        self.all
                    ),
                    &e,
                ));
                return;
            }
        };
        let in_mask = self.spinning.intersection(&mask);
        if in_mask.is_empty() {
            return;
        }
        let hk_mask = spare(host, &self.target)
            .map_or_else(|| "<housekeeping mask>".to_owned(), |s| s.to_mask());
        out.push(Self::finding(format!(
            "{} {} in /proc/irq/default_smp_affinity ({}): interrupts set up from now on may be \
             placed there. Fix: `echo {hk_mask} > /proc/irq/default_smp_affinity`, or boot with \
             `irqaffinity={}`",
            self.cpus.describe_all(&in_mask),
            if in_mask.len() == 1 { "is" } else { "are" },
            mask.to_mask(),
            self.hk
        )));
    }
}

/// T1: each busy-spinning CPU of `cpus` ([`Role::spins`]) is isolated from
/// the scheduler's load balancing, `nohz_full`, and outside
/// `/proc/irq/default_smp_affinity`.
///
/// Isolation and `nohz_full` are reported separately, per CPU: `isolcpus`
/// alone left the tick running. A CPU that holds only ergon-io's SQPOLL
/// thread ([`Role::SqPoll`]) is not asked to be `nohz_full` (not measured
/// here).
#[must_use]
pub fn t1_isolation(host: &Host, cpus: &HotSet) -> Vec<Finding> {
    let spinning = cpus.spinning();
    if spinning.is_empty() {
        return vec![Finding::ok(
            CheckId::T1,
            Severity::Warn,
            T1_EVIDENCE,
            "no busy-spinning CPU to check".to_owned(),
        )];
    }
    let isolated = isolated_cpus(host);
    let nohz_path = "/sys/devices/system/cpu/nohz_full";
    let nohz = host.read_optional(nohz_path).and_then(|t| {
        t.map(|t| CpuSet::parse_list(&t).map_err(|e| ReadError::cpus(nohz_path, e)))
            .transpose()
    });
    let none = CpuSet::new();
    let target = spinning
        .union(isolated.as_ref().unwrap_or(&none))
        .union(nohz.as_ref().ok().and_then(Option::as_ref).unwrap_or(&none));
    let hk = housekeeping(host, &target);
    let boot = format!(
        "boot with `isolcpus=nohz,domain,managed_irq,{target} nohz_full={target} \
         rcu_nocbs={target} irqaffinity={hk}`"
    );
    let tickless = cpus
        .iter()
        .filter(|h| h.role.spins() && h.role != Role::SqPoll)
        .map(|h| h.cpu)
        .collect();
    let t1 = T1 {
        cpus,
        all: cpus.describe_all(&spinning),
        spinning,
        tickless,
        target,
        hk,
        boot,
    };
    let mut out = Vec::new();
    t1.domain(&isolated, &mut out);
    t1.nohz(&nohz, &mut out);
    t1.default_affinity(host, &mut out);
    if out.is_empty() {
        out.push(t1.ok(&nohz));
    }
    out
}

const T7_EVIDENCE: &str = "git show \
     128747b9^:io/benches/results/flags-aws-20261015T070339Z/tables.md (c1-rt, c2-rt: HTTP \
     lines 22-29; WebSocket lines 63-70; interrupts lines 87-124): AWS c7i, two 8-minute \
     reps; Linux Documentation/scheduler/sched-rt-group.rst";

/// T7: `thread`, in `role`, is not under `SCHED_FIFO` or `SCHED_RR`. The
/// media driver checks its process before the agents start.
#[must_use]
pub fn t7_sched_policy(host: &Host, thread: Thread, role: Role) -> Finding {
    let id = CheckId::T7;
    let sev = Severity::Warn;
    let who = format!("the {} thread", role.name());
    match thread_policy(host, thread) {
        Ok(p @ (SchedPolicy::Fifo | SchedPolicy::RoundRobin)) => {
            let throttle = match host.read_i64("/proc/sys/kernel/sched_rt_runtime_us") {
                Ok(-1) => " (RT throttling is off here, sched_rt_runtime_us -1: nothing else gets \
                           that CPU)"
                    .to_owned(),
                Ok(us) => {
                    let period = host
                        .read_i64("/proc/sys/kernel/sched_rt_period_us")
                        .map_or_else(|_| "its period".to_owned(), |p| format!("{p} µs"));
                    format!(" (sched_rt_runtime_us {us} of {period})")
                }
                Err(_) => String::new(),
            };
            Finding::found(
                id,
                sev,
                T7_EVIDENCE,
                format!(
                    "{who} runs {}: its CPU's tick stays on (AWS c7i: 533–947 local-timer \
                     interrupts a second against 11–23), and RT throttling can take the CPU \
                     away{throttle}. The latency A/B was inconclusive: two 8-minute reps moved \
                     p99.99 by +1 to −45 µs and the max by −444 to +536 µs, while the two HTTP \
                     baselines alone differed by 42 µs at p99.99. Fix: run it SCHED_OTHER on an isolated CPU \
                     (`chrt --other -p 0 <tid>`)",
                    p.name()
                ),
            )
        }
        Ok(p) => Finding::ok(id, sev, T7_EVIDENCE, format!("{who} runs {}", p.name())),
        Err(e) => Finding::unknown(
            id,
            sev,
            T7_EVIDENCE,
            format!("cannot tell {who}'s scheduling policy"),
            e,
        ),
    }
}

const T8_EVIDENCE: &str = "git show \
     128747b9^:io/benches/results/flags-aws-20261015T122127Z/tables.md (lab-uring-sqpoll-c* \
     against lab-uring-sqpoll-sib-c*, lines 174-279): ergon-io's SQPOLL thread on the loop's \
     sibling, AWS c7i, three rounds; other sibling loads and two threads on one CPU not \
     measured here";

/// What T8 has seen so far.
struct T8<'a> {
    cpus: &'a HotSet,
    isolated: Result<CpuSet, ReadError>,
    pairs: Vec<(u32, u32)>,
    smt: bool,
    out: Vec<Finding>,
}

impl T8<'_> {
    fn push(&mut self, severity: Severity, message: String) {
        self.out
            .push(Finding::found(CheckId::T8, severity, T8_EVIDENCE, message));
    }

    /// Two busy-spinning roles pinned to one CPU.
    fn same_cpu(&mut self, cpu: u32) {
        let roles: Vec<&str> = self
            .cpus
            .roles(cpu)
            .into_iter()
            .filter(|r| r.spins())
            .map(Role::name)
            .collect();
        if roles.len() > 1 {
            self.push(
                Severity::Warn,
                format!(
                    "the {} are both on CPU {cpu}: two busy threads take turns on one CPU, so each \
                     waits for the other's time slice (not measured here). Fix: pin them to separate \
                     cores",
                    roles.join(" and ")
                ),
            );
        }
    }

    /// `cpu`, busy-spinning, and its SMT sibling `s`.
    fn sibling(&mut self, cpu: u32, s: u32) {
        self.smt = true;
        let roles = self.cpus.roles(s);
        let busy = roles
            .iter()
            .any(|r| r.spins() || matches!(r, Role::Conductor | Role::EalLcore));
        let (me, them) = (self.cpus.describe(cpu), self.cpus.describe(s));
        if busy {
            let pair = (cpu.min(s), cpu.max(s));
            if self.pairs.contains(&pair) {
                return;
            }
            self.pairs.push(pair);
            if sqpoll_with_loop(self.cpus, cpu, s) {
                return;
            }
            self.push(
                Severity::Warn,
                format!(
                    "{} and {} are hyperthreads of one core: two busy threads share its execution \
                     units; the lab gives each its own core. Fix: pin them to separate cores",
                    self.cpus.describe(pair.0),
                    self.cpus.describe(pair.1)
                ),
            );
        } else if roles.contains(&Role::Housekeeping) {
            self.push(
                Severity::Info,
                format!(
                    "{me} shares its core with {them}: what runs there takes the core's execution \
                     units from it (measured only for ergon-io's SQPOLL thread there: T10). Fix: \
                     keep CPU {s} idle, or give it a core of its own"
                ),
            );
        } else {
            match &self.isolated {
                Ok(iso) if iso.contains(s) => {}
                Ok(_) => self.push(
                    Severity::Info,
                    format!(
                        "{me} shares its core with CPU {s}, which is not isolated: other tasks run \
                         there and take the core's execution units from it (measured only for \
                         ergon-io's SQPOLL thread there: T10). Fix: isolate CPU {s} and leave it \
                         idle"
                    ),
                ),
                Err(e) => {
                    let f = Finding::unknown(
                        CheckId::T8,
                        Severity::Warn,
                        T8_EVIDENCE,
                        format!("cannot tell whether CPU {s}, {me}'s SMT sibling, is isolated"),
                        e,
                    );
                    self.out.push(f);
                }
            }
        }
    }
}

/// T8: no busy-spinning CPU of `cpus` shares its physical core with another
/// of the component's threads.
///
/// A warning when the SMT sibling (or the same CPU) holds another
/// busy-spinner, the conductor or an EAL lcore, except ergon-io's SQPOLL
/// thread on its loop's sibling (the recommended one-core layout); an
/// information line when the sibling is the component's housekeeping or is
/// not isolated. Without SMT (Graviton, c7a) it fires only when two
/// busy-spinning roles share one CPU.
#[must_use]
pub fn t8_smt_siblings(host: &Host, cpus: &HotSet) -> Vec<Finding> {
    let spinning = cpus.spinning();
    let mut t8 = T8 {
        cpus,
        isolated: isolated_cpus(host),
        pairs: Vec::new(),
        smt: false,
        out: Vec::new(),
    };
    for cpu in spinning.iter() {
        t8.same_cpu(cpu);
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list");
        match host.read_cpu_list(&path) {
            Ok(siblings) => {
                for s in siblings.iter().filter(|s| *s != cpu) {
                    t8.sibling(cpu, s);
                }
            }
            Err(e) => t8.out.push(Finding::unknown(
                CheckId::T8,
                Severity::Warn,
                T8_EVIDENCE,
                format!("cannot tell {}'s SMT siblings", cpus.describe(cpu)),
                e,
            )),
        }
    }
    if t8.out.is_empty() {
        let all = cpus.describe_all(&spinning);
        let message = if spinning.is_empty() {
            "no busy-spinning CPU to check".to_owned()
        } else if t8.smt {
            format!("{all}: no other busy thread on their cores, and their siblings are isolated")
        } else if Platform::detect(host).is_graviton() {
            format!("{all}: one thread per core (Graviton has no SMT)")
        } else {
            format!("{all}: one thread per core (no SMT sibling)")
        };
        t8.out.push(Finding::ok(
            CheckId::T8,
            Severity::Warn,
            T8_EVIDENCE,
            message,
        ));
    }
    t8.out
}

/// ergon-io's SQPOLL thread on its loop's SMT sibling: the one-core layout
/// ergon-io documents. Its cost against a core of its own is T10's figure.
fn sqpoll_with_loop(cpus: &HotSet, a: u32, b: u32) -> bool {
    let (ra, rb) = (cpus.roles(a), cpus.roles(b));
    (ra.contains(&Role::Loop) && rb.contains(&Role::SqPoll) && !rb.contains(&Role::Loop))
        || (rb.contains(&Role::Loop) && ra.contains(&Role::SqPoll) && !ra.contains(&Role::Loop))
}

const T10_EVIDENCE: &str = "git show \
     128747b9^:io/benches/results/flags-aws-20261015T122127Z/tables.md (lines 174-279, \
     arrival → decoded, three rounds); \
     AWS instance-optimize-cpu and instance-specify-cpu-options (CpuOptions ThreadsPerCore); \
     kernel-parameters.txt (nosmt); ABI/testing/sysfs-devices-system-cpu (smt/active, smt/control); \
     Azure compute-optimized Fasv6/Falsv6/Famsv6 and general-purpose Dpsv6/Dplsv6 (no SMT)";

/// Whether SMT (hyperthreading) is active on the host: `smt/active`, or any
/// `thread_siblings_list` with more than one CPU when that file is missing.
fn smt_active(host: &Host) -> Result<bool, ReadError> {
    if let Ok(line) = host.read_line("/sys/devices/system/cpu/smt/active") {
        return match line.trim() {
            "1" => Ok(true),
            "0" => Ok(false),
            _ => Err(ReadError::Malformed {
                path: "/sys/devices/system/cpu/smt/active".to_owned(),
                detail: "smt/active is neither 0 nor 1",
            }),
        };
    }
    let online = host.read_cpu_list("/sys/devices/system/cpu/online")?;
    let mut saw_one = false;
    for cpu in online.iter() {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list");
        match host.read_cpu_list(&path) {
            Ok(siblings) => {
                saw_one = true;
                if siblings.len() > 1 {
                    return Ok(true);
                }
            }
            Err(ReadError::Io { .. }) => {}
            Err(e) => return Err(e),
        }
    }
    if saw_one {
        Ok(false)
    } else {
        Err(ReadError::Malformed {
            path: "/sys/devices/system/cpu".to_owned(),
            detail: "no smt/active and no thread_siblings_list to fall back on",
        })
    }
}

/// T10: SMT is off on this x86 host.
///
/// On x86 a Kubernetes `cpu: 2` is one physical core, not two, and a busy
/// sibling on that core slows the loop. Graviton and other arm64 hosts pass
/// (they have no SMT); so does an x86 host with SMT off.
#[must_use]
pub fn t10_smt_on(host: &Host) -> Finding {
    let id = CheckId::T10;
    let sev = Severity::Warn;
    let platform = Platform::detect(host);
    if platform.is_graviton() || platform.arch == Arch::Aarch64 {
        return Finding::ok(
            id,
            sev,
            T10_EVIDENCE,
            if platform.is_graviton() {
                "Graviton: one vCPU is one physical core (no SMT)".to_owned()
            } else {
                "arm64: one vCPU is one physical core (no SMT)".to_owned()
            },
        );
    }
    match smt_active(host) {
        Ok(false) => Finding::ok(
            id,
            sev,
            T10_EVIDENCE,
            "SMT is off: one vCPU is one physical core".to_owned(),
        ),
        Ok(true) => Finding::found(
            id,
            sev,
            T10_EVIDENCE,
            "SMT is on: on x86 a Kubernetes `cpu: 2` is one physical core, not two, and a busy \
             sibling slows the loop. Measured for ergon-io's SQPOLL thread on the loop's sibling \
             against a core of its own (AWS c7i, arrival → decoded, p50 / p99 / max over three \
             rounds): 1 connection 4.0–4.3 / 7.0–11.7 / 35–46 µs against 3.5–3.8 / 8.4–12.9 / \
             32–118; 10 connections 7.5–8.4 / 29–33 / 96–349 against 5.2–6.0 / 20–24 / 77–109; \
             100 connections 7.5–7.8 / 32–34 / 95–112 against 5.0–5.3 / 21–22 / 76–301. Other \
             sibling loads are not measured. Turn SMT off on the NODE. EKS/EC2: launch template \
             `\"CpuOptions\": {\"CoreCount\": <the type's cores>, \"ThreadsPerCore\": 1}` (not \
             possible on bare metal, or on T2, C7a, M7a, R7a, Apple silicon Mac \
             or Graviton). Self-managed: \
             boot with `nosmt` (`nosmt=force` cannot be undone at runtime), or `echo off > \
             /sys/devices/system/cpu/smt/control` before the kubelet starts, then delete its \
             cpu_manager_state so it re-pins. Azure: a size without SMT (Fasv6, Falsv6, Famsv6, \
             Dpsv6, Dplsv6: one vCPU is one physical core)"
                .to_owned(),
        ),
        Err(e) => Finding::unknown(
            id,
            sev,
            T10_EVIDENCE,
            "cannot tell whether SMT is active on this host".to_owned(),
            e,
        ),
    }
}
