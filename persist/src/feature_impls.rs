//! Feature-gated `PersistAs` impls for external crate types.
//!
//! Each impl is gated behind its own feature flag in `Cargo.toml`.
//! Each feature compiles and tests independently.

// ponytail: imports used only in #[cfg(feature = "...")] blocks
#![allow(unused_imports)]

use crate::PersistAs;
use crate::types::ColumnType;

// ── rust_decimal ─────────────────────────────────────────────────────────────

/// `rust_decimal::Decimal` maps to `Decimal(18, 8)`.
///
/// Encoded as the scaled integer (value × 10⁸) in little-endian i64.
#[cfg(feature = "rust_decimal")]
impl PersistAs for rust_decimal::Decimal {
    fn column_type() -> ColumnType {
        ColumnType::Decimal {
            precision: 18,
            scale: 8,
        }
    }

    fn encode_value(&self) -> Vec<u8> {
        let mantissa = self.mantissa();
        let scale = self.scale();
        // ponytail: rescale to Decimal(18,8) — panics if result exceeds i64 range
        let adjusted = if scale <= 8 {
            mantissa * 10_i128.pow(8 - scale)
        } else {
            mantissa / 10_i128.pow(scale - 8)
        };
        let scaled =
            i64::try_from(adjusted).expect("Decimal value out of range for Decimal(18, 8)");
        scaled.to_le_bytes().to_vec()
    }
}

// ── chrono ──────────────────────────────────────────────────────────────────

/// `chrono::NaiveDateTime` maps to `DateTime64(9)`.
///
/// Encoded as nanoseconds since Unix epoch in little-endian i64.
#[cfg(feature = "chrono")]
impl PersistAs for chrono::NaiveDateTime {
    fn column_type() -> ColumnType {
        ColumnType::DateTime64(9)
    }

    fn encode_value(&self) -> Vec<u8> {
        self.and_utc()
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .to_le_bytes()
            .to_vec()
    }
}

/// `chrono::DateTime<Utc>` maps to `DateTime64(9)`.
#[cfg(feature = "chrono")]
impl PersistAs for chrono::DateTime<chrono::Utc> {
    fn column_type() -> ColumnType {
        ColumnType::DateTime64(9)
    }

    fn encode_value(&self) -> Vec<u8> {
        self.timestamp_nanos_opt()
            .unwrap_or_default()
            .to_le_bytes()
            .to_vec()
    }
}

/// `chrono::DateTime<FixedOffset>` maps to `DateTime64(9)`.
#[cfg(feature = "chrono")]
impl PersistAs for chrono::DateTime<chrono::FixedOffset> {
    fn column_type() -> ColumnType {
        ColumnType::DateTime64(9)
    }

    fn encode_value(&self) -> Vec<u8> {
        self.timestamp_nanos_opt()
            .unwrap_or_default()
            .to_le_bytes()
            .to_vec()
    }
}

/// `chrono::NaiveDate` maps to `Date`.
///
/// Encoded as days since Unix epoch in little-endian UInt16.
#[cfg(feature = "chrono")]
impl PersistAs for chrono::NaiveDate {
    fn column_type() -> ColumnType {
        ColumnType::Date
    }

    fn encode_value(&self) -> Vec<u8> {
        let epoch =
            chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("1970-01-01 is a valid date");
        let days = (*self - epoch).num_days();
        u16::try_from(days)
            .expect("NaiveDate out of range for ClickHouse Date (UInt16)")
            .to_le_bytes()
            .to_vec()
    }
}

// ── duration ────────────────────────────────────────────────────────────────

/// `std::time::Duration` maps to `Interval`.
///
/// Encoded as total nanoseconds in little-endian i64.
impl PersistAs for std::time::Duration {
    fn column_type() -> ColumnType {
        ColumnType::Interval
    }

    fn encode_value(&self) -> Vec<u8> {
        let nanos = self.as_nanos();
        let scaled = i64::try_from(nanos).expect("Duration out of range for i64 nanoseconds");
        scaled.to_le_bytes().to_vec()
    }
}

// ── chrono::TimeDelta ─────────────────────────────────────────────

/// `chrono::TimeDelta` maps to `Interval`.
///
/// Encoded as total nanoseconds in little-endian i64.
#[cfg(feature = "chrono")]
impl PersistAs for chrono::TimeDelta {
    fn column_type() -> ColumnType {
        ColumnType::Interval
    }

    fn encode_value(&self) -> Vec<u8> {
        let nanos = self
            .num_nanoseconds()
            .expect("TimeDelta out of range for i64 nanoseconds");
        nanos.to_le_bytes().to_vec()
    }
}

// ── serde ───────────────────────────────────────────────────────────────────

/// `serde_json::Value` maps to `String` (JSON).
///
/// Serializes the value as a JSON string. For custom `Serialize` types,
/// convert to `serde_json::Value` first via `serde_json::to_value`.
#[cfg(feature = "serde")]
impl PersistAs for serde_json::Value {
    fn column_type() -> ColumnType {
        ColumnType::String
    }

    fn encode_value(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("JSON serialization")
    }
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::PersistAs;
    use crate::types::ColumnType;

    // ── rust_decimal ────────────────────────────────────────────────────

    #[test]
    #[cfg(feature = "rust_decimal")]
    fn rust_decimal_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <rust_decimal::Decimal as PersistAs>::column_type(),
            ColumnType::Decimal {
                precision: 18,
                scale: 8
            }
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "rust_decimal")]
    fn rust_decimal_encode_len() -> Result<(), Box<dyn std::error::Error>> {
        let d = rust_decimal::Decimal::new(1, 0);
        assert_eq!(d.encode_value().len(), 8);
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "rust_decimal")]
    fn rust_decimal_encode_one() -> Result<(), Box<dyn std::error::Error>> {
        let d = rust_decimal::Decimal::new(1, 0);
        assert_eq!(d.encode_value(), 100_000_000i64.to_le_bytes().to_vec());
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "rust_decimal")]
    fn rust_decimal_encode_negative() -> Result<(), Box<dyn std::error::Error>> {
        let d = rust_decimal::Decimal::new(-1, 0);
        assert_eq!(d.encode_value(), (-100_000_000i64).to_le_bytes().to_vec());
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "rust_decimal")]
    fn rust_decimal_encode_with_scale() -> Result<(), Box<dyn std::error::Error>> {
        let d = rust_decimal::Decimal::new(123, 2); // 1.23
        assert_eq!(d.encode_value(), 123_000_000i64.to_le_bytes().to_vec());
    
        Ok(())
    }

    // ── chrono ──────────────────────────────────────────────────────────

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_naive_datetime_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <chrono::NaiveDateTime as PersistAs>::column_type(),
            ColumnType::DateTime64(9)
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_naive_datetime_encode_epoch() -> Result<(), Box<dyn std::error::Error>> {
        let dt = chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        assert_eq!(dt.encode_value(), vec![0u8; 8]);
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_datetime_utc_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <chrono::DateTime<chrono::Utc> as PersistAs>::column_type(),
            ColumnType::DateTime64(9)
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_datetime_utc_encode_epoch() -> Result<(), Box<dyn std::error::Error>> {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap();
        assert_eq!(dt.encode_value(), vec![0u8; 8]);
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_datetime_fixed_offset_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <chrono::DateTime<chrono::FixedOffset> as PersistAs>::column_type(),
            ColumnType::DateTime64(9)
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_datetime_fixed_offset_encode_epoch() -> Result<(), Box<dyn std::error::Error>> {
        use chrono::TimeZone;
        let ndt = chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let dt: chrono::DateTime<chrono::FixedOffset> = chrono::FixedOffset::east_opt(0)
            .unwrap()
            .from_utc_datetime(&ndt);
        assert_eq!(dt.encode_value(), vec![0u8; 8]);
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_naive_date_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <chrono::NaiveDate as PersistAs>::column_type(),
            ColumnType::Date
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_naive_date_encode_epoch() -> Result<(), Box<dyn std::error::Error>> {
        let d = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        assert_eq!(d.encode_value(), vec![0u8; 2]);
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn chrono_naive_date_encode_known() -> Result<(), Box<dyn std::error::Error>> {
        let d = chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let expected = u16::try_from((d - epoch).num_days())
            .unwrap()
            .to_le_bytes()
            .to_vec();
        assert_eq!(d.encode_value(), expected);
    
        Ok(())
    }

    // ── duration ────────────────────────────────────────────────────────

    #[test]
    fn duration_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <std::time::Duration as PersistAs>::column_type(),
            ColumnType::Interval
        );
    
        Ok(())
    }

    #[test]
    fn duration_encode_zero() -> Result<(), Box<dyn std::error::Error>> {
        let d = std::time::Duration::ZERO;
        assert_eq!(d.encode_value(), vec![0u8; 8]);
    
        Ok(())
    }

    #[test]
    fn duration_encode_one_second() -> Result<(), Box<dyn std::error::Error>> {
        let d = std::time::Duration::new(1, 0);
        assert_eq!(d.encode_value(), 1_000_000_000i64.to_le_bytes().to_vec());
    
        Ok(())
    }

    // ── chrono::TimeDelta ────────────────────────────────────────────

    #[test]
    #[cfg(feature = "chrono")]
    fn time_delta_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <chrono::TimeDelta as PersistAs>::column_type(),
            ColumnType::Interval
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn time_delta_encode_zero() -> Result<(), Box<dyn std::error::Error>> {
        let d = chrono::TimeDelta::nanoseconds(0);
        assert_eq!(d.encode_value(), vec![0u8; 8]);
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn time_delta_encode_one_second() -> Result<(), Box<dyn std::error::Error>> {
        let d = chrono::TimeDelta::nanoseconds(1_000_000_000);
        assert_eq!(d.encode_value(), 1_000_000_000i64.to_le_bytes().to_vec());
    
        Ok(())
    }

    // ── serde ───────────────────────────────────────────────────────────

    #[test]
    #[cfg(feature = "serde")]
    fn serde_value_column_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            <serde_json::Value as PersistAs>::column_type(),
            ColumnType::String
        );
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_value_encode_null() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::Value::Null;
        assert_eq!(v.encode_value(), b"null");
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_value_encode_string() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::Value::String("hello".into());
        assert_eq!(v.encode_value(), b"\"hello\"");
    
        Ok(())
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_value_encode_number() -> Result<(), Box<dyn std::error::Error>> {
        let v = serde_json::json!(42);
        assert_eq!(v.encode_value(), b"42");
    
        Ok(())
    }
}
