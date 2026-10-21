//! The transport's clock: [`ergon_clock`]'s calibration, handed to the native
//! transport.
//!
//! The native transport reads the time on every send and poll (ARP
//! freshness, the transmit pacer, receive-poll gaps). It calibrates no clock
//! of its own: [`DpdkTransport::install`](crate::DpdkTransport::install)
//! takes [`transport_clock`] (one calibration a process) and passes its
//! conversion ([`ergon_clock::advanced::Params`]) in the native configuration. The C
//! hot path then reads the counter inline, `rdtsc` on `x86_64` and
//! `mrs cntvct_el0` on `aarch64`, and converts it with ergon-clock's formula,
//! bit for bit (`Params`' documentation):
//!
//! ```text
//! ns = base_ns + (int64_t)(((__int128)(int64_t)(ticks - base_ticks) * (int64_t)mult) >> 32)
//! ```
//!
//! Where ergon-clock falls back, the ticks are `CLOCK_MONOTONIC`'s
//! nanoseconds through the same formula (a factor of one). Either way the
//! nanoseconds count in the monotonic domain, as the transport's always have.
//!
//! [`CLOCK_ENV`] chooses: `counter` (the default; `tsc`, its earlier name, on
//! either architecture) or `monotonic`, which forces the fallback to compare
//! the two. The counter is trusted on calibration's own measurement and
//! verification ([`Trust::CrossCheck`]), as the transport always did: a
//! paravirtual clock source (Azure's `hyperv_clocksource_tsc_page`,
//! `kvm-clock`) keeps the counter, which these intervals need no kernel's
//! vouching for. The wall clock's discipline is not probed: the transport's
//! clock never stamps wall-clock time (its media and NIC-arrival timestamps
//! read `CLOCK_REALTIME` itself).

use std::sync::OnceLock;

use ergon_clock::advanced::{Calibrated, Policy, TimeSyncPolicy, Trust};
use ergon_clock::{Config, Domain};

use crate::error::DpdkError;

/// The variable choosing the transport's clock: `counter` (the default, or
/// `tsc`) or `monotonic`.
pub const CLOCK_ENV: &str = "RUSTERON_DPDK_CLOCK";

/// Which clock the transport reads ([`CLOCK_ENV`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClockChoice {
    /// The hardware counter wherever ergon-clock's calibration takes it,
    /// else its `CLOCK_MONOTONIC` fallback (the report says why).
    #[default]
    Counter,
    /// `CLOCK_MONOTONIC`: ergon-clock's fallback, forced (to compare the
    /// two).
    Monotonic,
}

impl ClockChoice {
    /// Reads [`CLOCK_ENV`]: [`ClockChoice::Counter`] when unset or empty, or
    /// `counter`, or `tsc`; [`ClockChoice::Monotonic`] for `monotonic`.
    ///
    /// # Errors
    ///
    /// [`DpdkError::InvalidEnvironment`] naming the variable for any other
    /// value.
    pub fn from_env() -> Result<Self, DpdkError> {
        match std::env::var(CLOCK_ENV) {
            Err(std::env::VarError::NotPresent) => Ok(Self::Counter),
            Ok(v) if v.is_empty() || v == "counter" || v == "tsc" => Ok(Self::Counter),
            Ok(v) if v == "monotonic" => Ok(Self::Monotonic),
            Ok(v) => Err(DpdkError::InvalidEnvironment(format!(
                "{CLOCK_ENV}={v:?} must be counter (or tsc) or monotonic"
            ))),
            Err(std::env::VarError::NotUnicode(v)) => Err(DpdkError::InvalidEnvironment(format!(
                "{CLOCK_ENV}={} must be counter (or tsc) or monotonic",
                v.display()
            ))),
        }
    }
}

/// How the transport's clock is calibrated for `choice`.
///
/// The counter is trusted on calibration's own checks
/// ([`Trust::CrossCheck`]), in the monotonic domain, the wall clock's
/// discipline not probed; for [`ClockChoice::Monotonic`] the fallback,
/// forced.
#[must_use]
pub fn clock_config(choice: ClockChoice) -> Config {
    Config {
        policy: match choice {
            ClockChoice::Counter => Policy::Auto,
            ClockChoice::Monotonic => Policy::Monotonic,
        },
        trust: Trust::CrossCheck,
        domain: Domain::Monotonic,
        time_sync: TimeSyncPolicy::Off,
        ..Config::default()
    }
}

/// The transport's clock for `choice`, and ergon-clock's report of what it
/// chose and why.
///
/// [`ClockChoice::Counter`] is calibrated once a process (tens of
/// milliseconds, the first time; never on a hot path), so every transport,
/// and every thread a caller gives a copy of the clock, reads one timebase.
/// [`ClockChoice::Monotonic`] calibrates nothing.
///
/// # Errors
///
/// [`DpdkError::Clock`] when a kernel clock cannot be read.
pub fn transport_clock(choice: ClockChoice) -> Result<Calibrated, DpdkError> {
    static COUNTER: OnceLock<Result<Calibrated, ergon_clock::Error>> = OnceLock::new();
    match choice {
        ClockChoice::Counter => COUNTER
            .get_or_init(|| ergon_clock::advanced::calibrate(&clock_config(ClockChoice::Counter)))
            .clone()
            .map_err(DpdkError::Clock),
        ClockChoice::Monotonic => {
            ergon_clock::advanced::calibrate(&clock_config(ClockChoice::Monotonic)).map_err(DpdkError::Clock)
        }
    }
}
