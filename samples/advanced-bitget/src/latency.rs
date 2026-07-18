//! Feed latency DynamicSchema / DynamicRow helpers for the HA sample.
//!
//! Registers a runtime latency table via persist's `DynamicRecorderV2` and
//! encodes one row per accepted book apply. Hot path: positional values into
//! a caller buffer (typically an Aeron claim); drop-on-backpressure is the
//! caller's responsibility.

use ergo_clickhouse_persist::ColumnType;
use ergo_clickhouse_persist::dynamic::{
    DynamicRecorderBuilder, DynamicRecorderError, DynamicRecorderV2, DynamicValueRef,
};

/// Suggested ClickHouse / dynamic table name.
pub const FEED_LATENCY_TABLE: &str = "feed_latency";

/// Timestamps and identities for one latency sample (nanoseconds as provided
/// by the caller — this helper does not sample clocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencySample {
    pub leadership_term_id: i64,
    pub cluster_session_id: i64,
    pub sequence: u64,
    pub exchange_ts_ns: u64,
    pub receive_ts_ns: u64,
    pub ingress_claim_ts_ns: u64,
    pub egress_decode_ts_ns: u64,
    pub book_apply_ts_ns: u64,
    pub ch_enqueue_ts_ns: u64,
}

impl LatencySample {
    #[must_use]
    pub fn exchange_to_receive_ns(self) -> i64 {
        delta_ns(self.exchange_ts_ns, self.receive_ts_ns)
    }

    #[must_use]
    pub fn receive_to_claim_ns(self) -> i64 {
        delta_ns(self.receive_ts_ns, self.ingress_claim_ts_ns)
    }

    #[must_use]
    pub fn claim_to_egress_ns(self) -> i64 {
        delta_ns(self.ingress_claim_ts_ns, self.egress_decode_ts_ns)
    }

    #[must_use]
    pub fn e2e_ns(self) -> i64 {
        delta_ns(self.exchange_ts_ns, self.book_apply_ts_ns)
    }
}

fn delta_ns(start: u64, end: u64) -> i64 {
    if end >= start {
        (end - start) as i64
    } else {
        -((start - end) as i64)
    }
}

/// Build the feed_latency DynamicRecorderV2 (schema published separately by
/// the dynamic stream owner, same pattern as l2book_dynamic).
pub fn build_feed_latency_recorder() -> Result<DynamicRecorderV2, DynamicRecorderError> {
    DynamicRecorderBuilder::new(FEED_LATENCY_TABLE)
        .field("instrument", ColumnType::String)
        .field("leadership_term_id", ColumnType::Int64)
        .field("cluster_session_id", ColumnType::Int64)
        .field("sequence", ColumnType::UInt64)
        .field("exchange_ts_ns", ColumnType::UInt64)
        .field("receive_ts_ns", ColumnType::UInt64)
        .field("ingress_claim_ts_ns", ColumnType::UInt64)
        .field("egress_decode_ts_ns", ColumnType::UInt64)
        .field("book_apply_ts_ns", ColumnType::UInt64)
        .field("ch_enqueue_ts_ns", ColumnType::UInt64)
        .field("exchange_to_receive_ns", ColumnType::Int64)
        .field("receive_to_claim_ns", ColumnType::Int64)
        .field("claim_to_egress_ns", ColumnType::Int64)
        .field("e2e_ns", ColumnType::Int64)
        .build_v2()
}

/// Encode one latency row into `dst`. Values are positional and match
/// [`build_feed_latency_recorder`] field order.
pub fn record_latency_row<'a>(
    recorder: &DynamicRecorderV2,
    dst: &'a mut [u8],
    instrument: &str,
    sample: LatencySample,
) -> Result<&'a [u8], DynamicRecorderError> {
    let values = [
        DynamicValueRef::String(instrument),
        DynamicValueRef::Int64(sample.leadership_term_id),
        DynamicValueRef::Int64(sample.cluster_session_id),
        DynamicValueRef::UInt64(sample.sequence),
        DynamicValueRef::UInt64(sample.exchange_ts_ns),
        DynamicValueRef::UInt64(sample.receive_ts_ns),
        DynamicValueRef::UInt64(sample.ingress_claim_ts_ns),
        DynamicValueRef::UInt64(sample.egress_decode_ts_ns),
        DynamicValueRef::UInt64(sample.book_apply_ts_ns),
        DynamicValueRef::UInt64(sample.ch_enqueue_ts_ns),
        DynamicValueRef::Int64(sample.exchange_to_receive_ns()),
        DynamicValueRef::Int64(sample.receive_to_claim_ns()),
        DynamicValueRef::Int64(sample.claim_to_egress_ns()),
        DynamicValueRef::Int64(sample.e2e_ns()),
    ];
    recorder.record_into(dst, &values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas_are_non_negative_when_ordered() {
        let s = LatencySample {
            leadership_term_id: 1,
            cluster_session_id: 2,
            sequence: 3,
            exchange_ts_ns: 1_000,
            receive_ts_ns: 1_100,
            ingress_claim_ts_ns: 1_200,
            egress_decode_ts_ns: 1_500,
            book_apply_ts_ns: 1_600,
            ch_enqueue_ts_ns: 1_700,
        };
        assert_eq!(s.exchange_to_receive_ns(), 100);
        assert_eq!(s.receive_to_claim_ns(), 100);
        assert_eq!(s.claim_to_egress_ns(), 300);
        assert_eq!(s.e2e_ns(), 600);
    }

    #[test]
    fn recorder_builds_and_encodes_row() -> Result<(), Box<dyn std::error::Error>> {
        let rec = build_feed_latency_recorder()?;
        let s = LatencySample {
            leadership_term_id: 9,
            cluster_session_id: 42,
            sequence: 7,
            exchange_ts_ns: 10,
            receive_ts_ns: 20,
            ingress_claim_ts_ns: 30,
            egress_decode_ts_ns: 40,
            book_apply_ts_ns: 50,
            ch_enqueue_ts_ns: 60,
        };
        let mut buf = vec![0u8; 4096];
        let bytes = record_latency_row(&rec, &mut buf, "BTCUSDT", s)?;
        assert!(bytes.len() > 8, "encoded SBE frame should include header");
        Ok(())
    }
}
