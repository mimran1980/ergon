//! Exact Decimal adapter matrix at the sample level (Task 5): mixed
//! exponents including 15-decimal-place baby-token values, negatives, zero,
//! i64 boundaries, malformed text, and exact round trips through the
//! generated generic methods with the `rust_decimal` adapter.

use std::str::FromStr;

use exchange_example::decimal::parse_decimal_exact;
use exchange_example::normalized_app::{
    AppMessageEncoder, Decimal, L2BookDecoder, L2BookEncoder, Source, TryFromSbe, TryToSbe, sbe_rt,
};

#[test]
fn parse_decimal_exact_matrix() -> Result<(), Box<dyn std::error::Error>> {
    // (input, mantissa, exponent)
    let cases: &[(&str, i64, i8)] = &[
        ("0", 0, 0),
        ("1", 1, 0),
        ("-1", -1, 0),
        ("50000.5", 500005, -1),
        ("-50000.5", -500005, -1),
        // Baby token with 15 decimal places.
        ("0.000000000000015", 15, -15),
        ("-0.000000000000015", -15, -15),
        ("0.000123450000000", 123450000000, -15),
        // i64 boundaries.
        ("9223372036854775807", i64::MAX, 0),
        ("-9223372036854775808", i64::MIN, 0),
        ("92233720368.54775807", i64::MAX, -8),
    ];
    for &(input, m, e) in cases {
        let wd = parse_decimal_exact(input).unwrap();
        assert_eq!((wd.mantissa, wd.exponent), (m, e), "input {input}");
    }

    // Malformed and out-of-range inputs are rejected, never zeroed.
    for bad in ["", "abc", "1.2.3", "9223372036854775808", "1e5000"] {
        assert!(parse_decimal_exact(bad).is_err(), "must reject {bad:?}");
    }

    Ok(())
}

#[test]
fn rust_decimal_generic_roundtrip_through_generated_methods()
-> Result<(), Box<dyn std::error::Error>> {
    let values = ["50000.5", "0.000000000000015", "-42", "0", "0.25"];
    for text in values {
        let d = rust_decimal::Decimal::from_str(text).unwrap();

        let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(1, 0, 1);
        let mut buf_storage = [0u8; 8192];
        assert!(inner_len <= buf_storage.len(), "len exceeds stack pad");
        let buf = &mut buf_storage[..inner_len];
        let mut enc = L2BookEncoder::wrap_and_apply_header(buf, 0).unwrap();
        let _ = enc
            .source(Source::Bitget)
            .exchange_timestamp(1)
            .receive_timestamp(2)
            .sequence(3);
        let after = enc
            .bids(1, |g| {
                g.add(|e| {
                    e.price_from(&d).unwrap();
                    let _ = e.size_wire(Decimal::new(1, 0));
                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        let complete = after.asks(0, |_| Ok(())).unwrap().symbol(b"X").unwrap();
        let bytes = complete.as_bytes_with_header().to_vec();

        // Generic decode returns the exact same rust_decimal value.
        let dec = L2BookDecoder::decode(&bytes, 0).unwrap();
        let mut g = dec.into_bids().unwrap();
        let entry = g.next().unwrap();
        let back: rust_decimal::Decimal =
            rust_decimal::Decimal::try_from_sbe(entry.price_value()).unwrap();
        assert_eq!(back, d, "round trip for {text}");

        // Byte equivalence with the raw wire model.
        let wire = d.try_to_sbe().unwrap();
        let m = wire.mantissa();
        let e = wire.exponent();
        let mut buf2_storage = [0u8; 8192];
        assert!(inner_len <= buf2_storage.len(), "len exceeds stack pad");
        let buf2 = &mut buf2_storage[..inner_len];
        let mut enc = L2BookEncoder::wrap_and_apply_header(buf2, 0).unwrap();
        let _ = enc
            .source(Source::Bitget)
            .exchange_timestamp(1)
            .receive_timestamp(2)
            .sequence(3);
        let after = enc
            .bids(1, |g| {
                g.add(|entry| {
                    entry
                        .price_wire(Decimal::new(m, e))
                        .size_wire(Decimal::new(1, 0));
                    Ok(())
                })?;
                Ok(())
            })
            .unwrap();
        let complete = after.asks(0, |_| Ok(())).unwrap().symbol(b"X").unwrap();
        assert_eq!(
            complete.as_bytes_with_header(),
            &bytes[..],
            "generic and wire encodes must be byte-identical for {text}"
        );
    }
    // Silence unused import when AppMessageEncoder isn't referenced above.
    let _ = AppMessageEncoder::compute_encoded_length_with_message_header(1, 1);
    let _ = std::any::type_name::<sbe_rt::EncodeError>();
    Ok(())
}

#[test]
fn convert_error_display_and_wire_decimal_new() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::decimal::{DecimalConvertError, WireDecimal, to_clickhouse_decimal};

    assert_eq!(
        DecimalConvertError::Overflow.to_string(),
        "decimal overflow"
    );
    assert_eq!(
        DecimalConvertError::PrecisionLoss.to_string(),
        "decimal precision loss"
    );
    assert_eq!(
        DecimalConvertError::OutOfRange.to_string(),
        "decimal out of range"
    );

    let w = WireDecimal::new(5, -1);
    assert_eq!((w.mantissa, w.exponent), (5, -1));

    // ten_pow(>38) → Overflow; huge scaled value → OutOfRange.
    assert_eq!(
        to_clickhouse_decimal(1, 21),
        Err(DecimalConvertError::Overflow)
    );
    // 1.5e18 × 10^20 = 1.5e38: fits i128 but exceeds Decimal(38,18)'s range.
    assert_eq!(
        to_clickhouse_decimal(1_500_000_000_000_000_000, 2),
        Err(DecimalConvertError::OutOfRange)
    );

    Ok(())
}

#[test]
fn rust_decimal_adapter_positive_exponent_and_overflow() -> Result<(), Box<dyn std::error::Error>> {
    // Positive exponent scales up exactly.
    let d: rust_decimal::Decimal = TryFromSbe::<Decimal>::try_from_sbe(Decimal::new(5, 2)).unwrap();
    assert_eq!(d, rust_decimal::Decimal::from(500));
    let d: rust_decimal::Decimal =
        TryFromSbe::<Decimal>::try_from_sbe(Decimal::new(-5, 2)).unwrap();
    assert_eq!(d, rust_decimal::Decimal::from(-500));

    // Overflow: mantissa * 10^30 exceeds the adapter's exact range.
    let err =
        <rust_decimal::Decimal as TryFromSbe<Decimal>>::try_from_sbe(Decimal::new(i64::MAX, 30))
            .unwrap_err();
    assert_eq!(
        err,
        exchange_example::decimal::DecimalConvertError::Overflow
    );

    Ok(())
}
