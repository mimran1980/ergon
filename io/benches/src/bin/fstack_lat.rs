//! The transport latency suite's DPDK TCP arm: TCP over F-Stack
//! (`io_bench::transport::fstack`), one connection per stream, writing
//! `ergon_latency_core::frame`'s u32 LE frames, as `tcp_lat` does. Every
//! measurement, the echo and the files are `io_bench::transport::point`'s,
//! run by `io_bench::transport::sock::run` (`--help`).
//!
//! Prints the row (none for a pong) and exits non-zero, with no row, on
//! any error.
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
    use io_bench::transport::fstack::fstack_main;
    use io_bench::transport::fstack_conf::FSTACK_USAGE;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{FSTACK_USAGE}");
        return ExitCode::SUCCESS;
    }
    match fstack_main(args) {
        Ok(row) => {
            if let Some(row) = row {
                println!("{row}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            let line = format!("fstack_lat: {e}\n");
            eprint!("{line}");
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
            eprintln!("fstack_lat: Linux only (F-Stack on DPDK); selected backend {backend}");
        }
        Err(e) => {
            eprintln!("fstack_lat: {e}");
        }
    }
    ExitCode::FAILURE
}
