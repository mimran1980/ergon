//! Process set-up for a pinned busy-spin loop: thread affinity, locked
//! memory, and a check that the clock is the time-stamp counter. Linux only;
//! elsewhere each is a logged no-op.
#![allow(unsafe_code)]

/// Pin the calling thread to `cpu`.
pub fn pin_thread(cpu: usize) {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: `cpu_set_t` is plain data and all-zero is the empty set;
        // `CPU_SET` writes inside it, and `sched_setaffinity` reads exactly
        // `size_of::<cpu_set_t>()` bytes of it for the calling thread (0).
        let ok = unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_SET(cpu, &mut set);
            libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &raw const set) == 0
        };
        if ok {
            log::info!("loop pinned to CPU {cpu}");
        } else {
            log::error!(
                "could not pin the loop to CPU {cpu}: {}",
                std::io::Error::last_os_error()
            );
        }
    }
    #[cfg(not(target_os = "linux"))]
    log::warn!("CPU {cpu}: thread pinning is Linux only");
}

/// Lock every current and future page into memory, faulting them in now.
pub fn lock_memory() {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: `mlockall` takes flags only and touches no Rust memory.
        if unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } == 0 {
            log::info!("memory locked");
        } else {
            log::error!("mlockall: {}", std::io::Error::last_os_error());
        }
    }
    #[cfg(not(target_os = "linux"))]
    log::warn!("memory locking is Linux only");
}

/// Log loudly when Linux's clock source is not the TSC: every latency number
/// is then wrong or slow.
#[cfg_attr(not(target_os = "linux"), allow(clippy::missing_const_for_fn))]
pub fn check_clock() {
    #[cfg(target_os = "linux")]
    {
        let path = "/sys/devices/system/clocksource/clocksource0/current_clocksource";
        match std::fs::read_to_string(path) {
            Ok(source) if source.trim() == "tsc" => {}
            Ok(source) => log::error!(
                "clock source is {:?}, not tsc: latency numbers will be slow or wrong",
                source.trim()
            ),
            Err(e) => log::warn!("{path}: {e}"),
        }
    }
}
