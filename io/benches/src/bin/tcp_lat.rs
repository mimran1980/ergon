//! The transport latency suite's TCP arms: raw mio
//! ([`io_bench::transport::mio_tcp`]) and ergon-io framed TCP
//! ([`io_bench::transport::eio_tcp`]), epoll-spin, both writing
//! `ergon_latency_core::frame`'s u32 LE length frames. One connection per
//! stream; one busy-spinning thread per end. Flags and threads:
//! [`io_bench::transport::bench`] (`--help`).

use std::process::ExitCode;

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    use io_bench::args::Args;
    use io_bench::transport::bench::{TCP_USAGE, tcp_main};
    use io_bench::transport::point::Hooks;

    let a = Args::from_env();
    if a.flag("help") {
        println!("{TCP_USAGE}");
        return ExitCode::SUCCESS;
    }
    match tcp_main(&a, Hooks::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tcp_lat: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    eprintln!("tcp_lat: Linux only (ergon-io's driver)");
    ExitCode::FAILURE
}
