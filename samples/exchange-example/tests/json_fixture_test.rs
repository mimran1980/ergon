//! Real Binance spot responses, recorded once from public market-data
//! endpoints (no API key): each `tests/fixtures/spot-json/<name>.sbe` body, rendered
//! by the hook-generated `write_json` methods, must equal the venue's own JSON
//! answer to the same request (`<name>.json`, fetched with
//! `X-MBX-TIME-UNIT: MICROSECOND`) as a JSON value: same keys, same arrays,
//! same strings and exact numbers. Re-record with
//! `scripts/live-json-check.sh --save tests/fixtures/spot-json`.

use exchange_example::sbe_json::{JsonError, render};

const FIXTURES: &[&str] = &[
    "depth",
    "trades",
    "agg_trades",
    "klines",
    "avg_price",
    "ticker_price",
    "ticker_prices",
    "book_ticker",
    "book_tickers",
];

fn fixture(name: &str, ext: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(format!(
        "{}/tests/fixtures/spot-json/{name}.{ext}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn rendered_sbe_equals_the_venue_json() -> Result<(), Box<dyn std::error::Error>> {
    for name in FIXTURES {
        let rendered = render(&fixture(name, "sbe")?)?;
        let ours: serde_json::Value = serde_json::from_str(&rendered)?;
        let theirs: serde_json::Value = serde_json::from_slice(&fixture(name, "json")?)?;
        assert_eq!(ours, theirs, "{name}: rendered {rendered}");
    }
    Ok(())
}

/// Prices and quantities are strings with every digit the exponent implies,
/// never floats: compare one value textually too.
#[test]
fn decimals_keep_their_digits() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = render(&fixture("depth", "sbe")?)?;
    let depth: serde_json::Value = serde_json::from_str(&rendered)?;
    let price = depth["bids"][0][0]
        .as_str()
        .ok_or("bid price is a string")?;
    let (_, frac) = price.split_once('.').ok_or("bid price has a point")?;
    assert_eq!(frac.len(), 8, "{price}");
    Ok(())
}

#[test]
fn unknown_templates_and_short_frames_are_errors() {
    assert!(matches!(render(&[0u8; 3]), Err(JsonError::Short)));
    // ExchangeInfoResponse (103) has no JSON rule in this sample.
    let mut frame = vec![0u8; 8];
    frame[2..4].copy_from_slice(&103u16.to_le_bytes());
    assert!(matches!(render(&frame), Err(JsonError::Template(103))));
}
