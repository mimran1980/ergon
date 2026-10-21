//! The crate's errors.

use crate::detect::Fallback;
use crate::timesync::SyncVerdict;
use std::fmt;

/// What can go wrong making a clock.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The counter is required (`Policy::RequireCounter`, or parameters of
    /// a counter clock) and cannot be used here, for this reason.
    CounterUnavailable(Fallback),
    /// A kernel clock could not be read (`clock_gettime` failed), or its
    /// discipline state ([`kernel_sync`](crate::advanced::kernel_sync): `adjtimex`).
    ClockRead {
        /// The clock's name.
        clock: &'static str,
        /// The error number.
        errno: i32,
    },
    /// Parameters this process cannot use: the field at fault.
    InvalidParams(&'static str),
    /// `CLOCK_REALTIME` must be disciplined to a PTP hardware clock
    /// ([`TimeSyncPolicy::RequirePtp`](crate::advanced::TimeSyncPolicy::RequirePtp))
    /// and is not, or could not be shown to be.
    NotPtpGrade {
        /// The verdict.
        verdict: SyncVerdict,
        /// Why, with its fix: the time-sync finding.
        why: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CounterUnavailable(why) => {
                write!(f, "the hardware counter cannot be the clock here: {why}")
            }
            Self::ClockRead { clock, errno } => {
                write!(f, "{clock} could not be read (errno {errno})")
            }
            Self::InvalidParams(field) => write!(f, "clock parameters not usable here: {field}"),
            Self::NotPtpGrade { verdict, why } => write!(
                f,
                "CLOCK_REALTIME is required to be disciplined to a PTP hardware clock and is {}: {why}",
                verdict.name()
            ),
        }
    }
}

impl std::error::Error for Error {}
