//! quanta's clock, read and converted as quanta does, for the head-to-head
//! with ergon-clock (ergon-clock itself does not depend on quanta: see
//! `clock/BENCHMARKS.md` for why).
//!
//! quanta calibrates lazily on the first `quanta::Clock::new` in a process
//! (under a millisecond on Linux, up to 200 ms where `CLOCK_MONOTONIC` is
//! coarse) against `CLOCK_MONOTONIC`, which NTP slews; ignores Arm's
//! frequency register; and has no ordered read.

/// quanta's clock: its raw counter read (`Clock::raw`: `rdtsc`, or
/// `mrs cntvct_el0`) and its conversion (`Clock::delta_as_nanos`, its
/// cheapest route to a `u64` of nanoseconds), counting from when this was
/// made.
#[derive(Clone, Debug)]
pub struct QuantaClock {
    clock: quanta::Clock,
    origin: u64,
}

impl QuantaClock {
    /// A quanta clock. The first in a process runs quanta's global
    /// calibration; later ones copy it.
    #[must_use]
    pub fn new() -> Self {
        let clock = quanta::Clock::new();
        let origin = clock.raw();
        Self { clock, origin }
    }

    /// quanta's raw read, `Clock::raw`.
    #[inline]
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.clock.raw()
    }

    /// The duration, ns, of `ticks` ticks, by `Clock::delta_as_nanos`.
    #[inline]
    #[must_use]
    pub fn ticks_to_ns(&self, ticks: u64) -> u64 {
        self.clock.delta_as_nanos(0, ticks)
    }

    /// ns since this clock was made: `delta_as_nanos(origin, raw())`.
    #[inline]
    #[must_use]
    pub fn now_ns(&self) -> u64 {
        self.clock.delta_as_nanos(self.origin, self.clock.raw())
    }

    /// The wrapped quanta clock, for its own reads (`now`, `scaled`).
    #[must_use]
    pub const fn inner(&self) -> &quanta::Clock {
        &self.clock
    }
}

impl Default for QuantaClock {
    fn default() -> Self {
        Self::new()
    }
}
