//! Deterministic time sources, for tests and replays.

use crate::clock::TimeSource;
use std::cell::Cell;

/// A clock that reads what it was last set to: time moves only when the
/// test (or a replay, from its recorded stamps) moves it. One thread's
/// (`!Sync`): give each thread its own.
#[derive(Debug, Default)]
pub struct ManualClock {
    now: Cell<u64>,
}

impl ManualClock {
    /// A clock reading `now_ns`.
    #[must_use]
    pub const fn new(now_ns: u64) -> Self {
        Self {
            now: Cell::new(now_ns),
        }
    }

    /// Sets the time, ns (backwards too: elapsed time then saturates at 0).
    pub fn set(&self, now_ns: u64) {
        self.now.set(now_ns);
    }

    /// Moves the time on by `ns`.
    pub fn advance(&self, ns: u64) {
        self.now.set(self.now.get().wrapping_add(ns));
    }

    /// The time now, ns: what it was last set to. (Also on
    /// [`TimeSource`]; here so a test need not import the trait.)
    #[must_use]
    pub const fn now_ns(&self) -> u64 {
        self.now.get()
    }

    /// Nanoseconds since `start_ns`, never negative (also on
    /// [`TimeSource`]).
    #[must_use]
    pub const fn since_ns(&self, start_ns: u64) -> u64 {
        crate::clock::elapsed_ns(start_ns, self.now.get())
    }
}

impl TimeSource for ManualClock {
    #[inline]
    fn now_ns(&self) -> u64 {
        self.now.get()
    }
}

/// A clock that moves on a fixed step at every read.
///
/// It starts at a given time, and each read is one step after the last, as if every read took that
/// long (the DPDK transport's C clock has the same per-thread hook). One
/// thread's (`!Sync`).
#[derive(Debug)]
pub struct SteppedClock {
    next: Cell<u64>,
    step: u64,
}

impl SteppedClock {
    /// A clock whose first read is `start_ns`, each later one `step_ns` on.
    #[must_use]
    pub const fn new(start_ns: u64, step_ns: u64) -> Self {
        Self {
            next: Cell::new(start_ns),
            step: step_ns,
        }
    }
}

impl TimeSource for SteppedClock {
    #[inline]
    fn now_ns(&self) -> u64 {
        let now = self.next.get();
        self.next.set(now.wrapping_add(self.step));
        now
    }
}
