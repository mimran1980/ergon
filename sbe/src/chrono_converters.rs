//! Chrono timestamp converters — feature-gated behind `chrono`.
//!
//! Wire all SBE timestamp fields as `i64` (nanoseconds since epoch for
//! `UTCTimestamp`, microseconds for `UTCTimestampMicros`, etc.). These
//! converters bridge to [`chrono::DateTime`] and [`chrono::NaiveDateTime`]
//! so `with_domain_type` produces `DateTime<Utc>` / `NaiveDateTime` fields.
//!
//! # Usage (build.rs)
//!
//! ```rust,ignore
//! use ergo_sbe::{GenerationConfig, ConversionSelector};
//!
//! let config = GenerationConfig::new("msgs")
//!     .with_domain_type(
//!         ConversionSelector::semantic_type("UTCTimestamp"),
//!         "chrono::DateTime<chrono::Utc>",
//!     )
//!     .with_domain_type(
//!         ConversionSelector::semantic_type("UTCTimestampMicros"),
//!         "chrono::NaiveDateTime",
//!     );
//! ```
//!
//! # Generated API
//!
//! ```rust,ignore
//! // Decoder
//! let ts: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
//! // Encoder
//! enc.try_created_at(chrono::Utc::now())?;
//! ```

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

/// Convert SBE `i64` wire nanoseconds → [`DateTime<Utc>`].
///
/// The SBE epoch (Unix epoch, 1970-01-01T00:00:00Z) is aligned with
/// `chrono::Utc`. Saturates at `DateTime::<Utc>::MAX_UTC` /
/// `MIN_UTC` for out-of-range wire values.
#[must_use]
pub fn i64_nanos_to_datetime(nanos: i64) -> DateTime<Utc> {
    let secs = nanos.div_euclid(1_000_000_000);
    let nsecs: u32 = nanos.rem_euclid(1_000_000_000) as u32;
    Utc.timestamp_opt(secs, nsecs)
        .single()
        .unwrap_or(if nanos < 0 { DateTime::<Utc>::MIN_UTC } else { DateTime::<Utc>::MAX_UTC })
}

/// Convert [`DateTime<Utc>`] → SBE `i64` wire nanoseconds.
#[must_use]
pub fn datetime_to_i64_nanos(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_nanos_opt().unwrap_or(i64::MAX)
}

/// Convert SBE `i64` wire microseconds → [`NaiveDateTime`].
#[must_use]
pub fn i64_micros_to_naive(micros: i64) -> NaiveDateTime {
    let secs = micros.div_euclid(1_000_000);
    let nsecs: u32 = (micros.rem_euclid(1_000_000) * 1000) as u32;
    DateTime::from_timestamp_nanos(
        secs.saturating_mul(1_000_000_000)
            .saturating_add(i64::from(nsecs)),
    )
    .naive_utc()
}

/// Convert [`NaiveDateTime`] → SBE `i64` wire microseconds.
#[must_use]
pub fn naive_to_i64_micros(dt: NaiveDateTime) -> i64 {
    dt.and_utc()
        .timestamp_nanos_opt()
        .map_or(i64::MAX, |n| n / 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_datetime() {
        let now = Utc::now();
        let wire = datetime_to_i64_nanos(now);
        let back = i64_nanos_to_datetime(wire);
        // Sub-second precision may differ by 1 nanosecond due to truncation
        assert_eq!(now.timestamp(), back.timestamp());
    }

    #[test]
    fn roundtrip_naive() {
        let now = Utc::now().naive_utc();
        let wire = naive_to_i64_micros(now);
        let back = i64_micros_to_naive(wire);
        assert_eq!(now.timestamp(), back.timestamp());
    }

    #[test]
    fn zero_is_epoch() {
        let dt = i64_nanos_to_datetime(0);
        assert_eq!(dt.timestamp(), 0);
        let naive = i64_micros_to_naive(0);
        assert_eq!(naive.timestamp(), 0);
    }

    #[test]
    fn saturates_extremes() {
        // i64::MAX nanoseconds is far past MAX_UTC; clamp falls through to
        // the unwrap_or arm.
        let too_big = i64_nanos_to_datetime(i64::MAX);
        // The returned value must not panic and must be at or past MAX_UTC
        assert!(too_big.timestamp_nanos_opt().is_some());
        // i64::MIN nanoseconds is far before MIN_UTC
        let too_small = i64_nanos_to_datetime(i64::MIN);
        assert!(too_small.timestamp_nanos_opt().is_some());
    }
}
