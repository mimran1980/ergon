//! The three extension knobs wired in `build.rs`, asserted against the
//! published venue schema's own marks: `mbx:jsonValue` labels, the
//! `mbx:exponent` pair on `MaxAssetFilter`, and `mbx:jsonDefaultValue`
//! on the filter messages.

/// `with_variant_labels("jsonValue", "as_str")` replaces the built-in
/// `as_str` on every enum the schema labels, so the label comes from the
/// fixture's `mbx:jsonValue` value, not the Rust schema name.
#[test]
fn variant_labels_come_from_the_fixture_marks() {
    use exchange_example::binance_spot::{AccountType, FilterType};

    assert_eq!(FilterType::MaxAsset.as_str(), "MAX_ASSET");
    assert_eq!(AccountType::Spot.as_str(), "SPOT");
    assert_eq!(AccountType::Unknown.as_str(), "UNKNOWN");
    // The knob's own unknown arm — distinct from the hook's wildcard.
    assert_eq!(AccountType::NullVal.as_str(), "NullVal");
}

/// `with_exponent_links("exponent")` on a real fixture message: encode a
/// `MaxAssetFilter`, decode it, and read the pair helper — the tuple is
/// exactly what the two accessors return.
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

/// `with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull")` puts
/// the fixture's own strings on the message's `{Name}Schema` unit.
#[test]
fn schema_defaults_come_from_the_fixture_marks() {
    use exchange_example::binance_spot::TPlusSellFilterSchema;

    assert_eq!(
        TPlusSellFilterSchema::END_TIME_SCHEMA_DEFAULT,
        Some("-1"),
        "mbx:jsonDefaultValue on endTime, verbatim"
    );
}
