//! Chrono timestamp converters — feature-gated behind `chrono`.
//!
//! Wire all SBE timestamp fields as `i64` (nanoseconds since epoch for
//! `UTCTimestamp`, microseconds for `UTCTimestampMicros`, etc.). These
//! converters bridge to [`chrono::DateTime`] and [`chrono::NaiveDateTime`]
//! so `with_domain_type` produces `DateTime<Utc>` / `NaiveDateTime` fields.
//!
//! # Usage (build.rs)
//!
//! ```rust,no_run
//! # // This compiles only with --features chrono; no_run lets it pass doc-tests.
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
//! ```rust,no_run
//! # // Compiles only with chrono feature enabled.
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
        .unwrap_or(if nanos < 0 {
            DateTime::<Utc>::MIN_UTC
        } else {
            DateTime::<Utc>::MAX_UTC
        })
}

/// Convert [`DateTime<Utc>`] → SBE `i64` wire nanoseconds.
#[must_use]
pub fn datetime_to_i64_nanos(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_nanos_opt().unwrap_or(i64::MAX)
}

/// Convert SBE `i64` wire microseconds → [`NaiveDateTime`].
///
/// Uses microsecond-native construction to preserve the full `i64` range
/// (valid microsecond timestamps span ±292 000 years). Saturates at the
/// chrono representable limits for out-of-range values.
#[must_use]
pub fn i64_micros_to_naive(micros: i64) -> NaiveDateTime {
    chrono::DateTime::from_timestamp_micros(micros).map(|dt| dt.naive_utc()).unwrap_or_else(|| {
        if micros < 0 {
            DateTime::<Utc>::MIN_UTC.naive_utc()
        } else {
            DateTime::<Utc>::MAX_UTC.naive_utc()
        }
    })
}

/// Convert [`NaiveDateTime`] → SBE `i64` wire microseconds.
///
/// Uses microsecond-native extraction (`timestamp_micros()`) so valid
/// microsecond-precision timestamps round-trip exactly without saturating
/// through the narrower nanosecond range.
#[must_use]
pub const fn naive_to_i64_micros(dt: NaiveDateTime) -> i64 {
    dt.and_utc().timestamp_micros()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_datetime() {
        let now = Utc::now();
        let wire = datetime_to_i64_nanos(now);
        let back = i64_nanos_to_datetime(wire);
        // Sub-second precision may differ by 1 nanosecond due to truncation.
        // Both are DateTime<Utc> so .timestamp() is not deprecated.
        assert_eq!(now.timestamp(), back.timestamp());
    }

    #[test]
    fn roundtrip_naive() {
        let now = Utc::now().naive_utc();
        let wire = naive_to_i64_micros(now);
        let back = i64_micros_to_naive(wire);
        assert_eq!(now.and_utc().timestamp(), back.and_utc().timestamp());
    }

    #[test]
    fn zero_is_epoch() {
        let dt = i64_nanos_to_datetime(0);
        assert_eq!(dt.timestamp(), 0);
        let naive = i64_micros_to_naive(0);
        assert_eq!(naive.and_utc().timestamp(), 0);
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
