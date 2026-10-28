//! The transport latency suite's host check, run before a round.
//!
//! ergon-tune's host checks for the hot CPUs (each a busy-spinning loop),
//! the host's own (THP, NUMA balancing, CPU quota, SMT, clock source, time
//! sync, background noise), the data NICs' (`--nic`) and the Aeron
//! directory's (`--aeron-dir`), and the counter clock across each pair of
//! neighbouring hot CPUs (`C1`, `ergon_latency_core::clock::cross_cpu_check`,
//! live hosts only).
//!
//! ```text
//! hostcheck --hot-cpus CSV --out FILE [--nic IFACE,...] [--aeron-dir PATH]
//!           [--smt off|on] [--root DIR] [--clock-rounds 100000]
//! ```
//!
//! `--smt on`: the session declares SMT on (a transport sets file's `smt
//! on`, the SMT-on arms: a loop on one hyperthread, its sibling idle or
//! running that loop's `uring-sqpoll` `SQPOLL` thread). Of each pair of hot
//! CPUs that are SMT siblings, the core's first thread is checked as a loop
//! and the other as its `SQPOLL` thread (T8 then refuses any other busy
//! pairing, and T1 still wants both isolated), and T10 (SMT on) is reported,
//! not blocking. The report's first line, `S1 info:`, records the
//! exemption. Default `off`: every hot CPU a loop, T10 blocking.
//!
//! `--out` gets one line per finding, ergon-tune's own: `<id> ok: ...`,
//! `<id> info: ...`, `<id> warn: ... (evidence: ...)` or
//! `<id> unknown: ...: <reason>`. `--root` reads a tree standing for the
//! host's `/` instead (fixtures); kernel queries (the NIC's ethtool settings,
//! chronyd, the clock check) then report unknown, or are left out.
//!
//! Exit 0: clean (no `warn`, and no `unknown` of a check that must prove
//! something, ergon-tune's `unknown_stops_require`). Exit 2: not clean.
//! Exit 1: the check could not run (a flag, the file).
#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use ergon_latency_core::clock::{TscClock, cross_cpu_check};
use ergon_tune::{
    Component, EthtoolCache, Finding, Host, HotSet, Nic, Role, TimeSyncProbe, busy_poll_sysctls, n1_adaptive_rx,
    n2_deferred_irqs, n3_azure_vf, t1_isolation, t2_nic_irqs, t3_thp, t4_aeron_dir, t5_numa_balancing, t6_nic_node,
    t8_smt_siblings, t9_cpu_quota, t10_smt_on, t11_time_sync, t12_clocksource, t13_workqueue_cpumask, t14_compaction,
    t15_host_tasks, t16_instrumentation, t17_ksm, t18_steal, unknown_stops_require,
};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

/// The clock check's id: a property of these CPUs, so an unknown blocks.
const CLOCK_ID: &str = "C1";

/// What the command line asks.
struct Args {
    hot: Vec<u32>,
    out: PathBuf,
    nics: Vec<String>,
    aeron_dir: Option<PathBuf>,
    root: Option<PathBuf>,
    clock_rounds: u64,
    /// `--smt on`: the session declares SMT on.
    smt_on: bool,
}

fn usage(why: &str) -> String {
    format!(
        "{why}\nusage: hostcheck --hot-cpus CSV --out FILE [--nic IFACE,...] [--aeron-dir PATH] [--smt off|on] \
         [--root DIR] [--clock-rounds N]"
    )
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        let key = flag
            .strip_prefix("--")
            .ok_or_else(|| usage(&format!("{flag:?}: every argument is a --flag")))?;
        let value = it.next().ok_or_else(|| usage(&format!("--{key} needs a value")))?;
        if map.insert(key.to_owned(), value).is_some() {
            return Err(usage(&format!("--{key} given twice")));
        }
    }
    let list = |v: Option<String>| -> Vec<String> {
        v.map(|s| s.split(',').filter(|x| !x.is_empty()).map(str::to_owned).collect())
            .unwrap_or_default()
    };
    let hot: Vec<u32> = list(map.remove("hot-cpus"))
        .iter()
        .map(|c| {
            c.parse()
                .map_err(|_| usage(&format!("--hot-cpus: {c} is not a CPU number")))
        })
        .collect::<Result<_, _>>()?;
    if hot.is_empty() {
        return Err(usage("--hot-cpus: at least one CPU"));
    }
    let out = PathBuf::from(map.remove("out").ok_or_else(|| usage("--out is required"))?);
    let a = Args {
        hot,
        out,
        nics: list(map.remove("nic")),
        aeron_dir: map.remove("aeron-dir").map(PathBuf::from),
        root: map.remove("root").map(PathBuf::from),
        clock_rounds: map.remove("clock-rounds").map_or(Ok(100_000), |v| {
            v.parse().map_err(|_| usage("--clock-rounds: a number"))
        })?,
        smt_on: match map.remove("smt").as_deref() {
            None | Some("off") => false,
            Some("on") => true,
            Some(v) => return Err(usage(&format!("--smt {v}: off or on"))),
        },
    };
    if let Some(k) = map.keys().next() {
        return Err(usage(&format!("unknown flag --{k}")));
    }
    Ok(a)
}

/// Pins the calling thread to `cpu` (the clock check's sides); whether it
/// is there.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn pin(cpu: usize) -> bool {
    if cpu >= usize::try_from(libc::CPU_SETSIZE).unwrap_or(0) {
        return false;
    }
    // SAFETY: a zeroed cpu_set_t is the empty set; CPU_SET writes one bit
    // below CPU_SETSIZE (checked above); sched_setaffinity only reads it.
    let rc = unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(cpu, &mut set);
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw const set)
    };
    rc == 0
}

#[cfg(not(target_os = "linux"))]
const fn pin(_cpu: usize) -> bool {
    false
}

/// `C1` for each pair of neighbouring hot CPUs: the counter never runs
/// behind across them. Live Linux hosts only; elsewhere one `info` line
/// says it was left out.
fn clock_findings(host: &Host, cpus: &[u32], rounds: u64) -> Vec<String> {
    if !host.is_live() || !cfg!(target_os = "linux") {
        return vec![format!(
            "{CLOCK_ID} info: the counter clock across the hot CPUs: not checked (a tree standing for a host, or not \
             Linux)"
        )];
    }
    if cpus.len() < 2 {
        return vec![format!("{CLOCK_ID} ok: one hot CPU ({}): no pair to compare", cpus[0])];
    }
    let clock = TscClock::new();
    cpus.windows(2)
        .map(|pair| {
            let (a, b) = (pair[0] as usize, pair[1] as usize);
            let unpinned = AtomicBool::new(false);
            let checked = cross_cpu_check(&clock, a, b, rounds, |cpu| {
                if !pin(cpu) {
                    unpinned.store(true, Ordering::Relaxed);
                }
            });
            if unpinned.load(Ordering::Relaxed) {
                return format!(
                    "{CLOCK_ID} unknown: the counter clock between CPUs {a} and {b}: could not pin a side there"
                );
            }
            match checked {
                Ok(r) if r.consistent() => format!(
                    "{CLOCK_ID} ok: the counter never ran behind between CPUs {a} and {b} over {} handoffs (the \
                     shortest {} ns)",
                    r.rounds, r.min_forward_ns
                ),
                // A property of the host (a hypervisor's per-vCPU counter
                // offsets), not a setting to fix: reported, not refused, so
                // every one-way figure across these CPUs carries this bound;
                // a round trip (both stamps on one CPU) is unaffected.
                Ok(r) => format!(
                    "{CLOCK_ID} info: the counter ran behind {} times in {} handoffs between CPUs {a} and {b}, by up \
                     to {} ns: one-way latency across them carries a {} ns error bound; round trips are unaffected \
                     (evidence: ergon_latency_core::clock::cross_cpu_check)",
                    r.backward, r.rounds, r.max_backward_ns, r.max_backward_ns
                ),
                Err(e) => format!("{CLOCK_ID} unknown: the counter clock between CPUs {a} and {b}: {e}"),
            }
        })
        .collect()
}

/// The first thread of `cpu`'s core (`thread_siblings_list`'s first CPU),
/// or `cpu` itself where that cannot be read.
fn first_thread(host: &Host, cpu: u32) -> u32 {
    let path = host
        .root()
        .join(format!("sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list"));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().split([',', '-']).next().and_then(|c| c.parse().ok()))
        .unwrap_or(cpu)
}

/// The hot CPUs' roles: each a loop; with `--smt on`, a hot CPU whose
/// core's first thread is hot too is that loop's `SQPOLL` thread.
fn hot_set(host: &Host, a: &Args) -> HotSet {
    let mut set = HotSet::new();
    for &cpu in &a.hot {
        let first = first_thread(host, cpu);
        let role = if a.smt_on && first != cpu && a.hot.contains(&first) {
            Role::SqPoll
        } else {
            Role::Loop
        };
        set.push(cpu, role);
    }
    set
}

/// `--smt on`'s record in the report.
const SMT_ON_LINE: &str = "S1 info: SMT on by the session's declaration (--smt on): of each pair of hot SMT \
                           siblings the core's first thread is checked as a loop and the other as its SQPOLL \
                           thread (T8), and T10 is reported, not blocking";

/// Every ergon-tune check for `a`.
fn findings(host: &Host, a: &Args) -> Vec<Finding> {
    let set = hot_set(host, a);
    let mut f = Vec::new();
    f.extend(t1_isolation(host, &set));
    f.extend(t3_thp(host, Component::Md));
    if let Some(dir) = &a.aeron_dir {
        f.push(t4_aeron_dir(host, dir));
    }
    f.push(t5_numa_balancing(host));
    f.extend(t8_smt_siblings(host, &set));
    f.push(t9_cpu_quota(host));
    f.push(t10_smt_on(host));
    let probe = TimeSyncProbe {
        chrony: if host.is_live() {
            TimeSyncProbe::default().chrony
        } else {
            None
        },
        ..TimeSyncProbe::default()
    };
    f.extend(t11_time_sync(host, &probe));
    f.push(t12_clocksource(host));
    f.push(t13_workqueue_cpumask(host, &set));
    f.push(t14_compaction(host));
    f.push(t15_host_tasks(host, &set));
    f.push(t16_instrumentation(host));
    f.push(t17_ksm(host));
    f.push(t18_steal(host, &set));
    let busy_polls = busy_poll_sysctls(host).ok().map(|b| b.busy_read > 0 || b.busy_poll > 0);
    let mut ethtool = EthtoolCache::new();
    for name in &a.nics {
        let nic = Nic::Interface(name.clone());
        f.extend(t2_nic_irqs(host, &nic, &set));
        f.extend(t6_nic_node(host, &nic, &set));
        f.push(n1_adaptive_rx(host, name, &mut ethtool));
        f.extend(n2_deferred_irqs(host, name, busy_polls));
        f.push(n3_azure_vf(host, name));
    }
    f
}

/// ergon-tune's line for `f`, on one line.
fn line(f: &Finding) -> String {
    f.to_string().replace(['\n', '\r'], " ")
}

/// Findings reported but never blocking: T11 (how `CLOCK_REALTIME` is kept,
/// e.g. NTP rather than a PTP hardware clock, as on AWS) only bounds
/// wall-clock stamps compared across hosts, and every latency this suite
/// reports is taken on one host's counter (round trips, and one-way rows
/// within one host).
const REPORTED_ONLY: &[&str] = &["T11"];

/// Reported but never blocking under `--smt on`: T10 (SMT on), which the
/// session measures on purpose.
const REPORTED_ONLY_SMT_ON: &[&str] = &["T10"];

/// Whether a line keeps the host from being clean (`smt_on`: `--smt on`).
fn blocks(line: &str, smt_on: bool) -> bool {
    let Some((id, rest)) = line.split_once(' ') else {
        return true;
    };
    if REPORTED_ONLY.contains(&id) || (smt_on && REPORTED_ONLY_SMT_ON.contains(&id)) {
        return false;
    }
    rest.starts_with("warn:") || (rest.starts_with("unknown:") && unknown_stops_require(id))
}

fn main() -> ExitCode {
    let a = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hostcheck: {e}");
            return ExitCode::FAILURE;
        }
    };
    let host = a.root.as_ref().map_or_else(Host::system, Host::at);
    let mut lines: Vec<String> = Vec::new();
    if a.smt_on {
        lines.push(SMT_ON_LINE.to_owned());
    }
    lines.extend(findings(&host, &a).iter().map(line));
    lines.extend(clock_findings(&host, &a.hot, a.clock_rounds));
    let mut body = String::new();
    for l in &lines {
        let _ = writeln!(body, "{l}");
    }
    if let Err(e) = std::fs::write(&a.out, body) {
        eprintln!("hostcheck: {}: {e}", a.out.display());
        return ExitCode::FAILURE;
    }
    let bad: Vec<&String> = lines.iter().filter(|l| blocks(l, a.smt_on)).collect();
    if bad.is_empty() {
        eprintln!("hostcheck: clean, {} lines in {}", lines.len(), a.out.display());
        return ExitCode::SUCCESS;
    }
    eprintln!("hostcheck: {} line(s) keep the host from being clean:", bad.len());
    for l in bad {
        eprintln!("  {l}");
    }
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::{Args, blocks, hot_set, parse};
    use ergon_tune::{Host, Role};

    #[test]
    fn a_wall_clock_warning_is_reported_not_blocking() {
        assert!(!blocks(
            "T11 warn: CLOCK_REALTIME is not disciplined to a PTP hardware clock (chronyd tracks NTP server 169.254.169.123)",
            false
        ));
        assert!(blocks("T1 warn: CPU 2 is not isolated", false));
        assert!(blocks("garbage", false));
        assert!(!blocks("T1 ok: isolated", false));
    }

    /// T10 blocks unless the session declared SMT on; nothing else changes.
    #[test]
    fn smt_on_reports_t10_and_blocks_the_rest() {
        let t10 = "T10 warn: SMT is on: on x86 a Kubernetes `cpu: 2` is one physical core";
        assert!(blocks(t10, false));
        assert!(!blocks(t10, true));
        assert!(blocks("T8 warn: CPU 2 and CPU 10 are hyperthreads of one core", true));
        assert!(blocks("T1 warn: CPU 10 is not isolated", true));
    }

    fn args(v: &[&str]) -> Result<Args, String> {
        parse(v.iter().map(|s| (*s).to_owned()))
    }

    /// `--smt`: off by default, on or off, nothing else.
    #[test]
    fn the_smt_flag_is_off_or_on() -> Result<(), String> {
        let base = ["--hot-cpus", "2,10", "--out", "x"];
        assert!(!args(&base)?.smt_on);
        assert!(args(&[&base[..], &["--smt", "on"]].concat())?.smt_on);
        assert!(!args(&[&base[..], &["--smt", "off"]].concat())?.smt_on);
        assert!(args(&[&base[..], &["--smt", "yes"]].concat()).is_err());
        Ok(())
    }

    /// Under `--smt on`, a hot CPU whose core's first thread is hot is that
    /// loop's SQPOLL thread; off, every hot CPU is a loop. A fixture tree:
    /// CPUs 2 and 10 one core, 3 alone.
    #[test]
    fn smt_on_pairs_each_loop_with_its_sibling() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("hostcheck-smt-{}", std::process::id()));
        for (cpu, sib) in [(2, "2,10"), (10, "2,10"), (3, "3")] {
            let d = root.join(format!("sys/devices/system/cpu/cpu{cpu}/topology"));
            std::fs::create_dir_all(&d)?;
            std::fs::write(d.join("thread_siblings_list"), format!("{sib}\n"))?;
        }
        let host = Host::at(&root);
        let mut a = args(&["--hot-cpus", "2,10,3", "--out", "x", "--smt", "on"])?;
        let set = hot_set(&host, &a);
        assert_eq!(set.roles(2), vec![Role::Loop]);
        assert_eq!(set.roles(10), vec![Role::SqPoll]);
        assert_eq!(set.roles(3), vec![Role::Loop]);
        a.smt_on = false;
        let set = hot_set(&host, &a);
        assert_eq!(set.roles(10), vec![Role::Loop]);
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
