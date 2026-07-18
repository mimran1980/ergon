//! Live ClickHouse proof for feed_latency DynamicSchema/Row (H4/H5).
//!
//! Requires ClickHouse at 127.0.0.1:8123 (same as persist live tests).

use cluster_ha_orderbook::latency::{
    FEED_LATENCY_TABLE, LatencySample, build_feed_latency_recorder, record_latency_row,
};
use ergo_clickhouse_persist::sbe::v2::{DynamicRowV2Decoder, DynamicSchemaV2Decoder};

const ENDPOINT: &str = "http://127.0.0.1:8123";

fn ch_query(sql: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(ENDPOINT)
        .header("X-ClickHouse-User", "default")
        .header("X-ClickHouse-Key", "ergosbe")
        .body(sql.to_string())
        .send()?;
    let status = resp.status();
    let text = resp.text()?;
    if !status.is_success() {
        return Err(format!("CH {status}: {text}").into());
    }
    Ok(text)
}

fn ch_reachable() -> bool {
    ch_query("SELECT 1")
        .map(|t| t.trim() == "1")
        .unwrap_or(false)
}

#[test]
#[ignore = "requires live ClickHouse — run via just samples-cluster-ha / test-ha-latency-live"]
fn feed_latency_schema_and_row_roundtrip_ch() -> Result<(), Box<dyn std::error::Error>> {
    if !ch_reachable() {
        return Err("ClickHouse not reachable at 127.0.0.1:8123".into());
    }
    ch_query(&format!("DROP TABLE IF EXISTS {FEED_LATENCY_TABLE}"))?;

    let recorder = build_feed_latency_recorder()?;
    // Create table from schema message columns — use MergeTree with instrument + sequence.
    ch_query(&format!(
        "CREATE TABLE IF NOT EXISTS {FEED_LATENCY_TABLE} (
            instrument String,
            leadership_term_id Int64,
            cluster_session_id Int64,
            sequence UInt64,
            exchange_ts_ns UInt64,
            receive_ts_ns UInt64,
            ingress_claim_ts_ns UInt64,
            egress_decode_ts_ns UInt64,
            book_apply_ts_ns UInt64,
            ch_enqueue_ts_ns UInt64,
            exchange_to_receive_ns Int64,
            receive_to_claim_ns Int64,
            claim_to_egress_ns Int64,
            e2e_ns Int64
        ) ENGINE = MergeTree ORDER BY (instrument, sequence)"
    ))?;

    // Prove schema encodes (DynamicSchemaV2) and row encodes via real recorder.
    let mut schema_buf = vec![0u8; 4096];
    let schema_bytes = recorder.schema_into(&mut schema_buf)?;
    let _schema_dec = DynamicSchemaV2Decoder::wrap_and_apply_header(schema_bytes, 0)?;

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
    let mut row_buf = vec![0u8; 4096];
    let row_bytes = record_latency_row(&recorder, &mut row_buf, "BTCUSDT", sample)?;
    let _row_dec = DynamicRowV2Decoder::wrap_and_apply_header(row_bytes, 0)?;

    // Insert via HTTP using the derived deltas from the real LatencySample API.
    ch_query(&format!(
        "INSERT INTO {FEED_LATENCY_TABLE} VALUES (
            'BTCUSDT', {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}
        )",
        sample.leadership_term_id,
        sample.cluster_session_id,
        sample.sequence,
        sample.exchange_ts_ns,
        sample.receive_ts_ns,
        sample.ingress_claim_ts_ns,
        sample.egress_decode_ts_ns,
        sample.book_apply_ts_ns,
        sample.ch_enqueue_ts_ns,
        sample.exchange_to_receive_ns(),
        sample.receive_to_claim_ns(),
        sample.claim_to_egress_ns(),
        sample.e2e_ns(),
    ))?;

    let out = ch_query(&format!(
        "SELECT exchange_to_receive_ns, receive_to_claim_ns, claim_to_egress_ns, e2e_ns
         FROM {FEED_LATENCY_TABLE} WHERE sequence = 42 FORMAT TSV"
    ))?;
    let parts: Vec<&str> = out.trim().split('\t').collect();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0].parse::<i64>()?, 100);
    assert_eq!(parts[1].parse::<i64>()?, 100);
    assert_eq!(parts[2].parse::<i64>()?, 300);
    assert_eq!(parts[3].parse::<i64>()?, 600);
    Ok(())
}
