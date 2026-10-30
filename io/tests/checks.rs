//! ergon-io's host and NIC checks (`ergon_io::checks`) against fixture
//! trees standing for a host's `/sys` and `/proc`, with a stand-in for the
//! kernel's ethtool answers: which CPUs `Driver::new` treats as hot, what it
//! reports about them, what the warning thread reports about a route's
//! egress device (and again when its `carrier_changes` moves), and the
//! lines for `TCP_CORK`, `SO_RCVBUF` and NAPI ids. Pure: runs on every OS.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use ergon_io::checks::{self, DevChecks, DevContext, LoopCpu};
use ergon_io::placement::{Allowed, CpuError, Why};
use ergon_io::route::{self, Flow};
use ergon_io::{Backend, Config, ConnId};
use ergon_tune::{
    Coalesce, CpuSet, Ethtool, EthtoolCache, EthtoolError, Finding, Host, HostCheckPolicy, HotSet,
    Rings, Role, SILENCE, Status,
};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// A host tree under a temporary directory, removed when dropped.
struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(name: &str) -> R<Self> {
        let root =
            std::env::temp_dir().join(format!("ergon-io-checks-{name}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root)?;
        }
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn host(&self) -> Host {
        Host::at(&self.root)
    }

    fn at(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    fn file(&self, abs: &str, contents: &str) -> R<&Self> {
        let p = self.at(abs);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, contents)?;
        Ok(self)
    }

    fn link(&self, abs: &str, target: &str) -> R<&Self> {
        let p = self.at(abs);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        if p.symlink_metadata().is_ok() {
            std::fs::remove_file(&p)?;
        }
        std::os::unix::fs::symlink(target, p)?;
        Ok(self)
    }

    /// `n` CPUs, CPU c's SMT sibling `c ^ smt` when `smt` is set.
    fn cpus(&self, n: u32, smt: Option<u32>) -> R<&Self> {
        self.file("/sys/devices/system/cpu/online", &format!("0-{}\n", n - 1))?;
        for c in 0..n {
            let s = smt.map_or_else(
                || c.to_string(),
                |half| format!("{},{}", c % half, c % half + half),
            );
            self.file(
                &format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list"),
                &format!("{s}\n"),
            )?;
        }
        Ok(self)
    }

    fn isolation(&self, isolated: &str, nohz_full: Option<&str>, default_mask: &str) -> R<&Self> {
        self.file("/sys/devices/system/cpu/isolated", &format!("{isolated}\n"))?;
        if let Some(n) = nohz_full {
            self.file("/sys/devices/system/cpu/nohz_full", &format!("{n}\n"))?;
        }
        self.file(
            "/proc/irq/default_smp_affinity",
            &format!("{default_mask}\n"),
        )?;
        Ok(self)
    }

    fn thp(&self, enabled: &str) -> R<&Self> {
        self.file("/sys/kernel/mm/transparent_hugepage/enabled", enabled)?;
        Ok(self)
    }

    /// This process in a container's own cgroup (`0::/`) whose `cpu.max`
    /// is `cpu_max`.
    fn cpu_max(&self, cpu_max: &str) -> R<&Self> {
        self.file("/proc/self/cgroup", "0::/\n")?;
        self.file("/sys/fs/cgroup/cpu.max", cpu_max)?;
        Ok(self)
    }

    /// The calling thread's policy (field 41 of stat) and affinity.
    fn thread(&self, policy: u32, cpus_allowed: &str) -> R<&Self> {
        let mut fields: Vec<String> = vec!["S".to_owned()];
        fields.extend((4..=40).map(|i| i.to_string()));
        fields.push(policy.to_string());
        fields.extend((0..10).map(|_| "0".to_owned()));
        self.file(
            "/proc/thread-self/stat",
            &format!("4242 (loop) {}\n", fields.join(" ")),
        )?;
        self.file(
            "/proc/thread-self/status",
            &format!("Name:\tloop\nCpus_allowed_list:\t{cpus_allowed}\n"),
        )?;
        Ok(self)
    }

    /// An ENA interface on PCI `bdf` whose queue interrupts have `affinity`.
    fn ena(&self, name: &str, ifindex: u32, bdf: &str, affinity: &str) -> R<&Self> {
        let d = format!("/sys/bus/pci/devices/{bdf}");
        std::fs::create_dir_all(self.at("/sys/bus/pci/drivers/ena"))?;
        self.link(&format!("{d}/driver"), "../../../../bus/pci/drivers/ena")?;
        self.link(&format!("{d}/subsystem"), "../../../../bus/pci")?;
        self.file(&format!("{d}/numa_node"), "-1\n")?;
        self.file(&format!("{d}/irq"), "0\n")?;
        for (n, q) in [(61, 0), (62, 1)] {
            self.file(&format!("{d}/msi_irqs/{n}"), "msix\n")?;
            self.file(
                &format!("/proc/irq/{n}/smp_affinity_list"),
                &format!("{affinity}\n"),
            )?;
            self.file(
                &format!("/sys/kernel/irq/{n}/actions"),
                &format!("{name}-Tx-Rx-{q}\n"),
            )?;
        }
        self.netdev(name, ifindex)?;
        self.link(
            &format!("/sys/class/net/{name}/device"),
            &format!("../../../bus/pci/devices/{bdf}"),
        )?;
        Ok(self)
    }

    /// An interface's own files (no device: a veth on its own).
    fn netdev(&self, name: &str, ifindex: u32) -> R<&Self> {
        let d = format!("/sys/class/net/{name}");
        self.file(&format!("{d}/ifindex"), &format!("{ifindex}\n"))?;
        self.file(&format!("{d}/carrier_changes"), "2\n")?;
        self.file(&format!("{d}/operstate"), "up\n")?;
        self.file(&format!("{d}/napi_defer_hard_irqs"), "0\n")?;
        self.file(&format!("{d}/gro_flush_timeout"), "0\n")?;
        Ok(self)
    }
}

/// An untuned 8-CPU x86 host with SMT (CPU c's sibling c+4): nothing
/// isolated, no `nohz_full`, every CPU in the default IRQ affinity, THP
/// `madvise`, the calling thread `SCHED_OTHER` with `cpus_allowed`, in a
/// container with a CPU quota of 2 CPUs.
fn untuned(name: &str, cpus_allowed: &str) -> R<Fixture> {
    let f = Fixture::new(name)?;
    f.cpus(8, Some(4))?
        .isolation("", None, "ff")?
        .thp("always [madvise] never\n")?
        .thread(0, cpus_allowed)?
        .cpu_max("200000 100000\n")?;
    Ok(f)
}

/// A tuned one: SMT off (one vCPU is one physical core), CPUs 1-3 and 5-7
/// isolated and `nohz_full`, IRQs on 0 and 4, THP never, no CPU quota.
fn tuned(name: &str, cpus_allowed: &str) -> R<Fixture> {
    let f = Fixture::new(name)?;
    f.cpus(8, None)?
        .file("/sys/devices/system/cpu/smt/active", "0\n")?
        .isolation("1-3,5-7", Some("1-3,5-7"), "11")?
        .thp("always madvise [never]\n")?
        .thread(0, cpus_allowed)?
        .cpu_max("max 100000\n")?;
    // T13-T18 and T21: unbound workqueues on the housekeeping CPUs (0, 4),
    // compaction and KSM off, no instrumentation, no steal, no other task
    // on a hot CPU, autocorking off.
    f.file("/sys/devices/virtual/workqueue/cpumask", "11\n")?
        .file("/proc/sys/vm/compaction_proactiveness", "0\n")?
        .file("/proc/sys/vm/compact_unevictable_allowed", "0\n")?
        .file("/sys/kernel/mm/ksm/run", "0\n")?
        .file("/sys/kernel/debug/kprobes/list", "")?
        .file("/sys/kernel/debug/kprobes/enabled", "1\n")?
        .file("/sys/kernel/tracing/enabled_functions", "")?
        .file("/proc/kallsyms", "ffffffff81000000 T _stext\n")?
        .file("/proc/sys/net/core/bpf_jit_enable", "1\n")?
        .file("/proc/sys/net/core/bpf_jit_kallsyms", "1\n")?
        .file("/proc/sys/net/core/bpf_jit_harden", "0\n")?
        .file("/proc/sys/net/ipv4/tcp_autocorking", "0\n")?;
    let stat: String = std::iter::once("cpu  800 0 800 80000 0 0 0 0 0 0\n".to_owned())
        .chain((0..8).map(|c| format!("cpu{c} 100 0 100 10000 0 0 0 0 0 0\n")))
        .collect();
    f.file("/proc/stat", &stat)?;
    for (pid, name, flags, cpus) in [
        (2, "kthreadd", 0x0020_0000_u64, "0-7"),
        (812, "sshd", 0, "0,4"),
    ] {
        let d = format!("/proc/{pid}/task/{pid}");
        let mut fields = vec!["S".to_owned()];
        fields.extend((4..=52).map(|i| {
            if i == 9 {
                flags.to_string()
            } else {
                "0".to_owned()
            }
        }));
        f.file(
            &format!("{d}/stat"),
            &format!("{pid} ({name}) {}\n", fields.join(" ")),
        )?
        .file(
            &format!("{d}/status"),
            &format!("Name:\t{name}\nState:\tS (sleeping)\nCpus_allowed_list:\t{cpus}\n"),
        )?;
    }
    Ok(f)
}

fn ids(v: &[Finding]) -> Vec<&'static str> {
    v.iter().map(|f| f.id).collect()
}

fn lines(v: &[Finding]) -> Vec<String> {
    v.iter().filter_map(checks::line).collect()
}

// ---------------------------------------------------------------- I-host

#[test]
fn a_loop_pinned_to_one_cpu_is_checked_there() -> R {
    let f = untuned("pinned", "2")?;
    let cfg = Config::new(Backend::RecvBusy);
    let r = checks::host_report(&f.host(), &cfg);
    assert_eq!(r.loop_cpu, LoopCpu::Pinned(2));
    assert_eq!(r.hot, HotSet::new().with(2, Role::Loop));
    let found = ids(&r.findings);
    for id in [
        "T1", "T3", "T7", "T8", "T9", "T10", "T13", "T14", "T15", "T16", "T17", "T18", "T21",
    ] {
        assert!(found.contains(&id), "{id} missing: {found:?}");
    }
    let text = r.lines();
    assert!(
        text.iter()
            .any(|l| l.starts_with("ergon-io warning: T1 warn: CPU 2 (loop) is not isolated")),
        "{text:#?}"
    );
    assert!(
        text.iter()
            .any(|l| l.starts_with("ergon-io warning: T1 warn: CPU 2 (loop) is not nohz_full")),
        "{text:#?}"
    );
    // THP is information for ergon-io, never a warning.
    assert!(
        text.iter()
            .any(|l| l.starts_with("ergon-io info: T3 info:")),
        "{text:#?}"
    );
    assert!(!text.iter().any(|l| l.contains("T3 warn")), "{text:#?}");
    // SCHED_OTHER: T7 is fine and says nothing.
    assert!(!text.iter().any(|l| l.contains("T7")), "{text:#?}");
    // A CPU quota stops the busy-spinning loop when the container uses it up.
    assert!(
        text.iter().any(|l| l.starts_with(
            "ergon-io warning: T9 warn: this process's cgroup: /sys/fs/cgroup/cpu.max is \
             `200000 100000`"
        ) && l.contains("Never set kubelet `cpuCFSQuota: false`")
            && l.contains("whole number")),
        "{text:#?}"
    );
    // Its idle sibling (6) is not isolated: information, not a warning.
    assert!(
        text.iter()
            .any(|l| l
                .starts_with("ergon-io info: T8 info: CPU 2 (loop) shares its core with CPU 6")),
        "{text:#?}"
    );
    // SMT is on: a Kubernetes `cpu: 2` is one physical core. The warning
    // says how to turn SMT off, and how to silence it.
    let smt: Vec<&String> = text.iter().filter(|l| l.contains("T10 warn")).collect();
    assert_eq!(smt.len(), 1, "{text:#?}");
    assert!(
        smt[0].starts_with("ergon-io warning: T10 warn: SMT is on"),
        "{}",
        smt[0]
    );
    assert!(smt[0].ends_with(SILENCE), "{}", smt[0]);
    Ok(())
}

#[test]
fn a_tuned_host_reports_nothing() -> R {
    let f = tuned("tuned", "2")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    assert_eq!(r.lines(), Vec::<String>::new(), "{:#?}", r.findings);
    let found = ids(&r.findings);
    for id in [
        "T1", "T3", "T7", "T8", "T9", "T10", "T13", "T14", "T15", "T16", "T17", "T18", "T21",
    ] {
        assert!(found.contains(&id), "{id} not checked: {found:?}");
    }
    assert!(
        r.findings.iter().all(|f| f.status == Status::Ok),
        "{:#?}",
        r.findings
    );
    Ok(())
}

/// `ERGON_HOST_CHECKS`: `off` drops the warning lines (the findings stay),
/// `require` fails the start on a warning.
#[test]
fn the_host_check_switch_silences_warnings_and_require_refuses() -> R {
    let f = untuned("switch", "2")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    assert!(
        r.findings.iter().any(Finding::is_warning),
        "{:#?}",
        r.findings
    );
    let on = r.lines_under(HostCheckPolicy::On);
    assert!(
        on.iter()
            .any(|l| l.starts_with("ergon-io warning: ") && l.ends_with(SILENCE)),
        "{on:#?}"
    );
    let off = r.lines_under(HostCheckPolicy::Off);
    assert!(
        !off.iter().any(|l| l.starts_with("ergon-io warning: ")),
        "{off:#?}"
    );
    assert!(
        r.findings.iter().any(Finding::is_warning),
        "the results are still there"
    );
    assert_eq!(r.refusal(HostCheckPolicy::Off), None);
    assert_eq!(r.refusal(HostCheckPolicy::On), None);
    let e = r
        .refusal(HostCheckPolicy::Require)
        .ok_or("require did not refuse")?;
    let text = e.to_string();
    assert!(text.contains("ERGON_HOST_CHECKS=require"), "{text}");
    assert!(text.contains("T10"), "{text}");
    assert!(
        text.contains("ERGON_HOST_CHECKS=off to start anyway"),
        "{text}"
    );
    Ok(())
}

/// Inside a pod (a `kubepods` cgroup path) the Kubernetes checks run too;
/// outside one they do not.
#[test]
fn in_a_pod_the_kubernetes_checks_run_too() -> R {
    let f = tuned("kpod", "2")?;
    f.file("/proc/self/cgroup", "0::/kubepods/burstable/pod8f3c2a1b\n")?;
    let dir = "/sys/fs/cgroup/kubepods/burstable/pod8f3c2a1b";
    f.file(&format!("{dir}/cpuset.cpus.effective"), "2\n")?
        .file(&format!("{dir}/cpu.max"), "max 100000\n")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    let found = ids(&r.findings);
    for id in ["K1", "K4"] {
        assert!(found.contains(&id), "{id} missing: {found:?}");
    }
    // Not a pod: no K check.
    let f = tuned("notpod", "2")?;
    let found = ids(&checks::host_report(&f.host(), &Config::new(Backend::RecvBusy)).findings);
    assert!(!found.contains(&"K1"), "{found:?}");
    Ok(())
}

#[test]
fn the_configured_loop_cpu_wins_over_the_affinity() -> R {
    let f = tuned("configured", "0-7")?;
    let mut cfg = Config::new(Backend::RecvBusy);
    cfg.loop_cpu = Some(3);
    let r = checks::host_report(&f.host(), &cfg);
    assert_eq!(r.loop_cpu, LoopCpu::Configured(3));
    assert_eq!(r.hot, HotSet::new().with(3, Role::Loop));
    Ok(())
}

#[test]
fn an_unpinned_loop_gets_one_information_line_and_no_cpu_checks() -> R {
    let f = untuned("unpinned", "0-7")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    assert_eq!(r.loop_cpu, LoopCpu::Unpinned("0-7".to_owned()));
    assert_eq!(r.hot, HotSet::new());
    let found = ids(&r.findings);
    assert!(!found.contains(&"T1"), "{found:?}");
    assert!(!found.contains(&"T8"), "{found:?}");
    // The thread's policy, THP and the CPU quota do not depend on the CPU:
    // still checked.
    assert!(found.contains(&"T7"), "{found:?}");
    assert!(found.contains(&"T3"), "{found:?}");
    assert!(found.contains(&"T9"), "{found:?}");
    let text = r.lines();
    let notes: Vec<&String> = text.iter().filter(|l| l.contains("not pinned")).collect();
    assert_eq!(notes.len(), 1, "{text:#?}");
    let note = notes[0];
    assert!(note.starts_with("ergon-io info: "), "{note}");
    for part in ["0-7", "loop_cpu", "T1", "T2", "T8"] {
        assert!(note.contains(part), "{part:?} not in {note:?}");
    }
    Ok(())
}

/// With no `loop_cpu` and an unpinned creating thread the polling CPU is
/// unknown, so the helper choice cannot keep off it: the report says so,
/// and how to make it known.
#[test]
fn with_the_polling_cpu_unknown_the_helpers_may_include_it() -> R {
    let f = untuned("unknownpoll", "0-7")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    assert_eq!(r.loop_cpu, LoopCpu::Unpinned("0-7".to_owned()));
    let line = r.helper_line().ok_or("no helper line")?;
    for part in ["polling CPU is unknown", "may include it", "loop_cpu"] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
    let note = r
        .lines()
        .into_iter()
        .find(|l| l.contains("not pinned"))
        .ok_or("no unpinned note")?;
    assert!(note.contains("may include"), "{note}");
    Ok(())
}

#[test]
fn an_unreadable_affinity_is_unknown_not_a_warning() -> R {
    let f = Fixture::new("noaffinity")?;
    f.cpus(4, None)?
        .isolation("", None, "f")?
        .thp("[never]\n")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    assert!(
        matches!(r.loop_cpu, LoopCpu::Unknown(_)),
        "{:?}",
        r.loop_cpu
    );
    assert_eq!(r.hot, HotSet::new());
    let text = r.lines();
    assert!(
        text.iter().all(|l| !l.starts_with("ergon-io warning:")),
        "{text:#?}"
    );
    Ok(())
}

#[test]
fn sqpoll_cpu_is_hot_for_uring_sqpoll_only() -> R {
    let f = tuned("sqpoll", "2")?;
    let mut cfg = Config::new(Backend::UringSqpoll);
    cfg.sqpoll_cpu = Some(6);
    let r = checks::host_report(&f.host(), &cfg);
    assert_eq!(
        r.hot,
        HotSet::new().with(2, Role::Loop).with(6, Role::SqPoll)
    );
    // SMT is off on a tuned host: SQPOLL's own CPU is not a sibling of the
    // loop's, so T8 has nothing to warn about.
    assert!(
        !r.lines().iter().any(|l| l.contains("T8 warn")),
        "{:#?}",
        r.lines()
    );
    let mut recv = Config::new(Backend::RecvBusy);
    recv.sqpoll_cpu = Some(6);
    assert_eq!(
        checks::host_report(&f.host(), &recv).hot,
        HotSet::new().with(2, Role::Loop)
    );
    Ok(())
}

#[test]
fn a_real_time_loop_is_warned_about() -> R {
    let f = tuned("fifo", "2")?;
    f.thread(1, "2")?;
    let text = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy)).lines();
    assert!(
        text.iter()
            .any(|l| l.starts_with("ergon-io warning: T7 warn: the loop thread runs SCHED_FIFO")),
        "{text:#?}"
    );
    Ok(())
}

// ------------------------------------------------- helper thread CPUs

/// A Kubernetes container's cpuset (cgroup v2, its own cgroup namespace).
fn pod(f: &Fixture, cpuset: &str) -> R<()> {
    f.file("/proc/self/cgroup", "0::/\n")?.file(
        "/sys/fs/cgroup/cpuset.cpus.effective",
        &format!("{cpuset}\n"),
    )?;
    Ok(())
}

/// `cpu: 1`: the pod's one CPU is the loop's. The helper threads have
/// nowhere else to go: one warning says what refuses and what to do.
#[test]
fn a_single_cpu_pod_is_warned_about_at_start() -> R {
    let f = untuned("pod1", "2")?;
    pod(&f, "2")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    let p = r.helpers.clone().map_err(|e| e.to_string())?;
    assert_eq!((p.cpus.clone(), p.why), (CpuSet::new(), Why::NoSpareCpu));
    assert_eq!(p.allowed, CpuSet::of([2]));
    let text = r.lines();
    let one: Vec<&String> = text
        .iter()
        .filter(|l| l.contains("polling CPU only"))
        .collect();
    assert_eq!(one.len(), 1, "{text:#?}");
    assert!(one[0].starts_with("ergon-io warning: "), "{}", one[0]);
    assert!(one[0].ends_with(SILENCE), "{}", one[0]);
    for part in [
        "connect_background",
        "uring-sqpoll",
        "Driver::connect",
        "warning thread",
        "2 CPUs",
        "Guaranteed",
    ] {
        assert!(one[0].contains(part), "{part:?} not in {:?}", one[0]);
    }
    assert_eq!(r.helper_line(), None, "the warning says it all");
    // The switch drops that warning line too.
    assert!(
        !r.lines_under(HostCheckPolicy::Off)
            .iter()
            .any(|l| l.contains("polling CPU only")),
        "{:#?}",
        r.lines_under(HostCheckPolicy::Off)
    );
    Ok(())
}

/// x86 `cpu: 2` with whole cores: the loop's SMT sibling, reported with
/// why and from where.
#[test]
fn the_helper_choice_is_reported() -> R {
    let f = untuned("pod2", "2")?;
    pod(&f, "2,6")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::RecvBusy));
    let p = r.helpers.clone().map_err(|e| e.to_string())?;
    assert_eq!(
        (p.cpus.clone(), p.why, p.polling),
        (CpuSet::of([6]), Why::Sibling, Some(2))
    );
    assert_eq!(
        p.source,
        Allowed::Cgroup("/sys/fs/cgroup/cpuset.cpus.effective".to_owned())
    );
    assert!(
        !r.lines().iter().any(|l| l.contains("polling CPU only")),
        "{:#?}",
        r.lines()
    );
    let line = r.helper_line().ok_or("no helper line")?;
    assert!(line.starts_with("ergon-io info: "), "{line}");
    for part in ["CPU 6", "SMT sibling", "2,6", "cpuset.cpus.effective"] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
    Ok(())
}

/// `uring-sqpoll` without `sqpoll_cpu`: its default (the first helper CPU)
/// is hot like a configured one, and T8 has nothing to warn about (SMT is
/// off on a tuned host).
#[test]
fn uring_sqpolls_default_cpu_is_hot() -> R {
    let f = tuned("sqdefault", "2")?;
    pod(&f, "2,6")?;
    let r = checks::host_report(&f.host(), &Config::new(Backend::UringSqpoll));
    assert_eq!(
        r.hot,
        HotSet::new().with(2, Role::Loop).with(6, Role::SqPoll)
    );
    assert!(
        !r.lines().iter().any(|l| l.contains("T8 warn")),
        "{:#?}",
        r.lines()
    );
    Ok(())
}

/// `helper_cpus` are the shared-work CPUs (never a spinner or `SQPOLL`
/// there): `uring-sqpoll` without `sqpoll_cpu` takes a CPU of the pod's own
/// outside that list, and the start-up line says so. With none left, SQPOLL
/// is not hot: `Driver::new` refuses with `NoExclusiveCpu`.
#[test]
fn uring_sqpolls_default_stays_off_an_explicit_helper_list() -> R {
    let f = tuned("sqshared", "2")?;
    pod(&f, "2,6,7")?;
    let mut cfg = Config::new(Backend::UringSqpoll);
    cfg.helper_cpus = vec![7];
    let r = checks::host_report(&f.host(), &cfg);
    assert_eq!(
        r.hot,
        HotSet::new().with(2, Role::Loop).with(6, Role::SqPoll)
    );
    let line = r.helper_line().ok_or("no helper line")?;
    for part in [
        "CPU 7",
        "Config::helper_cpus",
        "CPU 6",
        "never a spinner on shared CPUs",
    ] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
    let f = tuned("sqshared1", "2")?;
    pod(&f, "2,7")?;
    let mut cfg = Config::new(Backend::UringSqpoll);
    cfg.helper_cpus = vec![7];
    assert_eq!(
        checks::host_report(&f.host(), &cfg).hot,
        HotSet::new().with(2, Role::Loop)
    );
    Ok(())
}

/// `helper_cpus` naming a CPU outside the pod: an error in the report,
/// which `Driver::new` returns.
#[test]
fn helper_cpus_outside_the_cpuset_are_an_error() -> R {
    let f = untuned("outside", "2")?;
    pod(&f, "2,6")?;
    let mut cfg = Config::new(Backend::RecvBusy);
    cfg.helper_cpus = vec![5];
    let r = checks::host_report(&f.host(), &cfg);
    assert_eq!(
        r.helpers,
        Err(CpuError::Outside {
            setting: "helper_cpus",
            cpu: 5,
            allowed: CpuSet::of([2, 6])
        })
    );
    Ok(())
}

// ---------------------------------------------------------------- I-dev

/// Ethtool answers by interface index, counting the questions.
#[derive(Default)]
struct Fake {
    coalesce: HashMap<u32, Result<Coalesce, EthtoolError>>,
    asked: usize,
}

impl Ethtool for Fake {
    fn coalesce(&mut self, _: &Host, ifindex: u32) -> Result<Coalesce, EthtoolError> {
        self.asked += 1;
        self.coalesce
            .get(&ifindex)
            .cloned()
            .unwrap_or(Err(EthtoolError::Kernel(19)))
    }

    fn rings(&mut self, _: &Host, _: u32) -> Result<Rings, EthtoolError> {
        Err(EthtoolError::Kernel(95))
    }
}

fn adaptive_on() -> DevChecks<Fake> {
    let mut fake = Fake::default();
    fake.coalesce.insert(
        3,
        Ok(Coalesce {
            use_adaptive_rx: Some(true),
            rx_usecs: Some(20),
        }),
    );
    DevChecks::with_cache(EthtoolCache::with_source(fake))
}

// Not `const`: `Config` owns a `Vec` (`helper_cpus`).
fn ctx(hot: HotSet, backend: Backend) -> DevContext {
    DevContext::new(&Config::new(backend), hot)
}

#[test]
fn the_egress_device_is_checked_once_and_again_after_a_reset() -> R {
    let f = tuned("dev", "2")?;
    // ENA re-places queue interrupts on CPU i mod n at every reset: here
    // both landed on the loop's CPU 2.
    f.ena("ens5", 3, "0000:00:05.0", "2")?;
    let host = f.host();
    let mut dev = adaptive_on();
    let c = ctx(HotSet::new().with(2, Role::Loop), Backend::RecvBusy);
    let first = dev.check(&host, "ens5", &c);
    assert_eq!(warned(&first), vec!["T2", "T2", "N1"], "{first:#?}");
    // A polling backend: N2 does not apply.
    assert!(!ids(&first).contains(&"N2"), "{first:#?}");
    assert!(ids(&first).contains(&"N3"), "{first:#?}");
    assert!(dev.watching());
    // The same device and context again: nothing new, nothing asked.
    assert_eq!(dev.check(&host, "ens5", &c), Vec::new());
    assert_eq!(dev.recheck(&host), Vec::new());
    // The interface resets (its carrier comes back): every check again,
    // once the counter has settled.
    f.file("/sys/class/net/ens5/carrier_changes", "4\n")?;
    let again = settled(&mut dev, &host)?;
    assert_eq!(warned(&again), vec!["T2", "T2", "N1"], "{again:#?}");
    assert_eq!(dev.recheck(&host), Vec::new(), "once per reset");
    // Fixed after the reset: the next one finds nothing to warn about.
    f.ena("ens5", 3, "0000:00:05.0", "0")?;
    f.file("/sys/class/net/ens5/carrier_changes", "6\n")?;
    let fixed = settled(&mut dev, &host)?;
    assert!(
        fixed.iter().all(|f| f.id != "T2" || !f.is_warning()),
        "{fixed:#?}"
    );
    Ok(())
}

fn warned(v: &[Finding]) -> Vec<&'static str> {
    v.iter().filter(|f| f.is_warning()).map(|f| f.id).collect()
}

/// Calls `recheck` as the warning thread does, once a read, until it
/// reports: it must stay quiet for exactly the settling reads first.
fn settled(dev: &mut DevChecks<Fake>, host: &Host) -> R<Vec<Finding>> {
    for read in 0..checks::SETTLE_READS {
        let early = dev.recheck(host);
        if !early.is_empty() {
            return Err(format!("re-checked at read {read}, before the carrier settled").into());
        }
    }
    Ok(dev.recheck(host))
}

#[test]
fn a_reconnect_right_after_a_reset_does_not_swallow_the_recheck() -> R {
    let f = tuned("dev-reconnect", "2")?;
    f.ena("ens5", 3, "0000:00:05.0", "2")?;
    let host = f.host();
    let mut dev = adaptive_on();
    let c = ctx(HotSet::new().with(2, Role::Loop), Backend::RecvBusy);
    assert_eq!(warned(&dev.check(&host, "ens5", &c)).len(), 3);
    // The interface resets, and the application reconnects at once: the
    // new connection's check comes before the warning thread's next read.
    // The warning thread prints `check`'s findings once per process, so a
    // re-check made there would be dropped as a repeat: it must be left to
    // `recheck`, whose findings are printed again.
    f.file("/sys/class/net/ens5/carrier_changes", "4\n")?;
    assert_eq!(dev.check(&host, "ens5", &c), Vec::new());
    let again = settled(&mut dev, &host)?;
    assert_eq!(warned(&again), vec!["T2", "T2", "N1"], "{again:#?}");
    Ok(())
}

#[test]
fn a_recheck_waits_until_the_carrier_counter_has_settled() -> R {
    let f = tuned("dev-settle", "2")?;
    f.ena("ens5", 3, "0000:00:05.0", "0")?;
    let host = f.host();
    let mut dev = adaptive_on();
    let c = ctx(HotSet::new().with(2, Role::Loop), Backend::RecvBusy);
    let first = dev.check(&host, "ens5", &c);
    assert!(!warned(&first).contains(&"T2"), "{first:#?}");
    // A reset: the carrier goes down (the counter moves once) and the
    // driver re-places its queue interrupts on the loop's CPU before it
    // comes back up. A re-pin hook (a link-event watcher) hears of the
    // link change only later, so a check now would warn about a host that
    // fixes itself.
    // (`ena` rewrites the interface's files, the counter included.)
    f.ena("ens5", 3, "0000:00:05.0", "2")?;
    f.file("/sys/class/net/ens5/carrier_changes", "3\n")?;
    assert_eq!(dev.recheck(&host), Vec::new(), "checked mid-reset");
    f.file("/sys/class/net/ens5/carrier_changes", "4\n")?;
    assert_eq!(
        dev.recheck(&host),
        Vec::new(),
        "checked as the carrier came back"
    );
    // The hook re-pins; the counter then holds still, and the device is
    // checked once it has held for the settling reads.
    f.ena("ens5", 3, "0000:00:05.0", "0")?;
    f.file("/sys/class/net/ens5/carrier_changes", "4\n")?;
    for _ in 1..checks::SETTLE_READS {
        assert_eq!(dev.recheck(&host), Vec::new(), "checked before it settled");
    }
    let after = dev.recheck(&host);
    assert!(
        after.iter().any(|f| f.id == "N1"),
        "not re-checked: {after:#?}"
    );
    assert!(!warned(&after).contains(&"T2"), "{after:#?}");
    Ok(())
}

#[test]
fn n2_is_checked_for_interrupt_driven_backends_only() -> R {
    let f = tuned("n2", "2")?;
    f.ena("ens5", 3, "0000:00:05.0", "0")?;
    f.file("/sys/class/net/ens5/napi_defer_hard_irqs", "2\n")?;
    f.file("/sys/class/net/ens5/gro_flush_timeout", "200000\n")?;
    let host = f.host();
    let loop_cpu = HotSet::new().with(2, Role::Loop);
    let spin = adaptive_on().check(&host, "ens5", &ctx(loop_cpu.clone(), Backend::EpollSpin));
    assert!(
        spin.iter().any(|f| f.id == "N2" && f.is_warning()),
        "{spin:#?}"
    );
    let busy = adaptive_on().check(&host, "ens5", &ctx(loop_cpu, Backend::RecvBusy));
    assert!(!ids(&busy).contains(&"N2"), "{busy:#?}");
    Ok(())
}

#[test]
fn an_unpinned_loop_skips_t2_but_not_the_nic_settings() -> R {
    let f = tuned("dev-unpinned", "0-7")?;
    f.ena("ens5", 3, "0000:00:05.0", "2")?;
    let found = adaptive_on().check(&f.host(), "ens5", &ctx(HotSet::new(), Backend::RecvBusy));
    assert!(!ids(&found).contains(&"T2"), "{found:#?}");
    assert!(
        found.iter().any(|f| f.id == "N1" && f.is_warning()),
        "{found:#?}"
    );
    Ok(())
}

#[test]
fn a_device_not_visible_here_is_one_unknown_line() -> R {
    let f = tuned("veth", "2")?;
    // A pod's eth0: a veth, no device and no lower interface.
    f.netdev("eth0", 9)?;
    let mut dev = adaptive_on();
    let found = dev.check(
        &f.host(),
        "eth0",
        &ctx(HotSet::new().with(2, Role::Loop), Backend::RecvBusy),
    );
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].is_unknown(), "{found:#?}");
    let text = lines(&found);
    assert_eq!(text.len(), 1);
    assert!(text[0].starts_with("ergon-io info: "), "{text:?}");
    for part in ["eth0", "T2", "N1", "N2", "N3", "veth"] {
        assert!(text[0].contains(part), "{part:?} not in {text:?}");
    }
    // Nothing to re-check: the warning thread need not wake once a second
    // for it.
    assert!(!dev.watching());
    Ok(())
}

// ---------------------------------------------------------------- lines

#[test]
fn ok_findings_print_nothing_and_unknown_ones_are_information() {
    let ok = Finding {
        id: "T1",
        severity: ergon_tune::Severity::Warn,
        status: Status::Ok,
        message: "fine".to_owned(),
        evidence: "",
    };
    assert_eq!(checks::line(&ok), None);
    let unknown = Finding {
        status: Status::Unknown("no such file".to_owned()),
        ..ok.clone()
    };
    assert_eq!(
        checks::line(&unknown).as_deref(),
        Some("ergon-io info: T1 unknown: fine: no such file")
    );
    // A warning ends with how to turn it off.
    let warn = Finding {
        status: Status::Finding,
        ..ok
    };
    let line = checks::line(&warn).unwrap_or_default();
    assert!(
        line.starts_with("ergon-io warning: T1 warn: fine"),
        "{line}"
    );
    assert!(line.ends_with(SILENCE), "{line}");
}

#[test]
fn a_rcvbuf_capped_by_rmem_max_is_a_warning() -> R {
    // The kernel grants twice the request, capped at net.core.rmem_max
    // (here Docker's default, 212992).
    let Some(line) = checks::rcvbuf_line(4 << 20, 2 * 212_992) else {
        return Err("a capped SO_RCVBUF was not reported".into());
    };
    assert!(line.starts_with("ergon-io warning: "), "{line}");
    for part in [
        "4194304",
        "212992",
        "net.core.rmem_max",
        "socket(7)",
        "autotuning",
        "sysctl -w net.core.rmem_max=4194304",
    ] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
    assert_eq!(checks::rcvbuf_line(4 << 20, 8 << 20), None);
    assert_eq!(checks::rcvbuf_line(4 << 20, (8 << 20) + 2), None);
    Ok(())
}

#[test]
fn a_corked_socket_is_named() {
    let line = checks::cork_line(ConnId::default());
    for part in ["TCP_CORK", "200 ms", "tcp(7)", "Driver::add"] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
}

fn flow() -> R<Flow> {
    Ok(Flow::new(
        "10.0.0.2:40000".parse::<SocketAddr>()?,
        "10.0.1.7:443".parse::<SocketAddr>()?,
    ))
}

// Captured on Linux 6.12 (see tests/route.rs, which checks them against
// `ip route get fibmatch`): `default via 172.17.0.1 dev eth0` (seq 1) and
// `local 127.0.0.1 dev lo table local proto kernel scope host src
// 127.0.0.1` (seq 6).
const V4_DEFAULT_PLAIN_REPLY: [u8; 52] = [
    0x34, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00,
    0x0b, 0x00, 0x00, 0x00,
];
const V4_LOCAL_REPLY: [u8; 60] = [
    0x3c, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x4e, 0x00, 0x00, 0x00,
    0x02, 0x20, 0x00, 0x00, 0xff, 0x02, 0xfe, 0x02, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xff, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x07, 0x00,
    0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00,
];

fn remote_route() -> R<route::Route> {
    Ok(route::decode_reply(&V4_DEFAULT_PLAIN_REPLY, 1)?)
}

#[test]
fn nic_polling_that_reaches_no_queue_is_a_warning() -> R {
    let flow = flow()?;
    let remote = remote_route()?;
    assert!(!remote.is_local());
    let id = ConnId::default();
    let line = checks::napi_line(Backend::RecvBusy, id, Some(&flow), &remote, Some(0));
    let Some(line) = line else {
        return Err("a polled connection without a NAPI id was not reported".into());
    };
    assert!(line.starts_with("ergon-io warning: "), "{line}");
    for part in [
        "recv-busy",
        "NAPI id",
        "10.0.1.7:443",
        "interrupts",
        "epoll-spin",
    ] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
    // No latency figure: the only one it quoted came from an Azure run whose
    // NIC interrupt settings were not recorded (io/benches/BENCHMARK.md).
    assert!(!line.contains("µs"), "{line:?}");
    // A NAPI id: polling reaches the NIC.
    assert_eq!(
        checks::napi_line(Backend::RecvBusy, id, Some(&flow), &remote, Some(8193)),
        None
    );
    // Not known (getsockopt failed): nothing claimed.
    assert_eq!(
        checks::napi_line(Backend::RecvBusy, id, Some(&flow), &remote, None),
        None
    );
    // An interrupt-driven backend does not poll.
    assert_eq!(
        checks::napi_line(Backend::EpollSpin, id, Some(&flow), &remote, Some(0)),
        None
    );
    Ok(())
}

#[test]
fn local_routes_have_no_nic_to_poll() -> R {
    let local = route::decode_reply(&V4_LOCAL_REPLY, 6)?;
    assert!(local.is_local());
    assert_eq!(
        checks::napi_line(
            Backend::RecvBusy,
            ConnId::default(),
            Some(&flow()?),
            &local,
            Some(0)
        ),
        None
    );
    Ok(())
}
