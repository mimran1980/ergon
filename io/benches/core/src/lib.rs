//! The measurement core every transport latency arm shares, so each one
//! sends the same bytes, reads the same clock, keeps the same schedule and
//! reports in the same format:
//!
//! - [`message`]: the 64-byte message;
//! - [`clock`]: the raw counter clock and a cross-CPU check;
//! - [`pace`]: the open-loop sender's schedule and its lateness counts;
//! - [`hist`]: latency histograms with exact min and max;
//! - [`loss`]: per-stream loss, reorder and duplicate accounting;
//! - [`frame`]: the u32 LE length framing both TCP arms write;
//! - [`row`]: the result row (`load`'s format plus the suite's columns),
//!   spikes and stalls;
//! - [`manifest`]: the run manifest.
//!
//! Everything a hot path calls is allocation-free once built
//! (`tests/hot_alloc.rs`).

mod error;

pub mod clock;
pub mod frame;
pub mod hist;
pub mod loss;
pub mod manifest;
pub mod message;
pub mod pace;
pub mod row;

pub use error::Error;
