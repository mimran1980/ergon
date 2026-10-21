//! Clocks, CPU pinning and per-thread accounting.
//!
//! - [`realtime_ns`]: `CLOCK_REALTIME` through the vDSO (no syscall on a TSC
//!   or `arch_sys_counter` clocksource). Compare kernel receive timestamps
//!   (`SO_TIMESTAMPING`) with this.
//! - [`rdtsc`]: cheaper, but another clock domain: durations only.
#![allow(unsafe_code)]

/// `CLOCK_REALTIME` in nanoseconds.
#[must_use]
pub fn realtime_ns() -> i64 {
    clock_ns(libc::CLOCK_REALTIME)
}

/// `CLOCK_MONOTONIC` in nanoseconds.
#[must_use]
pub fn monotonic_ns() -> i64 {
    clock_ns(libc::CLOCK_MONOTONIC)
}

fn clock_ns(id: libc::clockid_t) -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec.
    unsafe { libc::clock_gettime(id, &raw mut ts) };
    ts.tv_sec * 1_000_000_000 + ts.tv_nsec
}

/// Raw counter: `rdtsc` on x86-64, `cntvct_el0` on aarch64, 0 elsewhere.
///
/// On aarch64 an `isb` precedes the read, as in the kernel's own
/// `arch_timer_read_cntvct_el0` (`arch/arm64/include/asm/arch_timer.h`): the
/// architecture lets a `CNTVCT_EL0` read run speculatively and out of order,
/// so without it two reads could come back in the wrong order and a
/// difference wrap to an enormous value. The `isb` makes each read wait for
/// the instructions before it; its cost is not measured here.
#[must_use]
#[inline]
pub fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: RDTSC has no preconditions on x86-64.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(target_arch = "aarch64")]
    {
        let v: u64;
        // SAFETY: ISB has no preconditions; CNTVCT_EL0 is readable from EL0 on
        // Linux and macOS.
        unsafe {
            core::arch::asm!("isb", "mrs {}, cntvct_el0", out(reg) v, options(nomem, nostack));
        };
        v
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        0
    }
}

/// The raw counter's architected rate in Hz, where it has one.
///
/// `CNTFRQ_EL0` on aarch64 (what the kernel's `arch_sys_counter` clocksource
/// itself runs on). `None` where the rate is not architected: x86-64, whose
/// TSC rate is measured.
#[must_use]
#[allow(clippy::missing_const_for_fn)] // const only where the body is compiled out
pub fn counter_frequency_hz() -> Option<u64> {
    #[cfg(target_arch = "aarch64")]
    {
        let hz: u64;
        // SAFETY: CNTFRQ_EL0 is readable from EL0 on Linux and macOS.
        unsafe { core::arch::asm!("mrs {}, cntfrq_el0", out(reg) hz, options(nomem, nostack)) };
        // Only the low 32 bits are defined; 0 means firmware never set it.
        Some(hz & 0xFFFF_FFFF).filter(|&hz| hz > 0)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        None
    }
}

/// [`rdtsc`] ticks per microsecond, cached.
///
/// The architected rate where there is one ([`counter_frequency_hz`],
/// aarch64), otherwise measured once against `CLOCK_MONOTONIC` over 20 ms (the
/// TSC). Call it before any timed loop so the first call does not land inside
/// one.
#[must_use]
pub fn ticks_per_us() -> f64 {
    static TPU: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *TPU.get_or_init(|| {
        if let Some(hz) = counter_frequency_hz() {
            #[allow(clippy::cast_precision_loss)]
            return hz as f64 / 1e6;
        }
        let (m0, t0) = (monotonic_ns(), rdtsc());
        while monotonic_ns() - m0 < 20_000_000 {
            std::hint::spin_loop();
        }
        let (m1, t1) = (monotonic_ns(), rdtsc());
        #[allow(clippy::cast_precision_loss)]
        let tpu = t1.wrapping_sub(t0) as f64 / ((m1 - m0) as f64 / 1000.0);
        tpu.max(1e-3)
    })
}

/// Pins the calling thread to one CPU (Linux; no-op elsewhere).
///
/// # Errors
/// `sched_setaffinity` fails.
#[allow(clippy::missing_const_for_fn)] // const only where the body is compiled out
pub fn pin_current_thread(cpu: usize) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: zeroed cpu_set_t is valid; CPU_SET stays in bounds for cpu < 1024.
        let rc = unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_SET(cpu, &mut set);
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw const set)
        };
        if rc != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = cpu;
    Ok(())
}

/// Voluntary and involuntary context switches of the calling thread (Linux;
/// zeros elsewhere). A voluntary switch means the thread slept in the kernel.
#[must_use]
#[allow(clippy::missing_const_for_fn)] // const only where the body is compiled out
pub fn thread_ctx_switches() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: zeroed rusage is a valid out-parameter.
        let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: valid pointer to a rusage.
        if unsafe { libc::getrusage(libc::RUSAGE_THREAD, &raw mut ru) } == 0 {
            return (
                u64::try_from(ru.ru_nvcsw).unwrap_or(0),
                u64::try_from(ru.ru_nivcsw).unwrap_or(0),
            );
        }
    }
    (0, 0)
}

/// User and system CPU time of the calling thread in clock ticks, from
/// `/proc/thread-self/stat` (zeros when unavailable).
#[must_use]
pub fn thread_cpu_ticks() -> (u64, u64) {
    let Ok(s) = std::fs::read_to_string("/proc/thread-self/stat") else {
        return (0, 0);
    };
    let Some(close) = s.rfind(')') else {
        return (0, 0);
    };
    let rest: Vec<&str> = s[close + 1..].split_whitespace().collect();
    let field = |i: usize| rest.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
    (field(11), field(12))
}
