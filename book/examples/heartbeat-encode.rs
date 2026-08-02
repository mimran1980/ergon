//! Heartbeat encode/decode — fixed-only message, no groups or var-data.
//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: staged_chaining
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
// Buffer is exact size from const compute_length_with_header — no bounds check needed.
let len = HeartbeatEncoder::wrap_and_apply_header(&mut buf, 0).unwrap()
    .fixed(&HeartbeatFixedFields { sequence: 7, timestamp: 0 })
    .encoded_length_with_header();
let dec = HeartbeatDecoder::try_from(&buf[..len])?;
assert_eq!(dec.sequence(), 7);
// ANCHOR_END: staged_chaining

// ANCHOR: raw_fixed
// raw_fixed example — write individual fields when you don't have a FixedFields struct.
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
let mut w = HeartbeatEncoder::wrap_and_apply_header(&mut buf, 0).raw_fixed();
w.sequence(7);
w.timestamp(0);
// ANCHOR_END: raw_fixed
