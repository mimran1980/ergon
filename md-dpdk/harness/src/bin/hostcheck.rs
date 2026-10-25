//! The transport latency suite's host check: the counter clock across the
//! hot CPUs (`ergon_latency_core::clock::cross_cpu_check`) and the host's
//! settings, reported before a round.
//!
//! Not built yet: exits non-zero so no run reads it as a result.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("hostcheck: not implemented yet");
    ExitCode::FAILURE
}
