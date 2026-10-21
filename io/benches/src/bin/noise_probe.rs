//! Platform noise on one CPU: a pinned loop that only reads the counter and
//! logs every gap. It makes no syscalls and does no work, so whatever stalls
//! it is the kernel (visible in a trace of that CPU) or something below the
//! kernel (the hypervisor or firmware).
//!
//! ```text
//! noise_probe --cpu 2 --duration 30 --out DIR [--stall-us 5]
//! noise_probe --cpu 3 --duration 600 --load       (spin only: a busy sibling)
//! ```

use std::process::ExitCode;

use io_bench::args::Args;
use io_bench::clock::{counter_meta, monotonic_ns, pin_current_thread};
use io_bench::stalls::StallLog;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("noise-probe: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::from_env();
    let cpu: usize = a.req("cpu")?.parse()?;
    pin_current_thread(cpu)?;
    let duration_s: i64 = a.num_or("duration", 30)?;
    let load_only = a.flag("load");
    #[cfg(target_os = "linux")]
    // SAFETY: plain syscall with valid flags.
    let locked = unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } == 0;
    #[cfg(not(target_os = "linux"))]
    let locked = false;
    let mut log = StallLog::new(a.num_or("stall-us", 5.0)?, 1 << 20);
    let end = monotonic_ns() + duration_s * 1_000_000_000;
    log.start(0);
    let mut n = 0u32;
    loop {
        log.tick(0);
        n = n.wrapping_add(1);
        // One clock read every 4096 iterations.
        if n & 4095 == 0 && monotonic_ns() >= end {
            break;
        }
    }
    if load_only {
        return Ok(());
    }
    let out = std::path::PathBuf::from(a.req("out")?);
    std::fs::create_dir_all(&out)?;
    log.write(&out.join("stalls-0.txt"))?;
    let meta = format!(
        "mode=noise\nvariants=noise-probe\ncpu={cpu}\nduration_s={duration_s}\nmlockall={locked}\n\
         loop.0.spins={}\nloop.0.gap_max_us={:.1}\nloop.0.gaps_over_5us={}\n\
         loop.0.gaps_over_20us={}\nloop.0.gaps_over_100us={}\narch={}\n{}",
        log.iterations,
        log.max_us(),
        log.over[0],
        log.over[1],
        log.over[2],
        std::env::consts::ARCH,
        counter_meta()
    );
    std::fs::write(out.join("meta.txt"), &meta)?;
    print!("{meta}");
    Ok(())
}
