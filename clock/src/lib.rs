#![doc = include_str!("../README.md")]
//!
//! ## The API at a glance
//!
//! - [`calibrate`]: the one set-up call. It returns the [`Clock`] and
//!   prints one line per problem ([`Report::warnings`]).
//! - [`Clock::now_ns`]: event stamps, deadlines, ages, the start of a
//!   measurement. [`Clock::now_ns_ordered`]: the end of a measurement, and
//!   stamps that must never be earlier than another thread's.
//!   [`Clock::since_ns`] and [`elapsed_ns`]: durations, never negative.
//! - [`Clock::update`]: from the loop's idle time, about once a second;
//!   keeps the clock on UTC and returns its [`Health`]
//!   ([`Health::warning`], [`Warning::line`] for a log).
//! - [`Clock::follower`]: a [`Follower`] for each other thread.
//! - [`TimeSource`] and [`ManualClock`]: time-reading code written once,
//!   tested with time moved by hand.
//! - [`advanced`]: everything else (the `Copy` segment, raw ticks,
//!   parameters for C, the discipline, the time-sync probe).

mod arch;
mod calibrate;
mod chrony;
mod clock;
mod clockbound;
mod detect;
mod error;
mod manual;
mod owner;
mod params;
mod scale;
mod timesync;
mod wall;
mod warn;

pub mod advanced;

pub use calibrate::{Config, Report};
pub use clock::{Domain, TimeSource, elapsed_ns};
pub use error::Error;
pub use manual::ManualClock;
pub use owner::{Clock, Follower, Health, calibrate};
pub use warn::Warning;
