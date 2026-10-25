//! The transport latency suite's TCP arms: raw mio
//! ([`io_bench::transport::mio_tcp`]) and ergon-io framed TCP
//! ([`io_bench::transport::eio_tcp`]), epoll-spin, both writing
//! `ergon_latency_core::frame`'s u32 LE length frames.
//!
//! Not built yet: exits non-zero so no run reads it as a result.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("tcp_lat: not implemented yet");
    ExitCode::FAILURE
}
