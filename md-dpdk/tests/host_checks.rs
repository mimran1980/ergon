//! The media driver's low-latency host checks (the specification's M rows):
//! M1 agents, M2 term buffers, M3 socket buffers, M4 kernel busy polling, the
//! ergon-tune host checks on the agents' CPUs (M-host), the data NICs'
//! (M-nic) and their re-check after a reset (M-run), and how
//! `ConfiguredDriver::from_env` and the bundled binary report them: one line
//! each, and under `ERGON_HOST_CHECKS=require` no start while one warns.
//!
//! Every check reads a host tree (a fixture standing for `/sys` and `/proc`),
//! so they run on every platform; what needs the running kernel (the probe
//! socket, the re-check thread) is tested on Linux, and elsewhere reports
//! unknown.
#![allow(
    missing_docs,
    unsafe_code, // setting environment variables, under #[serial]
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]

use ergon_md_dpdk::DpdkError;
use ergon_md_dpdk::checks::{
    Agents, DataNics, HostCheckPolicy, HostReport, Inputs, Probe, Rechecks, SETTLE_READS, SocketBuffers,
    data_nics_from_env, host_report, housekeeping_cpus, line, m1_agents, m2_term_buffers, m3_socket_buffers,
    m4_busy_polling, probe_socket_buffers,
};
use ergon_md_dpdk::driver::ConfiguredDriver;
use ergon_tune::{
    Coalesce, CpuSet, Ethtool, EthtoolCache, EthtoolError, Finding, Host, HotSet, Rings, Role, Severity, Status,
};
use rusteron_media_driver::AeronDriverContext;
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use serial_test::serial;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEDICATED: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED;
const SHARED_NETWORK: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK;
const SHARED: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_SHARED;
const INVOKER: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER;

// --- Fixtures -----------------------------------------------------------------

/// A host tree under a fresh temporary directory, removed when dropped.
struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn empty(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ergon-md-host-checks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    /// An untuned x86 host: eight CPUs, no SMT, nothing isolated, THP never,
    /// one NUMA node, the calling thread SCHED_OTHER on every CPU, busy
    /// polling off, in a container's own cgroup with no CPU quota.
    fn x86(name: &str) -> Self {
        let f = Self::empty(name);
        f.cpus(8, |c| c.to_string());
        f.file("/proc/cpuinfo", "processor\t: 0\nvendor_id\t: GenuineIntel\n");
        f.file("/sys/class/dmi/id/sys_vendor", "Amazon EC2\n");
        f.file("/sys/class/dmi/id/product_name", "c7i.2xlarge\n");
        f.isolation("", Some(""), "ff");
        f.file(
            "/sys/kernel/mm/transparent_hugepage/enabled",
            "always madvise [never]\n",
        );
        f.file("/sys/devices/system/node/online", "0\n");
        f.file("/sys/devices/system/node/node0/cpulist", "0-7\n");
        f.file("/proc/sys/kernel/numa_balancing", "0\n");
        f.thread(0, "0-7");
        f.busy_poll("0", "0");
        f.file("/proc/self/cgroup", "0::/\n");
        f.file("/sys/fs/cgroup/cpu.max", "max 100000\n");
        f
    }

    /// A Graviton host (64-bit Arm on EC2), otherwise as `x86`.
    fn graviton(name: &str) -> Self {
        let f = Self::x86(name);
        f.file("/proc/cpuinfo", "processor\t: 0\nCPU implementer\t: 0x41\n");
        f.file("/sys/class/dmi/id/product_name", "c9g.2xlarge\n");
        f
    }

    fn host(&self) -> Host {
        Host::at(&self.root)
    }

    fn at(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    fn file(&self, abs: &str, contents: &str) -> &Self {
        let p = self.at(abs);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
        self
    }

    fn remove(&self, abs: &str) -> &Self {
        let _ = std::fs::remove_file(self.at(abs));
        self
    }

    fn link(&self, abs: &str, target: &str) -> &Self {
        let p = self.at(abs);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&p);
        std::os::unix::fs::symlink(target, p).unwrap();
        self
    }

    fn cpus(&self, n: u32, siblings: impl Fn(u32) -> String) -> &Self {
        self.file("/sys/devices/system/cpu/online", &format!("0-{}\n", n - 1));
        for c in 0..n {
            self.file(
                &format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list"),
                &format!("{}\n", siblings(c)),
            );
        }
        self
    }

    fn isolation(&self, isolated: &str, nohz_full: Option<&str>, default_mask: &str) -> &Self {
        self.file("/sys/devices/system/cpu/isolated", &format!("{isolated}\n"));
        match nohz_full {
            Some(n) => self.file("/sys/devices/system/cpu/nohz_full", &format!("{n}\n")),
            None => self.remove("/sys/devices/system/cpu/nohz_full"),
        };
        self.file("/proc/irq/default_smp_affinity", &format!("{default_mask}\n"))
    }

    /// The calling thread's policy (the 41st field of its stat) and CPUs.
    fn thread(&self, policy: u32, cpus_allowed: &str) -> &Self {
        let mut fields: Vec<String> = vec!["S".to_owned()];
        for i in 4..=40 {
            fields.push(if i == 40 { "0".to_owned() } else { i.to_string() });
        }
        fields.push(policy.to_string());
        fields.extend(std::iter::repeat_n("0".to_owned(), 10));
        self.file(
            "/proc/thread-self/stat",
            &format!("4242 (media_driver) {}\n", fields.join(" ")),
        );
        self.file(
            "/proc/thread-self/status",
            &format!("Name:\tmedia_driver\nCpus_allowed_list:\t{cpus_allowed}\n"),
        )
    }

    fn busy_poll(&self, busy_read: &str, busy_poll: &str) -> &Self {
        self.file("/proc/sys/net/core/busy_read", &format!("{busy_read}\n"));
        self.file("/proc/sys/net/core/busy_poll", &format!("{busy_poll}\n"))
    }

    /// A PCI device bound to `driver` on NUMA node 0, with one MSI interrupt
    /// `irq` whose affinity is `affinity`.
    fn pci(&self, bdf: &str, driver: &str, irq: u32, affinity: &str) -> &Self {
        let d = format!("/sys/bus/pci/devices/{bdf}");
        std::fs::create_dir_all(self.at(&format!("/sys/bus/pci/drivers/{driver}"))).unwrap();
        self.link(&format!("{d}/driver"), &format!("../../../../bus/pci/drivers/{driver}"));
        self.link(&format!("{d}/subsystem"), "../../../../bus/pci");
        self.file(&format!("{d}/numa_node"), "0\n");
        self.file(&format!("{d}/irq"), "0\n");
        self.file(&format!("{d}/msi_irqs/{irq}"), "msix\n");
        self.file(&format!("/proc/irq/{irq}/smp_affinity_list"), &format!("{affinity}\n"));
        self.file(&format!("/sys/kernel/irq/{irq}/actions"), &format!("{bdf}-Tx-Rx-0\n"))
    }

    /// A kernel interface on PCI device `bdf`, interrupts not deferred.
    fn netdev(&self, name: &str, ifindex: u32, bdf: &str) -> &Self {
        let d = format!("/sys/class/net/{name}");
        self.file(&format!("{d}/ifindex"), &format!("{ifindex}\n"));
        self.file(&format!("{d}/carrier_changes"), "2\n");
        self.file(&format!("{d}/operstate"), "up\n");
        self.file(&format!("{d}/napi_defer_hard_irqs"), "0\n");
        self.file(&format!("{d}/gro_flush_timeout"), "0\n");
        self.link(&format!("{d}/device"), &format!("../../../bus/pci/devices/{bdf}"))
    }
}

/// Ethtool answers standing in for the kernel's.
#[derive(Default)]
struct Stub {
    adaptive_rx: Option<bool>,
    rings: Option<(u32, u32)>,
}

impl Ethtool for Stub {
    fn coalesce(&mut self, _host: &Host, _ifindex: u32) -> Result<Coalesce, EthtoolError> {
        Ok(Coalesce {
            use_adaptive_rx: self.adaptive_rx,
            rx_usecs: Some(20),
        })
    }

    fn rings(&mut self, _host: &Host, _ifindex: u32) -> Result<Rings, EthtoolError> {
        Ok(self.rings.map_or_else(Rings::default, |(rx, max)| Rings {
            rx: Some(rx),
            rx_max: Some(max),
        }))
    }
}

const fn stub(adaptive_rx: bool, rings: Option<(u32, u32)>) -> EthtoolCache<Stub> {
    EthtoolCache::with_source(Stub {
        adaptive_rx: Some(adaptive_rx),
        rings,
    })
}

/// Agents in `mode` with nothing pinned and every idle strategy Aeron's
/// `backoff` default.
fn agents(mode: aeron_threading_mode_t) -> Agents {
    Agents {
        mode,
        conductor_cpu: None,
        sender_cpu: None,
        receiver_cpu: None,
        conductor_idle: "backoff".into(),
        sender_idle: "backoff".into(),
        receiver_idle: "backoff".into(),
        shared_network_idle: "backoff".into(),
        shared_idle: "backoff".into(),
    }
}

/// DEDICATED, the conductor on 0, the sender on 2 and the receiver on 3, both
/// spinning (`noop`).
fn tuned() -> Agents {
    Agents {
        conductor_cpu: Some(0),
        sender_cpu: Some(2),
        receiver_cpu: Some(3),
        sender_idle: "noop".into(),
        receiver_idle: "noop".into(),
        ..agents(DEDICATED)
    }
}

/// The socket transport's inputs for `agents`: non-sparse term buffers,
/// socket buffers granted, no data NIC named.
fn socket(agents: Agents) -> Inputs {
    Inputs {
        agents,
        dpdk: false,
        eal_main_cpu: None,
        term_buffer_sparse: false,
        buffers: SocketBuffers {
            rcvbuf: Probe::NotSet,
            sndbuf: Probe::NotSet,
        },
        aeron_dir: PathBuf::from("/dev/shm/aeron"),
        nics: DataNics::Socket(Vec::new()),
    }
}

fn with_id<'a>(findings: &'a [Finding], id: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.id == id).collect()
}

fn warnings<'a>(findings: &'a [Finding], id: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.id == id && f.is_warning()).collect()
}

/// Sets an environment variable for the guard's life (tests are `#[serial]`).
struct EnvGuard {
    key: &'static str,
    old: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let old = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, old }
    }

    fn unset(key: &'static str) -> Self {
        let old = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(v) => unsafe { std::env::set_var(self.key, v) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

// --- The environment ----------------------------------------------------------

/// `ERGON_HOST_CHECKS` is the one switch: `warn`/`on` when unset, `off`,
/// `warn`/`on` or `require` (any case); anything else fails start, naming
/// it.
#[test]
#[serial]
fn the_host_check_policy_is_read_from_the_environment() {
    let _a = EnvGuard::unset("RUSTERON_MEDIA_DRIVER_HOST_CHECK");
    {
        let _p = EnvGuard::unset("ERGON_HOST_CHECKS");
        assert_eq!(HostCheckPolicy::from_env().unwrap(), HostCheckPolicy::Warn);
    }
    for (value, want) in [
        ("off", HostCheckPolicy::Off),
        ("warn", HostCheckPolicy::Warn),
        ("on", HostCheckPolicy::Warn),
        ("Require", HostCheckPolicy::Require),
    ] {
        let _p = EnvGuard::set("ERGON_HOST_CHECKS", value);
        assert_eq!(HostCheckPolicy::from_env().unwrap(), want, "{value}");
    }
    for bad in ["", "strict", "no"] {
        let _p = EnvGuard::set("ERGON_HOST_CHECKS", bad);
        let e = HostCheckPolicy::from_env().unwrap_err();
        assert!(matches!(e, DpdkError::InvalidEnvironment(_)), "{bad:?}: {e:?}");
        assert!(e.to_string().contains("ERGON_HOST_CHECKS"), "{e}");
    }
}

/// `RUSTERON_MEDIA_DRIVER_HOST_CHECK`, the old alias, is no switch: set
/// alone, whatever its value, the policy is the default `warn`, and it never
/// overrides `ERGON_HOST_CHECKS`.
#[test]
#[serial]
fn the_old_alias_is_ignored() {
    {
        let _p = EnvGuard::unset("ERGON_HOST_CHECKS");
        for value in ["require", "off", "strict"] {
            let _a = EnvGuard::set("RUSTERON_MEDIA_DRIVER_HOST_CHECK", value);
            assert_eq!(HostCheckPolicy::from_env().unwrap(), HostCheckPolicy::Warn, "{value}");
        }
    }
    let _a = EnvGuard::set("RUSTERON_MEDIA_DRIVER_HOST_CHECK", "require");
    let _p = EnvGuard::set("ERGON_HOST_CHECKS", "off");
    assert_eq!(HostCheckPolicy::from_env().unwrap(), HostCheckPolicy::Off);
}

/// `RUSTERON_MEDIA_DRIVER_DATA_NICS` names the socket transport's data NICs,
/// separated by commas or spaces; a name the kernel could not have made fails
/// start, naming the variable.
#[test]
#[serial]
fn the_data_nics_are_read_from_the_environment() {
    let _unset = EnvGuard::unset("RUSTERON_MEDIA_DRIVER_DATA_NICS");
    assert!(data_nics_from_env().unwrap().is_empty());
    for (value, want) in [
        ("", vec![]),
        ("ens6", vec!["ens6"]),
        ("ens6,ens7", vec!["ens6", "ens7"]),
        (" eth1, eth2 eth3 ,", vec!["eth1", "eth2", "eth3"]),
        ("eth1,eth1 eth2", vec!["eth1", "eth2"]),
        ("a-name-15-bytes", vec!["a-name-15-bytes"]),
    ] {
        let _v = EnvGuard::set("RUSTERON_MEDIA_DRIVER_DATA_NICS", value);
        assert_eq!(data_nics_from_env().unwrap(), want, "{value:?}");
    }
    // IFNAMSIZ is 16 with the NUL: 15 bytes is the longest name
    for bad in ["../etc", "eth1:0", ".", "a-name-of-16byte"] {
        let _v = EnvGuard::set("RUSTERON_MEDIA_DRIVER_DATA_NICS", bad);
        let e = data_nics_from_env().unwrap_err();
        assert!(matches!(e, DpdkError::InvalidEnvironment(_)), "{bad:?}: {e:?}");
        assert!(e.to_string().contains("RUSTERON_MEDIA_DRIVER_DATA_NICS"), "{e}");
    }
}

/// The agents' layout comes from the context, the idle strategies as they
/// run: Aeron loads the strategy an `AERON_*_IDLE_STRATEGY` names but leaves
/// the stored name at `backoff`, so the variable is read, with or without
/// the `dpdk` feature.
#[test]
#[serial]
fn the_agents_layout_reads_idle_strategies_from_the_environment() {
    let _cpu = EnvGuard::set("AERON_SENDER_CPU_AFFINITY", "3");
    let _idle = EnvGuard::set("AERON_SENDER_IDLE_STRATEGY", "noop");
    let _shared = EnvGuard::set("AERON_SHAREDNETWORK_IDLE_STRATEGY", "spin");
    let _receiver = EnvGuard::unset("AERON_RECEIVER_IDLE_STRATEGY");
    let context = AeronDriverContext::new().unwrap();
    assert_eq!(context.get_sender_idle_strategy(), "backoff", "Aeron's stored name");
    let a = Agents::from_context(&context);
    assert_eq!(a.mode, DEDICATED);
    assert_eq!((a.sender_cpu, a.receiver_cpu, a.conductor_cpu), (Some(3), None, None));
    assert_eq!(a.sender_idle, "noop");
    assert_eq!(a.shared_network_idle, "spin");
    assert_eq!(a.receiver_idle, "backoff");
}

// --- Every driver --------------------------------------------------------------------

/// Every check runs for every driver, at its own severity: a driver left at
/// Aeron's defaults (agents floating, idling with `backoff`, sparse term
/// buffers) gets M1 for each network agent of its mode and M2, as warnings,
/// and no note excusing them. `ERGON_HOST_CHECKS=off` is the one switch.
#[test]
fn every_driver_gets_m1_and_m2_at_warning_severity() {
    let f = Fixture::x86("every-driver");
    for (mode, agents_warned) in [(DEDICATED, 2), (SHARED_NETWORK, 1), (SHARED, 1), (INVOKER, 1)] {
        let mut inputs = socket(agents(mode));
        inputs.term_buffer_sparse = true;
        let report = host_report(&f.host(), &inputs, &mut stub(false, None));
        assert_eq!(
            warnings(&report.findings, "M1").len(),
            agents_warned,
            "{mode:?}: {:?}",
            report.findings
        );
        assert_eq!(warnings(&report.findings, "M2").len(), 1, "{mode:?}");
        assert!(
            !report.notes.iter().any(|n| n.contains("latency")),
            "{mode:?}: {:?}",
            report.notes
        );
    }
    // fine where the driver is tuned
    let report = host_report(&f.host(), &socket(tuned()), &mut stub(false, None));
    assert!(warnings(&report.findings, "M1").is_empty(), "{:?}", report.findings);
    assert!(warnings(&report.findings, "M2").is_empty(), "{:?}", report.findings);
}

// --- M1 ---------------------------------------------------------------------------

/// A pinned agent that idles with a strategy that parks: one warning naming
/// the agent, its CPU and strategy, the measured cost (p50, and that no max
/// was recorded) and its variable's fix.
#[test]
fn m1_warns_for_a_pinned_agent_that_idles_with_backoff() {
    let a = Agents {
        sender_idle: "backoff".into(),
        ..tuned()
    };
    let m1 = m1_agents(&a);
    let w: Vec<&Finding> = m1.iter().filter(|f| f.is_warning()).collect();
    assert_eq!(w.len(), 1, "{m1:?}");
    let text = &w[0].message;
    for needle in [
        "sender",
        "CPU 2",
        "backoff",
        "timer slack, 50 µs by default",
        "not measured here",
        "AERON_SENDER_IDLE_STRATEGY=noop",
    ] {
        assert!(text.contains(needle), "{needle:?} in {text}");
    }
    assert!(!text.contains("receiver"), "{text}");
    assert_eq!((w[0].id, w[0].severity), ("M1", Severity::Warn));
}

/// An agent that busy-spins but floats: a warning naming its CPU variable
/// and the cost of agents sharing a core.
#[test]
fn m1_warns_for_a_busy_spinning_agent_that_floats() {
    let a = Agents {
        receiver_cpu: None,
        ..tuned()
    };
    let w: Vec<Finding> = m1_agents(&a).into_iter().filter(Finding::is_warning).collect();
    assert_eq!(w.len(), 1, "{w:?}");
    for needle in [
        "receiver",
        "AERON_RECEIVER_CPU_AFFINITY",
        "time slice",
        "not measured here",
    ] {
        assert!(w[0].message.contains(needle), "{needle:?} in {}", w[0].message);
    }
    // neither pinned nor spinning, while the other is set up: both fixes
    let a = Agents {
        receiver_cpu: None,
        receiver_idle: "backoff".into(),
        ..tuned()
    };
    let w: Vec<Finding> = m1_agents(&a).into_iter().filter(Finding::is_warning).collect();
    assert_eq!(w.len(), 1, "{w:?}");
    for needle in ["AERON_RECEIVER_CPU_AFFINITY", "AERON_RECEIVER_IDLE_STRATEGY=noop"] {
        assert!(w[0].message.contains(needle), "{needle:?} in {}", w[0].message);
    }
}

/// An agent on the conductor's CPU shares it with the conductor's work:
/// a warning under DEDICATED and SHARED_NETWORK; under SHARED and INVOKER
/// that CPU is the one agent's own.
#[test]
fn m1_warns_for_an_agent_on_the_conductors_cpu() {
    for (a, agent) in [
        (
            Agents {
                conductor_cpu: Some(2),
                ..tuned()
            },
            "sender",
        ),
        (
            Agents {
                conductor_cpu: Some(3),
                ..tuned()
            },
            "receiver",
        ),
        (
            Agents {
                conductor_cpu: Some(4),
                sender_cpu: Some(4),
                shared_network_idle: "noop".into(),
                ..agents(SHARED_NETWORK)
            },
            "network agent",
        ),
    ] {
        let w: Vec<Finding> = m1_agents(&a).into_iter().filter(Finding::is_warning).collect();
        assert_eq!(w.len(), 1, "{agent}: {w:?}");
        for needle in [agent, "conductor", "AERON_CONDUCTOR_CPU_AFFINITY", "time slice"] {
            assert!(w[0].message.contains(needle), "{needle:?} in {}", w[0].message);
        }
    }
    for mode in [SHARED, INVOKER] {
        let a = Agents {
            conductor_cpu: Some(4),
            shared_idle: "noop".into(),
            ..agents(mode)
        };
        let m1 = m1_agents(&a);
        assert!(m1.iter().all(|f| !f.is_finding()), "{mode:?}: {m1:?}");
    }
}

/// Agents busy-spinning on pinned CPUs of their own: one ok, no warning.
#[test]
fn m1_is_ok_for_agents_spinning_on_pinned_cpus_of_their_own() {
    let m1 = m1_agents(&tuned());
    assert_eq!(m1.len(), 1, "{m1:?}");
    assert_eq!((m1[0].id, &m1[0].status), ("M1", &Status::Ok));
    let a = Agents {
        conductor_cpu: None,
        ..tuned()
    };
    assert!(
        m1_agents(&a).iter().all(|f| !f.is_finding()),
        "a floating conductor is housekeeping"
    );
}

/// The shared modes check their one network agent, by its own variables.
#[test]
fn m1_checks_the_shared_modes_network_agent() {
    for (a, needles) in [
        (
            Agents {
                sender_cpu: Some(5),
                ..agents(SHARED_NETWORK)
            },
            ["network agent", "CPU 5", "AERON_SHAREDNETWORK_IDLE_STRATEGY=noop"],
        ),
        (
            Agents {
                conductor_cpu: Some(5),
                ..agents(SHARED)
            },
            ["shared agent", "CPU 5", "AERON_SHARED_IDLE_STRATEGY=noop"],
        ),
        (
            Agents {
                conductor_cpu: Some(5),
                ..agents(INVOKER)
            },
            ["application's thread", "CPU 5", "AERON_SHARED_IDLE_STRATEGY=noop"],
        ),
        (
            Agents {
                shared_idle: "noop".into(),
                ..agents(SHARED)
            },
            ["shared agent", "AERON_CONDUCTOR_CPU_AFFINITY", "time slice"],
        ),
    ] {
        let w: Vec<Finding> = m1_agents(&a).into_iter().filter(Finding::is_warning).collect();
        assert_eq!(w.len(), 1, "{:?}: {w:?}", a.mode);
        for needle in needles {
            assert!(w[0].message.contains(needle), "{needle:?} in {}", w[0].message);
        }
    }
}

/// Under DPDK the transport's install enforces M1 (it refuses to start), so
/// the report has no M1 warning of its own, whatever the layout.
#[test]
fn m1_under_dpdk_is_the_installs_not_a_warning() {
    let f = Fixture::x86("m1-dpdk");
    let mut inputs = socket(Agents {
        sender_idle: "backoff".into(),
        ..tuned()
    });
    inputs.dpdk = true;
    inputs.nics = DataNics::Dpdk {
        ports: vec![],
        vdev: false,
        named: vec![],
    };
    let report = host_report(&f.host(), &inputs, &mut stub(false, None));
    assert!(warnings(&report.findings, "M1").is_empty(), "{:?}", report.findings);
}

// --- M2 ---------------------------------------------------------------------------

/// Sparse term buffers: a warning with the fix, under either transport;
/// non-sparse: ok.
#[test]
fn m2_warns_for_sparse_term_buffers() {
    let m2 = m2_term_buffers(true);
    assert!(m2.is_warning(), "{m2:?}");
    assert_eq!(m2.id, "M2");
    for needle in ["sparse", "AERON_TERM_BUFFER_SPARSE_FILE=false", "not A/B'd"] {
        assert!(
            m2.message.contains(needle) || m2.evidence.contains(needle),
            "{needle:?} in {m2:?}"
        );
    }
    assert_eq!(m2_term_buffers(false).status, Status::Ok);

    let f = Fixture::x86("m2-dpdk");
    let mut inputs = socket(tuned());
    inputs.term_buffer_sparse = true;
    inputs.dpdk = true;
    inputs.nics = DataNics::Dpdk {
        ports: vec![],
        vdev: false,
        named: vec![],
    };
    let report = host_report(&f.host(), &inputs, &mut stub(false, None));
    assert_eq!(warnings(&report.findings, "M2").len(), 1, "{:?}", report.findings);
}

// --- M3 ---------------------------------------------------------------------------

/// A socket buffer the kernel cut (getsockopt below twice the request: the
/// kernel keeps double what it grants) warns under the socket transport and
/// informs under DPDK (host-local channels only); granted in full, or not
/// asked for, is ok; an unreadable probe is unknown.
#[test]
fn m3_flags_a_socket_buffer_the_kernel_cut() {
    let mib = 1 << 20;
    let cut = SocketBuffers {
        rcvbuf: Probe::Granted {
            asked: 2 * mib,
            granted: 3 * mib,
        },
        sndbuf: Probe::NotSet,
    };
    let m3 = m3_socket_buffers(&cut, false);
    let w: Vec<&Finding> = m3.iter().filter(|f| f.is_warning()).collect();
    assert_eq!(w.len(), 1, "{m3:?}");
    for needle in [
        "AERON_SOCKET_SO_RCVBUF",
        "2097152",
        "1572864",
        "net.core.rmem_max",
        "below half",
    ] {
        assert!(w[0].message.contains(needle), "{needle:?} in {}", w[0].message);
    }
    let dpdk = m3_socket_buffers(&cut, true);
    let found: Vec<&Finding> = dpdk.iter().filter(|f| f.is_finding()).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].severity,
        Severity::Info,
        "under DPDK only host-local channels use sockets"
    );

    // the boundary: exactly twice the request is granted in full, one byte less is cut
    for (granted, cut) in [(4 * mib, false), (4 * mib - 1, true)] {
        let b = SocketBuffers {
            rcvbuf: Probe::NotSet,
            sndbuf: Probe::Granted {
                asked: 2 * mib,
                granted,
            },
        };
        let w: Vec<Finding> = m3_socket_buffers(&b, false)
            .into_iter()
            .filter(Finding::is_warning)
            .collect();
        assert_eq!(w.len(), usize::from(cut), "granted {granted}: {w:?}");
        if cut {
            assert!(w[0].message.contains("AERON_SOCKET_SO_SNDBUF") && w[0].message.contains("net.core.wmem_max"));
        }
    }
    let unread = SocketBuffers {
        rcvbuf: Probe::Unknown("not Linux".into()),
        sndbuf: Probe::NotSet,
    };
    let m3 = m3_socket_buffers(&unread, false);
    assert!(
        m3.iter().any(Finding::is_unknown) && !m3.iter().any(Finding::is_finding),
        "{m3:?}"
    );
}

/// The probe asks the running kernel: on Linux it is granted a small buffer
/// in full (getsockopt reports double); elsewhere it is unknown.
#[test]
fn m3_probes_the_running_kernel() {
    let b = probe_socket_buffers(65536, 0);
    assert_eq!(b.sndbuf, Probe::NotSet);
    if cfg!(target_os = "linux") {
        match b.rcvbuf {
            Probe::Granted { asked, granted } => {
                assert_eq!(asked, 65536);
                assert!(granted >= 2 * 65536, "granted {granted}");
            }
            other => panic!("{other:?}"),
        }
    } else {
        assert!(matches!(b.rcvbuf, Probe::Unknown(_)), "{:?}", b.rcvbuf);
    }
}

// --- M4 ---------------------------------------------------------------------------

/// Kernel busy polling is reported, never warned about or recommended:
/// md-dpdk/BENCHMARKS.md lists it under "Measured, but no setting
/// recommended" (G1 and round 7 each change several settings at once, so
/// polling itself is not isolated). On or off, x86 or Graviton: information
/// with the measured rows and their maxima; unreadable (a pod), unknown at
/// information severity; not run under DPDK.
#[test]
fn m4_reports_busy_polling_as_information_and_recommends_no_setting() {
    let x86 = Fixture::x86("m4-x86");
    let arm = Fixture::graviton("m4-graviton");
    for fixture in [&x86, &arm] {
        let m4 = m4_busy_polling(&fixture.host());
        assert_eq!(
            (m4.id, &m4.status, m4.severity),
            ("M4", &Status::Finding, Severity::Info),
            "{m4:?}"
        );
        assert!(m4.message.contains("kernel busy polling is off"), "{}", m4.message);
    }
    for (read, poll) in [("50", "0"), ("0", "50")] {
        x86.busy_poll(read, poll);
        let m4 = m4_busy_polling(&x86.host());
        assert_eq!(
            (&m4.status, m4.severity),
            (&Status::Finding, Severity::Info),
            "{read} {poll}"
        );
        assert!(
            m4.message.contains(&format!(
                "kernel busy polling is on (net.core.busy_read {read}, busy_poll {poll})"
            )),
            "{}",
            m4.message
        );
    }
    let m4 = m4_busy_polling(&arm.host());
    // the two committed rows, each latency figure with its max
    // (BENCHMARKS.md, Measured, but no setting recommended)
    for needle in [
        "no setting is recommended",
        "5 of 5 reps",
        "max 9,175 (8,544–102,826) µs",
        "four settings",
        "max 382 (372–389) against 469 (448–578) µs",
        "two settings",
    ] {
        assert!(m4.message.contains(needle), "{needle:?} in {}", m4.message);
    }
    // nothing tells the operator to turn it on (or off)
    for absent in [
        "sysctl -w",
        "busy_read=50",
        "napi_defer_hard_irqs",
        "gro_flush_timeout",
        "Fix:",
        "dpdk-ena",
        "Azure",
    ] {
        assert!(!m4.message.contains(absent), "{absent:?} in {}", m4.message);
    }
    assert!(
        m4.evidence.contains("Measured, but no setting recommended"),
        "{}",
        m4.evidence
    );

    x86.remove("/proc/sys/net/core/busy_read");
    let unknown = m4_busy_polling(&x86.host());
    assert!(
        unknown.is_unknown() && unknown.severity == Severity::Info,
        "{unknown:?}"
    );

    let f = Fixture::x86("m4-dpdk");
    let mut inputs = socket(tuned());
    inputs.dpdk = true;
    inputs.nics = DataNics::Dpdk {
        ports: vec![],
        vdev: false,
        named: vec![],
    };
    let report = host_report(&f.host(), &inputs, &mut stub(false, None));
    assert!(with_id(&report.findings, "M4").is_empty(), "{:?}", report.findings);
}

// --- M-host -------------------------------------------------------------------------

/// The hot CPUs: the network agents' (spinning roles), the conductor's and the
/// EAL main lcore's (the component's other threads, for T8).
#[test]
fn the_hot_cpus_are_the_agents_the_conductor_and_the_eal_main_lcore() {
    let hot = tuned().hot(Some(1));
    let mut roles: Vec<(u32, Role)> = hot.iter().map(|h| (h.cpu, h.role)).collect();
    roles.sort_by_key(|(c, _)| *c);
    assert_eq!(
        roles,
        vec![
            (0, Role::Conductor),
            (1, Role::EalLcore),
            (2, Role::Sender),
            (3, Role::Receiver)
        ]
    );
    let sn = Agents {
        conductor_cpu: Some(0),
        sender_cpu: Some(4),
        receiver_cpu: Some(5),
        ..agents(SHARED_NETWORK)
    };
    let roles: Vec<(u32, Role)> = sn.hot(None).iter().map(|h| (h.cpu, h.role)).collect();
    assert_eq!(
        roles,
        vec![(4, Role::SharedAgent), (0, Role::Conductor)],
        "no receiver agent"
    );
    for mode in [SHARED, INVOKER] {
        let s = Agents {
            conductor_cpu: Some(6),
            sender_cpu: Some(4),
            ..agents(mode)
        };
        let roles: Vec<(u32, Role)> = s.hot(None).iter().map(|h| (h.cpu, h.role)).collect();
        assert_eq!(roles, vec![(6, Role::SharedAgent)], "{mode:?}");
    }
    assert!(agents(DEDICATED).hot(None).iter().next().is_none(), "nothing pinned");
}

/// T1, T8, T7, T3, T4, T5 and T9 run against the agents' CPUs, the driver's
/// directory and its cgroup: here the receiver's CPU is not nohz_full and
/// the conductor is the sender's SMT sibling.
#[test]
fn the_host_checks_run_on_the_agents_cpus() {
    let f = Fixture::x86("m-host");
    f.cpus(8, |c| format!("{},{}", c % 4, c % 4 + 4));
    f.isolation("1-3,5-7", Some("1-2,5-7"), "11");
    let a = Agents {
        conductor_cpu: Some(6),
        ..tuned()
    };
    let report = host_report(&f.host(), &socket(a), &mut stub(false, None));
    let t1 = warnings(&report.findings, "T1");
    assert_eq!(t1.len(), 1, "{t1:?}");
    assert!(t1[0].message.contains("CPU 3 (receiver)") && t1[0].message.contains("nohz_full"));
    let t8 = warnings(&report.findings, "T8");
    assert_eq!(t8.len(), 1, "{t8:?}");
    assert!(t8[0].message.contains("CPU 2 (sender)") && t8[0].message.contains("CPU 6 (conductor)"));
    for id in ["T3", "T4", "T5", "T7", "T9", "T13", "T14", "T15", "T16", "T17", "T18"] {
        assert_eq!(with_id(&report.findings, id).len(), 1, "{id}: {:?}", report.findings);
    }
    // T4 reads the driver's own directory (on a fixture: unknown, never a warning)
    let t4 = with_id(&report.findings, "T4");
    assert!(t4[0].message.contains("/dev/shm/aeron"), "{}", t4[0].message);
    // No CPU quota on the driver's cgroup.
    assert_eq!(with_id(&report.findings, "T9")[0].status, Status::Ok);
}

/// A CPU quota on the driver's cgroup (a pod with a CPU limit and no
/// exclusive cores) is a warning under either transport: the busy-spinning
/// agents run at the cap and the kernel stops the whole driver when it is
/// used up. Under `require` it refuses the start, naming T9.
#[test]
fn a_cpu_quota_on_the_drivers_cgroup_is_a_warning() {
    let f = Fixture::x86("t9");
    f.file("/sys/fs/cgroup/cpu.max", "300000 100000\n");
    for inputs in [
        socket(tuned()),
        Inputs {
            dpdk: true,
            nics: DataNics::Dpdk {
                ports: vec![],
                vdev: true,
                named: vec![],
            },
            ..socket(tuned())
        },
    ] {
        let report = host_report(&f.host(), &inputs, &mut stub(false, None));
        let t9 = warnings(&report.findings, "T9");
        assert_eq!(t9.len(), 1, "{:?}", report.findings);
        assert!(
            t9[0].message.contains("/sys/fs/cgroup/cpu.max is `300000 100000`")
                && t9[0].message.contains("Never set kubelet `cpuCFSQuota: false`"),
            "{}",
            t9[0].message
        );
        assert!(
            report
                .lines()
                .iter()
                .any(|l| l.starts_with("ergon-md-dpdk warning: T9 warn: ")),
            "{:?}",
            report.lines()
        );
        let refused = report.require().unwrap_err().to_string();
        assert!(refused.contains("T9"), "{refused}");
    }
}

// --- M-nic ----------------------------------------------------------------------------

/// The socket transport without RUSTERON_MEDIA_DRIVER_DATA_NICS: one
/// information line naming the variable, and no NIC check.
#[test]
fn unnamed_data_nics_give_one_info_line() {
    let f = Fixture::x86("nics-unnamed");
    let report = host_report(&f.host(), &socket(tuned()), &mut stub(true, None));
    let notes: Vec<&String> = report
        .notes
        .iter()
        .filter(|n| n.contains("RUSTERON_MEDIA_DRIVER_DATA_NICS"))
        .collect();
    assert_eq!(notes.len(), 1, "{:?}", report.notes);
    for id in ["T2", "T6", "N1", "N2", "N4"] {
        assert!(with_id(&report.findings, id).is_empty(), "{id}: {:?}", report.findings);
    }
}

/// Each named data NIC gets T2, T6, N1 and N2, and N4 when busy polling is on.
#[test]
fn each_named_data_nic_is_checked() {
    let f = Fixture::x86("nics-named");
    f.pci("0000:00:06.0", "ena", 40, "0-7");
    f.netdev("ens6", 3, "0000:00:06.0");
    f.file("/sys/class/net/ens6/napi_defer_hard_irqs", "2\n");
    f.file("/sys/class/net/ens6/gro_flush_timeout", "200000\n");
    let mut inputs = socket(tuned());
    inputs.nics = DataNics::Socket(vec!["ens6".into()]);
    let report = host_report(&f.host(), &inputs, &mut stub(true, Some((1024, 8192))));
    for id in ["T2", "N1", "N2"] {
        let w = warnings(&report.findings, id);
        assert_eq!(w.len(), 1, "{id}: {:?}", report.findings);
    }
    assert!(warnings(&report.findings, "T2")[0].message.contains("CPU 2 (sender)"));
    assert_eq!(with_id(&report.findings, "T6").len(), 1);
    assert!(with_id(&report.findings, "N4").is_empty(), "no busy polling, no N4");
    assert!(
        !report
            .notes
            .iter()
            .any(|n| n.contains("RUSTERON_MEDIA_DRIVER_DATA_NICS"))
    );

    // busy polling on: N2's deferral is how that is set up, N4 checks the ring
    f.busy_poll("50", "50");
    let report = host_report(&f.host(), &inputs, &mut stub(true, Some((1024, 8192))));
    assert!(warnings(&report.findings, "N2").is_empty(), "{:?}", report.findings);
    let n4 = warnings(&report.findings, "N4");
    assert_eq!(n4.len(), 1, "{:?}", report.findings);
    assert!(n4[0].message.contains("ethtool -G ens6 rx 4096"), "{}", n4[0].message);
}

/// Where the busy-polling sysctls cannot be read (a pod's network
/// namespace), whether the sockets busy-poll is unknown: N2 does not warn
/// about a NIC that defers its interrupts (unknown instead), and N4, which
/// applies only with busy polling, is not run.
#[test]
fn a_data_nic_whose_busy_polling_is_unknown_gets_no_n2_warning_and_no_n4() {
    let f = Fixture::x86("nics-pod");
    f.pci("0000:00:06.0", "ena", 40, "0-1");
    f.netdev("ens6", 3, "0000:00:06.0");
    f.file("/sys/class/net/ens6/napi_defer_hard_irqs", "2\n");
    f.file("/sys/class/net/ens6/gro_flush_timeout", "200000\n");
    f.remove("/proc/sys/net/core/busy_read");
    f.remove("/proc/sys/net/core/busy_poll");
    let mut inputs = socket(tuned());
    inputs.nics = DataNics::Socket(vec!["ens6".into()]);
    let report = host_report(&f.host(), &inputs, &mut stub(false, Some((1024, 8192))));
    assert!(with_id(&report.findings, "M4")[0].is_unknown());
    let n2 = with_id(&report.findings, "N2");
    assert!(!n2.is_empty() && n2.iter().all(|f| f.is_unknown()), "{n2:?}");
    assert!(with_id(&report.findings, "N4").is_empty(), "{:?}", report.findings);
}

/// Under DPDK the data NICs are the ports, by PCI address: T2 (interrupts
/// of a port bound to igb_uio) and T6, none of the kernel-path checks; a
/// test vdev is noted and not checked; a named socket NIC is noted as
/// ignored.
#[test]
fn dpdk_ports_are_checked_by_pci_address() {
    let f = Fixture::x86("nics-dpdk");
    f.pci("0000:00:06.0", "igb_uio", 41, "3");
    let mut inputs = socket(tuned());
    inputs.dpdk = true;
    inputs.nics = DataNics::Dpdk {
        ports: vec!["0000:00:06.0".into(), "0000:00:06.0".into()],
        vdev: false,
        named: vec![],
    };
    let report = host_report(&f.host(), &inputs, &mut stub(true, Some((1024, 8192))));
    let t2 = warnings(&report.findings, "T2");
    assert_eq!(t2.len(), 1, "one port, its one IRQ, once: {:?}", report.findings);
    assert!(t2[0].message.contains("0000:00:06.0") && t2[0].message.contains("CPU 3 (receiver)"));
    assert_eq!(with_id(&report.findings, "T6").len(), 1);
    for id in ["N1", "N2", "N4"] {
        assert!(with_id(&report.findings, id).is_empty(), "{id}: {:?}", report.findings);
    }

    inputs.nics = DataNics::Dpdk {
        ports: vec!["net_tap0".into(), "net_tap1".into()],
        vdev: true,
        named: vec!["ens6".into()],
    };
    let report = host_report(&f.host(), &inputs, &mut stub(true, None));
    assert!(with_id(&report.findings, "T2").is_empty(), "{:?}", report.findings);
    assert!(
        report.notes.iter().any(|n| n.contains("virtual devices")),
        "{:?}",
        report.notes
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("RUSTERON_MEDIA_DRIVER_DATA_NICS") && n.contains("ignored")),
        "{:?}",
        report.notes
    );
}

// --- Lines ------------------------------------------------------------------------------

/// One line per finding that is not fine: a warning as `ergon-md-dpdk
/// warning:`, information and what could not be read as `ergon-md-dpdk
/// info:`; the report's lines are its notes, then those.
#[test]
fn findings_are_printed_one_line_each() {
    let w = m2_term_buffers(true);
    let text = line(&w).unwrap();
    assert_eq!(
        text,
        format!("ergon-md-dpdk warning: {w} (ERGON_HOST_CHECKS=off silences host-check warnings)")
    );
    assert!(text.contains("(evidence: "));
    assert_eq!(line(&m2_term_buffers(false)), None);
    let x86 = Fixture::x86("lines");
    x86.remove("/proc/sys/net/core/busy_read");
    let unknown = m4_busy_polling(&x86.host());
    assert!(line(&unknown).unwrap().starts_with("ergon-md-dpdk info: M4 unknown: "));
    let arm = Fixture::graviton("lines-arm");
    assert!(
        line(&m4_busy_polling(&arm.host()))
            .unwrap()
            .starts_with("ergon-md-dpdk info: M4 info: ")
    );

    let report = host_report(&x86.host(), &socket(agents(DEDICATED)), &mut stub(false, None));
    let lines = report.lines();
    assert_eq!(lines[0], report.notes[0]);
    assert_eq!(
        lines.len(),
        report.notes.len() + report.findings.iter().filter_map(line).count()
    );
}

/// `ERGON_HOST_CHECKS=require` refuses on any one warning,
/// naming it, and never on an information line or on what could not be read
/// (a pod's namespace, a system other than Linux).
#[test]
fn require_refuses_on_one_warning_and_never_on_information_or_unknown() {
    let arm = Fixture::graviton("require-arm");
    let x86 = Fixture::x86("require-x86");
    x86.remove("/proc/sys/net/core/busy_read");
    // M3 under the socket transport: unknown at warning severity
    let unread = m3_socket_buffers(
        &SocketBuffers {
            rcvbuf: Probe::Unknown("not Linux".into()),
            sndbuf: Probe::NotSet,
        },
        false,
    );
    let quiet = HostReport {
        hot: HotSet::new(),
        notes: vec!["ergon-md-dpdk info: a note".into()],
        findings: vec![
            m2_term_buffers(false),
            m4_busy_polling(&arm.host()), // information
            m4_busy_polling(&x86.host()), // unknown
            unread[0].clone(),
        ],
    };
    assert_eq!(quiet.findings[1].severity, Severity::Info);
    assert!(quiet.findings[2].is_unknown());
    assert!(quiet.findings[3].is_unknown() && quiet.findings[3].severity == Severity::Warn);
    assert!(quiet.warnings().is_empty(), "{:?}", quiet.findings);
    assert_eq!(quiet.require(), Ok(()));

    let mut one = quiet.clone();
    one.findings.push(m2_term_buffers(true));
    assert_eq!(one.warnings().len(), 1);
    let e = one.require().unwrap_err();
    assert!(matches!(e, DpdkError::HostCheck(_)), "{e:?}");
    let text = e.to_string();
    assert!(
        text.contains("ERGON_HOST_CHECKS=require and 1 host check warning(s) (M2)")
            && text.contains("set ERGON_HOST_CHECKS=warn or off to start anyway")
            && !text.contains("RUSTERON_MEDIA_DRIVER_HOST_CHECK"),
        "{text}"
    );
}

// --- M-run ------------------------------------------------------------------------------

/// A data NIC's re-check: N1 and T2 again once its carrier_changes moved and
/// then held still for SETTLE_READS further reads (an ENA reset re-places its
/// queue interrupts and turns adaptive moderation back on); nothing while it
/// settles, nothing again after. A NIC whose counter cannot be read is not
/// watched.
#[test]
fn a_data_nic_is_rechecked_after_its_carrier_changes_settle() {
    let f = Fixture::x86("recheck");
    f.pci("0000:00:06.0", "ena", 40, "0-1");
    f.netdev("ens6", 3, "0000:00:06.0");
    let hot = tuned().hot(None);
    let mut r = Rechecks::new(
        &f.host(),
        &["ens6".to_owned(), "ens9".to_owned()],
        hot,
        stub(true, None),
    );
    assert_eq!(r.watched(), vec!["ens6".to_owned()], "ens9 has no carrier_changes");
    assert!(r.tick(&f.host()).is_empty(), "nothing moved");

    // a reset: the counter moves, ENA's interrupts land on the sender's CPU
    f.file("/sys/class/net/ens6/carrier_changes", "4\n");
    f.file("/proc/irq/40/smp_affinity_list", "0-7\n");
    for read in 0..SETTLE_READS {
        assert!(r.tick(&f.host()).is_empty(), "read {read}: still settling");
    }
    let due = r.tick(&f.host());
    assert_eq!(due.len(), 1, "{due:?}");
    assert_eq!((due[0].nic.as_str(), due[0].from, due[0].to), ("ens6", 2, 4));
    let ids: Vec<&'static str> = due[0].findings.iter().map(|f| f.id).collect();
    assert_eq!(ids, vec!["T2", "N1"]);
    assert!(due[0].findings.iter().all(Finding::is_warning), "{:?}", due[0].findings);
    for read in 0..=SETTLE_READS {
        assert!(r.tick(&f.host()).is_empty(), "read {read}: checked once a reset");
    }

    // a counter that moves again while settling starts the wait over
    f.file("/sys/class/net/ens6/carrier_changes", "5\n");
    assert!(r.tick(&f.host()).is_empty());
    f.file("/sys/class/net/ens6/carrier_changes", "6\n");
    for _ in 0..SETTLE_READS {
        assert!(r.tick(&f.host()).is_empty());
    }
    assert_eq!(r.tick(&f.host()).len(), 1);
}

/// The re-check thread's CPUs: those the creating thread may use, less every
/// busy-spinning agent's core (SMT siblings included); the conductor's alone
/// when it is pinned there and does not spin; none when nothing is left.
#[test]
fn the_recheck_thread_keeps_off_the_spinning_agents_cores() {
    let f = Fixture::x86("hk");
    f.cpus(8, |c| format!("{},{}", c % 4, c % 4 + 4));
    let a = Agents {
        conductor_cpu: None,
        ..tuned()
    };
    let hk = housekeeping_cpus(&f.host(), &a);
    assert_eq!(hk, Some(CpuSet::of([0, 1, 4, 5])), "less 2, 3 and their siblings 6, 7");
    let a = Agents {
        conductor_cpu: Some(1),
        ..tuned()
    };
    assert_eq!(housekeeping_cpus(&f.host(), &a), Some(CpuSet::of([1])));
    f.thread(0, "2,6");
    assert_eq!(housekeeping_cpus(&f.host(), &a), None, "only the sender's core");
}

// --- ConfiguredDriver and the binary ------------------------------------------------------

/// `from_env` runs the checks after pinning and before start, keeps the
/// report, and under `require` refuses while one warns (here M1: a sender
/// pinned but idling with backoff), naming the warnings; `off` runs none.
#[test]
#[serial]
fn from_env_reports_and_require_refuses_a_warning() {
    let _transport = EnvGuard::unset("RUSTERON_MEDIA_DRIVER_TRANSPORT");
    let _nics = EnvGuard::unset("RUSTERON_MEDIA_DRIVER_DATA_NICS");
    let _cpu = EnvGuard::set("AERON_SENDER_CPU_AFFINITY", "0");
    let _idle = EnvGuard::unset("AERON_SENDER_IDLE_STRATEGY");

    // The old alias alone changes nothing: the checks run and warn, and the
    // driver may start.
    let _policy = EnvGuard::unset("ERGON_HOST_CHECKS");
    let _alias = EnvGuard::set("RUSTERON_MEDIA_DRIVER_HOST_CHECK", "require");
    let configured = ConfiguredDriver::from_env(AeronDriverContext::new().unwrap())
        .expect("RUSTERON_MEDIA_DRIVER_HOST_CHECK=require alone is ignored");
    let report = configured.host_report().expect("the checks ran");
    assert!(!warnings(&report.findings, "M1").is_empty());
    drop(configured);
    let _alias = EnvGuard::unset("RUSTERON_MEDIA_DRIVER_HOST_CHECK");

    let _policy = EnvGuard::set("ERGON_HOST_CHECKS", "warn");
    let configured = ConfiguredDriver::from_env(AeronDriverContext::new().unwrap()).unwrap();
    let report = configured.host_report().expect("the checks ran");
    let m1 = warnings(&report.findings, "M1");
    assert!(
        m1.iter()
            .any(|f| f.message.contains("sender is pinned to CPU 0 but idles with `backoff`")),
        "{m1:?}"
    );
    assert!(!configured.rechecking(), "no data NIC named");
    drop(configured);

    let _policy = EnvGuard::set("ERGON_HOST_CHECKS", "require");
    let e = ConfiguredDriver::from_env(AeronDriverContext::new().unwrap())
        .err()
        .expect("require refuses a warning");
    assert!(matches!(e, DpdkError::HostCheck(_)), "{e:?}");
    let text = e.to_string();
    assert!(
        text.contains("M1") && text.contains("ERGON_HOST_CHECKS=require"),
        "{text}"
    );

    let _policy = EnvGuard::set("ERGON_HOST_CHECKS", "off");
    let configured = ConfiguredDriver::from_env(AeronDriverContext::new().unwrap()).unwrap();
    assert!(configured.host_report().is_none(), "off runs no check");

    let _policy = EnvGuard::set("ERGON_HOST_CHECKS", "loud");
    assert!(matches!(
        ConfiguredDriver::from_env(AeronDriverContext::new().unwrap()),
        Err(DpdkError::InvalidEnvironment(_))
    ));
}

/// The bundled binary prints each line on stderr, and under `require` exits
/// non-zero before starting a driver.
#[test]
#[serial]
fn the_binary_prints_the_lines_and_require_exits_before_starting() {
    let dir = std::env::temp_dir().join(format!("aeron-host-checks-{}", std::process::id()));
    let dir = dir.to_str().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_media_driver"))
        .env("AERON_DIR", dir)
        .env("AERON_DIR_DELETE_ON_START", "true")
        .env("AERON_DIR_DELETE_ON_SHUTDOWN", "true")
        .env("AERON_SENDER_CPU_AFFINITY", "0")
        .env_remove("AERON_SENDER_IDLE_STRATEGY")
        .env_remove("RUSTERON_MEDIA_DRIVER_TRANSPORT")
        .env("ERGON_HOST_CHECKS", "require")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_string(&mut text);
            }
            text
        })
    };
    let out_pipe = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let err_pipe = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    // A driver that started anyway runs until stopped: stop it, and fail.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let stdout = out_pipe.join().unwrap();
    let stderr = err_pipe.join().unwrap();
    let status = status.unwrap_or_else(|| panic!("require started a driver that kept running: {stdout}\n{stderr}"));
    assert_ne!(status.code(), Some(0), "require started a driver: {stdout}\n{stderr}");
    assert!(stderr.contains("ergon-md-dpdk warning: M1 warn: "), "{stderr}");
    assert!(stderr.contains("HostCheck"), "{stderr}");
    assert!(!stdout.contains("media driver started"), "{stdout}");
    assert!(!Path::new(dir).join("cnc.dat").exists(), "no driver created");
}

/// The bundled binary's stderr and exit status for `envs`, run until it
/// exits (within 30 s; one still running then is killed and reported).
fn run_binary(envs: &[(&str, &str)]) -> (Option<std::process::ExitStatus>, String, String) {
    let dir = std::env::temp_dir().join(format!("aeron-host-checks-off-{}", std::process::id()));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_media_driver"));
    cmd.env("AERON_DIR", dir.to_str().unwrap())
        .env("AERON_DIR_DELETE_ON_START", "true")
        .env("AERON_DIR_DELETE_ON_SHUTDOWN", "true")
        .env_remove("RUSTERON_MEDIA_DRIVER_TRANSPORT")
        .env_remove("RUSTERON_MEDIA_DRIVER_DATA_NICS")
        .env_remove("ERGON_HOST_CHECKS")
        .env_remove("RUSTERON_MEDIA_DRIVER_HOST_CHECK")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut out, &mut text);
        text
    });
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut err, &mut text);
        text
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    (status, out.join().unwrap(), err.join().unwrap())
}

/// `ERGON_HOST_CHECKS=off` runs no check and says so in one information
/// line, whatever the driver (here one at Aeron's defaults, which `warn`
/// would warn about); a value other than off, on, warn or require refuses to
/// start, naming the variable. The old alias `RUSTERON_MEDIA_DRIVER_HOST_CHECK`
/// is ignored, whatever it holds. (`RUSTERON_MEDIA_DRIVER_MLOCKALL` set wrong
/// stops the driver after the checks, before it is created.)
#[test]
#[serial]
fn the_binary_says_once_that_the_checks_are_off_and_refuses_an_unknown_policy() {
    let (status, stdout, stderr) = run_binary(&[
        ("ERGON_HOST_CHECKS", "off"),
        ("RUSTERON_MEDIA_DRIVER_MLOCKALL", "sometimes"),
        ("AERON_TERM_BUFFER_SPARSE_FILE", "true"),
    ]);
    let status = status.unwrap_or_else(|| panic!("a driver started: {stdout}\n{stderr}"));
    assert_ne!(status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("RUSTERON_MEDIA_DRIVER_MLOCKALL"), "{stderr}");
    let ours: Vec<&str> = stderr.lines().filter(|l| l.starts_with("ergon-md-dpdk ")).collect();
    assert_eq!(ours.len(), 1, "{stderr}");
    assert!(
        ours[0].starts_with("ergon-md-dpdk info: ERGON_HOST_CHECKS=off: no host check runs"),
        "{}",
        ours[0]
    );

    // the same driver under the default policy is warned about
    let (_, _, stderr) = run_binary(&[
        ("RUSTERON_MEDIA_DRIVER_MLOCKALL", "sometimes"),
        ("AERON_TERM_BUFFER_SPARSE_FILE", "true"),
    ]);
    assert!(stderr.contains("ergon-md-dpdk warning: M1 warn: "), "{stderr}");
    assert!(stderr.contains("ergon-md-dpdk warning: M2 warn: "), "{stderr}");
    assert!(!stderr.contains("no host check runs"), "{stderr}");

    // The old alias is ignored: `off` still warns, and `require` or a bad
    // value neither refuses nor stops the start (MLOCKALL stops it later).
    for value in ["off", "require", "loud"] {
        let (status, stdout, stderr) = run_binary(&[
            ("RUSTERON_MEDIA_DRIVER_HOST_CHECK", value),
            ("RUSTERON_MEDIA_DRIVER_MLOCKALL", "sometimes"),
            ("AERON_TERM_BUFFER_SPARSE_FILE", "true"),
        ]);
        let status = status.unwrap_or_else(|| panic!("{value:?}: a driver started: {stdout}\n{stderr}"));
        assert_ne!(status.code(), Some(0), "{value:?}: {stderr}");
        assert!(
            stderr.contains("ergon-md-dpdk warning: M1 warn: "),
            "{value:?}: {stderr}"
        );
        assert!(stderr.contains("RUSTERON_MEDIA_DRIVER_MLOCKALL"), "{value:?}: {stderr}");
        assert!(!stderr.contains("HostCheck"), "{value:?}: {stderr}");
        assert!(
            !stderr.contains("RUSTERON_MEDIA_DRIVER_HOST_CHECK"),
            "{value:?}: {stderr}"
        );
    }

    // Any case works: `OFF` is `off`. What is not one of the words is refused.
    for bad in ["loud", "", "strict", "sometimes"] {
        let (status, stdout, stderr) = run_binary(&[("ERGON_HOST_CHECKS", bad)]);
        let status = status.unwrap_or_else(|| panic!("{bad:?}: a driver started: {stdout}\n{stderr}"));
        assert_ne!(status.code(), Some(0), "{bad:?}: {stderr}");
        assert!(stderr.contains("ERGON_HOST_CHECKS"), "{bad:?}: {stderr}");
        assert!(!stdout.contains("media driver started"), "{bad:?}: {stdout}");
    }
    let (status, stdout, stderr) = run_binary(&[
        ("ERGON_HOST_CHECKS", "OFF"),
        ("RUSTERON_MEDIA_DRIVER_MLOCKALL", "sometimes"),
    ]);
    let status = status.unwrap_or_else(|| panic!("a driver started: {stdout}\n{stderr}"));
    assert_ne!(status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("ERGON_HOST_CHECKS=off"), "{stderr}");
    assert!(!stderr.contains("ergon-md-dpdk warning:"), "{stderr}");
}

/// On Linux, naming a data NIC whose carrier counter is readable starts the
/// re-check thread, on a CPU off the spinning agent's core, and dropping the
/// configured driver stops it; elsewhere nothing starts.
#[test]
#[serial]
fn the_recheck_thread_starts_for_named_nics_and_stops_with_the_driver() {
    let _transport = EnvGuard::unset("RUSTERON_MEDIA_DRIVER_TRANSPORT");
    let _policy = EnvGuard::set("ERGON_HOST_CHECKS", "warn");
    let _nics = EnvGuard::set("RUSTERON_MEDIA_DRIVER_DATA_NICS", "lo");
    let cpus = allowed_cpus();
    let spinner = *cpus.last().unwrap();
    let _cpu = EnvGuard::set("AERON_SENDER_CPU_AFFINITY", &spinner.to_string());
    let _idle = EnvGuard::set("AERON_SENDER_IDLE_STRATEGY", "noop");
    let configured = ConfiguredDriver::from_env(AeronDriverContext::new().unwrap()).unwrap();
    if cfg!(target_os = "linux") {
        assert!(cpus.len() >= 2, "two CPUs are needed: {cpus:?}");
        assert!(configured.rechecking(), "lo's carrier_changes is readable");
        let on = thread_cpus("md-host-checks").expect("the re-check thread");
        assert!(
            !on.is_empty() && !on.contains(&spinner),
            "on {on:?}, the sender on {spinner}"
        );
        drop(configured);
        assert_eq!(thread_cpus("md-host-checks"), None, "stopped with the driver");
    } else {
        assert!(!configured.rechecking(), "not Linux: nothing to re-check");
    }
}

/// The CPUs this process may use (all of them where /proc is absent).
fn allowed_cpus() -> Vec<u32> {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
                .and_then(|l| CpuSet::parse_list(l).ok())
        })
        .map_or_else(|| (0..2).collect(), |set| set.iter().collect())
}

/// The CPUs the thread named `name` may run on, if there is one.
fn thread_cpus(name: &str) -> Option<Vec<u32>> {
    for task in std::fs::read_dir("/proc/self/task").ok()? {
        let task = task.ok()?.path();
        if std::fs::read_to_string(task.join("comm")).is_ok_and(|c| c.trim() == name) {
            let status = std::fs::read_to_string(task.join("status")).ok()?;
            let list = status.lines().find_map(|l| l.strip_prefix("Cpus_allowed_list:"))?;
            return Some(CpuSet::parse_list(list).ok()?.iter().collect());
        }
    }
    None
}
