//! Foreground ClickHouse persistence: insert L2Book via clickhouse-rs client,
//! query back, and verify data integrity — including Decimal arrays.
#![allow(unused)]

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

/// Insert a row with Decimal arrays into ClickHouse and query it back.
#[test]
fn foreground_persist_l2book_with_decimal_arrays() {
    use normalized_app::{Decimal, Source};

    // ── Build row data with values that fit in Decimal(38,18) ────────
    // mantissa * 10^exponent: (500, -2) = 5.00, fits easily
    let bid_prices: Vec<(i64, i8)> = vec![(500, -2), (499, -2)];
    let bid_sizes: Vec<(i64, i8)> = vec![(150, -2), (200, -2)];
    let ask_prices: Vec<(i64, i8)> = vec![(501, -2)];
    let ask_sizes: Vec<(i64, i8)> = vec![(50, -2)];

    // Convert to Decimal(38,18) scaled integers
    fn scale_to_18(mantissa: i64, exponent: i8) -> i128 {
        let scale_diff = exponent as i32 + 18;
        if scale_diff >= 0 {
            (mantissa as i128) * 10i128.pow(scale_diff as u32)
        } else {
            (mantissa as i128) / 10i128.pow((-scale_diff) as u32)
        }
    }

    let bid_px_scaled: Vec<i128> = bid_prices.iter().map(|&(m, e)| scale_to_18(m, e)).collect();
    let bid_sz_scaled: Vec<i128> = bid_sizes.iter().map(|&(m, e)| scale_to_18(m, e)).collect();
    let ask_px_scaled: Vec<i128> = ask_prices.iter().map(|&(m, e)| scale_to_18(m, e)).collect();
    let ask_sz_scaled: Vec<i128> = ask_sizes.iter().map(|&(m, e)| scale_to_18(m, e)).collect();

    // ── Build INSERT query ──────────────────────────────────────────
    fn fmt_array(vals: &[i128]) -> String {
        let inner: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
        format!("[{}]", inner.join(", "))
    }

    let insert_sql = format!(
        "INSERT INTO l2book_typed \
         (source, symbol, exchange_timestamp, receive_timestamp, sequence, \
          bid_prices, bid_sizes, ask_prices, ask_sizes) \
         VALUES ('Bitget', 'BTCUSDT', 1700000000000000001, 1700000000000000002, 42, \
          {}, {}, {}, {})",
        fmt_array(&bid_px_scaled),
        fmt_array(&bid_sz_scaled),
        fmt_array(&ask_px_scaled),
        fmt_array(&ask_sz_scaled),
    );

    // Execute INSERT via HTTP
    let client = reqwest::blocking::Client::new();
    let insert_resp = client
        .post("http://127.0.0.1:8123/")
        .basic_auth("default", Some("ergosbe"))
        .body(insert_sql.clone())
        .send()
        .expect("INSERT request");
    assert!(insert_resp.status().is_success(),
        "INSERT failed: {} body: {}",
        insert_resp.status(),
        insert_resp.text().unwrap_or_default());

    // ── Query back ──────────────────────────────────────────────────
    let query = "SELECT source, symbol, sequence, bid_prices, bid_sizes, ask_prices, ask_sizes \
                 FROM l2book_typed WHERE sequence = 42 ORDER BY ingest_ts DESC LIMIT 1";
    let query_resp = client
        .post("http://127.0.0.1:8123/")
        .basic_auth("default", Some("ergosbe"))
        .body(query)
        .send()
        .expect("SELECT request");
    assert!(query_resp.status().is_success(), "SELECT failed");
    let body = query_resp.text().expect("response body");

    // Verify row data returned
    assert!(body.contains("BTCUSDT"), "symbol missing from result: {body}");
    assert!(body.contains("42"), "sequence missing from result: {body}");
    assert!(body.contains("Bitget"), "source missing from result: {body}");

    // Verify Decimal array values — ClickHouse returns them as [v1, v2, ...]
    for v in &bid_px_scaled {
        assert!(body.contains(&v.to_string()),
            "bid price {v} missing from result: {body}");
    }
}
