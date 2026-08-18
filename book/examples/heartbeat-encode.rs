//! Heartbeat encode/decode — fixed-only message, no groups or var-data.
//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: staged_chaining
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
// Buffer is exact size from const compute_length_with_header — no bounds check needed.
let len = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .fixed(&HeartbeatFixedFields { sequence: 7, timestamp: 0 })
    .encoded_length_with_header();
let dec = HeartbeatDecoder::try_from(&buf[..len])?;
assert_eq!(dec.sequence(), 7);
// ANCHOR_END: staged_chaining

// ANCHOR: raw_fixed
// Dedicated raw writer (setters also exist on the unfixed encoder).
// `as_bytes_with_header` is only on FieldsFixed after `fixed(&FixedFields)`.
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
let mut w = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .raw_fixed();
w.sequence(7);
w.timestamp_wire(0);
let dec = HeartbeatDecoder::try_from(&buf[..HeartbeatEncoder::ENCODED_LENGTH])?;
assert_eq!(dec.sequence(), 7);
// ANCHOR_END: raw_fixed
