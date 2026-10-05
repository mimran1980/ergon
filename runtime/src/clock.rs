//! A clock for low-latency threads: read the time once per iteration of the
//! loop, and reuse it (Agrona's `CachedNanoClock`).
//!
//! ```
//! use ergon_runtime::clock::Clock;
//!
//! let clock = Clock::new();
//! let now = clock.now(); // one read of the hardware clock, cached
//! assert_eq!(clock.cached(), now); // a plain load: no clock read
//! assert!(now.epoch_ns() > 1_700_000_000_000_000_000); // epoch ns: no system call
//! ```
//!
//! Times are [`Nanos`]: signed UNIX-epoch nanoseconds. A live read is the
//! epoch the clock paired with the monotonic clock when it was made, plus
//! monotonic elapsed since, one add. Clocks made apart differ by how far the
//! wall clock was corrected in between. A clock is the live path only: in a
//! simulation the runtime's [`Ctx::now`](crate::rt::Ctx::now) is the time.
//! On Linux the read is `minstant` (the time-stamp counter when it is
//! available). Elsewhere it is [`std::time::Instant`]: `minstant`'s fallback
//! there is the wall clock, which can step backwards.

use std::cell::Cell;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Monotonic clock for this process. Linux uses `minstant`. Other operating
/// systems use [`std::time::Instant`], because `minstant` reads the wall clock
/// when the time-stamp counter is not compiled in.
#[cfg(target_os = "linux")]
type Mono = minstant::Instant;
#[cfg(not(target_os = "linux"))]
type Mono = std::time::Instant;

/// UNIX-epoch nanoseconds. Signed so a converted venue timestamp can fall
/// before the epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nanos(pub i64);

impl Nanos {
    /// This value as UNIX nanoseconds. [`Nanos`] is already an epoch time.
    #[inline]
    #[must_use]
    pub const fn epoch_ns(self) -> i64 {
        self.0
    }

    /// A UNIX nanosecond timestamp, such as a venue's `ts_event`.
    #[inline]
    #[must_use]
    pub const fn from_epoch(epoch_ns: i64) -> Self {
        Self(epoch_ns)
    }

    /// `self - earlier` in nanoseconds; negative when `earlier` is later.
    #[inline]
    #[must_use]
    pub const fn since(self, earlier: Self) -> i64 {
        self.0 - earlier.0
    }
}

/// One pairing of the monotonic clock with the wall clock: a [`Clock`]
/// without its cache, which the `tracing` bridge shares between threads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Anchor {
    mono: Mono,
    epoch_ns: i64,
}

/// Wall-clock steps a pairing brackets at least, keeping the narrowest.
const STEPS: u32 = 8;
/// A bracket this narrow, a few reads, ends the search after [`STEPS`].
const NARROW: Duration = Duration::from_nanos(250);
/// After [`STEPS`] and this long, the narrowest bracket is taken however
/// wide: where reading the wall clock is slow.
const SEARCH: Duration = Duration::from_millis(1);
/// A wall clock that has not stepped for this long ends the search with
/// what it has.
const NO_STEP: Duration = Duration::from_millis(100);

impl Anchor {
    /// Paired where the wall clock steps: it read its new value somewhere
    /// between the monotonic reads either side of the read before, and of
    /// this one, and the pairing is the middle of them, within half their
    /// width. Where the wall clock counts whole microseconds (macOS), only a
    /// step tells when it read, so two clocks made apart agree to within
    /// their brackets rather than a microsecond.
    ///
    /// The narrowest of at least [`STEPS`] brackets, searching on while that
    /// is wider than [`NARROW`], until [`SEARCH`]: about 8 µs where the wall
    /// clock counts microseconds, unless reads are slow. A preemption widens
    /// only the bracket it lands in, and cannot end the search: the wall
    /// clock steps across it. Before [`STEPS`], only a wall clock that stops
    /// stepping for [`NO_STEP`] ends it, unbracketed only if it never
    /// stepped.
    pub(crate) fn new() -> Self {
        Self::paired().0
    }

    /// The pairing, and the width of its bracket: [`Duration::MAX`] when the
    /// wall clock did not step.
    fn paired() -> (Self, Duration) {
        let wall_ns = || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
        };
        let start = Mono::now();
        let mut last = wall_ns();
        // Monotonic reads before and after the wall clock last read `last`.
        let (mut before, mut after) = (start, Mono::now());
        let mut best = (
            Self {
                mono: start,
                epoch_ns: last,
            },
            Duration::MAX,
        );
        let (mut steps, mut stepped) = (0, start);
        loop {
            let epoch_ns = wall_ns();
            let now = Mono::now();
            if epoch_ns != last {
                let width = now.duration_since(before);
                if width < best.1 {
                    best = (
                        Self {
                            mono: before + width / 2,
                            epoch_ns,
                        },
                        width,
                    );
                }
                last = epoch_ns;
                (steps, stepped) = (steps + 1, now);
            }
            (before, after) = (after, now);
            let done = if steps < STEPS {
                now.duration_since(stepped) >= NO_STEP
            } else {
                best.1 <= NARROW || now.duration_since(start) >= SEARCH
            };
            if done {
                return best;
            }
        }
    }

    /// The epoch plus the monotonic time since the pairing.
    #[inline]
    pub(crate) fn read(&self) -> Nanos {
        let elapsed = i64::try_from(self.mono.elapsed().as_nanos()).unwrap_or(i64::MAX);
        Nanos(self.epoch_ns.saturating_add(elapsed))
    }
}

/// A clock that remembers its last read, paired with the wall clock when it
/// is made. `!Sync`: its owner reads it.
#[derive(Debug)]
pub struct Clock {
    cached: Cell<Nanos>,
    anchor: Anchor,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    /// A clock, paired with the wall clock where that steps, from the
    /// narrowest of at least 8 brackets, and cached at the time it was made.
    ///
    /// Not cheap: it reads the wall clock until it has stepped 8 times,
    /// about 8 µs where it counts microseconds, and up to a millisecond
    /// where reading it is slow (a VM with no vDSO clock source). Make one
    /// per thread at start-up and keep it; a one-off wall-clock read
    /// (`SystemTime::now`) needs none.
    #[must_use]
    pub fn new() -> Self {
        let clock = Self {
            cached: Cell::new(Nanos(0)),
            anchor: Anchor::new(),
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
    pub const fn cached(&self) -> Nanos {
        self.cached.get()
    }

    /// The wall clock now, as [`Nanos`]: for comparing with another
    /// process's timestamps (a feed handler's receive time, a venue's
    /// event time). [`Clock::now`] is the paired epoch plus a monotonic
    /// elapsed time, which never steps but drifts from the wall clock as
    /// that is corrected (a VM's clock resynced from its host, NTP): tens of
    /// milliseconds after a few minutes here. A vDSO read, tens of
    /// nanoseconds; not cached.
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

    /// The wall clock less [`Clock::read`], from the tightest of 8 paired
    /// reads. Adding it to a read gives the wall-clock time with no system
    /// call; re-measure it now and then to follow NTP slew.
    #[must_use]
    pub fn wall_offset(&self) -> i64 {
        let mut best = (i64::MAX, 0);
        for _ in 0..8 {
            let before = self.read();
            let wall = self.wall();
            let after = self.read();
            let gap = after.since(before);
            if gap < best.0 {
                best = (gap, wall.0 - (before.0 + gap / 2));
            }
        }
        best.1
    }

    /// Read the clock without caching it.
    #[inline]
    #[must_use]
    pub fn read(&self) -> Nanos {
        self.anchor.read()
    }
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
            .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
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
        assert!(
            venue.0 > 0,
            "an hour before now is still a positive epoch, not an offset"
        );
        assert_eq!(venue.epoch_ns(), now.epoch_ns() - 5_000_000_000_000);
        assert_eq!(
            Nanos::from_epoch(1_700_000_000_000_000_000).0,
            1_700_000_000_000_000_000
        );
        assert_eq!(now.since(venue), 5_000_000_000_000);
        assert_eq!(venue.since(now), -5_000_000_000_000);
    }

    #[test]
    fn two_clocks_made_apart_read_alike() {
        // Each pairing is within half its bracket of the wall clock's step,
        // give or take a monotonic tick at either end. Paired at a read that
        // is not a step, a wall clock that counts whole microseconds (macOS)
        // leaves two clocks up to a microsecond apart.
        let tick = (0..10_000)
            .map(|_| {
                let at = Mono::now();
                Mono::now().duration_since(at)
            })
            .filter(|d| !d.is_zero())
            .min()
            .unwrap_or_default();
        for _ in 0..8 {
            // Back to back: the wall clock's slew between the two pairings,
            // which no pairing can see, is a few nanoseconds at most.
            let (first, first_width) = Anchor::paired();
            let (other, other_width) = Anchor::paired();
            assert!(
                first_width != Duration::MAX && other_width != Duration::MAX,
                "a pairing is unbracketed: the wall clock did not step"
            );
            // The closest of many reads side by side: a preemption between
            // two reads is not the clocks' difference.
            let apart = (0..1_000)
                .map(|_| {
                    let (x, y) = (first.read(), other.read());
                    y.since(x).unsigned_abs()
                })
                .min()
                .unwrap_or(u64::MAX);
            let allowed = (first_width + other_width) / 2 + 2 * tick;
            assert!(
                u128::from(apart) <= allowed.as_nanos(),
                "two clocks read {apart} ns apart, brackets {first_width:?} and {other_width:?} allow {allowed:?}"
            );
            std::thread::sleep(Duration::from_micros(300));
        }
    }
}
