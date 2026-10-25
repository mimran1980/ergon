//! The transport latency suite's UDP arms: raw mio
//! ([`io_bench::transport::mio_udp`]) and ergon-io
//! ([`io_bench::transport::eio_udp`]), epoll-spin. One connected UDP
//! socket per stream, stream `i` on base port + `i` at both ends; one
//! busy-spinning thread per end. Flags and threads:
//! [`io_bench::transport::bench`] (`--help`).

use std::process::ExitCode;

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    use io_bench::args::Args;
    use io_bench::transport::bench::{UDP_USAGE, udp_main};
    use io_bench::transport::point::Hooks;

    let a = Args::from_env();
    if a.flag("help") {
        println!("{UDP_USAGE}");
        return ExitCode::SUCCESS;
    }
    match udp_main(&a, Hooks::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("udp_lat: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    eprintln!("udp_lat: Linux only (ergon-io's UDP is epoll-spin)");
    ExitCode::FAILURE
}
