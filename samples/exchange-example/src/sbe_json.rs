//! Render Binance spot SBE responses as the JSON its REST API returns.
//!
//! `build.rs`'s `json_hook` generates a `write_json` method on each covered
//! response decoder, group decoder and entry decoder from the schema's
//! `mbx:` marks; this module holds the value formatting those methods call
//! and the dispatch on the frame's template id. Timestamps stay in the SBE
//! unit (microseconds): compare with JSON fetched with
//! `X-MBX-TIME-UNIT: MICROSECOND`.

use crate::binance_spot::{self as spot, sbe_rt::DecodeError};

/// Why a frame could not be rendered.
#[derive(Debug)]
pub enum JsonError {
    /// The frame failed to decode.
    Decode(DecodeError),
    /// A string field is not UTF-8.
    Utf8,
    /// The frame is shorter than a message header.
    Short,
    /// No JSON rule for this template id.
    Template(u16),
}

impl From<DecodeError> for JsonError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

impl core::fmt::Display for JsonError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(e) => write!(f, "decode: {e}"),
            Self::Utf8 => f.write_str("string field is not UTF-8"),
            Self::Short => f.write_str("frame shorter than a message header"),
            Self::Template(id) => write!(f, "no JSON rule for template {id}"),
        }
    }
}

impl std::error::Error for JsonError {}

/// A mantissa the schema encodes: `mantissa64` (`i64`) or `mantissa128`
/// (16 little-endian bytes, signed).
pub trait Mantissa {
    /// The value as one signed integer.
    fn to_i128(self) -> i128;
}

impl Mantissa for i64 {
    fn to_i128(self) -> i128 {
        i128::from(self)
    }
}

impl Mantissa for [u8; 16] {
    fn to_i128(self) -> i128 {
        i128::from_le_bytes(self)
    }
}

/// `mantissa × 10^exponent` as a quoted exact decimal with `-exponent`
/// fractional digits, the venue's JSON form: `(914800, -8)` is `"0.00914800"`,
/// zero keeps its digits (`"0.00000000"`). Integer arithmetic only.
pub fn decimal(out: &mut String, mantissa: impl Mantissa, exponent: i8) {
    let m = mantissa.to_i128();
    out.push('"');
    if m < 0 {
        out.push('-');
    }
    let digits = m.unsigned_abs().to_string();
    if exponent >= 0 {
        out.push_str(&digits);
        out.extend(core::iter::repeat_n(
            '0',
            usize::from(exponent.unsigned_abs()),
        ));
    } else {
        let scale = usize::from(exponent.unsigned_abs());
        let padded = format!("{digits:0>width$}", width = scale + 1);
        let (int, frac) = padded.split_at(padded.len() - scale);
        out.push_str(int);
        out.push('.');
        out.push_str(frac);
    }
    out.push('"');
}

/// An integer as a JSON number.
pub fn number(out: &mut String, v: impl core::fmt::Display) {
    use core::fmt::Write as _;
    let _ = write!(out, "{v}");
}

/// UTF-8 bytes as a JSON string, escaped.
///
/// # Errors
///
/// [`JsonError::Utf8`] when the bytes are not UTF-8.
pub fn string(out: &mut String, bytes: &[u8]) -> Result<(), JsonError> {
    let s = core::str::from_utf8(bytes).map_err(|_| JsonError::Utf8)?;
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                use core::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(())
}

/// One SBE response frame (message header included) as JSON.
///
/// # Errors
///
/// [`JsonError`] when the frame does not decode or its template has no
/// JSON rule.
pub fn render(frame: &[u8]) -> Result<String, JsonError> {
    let id = frame
        .get(2..4)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(JsonError::Short)?;
    let mut out = String::new();
    match id {
        spot::ServerTimeResponseSchema::TEMPLATE_ID => {
            spot::ServerTimeResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::DepthResponseSchema::TEMPLATE_ID => {
            spot::DepthResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::TradesResponseSchema::TEMPLATE_ID => {
            spot::TradesResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::AggTradesResponseSchema::TEMPLATE_ID => {
            spot::AggTradesResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::KlinesResponseSchema::TEMPLATE_ID => {
            spot::KlinesResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::AveragePriceResponseSchema::TEMPLATE_ID => {
            spot::AveragePriceResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::PriceTickerSymbolResponseSchema::TEMPLATE_ID => {
            spot::PriceTickerSymbolResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::PriceTickerResponseSchema::TEMPLATE_ID => {
            spot::PriceTickerResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::BookTickerSymbolResponseSchema::TEMPLATE_ID => {
            spot::BookTickerSymbolResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        spot::BookTickerResponseSchema::TEMPLATE_ID => {
            spot::BookTickerResponseDecoder::try_from(frame)?.write_json(&mut out)?;
        }
        other => return Err(JsonError::Template(other)),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::decimal;

    fn dec(m: i64, e: i8) -> String {
        let mut s = String::new();
        decimal(&mut s, m, e);
        s
    }

    #[test]
    fn decimals_keep_the_exponent_digits() {
        assert_eq!(dec(914_800, -8), "\"0.00914800\"");
        assert_eq!(dec(0, -8), "\"0.00000000\"");
        assert_eq!(dec(-12_345, -2), "\"-123.45\"");
        assert_eq!(dec(95_901_520_000_000, -8), "\"959015.20000000\"");
        assert_eq!(dec(5, 2), "\"500\"");
        assert_eq!(dec(7, 0), "\"7\"");
        let mut s = String::new();
        decimal(&mut s, (-1_i128).to_le_bytes(), -3);
        assert_eq!(s, "\"-0.001\"");
    }
}
