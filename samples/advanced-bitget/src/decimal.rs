//! Exact Decimal conversion: SBE Decimal (mantissa × 10^exponent) ↔ ClickHouse Decimal(38,18).
//!
//! Conversion is checked and allocation-free. Rejects:
//! - Overflow beyond i128 range for the scaled integer
//! - Non-zero precision loss (discarded fractional digits)
//! - Values outside Decimal(38,18) range
//!
//! The stored scaled integer is `mantissa × 10^(exponent + 18)`.
//! When `exponent + 18` is negative, division is allowed only if every
//! discarded digit is zero.

/// An exact SBE Decimal value: `mantissa × 10^exponent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbeDecimal {
    pub mantissa: i64,
    pub exponent: i8,
}

/// Errors returned by Decimal conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecimalConvertError {
    /// Scaling factor overflowed i128.
    Overflow { mantissa: i64, exponent: i8 },
    /// Non-zero digits would be discarded by rescaling.
    PrecisionLoss { mantissa: i64, exponent: i8, lost_digits: u32 },
    /// Value outside ClickHouse Decimal(38,18) range.
    OutOfRange { scaled: i128 },
}

impl core::fmt::Display for DecimalConvertError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Overflow { mantissa, exponent } => {
                write!(f, "overflow scaling {mantissa} × 10^{exponent} to Decimal(38,18)")
            }
            Self::PrecisionLoss { mantissa, exponent, lost_digits } => {
                write!(f, "precision loss: {mantissa} × 10^{exponent} would discard {lost_digits} non-zero digits")
            }
            Self::OutOfRange { scaled } => {
                write!(f, "{scaled} outside Decimal(38,18) range")
            }
        }
    }
}

impl std::error::Error for DecimalConvertError {}

/// Maximum value for Decimal(38,18): (10^38 - 1).
const DECIMAL_38_18_MAX: i128 = 99999999999999999999999999999999999999;
const DECIMAL_38_18_MIN: i128 = -DECIMAL_38_18_MAX;

/// Convert an SBE (mantissa, exponent) pair to a ClickHouse Decimal(38,18) scaled integer.
///
/// The target scale is 18. Conversion: `scaled = mantissa × 10^(exponent + 18)`.
/// Rejects values that can't be exactly represented.
pub fn to_clickhouse_decimal(mantissa: i64, exponent: i8) -> Result<i128, DecimalConvertError> {
    let scale_diff = exponent as i32 + 18; // target scale is 18

    if scale_diff >= 0 {
        // Multiply: mantissa × 10^scale_diff
        let factor = ten_pow(scale_diff as u32)
            .ok_or(DecimalConvertError::Overflow { mantissa, exponent })?;
        let mantissa_i128 = mantissa as i128;
        let scaled = mantissa_i128
            .checked_mul(factor)
            .ok_or(DecimalConvertError::Overflow { mantissa, exponent })?;
        check_range(scaled)?;
        Ok(scaled)
    } else {
        // Divide: mantissa / 10^(-scale_diff)
        let divisor = ten_pow((-scale_diff) as u32)
            .ok_or(DecimalConvertError::Overflow { mantissa, exponent })?;
        let mantissa_i128 = mantissa as i128;
        // Check for non-zero remainder (precision loss)
        let remainder = mantissa_i128 % divisor;
        if remainder != 0 {
            // Count non-zero digits in the discarded portion
            let lost = count_non_zero_digits(remainder.unsigned_abs());
            return Err(DecimalConvertError::PrecisionLoss { mantissa, exponent, lost_digits: lost });
        }
        let scaled = mantissa_i128 / divisor;
        check_range(scaled)?;
        Ok(scaled)
    }
}

fn check_range(scaled: i128) -> Result<(), DecimalConvertError> {
    if scaled < DECIMAL_38_18_MIN || scaled > DECIMAL_38_18_MAX {
        Err(DecimalConvertError::OutOfRange { scaled })
    } else {
        Ok(())
    }
}

/// Compute 10^n, returning None on overflow.
fn ten_pow(n: u32) -> Option<i128> {
    if n > 38 {
        return None; // 10^39 > i128::MAX
    }
    let mut result: i128 = 1;
    for _ in 0..n {
        result = result.checked_mul(10)?;
    }
    Some(result)
}

fn count_non_zero_digits(mut n: u128) -> u32 {
    let mut count = 0;
    while n > 0 {
        if n % 10 != 0 {
            count += 1;
        }
        n /= 10;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_conversion_no_scaling() {
        // exponent = -18 → scale_diff = 0, no scaling needed
        let result = to_clickhouse_decimal(12345_6789_0123_4567, -18).unwrap();
        assert_eq!(result, 12345_6789_0123_4567);
    }

    #[test]
    fn scale_up_mantissa() {
        // exponent = -8 → scale_diff = 10, multiply by 10^10
        let result = to_clickhouse_decimal(1_0000_0000, -8).unwrap(); // 1.00000000
        assert_eq!(result, 1_0000_0000_0000_0000_00i128); // scaled to -18
    }

    #[test]
    fn scale_down_mantissa_exact() {
        // exponent = -20 → scale_diff = -2, divide by 100
        let result = to_clickhouse_decimal(12300, -20).unwrap();
        assert_eq!(result, 123); // 12300 / 100 = 123
    }

    #[test]
    fn precision_loss_rejected() {
        // 123 / 100 = 1.23, but 123 is not divisible by 100
        let err = to_clickhouse_decimal(123, -20).unwrap_err();
        assert!(matches!(err, DecimalConvertError::PrecisionLoss { .. }));
    }

    #[test]
    fn exact_values_roundtrip() {
        let cases = [
            (0, 0),                 // zero
            (1, 0),                // 1
            (-1, 0),               // -1
            (12345, -2),           // 123.45
            (50000_00, -2),        // 50000.00
            (1_50, -2),            // 1.50
            (i64::MAX, 0),         // max i64
            (1, -18),              // 0.000000000000000001
        ];
        for (mantissa, exponent) in &cases {
            let result = to_clickhouse_decimal(*mantissa, *exponent);
            assert!(result.is_ok(), "failed: {mantissa} × 10^{exponent}: {:?}", result.err());
        }
    }

    #[test]
    fn overflow_rejected() {
        // i64::MAX × 10^20 will overflow i128
        let err = to_clickhouse_decimal(i64::MAX, 20).unwrap_err();
        assert!(matches!(err, DecimalConvertError::Overflow { .. }));
    }
}
