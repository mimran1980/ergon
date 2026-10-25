//! The transport latency suite's IPC arm: `std::sync::mpsc::sync_channel`,
//! `try_send` / `try_recv` busy-spin, one way on one host.
//!
//! Not built yet: exits non-zero so no run reads it as a result.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("mpsc_lat: not implemented yet");
    ExitCode::FAILURE
}
