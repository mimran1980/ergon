//! Everything behind the front API, fully available.
//!
//! The fixed clock segment and its calibration, the counter and fallback
//! clocks, raw ticks, parameters for another process or for C, the
//! discipline that keeps a clock on UTC and the seqlock that shares it,
//! the host's facts and the time-sync probe.
//!
//! Two names exist here and at the crate root:
//! - [`Clock`] here is a fixed segment: `Copy`, five words; a copy never
//!   follows updates. [`crate::Clock`] is the owner that keeps one on UTC.
//! - [`calibrate`] here returns that segment and its report
//!   ([`Calibrated`]), keeps nothing and prints nothing.
//!   [`crate::calibrate`] returns the owner and prints the start-up
//!   warnings.

pub mod discipline;
pub mod raw;
pub mod share;

pub use crate::calibrate::{Calibrated, Kind, Policy, calibrate};
pub use crate::chrony::{ChronyError, ChronyReference, ChronyTracking};
pub use crate::clock::{Clock, CounterClock, MonotonicClock};
pub use crate::clockbound::{ClockBoundReading, ClockBoundStatus};
pub use crate::detect::{
    Arch, CLOCKSOURCE, Fallback, HostFacts, OrderedRead, Os, RateSource, Trust,
};
pub use crate::manual::SteppedClock;
pub use crate::owner::segment;
pub use crate::params::{KIND_COUNTER, KIND_MONOTONIC, PARAMS_VERSION, Params};
pub use crate::scale::Scale;
pub use crate::timesync::{
    Bound, BoundSource, Check, ChronyAnswer, ClockBoundAnswer, Finding, KernelSync,
    MAXERROR_LIMIT_US, Observed, PtpClock, STA_NANO, STA_UNSYNC, Severity, SyncVerdict, TimeDaemon,
    TimeSync, TimeSyncPolicy, TimeSyncProbe, human_ns, kernel_sync, time_sync,
};
pub use crate::wall::{
    KernelSource, Sample, SampleSource, Sampler, StepProbe, WallOffset, monotonic_ns,
    monotonic_raw_ns, realtime_ns, realtime_res_ns,
};
pub use crate::warn::HOST_CHECKS_ENV;
