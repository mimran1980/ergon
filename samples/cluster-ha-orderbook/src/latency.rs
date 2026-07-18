//! Feed latency DynamicSchema / DynamicRow encode + persist to ClickHouse.
//!
//! Hot path: [`DynamicRecorder::record`] (V1 SBE) → decode via
//! [`SchemaRegistry`]/[`RowDecoder`] → insert through [`ClickhouseSink`].
//! Values that land in CH must come from the decoded DynamicRow, not a
//! parallel SQL string built from the in-memory sample.

use std::cell::RefCell;
use std::rc::Rc;

use ergo_clickhouse_persist::consumer::{RowDecoder, SchemaRegistry, column_type_to_tag};
use ergo_clickhouse_persist::dynamic::{
    DynamicRecorder, DynamicRecorderBuilder, DynamicRecorderError, DynamicValue,
};
use ergo_clickhouse_persist::sbe::{DynamicRowDecoder, DynamicSchemaDecoder, DynamicSchemaEncoder};
use ergo_clickhouse_persist::{
    ClickhouseSink, ClickhouseSinkBuilder, ColumnType, PersistSender, SinkError,
};
use ergo_clickhouse_persist_derive::Persist;
use serde::Serialize;

/// Suggested ClickHouse / dynamic table name.
pub const FEED_LATENCY_TABLE: &str = "feed_latency";

/// Column layout shared by recorder, schema announcement, and Persist DTO.
const LATENCY_FIELDS: &[(&str, ColumnType)] = &[
    ("instrument", ColumnType::String),
    ("leadership_term_id", ColumnType::Int64),
    ("cluster_session_id", ColumnType::Int64),
    ("sequence", ColumnType::UInt64),
    ("exchange_ts_ns", ColumnType::UInt64),
    ("receive_ts_ns", ColumnType::UInt64),
    ("ingress_claim_ts_ns", ColumnType::UInt64),
    ("egress_decode_ts_ns", ColumnType::UInt64),
    ("book_apply_ts_ns", ColumnType::UInt64),
    ("ch_enqueue_ts_ns", ColumnType::UInt64),
    ("exchange_to_receive_ns", ColumnType::Int64),
    ("receive_to_claim_ns", ColumnType::Int64),
    ("claim_to_egress_ns", ColumnType::Int64),
    ("e2e_ns", ColumnType::Int64),
];

/// Timestamps and identities for one latency sample.
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

/// Build the feed_latency DynamicRecorder (V1 — SchemaRegistry-compatible).
pub fn build_feed_latency_recorder() -> Result<DynamicRecorder, DynamicRecorderError> {
    let mut b = DynamicRecorderBuilder::new(FEED_LATENCY_TABLE);
    for (name, ty) in LATENCY_FIELDS {
        b = b.field(*name, ty.clone());
    }
    b.build()
}

/// Encode one latency DynamicRow into the recorder buffer.
pub fn record_latency_row<'a>(
    recorder: &'a mut DynamicRecorder,
    instrument: &str,
    sample: LatencySample,
) -> Result<&'a [u8], DynamicRecorderError> {
    let values = [
        DynamicValue::String(instrument.to_owned()),
        DynamicValue::Int64(sample.leadership_term_id),
        DynamicValue::Int64(sample.cluster_session_id),
        DynamicValue::UInt64(sample.sequence),
        DynamicValue::UInt64(sample.exchange_ts_ns),
        DynamicValue::UInt64(sample.receive_ts_ns),
        DynamicValue::UInt64(sample.ingress_claim_ts_ns),
        DynamicValue::UInt64(sample.egress_decode_ts_ns),
        DynamicValue::UInt64(sample.book_apply_ts_ns),
        DynamicValue::UInt64(sample.ch_enqueue_ts_ns),
        DynamicValue::Int64(sample.exchange_to_receive_ns()),
        DynamicValue::Int64(sample.receive_to_claim_ns()),
        DynamicValue::Int64(sample.claim_to_egress_ns()),
        DynamicValue::Int64(sample.e2e_ns()),
    ];
    recorder.record(&values)
}

/// Encode the matching DynamicSchema announcement for `schema_id`.
pub fn encode_latency_schema(schema_id: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let max = DynamicSchemaEncoder::MAX_ENCODED_LENGTH;
    let mut buf = vec![0u8; max];
    let mut enc = DynamicSchemaEncoder::wrap_and_apply_header(&mut buf, 0)?;
    let _ = enc.schema_id(schema_id);

    let mut sym = Vec::new();
    for (name, _) in LATENCY_FIELDS {
        sym.extend_from_slice(name.as_bytes());
    }

    let enc = enc.metadata(0, |_g| {})?;
    let enc = enc.columns(LATENCY_FIELDS.len() as u16, |g| {
        for (i, (name, ty)) in LATENCY_FIELDS.iter().enumerate() {
            let tag = column_type_to_tag(ty).expect("supported column type");
            let _ = g.add(|e| {
                let _ = e
                    .field_id(i as u8)
                    .name_len(name.len() as u16)
                    .type_tag(tag);
            });
        }
    })?;
    let enc = enc.table_name(FEED_LATENCY_TABLE.as_bytes())?;
    let complete = enc.symbol_table(&sym)?;
    let len = complete.encoded_length_with_header();
    Ok(buf[..len].to_vec())
}

/// Persist DTO written to ClickHouse after DynamicRow decode.
#[derive(Debug, Clone, Persist, Serialize, PartialEq)]
pub struct FeedLatencyRow {
    pub instrument: String,
    pub leadership_term_id: i64,
    pub cluster_session_id: i64,
    pub sequence: u64,
    pub exchange_ts_ns: u64,
    pub receive_ts_ns: u64,
    pub ingress_claim_ts_ns: u64,
    pub egress_decode_ts_ns: u64,
    pub book_apply_ts_ns: u64,
    pub ch_enqueue_ts_ns: u64,
    pub exchange_to_receive_ns: i64,
    pub receive_to_claim_ns: i64,
    pub claim_to_egress_ns: i64,
    pub e2e_ns: i64,
}

/// Errors from the latency persist path.
#[derive(Debug)]
pub enum LatencyPersistError {
    Recorder(DynamicRecorderError),
    Registry(String),
    Decode(String),
    Sink(SinkError),
    Other(String),
}

impl std::fmt::Display for LatencyPersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recorder(e) => write!(f, "recorder: {e}"),
            Self::Registry(e) => write!(f, "registry: {e}"),
            Self::Decode(e) => write!(f, "decode: {e}"),
            Self::Sink(e) => write!(f, "sink: {e}"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LatencyPersistError {}

/// Shipped path: DynamicSchema once → DynamicRow encode → SchemaRegistry
/// decode → ClickhouseSink insert.
pub struct LatencyPersistor {
    recorder: DynamicRecorder,
    registry: Rc<RefCell<SchemaRegistry>>,
    row_decoder: RowDecoder,
    sender: PersistSender<FeedLatencyRow>,
    schema_announced: bool,
    pub rows_persisted: u64,
}

impl LatencyPersistor {
    /// Connect to ClickHouse (default URL / env, password `ergosbe` in tests).
    pub fn connect(url: &str, user: &str, password: &str) -> Result<Self, LatencyPersistError> {
        let sink = ClickhouseSinkBuilder::new()
            .url(url)
            .user(user)
            .password(password)
            .build()
            .map_err(LatencyPersistError::Sink)?;
        Self::with_sink(sink)
    }

    pub fn with_sink(sink: ClickhouseSink) -> Result<Self, LatencyPersistError> {
        let recorder = build_feed_latency_recorder().map_err(LatencyPersistError::Recorder)?;
        let registry = Rc::new(RefCell::new(SchemaRegistry::new()));
        let row_decoder = RowDecoder::new(Rc::clone(&registry));
        let sender = sink.sender(FEED_LATENCY_TABLE).build();
        Ok(Self {
            recorder,
            registry,
            row_decoder,
            sender,
            schema_announced: false,
            rows_persisted: 0,
        })
    }

    /// Announce DynamicSchema once and register it for row decode.
    pub fn ensure_schema(&mut self) -> Result<Vec<u8>, LatencyPersistError> {
        if self.schema_announced {
            return Ok(Vec::new());
        }
        let schema_bytes = encode_latency_schema(self.recorder.schema_id)
            .map_err(|e| LatencyPersistError::Other(e.to_string()))?;
        let schema = DynamicSchemaDecoder::wrap_and_apply_header(&schema_bytes, 0)
            .map_err(|e| LatencyPersistError::Decode(e.to_string()))?;
        self.registry
            .borrow_mut()
            .register(schema)
            .map_err(|e| LatencyPersistError::Registry(e.to_string()))?;
        self.schema_announced = true;
        Ok(schema_bytes)
    }

    /// Encode DynamicRow from `sample`, decode via SchemaRegistry, insert DTO.
    pub fn persist_sample(
        &mut self,
        instrument: &str,
        sample: LatencySample,
    ) -> Result<FeedLatencyRow, LatencyPersistError> {
        self.ensure_schema()?;
        let row_bytes = record_latency_row(&mut self.recorder, instrument, sample)
            .map_err(LatencyPersistError::Recorder)?
            .to_vec();
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0)
            .map_err(|e| LatencyPersistError::Decode(e.to_string()))?;
        let decoded = self
            .row_decoder
            .decode(row)
            .map_err(|e| LatencyPersistError::Decode(e.to_string()))?;

        // Build DTO exclusively from decoded DynamicRow column literals.
        let dto = FeedLatencyRow {
            instrument: require_str(&decoded, "instrument")?,
            leadership_term_id: require_i64(&decoded, "leadership_term_id")?,
            cluster_session_id: require_i64(&decoded, "cluster_session_id")?,
            sequence: require_u64(&decoded, "sequence")?,
            exchange_ts_ns: require_u64(&decoded, "exchange_ts_ns")?,
            receive_ts_ns: require_u64(&decoded, "receive_ts_ns")?,
            ingress_claim_ts_ns: require_u64(&decoded, "ingress_claim_ts_ns")?,
            egress_decode_ts_ns: require_u64(&decoded, "egress_decode_ts_ns")?,
            book_apply_ts_ns: require_u64(&decoded, "book_apply_ts_ns")?,
            ch_enqueue_ts_ns: require_u64(&decoded, "ch_enqueue_ts_ns")?,
            exchange_to_receive_ns: require_i64(&decoded, "exchange_to_receive_ns")?,
            receive_to_claim_ns: require_i64(&decoded, "receive_to_claim_ns")?,
            claim_to_egress_ns: require_i64(&decoded, "claim_to_egress_ns")?,
            e2e_ns: require_i64(&decoded, "e2e_ns")?,
        };
        self.sender
            .persist(&dto)
            .map_err(LatencyPersistError::Sink)?;
        self.rows_persisted += 1;
        Ok(dto)
    }

    pub fn flush(&self) -> Result<(), LatencyPersistError> {
        self.sender.flush();
        Ok(())
    }
}

fn require_str(
    row: &std::collections::HashMap<String, Option<String>>,
    col: &str,
) -> Result<String, LatencyPersistError> {
    let v = row
        .get(col)
        .and_then(|o| o.as_ref())
        .ok_or_else(|| LatencyPersistError::Decode(format!("missing {col}")))?;
    // SQL string literals may be quoted by the decoder.
    let s = v.trim().trim_matches('\'');
    Ok(s.to_owned())
}

fn require_i64(
    row: &std::collections::HashMap<String, Option<String>>,
    col: &str,
) -> Result<i64, LatencyPersistError> {
    let v = row
        .get(col)
        .and_then(|o| o.as_ref())
        .ok_or_else(|| LatencyPersistError::Decode(format!("missing {col}")))?;
    v.parse()
        .map_err(|e| LatencyPersistError::Decode(format!("{col}: {e}")))
}

fn require_u64(
    row: &std::collections::HashMap<String, Option<String>>,
    col: &str,
) -> Result<u64, LatencyPersistError> {
    let v = row
        .get(col)
        .and_then(|o| o.as_ref())
        .ok_or_else(|| LatencyPersistError::Decode(format!("missing {col}")))?;
    v.parse()
        .map_err(|e| LatencyPersistError::Decode(format!("{col}: {e}")))
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
        let mut rec = build_feed_latency_recorder()?;
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
        let bytes = record_latency_row(&mut rec, "BTCUSDT", s)?;
        assert!(bytes.len() > 8);
        let schema = encode_latency_schema(rec.schema_id)?;
        assert!(schema.len() > 8);
        Ok(())
    }

    #[test]
    fn schema_register_and_row_decode_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = build_feed_latency_recorder()?;
        let schema_bytes = encode_latency_schema(rec.schema_id)?;
        let reg = Rc::new(RefCell::new(SchemaRegistry::new()));
        {
            let schema = DynamicSchemaDecoder::wrap_and_apply_header(&schema_bytes, 0)?;
            reg.borrow_mut().register(schema)?;
        }
        let sample = LatencySample {
            leadership_term_id: 3,
            cluster_session_id: 9,
            sequence: 42,
            exchange_ts_ns: 1_000,
            receive_ts_ns: 1_100,
            ingress_claim_ts_ns: 1_200,
            egress_decode_ts_ns: 1_500,
            book_apply_ts_ns: 1_600,
            ch_enqueue_ts_ns: 1_700,
        };
        let row_bytes = record_latency_row(&mut rec, "BTCUSDT", sample)?.to_vec();
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0)?;
        let decoder = RowDecoder::new(Rc::clone(&reg));
        let decoded = decoder.decode(row)?;
        assert_eq!(
            decoded
                .get("exchange_to_receive_ns")
                .and_then(|o| o.as_deref()),
            Some("100")
        );
        assert_eq!(
            decoded.get("claim_to_egress_ns").and_then(|o| o.as_deref()),
            Some("300")
        );
        Ok(())
    }
}
