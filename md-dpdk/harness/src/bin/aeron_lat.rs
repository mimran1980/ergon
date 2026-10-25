//! The transport latency suite's Aeron arms (IPC, spy, kernel UDP and DPDK),
//! one binary against an external media driver (`AERON_DIR`), sending
//! `ergon_latency_core`'s 64-byte message on its schedule.
//!
//! Not built yet: exits non-zero so no run reads it as a result.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("aeron_lat: not implemented yet");
    ExitCode::FAILURE
}
