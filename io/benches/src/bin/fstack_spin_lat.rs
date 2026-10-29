//! ergon-io's `fstack-spin` TCP arm (`fstack_lat --impl ergon`'s measuring
//! loop). Ping runs `Driver<FrameConn<FstackTcp>>` and `point::ping`. Pong
//! is the kernel peer. The row is `tcp_lat`'s. Linux, `--features fstack-spin`.
#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::process::ExitCode;

use io_bench::transport::point::CountingAlloc;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    use io_bench::transport::fstack_conf::FSTACK_USAGE;
    use io_bench::transport::fstack_spin::main as run;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{FSTACK_USAGE}");
        return ExitCode::SUCCESS;
    }
    match run(args) {
        Ok(row) => {
            if let Some(row) = row {
                println!("{row}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("fstack_spin_lat: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    use io_bench::transport::fstack_conf::{FSTACK_USAGE, FstackArgs};
    use io_bench::transport::point::{Common, Flags};
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{FSTACK_USAGE}");
        return ExitCode::SUCCESS;
    }
    let parsed = (|| {
        let mut f = Flags::parse(args)?;
        let c = Common::parse(&mut f)?;
        let a = FstackArgs::parse(&mut f, c.role, &c.cpus)?;
        f.finish()?;
        Ok::<_, io_bench::transport::point::Error>(a.imp.backend())
    })();
    match parsed {
        Ok(backend) => {
            eprintln!("fstack_spin_lat: Linux only (F-Stack on DPDK); selected backend {backend}");
        }
        Err(e) => {
            eprintln!("fstack_spin_lat: {e}");
        }
    }
    ExitCode::FAILURE
}
