//! ClaimPublisher tests — the Task 7/9 publication seam.
//!
//! All tests drive publication through `ClaimPublisher::publish` with the
//! `RecordingPublication` adapter, then decode the captured claim bytes with
//! the generated consuming decoders.

use advanced_bitget::market::{Level, NormalizedEventRef, WireDec};
use advanced_bitget::normalized_app::{AnyMessage, AppMessageDecoder, Source};
use advanced_bitget::publication::{
    ClaimPublisher, DropReason, PublishOutcome, RecordingPublication,
};
use ergo_clickhouse_persist::sbe::v2::DynamicRowV2Decoder;

fn lvl(pm: i64, pe: i8, sm: i64, se: i8) -> Level {
    Level {
        price: WireDec::new(pm, pe),
        size: WireDec::new(sm, se),
    }
}

fn book_event<'a>(bids: &'a [Level], asks: &'a [Level]) -> NormalizedEventRef<'a> {
    NormalizedEventRef::L2Book {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_000,
        receive_ts_ns: 1_700_000_000_000_000_100,
        sequence: 7,
        bids,
        asks,
    }
}

#[test]
fn publish_l2book_encodes_app_message_on_typed_stream() -> Result<(), Box<dyn std::error::Error>> {
    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let bids = [lvl(500005, -1, 15, -1), lvl(500000, -1, 20, -1)];
    let asks = [lvl(500015, -1, 30, -1)];

    let outcome = p.publish(&book_event(&bids, &asks));
    assert_eq!(outcome, PublishOutcome::Published);
    assert_eq!(p.counters().published, 2, "typed + dynamic claims");

    let (typed, _dynamic) = p.into_adapters();
    assert_eq!(typed.committed.len(), 1);
    let bytes = &typed.committed[0];

    // Claim length must exactly equal the encoded message length.
    let app = AppMessageDecoder::wrap_and_apply_header(bytes, 0).unwrap();
    let (name, after) = app.into_app_name().unwrap();
    assert_eq!(name, b"ergosbe");
    let (frame, _complete) = after.into_payload_as_message().unwrap();
    let AnyMessage::L2Book(book) = frame.message else {
        panic!("payload must dispatch as L2Book");
    };
    assert_eq!(book.source(), Source::Bitget);
    assert_eq!(book.sequence(), 7);
    assert_eq!(book.exchange_timestamp(), 1_700_000_000_000_000_000);

    let mut got_bids = Vec::new();
    let mut g = book.into_bids().unwrap();
    for e in g.by_ref() {
        got_bids.push((
            (e.price_wire().mantissa(), e.price_wire().exponent()),
            (e.size_wire().mantissa(), e.size_wire().exponent()),
        ));
    }
    let after = g.finish().unwrap();
    assert_eq!(
        got_bids,
        vec![((500005, -1), (15, -1)), ((500000, -1), (20, -1))]
    );

    let mut got_asks = Vec::new();
    let mut g = after.into_asks().unwrap();
    for e in g.by_ref() {
        got_asks.push((e.price_wire().mantissa(), e.price_wire().exponent()));
    }
    let after = g.finish().unwrap();
    assert_eq!(got_asks, vec![(500015, -1)]);

    let (symbol, _) = after.into_symbol().unwrap();
    assert_eq!(symbol, b"BTCUSDT");

    Ok(())
}

#[test]
fn publish_l2book_publishes_dynamic_v2_row_with_same_correlation() -> Result<(), Box<dyn std::error::Error>> {
    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let bids = [lvl(500005, -1, 15, -1)];
    let asks = [lvl(500015, -1, 30, -1)];
    p.publish(&book_event(&bids, &asks));

    let schema_id = p.dynamic_schema_id();
    let (_typed, dynamic) = p.into_adapters();
    assert_eq!(dynamic.committed.len(), 1);
    let bytes = &dynamic.committed[0];

    let dec = DynamicRowV2Decoder::wrap_and_apply_header(bytes, 0).unwrap();
    assert_eq!(dec.schema_id(), schema_id);
    let dec = dec.into_row_metadata().unwrap().finish().unwrap();
    let dec = dec.into_int64_fields().unwrap().finish().unwrap();

    let mut u64s = Vec::new();
    let mut g = dec.into_uint64_fields().unwrap();
    for e in g.by_ref() {
        u64s.push(e.value());
    }
    let dec = g.finish().unwrap();
    // sequence + exchange_ts
    assert!(u64s.contains(&7), "sequence (correlation) must be present");

    let dec = dec.into_float64_fields().unwrap().finish().unwrap();
    let dec = dec.into_bool_fields().unwrap().finish().unwrap();
    let dec = dec.into_string_fields().unwrap().finish().unwrap();
    let dec = dec.into_null_fields().unwrap().finish().unwrap();

    let mut arrays: Vec<Vec<(i64, i8)>> = Vec::new();
    let mut g = dec.into_decimal_array_fields().unwrap();
    for e in g.by_ref() {
        let e = e.unwrap();
        arrays.push(
            e.values()
                .unwrap()
                .map(|v| (v.mantissa(), v.exponent()))
                .collect(),
        );
    }
    assert_eq!(
        arrays,
        vec![
            vec![(500005, -1)], // bid prices
            vec![(15, -1)],     // bid sizes
            vec![(500015, -1)], // ask prices
            vec![(30, -1)],     // ask sizes
        ]
    );

    Ok(())
}

#[test]
fn publish_trade_encodes_app_message_trade() -> Result<(), Box<dyn std::error::Error>> {
    let mut p =
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
    assert_eq!(p.publish(&ev), PublishOutcome::Published);

    let (typed, dynamic) = p.into_adapters();
    assert_eq!(typed.committed.len(), 1);
    assert!(dynamic.committed.is_empty(), "trades publish on 1001 only");

    let app = AppMessageDecoder::wrap_and_apply_header(&typed.committed[0], 0).unwrap();
    let (_, after) = app.into_app_name().unwrap();
    let (frame, _) = after.into_payload_as_message().unwrap();
    let AnyMessage::Trade(trade) = frame.message else {
        panic!("payload must dispatch as Trade");
    };
    assert_eq!(trade.trade_id(), 9);
    assert_eq!(trade.price_wire().mantissa(), 500005);
    assert_eq!(trade.size_wire().exponent(), -2);

    Ok(())
}

#[test]
fn backpressure_drops_once_without_retry_and_counts() -> Result<(), Box<dyn std::error::Error>> {
    let typed = RecordingPublication::failing(DropReason::Backpressured);
    let mut p = ClaimPublisher::new(typed, RecordingPublication::new()).unwrap();
    let bids = [lvl(1, 0, 1, 0)];
    let outcome = p.publish(&book_event(&bids, &[]));
    assert_eq!(outcome, PublishOutcome::Dropped(DropReason::Backpressured));
    assert_eq!(p.counters().dropped_backpressure, 1);

    let (typed, _) = p.into_adapters();
    assert_eq!(
        typed.claim_attempts, 1,
        "exactly one claim attempt, no retry"
    );
    assert!(typed.committed.is_empty());

    Ok(())
}

#[test]
fn claim_length_is_exact() -> Result<(), Box<dyn std::error::Error>> {
    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let bids = [lvl(500005, -1, 15, -1)];
    p.publish(&book_event(&bids, &[]));
    let (typed, dynamic) = p.into_adapters();
    // The recording adapter records the claimed length; committed bytes must
    // fill the entire claim.
    assert_eq!(typed.committed[0].len(), typed.claimed_lengths[0]);
    assert_eq!(dynamic.committed[0].len(), dynamic.claimed_lengths[0]);

    Ok(())
}

#[test]
fn publish_schema_emits_decodable_dynamic_schema_v2() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::sbe::v2::DynamicSchemaV2Decoder;

    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let outcome = p.publish_schema();
    assert_eq!(outcome, PublishOutcome::Published);
    let schema_id = p.dynamic_schema_id();

    let (typed, dynamic) = p.into_adapters();
    assert!(
        typed.committed.is_empty(),
        "schema goes to stream 1002 only"
    );
    assert_eq!(dynamic.committed.len(), 1);
    assert_eq!(dynamic.committed[0].len(), dynamic.claimed_lengths[0]);

    let dec = DynamicSchemaV2Decoder::wrap_and_apply_header(&dynamic.committed[0], 0).unwrap();
    assert_eq!(dec.schema_id(), schema_id);
    let dec = dec.into_metadata().unwrap().finish().unwrap();
    let g = dec.into_columns().unwrap();
    assert_eq!(
        g.len(),
        7,
        "sequence, exchange_ts, symbol, 4 decimal arrays"
    );
    let dec = g.finish().unwrap();
    let (table, _) = dec.into_table_name().unwrap();
    assert_eq!(table, b"l2book_dynamic");

    Ok(())
}

#[test]
fn short_claims_classify_encode_failures_for_all_message_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let bids = [lvl(1, 0, 1, 0)];

    // Typed book claim comes up short → EncodeFailed before the dynamic leg.
    let mut p =
        ClaimPublisher::new(RecordingPublication::short(4), RecordingPublication::new()).unwrap();
    assert_eq!(
        p.publish(&book_event(&bids, &[])),
        PublishOutcome::EncodeFailed
    );
    assert_eq!(p.counters().encode_failures, 1);

    // Dynamic claim short → typed leg publishes, dynamic leg fails.
    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::short(4)).unwrap();
    assert_eq!(
        p.publish(&book_event(&bids, &[])),
        PublishOutcome::EncodeFailed
    );
    assert_eq!(p.counters().published, 1, "typed leg still published");
    assert_eq!(p.counters().encode_failures, 1);

    // Trade claim short.
    let mut p =
        ClaimPublisher::new(RecordingPublication::short(4), RecordingPublication::new()).unwrap();
    let trade = NormalizedEventRef::Trade {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1,
        receive_ts_ns: 2,
        sequence: 3,
        price: WireDec::new(1, 0),
        size: WireDec::new(1, 0),
        is_buy: false,
    };
    assert_eq!(p.publish(&trade), PublishOutcome::EncodeFailed);

    // Short dynamic claim also fails the schema announcement.
    let mut p =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::short(4)).unwrap();
    assert_eq!(p.publish_schema(), PublishOutcome::EncodeFailed);

    Ok(())
}

#[test]
fn every_drop_reason_maps_to_its_counter() -> Result<(), Box<dyn std::error::Error>> {
    let bids = [lvl(1, 0, 1, 0)];
    let cases = [
        DropReason::Backpressured,
        DropReason::NotConnected,
        DropReason::AdminAction,
        DropReason::Closed,
        DropReason::MaxPosition,
    ];
    for reason in cases {
        let mut p = ClaimPublisher::new(
            RecordingPublication::failing(reason),
            RecordingPublication::new(),
        )
        .unwrap();
        assert_eq!(
            p.publish(&book_event(&bids, &[])),
            PublishOutcome::Dropped(reason)
        );
        let c = p.counters();
        let hit = c.dropped_backpressure
            + c.dropped_not_connected
            + c.dropped_admin_action
            + c.dropped_closed
            + c.dropped_max_position;
        assert_eq!(hit, 1, "exactly one drop counter for {reason:?}");
    }

    Ok(())
}

#[test]
fn derived_ipc_mtu_covers_the_largest_maintained_message() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::publication::{derive_ipc_mtu, worst_case_typed_claim_len};
    let worst = worst_case_typed_claim_len();
    let mtu = derive_ipc_mtu();
    assert!(mtu >= worst + 32, "claim + data header must fit one MTU");
    assert!(mtu.is_power_of_two());
    assert!(mtu >= 1408);

    Ok(())
}
