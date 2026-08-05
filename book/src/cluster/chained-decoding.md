# Chained Message Decoding

A `SessionMessageHeader` is followed by application payload bytes (another SBE
message). Use `get_metadata().remaining()` to get the payload byte slice, then
`AnyMessage::decode` to parse the next message. This uses the non-stable
`cluster_codec_types` seam described above:

```rust,ignore
use ergo_aeron_cluster::cluster_codec_types::*;

fn decode_chained() -> Result<(), Box<dyn std::error::Error>> {
    // Encode: SessionMessageHeader (32 bytes) + SessionKeepAlive (32 bytes).
    // Both lengths are const, so size the frame on the stack.
    let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH
        + SessionKeepAliveEncoder::ENCODED_LENGTH];

    let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.leadership_term_id(7)
        .cluster_session_id(99)
        .timestamp(42);

    // into_remaining_mut() returns the unwritten region after this message
    SessionKeepAliveEncoder::wrap_and_apply_header(enc.into_remaining_mut(), 0)
        .leadership_term_id(7)
        .cluster_session_id(99);

    // Decode the first message
    let smh = SessionMessageHeaderDecoder::decode(&buf, 0)?;

    // get_metadata().remaining() returns bytes after this message
    let tail = smh.get_metadata().remaining();
    assert_eq!(tail.len(), SessionKeepAliveEncoder::ENCODED_LENGTH);

    // Decode the next message via AnyMessage
    match AnyMessage::decode(tail, 0)? {
        AnyMessage::SessionKeepAlive(dec) => {
            assert_eq!(dec.cluster_session_id(), 99);
            // Fully decoded — nothing left
            assert!(dec.get_metadata().remaining().is_empty());
        }
        _ => panic!("unexpected message type"),
    }
    Ok(())
}
```

`get_metadata().buffer()` returns the entire original buffer. `remaining()`
returns bytes after this message — both scoped in the metadata struct so
no schema field can ever collide with them.
