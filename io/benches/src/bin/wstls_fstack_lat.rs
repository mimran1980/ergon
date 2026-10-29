//! WebSocket-over-TLS on ergon-io's `fstack-spin` (`wstls_lat`'s flags).
//!
//! Its own cargo invocation (`--features fstack-spin`). The smoltcp build of
//! `wstls_lat` does not link F-Stack. Pong is the kernel peer.
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
    use io_bench::transport::wstls::{WSTLS_USAGE, run};
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
            eprintln!("wstls_fstack_lat: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    use io_bench::transport::point::{Common, Flags};
    use io_bench::transport::wstls::{WSTLS_USAGE, WstlsArgs};
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("{WSTLS_USAGE}");
        return ExitCode::SUCCESS;
    }
    let parsed = (|| {
        let mut f = Flags::parse(args)?;
        let _c = Common::parse(&mut f)?;
        let a = WstlsArgs::parse(&mut f)?;
        f.finish()?;
        Ok::<_, io_bench::transport::point::Error>(a.backend.name())
    })();
    match parsed {
        Ok(backend) => {
            eprintln!("wstls_fstack_lat: Linux only (F-Stack on DPDK); selected backend {backend}");
        }
        Err(e) => {
            eprintln!("wstls_fstack_lat: {e}");
        }
    }
    ExitCode::FAILURE
}
