//! ForegroundPersistor tests — the Task 7/10 persistence seam.
//!
//! Bytes are produced by the real `ClaimPublisher` (recording adapter), so
//! these tests prove the publisher and persistor agree on the wire format.

use advanced_bitget::market::{Level, NormalizedEventRef, WireDec};
use advanced_bitget::persistence::{ForegroundPersistor, InMemorySink};
use advanced_bitget::publication::{ClaimPublisher, RecordingPublication};

fn lvl(pm: i64, pe: i8, sm: i64, se: i8) -> Level {
    Level {
        price: WireDec::new(pm, pe),
        size: WireDec::new(sm, se),
    }
}

/// Publish one L2 book through the recording publisher, returning the
/// committed (typed, dynamic) claim bytes.
fn published_book(sequence: u64, bids: &[Level], asks: &[Level]) -> (Vec<u8>, Vec<u8>, u32) {
    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let ev = NormalizedEventRef::L2Book {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_000,
        receive_ts_ns: 1_700_000_000_000_000_100,
        sequence,
        bids,
        asks,
    };
    p.publish(&ev);
    let schema_id = p.dynamic_schema_id();
    let (typed, dynamic) = p.into_adapters();
    (
        typed.committed[0].clone(),
        dynamic.committed[0].clone(),
        schema_id,
    )
}

#[test]
fn matched_books_persist_to_both_tables() {
    let bids = [lvl(500005, -1, 15, -1)];
    let asks = [lvl(500015, -1, 30, -1)];
    let (typed, dynamic, _schema) = published_book(7, &bids, &asks);

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.on_typed(&typed).unwrap();
    assert!(
        p.sink().l2book_typed.is_empty(),
        "waits for the dynamic match"
    );
    p.on_dynamic(&dynamic).unwrap();

    assert_eq!(p.sink().l2book_typed.len(), 1);
    assert_eq!(p.sink().l2book_dynamic.len(), 1);
    let row = &p.sink().l2book_typed[0];
    assert_eq!(row.sequence, 7);
    assert_eq!(row.symbol, "BTCUSDT");
    // Exact ClickHouse Decimal(38,18) scaled integers.
    assert_eq!(row.bid_prices, vec![50_000_500_000_000_000_000_000_i128]);
    assert_eq!(row.bid_sizes, vec![1_500_000_000_000_000_000_i128]);
    assert_eq!(row.ask_prices, vec![50_001_500_000_000_000_000_000_i128]);
    assert_eq!(p.counters().persisted_typed, 1);
    assert_eq!(p.counters().persisted_dynamic, 1);
}

#[test]
fn mismatched_book_values_are_counted_not_persisted() {
    let bids_a = [lvl(500005, -1, 15, -1)];
    let bids_b = [lvl(999999, -1, 15, -1)];
    let (typed, _, _) = published_book(7, &bids_a, &[]);
    let (_, dynamic, _) = published_book(7, &bids_b, &[]);

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.on_typed(&typed).unwrap();
    p.on_dynamic(&dynamic).unwrap();

    assert!(p.sink().l2book_typed.is_empty());
    assert!(p.sink().l2book_dynamic.is_empty());
    assert_eq!(p.counters().compare_failures, 1);
}

#[test]
fn smaller_unmatched_sequence_is_dropped_and_counted() {
    let bids = [lvl(1, 0, 1, 0)];
    let (typed_1, _, _) = published_book(1, &bids, &[]);
    let (typed_3, dynamic_3, _) = published_book(3, &bids, &[]);
    // Dynamic for sequence 1 never arrives.
    let (_, dynamic_missing, _) = published_book(3, &bids, &[]);
    let _ = dynamic_missing;

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.on_typed(&typed_1).unwrap();
    p.on_typed(&typed_3).unwrap();
    p.on_dynamic(&dynamic_3).unwrap();

    // Sequence 3 matched; the smaller unmatched typed 1 was dropped.
    assert_eq!(p.sink().l2book_typed.len(), 1);
    assert_eq!(p.sink().l2book_typed[0].sequence, 3);
    assert_eq!(p.counters().unmatched_dropped, 1);
}

#[test]
fn trade_persists_directly() {
    let mut pubr =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let ev = NormalizedEventRef::Trade {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_000,
        receive_ts_ns: 1_700_000_000_000_000_100,
        sequence: 9,
        price: WireDec::new(500005, -1),
        size: WireDec::new(25, -2),
        is_buy: true,
    };
    pubr.publish(&ev);
    let (typed, _) = pubr.into_adapters();

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.on_typed(&typed.committed[0]).unwrap();
    assert_eq!(p.sink().trade.len(), 1);
    let row = &p.sink().trade[0];
    assert_eq!(row.trade_id, 9);
    assert_eq!(row.price, 50_000_500_000_000_000_000_000_i128);
    assert_eq!(row.size, 250_000_000_000_000_000_i128);
    assert!(row.is_buy);
    assert_eq!(p.counters().persisted_trades, 1);
}

#[test]
fn recursive_app_message_payload_is_rejected() {
    use advanced_bitget::normalized_app::AppMessageEncoder;

    // AppMessage whose payload is itself an AppMessage.
    let inner_len = AppMessageEncoder::compute_encoded_length_with_message_header(b"x".len(), 0);
    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(b"x".len(), inner_len);
    let mut buf = vec![0u8; outer_len];
    {
        let mut outer = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = outer.sent_ts(1);
        let after = outer.app_name(b"x").unwrap();
        let _ = after
            .payload_with(
                inner_len,
                |payload| -> Result<(), advanced_bitget::normalized_app::sbe_rt::EncodeError> {
                    let mut inner = AppMessageEncoder::wrap_and_apply_header(payload, 0)?;
                    let _ = inner.sent_ts(2);
                    let after = inner.app_name(b"x")?;
                    let _ = after.payload(&[])?;
                    Ok(())
                },
            )
            .unwrap();
    }

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_typed(&buf).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("recursive") || msg.contains("payload"),
        "unexpected error: {msg}"
    );
    assert_eq!(p.counters().decode_failures, 1);
}

#[test]
fn malformed_bytes_are_decode_failures() {
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    assert!(p.on_typed(&[0u8; 4]).is_err());
    assert!(p.on_dynamic(&[0u8; 4]).is_err());
    assert_eq!(p.counters().decode_failures, 2);
}
