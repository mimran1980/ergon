//! Roundtrip encode/decode tests for ErgoSBE-generated exchange orderbook codecs.
//!
//! Tests that encoding a message and decoding the resulting bytes yields the
//! original field values. Each exchange schema gets at least one message type.
//!
//! Run: `RUSTC_WRAPPER="" cargo test -- --test-threads=1 --nocapture`

mod bitget_spot {
    include!(concat!(env!("OUT_DIR"), "/bitget_spot.rs"));
}
mod binance_spot {
    include!(concat!(env!("OUT_DIR"), "/binance_spot.rs"));
}

// ── Bitget: BestBidAsk (template 1002) ────────────────────────────────────

#[test]
fn bitget_best_bid_ask_roundtrip() {
    use bitget_spot::{
        best_bid_ask_encoder_state, BestBidAskDecoder, BestBidAskEncoder, InstCategory, Padding5,
    };

    let symbol = b"BTCUSDT";
    let buf_len = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::compute_encoded_length_with_message_header(symbol.len());
    let mut buf = vec![0u8; buf_len];

    // Encode
    let mut encoder = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::wrap_and_apply_header(&mut buf, 0)
        .expect("wrap_and_apply_header should succeed");
    encoder
        .ts(1712345678000u64)
        .bid1_price(50000123456i64)
        .bid1_size(123456789i64)
        .ask1_price(50000987654i64)
        .ask1_size(987654321i64)
        .price_exponent(-8i8)
        .size_exponent(-2i8)
        .seq(42u64)
        .sts(99u64)
        .category(InstCategory::Spot)
        .padding(Padding5([0u8; 5]));
    let complete = encoder
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes();

    // Decode
    let decoder =
        BestBidAskDecoder::try_from(encoded).expect("BestBidAskDecoder::try_from should succeed");

    assert_eq!(decoder.ts(), 1712345678000, "ts");
    assert_eq!(decoder.bid1_price(), 50000123456, "bid1_price");
    assert_eq!(decoder.bid1_size(), 123456789, "bid1_size");
    assert_eq!(decoder.ask1_price(), 50000987654, "ask1_price");
    assert_eq!(decoder.ask1_size(), 987654321, "ask1_size");
    assert_eq!(decoder.price_exponent(), -8, "price_exponent");
    assert_eq!(decoder.size_exponent(), -2, "size_exponent");
    assert_eq!(decoder.seq(), 42, "seq");
    assert_eq!(decoder.sts(), 99, "sts");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");
    assert_eq!(
        decoder.symbol_as_str().expect("symbol_as_str"),
        "BTCUSDT",
        "symbol"
    );
}

#[test]
fn bitget_best_bid_ask_verify_passes() {
    use bitget_spot::{
        best_bid_ask_encoder_state, BestBidAskDecoder, BestBidAskEncoder,
    };

    let symbol = b"BTCUSDT";
    let buf_len = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::compute_encoded_length_with_message_header(symbol.len());
    let mut buf = vec![0u8; buf_len];

    let mut encoder = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder
        .ts(1)
        .bid1_price(2)
        .bid1_size(3)
        .ask1_price(4)
        .ask1_size(5)
        .price_exponent(-8)
        .size_exponent(-2)
        .seq(10)
        .sts(20)
        .category(bitget_spot::InstCategory::Spot)
        .padding(bitget_spot::Padding5([0u8; 5]));
    let complete = encoder
        .symbol(symbol)
        .unwrap();
    let encoded = complete.as_bytes();

    assert!(BestBidAskDecoder::verify(encoded).is_ok());
}

// ── Bitget: Depth50 group roundtrip (template 1001) ──────────────────────

#[test]
fn bitget_depth50_group_roundtrip() {
    use bitget_spot::{
        depth50_encoder_state, Depth50Decoder, Depth50Encoder, InstCategory, Padding5,
    };

    let asks_count = 3u16;
    let bids_count = 2u16;
    let symbol = b"BTCUSDT";
    let buf_len = Depth50Encoder::<depth50_encoder_state::NeedsAsks>::compute_encoded_length_with_message_header(
        asks_count as usize,
        bids_count as usize,
        symbol.len(),
    );
    let mut buf = vec![0u8; buf_len];

    // Encode
    let mut encoder = Depth50Encoder::<depth50_encoder_state::NeedsAsks>::wrap_and_apply_header(&mut buf, 0)
        .expect("wrap_and_apply_header should succeed");
    encoder
        .ts(1000u64)
        .seq(1u64)
        .price_exponent(-8i8)
        .size_exponent(-2i8)
        .sts(0u64)
        .category(InstCategory::Spot)
        .padding(Padding5([0u8; 5]));
    let after_asks = encoder
        .asks(asks_count, |group| {
            group
                .add(|entry| {
                    entry.price(100i64).size(10i64);
                })
                .expect("ask entry 0 should succeed");
            group
                .add(|entry| {
                    entry.price(200i64).size(20i64);
                })
                .expect("ask entry 1 should succeed");
            group
                .add(|entry| {
                    entry.price(300i64).size(30i64);
                })
                .expect("ask entry 2 should succeed");
        })
        .expect("asks encoding should succeed");
    let after_bids = after_asks
        .bids(bids_count, |group| {
            group
                .add(|entry| {
                    entry.price(1000i64).size(100i64);
                })
                .expect("bid entry 0 should succeed");
            group
                .add(|entry| {
                    entry.price(2000i64).size(200i64);
                })
                .expect("bid entry 1 should succeed");
        })
        .expect("bids encoding should succeed");
    let complete = after_bids
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes();

    // Decode
    let decoder =
        Depth50Decoder::try_from(encoded).expect("Depth50Decoder::try_from should succeed");

    // Verify scalar fields
    assert_eq!(decoder.ts(), 1000, "ts");
    assert_eq!(decoder.seq(), 1, "seq");
    assert_eq!(decoder.price_exponent(), -8, "price_exponent");
    assert_eq!(decoder.size_exponent(), -2, "size_exponent");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");
    assert_eq!(
        decoder.symbol_as_str().expect("symbol_as_str"),
        "BTCUSDT",
        "symbol"
    );

    // Verify asks group entries
    let asks = decoder.asks().expect("asks group decode");
    let ask_prices: Vec<i64> = asks.map(|e| e.price()).collect();
    assert_eq!(ask_prices, vec![100, 200, 300], "ask prices");

    // Verify bids group entries
    let bids = decoder.bids().expect("bids group decode");
    let bid_prices: Vec<i64> = bids.map(|e| e.price()).collect();
    assert_eq!(bid_prices, vec![1000, 2000], "bid prices");
}

// ── Buffer-too-short error tests ──────────────────────────────────────────

#[test]
fn bitget_buffer_too_short_for_header() {
    use bitget_spot::BestBidAskDecoder;

    let too_short = [0u8; 4];
    let result = BestBidAskDecoder::try_from(&too_short[..]);
    assert!(
        result.is_err(),
        "decoding from a 4-byte buffer should fail"
    );
}

#[test]
fn bitget_verify_too_short() {
    use bitget_spot::BestBidAskDecoder;

    let too_short = [0u8; 4];
    let result = BestBidAskDecoder::verify(&too_short[..]);
    assert!(
        result.is_err(),
        "verify on a 4-byte buffer should fail"
    );
}

#[test]
fn bitget_encoder_buffer_too_short() {
    use bitget_spot::{best_bid_ask_encoder_state, BestBidAskEncoder};

    // Buffer too small even for header + block length
    let mut small_buf = [0u8; 4];
    let result = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::wrap_and_apply_header(&mut small_buf[..], 0);
    assert!(
        result.is_err(),
        "wrap_and_apply_header on a 4-byte buffer should fail"
    );
}

// ── Binance: ServerTimeResponse (template 102) ────────────────────────────

#[test]
fn binance_server_time_roundtrip() {
    use binance_spot::ServerTimeResponseDecoder;
    use binance_spot::ServerTimeResponseEncoder;

    let expected_ts: i64 = 1712345678000123;

    // Encode — ServerTimeResponseEncoder has no type-state, just plain methods
    let mut buf = [0u8; ServerTimeResponseEncoder::ENCODED_LENGTH];
    let mut encoder = ServerTimeResponseEncoder::wrap_and_apply_header(&mut buf, 0)
        .expect("wrap_and_apply_header should succeed");
    encoder.server_time(expected_ts);
    let encoded = encoder.as_ref();

    // Decode
    let decoder = ServerTimeResponseDecoder::try_from(encoded)
        .expect("ServerTimeResponseDecoder::try_from should succeed");
    assert_eq!(decoder.server_time(), expected_ts, "server_time");
}

#[test]
fn binance_server_time_verify_passes() {
    use binance_spot::ServerTimeResponseDecoder;
    use binance_spot::ServerTimeResponseEncoder;

    let mut buf = [0u8; ServerTimeResponseEncoder::ENCODED_LENGTH];
    let mut encoder = ServerTimeResponseEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.server_time(42);
    let encoded = encoder.as_ref();

    assert!(ServerTimeResponseDecoder::verify(encoded).is_ok());
}

#[test]
fn binance_server_time_buffer_too_short() {
    use binance_spot::ServerTimeResponseDecoder;

    let result = ServerTimeResponseDecoder::try_from(&[0u8; 4][..]);
    assert!(
        result.is_err(),
        "decoding from a 4-byte buffer should fail"
    );
}
