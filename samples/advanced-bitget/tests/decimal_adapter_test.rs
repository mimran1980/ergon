//! Exact Decimal adapter matrix at the sample level (Task 5): mixed
//! exponents including 15-decimal-place baby-token values, negatives, zero,
//! i64 boundaries, malformed text, and exact round trips through the
//! generated generic methods with the `rust_decimal` adapter.

use std::str::FromStr;

use advanced_bitget::decimal::parse_decimal_exact;
use advanced_bitget::normalized_app::{
    AppMessageEncoder, Decimal, L2BookDecoder, L2BookEncoder, SbeDecimal, Source, sbe_rt,
};

#[test]
fn parse_decimal_exact_matrix() {
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
        assert_eq!(parse_decimal_exact(input), Ok((m, e)), "input {input}");
    }

    // Malformed and out-of-range inputs are rejected, never zeroed.
    for bad in ["", "abc", "1.2.3", "9223372036854775808", "1e5000"] {
        assert!(parse_decimal_exact(bad).is_err(), "must reject {bad:?}");
    }
}

#[test]
fn rust_decimal_generic_roundtrip_through_generated_methods() {
    let values = ["50000.5", "0.000000000000015", "-42", "0", "0.25"];
    for text in values {
        let d = rust_decimal::Decimal::from_str(text).unwrap();

        let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(1, 0, 1);
        let mut buf = vec![0u8; inner_len];
        let mut enc = L2BookEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc
            .source(Source::Bitget)
            .exchange_timestamp(1)
            .receive_timestamp(2)
            .sequence(3);
        let after = enc
            .bids(1, |g| {
                let _ = g.add(|e| {
                    e.price::<rust_decimal::Decimal>(d).unwrap();
                    let _ = e.size_wire(Decimal::new(1, 0));
                });
            })
            .unwrap();
        let complete = after.asks(0, |_| {}).unwrap().symbol(b"X").unwrap();
        let bytes = complete.as_bytes_with_header().to_vec();

        // Generic decode returns the exact same rust_decimal value.
        let dec = L2BookDecoder::wrap_and_apply_header(&bytes, 0).unwrap();
        let mut g = dec.into_bids().unwrap();
        let entry = g.next().unwrap();
        let back: rust_decimal::Decimal = entry.price().unwrap();
        assert_eq!(back, d, "round trip for {text}");

        // Byte equivalence with the raw wire model.
        let (m, e) = d.try_into_sbe().unwrap();
        let mut buf2 = vec![0u8; inner_len];
        let mut enc = L2BookEncoder::wrap_and_apply_header(&mut buf2, 0).unwrap();
        let _ = enc
            .source(Source::Bitget)
            .exchange_timestamp(1)
            .receive_timestamp(2)
            .sequence(3);
        let after = enc
            .bids(1, |g| {
                let _ = g.add(|entry| {
                    let _ = entry
                        .price_wire(Decimal::new(m, e))
                        .size_wire(Decimal::new(1, 0));
                });
            })
            .unwrap();
        let complete = after.asks(0, |_| {}).unwrap().symbol(b"X").unwrap();
        assert_eq!(
            complete.as_bytes_with_header(),
            &bytes[..],
            "generic and wire encodes must be byte-identical for {text}"
        );
    }
    // Silence unused import when AppMessageEncoder isn't referenced above.
    let _ = AppMessageEncoder::compute_encoded_length_with_message_header(1, 1);
    let _ = std::any::type_name::<sbe_rt::EncodeError>();
}
