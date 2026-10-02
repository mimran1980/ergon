//! A clock for low-latency threads: read the time once per iteration of the
//! loop, and reuse it (Agrona's `CachedNanoClock`).
//!
//! ```
//! use runtime::clock::Clock;
//!
//! let clock = Clock::new();
//! let now = clock.now(); // one read of the hardware clock, cached
//! assert_eq!(clock.cached(), now); // a plain load: no clock read
//! assert!(now.epoch_ns() > 1_700_000_000_000_000_000); // anchor + offset: no system call
//! ```
//!
//! Times are [`Nanos`]: signed nanoseconds since one anchor per process,
//! taken when the first clock is made, so every clock in the process agrees.
//! On Linux the read is `minstant` (the time-stamp counter when it is
//! available). Elsewhere it is [`std::time::Instant`]: `minstant`'s fallback
//! there is the wall clock, which can step backwards.

use std::cell::Cell;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Monotonic clock for this process. Linux uses `minstant`. Other operating
/// systems use [`std::time::Instant`], because `minstant` reads the wall clock
/// when the time-stamp counter is not compiled in.
#[cfg(target_os = "linux")]
type Mono = minstant::Instant;
#[cfg(not(target_os = "linux"))]
type Mono = std::time::Instant;

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

/// One pairing of the monotonic clock with the wall clock.
struct Anchor {
    mono: Mono,
    epoch_ns: i64,
}

static ANCHOR: LazyLock<Anchor> = LazyLock::new(|| {
    // The tightest of a few paired reads: the wall clock read between two
    // monotonic reads that are closest together.
    let mut best: Option<(Duration, Anchor)> = None;
    for _ in 0..8 {
        let mono = Mono::now();
        let wall = SystemTime::now();
        let gap = mono.elapsed();
        if best.as_ref().is_none_or(|(g, _)| gap < *g) {
            let epoch_ns = wall
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
            best = Some((gap, Anchor { mono, epoch_ns }));
        }
    }
    best.map(|(_, a)| a).unwrap_or_else(|| Anchor {
        mono: Mono::now(),
        epoch_ns: 0,
    })
});

/// A per-thread clock that remembers its last read. `!Sync`: each thread
/// makes its own, and they all share the process's anchor.
#[derive(Debug)]
pub struct Clock {
    cached: Cell<Nanos>,
    /// The process's anchor, copied so a read touches only this clock.
    mono: Mono,
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
            mono: ANCHOR.mono,
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

    /// The wall clock now, as [`Nanos`]: for comparing with another
    /// process's timestamps (a feed handler's receive time, a venue's
    /// event time). [`Clock::now`] is this process's monotonic clock from
    /// its anchor, which is right within the process but drifts from the
    /// wall clock as that is corrected (a VM's clock resynced from its host,
    /// NTP): tens of milliseconds after a few minutes here. A vDSO read,
    /// tens of nanoseconds; not cached.
    #[inline]
    #[must_use]
    pub fn wall(&self) -> Nanos {
        let epoch_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
        Nanos::from_epoch(epoch_ns)
    }

    /// The monotonic time at which a message another process stamped
    /// `epoch_ns` (by its wall clock) was, given that it is being handled
    /// at `now` (this clock's): `now` less the wall-clock age. Starting a
    /// trace here makes its first stage the true age, and every later stage
    /// monotonic.
    #[inline]
    #[must_use]
    pub fn from_remote(&self, epoch_ns: i64, now: Nanos) -> Nanos {
        Nanos(now.0 - self.wall().since(Nanos::from_epoch(epoch_ns)))
    }

    /// Read the clock without caching it.
    #[inline]
    #[must_use]
    pub fn read(&self) -> Nanos {
        Nanos(nanos(self.mono.elapsed()))
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
    #[test]
    fn a_remote_stamp_is_placed_by_its_wall_clock_age() {
        let clock = Clock::new();
        let now = clock.now();
        // Stamped 5 ms ago by another process's wall clock.
        let stamped = clock.wall().epoch_ns() - 5_000_000;
        let at = clock.from_remote(stamped, now);
        let age = now.since(at);
        assert!((5_000_000..6_000_000).contains(&age), "{age}");
    }

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
