//! The transport's clock: the CPU counter at the rate DPDK gives it (or the
//! one measured at start, where DPDK's is off), or `CLOCK_MONOTONIC`.
//!
//! The native transport reads the time on every send and poll (ARP
//! freshness, the transmit pacer, receive-poll gaps). Its hot path reads the
//! counter inline, `rdtsc` on `x86_64` and `mrs cntvct_el0` on `aarch64`, and
//! converts it with a 32.32 factor made when the transport starts:
//!
//! ```text
//! ns = base_ns + (int64_t)(((__int128)(int64_t)(ticks - base_ticks) * (int64_t)mult) >> 32)
//! ```
//!
//! The factor's rate is DPDK's (`rte_get_tsc_hz()`) unless it is more than
//! 100 ppm off the counter's rate measured against `CLOCK_MONOTONIC_RAW` at
//! start (25 ms, before the transport's threads start); then it is the
//! measurement ([`RateSource`]). DPDK 23.11 rounds its own x86 measurement to
//! 10 MHz, up to 0.2% off.
//!
//! The fallback is `CLOCK_MONOTONIC` through the same formula (a factor of
//! one). Either way the time is nanoseconds since boot, for intervals only:
//! the transport's media and NIC-arrival timestamps read `CLOCK_REALTIME`
//! themselves.
//!
//! [`CLOCK_ENV`] chooses: `counter` (the default; `tsc`, its earlier name, on
//! either architecture) or `monotonic`. The counter falls back to
//! `CLOCK_MONOTONIC` where the kernel's clock source is not the counter
//! ([`kernel_refuses_counter`]) or DPDK's rate cannot be trusted
//! ([`MonotonicReason`]). [`DpdkTransport::clock`](crate::DpdkTransport::clock)
//! says which clock the transport reads, and why.

use std::fmt;

use crate::error::DpdkError;

/// The variable choosing the transport's clock: `counter` (the default, or
/// `tsc`) or `monotonic`.
pub const CLOCK_ENV: &str = "RUSTERON_DPDK_CLOCK";

/// Where Linux names the clock source the kernel keeps time with.
pub const CLOCKSOURCE_PATH: &str = "/sys/devices/system/clocksource/clocksource0/current_clocksource";

/// Which clock the transport reads ([`CLOCK_ENV`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClockChoice {
    /// The CPU counter at DPDK's rate, where the transport can trust it;
    /// else `CLOCK_MONOTONIC` ([`MonotonicReason`] says why).
    #[default]
    Counter,
    /// `CLOCK_MONOTONIC`, whatever the counter (to compare the two).
    Monotonic,
}

impl ClockChoice {
    /// Reads [`CLOCK_ENV`]: [`ClockChoice::Counter`] when unset or empty, or
    /// `counter`, or `tsc`; [`ClockChoice::Monotonic`] for `monotonic`.
    ///
    /// # Errors
    ///
    /// [`DpdkError::InvalidEnvironment`] naming the variable for any other
    /// value.
    pub fn from_env() -> Result<Self, DpdkError> {
        match std::env::var(CLOCK_ENV) {
            Err(std::env::VarError::NotPresent) => Ok(Self::Counter),
            Ok(v) if v.is_empty() || v == "counter" || v == "tsc" => Ok(Self::Counter),
            Ok(v) if v == "monotonic" => Ok(Self::Monotonic),
            Ok(v) => Err(DpdkError::InvalidEnvironment(format!(
                "{CLOCK_ENV}={v:?} must be counter (or tsc) or monotonic"
            ))),
            Err(std::env::VarError::NotUnicode(v)) => Err(DpdkError::InvalidEnvironment(format!(
                "{CLOCK_ENV}={} must be counter (or tsc) or monotonic",
                v.display()
            ))),
        }
    }
}

/// The clock the transport reads ([`DpdkTransport::clock`](crate::DpdkTransport::clock)).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportClock {
    /// The CPU counter: the TSC on `x86_64`, `CNTVCT_EL0` on `aarch64`.
    Counter {
        /// The rate the transport converts it with, ticks a second:
        /// `eal_hz` or `measured_hz`, as `rate` says.
        hz: u64,
        /// Which rate `hz` is.
        rate: RateSource,
        /// DPDK's rate for it (`rte_get_tsc_hz()`).
        eal_hz: u64,
        /// Its rate measured against `CLOCK_MONOTONIC_RAW` when the
        /// transport started.
        measured_hz: u64,
    },
    /// `CLOCK_MONOTONIC`.
    Monotonic {
        /// Why not the counter.
        reason: MonotonicReason,
    },
}

/// Which rate the transport converts the counter with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateSource {
    /// DPDK's (`rte_get_tsc_hz()`): the start-up measurement agrees with it
    /// within 100 ppm, or is no usable rate (0, or outside 1 MHz to 20 GHz).
    Eal,
    /// The start-up measurement: DPDK's rate is more than 100 ppm off it.
    Measured,
}

/// Why the transport reads `CLOCK_MONOTONIC` and not the counter.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MonotonicReason {
    /// `RUSTERON_DPDK_CLOCK=monotonic`.
    Chosen,
    /// The kernel's clock source (named) is neither the counter nor a
    /// hypervisor's view of it: the kernel rejected the counter, or never
    /// took it ([`kernel_refuses_counter`]).
    KernelClocksource(String),
    /// DPDK gave the counter no rate the transport converts
    /// (`rte_get_tsc_hz()`, here `eal_hz`: 0, or outside 1 MHz to 20 GHz).
    NoCounterRate {
        /// What `rte_get_tsc_hz()` returned.
        eal_hz: u64,
    },
    /// `x86_64`: CPUID does not call the TSC invariant, so its rate may change
    /// with the CPU's frequency or stop in a sleep state.
    TscNotInvariant,
    /// `aarch64`: DPDK's rate is not the generic timer's (`CNTFRQ_EL0`): a
    /// DPDK built to time with the PMU's cycle counter
    /// (`RTE_ARM_EAL_RDTSC_USE_PMU`), not the counter the transport reads.
    RateIsNotCntfrq {
        /// What `rte_get_tsc_hz()` returned.
        eal_hz: u64,
        /// `CNTFRQ_EL0`.
        cntfrq: u64,
    },
}

impl fmt::Display for MonotonicReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chosen => write!(f, "{CLOCK_ENV}=monotonic"),
            Self::KernelClocksource(source) => write!(
                f,
                "the kernel's clock source is {source:?}, neither the counter nor a hypervisor's view of it"
            ),
            Self::NoCounterRate { eal_hz } => {
                write!(f, "DPDK gave the counter no usable rate (rte_get_tsc_hz() = {eal_hz})")
            }
            Self::TscNotInvariant => write!(f, "CPUID does not call the TSC invariant"),
            Self::RateIsNotCntfrq { eal_hz, cntfrq } => write!(
                f,
                "DPDK's rate {eal_hz} Hz is not CNTFRQ_EL0's {cntfrq} Hz (a DPDK built to time with the PMU)"
            ),
        }
    }
}

impl fmt::Display for TransportClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Counter {
                hz,
                rate: RateSource::Eal,
                measured_hz,
                ..
            } => write!(f, "the CPU counter at {hz} Hz, DPDK's rate (measured {measured_hz} Hz)"),
            Self::Counter {
                hz,
                rate: RateSource::Measured,
                eal_hz,
                ..
            } => write!(
                f,
                "the CPU counter at {hz} Hz, as measured (DPDK's {eal_hz} Hz is more than 100 ppm off)"
            ),
            Self::Monotonic { reason } => write!(f, "CLOCK_MONOTONIC: {reason}"),
        }
    }
}

/// Whether the kernel's clock source rules the counter out.
///
/// `clocksource` is the name Linux gives it ([`CLOCKSOURCE_PATH`]): `None`
/// when it is the counter (`tsc` on `x86_64`, `arch_sys_counter` on
/// `aarch64`) or a hypervisor's scaled view of it (`kvm-clock`,
/// `hyperv_clocksource_tsc_page`, `xen`); otherwise
/// [`MonotonicReason::KernelClocksource`]. Another source (`hpet`, `acpi_pm`)
/// is what Linux switches to when its watchdog finds the counter unstable.
#[must_use]
pub fn kernel_refuses_counter(clocksource: &str) -> Option<MonotonicReason> {
    let source = clocksource.trim();
    if source == COUNTER_CLOCKSOURCE || PARAVIRT_CLOCKSOURCES.contains(&source) {
        None
    } else {
        Some(MonotonicReason::KernelClocksource(source.to_owned()))
    }
}

/// This architecture's counter as Linux names it as a clock source.
const COUNTER_CLOCKSOURCE: &str = if cfg!(target_arch = "aarch64") {
    "arch_sys_counter"
} else {
    "tsc"
};

/// Clock sources that are a hypervisor's scaled view of the counter: KVM's
/// pvclock, Hyper-V's TSC page (Azure), Xen's.
const PARAVIRT_CLOCKSOURCES: [&str; 3] = ["kvm-clock", "hyperv_clocksource_tsc_page", "xen"];

/// The kernel's clock source as Linux names it ([`CLOCKSOURCE_PATH`]), or
/// `None` where it cannot be read.
#[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
pub(crate) fn kernel_clocksource() -> Option<String> {
    std::fs::read_to_string(CLOCKSOURCE_PATH).ok()
}

/// Starts the native clock through `start` and says which clock the
/// transport reads.
///
/// `start` is asked for `choice`, except that [`ClockChoice::Counter`] becomes
/// [`ClockChoice::Monotonic`] where the kernel's clock source (`clocksource`,
/// `None` when unreadable, which keeps the counter) rules the counter out
/// ([`kernel_refuses_counter`]). It returns what it started and the clock the
/// native side reports. The result is that clock, except that a
/// `CLOCK_MONOTONIC` the kernel forced gives the kernel's reason, not
/// [`MonotonicReason::Chosen`].
///
/// # Errors
///
/// `start`'s.
#[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
pub(crate) fn start_clock<T, E>(
    choice: ClockChoice,
    clocksource: Option<&str>,
    start: impl FnOnce(ClockChoice) -> Result<(T, TransportClock), E>,
) -> Result<(T, TransportClock), E> {
    let kernel = match choice {
        ClockChoice::Counter => clocksource.and_then(kernel_refuses_counter),
        ClockChoice::Monotonic => None,
    };
    let asked = if kernel.is_some() {
        ClockChoice::Monotonic
    } else {
        choice
    };
    let (started, native) = start(asked)?;
    let clock = match (native, kernel) {
        (TransportClock::Monotonic { .. }, Some(reason)) => TransportClock::Monotonic { reason },
        (native, _) => native,
    };
    Ok((started, clock))
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::{ClockChoice, MonotonicReason, RateSource, TransportClock, start_clock};

    /// This architecture's counter as Linux names it as a clock source.
    const COUNTER: &str = if cfg!(target_arch = "aarch64") {
        "arch_sys_counter\n"
    } else {
        "tsc\n"
    };

    const fn counter(hz: u64) -> TransportClock {
        TransportClock::Counter {
            hz,
            rate: RateSource::Eal,
            eal_hz: hz,
            measured_hz: hz,
        }
    }

    const fn monotonic(reason: MonotonicReason) -> TransportClock {
        TransportClock::Monotonic { reason }
    }

    /// What the native side is asked for, and what the transport reports,
    /// when the native side reports `native`.
    fn decide(
        choice: ClockChoice,
        clocksource: Option<&str>,
        native: &TransportClock,
    ) -> (ClockChoice, TransportClock) {
        let Ok(decided) = start_clock::<_, Infallible>(choice, clocksource, |asked| Ok((asked, native.clone())));
        decided
    }

    /// The kernel keeps time with the counter, or a hypervisor's view of it:
    /// the native side is asked for the counter, and whatever it reports
    /// (the counter, or why not) is the transport's clock.
    #[test]
    fn a_kernel_on_the_counter_leaves_the_choice_to_the_native_side() {
        for source in [COUNTER, "kvm-clock\n", "hyperv_clocksource_tsc_page\n", "xen\n"] {
            for native in [
                counter(3_000_000_000),
                monotonic(MonotonicReason::NoCounterRate { eal_hz: 0 }),
                monotonic(MonotonicReason::TscNotInvariant),
            ] {
                assert_eq!(
                    decide(ClockChoice::Counter, Some(source), &native),
                    (ClockChoice::Counter, native.clone()),
                    "{source:?}"
                );
            }
        }
    }

    /// The kernel left the counter (`hpet`, `acpi_pm`): the native side is
    /// asked for `CLOCK_MONOTONIC`, and the transport reports the kernel's
    /// clock source as the reason, not the choice the native side saw.
    #[test]
    fn a_kernel_off_the_counter_asks_for_clock_monotonic_and_names_its_source() {
        for source in ["hpet", "acpi_pm"] {
            assert_eq!(
                decide(
                    ClockChoice::Counter,
                    Some(&format!("{source}\n")),
                    &monotonic(MonotonicReason::Chosen)
                ),
                (
                    ClockChoice::Monotonic,
                    monotonic(MonotonicReason::KernelClocksource(source.to_string()))
                ),
                "{source}"
            );
        }
    }

    /// An unreadable clock source (no sysfs) rules nothing out: the counter
    /// is asked for, and the native side's report stands.
    #[test]
    fn an_unreadable_clocksource_keeps_the_counter() {
        for native in [
            counter(24_000_000),
            monotonic(MonotonicReason::NoCounterRate { eal_hz: 0 }),
        ] {
            assert_eq!(
                decide(ClockChoice::Counter, None, &native),
                (ClockChoice::Counter, native.clone())
            );
        }
    }

    /// `RUSTERON_DPDK_CLOCK=monotonic` is asked for whatever the kernel's
    /// clock source, and reported as the choice, not the kernel's.
    #[test]
    fn the_monotonic_choice_wins_over_the_kernel_clocksource() {
        for source in [Some(COUNTER), Some("hpet\n"), None] {
            assert_eq!(
                decide(ClockChoice::Monotonic, source, &monotonic(MonotonicReason::Chosen)),
                (ClockChoice::Monotonic, monotonic(MonotonicReason::Chosen)),
                "{source:?}"
            );
        }
    }

    /// The native side's failure is the result, as it gave it.
    #[test]
    fn a_failed_start_is_returned() {
        let result: Result<((), TransportClock), &str> =
            start_clock(ClockChoice::Counter, Some("hpet\n"), |_| Err("no"));
        assert_eq!(result, Err("no"));
    }
}
