//! App code patterns for `with_conversion` — adapter impl, encode, decode.
//! Compiled against the tour_codec by the book-fence test.

// ANCHOR: adapter_impl
use rust_decimal::Decimal as Rd;
// App adapter: wire Decimal ↔ rust_decimal::Decimal
struct FixedPrice { mantissa: i64, exponent: i8 }

impl TryFromSbe<Decimal> for FixedPrice {
    type Error = &'static str;
    fn try_from_sbe(wire: Decimal) -> Result<Self, Self::Error> {
        Ok(FixedPrice {
            mantissa: wire.mantissa(),
            exponent: wire.exponent(),
        })
    }
}
impl TryToSbe<Decimal> for FixedPrice {
    type Error = &'static str;
    fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
        Ok(Decimal::new(self.mantissa, self.exponent))
    }
}
// ANCHOR_END: adapter_impl

// ANCHOR: conversion_encode_decode
// Encode using the generic conversion API:
let mut buf = [0u8; QuoteEncoder::compute_length_with_header()];
let price = Rd::new(12345, 2); // 123.45
let len = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .price_from(&price)?
    .size_from(&Rd::new(10, 0))?
    .encoded_length_with_header();
// Decode — generic `_as::<T>()` picks your adapter:
let dec = QuoteDecoder::try_from(&buf[..len])?;
let p: Rd = dec.price_as()?;
assert_eq!(p, Rd::new(12345, 2));
// Same buffer, different app type — only possible with with_conversion:
let fixed: FixedPrice = dec.price_as()?;
assert_eq!(fixed.mantissa, 12345);
assert_eq!(fixed.exponent, -2);
// ANCHOR_END: conversion_encode_decode
