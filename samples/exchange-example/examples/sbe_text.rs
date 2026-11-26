//! `sbe_text`: decode a filter message and print readable field text —
//! the extension-XML use case. Labels come from `build.rs`'s
//! `json_label_hook` (`jsonValue` → `json_label()`), the quantity renders
//! as an exact decimal through the `pairs` recipe's `max_qty_with_exponent`
//! (integer arithmetic only — no floats, no decimal crate). Self-contained:
//! it encodes one message first, then decodes it.
//!
//! Run: `cargo run --example sbe_text`

use exchange_example::binance_spot::{
    MaxAssetFilterDecoder, MaxAssetFilterEncoder, MaxAssetFilterFixedFields,
};
use exchange_example::market::WireDec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Encode one MaxAssetFilter: qtyExponent = -3, maxQty = 5 000 000.
    let buf_len = MaxAssetFilterEncoder::compute_encoded_length_with_message_header(7);
    let mut buf = vec![0u8; buf_len];
    let complete = MaxAssetFilterEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&MaxAssetFilterFixedFields {
            qty_exponent: -3,
            max_qty: 5_000_000,
        })
        .asset(b"BTCUSDT")?;

    // Decode it back and print each field as text.
    let dec = MaxAssetFilterDecoder::try_from(complete.as_bytes_with_header())?;
    let (mantissa, exponent) = dec.max_qty_with_exponent();

    // `filterType` is a constant enum field; its label is the schema's
    // `mbx:jsonValue`, exposed through the hook-generated `json_label`.
    println!("filter type : {}", dec.filter_type().json_label());
    // The pair helper returns exactly what the two accessors return;
    // WireDec renders `mantissa × 10^exponent` losslessly as text.
    println!(
        "max qty     : {} (mantissa {mantissa}, exponent {exponent})",
        WireDec::new(mantissa, exponent)
    );
    println!("asset       : {}", dec.asset_as_str()?);
    Ok(())
}
