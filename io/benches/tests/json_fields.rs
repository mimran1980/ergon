//! Field extraction from real Binance message shapes.

use io_bench::json::{self, Kind};

const SPOT_TRADE: &[u8] = br#"{"stream":"btcusdt@trade","data":{"e":"trade","E":1760345000123456,"s":"BTCUSDT","t":5200000001,"p":"62000.01000000","q":"0.00100000","T":1760345000123400,"m":true,"M":true}}"#;
const FUT_BOOK: &[u8] = br#"{"stream":"btcusdt@bookTicker","data":{"e":"bookTicker","u":8822334455,"s":"BTCUSDT","b":"62000.10","B":"1.234","a":"62000.20","A":"0.500","T":1760345000120,"E":1760345000121}}"#;
const FUT_AGG: &[u8] = br#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade","E":1760345000125,"a":2700000001,"s":"BTCUSDT","p":"62000.10","q":"0.010","f":5000000001,"l":5000000002,"T":1760345000124,"m":false}}"#;

#[test]
fn spot_trade_in_microseconds() {
    let (name, data) = json::combined(SPOT_TRADE).unwrap();
    assert_eq!(name, b"btcusdt@trade");
    let f = json::fields(data, Kind::of("btcusdt@trade").unwrap()).unwrap();
    assert_eq!(f.id, 5_200_000_001);
    assert_eq!(f.event_time, 1_760_345_000_123_456);
    assert_eq!(
        json::binance_ts_to_ns(f.event_time),
        1_760_345_000_123_456_000
    );
    assert_eq!(f.trade_time, 1_760_345_000_123_400);
}

#[test]
fn futures_book_ticker_uses_update_id_not_ask_price() {
    let (_, data) = json::combined(FUT_BOOK).unwrap();
    let f = json::fields(data, Kind::BookTicker).unwrap();
    assert_eq!(f.id, 8_822_334_455);
    assert_eq!(
        json::binance_ts_to_ns(f.event_time),
        1_760_345_000_121_000_000
    );
}

#[test]
fn futures_agg_trade() {
    let (_, data) = json::combined(FUT_AGG).unwrap();
    let f = json::fields(data, Kind::AggTrade).unwrap();
    assert_eq!(f.id, 2_700_000_001);
    assert_eq!(f.trade_time, 1_760_345_000_124);
}

#[test]
fn quoted_values_are_not_integers() {
    // "a" in bookTicker is a quoted price; it must not be read as an id.
    let (_, data) = json::combined(FUT_BOOK).unwrap();
    assert_eq!(json::find_int(data, b"a"), None);
    assert_eq!(json::find_int(b"{}", b"E"), None);
    assert_eq!(json::combined(b"{\"result\":null,\"id\":1}"), None);
}

#[test]
fn stream_kinds() {
    assert_eq!(Kind::of("ethusdt@trade"), Some(Kind::Trade));
    assert_eq!(Kind::of("btcusdt@depth@100ms"), Some(Kind::Depth));
    assert_eq!(Kind::of("btcusdt@kline_1m"), None);
}
