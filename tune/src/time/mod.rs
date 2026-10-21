//! T11 (how `CLOCK_REALTIME` is kept) and T12 (the kernel's clock source),
//! and [`ClockWatch`](crate::ClockWatch), which checks both again from
//! housekeeping without waiting.

mod chrony;
mod clockbound;
mod clocksource;
mod kernel;
mod probe;
mod sync;
mod watch;

pub use chrony::{ChronyError, ChronyReference, ChronyTracking, RefclockId};
pub use clockbound::{ClockBoundError, ClockBoundReading, ClockBoundStatus};
pub use clocksource::{Clocksource, ClocksourceKind, ClocksourceName, t12_clocksource};
pub use kernel::KernelSync;
pub use probe::{TimeSyncProbe, t11_time_sync, time_sync};
pub use sync::{
    Bound, BoundSource, ChronyAnswer, ClockBoundAnswer, Observed, PtpClock, SyncVerdict,
    TimeDaemon, TimeSync, TimeSyncLimits,
};
pub use watch::{ClockSample, ClockWatch};
