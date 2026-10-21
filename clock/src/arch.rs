//! Every instruction and system call the crate needs, and the only module
//! allowed `unsafe` (the counter reads are inline assembly or intrinsics,
//! the system calls are FFI). Each function here is safe to call: its
//! precondition, where it has one, is stated and enforced by its callers'
//! construction (a `CounterClock` or a `raw` token exists only where the
//! instruction does).
//!
//! The reads come in two flavours, the same on both architectures:
//! - unordered (`rdtsc`, `mrs cntvct_el0`): may be taken before earlier
//!   instructions finish; nothing orders it but its data dependencies. The
//!   cheapest read, for event time, pacing, freshness and rate gates.
//! - ordered (`rdtscp`, `isb; mrs cntvct_el0`): taken after every earlier
//!   instruction has completed, and the asm is a compiler barrier too (no
//!   `nomem`), so the stamp stays where it is written: a measurement
//!   bracket's end.
#![allow(unsafe_code)]

/// The counter, unordered: `rdtsc`.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn ticks() -> u64 {
    // SAFETY: RDTSC exists on every x86-64 CPU and reads the time-stamp
    // counter only; user mode may execute it unless CR4.TSD is set, which
    // neither Linux nor macOS does.
    #[allow(unused_unsafe)]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
}

/// The counter, ordered: `rdtscp`, which waits until every earlier
/// instruction has executed and earlier loads are globally visible.
///
/// Requires RDTSCP (CPUID `0x8000_0001` EDX bit 27): callers exist only where
/// [`rdtscp_supported`] said so. Without it the instruction faults and the
/// process ends with SIGILL; nothing undefined happens first.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn ticks_ordered() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: RDTSCP reads the TSC into EDX:EAX and IA32_TSC_AUX into ECX
    // (clobbered here), touches no memory and no flags; its presence is the
    // caller's precondition (above). No `nomem`: a compiler barrier.
    unsafe {
        core::arch::asm!("rdtscp", out("eax") lo, out("edx") hi, out("ecx") _, options(nostack, preserves_flags));
    }
    (u64::from(hi) << 32) | u64::from(lo)
}

/// `lfence; rdtsc`: the read ordered by a fence instead (Intel's documented
/// alternative to `rdtscp`; LFENCE is SSE2, so on every x86-64 CPU).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn lfence_ticks() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: LFENCE orders this thread's instructions; RDTSC as in
    // `ticks`. No memory touched; a compiler barrier.
    unsafe {
        core::arch::asm!("lfence", "rdtsc", out("eax") lo, out("edx") hi, options(nostack, preserves_flags));
    }
    (u64::from(hi) << 32) | u64::from(lo)
}

/// `rdtscp; lfence`: the ordered read that also keeps later instructions
/// from starting before it (Intel's recommendation for a region's start).
/// Requires RDTSCP, as [`ticks_ordered`].
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn ticks_ordered_fenced() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: as `ticks_ordered`, then LFENCE.
    unsafe {
        core::arch::asm!(
            "rdtscp",
            "lfence",
            out("eax") lo,
            out("edx") hi,
            out("ecx") _,
            options(nostack, preserves_flags)
        );
    }
    (u64::from(hi) << 32) | u64::from(lo)
}

/// CPUID: `[eax, ebx, ecx, edx]` of `leaf` and `subleaf`. Callers check the
/// maximum leaf first (an out-of-range leaf returns another leaf's values).
/// In a virtual machine each call is a VM exit (microseconds): start-up only.
#[cfg(target_arch = "x86_64")]
pub fn cpuid(leaf: u32, subleaf: u32) -> [u32; 4] {
    // SAFETY: CPUID exists on every x86-64 CPU, reads identification
    // registers only, and is defined for every leaf.
    #[allow(unused_unsafe)]
    let r = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
    [r.eax, r.ebx, r.ecx, r.edx]
}

/// Whether the CPU has RDTSCP: CPUID `0x8000_0001` EDX bit 27.
#[cfg(target_arch = "x86_64")]
pub fn rdtscp_supported() -> bool {
    cpuid(0x8000_0000, 0)[0] >= 0x8000_0001 && cpuid(0x8000_0001, 0)[3] & (1 << 27) != 0
}

/// The counter, unordered: the generic timer's virtual count, `CNTVCT_EL0`.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn ticks() -> u64 {
    let ticks: u64;
    // SAFETY: Linux and macOS let EL0 read CNTVCT_EL0 (CNTKCTL_EL1.EL0VCTEN;
    // where an erratum makes Linux trap it, the kernel emulates the read,
    // which calibration detects by its cost). Touches no memory: `nomem`,
    // so the compiler may move it, as the CPU may (unlike x86's `rdtsc`,
    // which LLVM treats as touching memory and so does not move).
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack, preserves_flags));
    }
    ticks
}

/// The counter, ordered: `isb` then `CNTVCT_EL0`, as Linux's vDSO and DPDK's
/// `rte_rdtsc_precise` read it. ISB completes every earlier instruction
/// before the read issues.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn ticks_ordered() -> u64 {
    let ticks: u64;
    // SAFETY: ISB only synchronises this thread's instruction stream; the
    // read as in `ticks`. No `nomem`: a compiler barrier, as `rdtscp`.
    unsafe {
        core::arch::asm!("isb", "mrs {}, cntvct_el0", out(reg) ticks, options(nostack, preserves_flags));
    }
    ticks
}

/// `FEAT_ECV`'s self-synchronised view of the counter, `CNTVCTSS_EL0`
/// (encoded `S3_3_C14_C0_6`, so assemblers that do not know the name, or
/// know it only with the `ecv` feature, accept it): ordered like `isb; mrs`
/// without the ISB, as Linux's own counter read takes it in their place.
///
/// Requires `FEAT_ECV`: callers exist only where detection reported it (a
/// clock whose ordered read is `OrderedRead::Cntvctss`, the `raw::Ecv`
/// token). Without it the read is an undefined instruction and the process
/// ends with SIGILL; nothing undefined happens first.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn ticks_self_synchronised() -> u64 {
    let ticks: u64;
    // SAFETY: a system register read with the same EL0 access as
    // CNTVCT_EL0; its presence is the caller's precondition (above). A
    // compiler barrier, as the ordered read.
    unsafe {
        core::arch::asm!("mrs {}, S3_3_C14_C0_6", out(reg) ticks, options(nostack, preserves_flags));
    }
    ticks
}

/// The counter's frequency register, `CNTFRQ_EL0`: its low 32 bits (the
/// rest are reserved), in Hz; 0 where firmware never set it.
#[cfg(target_arch = "aarch64")]
pub fn counter_frequency() -> u64 {
    let hz: u64;
    // SAFETY: CNTFRQ_EL0 is readable from EL0 on Linux and macOS; no memory.
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) hz, options(nomem, nostack, preserves_flags));
    }
    hz & 0xffff_ffff
}

/// Linux's `AT_HWCAP2`: the kernel's second word of CPU feature bits, set
/// only for features every CPU has (0 where it gave none). Decoded by
/// `detect`.
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn hwcap2() -> u64 {
    // SAFETY: getauxval only reads the process's auxiliary vector.
    unsafe { libc::getauxval(libc::AT_HWCAP2) }
}

/// A macOS `hw.optional.*` flag: whether sysctl reads it as non-zero (an
/// absent or unreadable flag is no).
#[cfg(target_os = "macos")]
pub fn sysctl_flag(name: &core::ffi::CStr) -> bool {
    let mut value: libc::c_int = 0;
    let mut size = core::mem::size_of::<libc::c_int>();
    // SAFETY: the name is a NUL-terminated string; `value` and `size` are a
    // valid out-buffer and its length, as sysctlbyname requires.
    let rc = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&raw mut value).cast(),
            &raw mut size,
            core::ptr::null_mut(),
            0,
        )
    };
    rc == 0 && value != 0
}

/// No counter this crate reads on other architectures. Never called: no
/// `CounterClock` can be made there (calibration and `from_params` refuse).
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub const fn ticks() -> u64 {
    0
}

/// As [`ticks`]: never called on other architectures.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub const fn ticks_ordered() -> u64 {
    0
}

/// `clock_gettime(id)` in nanoseconds, or the error number. Through the
/// vDSO (no system call) for the clocks it serves.
pub fn clock_ns(id: libc::clockid_t) -> Result<u64, i32> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec for the call's duration.
    if unsafe { libc::clock_gettime(id, &raw mut ts) } != 0 {
        return Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0));
    }
    Ok(timespec_ns(&ts))
}

/// `clock_getres(id)` in nanoseconds (at least 1), or the error number.
pub fn clock_res_ns(id: libc::clockid_t) -> Result<u64, i32> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: as `clock_ns`.
    if unsafe { libc::clock_getres(id, &raw mut ts) } != 0 {
        return Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0));
    }
    Ok(timespec_ns(&ts).max(1))
}

/// The hot path's `clock_gettime(id)`: the clock was read successfully at
/// calibration, so it cannot fail now (a valid id and pointer); were it to,
/// the read is the last value the timespec held, zero.
#[inline]
pub fn clock_ns_unchecked(id: libc::clockid_t) -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: as `clock_ns`.
    unsafe { libc::clock_gettime(id, &raw mut ts) };
    timespec_ns(&ts)
}

/// Linux's `adjtimex` with `modes` 0: the kernel's NTP state, read only
/// (only a non-zero `modes` needs `CAP_SYS_TIME`), so any process may call
/// it. The clock state (`TIME_OK` 0 ... `TIME_ERROR` 5) and the `timex`, or
/// the error number.
#[cfg(target_os = "linux")]
pub fn adjtimex_read() -> Result<(i32, libc::timex), i32> {
    // SAFETY: `timex` is a C struct of integers and a `timeval`: all-zero is
    // a valid value of it, and `modes` 0 asks the kernel to change nothing.
    let mut tx: libc::timex = unsafe { core::mem::zeroed() };
    // SAFETY: `tx` is a valid, writable timex for the call's duration.
    let state = unsafe { libc::adjtimex(&raw mut tx) };
    if state < 0 {
        return Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0));
    }
    Ok((state, tx))
}

/// A timespec as nanoseconds; a negative time (none of the clocks read
/// here has one) as 0.
#[inline]
fn timespec_ns(ts: &libc::timespec) -> u64 {
    let secs = u64::try_from(ts.tv_sec).unwrap_or(0);
    let nanos = u64::try_from(ts.tv_nsec).unwrap_or(0);
    secs.wrapping_mul(1_000_000_000).wrapping_add(nanos)
}
