//! TCP echo and WebSocket-over-TLS echo for the transport latency suite
//! (`io_bench::transport::wstls`). The measurement, the echo and the files
//! are `io_bench::transport::point`'s, the same runner `tcp_lat` uses.
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
use io_bench::transport::wstls::{WSTLS_USAGE, run};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{WSTLS_USAGE}");
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
            let line = format!("wstls_lat: {e}\n");
            eprint!("{line}");
            ExitCode::FAILURE
        }
    }
}
