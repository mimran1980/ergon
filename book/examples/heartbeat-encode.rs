//! Compilable heartbeat encode/decode — used by Getting Started pages.
//! The book-fence test injects the generated codec module; the anchor
//! content below compiles against `use demo_codec::*` from the test wrapper.

// ANCHOR: staged_chaining
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
let len = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .fixed(&HeartbeatFixedFields { seq: 7 })
    .encoded_length_with_header();
let dec = HeartbeatDecoder::try_from(&buf[..len])?;
assert_eq!(dec.seq(), 7);
// ANCHOR_END: staged_chaining
