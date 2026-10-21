//! What the host says about its counter, and the rule that decides from it
//! whether the counter may be the clock.

use crate::arch;
use crate::timesync::{Check, Finding, Severity};
use std::fmt;

/// Where Linux names the clock source it keeps time with.
pub const CLOCKSOURCE: &str = "/sys/devices/system/clocksource/clocksource0/current_clocksource";

/// The kernel's TSC rate in kHz, where a kernel exposes it (not mainline;
/// some vendor kernels do).
#[cfg(target_arch = "x86_64")]
const TSC_FREQ_KHZ: &str = "/sys/devices/system/cpu/cpu0/tsc_freq_khz";

/// The CPU architecture, as far as the counter is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    /// x86-64: the time-stamp counter.
    X86_64,
    /// 64-bit Arm: the generic timer's virtual counter.
    Aarch64,
    /// Anything else: no counter this crate reads.
    Other,
}

/// The operating system, as far as vouching for the counter is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// Linux: names its clock source in sysfs.
    Linux,
    /// macOS: no clock source to read.
    MacOs,
    /// Anything else.
    Other,
}

/// How much evidence the counter needs before it is the clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Trust {
    /// Only where the kernel keeps time with the counter itself (Linux's
    /// clock source `tsc` on x86, `arch_sys_counter` on Arm): the kernel has
    /// checked it, and watches it (its clocksource watchdog). The default.
    #[default]
    Kernel,
    /// Also where the kernel keeps time with a hypervisor's scaled view of
    /// the same counter (`kvm-clock`, Azure's `hyperv_clocksource_tsc_page`,
    /// `xen`) or names no clock source (macOS): calibration's own
    /// measurement and verification against the raw clock are the evidence.
    CrossCheck,
}

impl Trust {
    /// The name the report prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::CrossCheck => "cross-check",
        }
    }
}

/// Why the clock is the monotonic fallback and not the counter.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fallback {
    /// Asked for (`Policy::Monotonic`).
    Forced,
    /// No counter this crate reads on this architecture.
    NoCounter,
    /// x86: the TSC is not invariant (CPUID `0x8000_0007` EDX bit 8): its
    /// rate may change with frequency or stop in deep sleep states.
    TscNotInvariant,
    /// x86: no `rdtscp` (a hypervisor may hide it), so no ordered read.
    NoRdtscp,
    /// The kernel keeps time with this clock source, which is not the
    /// counter: it judged the counter unusable (or never had one).
    KernelClocksource(String),
    /// The kernel keeps time with a hypervisor's clock (named): the counter
    /// only with [`Trust::CrossCheck`].
    Paravirt(String),
    /// Linux named no clock source (sysfs unreadable): the counter only with
    /// [`Trust::CrossCheck`].
    ClocksourceUnreadable,
    /// An operating system with no clock source to read (macOS): the
    /// counter only with [`Trust::CrossCheck`].
    NoKernelVouch,
    /// A counter read cost this much (ns, the mean of many): the kernel
    /// traps and emulates it (an erratum workaround, a hypervisor).
    Trapped {
        /// Mean cost of one read, ns.
        read_ns: u64,
    },
    /// The counter did not advance over the calibration window.
    CounterStalled,
    /// The measured rate, Hz, is outside 1 kHz to 100 GHz.
    RateOutOfRange {
        /// The measured rate, Hz.
        hz: u64,
    },
    /// A second measurement disagreed with the calibrated rate by this
    /// much (ppm, rounded): the counter's rate is not steady.
    Unstable {
        /// The disagreement, ppm.
        ppm: i64,
    },
}

impl fmt::Display for Fallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forced => f.write_str("forced"),
            Self::NoCounter => f.write_str("no-counter"),
            Self::TscNotInvariant => f.write_str("tsc-not-invariant"),
            Self::NoRdtscp => f.write_str("no-rdtscp"),
            Self::KernelClocksource(name) => write!(f, "kernel-clocksource:{name}"),
            Self::Paravirt(name) => write!(f, "paravirt:{name}"),
            Self::ClocksourceUnreadable => f.write_str("clocksource-unreadable"),
            Self::NoKernelVouch => f.write_str("no-kernel-vouch"),
            Self::Trapped { read_ns } => write!(f, "trapped:{read_ns}ns"),
            Self::CounterStalled => f.write_str("counter-stalled"),
            Self::RateOutOfRange { hz } => write!(f, "rate-out-of-range:{hz}Hz"),
            Self::Unstable { ppm } => write!(f, "unstable:{ppm}ppm"),
        }
    }
}

/// Where a candidate rate for the counter came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateSource {
    /// Arm's frequency register, `CNTFRQ_EL0`: the rate the kernel's own
    /// clocks scale the counter by.
    Register,
    /// The kernel's TSC rate from sysfs (`tsc_freq_khz`, where exposed).
    KernelTscKhz,
    /// CPUID leaf 0x15: the crystal clock times the TSC/crystal ratio.
    CpuidCrystal,
    /// The hypervisor's timing leaf, CPUID `0x4000_0010` (TSC kHz).
    Hypervisor,
    /// CPUID leaf 0x16: the base frequency, MHz.
    CpuidBase,
    /// Measured against `CLOCK_MONOTONIC_RAW` (no candidate agreed).
    Measured,
    /// No counter: the fallback's nanoseconds.
    None,
}

impl RateSource {
    /// The name the report and the benchmarks print.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::KernelTscKhz => "kernel-tsc-khz",
            Self::CpuidCrystal => "cpuid-crystal",
            Self::Hypervisor => "hypervisor",
            Self::CpuidBase => "cpuid-base",
            Self::Measured => "measured",
            Self::None => "none",
        }
    }
}

/// The counter's ordered read: the instruction a measurement bracket's end
/// takes.
///
/// Chosen once from the host's facts ([`HostFacts::ordered_read`]) and kept
/// in the clock, so a read never consults anything global.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderedRead {
    /// x86-64: `rdtscp`, which waits for every earlier instruction.
    Rdtscp,
    /// 64-bit Arm: `isb` then `mrs cntvct_el0`, as Linux's vDSO reads the
    /// counter without `FEAT_ECV`.
    IsbCntvct,
    /// 64-bit Arm with `FEAT_ECV`: `mrs cntvctss_el0`, the self-synchronised
    /// view of the same counter, ordered as `isb; mrs cntvct_el0` is without
    /// the ISB. Linux reads it in their place wherever every CPU has it.
    Cntvctss,
    /// No counter this crate reads on this architecture.
    None,
}

impl OrderedRead {
    /// The instruction, as the report and the benchmarks print it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rdtscp => "rdtscp",
            Self::IsbCntvct => "isb; mrs cntvct_el0",
            Self::Cntvctss => "mrs cntvctss_el0",
            Self::None => "none",
        }
    }
}

/// What the host says about its counter, read once at calibration (CPUID,
/// system registers, sysfs). Each fact is `None` where it does not apply or
/// could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFacts {
    /// The architecture compiled for.
    pub arch: Arch,
    /// The operating system compiled for.
    pub os: Os,
    /// Linux's `current_clocksource`, trimmed.
    pub clocksource: Option<String>,
    /// x86: CPUID's invariant-TSC bit (`0x8000_0007` EDX bit 8).
    pub invariant_tsc: Option<bool>,
    /// x86: CPUID's RDTSCP bit (`0x8000_0001` EDX bit 27).
    pub rdtscp: Option<bool>,
    /// x86: running under a hypervisor (CPUID 1 ECX bit 31).
    pub hypervisor: Option<bool>,
    /// Arm: `FEAT_ECV`, as the operating system reports it.
    pub ecv: Option<bool>,
    /// Arm: `CNTFRQ_EL0`, Hz (0 where firmware left it unset).
    pub register_hz: Option<u64>,
    /// x86: nominal TSC rates from the kernel and CPUID, best evidence
    /// first; calibration takes the first its measurement cannot tell apart
    /// from the measured rate.
    pub nominal: Vec<(RateSource, u64)>,
}

impl HostFacts {
    /// A host about which nothing but its architecture and OS is known:
    /// the start for a host described by hand.
    #[must_use]
    pub const fn unknown(arch: Arch, os: Os) -> Self {
        Self {
            arch,
            os,
            clocksource: None,
            invariant_tsc: None,
            rdtscp: None,
            hypervisor: None,
            ecv: None,
            register_hz: None,
            nominal: Vec::new(),
        }
    }

    /// This host's facts. On x86 each CPUID is a VM exit in a guest
    /// (microseconds): call it at start-up, never on a hot path.
    #[must_use]
    pub fn detect() -> Self {
        let mut facts = Self::unknown(THIS_ARCH, THIS_OS);
        facts.clocksource = std::fs::read_to_string(CLOCKSOURCE)
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        facts.ecv = ecv_fact();
        #[cfg(target_arch = "x86_64")]
        x86_facts(&mut facts);
        #[cfg(target_arch = "aarch64")]
        {
            facts.register_hz = Some(arch::counter_frequency());
        }
        facts
    }

    /// The counter's ordered read on this host: `rdtscp` on x86-64; on Arm
    /// `mrs cntvctss_el0` where the operating system reports `FEAT_ECV`, else
    /// `isb; mrs cntvct_el0` (also where it is not known).
    #[must_use]
    pub const fn ordered_read(&self) -> OrderedRead {
        match self.arch {
            Arch::X86_64 => OrderedRead::Rdtscp,
            Arch::Aarch64 if matches!(self.ecv, Some(true)) => OrderedRead::Cntvctss,
            Arch::Aarch64 => OrderedRead::IsbCntvct,
            Arch::Other => OrderedRead::None,
        }
    }

    /// Whether the counter may be the clock on this host, with this much
    /// trust, and if not why: the rule [`calibrate`](crate::advanced::calibrate)
    /// applies before it measures anything. A counter that passes is still
    /// measured (its read cost, its rate, a verification) before it is used.
    ///
    /// # Errors
    /// The reason the counter may not be used.
    pub fn gate(&self, trust: Trust) -> Result<(), Fallback> {
        self.gate_with(self.clocksource.as_deref(), trust)
    }

    /// [`gate`](Self::gate) with the kernel's clock source given (a
    /// recheck's, read after calibration).
    pub(crate) fn gate_with(
        &self,
        clocksource: Option<&str>,
        trust: Trust,
    ) -> Result<(), Fallback> {
        let counter_source = match self.arch {
            Arch::Other => return Err(Fallback::NoCounter),
            Arch::X86_64 => {
                if self.invariant_tsc != Some(true) {
                    return Err(Fallback::TscNotInvariant);
                }
                if self.rdtscp != Some(true) {
                    return Err(Fallback::NoRdtscp);
                }
                "tsc"
            }
            Arch::Aarch64 => "arch_sys_counter",
        };
        if self.os != Os::Linux {
            return match trust {
                Trust::CrossCheck => Ok(()),
                Trust::Kernel => Err(Fallback::NoKernelVouch),
            };
        }
        let Some(source) = clocksource.map(str::trim) else {
            return match trust {
                Trust::CrossCheck => Ok(()),
                Trust::Kernel => Err(Fallback::ClocksourceUnreadable),
            };
        };
        match source_kind(counter_source, source) {
            SourceKind::Counter => Ok(()),
            SourceKind::Paravirt => match trust {
                Trust::CrossCheck => Ok(()),
                Trust::Kernel => Err(Fallback::Paravirt(source.to_owned())),
            },
            SourceKind::Other => Err(Fallback::KernelClocksource(source.to_owned())),
        }
    }

    /// What the report finds about the kernel's clock source, `clocksource`
    /// as Linux names it (`facts.clocksource` at calibration, or a fresh
    /// read from housekeeping): nothing where it is the counter; info for a
    /// hypervisor's view of it or an unreadable name; a warning for any
    /// other source, which the kernel's clocks are not read through the vDSO
    /// with. The gate's classification, not a second one; nothing off Linux
    /// or on an architecture without a counter this crate reads.
    #[must_use]
    pub fn clocksource_finding(&self, clocksource: Option<&str>) -> Option<Finding> {
        let counter_source = match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) => "tsc",
            (Os::Linux, Arch::Aarch64) => "arch_sys_counter",
            _ => return None,
        };
        let Some(source) = clocksource.map(str::trim).filter(|s| !s.is_empty()) else {
            return Some(Finding::new(
                Check::Clocksource,
                Severity::Info,
                format!("clock source unknown: {CLOCKSOURCE} could not be read"),
            ));
        };
        let monotonic = crate::wall::MONOTONIC_NAME;
        match source_kind(counter_source, source) {
            SourceKind::Counter => None,
            SourceKind::Paravirt => Some(Finding::new(
                Check::Clocksource,
                Severity::Info,
                format!(
                    "clocksource {source} is a hypervisor's scaled view of the counter, read through \
                     the vDSO where the hypervisor marks it stable; ergon-clock takes the counter \
                     itself only with `ergon_clock::advanced::Trust::CrossCheck` (by default it \
                     reads {monotonic}: fallback paravirt:{source})"
                ),
            )),
            SourceKind::Other => {
                // clock/BENCHMARKS.md, read cost: median of batch means
                // (slowest batch), with the counter as the clock source
                let (evidence, fix) = match self.arch {
                    Arch::X86_64 => (
                        "on c7i with the tsc source the counter clock read in 15.63 ns (slowest \
                         batch 16.68) against 25.01 (26.06) for CLOCK_MONOTONIC through the vDSO",
                        if self.invariant_tsc == Some(false) {
                            "The TSC is not invariant here, so the kernel's choice stands and \
                             forcing the TSC would be wrong."
                        } else {
                            // Not "boot with": forcing the TSC is a hardware decision, not a
                            // fix for a source the kernel rejected. The invariance flags say
                            // the counter will not stop or drift; they say nothing about
                            // whether it agrees across cores. `tsc=reliable` also turns off
                            // the kernel's TSC verification, and the calibration this finding
                            // is talking about still trusts the kernel's chosen source.
                            "Do not force it as a fix. `constant_tsc nonstop_tsc` say the counter \
                             will not stop or drift; they do not say it agrees across cores. \
                             `tsc=reliable` also turns off the kernel's TSC checks, and the \
                             calibration above still trusts the kernel's source. Force \
                             `clocksource=tsc tsc=reliable` only on hardware qualified as the \
                             same across cores against a PTP/PHC reference: record that choice, \
                             keep that check as health evidence, and never on a VM."
                        },
                    ),
                    _ => (
                        "on c8g with the arch_sys_counter source the counter clock read in \
                         12.15 ns (slowest batch 12.36) against 30.92 (31.13) for \
                         CLOCK_MONOTONIC through the vDSO",
                        "Boot with `clocksource=arch_sys_counter`, the generic timer the vDSO \
                         and ergon-clock read.",
                    ),
                };
                Some(Finding::new(
                    Check::Clocksource,
                    Severity::Warn,
                    format!(
                        "clocksource {source} is not the counter and is not read through the \
                         vDSO: every clock_gettime is a system call, and ergon-clock reads \
                         {monotonic} (fallback kernel-clocksource:{source}); {evidence} \
                         (clock/BENCHMARKS.md), and a system call costs more (not measured \
                         here). {fix}"
                    ),
                ))
            }
        }
    }
}

/// What a clock source is to the counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceKind {
    /// The counter itself (`tsc`, `arch_sys_counter`).
    Counter,
    /// A hypervisor's scaled view of it ([`PARAVIRT`]).
    Paravirt,
    /// Anything else (`hpet`, `acpi_pm`, `jiffies`): not read through the
    /// vDSO.
    Other,
}

/// `source` against the architecture's counter source.
fn source_kind(counter_source: &str, source: &str) -> SourceKind {
    if source == counter_source {
        SourceKind::Counter
    } else if PARAVIRT.contains(&source) {
        SourceKind::Paravirt
    } else {
        SourceKind::Other
    }
}

/// Clock sources that are a hypervisor's scaled view of the hardware
/// counter: KVM's pvclock, Hyper-V's TSC page (Azure), Xen's.
const PARAVIRT: [&str; 3] = ["kvm-clock", "hyperv_clocksource_tsc_page", "xen"];

/// The architecture compiled for.
const THIS_ARCH: Arch = if cfg!(target_arch = "x86_64") {
    Arch::X86_64
} else if cfg!(target_arch = "aarch64") {
    Arch::Aarch64
} else {
    Arch::Other
};

/// The operating system compiled for.
const THIS_OS: Os = if cfg!(target_os = "linux") {
    Os::Linux
} else if cfg!(target_os = "macos") {
    Os::MacOs
} else {
    Os::Other
};

/// This process's ordered read, from the one fact it depends on (Arm's
/// `FEAT_ECV`: the auxiliary vector or a sysctl; no CPUID, no sysfs): for a
/// clock adopted from another calibration's parameters, which may have
/// been made on another CPU.
pub fn this_cpu_ordered_read() -> OrderedRead {
    HostFacts {
        ecv: ecv_fact(),
        ..HostFacts::unknown(THIS_ARCH, THIS_OS)
    }
    .ordered_read()
}

/// [`HostFacts::ecv`] for this CPU: `None` off Arm.
#[allow(clippy::missing_const_for_fn, clippy::unnecessary_wraps)] // None, and const, off Arm
fn ecv_fact() -> Option<bool> {
    #[cfg(target_arch = "aarch64")]
    {
        Some(ecv_supported())
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        None
    }
}

/// Whether this CPU has `FEAT_ECV`, as the operating system reports it:
/// Linux's `HWCAP2_ECV`, macOS's `hw.optional.arm.FEAT_ECV`; elsewhere no
/// (the instruction is never guessed at). One read of the auxiliary vector
/// or one sysctl: start-up only, never a hot path.
#[cfg(target_arch = "aarch64")]
pub fn ecv_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        ecv_in_hwcap2(arch::hwcap2())
    }
    #[cfg(target_os = "macos")]
    {
        arch::sysctl_flag(c"hw.optional.arm.FEAT_ECV")
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Linux's `HWCAP2_ECV`, bit 19 of `AT_HWCAP2`
/// (`arch/arm64/include/uapi/asm/hwcap.h`): "functionality implied by
/// `ID_AA64MMFR0_EL1.ECV == 0b0001`" (`Documentation/arch/arm64/elf_hwcaps.rst`),
/// set only when every CPU has it.
#[cfg(any(test, all(target_os = "linux", target_arch = "aarch64")))]
const HWCAP2_ECV: u64 = 1 << 19;

/// Whether Linux's `AT_HWCAP2` reports `FEAT_ECV`.
#[cfg(any(test, all(target_os = "linux", target_arch = "aarch64")))]
const fn ecv_in_hwcap2(hwcap2: u64) -> bool {
    hwcap2 & HWCAP2_ECV != 0
}

/// Linux's current clock source, trimmed, to `f`: `None` where there is no
/// such file (macOS) or it cannot be read. Read into a buffer on the stack
/// (a clock source's name is at most 32 bytes, the kernel's `CS_NAME_LEN`),
/// so nothing is allocated; still a file read, so a cold path
/// (housekeeping), never a hot one.
pub fn with_clocksource<R>(f: impl FnOnce(Option<&str>) -> R) -> R {
    use std::io::Read;
    let mut buf = [0_u8; 64];
    let len = std::fs::File::open(CLOCKSOURCE)
        .and_then(|mut file| file.read(&mut buf))
        .unwrap_or(0);
    let name = std::str::from_utf8(buf.get(..len).unwrap_or_default())
        .ok()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    f(name)
}

/// x86's CPUID facts and nominal TSC rates.
#[cfg(target_arch = "x86_64")]
fn x86_facts(facts: &mut HostFacts) {
    let max_basic = arch::cpuid(0, 0)[0];
    let max_extended = arch::cpuid(0x8000_0000, 0)[0];
    facts.invariant_tsc =
        Some(max_extended >= 0x8000_0007 && arch::cpuid(0x8000_0007, 0)[3] & (1 << 8) != 0);
    facts.rdtscp = Some(arch::rdtscp_supported());
    let hypervisor = max_basic >= 1 && arch::cpuid(1, 0)[2] & (1 << 31) != 0;
    facts.hypervisor = Some(hypervisor);
    if let Some(khz) = std::fs::read_to_string(TSC_FREQ_KHZ)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&k| k > 0)
    {
        facts
            .nominal
            .push((RateSource::KernelTscKhz, khz.saturating_mul(1_000)));
    }
    if max_basic >= 0x15 {
        let [denominator, numerator, crystal, _] = arch::cpuid(0x15, 0);
        if denominator != 0 && numerator != 0 && crystal != 0 {
            let hz = u64::from(crystal) * u64::from(numerator) / u64::from(denominator);
            facts.nominal.push((RateSource::CpuidCrystal, hz));
        }
    }
    if hypervisor && arch::cpuid(0x4000_0000, 0)[0] >= 0x4000_0010 {
        let khz = arch::cpuid(0x4000_0010, 0)[0];
        if khz != 0 {
            facts
                .nominal
                .push((RateSource::Hypervisor, u64::from(khz) * 1_000));
        }
    }
    if max_basic >= 0x16 {
        let mhz = arch::cpuid(0x16, 0)[0] & 0xffff;
        if mhz != 0 {
            facts
                .nominal
                .push((RateSource::CpuidBase, u64::from(mhz) * 1_000_000));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Linux reports `FEAT_ECV` as bit 19 of `AT_HWCAP2`, and that bit alone
    /// decides: not its neighbours (`HWCAP2_MTE`, 18; `HWCAP2_AFP`, 20), not
    /// every other bit set.
    #[test]
    fn ecv_is_bit_19_of_hwcap2() {
        assert_eq!(HWCAP2_ECV, 0x8_0000);
        assert!(!ecv_in_hwcap2(0));
        assert!(ecv_in_hwcap2(1 << 19));
        assert!(!ecv_in_hwcap2((1 << 18) | (1 << 20)));
        assert!(!ecv_in_hwcap2(!(1 << 19)));
        assert!(ecv_in_hwcap2(u64::MAX));
    }

    /// The clock source read into a buffer on the stack is the one
    /// detection reads (that the read allocates nothing is
    /// `tests/recheck.rs`'s).
    #[test]
    fn the_clock_source_read_on_the_stack_is_the_one_detection_reads() {
        let read = with_clocksource(|name| name.map(str::to_owned));
        assert_eq!(read, HostFacts::detect().clocksource);
    }
}
