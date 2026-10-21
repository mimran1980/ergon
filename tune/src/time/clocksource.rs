//! T12: the kernel's clock source. The counter itself (`tsc`,
//! `arch_sys_counter`) is read through the vDSO with no system call; a
//! hypervisor's view of it (`kvm-clock`, Hyper-V's TSC page, `xen`) is too
//! where the hypervisor marks it stable; anything else (`hpet`, `acpi_pm`)
//! has no vDSO read mode, so every `CLOCK_REALTIME` and `CLOCK_MONOTONIC`
//! read is a system call (Linux `lib/vdso/gettimeofday.c` falls back to one
//! for a clock source whose `vdso_clock_mode` is none).

use std::fmt;

use crate::finding::{CheckId, Finding, Severity};
use crate::host::{Host, ReadError};
use crate::platform::{Arch, arch_of};

/// Where Linux names the clock source it keeps time with.
pub const CLOCKSOURCE: &str = "/sys/devices/system/clocksource/clocksource0/current_clocksource";

/// Where T12's claims come from.
const T12_EVIDENCE: &str = "Linux lib/vdso/gettimeofday.c (the system-call fallback for a clock \
     source with no vDSO mode), kernel/time/clocksource.c (the watchdog), \
     Documentation/admin-guide/kernel-parameters.txt (clocksource=, tsc=reliable); not measured \
     here";

/// Clock sources that are a hypervisor's scaled view of the hardware
/// counter: KVM's pvclock, Hyper-V's TSC page (Azure), Xen's.
const PARAVIRT: [&str; 3] = ["kvm-clock", "hyperv_clocksource_tsc_page", "xen"];

/// What a clock source is to the hardware counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClocksourceKind {
    /// The counter itself: `tsc` (x86), `arch_sys_counter` (Arm).
    Counter,
    /// A hypervisor's scaled view of it: `kvm-clock`,
    /// `hyperv_clocksource_tsc_page`, `xen`.
    Paravirt,
    /// Anything else (`hpet`, `acpi_pm`, `jiffies`): no vDSO read mode.
    Other,
}

impl ClocksourceKind {
    /// The kind of the clock source Linux names `name` (trimmed).
    #[must_use]
    pub fn of(name: &str) -> Self {
        match name.trim() {
            "tsc" | "arch_sys_counter" => Self::Counter,
            n if PARAVIRT.contains(&n) => Self::Paravirt,
            _ => Self::Other,
        }
    }
}

/// The kernel's clock source and, for the fix, the CPU's architecture and
/// whether its TSC is invariant: what T12 judges. Build one to judge a
/// host that is not here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clocksource {
    /// The clock source's name (`current_clocksource`), or why it could not
    /// be read.
    pub name: Result<String, ReadError>,
    /// The CPU architecture (`/proc/cpuinfo`).
    pub arch: Arch,
    /// On x86, whether `/proc/cpuinfo`'s flags have both `constant_tsc` and
    /// `nonstop_tsc` (the TSC neither changes rate nor stops); `None` where
    /// there are no x86 flags.
    pub invariant_tsc: Option<bool>,
}

impl Clocksource {
    /// Reads `current_clocksource` and `/proc/cpuinfo` under `host`.
    #[must_use]
    pub fn read(host: &Host) -> Self {
        let name = host.read_line(CLOCKSOURCE).and_then(|n| {
            if n.is_empty() {
                Err(ReadError::Malformed {
                    path: CLOCKSOURCE.to_owned(),
                    detail: "empty",
                })
            } else {
                Ok(n)
            }
        });
        let cpuinfo = host.read("/proc/cpuinfo").ok();
        Self {
            name,
            arch: cpuinfo.as_deref().map_or(Arch::Unknown, arch_of),
            invariant_tsc: cpuinfo.as_deref().and_then(invariant_tsc),
        }
    }

    /// The clock source's kind, where its name was read.
    #[must_use]
    pub fn kind(&self) -> Option<ClocksourceKind> {
        self.name.as_deref().ok().map(ClocksourceKind::of)
    }

    /// T12: ok for the counter; info for a hypervisor's view of it; a
    /// warning for any other source on x86-64 or 64-bit Arm, with the fix
    /// for the architecture; unknown where the name could not be read, or
    /// for another source on an architecture this check does not know
    /// (RISC-V and PowerPC have counter sources of their own).
    #[must_use]
    pub fn finding(&self) -> Finding {
        let name = match &self.name {
            Ok(n) => n.trim(),
            Err(e) => {
                return Finding::unknown(
                    CheckId::T12,
                    Severity::Warn,
                    T12_EVIDENCE,
                    "the kernel's clock source".to_owned(),
                    e,
                );
            }
        };
        match ClocksourceKind::of(name) {
            ClocksourceKind::Counter => Finding::ok(
                CheckId::T12,
                Severity::Warn,
                T12_EVIDENCE,
                format!("clocksource {name}: the hardware counter, read through the vDSO"),
            ),
            ClocksourceKind::Paravirt => Finding::found(
                CheckId::T12,
                Severity::Info,
                T12_EVIDENCE,
                format!(
                    "clocksource {name} is a hypervisor's scaled view of the counter, read \
                     through the vDSO where the hypervisor marks it stable"
                ),
            ),
            ClocksourceKind::Other if self.arch == Arch::Unknown => Finding::unknown(
                CheckId::T12,
                Severity::Warn,
                T12_EVIDENCE,
                "the kernel's clock source".to_owned(),
                format!(
                    "clocksource {name} on an architecture other than x86-64 or 64-bit Arm, or \
                     with no /proc/cpuinfo to tell: whether it is read through the vDSO is not \
                     known here"
                ),
            ),
            ClocksourceKind::Other => Finding::found(
                CheckId::T12,
                Severity::Warn,
                T12_EVIDENCE,
                format!(
                    "clocksource {name} is not the counter and has no vDSO read mode: every \
                     CLOCK_REALTIME and CLOCK_MONOTONIC read (each UTC stamp, Aeron's clocks) is \
                     a system call, which costs more than a vDSO read (not measured here). The \
                     kernel leaves the counter when its watchdog marks it unstable (`dmesg | grep \
                     -i clocksource` says why); a clock that reads the counter itself (quanta) \
                     is not told. {}",
                    self.fix()
                ),
            ),
        }
    }

    /// The fix for a clock source other than the counter, by architecture.
    const fn fix(&self) -> &'static str {
        match (self.arch, self.invariant_tsc) {
            (Arch::X86_64, Some(false)) => {
                "The TSC is not invariant here (no `constant_tsc nonstop_tsc` in /proc/cpuinfo), \
                 so the kernel's choice stands and forcing the TSC would be wrong."
            }
            // Not "boot with": forcing the TSC is a hardware decision, not a
            // fix for a source the kernel rejected. The invariance flags say
            // the counter will not stop or drift; they say nothing about
            // whether it agrees across cores, and `tsc=reliable` turns off
            // the kernel's checks of exactly that.
            (Arch::X86_64, _) => {
                "Do not force it as a fix: `constant_tsc nonstop_tsc` say the TSC will not stop \
                 or drift, not that it agrees across cores, and `tsc=reliable` turns off the \
                 kernel's TSC checks. Force `clocksource=tsc tsc=reliable` on the node only on \
                 hardware qualified as the same across cores against a PTP/PHC reference, and \
                 never on a VM."
            }
            (Arch::Aarch64, _) => {
                "On the node, boot with `clocksource=arch_sys_counter`, the generic timer the \
                 vDSO reads."
            }
            _ => {
                "Find why the kernel left the counter (`dmesg | grep -i clocksource`) before \
                 changing it on the node."
            }
        }
    }
}

/// Whether `/proc/cpuinfo`'s first `flags` line has both `constant_tsc` and
/// `nonstop_tsc`; `None` without one (not x86, or unreadable).
fn invariant_tsc(cpuinfo: &str) -> Option<bool> {
    let flags = cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "flags").then_some(value)
    })?;
    let has = |flag: &str| flags.split_whitespace().any(|f| f == flag);
    Some(has("constant_tsc") && has("nonstop_tsc"))
}

/// T12: the kernel's clock source on `host` ([`Clocksource::finding`]).
#[must_use]
pub fn t12_clocksource(host: &Host) -> Finding {
    Clocksource::read(host).finding()
}

/// A clock source's name held inline (at most 32 bytes, the kernel's
/// `CS_NAME_LEN`), so [`ClockWatch`](crate::ClockWatch) compares a fresh
/// read with nothing allocated.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClocksourceName {
    bytes: [u8; 32],
    len: u8,
}

impl ClocksourceName {
    /// `name`, trimmed; `None` when that is empty or longer than 32 bytes.
    #[must_use]
    pub fn new(name: &str) -> Option<Self> {
        let name = name.trim();
        let len = u8::try_from(name.len())
            .ok()
            .filter(|n| (1..=32).contains(n))?;
        let mut bytes = [0_u8; 32];
        bytes
            .get_mut(..name.len())?
            .copy_from_slice(name.as_bytes());
        Some(Self { bytes, len })
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.bytes.get(..usize::from(self.len)).unwrap_or_default())
            .unwrap_or_default()
    }
}

impl fmt::Debug for ClocksourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClocksourceName({:?})", self.as_str())
    }
}

impl fmt::Display for ClocksourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kinds, by name, whitespace aside.
    #[test]
    fn clock_sources_are_classified_by_name() {
        assert_eq!(ClocksourceKind::of("tsc\n"), ClocksourceKind::Counter);
        assert_eq!(
            ClocksourceKind::of("arch_sys_counter"),
            ClocksourceKind::Counter
        );
        assert_eq!(ClocksourceKind::of("kvm-clock"), ClocksourceKind::Paravirt);
        assert_eq!(ClocksourceKind::of("hpet"), ClocksourceKind::Other);
        assert_eq!(
            ClocksourceKind::of("arch_mem_counter"),
            ClocksourceKind::Other
        );
    }

    /// Invariance needs both flags, as whole words, on a `flags` line.
    #[test]
    fn an_invariant_tsc_needs_both_flags() {
        assert_eq!(
            invariant_tsc("flags\t\t: fpu tsc constant_tsc nonstop_tsc\n"),
            Some(true)
        );
        assert_eq!(invariant_tsc("flags\t: tsc constant_tsc\n"), Some(false));
        assert_eq!(
            invariant_tsc("flags\t: tsc constant_tsc_x nonstop_tsc\n"),
            Some(false)
        );
        assert_eq!(invariant_tsc("Features\t: fp asimd\n"), None);
    }

    /// A name inline: trimmed, at most 32 bytes, never empty.
    #[test]
    fn a_name_is_held_inline() {
        assert_eq!(
            ClocksourceName::new("tsc\n").map(|n| n.to_string()),
            Some("tsc".to_owned())
        );
        assert_eq!(ClocksourceName::new("  "), None);
        assert_eq!(ClocksourceName::new(&"x".repeat(33)), None);
        assert!(ClocksourceName::new(&"x".repeat(32)).is_some());
    }
}
