//! Fixture trees standing for a host's `/sys` and `/proc`, built in a fresh
//! temporary directory per test and removed when dropped.
//!
//! Five hosts, from what the lab hosts showed (`md-dpdk/benches/cloud`,
//! `io/benches`): a tuned AWS Graviton c9g (no SMT, no OS idle states), a
//! tuned AWS c7i x86 with SMT, an untuned Debian 13 EC2 host at its
//! defaults, an Azure host whose `hv_netvsc` interfaces have MANA and
//! Mellanox VFs, and a pod's network namespace (host CPU, IRQ and memory
//! files, but only a veth and none of the host's network sysctls).
#![allow(
    dead_code,
    missing_docs,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]

use std::path::{Path, PathBuf};

use ergon_tune::Host;

/// A host tree under a temporary directory.
pub struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

impl Fixture {
    /// An empty tree named after the test.
    pub fn empty(name: &str) -> R<Self> {
        let root = std::env::temp_dir().join(format!("ergon-tune-{name}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root)?;
        }
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn host(&self) -> Host {
        Host::at(&self.root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn at(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    /// Writes `contents` to `abs`, creating its directories.
    pub fn file(&self, abs: &str, contents: &str) -> R<&Self> {
        let p = self.at(abs);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, contents)?;
        Ok(self)
    }

    /// A directory at `abs`.
    pub fn dir(&self, abs: &str) -> R<&Self> {
        std::fs::create_dir_all(self.at(abs))?;
        Ok(self)
    }

    /// A symbolic link at `abs` to `target` (relative to the link, as sysfs
    /// has them).
    pub fn link(&self, abs: &str, target: &str) -> R<&Self> {
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

    /// Removes the file, link or directory at `abs`.
    pub fn remove(&self, abs: &str) -> R<&Self> {
        let p = self.at(abs);
        if p.is_dir() && !p.is_symlink() {
            std::fs::remove_dir_all(p)?;
        } else {
            std::fs::remove_file(p)?;
        }
        Ok(self)
    }

    /// `n` CPUs online, each with `thread_siblings_list` from `siblings`.
    pub fn cpus(&self, n: u32, siblings: impl Fn(u32) -> String) -> R<&Self> {
        self.file("/sys/devices/system/cpu/online", &format!("0-{}\n", n - 1))?;
        self.file(
            "/sys/devices/system/cpu/possible",
            &format!("0-{}\n", n - 1),
        )?;
        for c in 0..n {
            self.file(
                &format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list"),
                &format!("{}\n", siblings(c)),
            )?;
        }
        Ok(self)
    }

    /// The boot's isolation: `isolated`, `nohz_full` (`None`: a kernel
    /// without `NO_HZ_FULL`) and `default_smp_affinity`.
    pub fn isolation(
        &self,
        isolated: &str,
        nohz_full: Option<&str>,
        default_mask: &str,
    ) -> R<&Self> {
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

    /// Transparent hugepages: the top-level setting and each mTHP size's.
    pub fn thp(&self, top: &str, sizes: &[(&str, &str)]) -> R<&Self> {
        self.file(
            "/sys/kernel/mm/transparent_hugepage/enabled",
            &format!("{}\n", bracket(&["always", "madvise", "never"], top)),
        )?;
        for (size, v) in sizes {
            self.file(
                &format!("/sys/kernel/mm/transparent_hugepage/hugepages-{size}kB/enabled"),
                &format!(
                    "{}\n",
                    bracket(&["always", "inherit", "madvise", "never"], v)
                ),
            )?;
        }
        Ok(self)
    }

    /// NUMA nodes, each with its CPU list, and `numa_balancing`.
    pub fn numa(&self, nodes: &[&str], balancing: Option<&str>) -> R<&Self> {
        let online = if nodes.len() == 1 {
            "0".to_owned()
        } else {
            format!("0-{}", nodes.len() - 1)
        };
        self.file("/sys/devices/system/node/online", &format!("{online}\n"))?;
        for (i, cpus) in nodes.iter().enumerate() {
            self.file(
                &format!("/sys/devices/system/node/node{i}/cpulist"),
                &format!("{cpus}\n"),
            )?;
        }
        if let Some(b) = balancing {
            self.file("/proc/sys/kernel/numa_balancing", &format!("{b}\n"))?;
        } else {
            self.dir("/proc/sys/kernel")?;
        }
        Ok(self)
    }

    /// The calling thread's policy (41st field of stat) and affinity.
    pub fn thread(&self, policy: u32, cpus_allowed: &str) -> R<&Self> {
        // pid (comm with a space and a parenthesis) state ppid ... policy:
        // 38 fields after the comm before the policy.
        let mut fields: Vec<String> = vec!["S".to_owned()];
        for i in 4..=40 {
            fields.push(if i == 40 {
                "0".to_owned()
            } else {
                i.to_string()
            });
        }
        fields.push(policy.to_string());
        for _ in 0..10 {
            fields.push("0".to_owned());
        }
        self.file(
            "/proc/thread-self/stat",
            &format!("4242 (ergon (loop) x) {}\n", fields.join(" ")),
        )?;
        self.file(
            "/proc/thread-self/status",
            &format!(
                "Name:\tergon-loop\nState:\tR (running)\nCpus_allowed:\tff\nCpus_allowed_list:\t{cpus_allowed}\nMems_allowed_list:\t0\n"
            ),
        )?;
        Ok(self)
    }

    /// A PCI device with its driver, NUMA node and MSI interrupts (each
    /// with its handler name and affinities).
    pub fn pci(&self, bdf: &str, driver: &str, node: i32, irqs: &[Irq]) -> R<&Self> {
        let d = format!("/sys/bus/pci/devices/{bdf}");
        self.dir(&d)?;
        self.dir(&format!("/sys/bus/pci/drivers/{driver}"))?;
        self.link(
            &format!("{d}/driver"),
            &format!("../../../../bus/pci/drivers/{driver}"),
        )?;
        self.link(&format!("{d}/subsystem"), "../../../../bus/pci")?;
        self.file(&format!("{d}/numa_node"), &format!("{node}\n"))?;
        self.file(&format!("{d}/irq"), "0\n")?;
        if !irqs.is_empty() {
            self.dir(&format!("{d}/msi_irqs"))?;
        }
        for irq in irqs {
            self.file(&format!("{d}/msi_irqs/{}", irq.n), "msix\n")?;
            self.irq(irq)?;
        }
        Ok(self)
    }

    /// One interrupt's `/proc/irq/<n>` and `/sys/kernel/irq/<n>/actions`.
    pub fn irq(&self, irq: &Irq) -> R<&Self> {
        self.file(
            &format!("/proc/irq/{}/smp_affinity_list", irq.n),
            &format!("{}\n", irq.affinity),
        )?;
        if let Some(e) = irq.effective {
            self.file(
                &format!("/proc/irq/{}/effective_affinity_list", irq.n),
                &format!("{e}\n"),
            )?;
        }
        self.file(
            &format!("/sys/kernel/irq/{}/actions", irq.n),
            &format!("{}\n", irq.name),
        )?;
        Ok(self)
    }

    /// A network interface on a device (`/sys/bus/<bus>/devices/<dev>`).
    pub fn netdev(&self, name: &str, ifindex: u32, device: Option<(&str, &str)>) -> R<&Self> {
        let d = format!("/sys/class/net/{name}");
        self.file(&format!("{d}/ifindex"), &format!("{ifindex}\n"))?;
        self.file(&format!("{d}/carrier_changes"), "2\n")?;
        self.file(&format!("{d}/operstate"), "up\n")?;
        self.file(&format!("{d}/napi_defer_hard_irqs"), "0\n")?;
        self.file(&format!("{d}/gro_flush_timeout"), "0\n")?;
        if let Some((bus, dev)) = device {
            self.link(
                &format!("{d}/device"),
                &format!("../../../bus/{bus}/devices/{dev}"),
            )?;
        }
        Ok(self)
    }

    /// A `VMBus` device (Azure's synthetic NIC) bound to `hv_netvsc`.
    pub fn vmbus(&self, guid: &str) -> R<&Self> {
        let d = format!("/sys/bus/vmbus/devices/{guid}");
        self.dir(&d)?;
        self.dir("/sys/bus/vmbus/drivers/hv_netvsc")?;
        self.link(
            &format!("{d}/driver"),
            "../../../../bus/vmbus/drivers/hv_netvsc",
        )?;
        self.link(&format!("{d}/subsystem"), "../../../../bus/vmbus")?;
        Ok(self)
    }

    /// DMI vendor and product, and a `/proc/cpuinfo` for the architecture.
    pub fn platform(&self, vendor: &str, product: &str, arm: bool) -> R<&Self> {
        self.file("/sys/class/dmi/id/sys_vendor", &format!("{vendor}\n"))?;
        self.file("/sys/class/dmi/id/product_name", &format!("{product}\n"))?;
        let cpuinfo = if arm {
            "processor\t: 0\nBogoMIPS\t: 2000.00\nFeatures\t: fp asimd evtstrm\nCPU implementer\t: 0x41\nCPU architecture: 8\n"
        } else {
            "processor\t: 0\nvendor_id\t: GenuineIntel\ncpu family\t: 6\nflags\t\t: fpu tsc constant_tsc nonstop_tsc\n"
        };
        self.file("/proc/cpuinfo", cpuinfo)?;
        Ok(self)
    }

    /// The host-wide busy-polling sysctls.
    pub fn busy_poll(&self, read: u32, poll: u32) -> R<&Self> {
        self.file("/proc/sys/net/core/busy_read", &format!("{read}\n"))?;
        self.file("/proc/sys/net/core/busy_poll", &format!("{poll}\n"))?;
        Ok(self)
    }

    /// This process in cgroup v2 at `path` (`/proc/self/cgroup` `0::<path>`),
    /// its directory under `/sys/fs/cgroup` made.
    pub fn cgroup_v2(&self, path: &str) -> R<&Self> {
        self.file("/proc/self/cgroup", &format!("0::{path}\n"))?;
        self.dir(&format!("/sys/fs/cgroup{}", path.trim_end_matches('/')))
    }

    /// A cgroup v2 `cpu.stat` in `dir` with the cpu controller's counters.
    pub fn cpu_stat(&self, dir: &str, periods: u64, throttled: u64, usec: u64) -> R<&Self> {
        self.file(
            &format!("{dir}/cpu.stat"),
            &format!(
                "usage_usec 912345\nuser_usec 900000\nsystem_usec 12345\nnr_periods {periods}\n\
                 nr_throttled {throttled}\nthrottled_usec {usec}\nnr_bursts 0\nburst_usec 0\n"
            ),
        )
    }

    /// cpuidle: the driver (`current_driver`), and on each CPU of `cpus`
    /// the states `(name, exit latency µs, disabled)`, state0 first, as
    /// `/sys/devices/system/cpu/cpu<N>/cpuidle/state<i>` has them.
    pub fn cpuidle(&self, driver: &str, cpus: &[u32], states: &[IdleState]) -> R<&Self> {
        self.file(
            "/sys/devices/system/cpu/cpuidle/current_driver",
            &format!("{driver}\n"),
        )?;
        for c in cpus {
            for (i, (name, latency, disabled)) in states.iter().enumerate() {
                let d = format!("/sys/devices/system/cpu/cpu{c}/cpuidle/state{i}");
                self.file(&format!("{d}/name"), &format!("{name}\n"))?;
                self.file(&format!("{d}/latency"), &format!("{latency}\n"))?;
                self.file(&format!("{d}/residency"), &format!("{}\n", latency * 3))?;
                self.file(
                    &format!("{d}/disable"),
                    if *disabled { "1\n" } else { "0\n" },
                )?;
                self.file(&format!("{d}/usage"), "100\n")?;
                self.file(&format!("{d}/time"), "1000\n")?;
            }
        }
        Ok(self)
    }

    /// One idle state of CPU `cpu` disabled (`1`) or enabled (`0`).
    pub fn idle_disable(&self, cpu: u32, state: u32, disabled: bool) -> R<&Self> {
        self.file(
            &format!("/sys/devices/system/cpu/cpu{cpu}/cpuidle/state{state}/disable"),
            if disabled { "1\n" } else { "0\n" },
        )
    }
}

/// An idle state in a fixture: name, exit latency (µs), disabled.
pub type IdleState = (&'static str, u32, bool);

/// `intel_idle`'s states on AWS c7i (Sapphire Rapids, kernel 7.2), every one
/// enabled: as lab session aws-20261028-092158 read them.
pub const INTEL_SPR: [IdleState; 4] = [
    ("POLL", 0, false),
    ("C1", 1, false),
    ("C1E", 2, false),
    ("C6", 190, false),
];

/// An interrupt in a fixture.
#[derive(Clone, Copy)]
pub struct Irq {
    pub n: u32,
    pub name: &'static str,
    pub affinity: &'static str,
    pub effective: Option<&'static str>,
}

pub const fn irq(
    n: u32,
    name: &'static str,
    affinity: &'static str,
    effective: Option<&'static str>,
) -> Irq {
    Irq {
        n,
        name,
        affinity,
        effective,
    }
}

fn bracket(choices: &[&str], v: &str) -> String {
    choices
        .iter()
        .map(|c| {
            if *c == v {
                format!("[{c}]")
            } else {
                (*c).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// mTHP sizes as arm64 kernels list them (4 KiB pages: 16 KiB to 2 MiB),
/// each at `v`, the PMD size at `pmd`.
pub fn arm64_mthp(v: &'static str, pmd: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("16", v),
        ("32", v),
        ("64", v),
        ("128", v),
        ("256", v),
        ("512", v),
        ("1024", v),
        ("2048", pmd),
    ]
}

/// A tuned AWS c9g.2xlarge (Graviton5): 8 cores, one thread each, CPU 0
/// housekeeping and 1-7 isolated and `nohz_full`, THP never at every size,
/// no OS idle states, one NUMA node; data NIC ens6 (ENA, PCI 0000:00:06.0)
/// with its IRQs on CPU 0; the calling thread `SCHED_OTHER` on CPU 3.
pub fn c9g_tuned(name: &str) -> R<Fixture> {
    let f = Fixture::empty(name)?;
    f.platform("Amazon EC2", "c9g.2xlarge", true)?
        .cpus(8, |c| c.to_string())?
        .isolation("1-7", Some("1-7"), "01")?
        .thp("never", &arm64_mthp("never", "inherit"))?
        .file("/sys/devices/system/cpu/cpuidle/current_driver", "none\n")?
        .numa(&["0-7"], Some("0"))?
        .thread(0, "3")?
        .busy_poll(0, 0)?
        .pci(
            "0000:00:06.0",
            "ena",
            -1,
            &[
                irq(50, "ena-mgmnt@pci:0000:00:06.0", "0", Some("0")),
                irq(51, "ens6-Tx-Rx-0", "0", Some("0")),
                irq(52, "ens6-Tx-Rx-1", "0", Some("0")),
            ],
        )?
        .netdev("ens6", 3, Some(("pci", "0000:00:06.0")))?
        .pci(
            "0000:00:05.0",
            "ena",
            -1,
            &[irq(40, "ens5-Tx-Rx-0", "0-7", Some("1"))],
        )?
        .netdev("ens5", 2, Some(("pci", "0000:00:05.0")))?;
    Ok(f)
}

/// A tuned AWS c7i.2xlarge: 4 cores x 2 threads, CPU n's sibling n+4;
/// CPUs 0 and 4 housekeeping, 1-3 and 5-7 isolated and `nohz_full`, THP
/// never, one node; data NIC ens6 (ENA) with its IRQs on CPU 0.
pub fn c7i_tuned(name: &str) -> R<Fixture> {
    let f = Fixture::empty(name)?;
    f.platform("Amazon EC2", "c7i.2xlarge", false)?
        .cpus(8, |c| format!("{},{}", c % 4, c % 4 + 4))?
        .isolation("1-3,5-7", Some("1-3,5-7"), "11")?
        .thp("never", &[("2048", "inherit")])?
        .numa(&["0-7"], Some("0"))?
        .thread(0, "2")?
        .busy_poll(0, 0)?
        .pci(
            "0000:00:06.0",
            "ena",
            -1,
            &[
                irq(60, "ena-mgmnt@pci:0000:00:06.0", "0", Some("0")),
                irq(61, "ens6-Tx-Rx-0", "0", Some("0")),
                irq(62, "ens6-Tx-Rx-1", "0", Some("0")),
            ],
        )?
        .netdev("ens6", 3, Some(("pci", "0000:00:06.0")))?;
    Ok(f)
}

/// An untuned Debian 13 c7i.2xlarge at its defaults: nothing isolated (a
/// `NO_HZ_FULL` kernel with no `nohz_full` CPUs), default IRQ affinity every
/// CPU, THP always, ENA's queue IRQs where ENA puts them (queue i on CPU i),
/// busy polling off.
pub fn debian_default(name: &str) -> R<Fixture> {
    let f = Fixture::empty(name)?;
    f.platform("Amazon EC2", "c7i.2xlarge", false)?
        .cpus(8, |c| format!("{},{}", c % 4, c % 4 + 4))?
        .isolation("", Some(""), "ff")?
        .thp("always", &[("2048", "inherit")])?
        .numa(&["0-7"], Some("0"))?
        .thread(0, "0-7")?
        .busy_poll(0, 0)?
        .pci(
            "0000:00:05.0",
            "ena",
            0,
            &[
                irq(24, "ena-mgmnt@pci:0000:00:05.0", "0-7", Some("0")),
                irq(25, "ens5-Tx-Rx-0", "0", Some("0")),
                irq(26, "ens5-Tx-Rx-1", "1", Some("1")),
                irq(27, "ens5-Tx-Rx-2", "2", Some("2")),
                irq(28, "ens5-Tx-Rx-3", "3", Some("3")),
            ],
        )?
        .netdev("ens5", 2, Some(("pci", "0000:00:05.0")))?;
    Ok(f)
}

/// An Azure `Standard_D8s_v6`: 4 cores x 2 threads, siblings adjacent (0,1),
/// CPUs 0-1 housekeeping and 2-7 isolated; `eth1` an `hv_netvsc` NIC with
/// its MANA VF `enP30832s1` (PCI 7870:00:00.0, up, its interrupts deferred
/// for busy polling); `eth2` an `hv_netvsc` NIC whose Mellanox VF
/// `enP4660s1` is down; `eth3` an `hv_netvsc` NIC with no VF.
pub fn azure(name: &str) -> R<Fixture> {
    let f = Fixture::empty(name)?;
    f.platform("Microsoft Corporation", "Virtual Machine", false)?
        .cpus(8, |c| format!("{}-{}", c & !1, (c & !1) + 1))?
        .isolation("2-7", Some("2-7"), "03")?
        .thp("never", &[("2048", "inherit")])?
        .numa(&["0-7"], Some("0"))?
        .thread(0, "4")?
        .busy_poll(50, 50)?
        .vmbus("000d3a1b-2c3d-4e5f-8a9b-0c1d2e3f4a5b")?
        .netdev(
            "eth1",
            3,
            Some(("vmbus", "000d3a1b-2c3d-4e5f-8a9b-0c1d2e3f4a5b")),
        )?
        .pci(
            "7870:00:00.0",
            "mana",
            0,
            &[
                irq(70, "mana_hwc@pci:7870:00:00.0", "0", Some("0")),
                irq(71, "mana_q0@pci:7870:00:00.0", "0-1", Some("1")),
                irq(72, "mana_q1@pci:7870:00:00.0", "4", Some("4")),
            ],
        )?
        .netdev("enP30832s1", 5, Some(("pci", "7870:00:00.0")))?
        .file("/sys/class/net/enP30832s1/napi_defer_hard_irqs", "2\n")?
        .file("/sys/class/net/enP30832s1/gro_flush_timeout", "200000\n")?
        .link("/sys/class/net/eth1/lower_enP30832s1", "../enP30832s1")?
        .vmbus("000d3a1b-2c3d-4e5f-8a9b-0c1d2e3f4a6c")?
        .netdev(
            "eth2",
            4,
            Some(("vmbus", "000d3a1b-2c3d-4e5f-8a9b-0c1d2e3f4a6c")),
        )?
        .pci(
            "1234:00:02.0",
            "mlx5_core",
            0,
            &[irq(80, "mlx5_comp0@pci:1234:00:02.0", "0", Some("0"))],
        )?
        .netdev("enP4660s1", 6, Some(("pci", "1234:00:02.0")))?
        .file("/sys/class/net/enP4660s1/operstate", "down\n")?
        .link("/sys/class/net/eth2/lower_enP4660s1", "../enP4660s1")?
        .vmbus("000d3a1b-2c3d-4e5f-8a9b-0c1d2e3f4a7d")?
        .netdev(
            "eth3",
            7,
            Some(("vmbus", "000d3a1b-2c3d-4e5f-8a9b-0c1d2e3f4a7d")),
        )?;
    Ok(f)
}

/// A pod on a two-node host: the host's CPU, IRQ and memory files (host
/// wide, so visible, `numa_balancing` on), but its own network namespace:
/// only `lo` and a veth `eth0` (no device) and no `/proc/sys/net/core` (the
/// host's network sysctls), and a non-root cgroup (no
/// `cpuset.cpus.isolated`).
pub fn pod(name: &str) -> R<Fixture> {
    let f = Fixture::empty(name)?;
    f.platform("Amazon EC2", "c7i.2xlarge", false)?
        .cpus(8, |c| format!("{},{}", c % 4, c % 4 + 4))?
        .isolation("2-3,6-7", Some("2-3,6-7"), "33")?
        .thp("never", &[("2048", "inherit")])?
        .numa(&["0-3", "4-7"], Some("1"))?
        .thread(0, "2")?
        .netdev("eth0", 3, None)?
        .netdev("lo", 1, None)?
        .dir("/proc/sys/net/ipv4")?
        .dir("/sys/fs/cgroup")?;
    Ok(f)
}
