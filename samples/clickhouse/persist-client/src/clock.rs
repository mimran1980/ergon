//! A clock for low-latency threads: read the time once per iteration of the
//! loop, and reuse it (Agrona's `CachedNanoClock`).
//!
//! ```
//! use persist_client::clock::Clock;
//!
//! let clock = Clock::new();
//! let now = clock.now(); // one read of the hardware clock, cached
//! assert_eq!(clock.cached(), now); // a plain load: no clock read
//! assert!(now.epoch_ns() > 1_700_000_000_000_000_000); // anchor + offset: no system call
//! ```
//!
//! Times are [`Nanos`]: signed nanoseconds since one anchor per process,
//! taken when the first clock is made, so every clock in the process agrees.
//! Where the CPU's time-stamp counter is invariant (x86-64 Linux, via
//! `minstant`), a read is `rdtsc` and a multiply. Elsewhere `minstant` would
//! fall back to the wall clock, which can step backwards, so the clock reads
//! `std::time::Instant` instead: monotonic everywhere.

use std::cell::Cell;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Nanoseconds since the process's anchor. Signed: a time converted from
/// another clock (a venue's timestamp, say) may be before the anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nanos(pub i64);

impl Nanos {
    /// UNIX nanoseconds: the anchor's wall-clock time plus this offset.
    #[inline]
    #[must_use]
    pub fn epoch_ns(self) -> i64 {
        ANCHOR.epoch_ns + self.0
    }

    /// The `Nanos` of a UNIX nanosecond timestamp, such as a venue's
    /// `ts_event`: comparable with [`Clock::now`].
    #[inline]
    #[must_use]
    pub fn from_epoch(epoch_ns: i64) -> Self {
        Self(epoch_ns - ANCHOR.epoch_ns)
    }

    /// `self - earlier` in nanoseconds; negative when `earlier` is later.
    #[inline]
    #[must_use]
    pub fn since(self, earlier: Self) -> i64 {
        self.0 - earlier.0
    }
}

/// One pairing of the monotonic clocks with the wall clock.
struct Anchor {
    tsc: minstant::Instant,
    std: Instant,
    epoch_ns: i64,
    /// Read the time-stamp counter: invariant and calibrated.
    use_tsc: bool,
}

static ANCHOR: LazyLock<Anchor> = LazyLock::new(|| {
    // The tightest of a few paired reads: the wall clock read between two
    // monotonic reads that are closest together.
    let mut best: Option<(Duration, Anchor)> = None;
    for _ in 0..8 {
        let (tsc, std) = (minstant::Instant::now(), Instant::now());
        let wall = SystemTime::now();
        let gap = std.elapsed();
        if best.as_ref().is_none_or(|(g, _)| gap < *g) {
            let epoch_ns = wall
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
            best = Some((
                gap,
                Anchor {
                    tsc,
                    std,
                    epoch_ns,
                    use_tsc: minstant::is_tsc_available(),
                },
            ));
        }
    }
    best.map(|(_, a)| a).unwrap_or_else(|| Anchor {
        tsc: minstant::Instant::now(),
        std: Instant::now(),
        epoch_ns: 0,
        use_tsc: false,
    })
});

/// A per-thread clock that remembers its last read. `!Sync`: each thread
/// makes its own, and they all share the process's anchor.
#[derive(Debug)]
pub struct Clock {
    cached: Cell<Nanos>,
    /// The process's anchor, copied so a read touches only this clock.
    use_tsc: bool,
    tsc: minstant::Instant,
    std: Instant,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    /// A clock, cached at the time it was made.
    #[must_use]
    pub fn new() -> Self {
        let clock = Self {
            cached: Cell::new(Nanos(0)),
            use_tsc: ANCHOR.use_tsc,
            tsc: ANCHOR.tsc,
            std: ANCHOR.std,
        };
        clock.now();
        clock
    }

    /// Read the clock, and cache the time for [`Clock::cached`].
    #[inline]
    pub fn now(&self) -> Nanos {
        let now = self.read();
        self.cached.set(now);
        now
    }

    /// The time [`Clock::now`] last read: a plain load.
    #[inline]
    #[must_use]
    pub fn cached(&self) -> Nanos {
        self.cached.get()
    }

    /// Read the clock without caching it.
    #[inline]
    #[must_use]
    pub fn read(&self) -> Nanos {
        let since = if self.use_tsc {
            minstant::Instant::now().duration_since(self.tsc)
        } else {
            self.std.elapsed()
        };
        Nanos(nanos(since))
    }
}

/// A duration in nanoseconds, without `as_nanos`' 128-bit multiply.
#[inline]
fn nanos(d: Duration) -> i64 {
    (d.as_secs() as i64)
        .wrapping_mul(1_000_000_000)
        .wrapping_add(i64::from(d.subsec_nanos()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_is_monotonic_and_agrees_with_the_wall_clock() {
        let clock = Clock::new();
        let mut last = clock.now();
        for _ in 0..100_000 {
            let now = clock.now();
            assert!(now >= last, "{now:?} before {last:?}");
            assert_eq!(clock.cached(), now);
            last = now;
        }
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i64);
        let skew = (clock.now().epoch_ns() - wall).abs();
        assert!(
            skew < 1_000_000,
            "epoch time is {skew} ns off the wall clock"
        );
    }

    #[test]
    fn epoch_times_convert_both_ways_including_before_the_anchor() {
        let now = Clock::new().now();
        assert_eq!(Nanos::from_epoch(now.epoch_ns()), now);
        let venue = Nanos::from_epoch(now.epoch_ns() - 5_000_000_000_000);
        assert!(venue.0 < 0, "an hour-plus before the anchor stays signed");
        assert_eq!(now.since(venue), 5_000_000_000_000);
        assert_eq!(venue.since(now), -5_000_000_000_000);
    }
}
