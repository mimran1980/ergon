//! Metrics from `persist_client::metrics`: two fixed tables, one row per
//! series per interval.
//!
//! A series' name and labels arrive once as a `MetricDef` message (saved
//! beside the checkpoint); each `Metrics` and `Histogram` message carries
//! only series ids and numbers, and is written out with the names.
//!
//! | table | row |
//! |---|---|
//! | `metrics` | a counter's total and what the interval added, or a gauge's value |
//! | `metrics_histogram` | a histogram's count, sum, min, max, p50…p99.99, and its non-empty buckets |
//!
//! Percentiles come from the buckets, so they are within the histogram's
//! precision (3.1% by default) and clamped to the exact min and max. To
//! combine intervals or applications, merge the buckets rather than the
//! percentiles:
//!
//! ```sql
//! SELECT quantileExactWeighted(0.99)(le, c)
//! FROM metrics_histogram ARRAY JOIN buckets.le AS le, buckets.count AS c
//! WHERE name = 'tick_to_trade_ns' AND ts > now() - INTERVAL 1 HOUR
//! ```

use std::collections::HashMap;

use persist_client::event::codec;
use persist_client::metrics::{MetricDef, bucket_bounds};

use crate::table::{Column, DecodeError, Shape, write_string, write_varint};

/// Counters and gauges.
pub(crate) const METRICS: &str = "metrics";
/// Histograms.
pub(crate) const HISTOGRAMS: &str = "metrics_histogram";

/// The percentiles each histogram row carries.
const QUANTILES: [(&str, f64); 5] = [
    ("p50", 0.5),
    ("p90", 0.9),
    ("p99", 0.99),
    ("p999", 0.999),
    ("p9999", 0.9999),
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
    ];
    columns.extend(QUANTILES.iter().map(|(name, _)| column(name, "Float64")));
    columns.push(column("buckets.le", "Array(UInt64)"));
    columns.push(column("buckets.count", "Array(UInt64)"));
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
        let h = codec::HistogramDecoder::decode(message, 0).ok()?;
        return Some(usize::from(!defs.contains_key(&h.series())));
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

/// A `Histogram` message as a row of `metrics_histogram`.
pub(crate) fn write_histogram(
    message: &[u8],
    include: &[bool],
    origin: &[u8],
    defs: &HashMap<u64, MetricDef>,
    out: &mut Vec<u8>,
) -> Result<usize, DecodeError> {
    let h = codec::HistogramDecoder::decode(message, 0)
        .map_err(|_| DecodeError("undecodable Histogram message"))?;
    let Some(def) = defs.get(&h.series()) else {
        return Ok(0);
    };
    let precision = h.precision();
    let buckets: Vec<(u64, u64, u64)> = h
        .buckets()
        .map_err(|_| DecodeError("undecodable Histogram message"))?
        .map(|e| {
            let (low, high) = bucket_bounds(usize::from(e.index()), precision);
            (low, high, e.count())
        })
        .collect();
    let (count, min, max) = (h.count(), h.min(), h.max());
    let mut row = Row::new(include, out);
    row.u64(h.ts())
        .str(&def.name)
        .u64(def.series)
        .map(labels(def))
        .u64(h.interval())
        .u64(count)
        .u64(h.sum())
        .u64(min)
        .u64(max);
    for (_, q) in QUANTILES {
        row.f64(quantile(&buckets, count, q, min, max));
    }
    row.put(|out| {
        write_varint(buckets.len() as u64, out);
        for (_, high, _) in &buckets {
            out.extend_from_slice(&high.to_le_bytes());
        }
    })
    .put(|out| {
        write_varint(buckets.len() as u64, out);
        for (.., c) in &buckets {
            out.extend_from_slice(&c.to_le_bytes());
        }
    })
    .end(origin);
    Ok(1)
}

/// The value at quantile `q`: the middle of the bucket holding that rank,
/// clamped to the exact `min` and `max`.
fn quantile(buckets: &[(u64, u64, u64)], count: u64, q: f64, min: u64, max: u64) -> f64 {
    let rank = ((q * count as f64).ceil() as u64).max(1);
    let mut seen = 0;
    for &(low, high, c) in buckets {
        seen += c;
        if seen >= rank {
            return (low as f64 / 2.0 + high as f64 / 2.0).clamp(min as f64, max as f64);
        }
    }
    max as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_come_from_the_bucket_holding_the_rank() {
        // 90 values in 100..=103, 9 in 1000..=1031, 1 at 49_500.
        let buckets = [(100, 103, 90), (1000, 1031, 9), (49_152, 50_175, 1)];
        let q = |q| quantile(&buckets, 100, q, 100, 49_500);
        assert_eq!(q(0.5), 101.5);
        assert_eq!(q(0.9), 101.5);
        assert_eq!(q(0.95), 1015.5);
        assert_eq!(q(0.99), 1015.5);
        assert_eq!(
            q(0.999),
            49_500.0,
            "the bucket's middle, clamped to the exact max"
        );
        assert_eq!(quantile(&[(0, 0, 1)], 1, 0.5, 0, 0), 0.0);
    }
}
