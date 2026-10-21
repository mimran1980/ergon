//! Latency benchmark for `ergon-io`: the [`ergon_io::Driver`] backends compared
//! on live Binance traffic and in a two-host lab. See `BENCHMARK.md`.
//!
//! The protocol layers come from the library ([`tls`], [`ws`]); this crate adds
//! the measured loop ([`runner`]), the recording applications ([`app`]), the
//! record format and the statistics.

pub mod args;
pub mod clock;
pub mod diag;
pub mod fair;
pub mod json;
pub mod matrix;
pub mod mixed;
pub mod order;
pub mod periodic;
pub mod record;
pub mod stalls;
pub mod stats;

#[cfg(target_os = "linux")]
pub mod app;
#[cfg(target_os = "linux")]
pub mod runner;

pub use ergon_io::{Error, Result, tls, ws};
