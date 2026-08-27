//! Roundtrip encode/decode tests for ergo-sbe-generated exchange orderbook codecs.
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused,
    warnings
)]
#![allow(unused_must_use)] // encoder builder calls return &mut Self in tests, remove when builder API uses unit return
//!
//! Tests that encoding a message and decoding the resulting bytes yields the
//! original field values. Each exchange schema gets at least one message type.
//!
//! Run: `RUSTC_WRAPPER="" cargo test -- --test-threads=1 --nocapture`

// ── Bitget: BestBidAsk (template 1002) ────────────────────────────────────

#[test]
fn bitget_best_bid_ask_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::{
        BestBidAskDecoder, BestBidAskEncoder, BestBidAskFixedFields, InstCategory, Padding5,
    };

    let symbol = b"BTCUSDT";
    let buf_len = BestBidAskEncoder::compute_encoded_length_with_message_header(symbol.len());
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    // Encode
    let complete = BestBidAskEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&BestBidAskFixedFields {
            ts: 1712345678000u64,
            bid1_price: 50000123456i64,
            bid1_size: 123456789i64,
            ask1_price: 50000987654i64,
            ask1_size: 987654321i64,
            price_exponent: -8i8,
            size_exponent: -2i8,
            seq: 42u64,
            sts: 99u64,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes_with_header();

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
    let (symbol_bytes, _complete) = decoder.into_symbol().expect("symbol");
    assert_eq!(
        core::str::from_utf8(symbol_bytes).expect("symbol_as_str"),
        "BTCUSDT",
        "symbol"
    );

    Ok(())
}

#[test]
fn bitget_best_bid_ask_verify_passes() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::{
        BestBidAskDecoder, BestBidAskEncoder, BestBidAskFixedFields, InstCategory, Padding5,
    };

    let symbol = b"BTCUSDT";
    let buf_len = BestBidAskEncoder::compute_encoded_length_with_message_header(symbol.len());
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    let complete = BestBidAskEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&BestBidAskFixedFields {
            ts: 1,
            bid1_price: 2,
            bid1_size: 3,
            ask1_price: 4,
            ask1_size: 5,
            price_exponent: -8,
            size_exponent: -2,
            seq: 10,
            sts: 20,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .symbol(symbol)
        .unwrap();
    let encoded = complete.as_bytes_with_header();

    assert!(BestBidAskDecoder::verify(encoded).is_ok());

    Ok(())
}

// ── Bitget: Depth50 group roundtrip (template 1001) ──────────────────────

#[test]
fn bitget_depth50_group_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::{
        Depth50Decoder, Depth50Encoder, Depth50FixedFields, InstCategory, Padding5,
    };

    let asks_count = 3u16;
    let bids_count = 2u16;
    let symbol = b"BTCUSDT";
    let buf_len = Depth50Encoder::compute_encoded_length_with_message_header(
        asks_count as usize,
        bids_count as usize,
        symbol.len(),
    );
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    // Encode
    let after_asks = Depth50Encoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&Depth50FixedFields {
            ts: 1000u64,
            seq: 1u64,
            price_exponent: -8i8,
            size_exponent: -2i8,
            sts: 0u64,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .asks(asks_count, |group| {
            group
                .add(|entry| {
                    entry.price(100i64).size(10i64);
                    Ok(())
                })
                .expect("ask entry 0 should succeed");
            group
                .add(|entry| {
                    entry.price(200i64).size(20i64);
                    Ok(())
                })
                .expect("ask entry 1 should succeed");
            group
                .add(|entry| {
                    entry.price(300i64).size(30i64);
                    Ok(())
                })
                .expect("ask entry 2 should succeed");
            Ok(())
        })
        .expect("asks encoding should succeed");
    let after_bids = after_asks
        .bids(bids_count, |group| {
            group
                .add(|entry| {
                    entry.price(1000i64).size(100i64);
                    Ok(())
                })
                .expect("bid entry 0 should succeed");
            group
                .add(|entry| {
                    entry.price(2000i64).size(200i64);
                    Ok(())
                })
                .expect("bid entry 1 should succeed");
            Ok(())
        })
        .expect("bids encoding should succeed");
    let complete = after_bids
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    // Decode
    let decoder =
        Depth50Decoder::try_from(encoded).expect("Depth50Decoder::try_from should succeed");

    // Verify scalar fields
    assert_eq!(decoder.ts(), 1000, "ts");
    assert_eq!(decoder.seq(), 1, "seq");
    assert_eq!(decoder.price_exponent(), -8, "price_exponent");
    assert_eq!(decoder.size_exponent(), -2, "size_exponent");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");

    // Tail components in wire order: asks -> bids -> symbol. The consuming
    // Stages enforce this order; symbol is read last.
    let mut asks = decoder.into_asks().expect("asks group decode");
    let mut ask_prices = Vec::new();
    while let Some(e) = asks.next() {
        ask_prices.push(e.price());
    }
    assert_eq!(ask_prices, vec![100, 200, 300], "ask prices");

    let mut bids = asks
        .finish()
        .expect("finish asks")
        .into_bids()
        .expect("bids group decode");
    let mut bid_prices = Vec::new();
    while let Some(e) = bids.next() {
        bid_prices.push(e.price());
    }
    assert_eq!(bid_prices, vec![1000, 2000], "bid prices");

    let (symbol, _done) = bids
        .finish()
        .expect("finish bids")
        .into_symbol()
        .expect("symbol decode");
    assert_eq!(core::str::from_utf8(symbol).unwrap(), "BTCUSDT", "symbol");

    Ok(())
}

// ── Buffer-too-short error tests ──────────────────────────────────────────

#[test]
fn bitget_buffer_too_short_for_header() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::BestBidAskDecoder;

    let too_short = [0u8; 4];
    let result = BestBidAskDecoder::try_from(&too_short[..]);
    assert!(result.is_err(), "decoding from a 4-byte buffer should fail");

    Ok(())
}

#[test]
fn bitget_verify_too_short() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::BestBidAskDecoder;

    let too_short = [0u8; 4];
    let result = BestBidAskDecoder::verify(&too_short[..]);
    assert!(result.is_err(), "verify on a 4-byte buffer should fail");

    Ok(())
}

// ── Binance: ServerTimeResponse (template 102) ────────────────────────────

#[test]
fn binance_server_time_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        ServerTimeResponseDecoder, ServerTimeResponseEncoder, ServerTimeResponseFixedFields,
    };

    let expected_ts: i64 = 1712345678000123;

    // Encode — ServerTimeResponseEncoder has no type-state, just plain methods
    let mut buf = [0u8; ServerTimeResponseEncoder::ENCODED_LENGTH];
    let encoder = ServerTimeResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&ServerTimeResponseFixedFields {
            server_time: expected_ts,
        });
    let encoded = encoder.as_bytes_with_header();

    // Decode
    let decoder = ServerTimeResponseDecoder::try_from(encoded)
        .expect("ServerTimeResponseDecoder::try_from should succeed");
    assert_eq!(decoder.server_time(), expected_ts, "server_time");

    Ok(())
}

#[test]
fn binance_server_time_verify_passes() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        ServerTimeResponseDecoder, ServerTimeResponseEncoder, ServerTimeResponseFixedFields,
    };

    let mut buf = [0u8; ServerTimeResponseEncoder::ENCODED_LENGTH];
    let encoder = ServerTimeResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&ServerTimeResponseFixedFields { server_time: 42 });
    let encoded = encoder.as_bytes_with_header();

    assert!(ServerTimeResponseDecoder::verify(encoded).is_ok());

    Ok(())
}

#[test]
fn binance_server_time_buffer_too_short() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::ServerTimeResponseDecoder;

    let result = ServerTimeResponseDecoder::try_from(&[0u8; 4][..]);
    assert!(result.is_err(), "decoding from a 4-byte buffer should fail");

    Ok(())
}

// ── Bitget: Trade (template 1003) — group + var-data message ──────────────

#[test]
fn bitget_trade_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::{
        InstCategory, Padding5, Padding7, TradeDecoder, TradeEncoder, TradeFixedFields, TradeSide,
    };

    let trades_count = 2u16;
    let symbol = b"ETHUSDT";
    let buf_len = TradeEncoder::compute_encoded_length(trades_count as usize, symbol.len()) + 8;
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    // Encode
    let after_trades = TradeEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&TradeFixedFields {
            price_exponent: -5i8,
            size_exponent: -3i8,
            sts: 42u64,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .trades(trades_count, |group| {
            group
                .add(|entry| {
                    entry
                        .ts(1000u64)
                        .exec_id(1u64)
                        .price(10000i64)
                        .size(10i64)
                        .side(TradeSide::Buy)
                        .padding(Padding7([0u8; 7]));
                    Ok(())
                })
                .expect("trade entry 0 should succeed");
            group
                .add(|entry| {
                    entry
                        .ts(2000u64)
                        .exec_id(2u64)
                        .price(20000i64)
                        .size(20i64)
                        .side(TradeSide::Sell)
                        .padding(Padding7([0u8; 7]));
                    Ok(())
                })
                .expect("trade entry 1 should succeed");
            Ok(())
        })
        .expect("trades encoding should succeed");
    let complete = after_trades
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    // Decode
    let decoder = TradeDecoder::try_from(encoded).expect("TradeDecoder::try_from should succeed");

    // Verify scalar fields (fixed block — always available on the body view)
    assert_eq!(decoder.price_exponent(), -5, "price_exponent");
    assert_eq!(decoder.size_exponent(), -3, "size_exponent");
    assert_eq!(decoder.sts(), 42, "sts");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");

    // Consume into trades group (consuming stage transition)
    let mut trades = decoder.into_trades().expect("trades group decode");
    let mut entries = Vec::new();
    while let Some(entry) = trades.next() {
        entries.push(entry);
    }
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

    // Advance past the trades group to access the trailing var-data symbol
    let after_trades = trades.finish().expect("finish trades");
    let (symbol_bytes, _complete) = after_trades.into_symbol().expect("symbol");
    assert_eq!(
        core::str::from_utf8(symbol_bytes).expect("symbol_as_str"),
        "ETHUSDT",
        "symbol"
    );

    Ok(())
}

#[test]
fn bitget_trade_max_uint64() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::{
        InstCategory, Padding5, Padding7, TradeDecoder, TradeEncoder, TradeFixedFields, TradeSide,
    };

    let sts_max = u64::MAX;
    let trades_count = 1u16;
    let symbol = b"BTCUSDT";
    let buf_len = TradeEncoder::compute_encoded_length(trades_count as usize, symbol.len()) + 8;
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    let after_trades = TradeEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&TradeFixedFields {
            price_exponent: -8i8,
            size_exponent: -2i8,
            sts: sts_max,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .trades(trades_count, |group| {
            group
                .add(|entry| {
                    entry
                        .ts(1u64)
                        .exec_id(1u64)
                        .price(1i64)
                        .size(1i64)
                        .side(TradeSide::Buy)
                        .padding(Padding7([0u8; 7]));
                    Ok(())
                })
                .expect("trade entry should succeed");
            Ok(())
        })
        .expect("trades encoding should succeed");
    let complete = after_trades
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    let decoder = TradeDecoder::try_from(encoded).expect("TradeDecoder::try_from should succeed");

    assert_eq!(decoder.sts(), sts_max, "sts should be u64::MAX");

    Ok(())
}

#[test]
fn bitget_trade_zero_values() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::bitget_spot::{
        InstCategory, Padding5, Padding7, TradeDecoder, TradeEncoder, TradeFixedFields, TradeSide,
    };

    let trades_count = 1u16;
    let symbol = b"";
    let buf_len = TradeEncoder::compute_encoded_length(trades_count as usize, symbol.len()) + 8;
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    let after_trades = TradeEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&TradeFixedFields {
            price_exponent: 0i8,
            size_exponent: 0i8,
            sts: 0u64,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .trades(trades_count, |group| {
            group
                .add(|entry| {
                    entry
                        .ts(0u64)
                        .exec_id(0u64)
                        .price(0i64)
                        .size(0i64)
                        .side(TradeSide::Buy)
                        .padding(Padding7([0u8; 7]));
                    Ok(())
                })
                .expect("trade entry should succeed");
            Ok(())
        })
        .expect("trades encoding should succeed");
    let complete = after_trades
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    let decoder = TradeDecoder::try_from(encoded).expect("TradeDecoder::try_from should succeed");

    // Scalars from the fixed block
    assert_eq!(decoder.price_exponent(), 0, "price_exponent");
    assert_eq!(decoder.size_exponent(), 0, "size_exponent");
    assert_eq!(decoder.sts(), 0, "sts");
    assert_eq!(decoder.category(), InstCategory::Spot, "category");

    // Consuming stages: trades group before var-data (wire order)
    let trades = decoder.into_trades().expect("trades group decode");
    let entry = trades.entry_at(0).expect("first trade entry");
    assert_eq!(entry.ts(), 0, "entry ts");
    assert_eq!(entry.exec_id(), 0, "entry exec_id");
    assert_eq!(entry.price(), 0, "entry price");
    assert_eq!(entry.size(), 0, "entry size");
    assert_eq!(entry.side(), TradeSide::Buy, "entry side");
    let after_trades = trades.finish().expect("finish trades");
    let (symbol_bytes, _complete) = after_trades.into_symbol().expect("symbol");
    assert_eq!(symbol_bytes, b"", "empty symbol");

    Ok(())
}

// ── Binance: WebSocketSessionLogonResponse (template 51) — multiple scalars + var data ──

#[test]
fn binance_logon_response_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        BoolEnum, WebSocketSessionLogonResponseDecoder, WebSocketSessionLogonResponseEncoder,
        WebSocketSessionLogonResponseFixedFields,
    };

    let api_key = b"my-test-api-key";
    let buf_len = WebSocketSessionLogonResponseEncoder::compute_encoded_length_with_message_header(
        api_key.len(),
    );
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    // Encode
    let complete = WebSocketSessionLogonResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&WebSocketSessionLogonResponseFixedFields {
            authorized_since: 1712345678000000i64,
            connected_since: 1712345679000000i64,
            return_rate_limits: BoolEnum::True,
            server_time: 1712345680000000i64,
            user_data_stream: Some(BoolEnum::False),
        })
        .logged_on_api_key(api_key)
        .expect("logged_on_api_key encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    // Decode
    let decoder = WebSocketSessionLogonResponseDecoder::try_from(encoded)
        .expect("WebSocketSessionLogonResponseDecoder::try_from should succeed");

    assert_eq!(
        decoder.authorized_since(),
        1712345678000000_i64,
        "authorized_since"
    );
    assert_eq!(
        decoder.connected_since(),
        1712345679000000_i64,
        "connected_since"
    );
    assert_eq!(
        decoder.return_rate_limits(),
        BoolEnum::True,
        "return_rate_limits"
    );
    assert_eq!(decoder.server_time(), 1712345680000000_i64, "server_time");
    assert_eq!(
        decoder.user_data_stream(),
        BoolEnum::False,
        "user_data_stream"
    );
    let (api_key_bytes, _complete) = decoder.into_logged_on_api_key().expect("logged_on_api_key");
    assert_eq!(
        core::str::from_utf8(api_key_bytes).expect("logged_on_api_key_as_str"),
        "my-test-api-key",
        "logged_on_api_key"
    );

    Ok(())
}

// ── Binance: WebSocketResponse (template 50) — group + var-data message ───

#[test]
fn binance_websocket_response_group_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        BoolEnum, RateLimitInterval, RateLimitType, WebSocketResponseDecoder,
        WebSocketResponseEncoder, WebSocketResponseFixedFields,
    };

    let rate_limits_count = 2u16;
    let id = b"test-id-1";
    let result = b"{\"data\":\"ok\"}";
    let buf_len = WebSocketResponseEncoder::compute_encoded_length(
        rate_limits_count as usize,
        id.len(),
        result.len(),
    ) + 8;
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    // Encode
    let after_rate_limits = WebSocketResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&WebSocketResponseFixedFields {
            sbe_schema_id_version_deprecated: BoolEnum::False,
            status: 200u16,
        })
        .rate_limits(rate_limits_count, |group| {
            group
                .add(|entry| {
                    entry
                        .rate_limit_type(RateLimitType::RequestWeight)
                        .interval(RateLimitInterval::Minute)
                        .interval_num(1u8)
                        .rate_limit(1200i64)
                        .current(50i64);
                    Ok(())
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
                    Ok(())
                })
                .expect("rate limit entry 1 should succeed");
            Ok(())
        })
        .expect("rate_limits encoding should succeed");
    let after_id = after_rate_limits
        .id(id)
        .expect("id encoding should succeed");
    let complete = after_id
        .result(result)
        .expect("result encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    // Decode
    let decoder = WebSocketResponseDecoder::try_from(encoded)
        .expect("WebSocketResponseDecoder::try_from should succeed");

    assert_eq!(
        decoder.sbe_schema_id_version_deprecated(),
        BoolEnum::False,
        "sbe_schema_id_version_deprecated"
    );
    assert_eq!(decoder.status(), 200, "status");

    // Consume into rate_limits group (wire-order enforced)
    let mut rate_limits = decoder
        .into_rate_limits()
        .expect("rate_limits group decode");
    let mut entries = Vec::new();
    while let Some(entry) = rate_limits.next() {
        entries.push(entry);
    }
    assert_eq!(entries.len(), 2, "should have 2 rate limit entries");

    assert_eq!(
        entries[0].rate_limit_type(),
        RateLimitType::RequestWeight,
        "entry 0 type"
    );
    assert_eq!(
        entries[0].interval(),
        RateLimitInterval::Minute,
        "entry 0 interval"
    );
    assert_eq!(entries[0].interval_num(), 1, "entry 0 interval_num");
    assert_eq!(entries[0].rate_limit(), 1200, "entry 0 rate_limit");
    assert_eq!(entries[0].current(), 50, "entry 0 current");

    assert_eq!(
        entries[1].rate_limit_type(),
        RateLimitType::Orders,
        "entry 1 type"
    );
    assert_eq!(
        entries[1].interval(),
        RateLimitInterval::Second,
        "entry 1 interval"
    );
    assert_eq!(entries[1].interval_num(), 10, "entry 1 interval_num");
    assert_eq!(entries[1].rate_limit(), 100, "entry 1 rate_limit");
    assert_eq!(entries[1].current(), 0, "entry 1 current");

    // Advance past rate_limits to access trailing var-data fields
    let after_rates = rate_limits.finish().expect("finish rate_limits");
    let (id_bytes, after_id) = after_rates.into_id().expect("id");
    assert_eq!(
        core::str::from_utf8(id_bytes).expect("id_as_str"),
        "test-id-1",
        "id"
    );
    let (result_bytes, _complete) = after_id.into_result().expect("result");
    assert_eq!(
        core::str::from_utf8(result_bytes).expect("result_as_str"),
        "{\"data\":\"ok\"}",
        "result"
    );

    Ok(())
}

#[test]
fn binance_websocket_response_group_buffer_too_short() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::WebSocketResponseDecoder;

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
    let result = decoder.into_rate_limits();
    assert!(
        result.is_err(),
        "rate_limits() on buffer without group dim should fail"
    );

    Ok(())
}

// ── Wrong-schema detection: bitget schema bytes rejected by binance, and vice versa ──

#[test]
fn wrong_schema_bitget_encoded_rejected_by_binance() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::WebSocketResponseDecoder;
    use exchange_example::bitget_spot::{
        BestBidAskEncoder, BestBidAskFixedFields, InstCategory, Padding5,
    };

    // Encode a valid bitget BestBidAsk message
    let symbol = b"BTCUSDT";
    let buf_len = BestBidAskEncoder::compute_encoded_length_with_message_header(symbol.len());
    let mut buf_storage = [0u8; 8192];
    assert!(buf_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..buf_len];

    let complete = BestBidAskEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&BestBidAskFixedFields {
            ts: 1u64,
            bid1_price: 2i64,
            bid1_size: 3i64,
            ask1_price: 4i64,
            ask1_size: 5i64,
            price_exponent: -8i8,
            size_exponent: -2i8,
            seq: 10u64,
            sts: 20u64,
            category: InstCategory::Spot,
            padding: Padding5([0u8; 5]),
        })
        .symbol(symbol)
        .expect("symbol encoding should succeed");
    let encoded = complete.as_bytes_with_header();

    // Try to decode as binance WebSocketResponse — should fail with WrongSchema
    let result = WebSocketResponseDecoder::try_from(encoded);
    assert!(
        result.is_err(),
        "bitget-encoded data rejected by binance decoder: schema_id differs"
    );

    Ok(())
}

// ── Type-inventory tests: catch codegen regressions that silently drop message types ──

#[test]
fn bitget_type_inventory() -> Result<(), Box<dyn std::error::Error>> {
    // BestBidAskDecoder (template 1002) — verify SCHEMA_ID, SCHEMA_VERSION, TEMPLATE_ID
    assert_eq!(
        exchange_example::bitget_spot::BestBidAskDecoder::SCHEMA_ID,
        1,
        "BestBidAskDecoder::SCHEMA_ID"
    );
    assert_eq!(
        exchange_example::bitget_spot::BestBidAskDecoder::SCHEMA_VERSION,
        3,
        "BestBidAskDecoder::SCHEMA_VERSION"
    );
    assert_eq!(
        exchange_example::bitget_spot::BestBidAskDecoder::TEMPLATE_ID,
        1002,
        "BestBidAskDecoder::TEMPLATE_ID"
    );

    // Depth50Decoder (template 1001)
    assert_eq!(
        exchange_example::bitget_spot::Depth50Decoder::SCHEMA_ID,
        1,
        "Depth50Decoder::SCHEMA_ID"
    );
    assert_eq!(
        exchange_example::bitget_spot::Depth50Decoder::TEMPLATE_ID,
        1001,
        "Depth50Decoder::TEMPLATE_ID"
    );

    // TradeDecoder (template 1003)
    assert_eq!(
        exchange_example::bitget_spot::TradeDecoder::SCHEMA_ID,
        1,
        "TradeDecoder::SCHEMA_ID"
    );
    assert_eq!(
        exchange_example::bitget_spot::TradeDecoder::TEMPLATE_ID,
        1003,
        "TradeDecoder::TEMPLATE_ID"
    );

    // Free function schema_id_from_header
    let header = exchange_example::bitget_spot::MessageHeader([32, 0, 233, 3, 1, 0, 3, 0]); // Depth50 header
    assert_eq!(
        exchange_example::bitget_spot::schema_id_from_header(&header.0),
        Some(1),
        "schema_id_from_header should return SCHEMA_ID for valid header"
    );
    assert_eq!(
        exchange_example::bitget_spot::schema_id_from_header(&[0u8; 8]),
        Some(0),
        "schema_id_from_header on all-zeros header"
    );
    assert_eq!(
        exchange_example::bitget_spot::schema_id_from_header(&[0u8; 4]),
        None,
        "schema_id_from_header on too-short buffer"
    );

    // Module-level constants
    assert_eq!(
        exchange_example::bitget_spot::SEMANTIC_VERSION,
        "1.0.0",
        "bitget SCHEMA_SEMANTIC_VERSION"
    );

    Ok(())
}

#[test]
fn binance_type_inventory() -> Result<(), Box<dyn std::error::Error>> {
    // WebSocketSessionLogonResponseDecoder
    assert_eq!(
        exchange_example::binance_spot::WebSocketSessionLogonResponseDecoder::SCHEMA_ID,
        3,
        "WebSocketSessionLogonResponseDecoder::SCHEMA_ID"
    );
    assert_eq!(
        exchange_example::binance_spot::WebSocketSessionLogonResponseDecoder::TEMPLATE_ID,
        51,
        "WebSocketSessionLogonResponseDecoder::TEMPLATE_ID"
    );

    // WebSocketResponseDecoder (template 50)
    assert_eq!(
        exchange_example::binance_spot::WebSocketResponseDecoder::SCHEMA_ID,
        3,
        "WebSocketResponseDecoder::SCHEMA_ID"
    );
    assert_eq!(
        exchange_example::binance_spot::WebSocketResponseDecoder::TEMPLATE_ID,
        50,
        "WebSocketResponseDecoder::TEMPLATE_ID"
    );

    // ServerTimeResponseDecoder (template 102)
    assert_eq!(
        exchange_example::binance_spot::ServerTimeResponseDecoder::SCHEMA_ID,
        3,
        "ServerTimeResponseDecoder::SCHEMA_ID"
    );
    assert_eq!(
        exchange_example::binance_spot::ServerTimeResponseDecoder::TEMPLATE_ID,
        102,
        "ServerTimeResponseDecoder::TEMPLATE_ID"
    );

    // Free function schema_id_from_header
    let header = exchange_example::binance_spot::MessageHeader([0, 0, 102, 0, 3, 0, 5, 0]); // ServerTimeResponse header
    assert_eq!(
        exchange_example::binance_spot::schema_id_from_header(&header.0),
        Some(3),
        "schema_id_from_header should return SCHEMA_ID for valid header"
    );
    assert_eq!(
        exchange_example::binance_spot::schema_id_from_header(&[0u8; 4]),
        None,
        "schema_id_from_header on too-short buffer"
    );

    // Module-level constants
    assert_eq!(
        exchange_example::binance_spot::SEMANTIC_VERSION,
        "5.2",
        "binance SEMANTIC_VERSION"
    );

    Ok(())
}

#[test]
fn wrong_schema_binance_encoded_rejected_by_bitget() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        ServerTimeResponseEncoder, ServerTimeResponseFixedFields,
    };
    use exchange_example::bitget_spot::BestBidAskDecoder;

    // Encode a valid binance ServerTimeResponse message
    let mut buf = [0u8; ServerTimeResponseEncoder::ENCODED_LENGTH];
    let encoder = ServerTimeResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&ServerTimeResponseFixedFields { server_time: 42 });
    let encoded = encoder.as_bytes_with_header();

    // Try to decode as bitget BestBidAsk — should fail with WrongSchema
    let result = BestBidAskDecoder::try_from(encoded);
    assert!(
        result.is_err(),
        "binance-encoded data rejected by bitget decoder: schema_id differs"
    );

    Ok(())
}

// ── AppMessage / L2Book / Trade roundtrip ─────────────────────────────

#[test]
fn app_message_l2book_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::normalized_app::{
        AnyMessage, AppMessageDecoder, AppMessageEncoder, AppMessageFixedFields, Decimal,
        L2BookEncoder, L2BookFixedFields, Source, sbe_rt,
    };

    let symbol = b"BTCUSDT";
    let bids_count: u16 = 2;
    let asks_count: u16 = 1;

    // Encode L2Book (inner payload)
    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids_count as usize,
        asks_count as usize,
        symbol.len(),
    );
    let app_name = b"bitget";
    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(app_name.len(), inner_len);

    let mut buf_storage = [0u8; 8192];
    assert!(outer_len <= buf_storage.len(), "len exceeds stack pad");
    let mut buf = &mut buf_storage[..outer_len];
    let complete = AppMessageEncoder::try_wrap_and_apply_header(&mut buf, 0)
        .unwrap()
        .fixed(&AppMessageFixedFields {
            sent_ts: 1_700_000_000_000_000_000,
        })
        .app_name(app_name)
        .unwrap()
        .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
            let book = L2BookEncoder::try_wrap_and_apply_header(payload, 0)
                .unwrap()
                .fixed(&L2BookFixedFields {
                    source: Source::Bitget,
                    exchange_timestamp: 1_700_000_000_000_000_001,
                    receive_timestamp: 1_700_000_000_000_000_002,
                    sequence: 42,
                })
                .bids(bids_count, |g| {
                    g.add(|e| {
                        e.price_wire(Decimal::new(50000_00, -2));
                        e.size_wire(Decimal::new(1_50, -2));
                        Ok(())
                    });
                    g.add(|e| {
                        e.price_wire(Decimal::new(49900_00, -2));
                        e.size_wire(Decimal::new(2_00, -2));
                        Ok(())
                    });
                    Ok(())
                })
                .unwrap();
            let book = book
                .asks(asks_count, |g| {
                    g.add(|e| {
                        e.price_wire(Decimal::new(50100_00, -2));
                        e.size_wire(Decimal::new(0_50, -2));
                        Ok(())
                    });
                    Ok(())
                })
                .unwrap();
            let book = book.symbol(symbol).unwrap();
            let _ = book.as_bytes_with_header();
            Ok(())
        })
        .unwrap();
    assert_eq!(complete.as_bytes_with_header().len(), outer_len);

    // Decode outer -> inner
    let outer_dec = AppMessageDecoder::try_decode(&buf, 0).unwrap();
    assert_eq!(outer_dec.sent_ts(), 1_700_000_000_000_000_000);
    let (name, after_name) = outer_dec.into_app_name().unwrap();
    assert_eq!(name, app_name);
    let (frame, _complete) = after_name.into_payload_as_message().unwrap();
    match frame.message {
        AnyMessage::L2Book(book) => {
            assert_eq!(book.source(), Source::Bitget);
            assert_eq!(book.sequence(), 42);
            // Verify bids
            let bids = book.into_bids().unwrap();
            assert_eq!(bids.len(), 2);
            let mut iter = bids.into_iter();
            let b0 = iter.next().unwrap();
            assert_eq!(b0.price_wire().mantissa(), 50000_00);
            assert_eq!(b0.price_wire().exponent(), -2);
            let b1 = iter.next().unwrap();
            assert_eq!(b1.price_wire().mantissa(), 49900_00);
        }
        _ => panic!("expected L2Book"),
    }
    Ok(())
}
