//! What the hot loop records: round-trip latency, its own loop gaps, and
//! counters that `ll-stats` reads from the shared CPU.
//!
//! Everything is allocated before the measured run; recording never
//! allocates. The hot thread owns [`Latency`] and [`LoopGap`] outright and
//! publishes progress through [`Counters`], one cache line per counter,
//! stored relaxed.

use std::sync::atomic::{AtomicU64, Ordering};

use hdrhistogram::Histogram;

use crate::LlError;

/// One value on a cache line of its own (128 bytes: two 64-byte lines on
/// x86, where the adjacent-line prefetcher pairs them; one line on
/// Neoverse).
#[derive(Debug, Default)]
#[repr(align(128))]
pub struct Padded<T>(pub T);

/// Progress the hot thread publishes for `ll-stats`.
#[derive(Debug, Default)]
pub struct Counters {
    /// Round trips measured (ping) or messages echoed (pong).
    pub messages: Padded<AtomicU64>,
    /// Echoes that did not come back in time.
    pub lost: Padded<AtomicU64>,
    /// Warning lines the hot thread let through its rate limits.
    pub warn_events: Padded<AtomicU64>,
    /// The longest loop gap so far, ns.
    pub max_gap_ns: Padded<AtomicU64>,
}

impl Counters {
    /// Adds one to `counter` (the owner is the only writer).
    #[inline]
    pub fn bump(counter: &Padded<AtomicU64>) {
        let v = counter.0.load(Ordering::Relaxed);
        counter.0.store(v.wrapping_add(1), Ordering::Relaxed);
    }

    /// Stores `value` (the owner is the only writer).
    #[inline]
    pub fn set(counter: &Padded<AtomicU64>, value: u64) {
        counter.0.store(value, Ordering::Relaxed);
    }

    /// Reads `counter`.
    #[must_use]
    pub fn get(counter: &Padded<AtomicU64>) -> u64 {
        counter.0.load(Ordering::Relaxed)
    }
}

/// The hot loop's own stalls: the time between two passes of its loop.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoopGap {
    last_ns: u64,
    /// The longest gap, ns.
    pub max_ns: u64,
    /// Gaps over 5 µs.
    pub over_5us: u64,
    /// Gaps over 20 µs.
    pub over_20us: u64,
    /// Gaps over 100 µs.
    pub over_100us: u64,
}

impl LoopGap {
    /// One pass of the loop at `now_ns`: the gap since the last pass, 0 for
    /// the first.
    #[inline]
    pub const fn pass(&mut self, now_ns: u64) -> u64 {
        let gap = if self.last_ns == 0 {
            0
        } else {
            now_ns.saturating_sub(self.last_ns)
        };
        self.last_ns = now_ns;
        if gap > self.max_ns {
            self.max_ns = gap;
        }
        if gap > 5_000 {
            self.over_5us += 1;
            if gap > 20_000 {
                self.over_20us += 1;
                if gap > 100_000 {
                    self.over_100us += 1;
                }
            }
        }
        gap
    }

    /// Forgets the last pass (after a phase that is not the loop).
    pub const fn restart(&mut self) {
        self.last_ns = 0;
    }
}

/// How many of the largest round trips keep their time of day.
pub const TOP: usize = 10;

/// The latency percentiles of a run, ns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Percentiles {
    /// Median.
    pub p50: u64,
    /// 90th.
    pub p90: u64,
    /// 99th.
    pub p99: u64,
    /// 99.9th.
    pub p999: u64,
    /// 99.99th.
    pub p9999: u64,
    /// The largest, exact (not rounded to a histogram bucket).
    pub max: u64,
    /// The mean, rounded.
    pub mean: u64,
    /// How many values.
    pub count: u64,
}

/// Round-trip latency, recorded without allocating.
///
/// A histogram from 1 ns to 10 s at 3 significant figures for the
/// percentiles, the exact largest value (the histogram's own max is its
/// bucket's upper edge), and the [`TOP`] largest with their `CLOCK_REALTIME`.
#[derive(Clone, Debug)]
pub struct Latency {
    hist: Histogram<u64>,
    max: u64,
    top: [(u64, u64); TOP],
}

impl Latency {
    /// Allocates the histogram.
    ///
    /// # Errors
    ///
    /// [`LlError::BadValue`] if hdrhistogram refuses its bounds (it does
    /// not for these).
    pub fn new() -> Result<Self, LlError> {
        let hist = Histogram::new_with_bounds(1, 10_000_000_000, 3)
            .map_err(|e| LlError::bad("histogram", "1..10s/3", e.to_string()))?;
        Ok(Self {
            hist,
            max: 0,
            top: [(0, 0); TOP],
        })
    }

    /// Records one round trip of `rtt_ns`, which ended at `realtime_ns`
    /// (computed only when the value enters the top ten: pass a closure).
    #[inline]
    pub fn record(&mut self, rtt_ns: u64, realtime_ns: impl FnOnce() -> u64) {
        // saturate rather than fail: a value past 10 s is still a value
        self.hist.saturating_record(rtt_ns.max(1));
        if rtt_ns > self.max {
            self.max = rtt_ns;
        }
        // the smallest of the kept ten; slots with rtt 0 are empty
        let mut min_i = 0;
        for i in 1..TOP {
            if self.top[i].1 < self.top[min_i].1 {
                min_i = i;
            }
        }
        if rtt_ns > self.top[min_i].1 {
            self.top[min_i] = (realtime_ns(), rtt_ns);
        }
    }

    /// The percentiles so far.
    #[must_use]
    pub fn percentiles(&self) -> Percentiles {
        let h = &self.hist;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // a mean of u64 ns, rounded
        let mean = h.mean().round() as u64;
        Percentiles {
            p50: h.value_at_quantile(0.50),
            p90: h.value_at_quantile(0.90),
            p99: h.value_at_quantile(0.99),
            p999: h.value_at_quantile(0.999),
            p9999: h.value_at_quantile(0.9999),
            max: self.max,
            mean,
            count: h.len(),
        }
    }

    /// The largest round trips, largest first, as `(realtime_ns, rtt_ns)`.
    #[must_use]
    pub fn top(&self) -> Vec<(u64, u64)> {
        let mut top: Vec<(u64, u64)> = self
            .top
            .iter()
            .copied()
            .filter(|&(_, rtt)| rtt > 0)
            .collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));
        top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_gaps_count_over_each_threshold() {
        let mut gap = LoopGap::default();
        assert_eq!(gap.pass(1_000), 0);
        assert_eq!(gap.pass(2_000), 1_000);
        assert_eq!(gap.pass(8_000), 6_000);
        assert_eq!(gap.pass(30_000), 22_000);
        assert_eq!(gap.pass(230_000), 200_000);
        assert_eq!(gap.max_ns, 200_000);
        assert_eq!((gap.over_5us, gap.over_20us, gap.over_100us), (3, 2, 1));
        gap.restart();
        assert_eq!(gap.pass(10_000_000), 0);
    }

    #[test]
    fn latency_percentiles_and_top_ten() -> Result<(), LlError> {
        let mut lat = Latency::new()?;
        for v in 1..=1_000_u64 {
            lat.record(v * 1_000, || v);
        }
        let p = lat.percentiles();
        assert_eq!(p.count, 1_000);
        // 3 significant figures: within 0.1 % of the exact value.
        let near = |got: u64, want: u64| got.abs_diff(want) * 1_000 <= want;
        assert!(near(p.p50, 500_000), "{p:?}");
        assert!(near(p.p90, 900_000), "{p:?}");
        assert!(near(p.p99, 990_000), "{p:?}");
        assert!(near(p.max, 1_000_000), "{p:?}");
        assert!(near(p.mean, 500_500), "{p:?}");
        let top = lat.top();
        assert_eq!(top.len(), TOP);
        assert_eq!(top[0], (1_000, 1_000_000));
        assert_eq!(top[9], (991, 991_000));
        Ok(())
    }

    #[test]
    fn max_is_the_measured_value_not_a_bucket_edge() -> Result<(), LlError> {
        // From tests/fixtures/e2e-docker-arm64-generic.txt: hdr reported
        // 14696447 (its bucket's edge) for a measured 14695216.
        let mut lat = Latency::new()?;
        lat.record(20_000, || 1);
        lat.record(14_695_216, || 2);
        assert_eq!(lat.percentiles().max, 14_695_216);
        assert_eq!(Latency::new()?.percentiles().max, 0);
        Ok(())
    }

    #[test]
    fn top_ten_ignores_empty_slots() -> Result<(), LlError> {
        let mut lat = Latency::new()?;
        lat.record(5, || 1);
        lat.record(7, || 2);
        assert_eq!(lat.top(), vec![(2, 7), (1, 5)]);
        Ok(())
    }

    #[test]
    fn counters_store_and_read() {
        let c = Counters::default();
        Counters::bump(&c.messages);
        Counters::bump(&c.messages);
        Counters::set(&c.max_gap_ns, 9);
        assert_eq!(Counters::get(&c.messages), 2);
        assert_eq!(Counters::get(&c.max_gap_ns), 9);
        assert_eq!(std::mem::align_of::<Padded<AtomicU64>>(), 128);
    }
}
