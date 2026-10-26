//! The transport latency suite's UDP arms: raw mio
//! (`io_bench::transport::mio_udp`) and ergon-io
//! (`io_bench::transport::eio_udp`), epoll-spin, one connected socket per
//! stream. Every measurement, the echo and the files are
//! `io_bench::transport::point`'s; flags, threads and roles:
//! `io_bench::transport::sock` (`--help`).
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
    use io_bench::transport::sock::{UDP_USAGE, udp_main};
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{UDP_USAGE}");
        return ExitCode::SUCCESS;
    }
    match udp_main(args) {
        Ok(row) => {
            if let Some(row) = row {
                println!("{row}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            // one write, its newline in it: ergon-io's warning thread may be
            // writing to the same file (a runner's `>out 2>&1`)
            let line = format!("udp_lat: {e}\n");
            eprint!("{line}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    eprintln!("udp_lat: Linux only (ergon-io's UDP is epoll-spin)");
    ExitCode::FAILURE
}
