//! `ergon-tune`: runs ergon-tune's host checks on this node (or the tree
//! `--root` names) and prints every finding with its fix, so a node or pod
//! can be checked before anything is deployed on it.
//!
//! ```text
//! ergon-tune [--hot CPUS] [--nic NAME|PCI]... [--tcp] [--busy-poll]
//!            [--component io|md|rt] [--root DIR] [--all] [--require]
//! ```
//!
//! - `--hot`: the busy-spinning CPUs (default: the node's isolated ones).
//! - `--nic`: a data NIC, a kernel interface (`ens6`) or a DPDK port's PCI
//!   address (`0000:00:06.0`); adds T2, T6 and T20. Repeatable.
//! - `--tcp`: the app sends latency-sensitive TCP (T21 warns, not informs).
//! - `--busy-poll`: sockets busy-poll (K5 asks for `CAP_NET_ADMIN`).
//! - `--component`: who runs on the hot CPUs, for T3's severity (default md).
//! - `--root`: check a tree mounted from a node (`/host`), not `/`.
//! - `--all`: print the checks that passed too.
//! - `--require`: exit 1 when a start under `ERGON_HOST_CHECKS=require`
//!   would refuse.
//!
//! Exit status: 0 reported, 1 refused (`--require`), 2 usage.

use std::process::ExitCode;

use ergon_tune::{
    Component, CpuSet, Finding, Host, HostCheckPolicy, HotSet, Needs, Nic, Role, Severity, Status,
    Thread, isolated_cpus, k8s_checks, t1_isolation, t2_nic_irqs, t3_thp, t5_numa_balancing,
    t6_nic_node, t8_smt_siblings, t9_cpu_quota, t10_smt_on, t12_clocksource, t13_workqueue_cpumask,
    t14_compaction, t15_host_tasks, t16_instrumentation, t17_ksm, t18_steal, t20_irq_idle,
    t21_tcp_autocorking,
};

const USAGE: &str = "usage: ergon-tune [--hot CPUS] [--nic NAME|PCI]... [--tcp] [--busy-poll] \
                     [--component io|md|rt] [--root DIR] [--all] [--require]";

#[allow(clippy::struct_excessive_bools)] // independent switches from the command line
struct Args {
    hot: Option<CpuSet>,
    nics: Vec<Nic>,
    tcp: bool,
    busy_poll: bool,
    component: Component,
    root: Option<String>,
    all: bool,
    require: bool,
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        hot: None,
        nics: Vec::new(),
        tcp: false,
        busy_poll: false,
        component: Component::Md,
        root: None,
        all: false,
        require: false,
    };
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--hot" => {
                let v = value("--hot")?;
                a.hot = Some(CpuSet::parse_list(&v).map_err(|e| format!("--hot {v}: {e}"))?);
            }
            "--nic" => {
                let v = value("--nic")?;
                // A PCI address is domain:bus:device.function.
                a.nics
                    .push(if v.matches(':').count() == 2 && v.contains('.') {
                        Nic::Pci(v)
                    } else {
                        Nic::Interface(v)
                    });
            }
            "--component" => {
                a.component = match value("--component")?.as_str() {
                    "io" => Component::Io,
                    "md" => Component::Md,
                    "rt" => Component::Rt,
                    other => return Err(format!("--component {other}: io, md or rt")),
                };
            }
            "--root" => a.root = Some(value("--root")?),
            "--tcp" => a.tcp = true,
            "--busy-poll" => a.busy_poll = true,
            "--all" => a.all = true,
            "--require" => a.require = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(a)
}

fn findings(host: &Host, a: &Args, spin: &HotSet) -> Vec<Finding> {
    let mut f = Vec::new();
    f.extend(t1_isolation(host, spin));
    f.extend(t8_smt_siblings(host, spin));
    f.push(t10_smt_on(host));
    f.extend(t3_thp(host, a.component));
    f.push(t5_numa_balancing(host));
    f.push(t9_cpu_quota(host));
    f.push(t12_clocksource(host));
    f.push(t13_workqueue_cpumask(host, spin));
    f.push(t14_compaction(host));
    f.push(t15_host_tasks(host, spin));
    f.push(t16_instrumentation(host));
    f.push(t17_ksm(host));
    f.push(t18_steal(host, spin));
    f.push(t21_tcp_autocorking(host, a.tcp));
    for nic in &a.nics {
        f.extend(t2_nic_irqs(host, nic, spin));
        f.extend(t6_nic_node(host, nic, spin));
    }
    if !a.nics.is_empty() {
        f.extend(t20_irq_idle(host, &a.nics, spin));
    }
    let mut needs = Needs::new(Thread::Current, Role::Loop);
    if a.busy_poll {
        needs = needs.busy_polls();
    }
    if let Some(nic) = a.nics.first() {
        needs = needs.with_nic(nic.clone());
    }
    f.extend(k8s_checks(host, spin, &needs));
    f
}

fn line(f: &Finding) -> String {
    let tag = match (&f.status, f.severity) {
        (Status::Ok, _) => "ok",
        (Status::Unknown(_), _) => "UNKNOWN",
        (Status::Finding, Severity::Warn) => "WARN",
        (Status::Finding, _) => "info",
    };
    format!("{tag} {f}")
}

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|a| a == "--help" || a == "-h") {
        println!(
            "{USAGE}\n\nRuns ergon-tune's host checks on this node and prints each finding with its fix.\nExit status: 0 reported, 1 refused (--require), 2 usage."
        );
        return ExitCode::SUCCESS;
    }
    let a = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ergon-tune: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let host = a.root.as_deref().map_or_else(Host::system, Host::at);
    let (cpus, from) = a.hot.as_ref().map_or_else(
        || (isolated_cpus(&host).unwrap_or_default(), "isolated"),
        |c| (c.clone(), "--hot"),
    );
    let mut spin = HotSet::new();
    for c in cpus.iter() {
        spin.push(c, Role::Loop);
    }
    if cpus.is_empty() {
        println!(
            "hot CPUs: none ({from}): the per-CPU checks have nothing to check; name them with --hot"
        );
    } else {
        println!("hot CPUs: {cpus} ({from})");
    }
    let all = findings(&host, &a, &spin);
    let (mut warn, mut info, mut unknown, mut ok) = (0, 0, 0, 0);
    for f in &all {
        match (&f.status, f.severity) {
            (Status::Ok, _) => ok += 1,
            (Status::Unknown(_), _) => unknown += 1,
            (Status::Finding, Severity::Warn) => warn += 1,
            (Status::Finding, _) => info += 1,
        }
        if a.all || f.status != Status::Ok {
            println!("{}", line(f));
        }
    }
    println!("summary: {warn} warnings, {info} info, {unknown} unknown, {ok} ok");
    if a.require
        && let Some(refusal) = HostCheckPolicy::Require.refusal(&all)
    {
        eprintln!("refused: {refusal}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
