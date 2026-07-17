//! BitgetIngestor pure state-machine tests — the Task 7/8 seam.
//!
//! All tests drive the ingestor through `apply(event, emit)` only: borrowed
//! events in, borrowed normalized events out through the callback.

use advanced_bitget::bitget::{ApplyError, BitgetEventRef, BitgetIngestor};
use advanced_bitget::market::NormalizedEventRef;

/// Collected owned snapshot of an emitted book for assertions.
#[derive(Debug, PartialEq)]
struct BookOut {
    symbol: String,
    sequence: u64,
    bids: Vec<((i64, i8), (i64, i8))>,
    asks: Vec<((i64, i8), (i64, i8))>,
}

fn collect_books(
    events: &mut Vec<BookOut>,
) -> impl FnMut(NormalizedEventRef<'_>) -> Result<(), std::convert::Infallible> + '_ {
    |ev| {
        if let NormalizedEventRef::L2Book {
            symbol,
            sequence,
            bids,
            asks,
            ..
        } = ev
        {
            events.push(BookOut {
                symbol: symbol.to_string(),
                sequence,
                bids: bids
                    .iter()
                    .map(|l| {
                        (
                            (l.price.mantissa, l.price.exponent),
                            (l.size.mantissa, l.size.exponent),
                        )
                    })
                    .collect(),
                asks: asks
                    .iter()
                    .map(|l| {
                        (
                            (l.price.mantissa, l.price.exponent),
                            (l.size.mantissa, l.size.exponent),
                        )
                    })
                    .collect(),
            });
        }
        Ok(())
    }
}

fn snapshot<'a>(bids: &'a [[&'a str; 2]], asks: &'a [[&'a str; 2]]) -> BitgetEventRef<'a> {
    BitgetEventRef::BookSnapshot {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_000,
        bids,
        asks,
    }
}

fn update<'a>(bids: &'a [[&'a str; 2]], asks: &'a [[&'a str; 2]]) -> BitgetEventRef<'a> {
    BitgetEventRef::BookUpdate {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_001,
        bids,
        asks,
    }
}

#[test]
fn snapshot_emits_ordered_book() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    ing.apply(
        snapshot(
            &[["50000.5", "1.5"], ["50001.0", "2.0"]],
            &[["50002.0", "3.0"], ["50001.5", "1.0"]],
        ),
        collect_books(&mut out),
    )
    .unwrap();

    assert_eq!(out.len(), 1);
    let book = &out[0];
    assert_eq!(book.symbol, "BTCUSDT");
    // Bids descending (best first): 50001.0 then 50000.5.
    assert_eq!(book.bids[0].0, (500010, -1));
    assert_eq!(book.bids[1].0, (500005, -1));
    // Asks ascending (best first): 50001.5 then 50002.0.
    assert_eq!(book.asks[0].0, (500015, -1));
    assert_eq!(book.asks[1].0, (500020, -1));
}

#[test]
fn update_before_snapshot_is_suppressed_and_counted() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    ing.apply(update(&[["50000.5", "1.5"]], &[]), collect_books(&mut out))
        .unwrap();
    assert!(out.is_empty(), "no book may be emitted before a snapshot");
    assert_eq!(ing.counters().updates_before_snapshot, 1);
}

#[test]
fn update_after_snapshot_applies_and_zero_size_deletes() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    ing.apply(
        snapshot(&[["100.0", "1.0"], ["99.0", "2.0"]], &[["101.0", "1.0"]]),
        collect_books(&mut out),
    )
    .unwrap();
    // Delete the 99.0 bid (size 0), modify the 100.0 bid, add an ask.
    ing.apply(
        update(&[["99.0", "0"], ["100.0", "5.0"]], &[["102.0", "7.0"]]),
        collect_books(&mut out),
    )
    .unwrap();

    assert_eq!(out.len(), 2);
    let book = &out[1];
    assert_eq!(book.bids.len(), 1, "zero-size level must be deleted");
    assert_eq!(book.bids[0], ((1000, -1), (50, -1)));
    assert_eq!(book.asks.len(), 2);
    assert_eq!(book.asks[0].0, (1010, -1));
    assert_eq!(book.asks[1].0, (1020, -1));
}

#[test]
fn malformed_price_is_structured_error_not_zero() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    let err = ing
        .apply(
            snapshot(&[["not_a_number", "1.0"]], &[]),
            collect_books(&mut out),
        )
        .unwrap_err();
    assert!(matches!(err, ApplyError::MalformedDecimal { .. }));
    assert!(out.is_empty(), "malformed data must never emit a book");
    assert_eq!(ing.counters().malformed_values, 1);
}

#[test]
fn callback_error_bubbles_unchanged() {
    #[derive(Debug, PartialEq)]
    struct AppError(&'static str);

    let mut ing = BitgetIngestor::new();
    let err = ing
        .apply(
            snapshot(&[["100.0", "1.0"]], &[]),
            |_ev| -> Result<(), AppError> { Err(AppError("downstream full")) },
        )
        .unwrap_err();
    assert_eq!(err, ApplyError::Emit(AppError("downstream full")));
}

#[test]
fn sequence_is_monotonic_across_emitted_books() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    ing.apply(snapshot(&[["1.0", "1.0"]], &[]), collect_books(&mut out))
        .unwrap();
    ing.apply(update(&[["2.0", "1.0"]], &[]), collect_books(&mut out))
        .unwrap();
    assert_eq!(out.len(), 2);
    assert!(out[1].sequence > out[0].sequence);
}

#[test]
fn trade_emits_normalized_trade() {
    let mut ing = BitgetIngestor::new();
    let mut trades = Vec::new();
    ing.apply(
        BitgetEventRef::Trade {
            symbol: "BTCUSDT",
            exchange_ts_ns: 1_700_000_000_000_000_002,
            price: "50000.5",
            size: "0.25",
            is_buy: true,
        },
        |ev| -> Result<(), std::convert::Infallible> {
            if let NormalizedEventRef::Trade {
                symbol,
                price,
                size,
                is_buy,
                ..
            } = ev
            {
                trades.push((
                    symbol.to_string(),
                    (price.mantissa, price.exponent),
                    (size.mantissa, size.exponent),
                    is_buy,
                ));
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        trades,
        vec![("BTCUSDT".to_string(), (500005, -1), (25, -2), true)]
    );
}

#[test]
fn disconnect_clears_state_and_resuppresses_until_snapshot() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    ing.apply(snapshot(&[["1.0", "1.0"]], &[]), collect_books(&mut out))
        .unwrap();
    assert_eq!(out.len(), 1);

    ing.on_disconnect();

    ing.apply(update(&[["2.0", "1.0"]], &[]), collect_books(&mut out))
        .unwrap();
    assert_eq!(out.len(), 1, "book publication suppressed after reconnect");
    assert_eq!(ing.counters().updates_before_snapshot, 1);

    ing.apply(snapshot(&[["3.0", "1.0"]], &[]), collect_books(&mut out))
        .unwrap();
    assert_eq!(out.len(), 2, "fresh snapshot resumes publication");
    // Stale pre-disconnect levels must not leak into the fresh book.
    assert_eq!(out[1].bids.len(), 1);
    assert_eq!(out[1].bids[0].0, (30, -1));
}

// ── WebSocket frame parsing (captured fixtures) ─────────────────────────

use advanced_bitget::bitget::{FrameError, parse_frame};

#[test]
fn book_snapshot_fixture_drives_ingestor() {
    let frame = parse_frame(include_str!("fixtures/bitget/book_snapshot.json")).unwrap();
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    frame.apply_to(&mut ing, collect_books(&mut out)).unwrap();

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].symbol, "BTCUSDT");
    assert_eq!(out[0].bids.len(), 2);
    assert_eq!(out[0].bids[0].0, (500010, -1), "best bid first");
    assert_eq!(out[0].asks[0].0, (500015, -1), "best ask first");
}

#[test]
fn book_update_fixture_applies_deletion() {
    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    parse_frame(include_str!("fixtures/bitget/book_snapshot.json"))
        .unwrap()
        .apply_to(&mut ing, collect_books(&mut out))
        .unwrap();
    parse_frame(include_str!("fixtures/bitget/book_update.json"))
        .unwrap()
        .apply_to(&mut ing, collect_books(&mut out))
        .unwrap();

    assert_eq!(out.len(), 2);
    // The 50002.0 ask was deleted (size 0); the 50000.5 bid was modified.
    assert_eq!(out[1].asks.len(), 1);
    assert_eq!(out[1].bids[1], ((500005, -1), (25, -1)));
}

#[test]
fn trades_fixture_emits_each_trade() {
    let mut ing = BitgetIngestor::new();
    let mut trades = Vec::new();
    parse_frame(include_str!("fixtures/bitget/trades.json"))
        .unwrap()
        .apply_to(&mut ing, |ev| -> Result<(), std::convert::Infallible> {
            if let NormalizedEventRef::Trade {
                exchange_ts_ns,
                price,
                is_buy,
                ..
            } = ev
            {
                trades.push((exchange_ts_ns, (price.mantissa, price.exponent), is_buy));
            }
            Ok(())
        })
        .unwrap();

    assert_eq!(
        trades,
        vec![
            (1_700_000_000_200_000_000, (500005, -1), true),
            (1_700_000_000_201_000_000, (500000, -1), false),
        ]
    );
}

#[test]
fn subscribe_ack_and_pong_are_control_frames() {
    let mut ing = BitgetIngestor::new();
    for text in [include_str!("fixtures/bitget/subscribe_ack.json"), "pong"] {
        let mut emitted = false;
        parse_frame(text)
            .unwrap()
            .apply_to(&mut ing, |_| -> Result<(), std::convert::Infallible> {
                emitted = true;
                Ok(())
            })
            .unwrap();
        assert!(!emitted, "control frames must not emit events: {text}");
    }
}

#[test]
fn malformed_price_fixture_is_rejected_in_apply() {
    let mut ing = BitgetIngestor::new();
    let err = parse_frame(include_str!("fixtures/bitget/malformed_price.json"))
        .unwrap()
        .apply_to(&mut ing, |_| -> Result<(), std::convert::Infallible> {
            panic!("must not emit")
        })
        .unwrap_err();
    assert!(matches!(err, ApplyError::MalformedDecimal { .. }));
}

#[test]
fn garbage_text_is_a_frame_error() {
    assert!(matches!(parse_frame("{not json"), Err(FrameError::Json(_))));
}

#[test]
fn book_deeper_than_max_levels_is_truncated_to_best() {
    use advanced_bitget::config::MAX_BOOK_LEVELS;

    let deep: Vec<[String; 2]> = (0..MAX_BOOK_LEVELS + 10)
        .map(|i| [format!("{}", 1000 + i), "1".to_string()])
        .collect();
    let refs: Vec<[&str; 2]> = deep
        .iter()
        .map(|l| [l[0].as_str(), l[1].as_str()])
        .collect();

    let mut ing = BitgetIngestor::new();
    let mut out = Vec::new();
    ing.apply(snapshot(&refs, &[]), collect_books(&mut out))
        .unwrap();

    assert_eq!(out[0].bids.len(), MAX_BOOK_LEVELS, "truncated to cap");
    // Best (highest) bid retained.
    assert_eq!(out[0].bids[0].0, (1000 + (MAX_BOOK_LEVELS as i64 + 9), 0));
}
