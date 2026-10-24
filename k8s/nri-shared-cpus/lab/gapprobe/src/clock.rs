//! Counter ticks to nanoseconds, and to `CLOCK_REALTIME` through two anchors
//! (a counter read beside a realtime read at the start and at the end of a
//! run), so a gap's timestamp lines up with the sampler's.

const NS_PER_S: u64 = 1_000_000_000;

/// Ticks to nanoseconds at a fixed rate, exactly (two divisions: not for
/// the spin loop, which records ticks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    hz: u64,
}

impl Rate {
    /// A counter of `hz` ticks a second; `None` for 0.
    #[must_use]
    pub fn from_hz(hz: u64) -> Option<Self> {
        (hz > 0).then_some(Self { hz })
    }

    /// Ticks a second.
    #[must_use]
    pub fn hz(&self) -> u64 {
        self.hz
    }

    /// `ticks` as nanoseconds, saturating.
    #[must_use]
    pub fn ns(&self, ticks: u64) -> u64 {
        let whole = (ticks / self.hz).saturating_mul(NS_PER_S);
        let part = u128::from(ticks % self.hz) * u128::from(NS_PER_S) / u128::from(self.hz);
        whole.saturating_add(part as u64)
    }

    /// Nanoseconds as ticks, rounded up: a threshold in ticks that no
    /// shorter gap reaches.
    #[must_use]
    pub fn ticks(&self, ns: u64) -> u64 {
        let t = (u128::from(ns) * u128::from(self.hz)).div_ceil(u128::from(NS_PER_S));
        u64::try_from(t).unwrap_or(u64::MAX)
    }
}

/// Two (ticks, `CLOCK_REALTIME` ns) anchors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchors {
    /// ticks at the first anchor
    pub t0: u64,
    /// realtime ns at the first anchor
    pub r0: u64,
    /// ticks at the second
    pub t1: u64,
    /// realtime ns at the second
    pub r1: u64,
}

impl Anchors {
    /// The realtime of `ticks`, on the line through both anchors; `None`
    /// when the anchors do not advance.
    #[must_use]
    pub fn realtime(&self, ticks: u64) -> Option<u64> {
        if self.t1 <= self.t0 || self.r1 < self.r0 {
            return None;
        }
        let dt = u128::from(ticks.saturating_sub(self.t0));
        let off = dt * u128::from(self.r1 - self.r0) / u128::from(self.t1 - self.t0);
        u64::try_from(off).ok().map(|o| self.r0.saturating_add(o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_converts_both_ways() {
        let ghz = Rate::from_hz(1_000_000_000);
        assert_eq!(ghz.map(|r| r.ns(1234)), Some(1234));
        assert_eq!(ghz.map(|r| r.ticks(1000)), Some(1000));
        // Graviton2's 121.875 MHz-style counter: 8.205 ns a tick
        let slow = Rate::from_hz(121_875_000);
        assert_eq!(slow.map(|r| r.ns(121_875_000)), Some(1_000_000_000));
        // 1 us is 121.875 ticks: a gap of 122 ticks reaches the threshold
        assert_eq!(slow.map(|r| r.ticks(1000)), Some(122));
        assert_eq!(Rate::from_hz(0), None);
    }

    #[test]
    fn rate_saturates_and_does_not_overflow_on_long_gaps() {
        let r = Rate::from_hz(25_000_000);
        // an hour at 25 MHz
        assert_eq!(r.map(|r| r.ns(90_000_000_000)), Some(3_600_000_000_000));
        assert_eq!(r.map(|r| r.ns(u64::MAX)), Some(u64::MAX));
    }

    #[test]
    fn realtime_interpolates_between_anchors() {
        let a = Anchors {
            t0: 1_000,
            r0: 5_000_000_000,
            t1: 1_000 + 2_000_000,
            r1: 5_000_000_000 + 1_000_000,
        };
        // two ticks a nanosecond
        assert_eq!(a.realtime(1_000), Some(5_000_000_000));
        assert_eq!(a.realtime(1_000 + 1_000_000), Some(5_000_500_000));
        assert_eq!(a.realtime(1_000 + 2_000_000), Some(5_001_000_000));
        let flat = Anchors {
            t0: 5,
            r0: 1,
            t1: 5,
            r1: 9,
        };
        assert_eq!(flat.realtime(5), None);
    }
}
