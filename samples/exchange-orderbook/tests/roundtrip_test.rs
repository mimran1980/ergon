//! Roundtrip encode/decode tests for ErgoSBE-generated exchange orderbook codecs.
#![allow(unused_must_use)] // ponytail: encoder builder calls return &mut Self in tests
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

// ── Bitget: Trade (template 1003) — group + var-data message ──────────────

#[test]
fn bitget_trade_roundtrip() {
    use bitget_spot::{
        trade_encoder_state, InstCategory, Padding5, Padding7, TradeDecoder, TradeEncoder, TradeSide,
    };

    let trades_count = 2u16;
    let symbol = b"ETHUSDT";
    let buf_len =
        TradeEncoder::<trade_encoder_state::NeedsTrades>::compute_encoded_length_with_message_header(
            trades_count as usize,
            symbol.len(),
        );
    let mut buf = vec![0u8; buf_len];

    // Encode
    let mut encoder =
        TradeEncoder::<trade_encoder_state::NeedsTrades>::wrap_and_apply_header(&mut buf, 0)
            .expect("wrap_and_apply_header should succeed");
    encoder
        .price_exponent(-5i8)
        .size_exponent(-3i8)
        .sts(42u64)
        .category(InstCategory::Spot)
        .padding(Padding5([0u8; 5]));
    let after_trades = encoder
        .trades(trades_count, |group| {
            group
                .add(|entry| {
                    entry.ts(1000u64).exec_id(1u64).price(10000i64).size(10i64).side(TradeSide::Buy).padding(Padding7([0u8; 7]));
                })
                .expect("trade entry 0 should succeed");
            group
                .add(|entry| {
                    entry.ts(2000u64).exec_id(2u64).price(20000i64).size(20i64).side(TradeSide::Sell).padding(Padding7([0u8; 7]));
                })
                .expect("trade entry 1 should succeed");
        })
        .expect("trades encoding should succeed");
    let complete = after_trades
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes();

    // Decode
    let decoder = TradeDecoder::try_from(encoded)
        .expect("TradeDecoder::try_from should succeed");

    // Verify scalar fields
    assert_eq!(decoder.price_exponent(), -5, "price_exponent");
    assert_eq!(decoder.size_exponent(), -3, "size_exponent");
    assert_eq!(decoder.sts(), 42, "sts");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");

    // Verify group entries via iterator
    let trades = decoder.trades().expect("trades group decode");
    let entries: Vec<_> = trades.collect();
    assert_eq!(entries.len(), 2, "should have 2 trade entries");

    assert_eq!(entries[0].ts(), 1000, "entry 0 ts");
    assert_eq!(entries[0].exec_id(), 1, "entry 0 exec_id");
    assert_eq!(entries[0].price(), 10000, "entry 0 price");
    assert_eq!(entries[0].size(), 10, "entry 0 size");
    assert_eq!(entries[0].side(), TradeSide::Buy, "entry 0 side");

    assert_eq!(entries[1].ts(), 2000, "entry 1 ts");
    assert_eq!(entries[1].exec_id(), 2, "entry 1 exec_id");
    assert_eq!(entries[1].price(), 20000, "entry 1 price");
    assert_eq!(entries[1].size(), 20, "entry 1 size");
    assert_eq!(entries[1].side(), TradeSide::Sell, "entry 1 side");

    // Verify var-data
    assert_eq!(
        decoder.symbol_as_str().expect("symbol_as_str"),
        "ETHUSDT",
        "symbol"
    );
}

#[test]
fn bitget_trade_max_uint64() {
    use bitget_spot::{
        trade_encoder_state, InstCategory, Padding5, Padding7, TradeDecoder, TradeEncoder, TradeSide,
    };

    let sts_max = u64::MAX;
    let trades_count = 1u16;
    let symbol = b"BTCUSDT";
    let buf_len =
        TradeEncoder::<trade_encoder_state::NeedsTrades>::compute_encoded_length_with_message_header(
            trades_count as usize,
            symbol.len(),
        );
    let mut buf = vec![0u8; buf_len];

    let mut encoder =
        TradeEncoder::<trade_encoder_state::NeedsTrades>::wrap_and_apply_header(&mut buf, 0)
            .expect("wrap_and_apply_header should succeed");
    encoder
        .price_exponent(-8i8)
        .size_exponent(-2i8)
        .sts(sts_max)
        .category(InstCategory::Spot)
        .padding(Padding5([0u8; 5]));
    let after_trades = encoder
        .trades(trades_count, |group| {
            group
                .add(|entry| {
                    entry.ts(1u64).exec_id(1u64).price(1i64).size(1i64).side(TradeSide::Buy).padding(Padding7([0u8; 7]));
                })
                .expect("trade entry should succeed");
        })
        .expect("trades encoding should succeed");
    let complete = after_trades
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes();

    let decoder = TradeDecoder::try_from(encoded)
        .expect("TradeDecoder::try_from should succeed");

    assert_eq!(decoder.sts(), sts_max, "sts should be u64::MAX");
}

#[test]
fn bitget_trade_zero_values() {
    use bitget_spot::{
        trade_encoder_state, InstCategory, Padding5, Padding7, TradeDecoder, TradeEncoder, TradeSide,
    };

    let trades_count = 1u16;
    let symbol = b"";
    let buf_len =
        TradeEncoder::<trade_encoder_state::NeedsTrades>::compute_encoded_length_with_message_header(
            trades_count as usize,
            symbol.len(),
        );
    let mut buf = vec![0u8; buf_len];

    let mut encoder =
        TradeEncoder::<trade_encoder_state::NeedsTrades>::wrap_and_apply_header(&mut buf, 0)
            .expect("wrap_and_apply_header should succeed");
    encoder
        .price_exponent(0i8)
        .size_exponent(0i8)
        .sts(0u64)
        .category(InstCategory::Spot)
        .padding(Padding5([0u8; 5]));
    let after_trades = encoder
        .trades(trades_count, |group| {
            group
                .add(|entry| {
                    entry.ts(0u64).exec_id(0u64).price(0i64).size(0i64).side(TradeSide::Buy).padding(Padding7([0u8; 7]));
                })
                .expect("trade entry should succeed");
        })
        .expect("trades encoding should succeed");
    let complete = after_trades
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes();

    let decoder = TradeDecoder::try_from(encoded)
        .expect("TradeDecoder::try_from should succeed");

    assert_eq!(decoder.price_exponent(), 0, "price_exponent");
    assert_eq!(decoder.size_exponent(), 0, "size_exponent");
    assert_eq!(decoder.sts(), 0, "sts");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");
    assert_eq!(
        decoder.symbol().expect("symbol"),
        b"",
        "empty symbol"
    );

    let trades = decoder.trades().expect("trades group decode");
    let entry = trades.nth(0).expect("first trade entry");
    assert_eq!(entry.ts(), 0, "entry ts");
    assert_eq!(entry.exec_id(), 0, "entry exec_id");
    assert_eq!(entry.price(), 0, "entry price");
    assert_eq!(entry.size(), 0, "entry size");
    assert_eq!(entry.side(), TradeSide::Buy, "entry side");
}

// ── Binance: WebSocketSessionLogonResponse (template 51) — multiple scalars + var data ──

#[test]
fn binance_logon_response_roundtrip() {
    use binance_spot::{
        web_socket_session_logon_response_encoder_state, BoolEnum,
        WebSocketSessionLogonResponseDecoder, WebSocketSessionLogonResponseEncoder,
    };

    let api_key = b"my-test-api-key";
    let buf_len =
        WebSocketSessionLogonResponseEncoder::<web_socket_session_logon_response_encoder_state::NeedsLoggedOnApiKey>::compute_encoded_length_with_message_header(api_key.len());
    let mut buf = vec![0u8; buf_len];

    // Encode
    let mut encoder =
        WebSocketSessionLogonResponseEncoder::<web_socket_session_logon_response_encoder_state::NeedsLoggedOnApiKey>::wrap_and_apply_header(
            &mut buf, 0,
        )
        .expect("wrap_and_apply_header should succeed");
    encoder
        .authorized_since(1712345678000000i64)
        .connected_since(1712345679000000i64)
        .return_rate_limits(BoolEnum::True)
        .server_time(1712345680000000i64)
        .user_data_stream(BoolEnum::False);
    let complete = encoder
        .logged_on_api_key(api_key)
        .expect("logged_on_api_key encoding should succeed");
    let encoded = complete.as_bytes();

    // Decode
    let decoder = WebSocketSessionLogonResponseDecoder::try_from(encoded)
        .expect("WebSocketSessionLogonResponseDecoder::try_from should succeed");

    assert_eq!(decoder.authorized_since(), 1712345678000000_i64, "authorized_since");
    assert_eq!(decoder.connected_since(), 1712345679000000_i64, "connected_since");
    assert_eq!(decoder.return_rate_limits(), BoolEnum::True, "return_rate_limits");
    assert_eq!(decoder.server_time(), 1712345680000000_i64, "server_time");
    assert_eq!(decoder.user_data_stream(), BoolEnum::False, "user_data_stream");
    assert_eq!(
        decoder.logged_on_api_key_as_str().expect("logged_on_api_key_as_str"),
        "my-test-api-key",
        "logged_on_api_key"
    );
}

// ── Binance: WebSocketResponse (template 50) — group + var-data message ───

#[test]
fn binance_websocket_response_group_roundtrip() {
    use binance_spot::{
        web_socket_response_encoder_state, BoolEnum, RateLimitInterval, RateLimitType,
        WebSocketResponseDecoder, WebSocketResponseEncoder,
    };

    let rate_limits_count = 2u16;
    let id = b"test-id-1";
    let result = b"{\"data\":\"ok\"}";
    let buf_len =
        WebSocketResponseEncoder::<web_socket_response_encoder_state::NeedsRateLimits>::compute_encoded_length_with_message_header(
            rate_limits_count as usize,
            id.len(),
            result.len(),
        );
    let mut buf = vec![0u8; buf_len];

    // Encode
    let mut encoder =
        WebSocketResponseEncoder::<web_socket_response_encoder_state::NeedsRateLimits>::wrap_and_apply_header(
            &mut buf, 0,
        )
        .expect("wrap_and_apply_header should succeed");
    encoder
        .sbe_schema_id_version_deprecated(BoolEnum::False)
        .status(200u16);
    let after_rate_limits = encoder
        .rate_limits(rate_limits_count, |group| {
            group
                .add(|entry| {
                    entry
                        .rate_limit_type(RateLimitType::RequestWeight)
                        .interval(RateLimitInterval::Minute)
                        .interval_num(1u8)
                        .rate_limit(1200i64)
                        .current(50i64);
                })
                .expect("rate limit entry 0 should succeed");
            group
                .add(|entry| {
                    entry
                        .rate_limit_type(RateLimitType::Orders)
                        .interval(RateLimitInterval::Second)
                        .interval_num(10u8)
                        .rate_limit(100i64)
                        .current(0i64);
                })
                .expect("rate limit entry 1 should succeed");
        })
        .expect("rate_limits encoding should succeed");
    let after_id = after_rate_limits
        .id(id)
        .expect("id encoding should succeed");
    let complete = after_id
        .result(result)
        .expect("result encoding should succeed");
    let encoded = complete.as_bytes();

    // Decode
    let decoder = WebSocketResponseDecoder::try_from(encoded)
        .expect("WebSocketResponseDecoder::try_from should succeed");

    assert_eq!(
        decoder.sbe_schema_id_version_deprecated(),
        BoolEnum::False,
        "sbe_schema_id_version_deprecated"
    );
    assert_eq!(decoder.status(), 200, "status");

    // Verify group entries
    let rate_limits = decoder.rate_limits().expect("rate_limits group decode");
    let entries: Vec<_> = rate_limits.collect();
    assert_eq!(entries.len(), 2, "should have 2 rate limit entries");

    assert_eq!(entries[0].rate_limit_type(), RateLimitType::RequestWeight, "entry 0 type");
    assert_eq!(entries[0].interval(), RateLimitInterval::Minute, "entry 0 interval");
    assert_eq!(entries[0].interval_num(), 1, "entry 0 interval_num");
    assert_eq!(entries[0].rate_limit(), 1200, "entry 0 rate_limit");
    assert_eq!(entries[0].current(), 50, "entry 0 current");

    assert_eq!(entries[1].rate_limit_type(), RateLimitType::Orders, "entry 1 type");
    assert_eq!(entries[1].interval(), RateLimitInterval::Second, "entry 1 interval");
    assert_eq!(entries[1].interval_num(), 10, "entry 1 interval_num");
    assert_eq!(entries[1].rate_limit(), 100, "entry 1 rate_limit");
    assert_eq!(entries[1].current(), 0, "entry 1 current");

    // Verify var-data fields
    assert_eq!(
        decoder.id_as_str().expect("id_as_str"),
        "test-id-1",
        "id"
    );
    assert_eq!(
        decoder.result_as_str().expect("result_as_str"),
        "{\"data\":\"ok\"}",
        "result"
    );
}

#[test]
fn binance_websocket_response_group_buffer_too_short() {
    use binance_spot::WebSocketResponseDecoder;

    // Encode a valid header + body but truncate before the group data
    let mut buf = [0u8; 11];
    // Header: blockLength=3, templateId=50, schemaId=3, version=5
    buf[0..8].copy_from_slice(&[3, 0, 50, 0, 3, 0, 5, 0]);
    // Body: sbe_schema_id_version_deprecated=false, status=200
    buf[8] = 0; // BoolEnum::False
    buf[9..11].copy_from_slice(&200u16.to_le_bytes());

    // The buffer has no group dimension or var data — accessing group should fail
    let decoder = WebSocketResponseDecoder::try_from(&buf[..])
        .expect("try_from should succeed at header level");
    let result = decoder.rate_limits();
    assert!(
        result.is_err(),
        "rate_limits() on buffer without group dim should fail"
    );
}

// ── Wrong-schema detection: bitget schema bytes rejected by binance, and vice versa ──

#[test]
fn wrong_schema_bitget_encoded_rejected_by_binance() {
    use binance_spot::WebSocketResponseDecoder;
    use bitget_spot::{
        best_bid_ask_encoder_state, BestBidAskEncoder, InstCategory, Padding5,
    };

    // Encode a valid bitget BestBidAsk message
    let symbol = b"BTCUSDT";
    let buf_len = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::compute_encoded_length_with_message_header(symbol.len());
    let mut buf = vec![0u8; buf_len];

    let mut encoder = BestBidAskEncoder::<best_bid_ask_encoder_state::NeedsSymbol>::wrap_and_apply_header(&mut buf, 0)
        .expect("wrap_and_apply_header should succeed");
    encoder
        .ts(1u64)
        .bid1_price(2i64)
        .bid1_size(3i64)
        .ask1_price(4i64)
        .ask1_size(5i64)
        .price_exponent(-8i8)
        .size_exponent(-2i8)
        .seq(10u64)
        .sts(20u64)
        .category(InstCategory::Spot)
        .padding(Padding5([0u8; 5]));
    let complete = encoder
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes();

    // Try to decode as binance WebSocketResponse — should fail with WrongSchema
    let result = WebSocketResponseDecoder::try_from(encoded);
    assert!(
        result.is_err(),
        "bitget-encoded data rejected by binance decoder: schema_id differs"
    );
}

#[test]
fn wrong_schema_binance_encoded_rejected_by_bitget() {
    use binance_spot::ServerTimeResponseEncoder;
    use bitget_spot::BestBidAskDecoder;

    // Encode a valid binance ServerTimeResponse message
    let mut buf = [0u8; ServerTimeResponseEncoder::ENCODED_LENGTH];
    let mut encoder = ServerTimeResponseEncoder::wrap_and_apply_header(&mut buf, 0)
        .expect("wrap_and_apply_header should succeed");
    encoder.server_time(42);
    let encoded = encoder.as_ref();

    // Try to decode as bitget BestBidAsk — should fail with WrongSchema
    let result = BestBidAskDecoder::try_from(encoded);
    assert!(
        result.is_err(),
        "binance-encoded data rejected by bitget decoder: schema_id differs"
    );
}
