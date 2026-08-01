//! Heartbeat encode/decode — fixed-only message, no groups or var-data.
//! Compiled against the feature-tour codec by the book-fence test.

// ANCHOR: staged_chaining
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
let len = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .fixed(&HeartbeatFixedFields { sequence: 7, timestamp: 0 })
    .encoded_length_with_header();
let dec = HeartbeatDecoder::try_from(&buf[..len])?;
assert_eq!(dec.sequence(), 7);
// ANCHOR_END: staged_chaining
