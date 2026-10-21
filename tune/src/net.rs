//! A NIC's interrupts (T2) and NUMA node (T6), its `carrier_changes`, and
//! the host's busy-polling sysctls.

use std::fmt;

use crate::cpu::{housekeeping, isolated_cpus};
use crate::cpuset::CpuSet;
use crate::finding::{CheckId, Finding, HotSet, Severity};
use crate::host::{Host, ReadError};
use crate::mem::nodes_online;

/// A NIC, by kernel interface name or PCI address.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Nic {
    /// A kernel network interface (`ens5`, `eth1`). The interfaces below
    /// it (`lower_*`) are included: on Azure its accelerated-networking VF,
    /// under a VLAN or bond the NICs it runs on.
    Interface(String),
    /// A PCI device by address (`0000:00:06.0`): a DPDK port, bound to
    /// `igb_uio` or `vfio-pci`.
    Pci(String),
}

impl fmt::Display for Nic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interface(name) => f.write_str(name),
            Self::Pci(bdf) => write!(f, "PCI device {bdf}"),
        }
    }
}

/// One interrupt of a NIC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NicIrq {
    /// The IRQ number.
    pub irq: u32,
    /// The PCI device it belongs to.
    pub device: String,
    /// Its handler's name (`ens5-Tx-Rx-0`), from `/sys/kernel/irq/<n>/actions`.
    pub action: Option<String>,
}

impl NicIrq {
    fn label(&self) -> String {
        self.action.as_ref().map_or_else(
            || format!("IRQ {} of {}", self.irq, self.device),
            |a| format!("IRQ {} ({a}) of {}", self.irq, self.device),
        )
    }
}

/// An interface name the kernel could have made: 1 to 15 bytes, not `.` or
/// `..`, no `/`, `:` or whitespace (`dev_valid_name`). Keeps a name from
/// reaching outside `/sys/class/net`.
pub fn check_interface(name: &str) -> Result<(), ReadError> {
    let ok = !name.is_empty()
        && name.len() < 16
        && name != "."
        && name != ".."
        && !name
            .bytes()
            .any(|b| b == b'/' || b == b':' || b.is_ascii_whitespace());
    if ok {
        Ok(())
    } else {
        Err(ReadError::Malformed {
            path: name.to_owned(),
            detail: "not an interface name",
        })
    }
}

fn check_bdf(bdf: &str) -> Result<(), ReadError> {
    let ok = !bdf.is_empty()
        && bdf.len() <= 16
        && bdf
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
    if ok {
        Ok(())
    } else {
        Err(ReadError::Malformed {
            path: bdf.to_owned(),
            detail: "not a PCI address",
        })
    }
}

/// An interface's device: its name on its bus, the bus, and its driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub name: String,
    pub bus: Option<String>,
    pub driver: Option<String>,
}

/// `interface`'s device, or `None` when it has none (a virtual interface).
///
/// # Errors
/// No such interface, or its links cannot be read.
pub fn interface_device(host: &Host, interface: &str) -> Result<Option<Device>, ReadError> {
    check_interface(interface)?;
    let base = format!("/sys/class/net/{interface}");
    // `bonding_masters` sits beside the interfaces, a file.
    if !host.is_dir(&base) {
        return Err(host.list(&base).err().unwrap_or_else(|| ReadError::Io {
            path: base.clone(),
            kind: std::io::ErrorKind::NotFound,
        }));
    }
    let Some(name) = host.link_name(&format!("{base}/device"))? else {
        return Ok(None);
    };
    Ok(Some(Device {
        name,
        bus: host.link_name(&format!("{base}/device/subsystem"))?,
        driver: host.link_name(&format!("{base}/device/driver"))?,
    }))
}

/// The interfaces below `interface` (`lower_<name>` links: Azure's
/// accelerated-networking VF under its `hv_netvsc` interface).
pub fn lower_interfaces(host: &Host, interface: &str) -> Result<Vec<String>, ReadError> {
    check_interface(interface)?;
    Ok(host
        .list(&format!("/sys/class/net/{interface}"))?
        .iter()
        .filter_map(|n| n.strip_prefix("lower_"))
        .map(str::to_owned)
        .collect())
}

/// The error for an interface whose device is not visible.
pub fn no_device(interface: &str) -> ReadError {
    ReadError::NoDevice {
        interface: interface.to_owned(),
        detail: "no device (a virtual interface, such as a pod's veth): its NIC is not visible here",
    }
}

/// How deep [`pci_devices`] follows `lower_*` links: a VLAN on a bond of
/// Azure `hv_netvsc` interfaces over their VFs is three levels.
const LOWER_DEPTH: u32 = 4;

/// The PCI devices of `interface` and of the interfaces below it
/// (`lower_*`: an Azure VF, a VLAN's or bond's NICs), into `out`; the first
/// device seen that is not PCI, for the reason when there is none.
fn collect_pci(
    host: &Host,
    interface: &str,
    depth: u32,
    out: &mut Vec<String>,
    other: &mut Option<Device>,
) -> Result<(), ReadError> {
    match interface_device(host, interface)? {
        Some(d) if d.bus.as_deref() == Some("pci") => {
            if !out.contains(&d.name) {
                out.push(d.name);
            }
        }
        Some(d) => {
            other.get_or_insert(d);
        }
        None => {}
    }
    if depth == 0 {
        return Ok(());
    }
    for lower in lower_interfaces(host, interface)? {
        collect_pci(host, &lower, depth - 1, out, other)?;
    }
    Ok(())
}

/// The PCI devices behind `nic`: an interface's own device if it is PCI,
/// and those of the interfaces below it (`lower_*`: its Azure VF, the NICs
/// under a VLAN or bond); a PCI address as given.
pub fn pci_devices(host: &Host, nic: &Nic) -> Result<Vec<String>, ReadError> {
    match nic {
        Nic::Pci(bdf) => {
            check_bdf(bdf)?;
            let dir = format!("/sys/bus/pci/devices/{bdf}");
            if !host.exists(&dir) {
                return Err(ReadError::Io {
                    path: dir,
                    kind: std::io::ErrorKind::NotFound,
                });
            }
            Ok(vec![bdf.clone()])
        }
        Nic::Interface(name) => {
            let mut out = Vec::new();
            let mut other = None;
            collect_pci(host, name, LOWER_DEPTH, &mut out, &mut other)?;
            if !out.is_empty() {
                return Ok(out);
            }
            let Some(dev) = other else {
                return Err(no_device(name));
            };
            let detail = match dev.bus.as_deref() {
                Some("vmbus") => {
                    "no PCI device (a VMBus device with no VF): its interrupts are not in /proc/irq"
                }
                Some("virtio") => {
                    "no PCI device of its own (a virtio device: its interrupts are its parent \
                     PCI device's, which this check does not follow)"
                }
                _ => "no PCI device: its interrupts cannot be mapped",
            };
            Err(ReadError::NoDevice {
                interface: name.clone(),
                detail,
            })
        }
    }
}

/// The interrupts of `nic`'s PCI devices (an interface's VFs included), in
/// IRQ order: each device's `msi_irqs`, or its legacy `irq` when that has a
/// handler.
///
/// # Errors
/// The interface or device is not there, has no PCI device, or its files
/// cannot be read.
pub fn nic_irqs(host: &Host, nic: &Nic) -> Result<Vec<NicIrq>, ReadError> {
    let mut out = Vec::new();
    for bdf in pci_devices(host, nic)? {
        let dir = format!("/sys/bus/pci/devices/{bdf}");
        let numbers: Vec<u32> = match host.list(&format!("{dir}/msi_irqs")) {
            Ok(names) => names
                .iter()
                .map(|n| {
                    n.parse().map_err(|_| ReadError::Malformed {
                        path: format!("{dir}/msi_irqs/{n}"),
                        detail: "not an IRQ number",
                    })
                })
                .collect::<Result<_, _>>()?,
            Err(ReadError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => {
                let legacy = host.read_i64(&format!("{dir}/irq")).unwrap_or(0);
                match u32::try_from(legacy) {
                    Ok(n)
                        if n > 0
                            && host
                                .read_line(&format!("/sys/kernel/irq/{n}/actions"))
                                .is_ok_and(|a| !a.is_empty()) =>
                    {
                        vec![n]
                    }
                    _ => Vec::new(),
                }
            }
            Err(e) => return Err(e),
        };
        for irq in numbers {
            let action = host
                .read_line(&format!("/sys/kernel/irq/{irq}/actions"))
                .ok()
                .filter(|a| !a.is_empty());
            out.push(NicIrq {
                irq,
                device: bdf.clone(),
                action,
            });
        }
    }
    out.sort_by_key(|i| i.irq);
    Ok(out)
}

const T2_EVIDENCE: &str = "git show \
     128747b9^:io/benches/results/flags-aws-20261015T062428Z/tables.md (c1/c2-base against \
     c1/c2-irqlocal: noise lines 9-15, market data lines 56-62): AWS c7i, two reps; Linux \
     drivers/net/ethernet/amazon/ena/ena_netdev.c (ena_setup_io_intr: queue i's IRQ on CPU i \
     modulo the online CPUs, set again at every ena_up)";

/// T2 for one interrupt: a warning when its `smp_affinity_list` names a
/// busy-spinning CPU, information when only its effective affinity still
/// does, unknown when unreadable, nothing when it is clear.
fn t2_irq(
    host: &Host,
    irq: &NicIrq,
    cpus: &HotSet,
    spinning: &CpuSet,
    hk: &str,
) -> Option<Finding> {
    let path = format!("/proc/irq/{}/smp_affinity_list", irq.irq);
    let affinity = match host.read_cpu_list(&path) {
        Ok(a) => a,
        Err(e) => {
            return Some(Finding::unknown(
                CheckId::T2,
                Severity::Warn,
                T2_EVIDENCE,
                format!("cannot tell where {} may fire", irq.label()),
                e,
            ));
        }
    };
    let hits = affinity.intersection(spinning);
    if !hits.is_empty() {
        let ena = host
            .link_name(&format!("/sys/bus/pci/devices/{}/driver", irq.device))
            .ok()
            .flatten()
            .is_some_and(|d| d == "ena");
        let (reset, repin) = if ena {
            (
                " ENA re-places its queue IRQs at every interface reset (queue i on CPU i modulo \
                 the number of online CPUs, isolated ones included).",
                ", and re-pin at every link event (when carrier_changes moves)",
            )
        } else {
            ("", "")
        };
        return Some(Finding::found(
            CheckId::T2,
            Severity::Warn,
            T2_EVIDENCE,
            format!(
                "{} may fire on {}: its smp_affinity_list is {affinity}. On AWS c7i, NIC interrupts on \
                 the loop's CPU raised a gap-spinner's gaps over 5 µs from 126–153 to 462–547 a \
                 minute, and busy-polled market data's p99 / max from 7.9–8.1 / 36–48 µs to \
                 20–26 / 74–99 µs (two reps).{reset} Fix: `echo {hk} > \
                 /proc/irq/{}/smp_affinity_list`{repin}",
                irq.label(),
                cpus.describe_all(&hits),
                irq.irq
            ),
        ));
    }
    let effective = host
        .read_cpu_list(&format!("/proc/irq/{}/effective_affinity_list", irq.irq))
        .unwrap_or_default();
    let still = effective.intersection(spinning);
    (!still.is_empty()).then(|| {
        Finding::found(
            CheckId::T2,
            Severity::Info,
            T2_EVIDENCE,
            format!(
                "{} is still delivered to {} (effective_affinity_list {effective}) although its \
                 smp_affinity_list ({affinity}) excludes it: it moves at its next interrupt",
                irq.label(),
                cpus.describe_all(&still)
            ),
        )
    })
}

/// T2: no interrupt of `nic` may fire on a busy-spinning CPU of `cpus`.
///
/// Judged by each IRQ's `smp_affinity_list` (a warning per IRQ). An IRQ
/// whose list excludes the hot CPUs but whose `effective_affinity_list`
/// still names one is information: a VM moves an IRQ only at its next
/// interrupt.
#[must_use]
pub fn t2_nic_irqs(host: &Host, nic: &Nic, cpus: &HotSet) -> Vec<Finding> {
    let id = CheckId::T2;
    let sev = Severity::Warn;
    let spinning = cpus.spinning();
    if spinning.is_empty() {
        return vec![Finding::ok(
            id,
            sev,
            T2_EVIDENCE,
            "no busy-spinning CPU to check".to_owned(),
        )];
    }
    let irqs = match nic_irqs(host, nic) {
        Ok(v) => v,
        Err(e) => {
            return vec![Finding::unknown(
                id,
                sev,
                T2_EVIDENCE,
                format!("cannot map {nic}'s interrupts"),
                e,
            )];
        }
    };
    let isolated = isolated_cpus(host).unwrap_or_default();
    let hk = housekeeping(host, &spinning.union(&isolated));
    let mut out: Vec<Finding> = irqs
        .iter()
        .filter_map(|irq| t2_irq(host, irq, cpus, &spinning, &hk))
        .collect();
    if irqs.is_empty() {
        return vec![Finding::ok(
            id,
            sev,
            T2_EVIDENCE,
            format!(
                "{nic} has no interrupts (no msi_irqs, no legacy IRQ with a handler): nothing \
                 can fire on {}",
                cpus.describe_all(&spinning)
            ),
        )];
    }
    if out.is_empty() {
        let devices: Vec<&str> =
            irqs.iter()
                .map(|i| i.device.as_str())
                .fold(Vec::new(), |mut v, d| {
                    if !v.contains(&d) {
                        v.push(d);
                    }
                    v
                });
        out.push(Finding::ok(
            id,
            sev,
            T2_EVIDENCE,
            format!(
                "{} interrupts of {nic} ({}) are off {}",
                irqs.len(),
                devices.join(", "),
                cpus.describe_all(&spinning)
            ),
        ));
    }
    out
}

const T6_EVIDENCE: &str = "DPDK's documentation (NUMA placement of ports, cores and memory); \
     not measured: every lab host had one node";

/// T6: on a host with more than one NUMA node, `nic` is on the node of the
/// busy-spinning CPUs of `cpus`. The media driver's check (ergon-io does not
/// call it).
#[must_use]
pub fn t6_nic_node(host: &Host, nic: &Nic, cpus: &HotSet) -> Vec<Finding> {
    let id = CheckId::T6;
    let sev = Severity::Warn;
    let unknown =
        |what: String, e: &dyn fmt::Display| vec![Finding::unknown(id, sev, T6_EVIDENCE, what, e)];
    let nodes = match nodes_online(host) {
        Ok(Some(n)) if n.len() > 1 => n,
        Ok(_) => {
            return vec![Finding::ok(
                id,
                sev,
                T6_EVIDENCE,
                "one NUMA node: the NIC's node does not apply".to_owned(),
            )];
        }
        Err(e) => {
            return unknown(
                "cannot tell how many NUMA nodes the host has".to_owned(),
                &e,
            );
        }
    };
    let mut cpus_of: Vec<(u32, CpuSet)> = Vec::new();
    for n in nodes.iter() {
        match host.read_cpu_list(&format!("/sys/devices/system/node/node{n}/cpulist")) {
            Ok(c) => cpus_of.push((n, c)),
            Err(e) => return unknown(format!("cannot tell node {n}'s CPUs"), &e),
        }
    }
    let node_of = |cpu: u32| {
        cpus_of
            .iter()
            .find(|(_, c)| c.contains(cpu))
            .map(|(n, _)| *n)
    };
    let devices = match pci_devices(host, nic) {
        Ok(d) => d,
        Err(e) => return unknown(format!("cannot tell {nic}'s NUMA node"), &e),
    };
    let spinning = cpus.spinning();
    let mut out = Vec::new();
    for bdf in devices {
        let node = match host.read_i64(&format!("/sys/bus/pci/devices/{bdf}/numa_node")) {
            Ok(n) => n,
            Err(e) => return unknown(format!("cannot tell {bdf}'s NUMA node"), &e),
        };
        let Ok(node) = u32::try_from(node) else {
            out.push(Finding::unknown(
                id,
                sev,
                T6_EVIDENCE,
                format!("cannot tell {bdf}'s NUMA node"),
                "its numa_node is -1: the firmware gave the kernel none",
            ));
            continue;
        };
        let remote: CpuSet = spinning
            .iter()
            .filter(|c| node_of(*c) != Some(node))
            .collect();
        if remote.is_empty() {
            out.push(Finding::ok(
                id,
                sev,
                T6_EVIDENCE,
                format!(
                    "{bdf} is on node {node}, with {}",
                    cpus.describe_all(&spinning)
                ),
            ));
            continue;
        }
        let local = cpus_of
            .iter()
            .find(|(n, _)| *n == node)
            .map_or_else(String::new, |(_, c)| c.to_string());
        let theirs: Vec<String> = remote
            .iter()
            .map(|c| {
                node_of(c).map_or_else(
                    || format!("{} on no node", cpus.describe(c)),
                    |n| format!("{} on node {n}", cpus.describe(c)),
                )
            })
            .collect();
        out.push(Finding::found(
            id,
            sev,
            T6_EVIDENCE,
            format!(
                "port {bdf} is on node {node} and {}: every packet crosses the interconnect between \
                 the nodes. Fix: pin them to node {node}'s CPUs ({local}), or use a NIC on their node",
                theirs.join(", ")
            ),
        ));
    }
    out
}

/// How many times `interface`'s carrier has changed
/// (`/sys/class/net/<if>/carrier_changes`): a re-check is due when it
/// moves (an ENA reset brings the interface down and up).
///
/// # Errors
/// The file cannot be read or is not a number.
pub fn carrier_changes(host: &Host, interface: &str) -> Result<u64, ReadError> {
    check_interface(interface)?;
    let path = format!("/sys/class/net/{interface}/carrier_changes");
    let n = host.read_i64(&path)?;
    u64::try_from(n).map_err(|_| ReadError::Malformed {
        path,
        detail: "a negative count",
    })
}

/// The host-wide kernel busy-polling settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusyPoll {
    /// `net.core.busy_read` (µs): blocking reads busy-poll this long.
    pub busy_read: u32,
    /// `net.core.busy_poll` (µs): `poll`/`select` busy-poll this long.
    pub busy_poll: u32,
}

/// Reads `net.core.busy_read` and `busy_poll`. They exist only in the
/// host's network namespace: in a pod they are absent, which is an error
/// (unknown), never zero.
///
/// # Errors
/// Either file cannot be read or is not a number.
pub fn busy_poll_sysctls(host: &Host) -> Result<BusyPoll, ReadError> {
    let read = |name: &str| {
        let path = format!("/proc/sys/net/core/{name}");
        let v = host.read_i64(&path)?;
        u32::try_from(v).map_err(|_| ReadError::Malformed {
            path,
            detail: "out of range",
        })
    };
    Ok(BusyPoll {
        busy_read: read("busy_read")?,
        busy_poll: read("busy_poll")?,
    })
}
