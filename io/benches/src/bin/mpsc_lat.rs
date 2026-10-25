//! The transport latency suite's IPC baseline: `std::sync::mpsc::sync_channel`
//! with `try_send` / `try_recv` busy-spin, both ends in this process
//! (`io_bench::transport::mpsc`).
//!
//! ```text
//! mpsc_lat --role local [--round-trip] --rate MSG_PER_S --label TEXT --out DIR
//!          [--streams N] [--capacity 1024] [--warmup-s 30] [--duration-s 30]
//!          [--size 64] [--cpus A,B] [--spike-ns 100000] [--stall-ns 10000]
//! ```
//!
//! Two busy threads; `--cpus` names their CPUs in order: sender, receiver
//! (one way, an `oneway` row) or ping, echo (`--round-trip`, a `load` row).
//! Without `--cpus` both run unpinned (smoke runs). Writes `results.csv`,
//! `manifest.json`, `histogram-<label>.csv`, `histogram-send-<label>.csv`,
//! `spikes-<label>.csv` and `stalls-<label>.csv` to `--out`, prints the row,
//! and exits non-zero, with no row, on any error.
#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use io_bench::transport::mpsc::{Options, run};
use io_bench::transport::point::{Common, CountingAlloc, Error, Flags};
use std::process::ExitCode;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

fn main() -> ExitCode {
    let go = || -> Result<String, Error> {
        let mut f = Flags::parse(std::env::args().skip(1))?;
        let c = Common::parse(&mut f)?;
        let o = Options {
            capacity: f.num("capacity", 1024)?,
        };
        f.finish()?;
        run(&c, &o)
    };
    match go() {
        Ok(row) => {
            println!("{row}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mpsc_lat: {e}");
            ExitCode::FAILURE
        }
    }
}
