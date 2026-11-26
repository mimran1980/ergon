//! Synthetic frames for the `write_json` paths no recorded response hits:
//! `jsonDefaultValue` filling an absent optional, and a negative
//! `mantissa128`. The expected JSON is derived from the schema's marks, not
//! recorded from the venue.

use exchange_example::binance_spot::{
    AveragePriceResponseEncoder, AveragePriceResponseFixedFields, KlinesResponseEncoder,
    KlinesResponseFixedFields,
};
use exchange_example::sbe_json::render;

/// `price` (`mbx:jsonDefaultValue="0"`, scaled by `priceExponent`) and
/// `closeTime` (`mbx:jsonDefaultValue="-1"`) are null on the wire.
#[test]
fn absent_optionals_render_their_json_default() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 64];
    let done = AveragePriceResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)?.fixed(
        &AveragePriceResponseFixedFields {
            mins: 5,
            price_exponent: -8,
            price: None,
            close_time: None,
        },
    );
    let rendered = render(done.as_bytes_with_header())?;
    let ours: serde_json::Value = serde_json::from_str(&rendered)?;
    let expected = serde_json::json!({"mins": 5, "price": "0.00000000", "closeTime": -1});
    assert_eq!(ours, expected, "{rendered}");
    Ok(())
}

/// Every `mantissa128` column of a kline row, negative, keeps its sign and
/// the digits its exponent implies.
#[test]
fn negative_mantissa128_renders_signed() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 256];
    let done = KlinesResponseEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&KlinesResponseFixedFields {
            price_exponent: -2,
            qty_exponent: -3,
        })
        .klines(1, |g| {
            g.add(|e| {
                e.open_time(1)
                    .open_price(100)
                    .high_price(200)
                    .low_price(50)
                    .close_price(150)
                    .volume((-1_234_i128).to_le_bytes())
                    .close_time(2)
                    .quote_volume((-5_i128).to_le_bytes())
                    .num_trades(3)
                    .taker_buy_base_volume(
                        (-170_141_183_460_469_231_731_687_303_715_884_105_728_i128).to_le_bytes(),
                    )
                    .taker_buy_quote_volume((-99_i128).to_le_bytes());
                Ok(())
            })?;
            Ok(())
        })?;
    let rendered = render(done.as_bytes_with_header())?;
    let ours: serde_json::Value = serde_json::from_str(&rendered)?;
    let expected = serde_json::json!([[
        1,
        "1.00",
        "2.00",
        "0.50",
        "1.50",
        "-1.234",
        2,
        "-0.05",
        3,
        "-170141183460469231731687303715884105.728",
        "-0.99",
        "0"
    ]]);
    assert_eq!(ours, expected, "{rendered}");
    Ok(())
}
