//! Live ClickHouse proof: DynamicSchema → DynamicRow → SchemaRegistry decode
//! → ClickhouseSink (shipped `LatencyPersistor` path). H4/H5.
//!
//! Requires ClickHouse at 127.0.0.1:8123 (password `ergo-sbe`).

use cluster_ha_orderbook::latency::{FEED_LATENCY_TABLE, LatencyPersistor, LatencySample};

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
#[ignore = "requires live ClickHouse — run via just samples-cluster-ha"]
fn feed_latency_via_latency_persistor_into_clickhouse() -> Result<(), Box<dyn std::error::Error>> {
    if !ch_reachable() {
        return Err("ClickHouse not reachable at 127.0.0.1:8123".into());
    }
    // Clean slate so SELECT is exact.
    let _ = ch_query(&format!("DROP TABLE IF EXISTS {FEED_LATENCY_TABLE}"));

    let mut persistor = LatencyPersistor::connect(ENDPOINT, "default", "ergosbe")?;
    // DynamicSchema announcement + registry registration (must run before rows).
    let schema_bytes = persistor.ensure_schema()?;
    assert!(
        !schema_bytes.is_empty(),
        "first ensure_schema must return DynamicSchema SBE bytes"
    );

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

    // Shipped path: DynamicRow encode → decode → ClickhouseSink.persist
    let dto = persistor.persist_sample("BTCUSDT", sample)?;
    persistor.flush()?;
    // Small pause so background sink flushes.
    std::thread::sleep(std::time::Duration::from_millis(300));
    persistor.flush()?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    // DTO must reflect decoded DynamicRow (deltas from the wire path).
    assert_eq!(dto.sequence, 42);
    assert_eq!(dto.exchange_to_receive_ns, 100);
    assert_eq!(dto.receive_to_claim_ns, 100);
    assert_eq!(dto.claim_to_egress_ns, 300);
    assert_eq!(dto.e2e_ns, 600);
    assert_eq!(persistor.rows_persisted, 1);

    // Query CH — values must match what DynamicRow decode produced.
    let out = ch_query(&format!(
        "SELECT exchange_to_receive_ns, receive_to_claim_ns, claim_to_egress_ns, e2e_ns, instrument
         FROM {FEED_LATENCY_TABLE} WHERE sequence = 42 FORMAT TSV"
    ))?;
    let parts: Vec<&str> = out.trim().split('\t').collect();
    assert!(
        parts.len() >= 4,
        "expected CH row for sequence=42, got {out:?}"
    );
    assert_eq!(parts[0].parse::<i64>()?, 100);
    assert_eq!(parts[1].parse::<i64>()?, 100);
    assert_eq!(parts[2].parse::<i64>()?, 300);
    assert_eq!(parts[3].parse::<i64>()?, 600);
    if parts.len() >= 5 {
        assert!(parts[4].contains("BTCUSDT") || parts[4] == "BTCUSDT");
    }
    Ok(())
}
