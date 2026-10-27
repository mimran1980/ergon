//! Latency histograms.
//!
//! `HdrHistogram` (3 significant digits, 1 ns to 60 s) for the percentiles, with the exact min and max beside it, and a count of
//! censored values (only known to be at least what was recorded).
//!
//! A point keeps two ([`PointLatency`]): from each message's intended send
//! time (the schedule's: no coordinated omission, the one the row reports)
//! and from its actual send time (the transport's alone).

use crate::Error;
use crate::clock::TscClock;
use crate::message::Message;
use hdrhistogram::Histogram;

/// The highest latency the histogram tracks. A longer one is recorded at
/// this bound and counted censored, so a max at the bound is a floor.
pub const HISTOGRAM_MAX_NS: u64 = 60_000_000_000;

/// One histogram's numbers, in ns.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Summary {
    /// Median.
    pub p50: u64,
    /// 90th percentile.
    pub p90: u64,
    /// 99th percentile.
    pub p99: u64,
    /// 99.9th percentile.
    pub p999: u64,
    /// 99.99th percentile.
    pub p9999: u64,
    /// The exact largest value recorded (or the bound, for a censored one).
    pub max: u64,
    /// The exact smallest value recorded.
    pub min: u64,
    /// Mean.
    pub mean: f64,
    /// Values recorded.
    pub count: u64,
    /// Of them, censored: at the bound, or owed and recorded at a lower bound.
    pub censored: u64,
}

/// A histogram with an exact min and max. Recording never allocates.
#[derive(Debug, Clone)]
pub struct Latency {
    hist: Histogram<u64>,
    min: u64,
    max: u64,
    censored: u64,
}

impl Latency {
    /// Empty, 1 ns to [`HISTOGRAM_MAX_NS`] at 3 significant digits.
    ///
    /// # Errors
    /// [`Error::Histogram`] if `HdrHistogram` refuses the bounds.
    pub fn new() -> Result<Self, Error> {
        let hist = Histogram::new_with_bounds(1, HISTOGRAM_MAX_NS, 3)
            .map_err(|e| Error::Histogram(e.to_string()))?;
        Ok(Self {
            hist,
            min: u64::MAX,
            max: 0,
            censored: 0,
        })
    }

    #[inline]
    fn put(&mut self, ns: u64) {
        let ns = ns.min(HISTOGRAM_MAX_NS);
        self.hist.saturating_record(ns);
        self.min = self.min.min(ns);
        self.max = self.max.max(ns);
    }

    /// Records `ns`; at or past the bound it is kept at the bound and counted
    /// censored.
    #[inline]
    pub fn record(&mut self, ns: u64) {
        if ns >= HISTOGRAM_MAX_NS {
            self.censored += 1;
        }
        self.put(ns);
    }

    /// Records `ns` as a lower bound (a message owed without a reply),
    /// counted censored.
    #[inline]
    pub fn record_censored(&mut self, ns: u64) {
        self.censored += 1;
        self.put(ns);
    }

    /// The percentiles, each within the exact min and max.
    #[must_use]
    pub fn summary(&self) -> Summary {
        let count = self.hist.len();
        if count == 0 {
            return Summary::default();
        }
        let q = |quantile| {
            self.hist
                .value_at_quantile(quantile)
                .clamp(self.min, self.max)
        };
        Summary {
            p50: q(0.5),
            p90: q(0.9),
            p99: q(0.99),
            p999: q(0.999),
            p9999: q(0.9999),
            max: self.max,
            min: self.min,
            mean: self.hist.mean(),
            count,
            censored: self.censored,
        }
    }

    /// The histogram as `value_ns,count` rows, one per recorded value, under
    /// that header (analyse.py merges a point's reps from these).
    #[must_use]
    pub fn csv(&self) -> String {
        use std::fmt::Write;
        let mut rows = String::from("value_ns,count\n");
        for v in self.hist.iter_recorded() {
            // writing to a String cannot fail
            let _ = writeln!(rows, "{},{}", v.value_iterated_to(), v.count_at_value());
        }
        rows
    }
}

/// A point's two histograms.
#[derive(Debug, Clone)]
pub struct PointLatency {
    /// From each message's intended send time: what the row reports.
    pub from_intended: Latency,
    /// From each message's actual send time.
    pub from_send: Latency,
}

impl PointLatency {
    /// Both empty.
    ///
    /// # Errors
    /// As [`Latency::new`].
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            from_intended: Latency::new()?,
            from_send: Latency::new()?,
        })
    }

    /// `m` (an echo, or a one-way message) arrived at raw `recv`.
    #[inline]
    pub fn record(&mut self, clock: &TscClock, m: &Message, recv: u64) {
        self.from_intended
            .record(clock.delta_ns(m.intended_tsc, recv));
        self.from_send.record(clock.delta_ns(m.send_tsc, recv));
    }

    /// A measured message owed at the run's raw `end` with no reply: due at
    /// `intended`, sent at `send` (`None`: never sent). Recorded censored at
    /// its lower bound, `end` minus each start; one never sent has no send
    /// time, so only the intended histogram holds it.
    #[inline]
    pub fn record_owed(&mut self, clock: &TscClock, intended: u64, send: Option<u64>, end: u64) {
        self.from_intended
            .record_censored(clock.delta_ns(intended, end));
        if let Some(send) = send {
            self.from_send.record_censored(clock.delta_ns(send, end));
        }
    }
}
