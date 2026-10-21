//! The bare counter reads, uncalibrated.
//!
//! For benchmarks and for code that keeps raw ticks (compared across processes on one host, or converted
//! later with a [`Scale`](crate::advanced::Scale)). An instruction not every CPU has
//! is behind a token that only detection makes.

use crate::arch;

/// `rdtsc`: the time-stamp counter, unordered.
#[cfg(target_arch = "x86_64")]
#[inline]
#[must_use]
pub fn rdtsc() -> u64 {
    arch::ticks()
}

/// `lfence; rdtsc`: ordered by a fence (every x86-64 CPU has LFENCE).
#[cfg(target_arch = "x86_64")]
#[inline]
#[must_use]
pub fn lfence_rdtsc() -> u64 {
    arch::lfence_ticks()
}

/// Proof that this CPU has `rdtscp` (CPUID `0x8000_0001` EDX bit 27).
#[cfg(target_arch = "x86_64")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rdtscp(());

#[cfg(target_arch = "x86_64")]
impl Rdtscp {
    /// The token, where CPUID reports the instruction (a CPUID: a VM exit in
    /// a guest; detect once).
    #[must_use]
    pub fn detect() -> Option<Self> {
        arch::rdtscp_supported().then_some(Self(()))
    }

    /// `rdtscp`: ordered (after every earlier instruction).
    #[inline]
    #[must_use]
    pub fn read(self) -> u64 {
        arch::ticks_ordered()
    }

    /// `rdtscp; lfence`: ordered both ways (later instructions wait too).
    #[inline]
    #[must_use]
    pub fn read_fenced(self) -> u64 {
        arch::ticks_ordered_fenced()
    }
}

/// `mrs cntvct_el0`: the generic timer's virtual counter, unordered.
#[cfg(target_arch = "aarch64")]
#[inline]
#[must_use]
pub fn cntvct() -> u64 {
    arch::ticks()
}

/// `isb; mrs cntvct_el0`: ordered.
#[cfg(target_arch = "aarch64")]
#[inline]
#[must_use]
pub fn isb_cntvct() -> u64 {
    arch::ticks_ordered()
}

/// `CNTFRQ_EL0`: the counter's frequency register, Hz (0 if never set).
#[cfg(target_arch = "aarch64")]
#[must_use]
pub fn cntfrq() -> u64 {
    arch::counter_frequency()
}

/// Proof that this CPU has `FEAT_ECV`, as the operating system reports it.
#[cfg(target_arch = "aarch64")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ecv(());

#[cfg(target_arch = "aarch64")]
impl Ecv {
    /// The token, where the operating system reports `FEAT_ECV`.
    #[must_use]
    pub fn detect() -> Option<Self> {
        crate::detect::ecv_supported().then_some(Self(()))
    }

    /// `mrs cntvctss_el0`: the self-synchronised read, ordered like
    /// `isb; mrs cntvct_el0` without the ISB.
    #[inline]
    #[must_use]
    pub fn read(self) -> u64 {
        arch::ticks_self_synchronised()
    }
}
