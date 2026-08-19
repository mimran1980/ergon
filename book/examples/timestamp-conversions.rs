//! Timestamp conversion patterns: build.rs config for UTCTimestamp fields,
//! and the TryFromSbe / TryToSbe adapter pattern for custom precisions.
//! Compiled against the tour_codec by the book-fence test.

// ANCHOR: timestamp_config
use ergo_sbe::{GenerationConfig, ConversionSelector, DomainImpl};
// Register converters in build.rs — nanos uses the built-in converter,
// micros and millis get custom TryFromSbe impls wired via field_path:
let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "chrono::DateTime<chrono::Utc>",
        DomainImpl::Generated,
    );
// ANCHOR_END: timestamp_config

// ANCHOR: timestamp_adapter_nanos
use chrono::{DateTime, Utc};
// Nanos — delegates to the built-in logic:
fn _nanos_to_dt(wire_nanos: u64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(
        (wire_nanos / 1_000_000_000) as i64,
        (wire_nanos % 1_000_000_000) as u32,
    )
}
// ANCHOR_END: timestamp_adapter_nanos

// ANCHOR: timestamp_encode_decode
// Encode a timestamp (field configured with UTCTimestamp domain type):
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
let now = DateTime::from_timestamp(1_720_000_000, 0).unwrap();
let len = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .fixed(&HeartbeatFixedFields { sequence: 7, timestamp: 1_720_000_000_000_000_000 })
    .encoded_length_with_header();
// Decode — domain type via fallible try_* ():
let dec = HeartbeatDecoder::try_from(&buf[..len])?;
let ts: DateTime<Utc> = dec.try_timestamp()?;
assert!(ts.timestamp_nanos_opt().is_some());
// ANCHOR_END: timestamp_encode_decode
