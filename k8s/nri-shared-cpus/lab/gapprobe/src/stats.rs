//! Loop-gap statistics: a log-linear histogram of every iteration (exact
//! below 1,024 ns, then 64 sub-buckets per power of two, so a percentile is
//! at most 1/64 above the true value) and the exact max.

/// Values below this are counted exactly, one bucket each.
const LINEAR: u64 = 1024;
/// log2 of `LINEAR`.
const LINEAR_BITS: u32 = 10;
/// Sub-buckets per power of two above `LINEAR`: 2^6 = 64.
const SUB_BITS: u32 = 6;
/// Buckets: `LINEAR` exact ones, then 64 for each power of two from 2^10 to 2^63.
const BUCKETS: usize = LINEAR as usize + ((64 - LINEAR_BITS) as usize) * (1 << SUB_BITS);

/// A histogram of loop iterations in nanoseconds.
#[derive(Clone, Debug)]
pub struct Histogram {
    counts: Vec<u64>,
    total: u64,
    max: u64,
}

/// The percentiles a run reports, in nanoseconds; `max` is exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// iterations recorded
    pub samples: u64,
    /// median
    pub p50: u64,
    /// 90th percentile
    pub p90: u64,
    /// 99th percentile
    pub p99: u64,
    /// 99.9th percentile
    pub p999: u64,
    /// 99.99th percentile
    pub p9999: u64,
    /// the largest iteration, exact
    pub max: u64,
}

impl Summary {
    /// Every value but `samples` through `f`: the histogram counts ticks,
    /// the report shows nanoseconds.
    #[must_use]
    pub fn map(self, f: impl Fn(u64) -> u64) -> Self {
        Self {
            samples: self.samples,
            p50: f(self.p50),
            p90: f(self.p90),
            p99: f(self.p99),
            p999: f(self.p999),
            p9999: f(self.p9999),
            max: f(self.max),
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// The bucket of `v`.
#[inline]
fn bucket(v: u64) -> usize {
    if v < LINEAR {
        return v as usize;
    }
    let e = v.ilog2();
    let sub = (v >> (e - SUB_BITS)) & ((1 << SUB_BITS) - 1);
    LINEAR as usize + ((e - LINEAR_BITS) as usize) * (1 << SUB_BITS) + sub as usize
}

/// The largest value bucket `i` holds.
fn upper(i: usize) -> u64 {
    if i < LINEAR as usize {
        return i as u64;
    }
    let k = i - LINEAR as usize;
    let e = LINEAR_BITS + (k >> SUB_BITS) as u32;
    let sub = (k & ((1 << SUB_BITS) - 1)) as u64;
    let width = 1_u64 << (e - SUB_BITS);
    ((1_u64 << SUB_BITS) + sub)
        .saturating_mul(width)
        .saturating_add(width - 1)
}

impl Histogram {
    /// An empty histogram.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counts: vec![0; BUCKETS],
            total: 0,
            max: 0,
        }
    }

    /// Records one iteration of `ns` nanoseconds.
    #[inline]
    pub fn record(&mut self, ns: u64) {
        if let Some(c) = self.counts.get_mut(bucket(ns)) {
            *c += 1;
        }
        self.total += 1;
        if ns > self.max {
            self.max = ns;
        }
    }

    /// Iterations recorded.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// The exact max, or `None` when empty.
    #[must_use]
    pub fn max(&self) -> Option<u64> {
        (self.total > 0).then_some(self.max)
    }

    /// The `q` quantile (0 < q <= 1, to the millionth): the upper bound of
    /// the bucket holding the ceil(q x total)-th iteration, never above the
    /// max. `None` when empty or `q` is outside (0, 1].
    #[must_use]
    pub fn quantile(&self, q: f64) -> Option<u64> {
        if !(q > 0.0 && q <= 1.0) {
            return None;
        }
        let ppm = (q * 1e6).round() as u64;
        self.quantile_ppm(ppm.max(1))
    }

    /// The quantile at `ppm` parts per million (1 to 1,000,000).
    #[must_use]
    pub fn quantile_ppm(&self, ppm: u64) -> Option<u64> {
        if self.total == 0 || ppm == 0 || ppm > 1_000_000 {
            return None;
        }
        let rank = (u128::from(self.total) * u128::from(ppm)).div_ceil(1_000_000) as u64;
        let mut seen = 0_u64;
        for (i, c) in self.counts.iter().enumerate() {
            seen += c;
            if seen >= rank {
                return Some(upper(i).min(self.max));
            }
        }
        Some(self.max)
    }

    /// p50, p90, p99, p99.9, p99.99 and the max, or `None` when empty.
    #[must_use]
    pub fn summary(&self) -> Option<Summary> {
        Some(Summary {
            samples: self.total,
            p50: self.quantile_ppm(500_000)?,
            p90: self.quantile_ppm(900_000)?,
            p99: self.quantile_ppm(990_000)?,
            p999: self.quantile_ppm(999_000)?,
            p9999: self.quantile_ppm(999_900)?,
            max: self.max()?,
        })
    }

    /// Adds another histogram's iterations.
    pub fn merge(&mut self, other: &Self) {
        for (a, b) in self.counts.iter_mut().zip(&other.counts) {
            *a += b;
        }
        self.total += other.total;
        self.max = self.max.max(other.max);
    }

    /// The non-empty buckets as (upper bound, count), ascending.
    #[must_use]
    pub fn buckets(&self) -> Vec<(u64, u64)> {
        self.counts
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(i, c)| (upper(i), *c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_has_no_summary() {
        let h = Histogram::new();
        assert_eq!(h.total(), 0);
        assert_eq!(h.max(), None);
        assert_eq!(h.quantile(0.5), None);
        assert_eq!(h.summary(), None);
    }

    #[test]
    fn exact_below_1024_ns() {
        let mut h = Histogram::new();
        for ns in 1..=1000 {
            h.record(ns);
        }
        assert_eq!(h.total(), 1000);
        assert_eq!(h.quantile(0.5), Some(500));
        assert_eq!(h.quantile(0.9), Some(900));
        assert_eq!(h.quantile(0.99), Some(990));
        assert_eq!(h.quantile(0.999), Some(999));
        assert_eq!(h.quantile(1.0), Some(1000));
        assert_eq!(h.max(), Some(1000));
    }

    #[test]
    fn quantile_out_of_range_is_none() {
        let mut h = Histogram::new();
        h.record(10);
        assert_eq!(h.quantile(0.0), None);
        assert_eq!(h.quantile(1.5), None);
        assert_eq!(h.quantile(f64::NAN), None);
    }

    #[test]
    fn max_is_exact_and_caps_the_bucket_bound() {
        let mut h = Histogram::new();
        h.record(40);
        h.record(1_234_567);
        assert_eq!(h.max(), Some(1_234_567));
        // the bucket's upper bound is above 1,234,567: the max caps it
        assert_eq!(h.quantile(1.0), Some(1_234_567));
    }

    #[test]
    fn large_values_within_1_6_percent_above() {
        for v in [
            1024_u64,
            1500,
            4096,
            10_000,
            99_999,
            1_000_000,
            123_456_789,
            u64::MAX / 3,
        ] {
            let mut h = Histogram::new();
            h.record(v);
            h.record(1);
            // the 50th percentile is the 1 ns sample, the 100th the large one,
            // reported as its bucket's bound: never below, at most 1/64 above
            let q = h.quantile(1.0);
            assert_eq!(q, Some(v), "max caps the bound for {v}");
            let mut h2 = Histogram::new();
            h2.record(v);
            h2.record(v.saturating_mul(2));
            let p50 = h2.quantile(0.5).unwrap_or(0);
            assert!(p50 >= v, "{p50} below {v}");
            assert!(
                (p50 - v) as f64 <= v as f64 / 64.0,
                "{p50} more than 1/64 above {v}"
            );
        }
    }

    #[test]
    fn tail_percentiles_of_a_spinner() {
        // 999,000 iterations of 30 ns, 900 of 2 us, 90 of 20 us, 9 of 200 us
        // and one of 5 ms: the shape a quiet CPU with a few interrupts gives
        let mut h = Histogram::new();
        for _ in 0..999_000 {
            h.record(30);
        }
        for _ in 0..900 {
            h.record(2_000);
        }
        for _ in 0..90 {
            h.record(20_000);
        }
        for _ in 0..9 {
            h.record(200_000);
        }
        h.record(5_000_000);
        let s = h.summary().unwrap_or_default();
        assert_eq!(s.samples, 1_000_000);
        assert_eq!(s.p50, 30);
        assert_eq!(s.p90, 30);
        assert_eq!(s.p99, 30);
        // the 999,000th is the last 30 ns one; the 999,900th the last 2 us
        assert_eq!(s.p999, 30);
        assert!((2_000..=2_032).contains(&s.p9999), "p99.99 {}", s.p9999);
        assert_eq!(s.max, 5_000_000);
    }

    #[test]
    fn summary_maps_values_not_samples() {
        let s = Summary {
            samples: 9,
            p50: 1,
            p90: 2,
            p99: 3,
            p999: 4,
            p9999: 5,
            max: 6,
        };
        assert_eq!(
            s.map(|v| v * 10),
            Summary {
                samples: 9,
                p50: 10,
                p90: 20,
                p99: 30,
                p999: 40,
                p9999: 50,
                max: 60
            }
        );
    }

    #[test]
    fn merge_adds_counts_and_keeps_the_larger_max() {
        let mut a = Histogram::new();
        let mut b = Histogram::new();
        for _ in 0..3 {
            a.record(10);
        }
        b.record(500);
        b.record(7_000);
        a.merge(&b);
        assert_eq!(a.total(), 5);
        assert_eq!(a.max(), Some(7_000));
        assert_eq!(a.quantile(0.6), Some(10));
        assert_eq!(a.quantile(0.8), Some(500));
    }
}
