//! `/proc/interrupts` and `/proc/softirqs`, and which rows are the per-CPU
//! timer.
//!
//! The per-CPU timer fires on an isolated CPU too (a `nohz_full` CPU still
//! ticks at least once a second), so the device-IRQ check (F2) leaves it
//! out and the tick check (F3) owns it. Which row it is depends on the
//! platform:
//!
//! | platform | timer row |
//! |---|---|
//! | x86 (KVM, Nitro, metal) | `LOC` |
//! | x86 Hyper-V (Azure) | `HVS` (`LOC` kept too) |
//! | arm64 (GIC) | the numbered row whose description holds `arch_timer` |

use std::collections::BTreeMap;

use crate::LlError;

/// What a row counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrqKind {
    /// A numbered row that is not the timer: a device interrupt.
    Device,
    /// The per-CPU timer ([module docs](self)).
    Timer,
    /// A named row that is not the timer: IPIs and architecture events.
    Ipi,
    /// A row with no per-CPU counts (`ERR`, `MIS`).
    Total,
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrqRow {
    /// `24`, `LOC`, `IPI0`…
    pub name: String,
    /// What it counts.
    pub kind: IrqKind,
    /// Its count per CPU of the header (a short row has fewer).
    pub counts: Vec<u64>,
    /// The rest of the line.
    pub desc: String,
}

/// A parsed table: the header's CPUs and the rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// The CPU of each count column (offline CPUs have none).
    pub cpus: Vec<usize>,
    /// The rows.
    pub rows: Vec<IrqRow>,
}

impl Table {
    /// Row `name`'s count on `cpu`, when the row has a column for it.
    #[must_use]
    pub fn count(&self, row: &IrqRow, cpu: usize) -> Option<u64> {
        let col = self.cpus.iter().position(|&c| c == cpu)?;
        row.counts.get(col).copied()
    }

    /// Per row, the increase from `before` to `self` on `cpu` (rows absent
    /// from `before` count from 0).
    #[must_use]
    pub fn deltas(&self, before: &Self, cpu: usize) -> Vec<(String, IrqKind, u64)> {
        let earlier: BTreeMap<&str, (&Self, &IrqRow)> = before
            .rows
            .iter()
            .map(|r| (r.name.as_str(), (before, r)))
            .collect();
        self.rows
            .iter()
            .filter_map(|r| {
                let now = self.count(r, cpu)?;
                let then = earlier
                    .get(r.name.as_str())
                    .and_then(|(t, row)| t.count(row, cpu))
                    .unwrap_or(0);
                Some((r.name.clone(), r.kind, now.saturating_sub(then)))
            })
            .collect()
    }
}

/// Parses `/proc/interrupts`.
///
/// # Errors
///
/// [`LlError::BadValue`] when the header names no CPU or a count does not
/// parse.
pub fn parse_interrupts(text: &str) -> Result<Table, LlError> {
    parse_table("/proc/interrupts", text, |name, desc| {
        let numbered = name.bytes().all(|b| b.is_ascii_digit());
        if name == "LOC" || name == "HVS" || (numbered && desc.contains("arch_timer")) {
            IrqKind::Timer
        } else if numbered {
            IrqKind::Device
        } else {
            IrqKind::Ipi
        }
    })
}

fn parse_table(
    what: &str,
    text: &str,
    kind: impl Fn(&str, &str) -> IrqKind,
) -> Result<Table, LlError> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    let cpus: Vec<usize> = header
        .split_whitespace()
        .map(|c| c.strip_prefix("CPU").and_then(|n| n.parse().ok()))
        .collect::<Option<Vec<usize>>>()
        .ok_or_else(|| LlError::bad(what, header, "the header is not CPU0 CPU1 …"))?;
    if cpus.is_empty() {
        return Err(LlError::bad(what, header, "the header names no CPU"));
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_owned();
        let mut counts = Vec::new();
        let mut words = rest.split_whitespace().peekable();
        while counts.len() < cpus.len() {
            match words.peek().map(|w| w.parse::<u64>()) {
                Some(Ok(n)) => {
                    counts.push(n);
                    words.next();
                }
                _ => break,
            }
        }
        let desc = words.collect::<Vec<_>>().join(" ");
        if counts.is_empty() && !desc.is_empty() {
            return Err(LlError::bad(what, line, "a row with no counts"));
        }
        // ERR and MIS (arm64: Err) are one machine-wide count, not per CPU
        let kind = if matches!(name.as_str(), "ERR" | "MIS" | "Err") {
            IrqKind::Total
        } else {
            kind(&name, &desc)
        };
        rows.push(IrqRow {
            name,
            kind,
            counts,
            desc,
        });
    }
    Ok(Table { cpus, rows })
}

/// Parses `/proc/softirqs` (every row is [`IrqKind::Ipi`]-like: named; the
/// kind is set to [`IrqKind::Ipi`]).
///
/// # Errors
///
/// As [`parse_interrupts`].
pub fn parse_softirqs(text: &str) -> Result<Table, LlError> {
    parse_table("/proc/softirqs", text, |_, _| IrqKind::Ipi)
}

#[cfg(test)]
mod tests {
    use super::*;

    const X86_KVM: &str = "\
           CPU0       CPU1       CPU2       CPU3
  0:         36          0          0          0   IO-APIC   2-edge      timer
 24:       1000          0          5          0   PCI-MSI 16384-edge      virtio0-input.0
NMI:          0          0          0          0   Non-maskable interrupts
LOC:      90000       8000       2000        700   Local timer interrupts
RES:         10          2          1          0   Rescheduling interrupts
ERR:          0
MIS:          0
";

    const X86_HYPERV: &str = "\
           CPU0       CPU1
 24:        100          0  Hyper-V PCIe MSI 134250496-edge      mlx5_comp0
HVS:       5000        900   Hyper-V stimer0 interrupts
LOC:          0          0   Local timer interrupts
HYP:         12          3   Hypervisor callback interrupts
";

    const ARM64_GIC: &str = "\
           CPU0       CPU1       CPU2
 11:      73000       1200        950     GICv3  27 Level     arch_timer
 14:          0          0          0     GICv3  26 Level     kvm guest ptimer
 35:        900          0          1     GICv3 32 Level     ena-mgmnt@pci:0000:00:05.0
IPI0:        40         11          2       Rescheduling interrupts
IPI1:         3          1          0       Function call interrupts
Err:          0
";

    // CPU1 offline: no column for it; a row with fewer counts than CPUs.
    const OFFLINE: &str = "\
           CPU0       CPU2
 24:         10          3   PCI-MSI   eth0
LOC:        100
";

    #[test]
    fn x86_kvm_timer_is_loc() -> Result<(), LlError> {
        let t = parse_interrupts(X86_KVM)?;
        assert_eq!(t.cpus, vec![0, 1, 2, 3]);
        let kind = |n: &str| t.rows.iter().find(|r| r.name == n).map(|r| r.kind);
        assert_eq!(kind("LOC"), Some(IrqKind::Timer));
        assert_eq!(kind("24"), Some(IrqKind::Device));
        assert_eq!(kind("0"), Some(IrqKind::Device));
        assert_eq!(kind("RES"), Some(IrqKind::Ipi));
        assert_eq!(kind("ERR"), Some(IrqKind::Total));
        let row = t.rows.iter().find(|r| r.name == "24").cloned();
        assert_eq!(row.and_then(|r| t.count(&r, 2)), Some(5));
        Ok(())
    }

    #[test]
    fn hyperv_timer_is_hvs() -> Result<(), LlError> {
        let t = parse_interrupts(X86_HYPERV)?;
        let kind = |n: &str| t.rows.iter().find(|r| r.name == n).map(|r| r.kind);
        assert_eq!(kind("HVS"), Some(IrqKind::Timer));
        assert_eq!(kind("LOC"), Some(IrqKind::Timer));
        assert_eq!(kind("HYP"), Some(IrqKind::Ipi));
        assert_eq!(kind("24"), Some(IrqKind::Device));
        Ok(())
    }

    #[test]
    fn arm64_timer_is_the_numbered_arch_timer_row() -> Result<(), LlError> {
        let t = parse_interrupts(ARM64_GIC)?;
        let kind = |n: &str| t.rows.iter().find(|r| r.name == n).map(|r| r.kind);
        assert_eq!(kind("11"), Some(IrqKind::Timer));
        assert_eq!(kind("14"), Some(IrqKind::Device));
        assert_eq!(kind("35"), Some(IrqKind::Device));
        assert_eq!(kind("IPI0"), Some(IrqKind::Ipi));
        assert_eq!(kind("Err"), Some(IrqKind::Total));
        Ok(())
    }

    #[test]
    fn offline_cpus_and_short_rows() -> Result<(), LlError> {
        let t = parse_interrupts(OFFLINE)?;
        assert_eq!(t.cpus, vec![0, 2]);
        let loc = t.rows.iter().find(|r| r.name == "LOC").cloned();
        assert_eq!(loc.as_ref().map(|r| r.kind), Some(IrqKind::Timer));
        assert_eq!(loc.as_ref().and_then(|r| t.count(r, 0)), Some(100));
        assert_eq!(loc.as_ref().and_then(|r| t.count(r, 2)), None);
        assert_eq!(loc.as_ref().and_then(|r| t.count(r, 1)), None);
        Ok(())
    }

    #[test]
    fn deltas_per_cpu() -> Result<(), LlError> {
        let before = parse_interrupts(X86_KVM)?;
        let after = parse_interrupts(&X86_KVM.replace("       2000 ", "       2600 "))?;
        let d = after.deltas(&before, 2);
        assert!(
            d.contains(&("LOC".to_owned(), IrqKind::Timer, 600)),
            "{d:?}"
        );
        assert!(d.contains(&("24".to_owned(), IrqKind::Device, 0)), "{d:?}");
        Ok(())
    }

    #[test]
    fn parses_softirqs() -> Result<(), LlError> {
        let t = parse_softirqs(
            "                    CPU0       CPU1\n          HI:          1          0\n       TIMER:        500         20\n      NET_RX:         10          7\n",
        )?;
        assert_eq!(t.cpus, vec![0, 1]);
        assert_eq!(t.rows.len(), 3);
        let net = t.rows.iter().find(|r| r.name == "NET_RX").cloned();
        assert_eq!(net.and_then(|r| t.count(&r, 1)), Some(7));
        Ok(())
    }

    #[test]
    fn refuses_garbage() {
        assert!(parse_interrupts("").is_err());
        assert!(parse_interrupts("CPU0\n 1: x y\n").is_err());
    }
}
