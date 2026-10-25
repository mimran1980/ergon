//! The transport latency suite's UDP arms: raw mio
//! ([`io_bench::transport::mio_udp`]) and ergon-io
//! ([`io_bench::transport::eio_udp`]), epoll-spin.
//!
//! Not built yet: exits non-zero so no run reads it as a result.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("udp_lat: not implemented yet");
    ExitCode::FAILURE
}
