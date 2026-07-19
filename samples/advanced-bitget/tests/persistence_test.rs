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
fn matched_books_persist_to_both_tables() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

#[test]
fn mismatched_book_values_are_counted_not_persisted() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

#[test]
fn smaller_unmatched_sequence_is_dropped_and_counted() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

#[test]
fn trade_persists_directly() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

#[test]
fn recursive_app_message_payload_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

#[test]
fn malformed_bytes_are_decode_failures() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    assert!(p.on_typed(&[0u8; 4]).is_err());
    assert!(p.on_dynamic(&[0u8; 4]).is_err());
    assert_eq!(p.counters().decode_failures, 2);

    Ok(())
}

#[test]
fn schema_message_on_dynamic_stream_is_recognised_not_a_row()
-> Result<(), Box<dyn std::error::Error>> {
    let mut pubr =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    pubr.publish_schema();
    let (_, dynamic) = pubr.into_adapters();

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.on_dynamic(&dynamic.committed[0]).unwrap();
    assert_eq!(p.counters().schemas_seen, 1);
    assert!(p.sink().l2book_dynamic.is_empty());
    assert_eq!(p.counters().decode_failures, 0);

    Ok(())
}

#[test]
fn wrong_template_on_dynamic_stream_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // An AppMessage (typed schema/template) arriving on the dynamic stream.
    let bids = [lvl(1, 0, 1, 0)];
    let (typed, _, _) = published_book(1, &bids, &[]);

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    assert!(p.on_dynamic(&typed).is_err());
    assert_eq!(p.counters().decode_failures, 1);

    Ok(())
}

#[test]
fn persist_error_display_strings() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::decimal::DecimalConvertError;
    use advanced_bitget::persistence::PersistError;

    assert!(
        PersistError::Decode("x".into())
            .to_string()
            .contains("decode failure")
    );
    assert!(
        PersistError::RecursivePayload
            .to_string()
            .contains("recursive")
    );
    assert!(
        PersistError::Convert(DecimalConvertError::Overflow)
            .to_string()
            .contains("decimal conversion")
    );
    assert!(
        PersistError::Sink("db".into())
            .to_string()
            .contains("sink failure")
    );

    Ok(())
}

#[test]
fn dec38_18_literals_are_exact() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::persistence::dec38_18;
    assert_eq!(dec38_18(0), "0");
    assert_eq!(dec38_18(1_500_000_000_000_000_000), "1.5");
    assert_eq!(dec38_18(-1_500_000_000_000_000_000), "-1.5");
    assert_eq!(dec38_18(1), "0.000000000000000001");
    assert_eq!(dec38_18(50_000_500_000_000_000_000_000), "50000.5");

    Ok(())
}

/// Sink whose writes fail — proves error mapping and surfacing.
#[derive(Default)]
struct FailingSink;

impl advanced_bitget::persistence::RowSink for FailingSink {
    fn insert_l2book_typed(
        &mut self,
        _row: &advanced_bitget::persistence::L2BookRow,
    ) -> Result<(), String> {
        Err("typed down".into())
    }
    fn insert_l2book_dynamic(
        &mut self,
        _row: &advanced_bitget::persistence::L2BookRow,
    ) -> Result<(), String> {
        Err("dynamic down".into())
    }
    fn insert_trade(
        &mut self,
        _row: &advanced_bitget::persistence::TradeRow,
    ) -> Result<(), String> {
        Err("trade down".into())
    }
    fn flush(&mut self) -> Result<(), String> {
        Err("flush down".into())
    }
}

#[test]
fn sink_failures_surface_as_persist_errors() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::persistence::PersistError;

    let bids = [lvl(1, 0, 1, 0)];
    let (typed, dynamic, _) = published_book(1, &bids, &[]);

    let mut p = ForegroundPersistor::new(FailingSink);
    p.on_typed(&typed).unwrap();
    let err = p.on_dynamic(&dynamic).unwrap_err();
    assert!(matches!(err, PersistError::Sink(_)));

    let mut pubr =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    pubr.publish(&NormalizedEventRef::Trade {
        symbol: "X",
        exchange_ts_ns: 1,
        receive_ts_ns: 2,
        sequence: 3,
        price: WireDec::new(1, 0),
        size: WireDec::new(1, 0),
        is_buy: true,
    });
    let (typed, _) = pubr.into_adapters();
    let mut p = ForegroundPersistor::new(FailingSink);
    assert!(matches!(
        p.on_typed(&typed.committed[0]).unwrap_err(),
        PersistError::Sink(_)
    ));
    assert!(matches!(p.flush().unwrap_err(), PersistError::Sink(_)));

    Ok(())
}

#[test]
fn queue_bound_drops_oldest_unmatched() -> Result<(), Box<dyn std::error::Error>> {
    // 1026 typed books with no dynamic counterparts overflow the 1024-entry
    // bounded queue; the oldest entries are dropped and counted.
    let mut pubr =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    let bids = [lvl(1, 0, 1, 0)];
    for seq in 1..=1026u64 {
        pubr.publish(&NormalizedEventRef::L2Book {
            symbol: "X",
            exchange_ts_ns: 1,
            receive_ts_ns: 2,
            sequence: seq,
            bids: &bids,
            asks: &[],
        });
    }
    let (typed, _) = pubr.into_adapters();
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    for bytes in &typed.committed {
        p.on_typed(bytes).unwrap();
    }
    assert_eq!(p.counters().unmatched_dropped, 2, "1026 - 1024 dropped");
    assert!(p.sink().l2book_typed.is_empty());

    Ok(())
}

#[test]
fn in_memory_flush_counts() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.flush().unwrap();
    assert_eq!(p.sink().flushes, 1);

    Ok(())
}

#[test]
fn unknown_payload_template_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::normalized_app::AppMessageEncoder;

    // AppMessage whose payload is an 8-byte SBE header with template id 99.
    let payload: [u8; 8] = {
        let mut h = [0u8; 8];
        h[0..2].copy_from_slice(&0u16.to_le_bytes()); // blockLength
        h[2..4].copy_from_slice(&99u16.to_le_bytes()); // templateId
        h[4..6].copy_from_slice(&92u16.to_le_bytes()); // schemaId
        h[6..8].copy_from_slice(&0u16.to_le_bytes()); // version
        h
    };
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(1, payload.len());
    let mut buf = vec![0u8; outer_len];
    {
        let mut outer = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = outer.sent_ts(1);
        let after = outer.app_name(b"x").unwrap();
        let _ = after.payload(&payload).unwrap();
    }

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_typed(&buf).unwrap_err();
    assert!(err.to_string().contains("unknown payload template"));
    assert_eq!(p.counters().decode_failures, 1);

    Ok(())
}

#[test]
fn smaller_dynamic_sequence_is_dropped_and_dynamic_queue_is_bounded()
-> Result<(), Box<dyn std::error::Error>> {
    let bids = [lvl(1, 0, 1, 0)];
    let (_, dynamic_1, _) = published_book(1, &bids, &[]);
    let (typed_2, dynamic_2, _) = published_book(2, &bids, &[]);

    // Dynamic 1 arrives, then typed 2: the smaller dynamic head is dropped.
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    p.on_dynamic(&dynamic_1).unwrap();
    p.on_typed(&typed_2).unwrap();
    p.on_dynamic(&dynamic_2).unwrap();
    assert_eq!(p.counters().unmatched_dropped, 1);
    assert_eq!(p.sink().l2book_typed.len(), 1);

    // Dynamic-side queue bound: 1026 rows with no typed counterparts.
    let mut pubr =
        ClaimPublisher::new(RecordingPublication::new(), RecordingPublication::new()).unwrap();
    for seq in 1..=1026u64 {
        pubr.publish(&NormalizedEventRef::L2Book {
            symbol: "X",
            exchange_ts_ns: 1,
            receive_ts_ns: 2,
            sequence: seq,
            bids: &bids,
            asks: &[],
        });
    }
    let (_, dynamic) = pubr.into_adapters();
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    for bytes in &dynamic.committed {
        p.on_dynamic(bytes).unwrap();
    }
    assert_eq!(p.counters().unmatched_dropped, 2);

    Ok(())
}

#[test]
fn malformed_dynamic_rows_report_structured_decode_errors() -> Result<(), Box<dyn std::error::Error>>
{
    use ergo_clickhouse_persist::sbe::v2::DynamicRowV2Encoder;

    // Row with an unexpected uint64 field id (9) for the publisher layout.
    let mut buf = vec![0u8; 512];
    {
        let mut enc = DynamicRowV2Encoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(1);
        let _ = enc
            .row_metadata(0, |_| {})
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(1, |g| {
                let _ = g.add(|e| {
                    let _ = e.field_id(9).value(1);
                });
            })
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(0, |_| {})
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .decimal_array_fields(0, |_| {})
            .unwrap()
            .symbol_table(b"")
            .unwrap();
    }
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_dynamic(&buf).unwrap_err();
    assert!(err.to_string().contains("unexpected uint64 field"));

    // Row whose string entry claims more bytes than the symbol table holds.
    let mut buf = vec![0u8; 512];
    {
        let mut enc = DynamicRowV2Encoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(1);
        let _ = enc
            .row_metadata(0, |_| {})
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(1, |g| {
                let _ = g.add(|e| {
                    let _ = e.field_id(2).str_len(64);
                });
            })
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .decimal_array_fields(0, |_| {})
            .unwrap()
            .symbol_table(b"tiny")
            .unwrap();
    }
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_dynamic(&buf).unwrap_err();
    assert!(err.to_string().contains("symbol table too short"));

    Ok(())
}

#[test]
fn clickhouse_connect_fails_cleanly_when_ping_rejects() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::persistence::ClickHouseRowSink;
    use std::io::{Read, Write};

    // A stub HTTP server whose every response is 500.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(1) {
            let mut s = stream.unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let _ = s.write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n");
        }
    });

    let err = match ClickHouseRowSink::connect(&format!("http://{addr}")) {
        Ok(_) => panic!("connect must fail when ping rejects"),
        Err(e) => e,
    };
    assert!(err.contains("ping failed"), "unexpected error: {err}");

    Ok(())
}

#[test]
fn unexpected_decimal_array_field_and_mismatched_lengths_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::sbe::v2::DynamicRowV2Encoder;

    fn row(build: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
        let mut buf = vec![0u8; 512];
        build(&mut buf);
        buf
    }

    // Decimal array with an unknown field id.
    let bad_fid = row(|buf| {
        let mut enc = DynamicRowV2Encoder::wrap_and_apply_header(buf, 0).unwrap();
        let _ = enc.schema_id(1);
        let _ = enc
            .row_metadata(0, |_| {})
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(0, |_| {})
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .decimal_array_fields(1, |g| {
                let _ = g.add(|e| {
                    let _ = e.field_id(9);
                    let _ = e.values(0, |_| {});
                });
            })
            .unwrap()
            .symbol_table(b"")
            .unwrap();
    });
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_dynamic(&bad_fid).unwrap_err();
    assert!(err.to_string().contains("unexpected decimal array field"));

    // bid_prices has one value, bid_sizes none → mismatched level lengths.
    let mismatched = row(|buf| {
        let mut enc = DynamicRowV2Encoder::wrap_and_apply_header(buf, 0).unwrap();
        let _ = enc.schema_id(1);
        let _ = enc
            .row_metadata(0, |_| {})
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(0, |_| {})
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .decimal_array_fields(2, |g| {
                let _ = g.add(|e| {
                    let _ = e.field_id(3);
                    let _ = e.values(1, |vg| {
                        let _ = vg.add(|ve| {
                            let _ = ve.mantissa(1).exponent(0);
                        });
                    });
                });
                let _ = g.add(|e| {
                    let _ = e.field_id(4);
                    let _ = e.values(0, |_| {});
                });
            })
            .unwrap()
            .symbol_table(b"")
            .unwrap();
    });
    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_dynamic(&mismatched).unwrap_err();
    assert!(err.to_string().contains("mismatched level array lengths"));

    Ok(())
}

#[test]
fn truncated_l2book_payload_is_a_decode_error() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::normalized_app::AppMessageEncoder;

    let bids = [lvl(1, 0, 1, 0)];
    let (typed, _, _) = published_book(1, &bids, &[]);

    // Extract the intact inner payload, truncate it, and re-wrap it.
    let dec = advanced_bitget::normalized_app::AppMessageDecoder::wrap_and_apply_header(&typed, 0)
        .unwrap();
    let (_, after) = dec.into_app_name().unwrap();
    let (payload, _) = after.into_payload().unwrap();
    let truncated = &payload[..payload.len() - 4];

    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(1, truncated.len());
    let mut buf = vec![0u8; outer_len];
    {
        let mut outer = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = outer.sent_ts(1);
        let after = outer.app_name(b"x").unwrap();
        let _ = after.payload(truncated).unwrap();
    }

    let mut p = ForegroundPersistor::new(InMemorySink::default());
    let err = p.on_typed(&buf).unwrap_err();
    assert!(matches!(
        err,
        advanced_bitget::persistence::PersistError::Decode(_)
    ));

    Ok(())
}

#[test]
fn clickhouse_sql_errors_surface_with_status_and_body() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::persistence::ClickHouseRowSink;
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Stateful stub: 200 for /ping, then 500 for every SQL POST.
    static HITS: AtomicUsize = AtomicUsize::new(0);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { break };
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let n = HITS.fetch_add(1, Ordering::SeqCst);
            let resp: &[u8] = if n == 0 {
                b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\nOk."
            } else {
                b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 4\r\n\r\nboom"
            };
            let _ = s.write_all(resp);
        }
    });

    // Ping succeeds; the first CREATE TABLE fails with the surfaced body.
    let err = match ClickHouseRowSink::connect(&format!("http://{addr}")) {
        Ok(_) => panic!("DDL must fail against the stub"),
        Err(e) => e,
    };
    assert!(err.contains("500"), "status surfaced: {err}");
    assert!(err.contains("boom"), "body surfaced: {err}");

    Ok(())
}
