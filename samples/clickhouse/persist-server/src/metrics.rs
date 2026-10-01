//! Metrics from `persist_client::metrics`: two fixed tables, one row per
//! series per interval.
//!
//! A series' name and labels arrive once as a `MetricDef` message (saved
//! beside the checkpoint). A `Metrics` message carries series ids and
//! numbers. A `Histogram` message is one millisecond of count, sum, min, and
//! max per series. Those summaries are folded into a 5 s
//! [HdrHistogram](https://github.com/HdrHistogram/HdrHistogram) and written
//! as one row.
//!
//! | table | row |
//! |---|---|
//! | `metrics` | a counter's total and what the interval added, or a gauge's value |
//! | `metrics_histogram` | count, sum, min, max, avg, and p50 … p99.999 |
//!
//! Each millisecond records its minimum, its maximum, and the mean of the
//! rest (3 significant figures). `avg` is the exact sum divided by the
//! count. A busy millisecond pulls the percentiles toward that mean. Min and
//! max stay exact. Percentiles estimate the reconstructed distribution, not
//! the original samples. Rows have no buckets, so percentiles do not merge.
//! Counts, sums, minima, and maxima do:
//!
//! ```sql
//! SELECT sum(count) AS sample_count, sum(sum) / sum(count) AS mean,
//!        min(min) AS minimum, max(max) AS maximum
//! FROM metrics_histogram
//! WHERE name = 'tick_to_trade_ns' AND ts > now() - INTERVAL 1 HOUR
//! ```

use std::collections::HashMap;

use hdrhistogram::Histogram;
use persist_client::event::codec;
use persist_client::metrics::MetricDef;

use crate::table::{Column, DecodeError, Shape, write_string, write_varint};

/// Counters and gauges.
pub(crate) const METRICS: &str = "metrics";
/// Histograms.
pub(crate) const HISTOGRAMS: &str = "metrics_histogram";

/// The table a series' rows go to.
pub(crate) const fn table_of(kind: persist_client::metrics::MetricKind) -> &'static str {
    use persist_client::metrics::MetricKind;
    match kind {
        MetricKind::Counter | MetricKind::Gauge => METRICS,
        MetricKind::Histogram => HISTOGRAMS,
    }
}

/// How long one stored histogram row covers.
const WINDOW_NS: u64 = 5_000_000_000;

/// Bytes charged for one histogram row waiting to be inserted.
/// The open window is not charged: counting it holds the checkpoint.
pub(crate) const HIST_ROW_BYTES: usize = 256;

/// The percentiles each histogram row carries, low to high.
const QUANTILES: [(&str, f64); 7] = [
    ("p50", 0.5),
    ("p75", 0.75),
    ("p90", 0.9),
    ("p99", 0.99),
    ("p999", 0.999),
    ("p9999", 0.9999),
    ("p99999", 0.99999),
];

pub(crate) fn column(name: &str, ch_type: &str) -> Column {
    Column {
        name: name.into(),
        ch_type: ch_type.into(),
    }
}

const TS: &str = "DateTime64(9, 'UTC')";
const LABELS: &str = "Map(LowCardinality(String), String)";

pub(crate) fn metrics_shape() -> Shape {
    Shape {
        name: METRICS.into(),
        columns: vec![
            column("ts", TS),
            column("name", "LowCardinality(String)"),
            column("kind", "LowCardinality(String)"),
            column("series", "UInt64"),
            column("labels", LABELS),
            column("value", "Float64"),
            column("delta", "Nullable(Float64)"),
        ],
        order_by: vec!["name".into(), "series".into(), "ts".into()],
        partition: Some("ts".into()),
    }
}

pub(crate) fn histograms_shape() -> Shape {
    let mut columns = vec![
        column("ts", TS),
        column("name", "LowCardinality(String)"),
        column("series", "UInt64"),
        column("labels", LABELS),
        column("interval_ns", "UInt64"),
        column("count", "UInt64"),
        column("sum", "UInt64"),
        column("min", "UInt64"),
        column("max", "UInt64"),
        column("avg", "Float64"),
    ];
    columns.extend(QUANTILES.iter().map(|(name, _)| column(name, "Float64")));
    Shape {
        name: HISTOGRAMS.into(),
        columns,
        order_by: vec!["name".into(), "series".into(), "ts".into()],
        partition: Some("ts".into()),
    }
}

/// Writes the included columns of one RowBinary row, in order.
pub(crate) struct Row<'a> {
    include: &'a [bool],
    col: usize,
    pub(crate) out: &'a mut Vec<u8>,
}

impl<'a> Row<'a> {
    pub(crate) fn new(include: &'a [bool], out: &'a mut Vec<u8>) -> Self {
        Self {
            include,
            col: 0,
            out,
        }
    }

    pub(crate) fn put(&mut self, write: impl FnOnce(&mut Vec<u8>)) -> &mut Self {
        if self.include.get(self.col).copied().unwrap_or(false) {
            write(self.out);
        }
        self.col += 1;
        self
    }

    pub(crate) fn u64(&mut self, v: u64) -> &mut Self {
        self.put(|out| out.extend_from_slice(&v.to_le_bytes()))
    }

    pub(crate) fn f64(&mut self, v: f64) -> &mut Self {
        self.put(|out| out.extend_from_slice(&v.to_le_bytes()))
    }

    pub(crate) fn str(&mut self, v: &str) -> &mut Self {
        self.put(|out| write_string(v.as_bytes(), out))
    }

    pub(crate) fn map<'s>(
        &mut self,
        pairs: impl ExactSizeIterator<Item = (&'s str, &'s str)>,
    ) -> &mut Self {
        self.put(|out| {
            write_varint(pairs.len() as u64, out);
            for (k, v) in pairs {
                write_string(k.as_bytes(), out);
                write_string(v.as_bytes(), out);
            }
        })
    }

    /// The row's `host`, `pod` and `app`, already RowBinary.
    pub(crate) fn end(&mut self, origin: &[u8]) {
        self.out.extend_from_slice(origin);
    }
}

fn labels(def: &MetricDef) -> impl ExactSizeIterator<Item = (&str, &str)> {
    def.labels.iter().map(|(k, v)| (k.as_str(), v.as_str()))
}

/// Series of a `Metrics` or `Histogram` message whose `MetricDef` has not
/// arrived; `None` when the message is malformed.
pub(crate) fn unknown_series(message: &[u8], defs: &HashMap<u64, MetricDef>) -> Option<usize> {
    let template = u16::from_le_bytes(message.get(2..4)?.try_into().ok()?);
    if template == codec::HistogramEncoder::TEMPLATE_ID {
        let decoded = codec::HistogramDecoder::decode(message, 0).ok()?;
        let unknown = decoded
            .samples()
            .ok()?
            .filter(|entry| !defs.contains_key(&entry.series()))
            .count();
        return Some(unknown);
    }
    let m = codec::MetricsDecoder::decode(message, 0).ok()?;
    let counters = m
        .counters()
        .ok()?
        .filter(|e| !defs.contains_key(&e.series()));
    let gauges = m.gauges().ok()?.filter(|e| !defs.contains_key(&e.series()));
    Some(counters.count() + gauges.count())
}

/// Every counter and gauge of a `Metrics` message as a row of `metrics`.
/// Returns the rows written; a series with no def is skipped.
pub(crate) fn write_metrics(
    message: &[u8],
    include: &[bool],
    origin: &[u8],
    defs: &HashMap<u64, MetricDef>,
    out: &mut Vec<u8>,
) -> Result<usize, DecodeError> {
    let m = codec::MetricsDecoder::decode(message, 0)
        .map_err(|_| DecodeError("undecodable Metrics message"))?;
    let bad = |_| DecodeError("undecodable Metrics message");
    let mut rows = 0;
    for e in m.counters().map_err(bad)? {
        let Some(def) = defs.get(&e.series()) else {
            continue;
        };
        Row::new(include, out)
            .u64(m.ts())
            .str(&def.name)
            .str("counter")
            .u64(def.series)
            .map(labels(def))
            .f64(e.value() as f64)
            .put(|out| {
                out.push(0);
                out.extend_from_slice(&(e.delta() as f64).to_le_bytes());
            })
            .end(origin);
        rows += 1;
    }
    for e in m.gauges().map_err(bad)? {
        let Some(def) = defs.get(&e.series()) else {
            continue;
        };
        Row::new(include, out)
            .u64(m.ts())
            .str(&def.name)
            .str("gauge")
            .u64(def.series)
            .map(labels(def))
            .f64(e.value())
            .put(|out| out.push(1)) // no delta: NULL
            .end(origin);
        rows += 1;
    }
    Ok(rows)
}

/// One stored histogram row. `ts` is the end of its 5 s window.
pub(crate) struct Ready {
    pub(crate) source: u64,
    pub(crate) feed: bool,
    ts: u64,
    pub(crate) series: u64,
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
    avg: f64,
    percentiles: [f64; 7],
}

/// Append one folded histogram row. `origin` is `host`, `pod`, and `app`
/// already encoded, for the columns `include` kept.
pub(crate) fn write_ready(
    row: &Ready,
    include: &[bool],
    origin: &[u8],
    def: &MetricDef,
    out: &mut Vec<u8>,
) {
    let mut line = Row::new(include, out);
    line.u64(row.ts)
        .str(&def.name)
        .u64(def.series)
        .map(labels(def))
        .u64(WINDOW_NS)
        .u64(row.count)
        .u64(row.sum)
        .u64(row.min)
        .u64(row.max)
        .f64(row.avg);
    for value in row.percentiles {
        line.f64(value);
    }
    line.end(origin);
}

/// The end of the 5 s window that contains `ts`. A timestamp on a boundary
/// belongs to the window that ends there. `0` stays `0`.
fn window_end(ts: u64) -> u64 {
    match ts % WINDOW_NS {
        0 => ts,
        rem => ts + (WINDOW_NS - rem),
    }
}

/// One millisecond summary from a `Histogram` message.
#[derive(Clone, Copy)]
struct Sample {
    ts: u64,
    series: u64,
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
}

/// The open 5 s window for one series.
struct Acc {
    source: u64,
    feed: bool,
    window_end: u64,
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
    hist: Histogram<u64>,
}

fn new_hist() -> Histogram<u64> {
    match Histogram::<u64>::new(3) {
        Ok(hist) => hist,
        Err(err) => match Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3) {
            Ok(hist) => hist,
            Err(err2) => {
                panic!("HdrHistogram rejected 3 significant figures ({err}; {err2})")
            }
        },
    }
}

impl Acc {
    fn new(source: u64, feed: bool, window_end: u64) -> Self {
        Self {
            source,
            feed,
            window_end,
            count: 0,
            sum: 0,
            min: u64::MAX,
            max: 0,
            hist: new_hist(),
        }
    }

    /// Minimum, maximum, and `count - 2` copies of the mean of the rest.
    /// One sample records that value. Nothing is recorded when `count` is 0
    /// or the minimum is above the maximum.
    fn deposit(&mut self, count: u64, sum: u64, min: u64, max: u64) {
        if count == 0 || min > max {
            return;
        }
        self.count = self.count.wrapping_add(count);
        self.sum = self.sum.wrapping_add(sum);
        self.min = self.min.min(min);
        self.max = self.max.max(max);
        if count == 1 {
            self.add(min, 1);
            return;
        }
        self.add(min, 1);
        self.add(max, 1);
        let rest = count - 2;
        if let Some(mean) = sum.wrapping_sub(min).wrapping_sub(max).checked_div(rest) {
            self.add(mean, rest);
        }
    }

    fn add(&mut self, value: u64, n: u64) {
        if n == 0 {
            return;
        }
        if self.hist.record_n(value, n).is_err() {
            self.hist.saturating_record_n(value, n);
        }
    }

    fn into_ready(self, series: u64) -> Option<Ready> {
        if self.count == 0 {
            return None;
        }
        let avg = self.sum as f64 / self.count as f64;
        let percentiles = QUANTILES.map(|(_, q)| percentile(&self.hist, q, self.min, self.max));
        Some(Ready {
            source: self.source,
            feed: self.feed,
            ts: self.window_end,
            series,
            count: self.count,
            sum: self.sum,
            min: self.min,
            max: self.max,
            avg,
            percentiles,
        })
    }
}

fn accumulate(source: u64, feed: bool, end: u64, sample: &Sample) -> Acc {
    let mut acc = Acc::new(source, feed, end);
    acc.deposit(sample.count, sample.sum, sample.min, sample.max);
    acc
}

fn percentile(hist: &Histogram<u64>, q: f64, min: u64, max: u64) -> f64 {
    if hist.is_empty() {
        return min as f64;
    }
    (hist.value_at_quantile(q) as f64).clamp(min as f64, max as f64)
}

/// Open 5 s windows, keyed by source id and series.
///
/// [`Fold::ready`] is what the writer inserts. The open window is not part
/// of that list: a crash drops at most those 5 s.
#[derive(Default)]
pub(crate) struct Fold {
    open: HashMap<(u64, u64), Acc>,
    pub(crate) ready: Vec<Ready>,
}

impl Fold {
    /// Fold every sample in one `Histogram` message. `source` is the Aeron
    /// frame's reserved value. `feed` is a feed recording.
    pub(crate) fn push_message(
        &mut self,
        message: &[u8],
        source: u64,
        feed: bool,
    ) -> Result<(), DecodeError> {
        let decoded = codec::HistogramDecoder::decode(message, 0)
            .map_err(|_| DecodeError("undecodable Histogram message"))?;
        let ts = decoded.ts();
        for entry in decoded
            .samples()
            .map_err(|_| DecodeError("undecodable Histogram message"))?
        {
            self.push_summary(
                source,
                feed,
                Sample {
                    ts,
                    series: entry.series(),
                    count: entry.count(),
                    sum: entry.sum(),
                    min: entry.min(),
                    max: entry.max(),
                },
            );
        }
        Ok(())
    }

    fn push_summary(&mut self, source: u64, feed: bool, sample: Sample) {
        if sample.count == 0 || sample.min > sample.max {
            return;
        }
        let end = window_end(sample.ts);
        let key = (source, sample.series);
        let Some(open_end) = self.open.get(&key).map(|acc| acc.window_end) else {
            self.insert_new(source, feed, end, &sample);
            return;
        };
        if end > open_end {
            self.emit(key);
            self.insert_new(source, feed, end, &sample);
        } else if end < open_end {
            self.lone(source, feed, end, &sample);
        } else if let Some(acc) = self.open.get_mut(&key) {
            acc.deposit(sample.count, sample.sum, sample.min, sample.max);
        }
    }

    fn insert_new(&mut self, source: u64, feed: bool, end: u64, sample: &Sample) {
        let acc = accumulate(source, feed, end, sample);
        self.open.insert((source, sample.series), acc);
    }

    /// A sample older than the open window. Its own row, so the open window
    /// stays where it is.
    fn lone(&mut self, source: u64, feed: bool, end: u64, sample: &Sample) {
        let acc = accumulate(source, feed, end, sample);
        if let Some(row) = acc.into_ready(sample.series) {
            self.ready.push(row);
        }
    }

    fn emit(&mut self, key: (u64, u64)) {
        let Some(acc) = self.open.remove(&key) else {
            return;
        };
        if let Some(row) = acc.into_ready(key.1) {
            self.ready.push(row);
        }
    }

    /// Close every open window whose end `now` has reached. Does nothing
    /// until the replay is caught up, so a partial read does not publish a
    /// window that still has samples coming.
    pub(crate) fn flush_elapsed(&mut self, now: u64, caught_up: bool) {
        if !caught_up {
            return;
        }
        let finished: Vec<_> = self
            .open
            .extract_if(|_, acc| now >= acc.window_end)
            .collect();
        for (key, acc) in finished {
            if let Some(row) = acc.into_ready(key.1) {
                self.ready.push(row);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn at(ts: u64, series: u64, count: u64, sum: u64, min: u64, max: u64) -> Sample {
        Sample {
            ts,
            series,
            count,
            sum,
            min,
            max,
        }
    }

    fn only(fold: &Fold) -> Result<&Ready, Box<dyn std::error::Error>> {
        let [row] = fold.ready.as_slice() else {
            return Err(format!("{} rows", fold.ready.len()).into());
        };
        Ok(row)
    }

    fn within(row: &Ready) -> bool {
        row.percentiles
            .iter()
            .all(|p| *p >= row.min as f64 && *p <= row.max as f64)
    }

    #[test]
    fn one_busy_millisecond_pulls_percentiles_toward_the_mean() -> TestResult {
        let mut fold = Fold::default();
        // 1..=1000 each recorded as k * 1000, in one millisecond.
        fold.push_summary(
            0,
            false,
            at(1_000_000, 1, 1000, 500_500_000, 1000, 1_000_000),
        );
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert_eq!(
            (row.count, row.sum, row.min, row.max, row.avg),
            (1000, 500_500_000, 1000, 1_000_000, 500_500.0)
        );
        let mean = &row.percentiles[..5];
        assert!(
            mean.windows(2).all(|w| w[0] == w[1]),
            "p50 through p99.9 are the same mass: {mean:?}"
        );
        let scale = 500_500.0;
        assert!(
            (mean[0] - scale).abs() <= scale * 0.001,
            "{mean:?} is not within 0.1% of {scale}"
        );
        assert_eq!(row.percentiles[5], 1_000_000.0);
        assert_eq!(row.percentiles[6], 1_000_000.0);
        assert!(within(row), "{:?}", row.percentiles);
        Ok(())
    }

    #[test]
    fn a_zero_sample_stays_zero() -> TestResult {
        let mut fold = Fold::default();
        fold.push_summary(0, false, at(1, 1, 1, 0, 0, 0));
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert!(
            row.percentiles.iter().all(|p| *p == 0.0),
            "a zero clamped up to 1: {:?}",
            row.percentiles
        );

        let mut fold = Fold::default();
        fold.push_summary(0, false, at(1, 1, 2, 10, 0, 10));
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert_eq!(
            row.percentiles[0], 0.0,
            "the lower half is the zero, not 1: {:?}",
            row.percentiles
        );
        assert!(within(row), "{:?}", row.percentiles);
        Ok(())
    }

    #[test]
    fn summaries_in_one_window_merge_exactly() -> TestResult {
        let mut fold = Fold::default();
        fold.push_summary(1, false, at(1_000_000, 3, 4, 100, 10, 40));
        fold.push_summary(1, false, at(2_000_000, 3, 2, 27, 5, 50));
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert_eq!((row.count, row.sum, row.min, row.max), (6, 127, 5, 50));
        assert_eq!(row.avg, 127.0 / 6.0);
        assert_eq!(row.ts, WINDOW_NS);
        assert!(within(row), "{:?}", row.percentiles);
        Ok(())
    }

    #[test]
    fn an_open_window_flushes_only_when_caught_up_past_its_end() -> TestResult {
        let mut fold = Fold::default();
        fold.push_summary(1, false, at(1, 3, 1, 4, 4, 4));
        fold.flush_elapsed(WINDOW_NS, false);
        assert!(fold.ready.is_empty(), "not caught up");
        fold.flush_elapsed(WINDOW_NS - 1, true);
        assert!(fold.ready.is_empty(), "the window has not ended");
        assert_eq!(
            fold.open.get(&(1, 3)).map(|acc| acc.window_end),
            Some(WINDOW_NS)
        );
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert_eq!(row.ts, WINDOW_NS);
        assert!(fold.open.is_empty());
        Ok(())
    }

    #[test]
    fn a_late_sample_after_a_newer_window_is_its_own_row() -> TestResult {
        let mut fold = Fold::default();
        fold.push_summary(1, false, at(6_000_000_000, 3, 1, 8, 8, 8));
        assert!(fold.ready.is_empty());
        assert_eq!(
            fold.open.get(&(1, 3)).map(|acc| acc.window_end),
            Some(10_000_000_000)
        );
        fold.push_summary(1, true, at(1_000_000, 3, 1, 3, 3, 3));
        let row = only(&fold)?;
        assert_eq!(row.ts, WINDOW_NS);
        assert_eq!((row.count, row.sum), (1, 3));
        assert!(row.feed);
        assert_eq!(
            fold.open
                .get(&(1, 3))
                .map(|acc| (acc.window_end, acc.count)),
            Some((10_000_000_000, 1))
        );
        Ok(())
    }

    #[test]
    fn one_value_lands_on_every_percentile() -> TestResult {
        let mut fold = Fold::default();
        fold.push_summary(0, false, at(1, 1, 1, 500_500, 500_500, 500_500));
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert_eq!(row.count, 1);
        assert_eq!(row.avg, 500_500.0);
        assert!(
            row.percentiles.iter().all(|p| *p == 500_500.0),
            "{:?}",
            row.percentiles
        );
        Ok(())
    }

    #[test]
    fn an_empty_summary_adds_nothing() {
        let mut fold = Fold::default();
        fold.push_summary(0, false, at(1, 1, 0, 0, 0, 0));
        fold.push_summary(0, false, at(1, 1, 2, 3, 5, 1));
        fold.flush_elapsed(u64::MAX, true);
        assert!(fold.ready.is_empty());
        assert!(fold.open.is_empty());
    }

    #[test]
    fn a_histogram_message_folds() -> TestResult {
        let mut buf = [0u8; codec::HistogramEncoder::compute_length_with_header(1)];
        let len = codec::HistogramEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&codec::HistogramFixedFields {
                ts: 1_000_000,
                interval: 1_000_000,
            })
            .samples(1, |group| {
                group.add(|entry| {
                    entry.series(7).count(2).sum(30).min(10).max(20);
                    Ok(())
                })?;
                Ok(())
            })?
            .encoded_length_with_header();
        let mut fold = Fold::default();
        fold.push_message(&buf[..len], 4, true)
            .map_err(|err| err.0)?;
        fold.flush_elapsed(WINDOW_NS, true);
        let row = only(&fold)?;
        assert_eq!(row.source, 4);
        assert!(row.feed);
        assert_eq!(row.series, 7);
        assert_eq!((row.count, row.sum, row.min, row.max), (2, 30, 10, 20));
        assert_eq!(row.avg, 15.0);
        assert!(within(row), "{:?}", row.percentiles);
        Ok(())
    }
}
