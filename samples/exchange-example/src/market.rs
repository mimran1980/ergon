//! Normalized market data types, the vocabulary shared by the three deep
//! modules (`BitgetIngestor`, `ClaimPublisher`, `ForegroundPersistor`).

/// A wire decimal value: `mantissa × 10^exponent`. Matches the SBE
/// `Decimal { mantissa: int64, exponent: int8 }` composite exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireDec {
    pub mantissa: i64,
    pub exponent: i8,
}

impl WireDec {
    pub const fn new(mantissa: i64, exponent: i8) -> Self {
        Self { mantissa, exponent }
    }
}

/// Exact decimal text for `mantissa × 10^exponent`: integer string
/// arithmetic only, so large mantissas round-trip losslessly (no floats,
/// no decimal crate).
impl core::fmt::Display for WireDec {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.mantissa == 0 {
            return f.write_str("0");
        }
        let negative = self.mantissa < 0;
        let digits = self.mantissa.unsigned_abs().to_string();
        if negative {
            f.write_str("-")?;
        }
        if self.exponent >= 0 {
            f.write_str(&digits)?;
            for _ in 0..self.exponent {
                f.write_str("0")?;
            }
            return Ok(());
        }
        // Negative exponent: insert the point `exponent` places from the
        // right of the digit string, left-padding with "0." when it runs off.
        let point = digits.len() as i64 + i64::from(self.exponent);
        if point > 0 {
            f.write_str(&digits[..point as usize])?;
            f.write_str(".")?;
            f.write_str(&digits[point as usize..])
        } else {
            f.write_str("0.")?;
            for _ in 0..-point {
                f.write_str("0")?;
            }
            f.write_str(&digits)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WireDec;

    #[test]
    fn display_renders_exact_decimals() {
        assert_eq!(WireDec::new(5_000_000, -3).to_string(), "5000.000");
        assert_eq!(WireDec::new(-5_000_000, -3).to_string(), "-5000.000");
        assert_eq!(WireDec::new(123, 0).to_string(), "123");
        assert_eq!(WireDec::new(5, 2).to_string(), "500");
        assert_eq!(WireDec::new(5, -8).to_string(), "0.00000005");
        assert_eq!(WireDec::new(42, -1).to_string(), "4.2");
        assert_eq!(WireDec::new(0, -3).to_string(), "0");
        // Full i64 magnitude: no float ever touches this path.
        assert_eq!(
            WireDec::new(i64::MAX, -6).to_string(),
            "9223372036854.775807"
        );
    }
}

/// One side level of a normalized order book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub price: WireDec,
    pub size: WireDec,
}

/// Borrowed normalized event emitted by the ingestor. Publication encodes
/// straight from these borrows, with no owned intermediate message objects.
#[derive(Debug, Clone, Copy)]
pub enum NormalizedEventRef<'a> {
    L2Book {
        symbol: &'a str,
        exchange_ts_ns: u64,
        receive_ts_ns: u64,
        /// Monotonic correlation value assigned by the ingestor.
        sequence: u64,
        /// Best-first (descending price).
        bids: &'a [Level],
        /// Best-first (ascending price).
        asks: &'a [Level],
    },
    Trade {
        symbol: &'a str,
        exchange_ts_ns: u64,
        receive_ts_ns: u64,
        sequence: u64,
        price: WireDec,
        size: WireDec,
        is_buy: bool,
    },
}
