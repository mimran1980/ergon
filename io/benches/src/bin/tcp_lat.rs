//! The transport latency suite's TCP arms: raw mio
//! (`io_bench::transport::mio_tcp`) and ergon-io framed TCP
//! (`io_bench::transport::eio_tcp`), epoll-spin, one connection per
//! stream, both writing `ergon_latency_core::frame`'s u32 LE length
//! frames. Every measurement, the echo and the files are
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
    use io_bench::transport::sock::{TCP_USAGE, tcp_main};
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{TCP_USAGE}");
        return ExitCode::SUCCESS;
    }
    match tcp_main(args) {
        Ok(row) => {
            if let Some(row) = row {
                println!("{row}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            // one write, its newline in it: ergon-io's warning thread may be
            // writing to the same file (a runner's `>out 2>&1`)
            let line = format!("tcp_lat: {e}\n");
            eprint!("{line}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    eprintln!("tcp_lat: Linux only (ergon-io's driver)");
    ExitCode::FAILURE
}
