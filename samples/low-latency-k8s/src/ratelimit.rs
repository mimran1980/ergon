//! At most one log line per interval from one call site.
//!
//! The hot loop owns its limiters (no atomics, no allocation): each abnormal
//! iteration asks [`RateLimit::check`] with the time it already read, and
//! logs only when it says so, with the count suppressed since the last line.

/// One call site's limiter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimit {
    interval_ns: u64,
    next_ns: u64,
    suppressed: u64,
    emitted: u64,
    total_suppressed: u64,
}

impl RateLimit {
    /// One line per `interval_ns`; the first call always passes.
    #[must_use]
    pub const fn new(interval_ns: u64) -> Self {
        Self {
            interval_ns,
            next_ns: 0,
            suppressed: 0,
            emitted: 0,
            total_suppressed: 0,
        }
    }

    /// One line a second.
    #[must_use]
    pub const fn per_second() -> Self {
        Self::new(1_000_000_000)
    }

    /// `Some(n)` when a line may go now (`n`: the events suppressed since the
    /// last line), else `None` (this one is counted as suppressed).
    pub const fn check(&mut self, now_ns: u64) -> Option<u64> {
        if self.emitted == 0 || now_ns >= self.next_ns {
            let suppressed = self.suppressed;
            self.suppressed = 0;
            self.emitted += 1;
            self.next_ns = now_ns.saturating_add(self.interval_ns);
            Some(suppressed)
        } else {
            self.suppressed += 1;
            self.total_suppressed += 1;
            None
        }
    }

    /// Lines let through.
    #[must_use]
    pub const fn emitted(&self) -> u64 {
        self.emitted
    }

    /// Events suppressed over the limiter's life.
    #[must_use]
    pub const fn total_suppressed(&self) -> u64 {
        self.total_suppressed
    }
}

#[cfg(test)]
mod tests {
    use super::RateLimit;

    #[test]
    fn lets_one_line_through_per_interval() {
        let mut limit = RateLimit::new(1_000);
        assert_eq!(limit.check(5), Some(0));
        assert_eq!(limit.check(6), None);
        assert_eq!(limit.check(900), None);
        assert_eq!(limit.check(1_005), Some(2));
        assert_eq!(limit.check(1_006), None);
        assert_eq!(limit.check(5_000), Some(1));
        assert_eq!(limit.emitted(), 3);
        assert_eq!(limit.total_suppressed(), 3);
    }

    #[test]
    fn survives_a_clock_at_zero_and_a_huge_interval() {
        let mut limit = RateLimit::new(u64::MAX);
        assert_eq!(limit.check(0), Some(0));
        assert_eq!(limit.check(u64::MAX - 1), None);
    }
}
