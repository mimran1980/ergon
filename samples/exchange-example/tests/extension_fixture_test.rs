//! The extension marks of the published venue schema, served through the
//! generic API wired in `build.rs`: `json_label_hook` labels, the `pairs`
//! recipe's mantissa/exponent helpers, and `with_extension_consts` for every
//! other mark (`mbx:jsonPath`, `mbx:jsonDefaultValue`, `mbx:jsonOmitNull`).

/// Labels come from the fixture's `mbx:jsonValue` marks through `json_label()`
/// and the consts; the built-in `as_str` keeps the Rust variant name.
#[test]
fn labels_come_from_the_fixture_marks() {
    use exchange_example::binance_spot::{AccountType, FilterType};

    assert_eq!(FilterType::MaxAsset.json_label(), "MAX_ASSET");
    assert_eq!(AccountType::Spot.json_label(), "SPOT");
    assert_eq!(AccountType::Unknown.json_label(), "UNKNOWN");
    assert_eq!(AccountType::SPOT_JSON_VALUE, "SPOT");
    assert_eq!(FilterType::MAX_ASSET_JSON_VALUE, "MAX_ASSET");
    assert_eq!(AccountType::Spot.as_str(), "Spot");
    assert_eq!(AccountType::NullVal.as_str(), "NullVal");
}

/// The pair helper on a real fixture message: encode a `MaxAssetFilter`,
/// decode it, and read the pair; the tuple is exactly what the two
/// accessors return.
#[test]
fn exponent_pair_roundtrips_on_max_asset_filter() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        MaxAssetFilterDecoder, MaxAssetFilterEncoder, MaxAssetFilterFixedFields,
    };

    let buf_len = MaxAssetFilterEncoder::compute_encoded_length_with_message_header(7);
    let mut buf = vec![0u8; buf_len];
    let complete = MaxAssetFilterEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&MaxAssetFilterFixedFields {
            qty_exponent: -3,
            max_qty: 5_000_000,
        })
        .asset(b"BTCUSDT")?;
    let dec = MaxAssetFilterDecoder::try_from(complete.as_bytes_with_header())?;

    let (mantissa, exponent) = dec.max_qty_with_exponent();
    assert_eq!(
        (mantissa, exponent),
        (5_000_000, -3),
        "fixture pair roundtrip"
    );
    assert_eq!(dec.qty_exponent(), -3, "accessor agrees with the pair");
    assert_eq!(dec.max_qty(), 5_000_000, "accessor agrees with the pair");
    Ok(())
}

/// A group mantissa whose exponent is declared on the message (the venue's
/// dominant shape): the helper takes the message decoder.
#[test]
fn group_mantissa_pairs_with_the_message_exponent() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::binance_spot::{
        DepthResponseDecoder, DepthResponseEncoder, DepthResponseFixedFields,
    };

    let mut buf = vec![0u8; 256];
    let done = DepthResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&DepthResponseFixedFields {
            last_update_id: 42,
            price_exponent: -8,
            qty_exponent: -5,
        })
        .bids(1, |g| {
            g.add(|e| {
                e.price(914_800).qty(221_900);
                Ok(())
            })?;
            Ok(())
        })?
        .asks(0, |_| Ok(()))?;
    let dec = DepthResponseDecoder::try_from(done.as_bytes_with_header())?;
    let bid = dec.bids()?.next().ok_or("one bid")?;
    assert_eq!(bid.price_with_exponent(&dec), (914_800, -8));
    assert_eq!(bid.qty_with_exponent(&dec), (221_900, -5));
    Ok(())
}

/// Every other mark is a const with the schema's own string, including the
/// `jsonPath` marks and the group-level `jsonOmitNull`.
#[test]
fn marks_are_consts_next_to_their_items() {
    use exchange_example::binance_spot::{
        AveragePriceResponseSchema, ExchangeInfoResponseSchema, MaxAssetFilterSchema,
        TPlusSellFilterSchema,
    };

    assert_eq!(TPlusSellFilterSchema::END_TIME_JSON_DEFAULT_VALUE, "-1");
    assert_eq!(MaxAssetFilterSchema::MAX_QTY_JSON_PATH, "limit");
    assert_eq!(MaxAssetFilterSchema::MAX_QTY_EXPONENT, "qtyExponent");
    assert_eq!(ExchangeInfoResponseSchema::SORS_JSON_OMIT_NULL, "true");
    assert_eq!(
        AveragePriceResponseSchema::CLOSE_TIME_JSON_DEFAULT_VALUE,
        "-1"
    );
}
