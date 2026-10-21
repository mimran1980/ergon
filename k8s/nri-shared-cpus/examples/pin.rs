// SPDX-License-Identifier: Apache-2.0
//! Pin a hot thread by index into `ERGON_ISOLATED_CPUS`. Leave helpers alone:
//! every thread already starts on the shared CPUs.
//!
//! Build:  rustc -O -o pin pin.rs
//! Run:    in a container that asked for `ergon.io/shared-millicpus`.

use std::io::{Error, ErrorKind};

#[repr(C)]
struct CpuSet([u64; 16]); // cpu_set_t: 1024 bits (glibc and musl)

unsafe extern "C" {
    fn sched_setaffinity(pid: i32, size: usize, mask: *const CpuSet) -> i32;
    fn sched_getcpu() -> i32;
}

/// Pin the calling thread to isolated CPU index `i`; returns the CPU.
pub fn pin_isolated(i: usize) -> Result<usize, Error> {
    let list = std::env::var("ERGON_ISOLATED_CPUS")
        .map_err(|_| Error::new(ErrorKind::NotFound, "ERGON_ISOLATED_CPUS is not set"))?;
    let cpu = list
        .split(',')
        .nth(i)
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&cpu| cpu < 1024)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("no isolated CPU {i} in {list}"),
            )
        })?;
    let mut set = CpuSet([0; 16]);
    set.0[cpu / 64] |= 1 << (cpu % 64);
    // SAFETY: `set` is a valid 128-byte cpu_set_t for the whole call.
    if unsafe { sched_setaffinity(0, size_of::<CpuSet>(), &set) } != 0 {
        return Err(Error::last_os_error());
    }
    Ok(cpu)
}

fn main() {
    let iso = std::env::var("ERGON_ISOLATED_CPUS").unwrap_or_else(|_| "(unset)".into());
    let shared = std::env::var("ERGON_SHARED_CPUS").unwrap_or_else(|_| "(unset)".into());
    println!("ERGON_ISOLATED_CPUS={iso}");
    println!("ERGON_SHARED_CPUS={shared}");

    // Every thread starts on the shared CPUs.
    println!("helper (default) is on cpu {}", unsafe { sched_getcpu() });

    match pin_isolated(0) {
        Ok(cpu) => println!("pinned the hot thread to isolated index 0 = cpu {cpu}"),
        Err(e) => {
            eprintln!("pin_isolated(0) failed: {e}");
            std::process::exit(1);
        }
    }
    println!("hot (pinned) is on cpu {}", unsafe { sched_getcpu() });

    if let Ok(cpu1) = pin_isolated(1) {
        println!("isolated index 1 = cpu {cpu1}");
        println!("hot2 (pinned) is on cpu {}", unsafe { sched_getcpu() });
    }
}
