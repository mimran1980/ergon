# Multi-Message & Framing

Ergon supports two framing approaches for adjacent messages, and an `AnyMessage`
dispatch enum for multi-message streams where the next type isn't known until
runtime.

## Two framing approaches

### 1. Back-to-back with encoded length

Pre-compute each message's exact size, lay them out at known offsets, and
validate after encoding. Safest when you know all messages ahead of time.

```rust,ignore
// Size every message first (both const).
let len_a = MsgAEncoder::compute_length_with_header();
let len_b = MsgBEncoder::compute_length_with_header(data_b.len());

let mut buf = vec![0u8; len_a + len_b];

// Encode MsgA at offset 0.
let a_len = MsgAEncoder::wrap_and_apply_header(&mut buf[..len_a], 0)
    .fixed(&fields_a)
    .data(data_a)?
    .encoded_length_with_header();
assert_eq!(a_len, len_a);

// Encode MsgB at offset len_a.
let b_len = MsgBEncoder::wrap_and_apply_header(&mut buf[len_a..], 0)
    .fixed(&fields_b)
    .data(data_b)?
    .encoded_length_with_header();
assert_eq!(b_len, len_b);

// Wire frame: two self-describing SBE messages back-to-back.
let wire = &buf[..len_a + len_b];
```

### 2. Stream / `remaining()` slot

Write sequentially; use `remaining()` to find where the next message starts.
Idiomatic for Aeron cluster sessions where a `SessionMessageHeader` is
immediately followed by application payload.

```rust,ignore
use ergo_aeron_cluster::cluster_codec_types::*;

let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH
    + SessionKeepAliveEncoder::ENCODED_LENGTH];

// Encode the outer message. `fixed()` writes the required body so a reused
// buffer cannot publish leftover bytes.
let enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&SessionMessageHeaderFixedFields {
        leadership_term_id: 7,
        cluster_session_id: 99,
        timestamp: 42,
    });

// into_remaining_mut() returns the unwritten tail.
SessionKeepAliveEncoder::wrap_and_apply_header(enc.into_remaining_mut(), 0)
    .fixed(&SessionKeepAliveFixedFields {
        leadership_term_id: 7,
        cluster_session_id: 99,
    });

// Decode: remaining() gives bytes after the first message.
let smh = SessionMessageHeaderDecoder::decode(&buf, 0)?;
let tail = smh.get_metadata().remaining();
assert_eq!(tail.len(), SessionKeepAliveEncoder::ENCODED_LENGTH);
```

## AnyMessage dispatch

Cluster sessions multiplex many message types on a single stream.
`AnyMessage::decode` reads the 8-byte SBE header, inspects the template ID,
and returns the matching variant:

```rust,ignore
use ergo_aeron_cluster::cluster_codec_types::*;

fn dispatch(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match AnyMessage::decode(data, 0)? {
        AnyMessage::SessionMessageHeader(decoder) => {
            // This wraps application payload. Use remaining() to get
            // the bytes after the 32-byte header, then decode again.
            let payload = decoder.get_metadata().remaining();
            if !payload.is_empty() {
                dispatch(payload)?;
            }
        }
        AnyMessage::SessionEvent(decoder) => {
            let code = decoder.code();
            let (detail, _) = decoder.into_detail_as_str()?;
            println!("event {code}: {detail}");
        }
        AnyMessage::NewLeaderEvent(decoder) => {
            let (endpoints, _) = decoder.into_ingress_endpoints_as_str()?;
            println!("new leader at {endpoints}");
        }
        AnyMessage::Challenge(decoder) => {
            let (chal, _) = decoder.into_encoded_challenge()?;
            // respond to challenge...
        }
        AnyMessage::AdminResponse(decoder) => {
            let (msg, after) = decoder.into_message()?;
            let (payload, _) = after.into_payload()?;
            println!("admin response: {msg:?}");
        }
        AnyMessage::SessionKeepAlive(decoder) => {
            // heartbeat — nothing to do
        }
        AnyMessage::Unknown { .. } => {
            // Not an error — the cluster may send messages
            // not in our schema. Skip them.
        }
    }
    Ok(())
}
```

`AnyMessage::decode` validates only the 8-byte SBE frame header. Always guard
truncated payloads before slicing — e.g. check
`data.len() >= SessionMessageHeaderEncoder::ENCODED_LENGTH` before calling
`remaining()`.

## Metadata

Every decoder exposes `get_metadata()` which returns a `Metadata` struct:

| Method | Returns |
|--------|---------|
| `buffer()` | The entire original `&[u8]` buffer |
| `remaining()` | Bytes after the acting fixed block (`&buffer[limit()..]`) |
| `message_offset()` | Absolute offset of this message's frame start within `buffer()` |
| `limit()` | End of the acting fixed block (not the full frame when tails follow) |

`remaining()` is the key for chaining — it gives you the exact tail slice where
the next message begins, zero-copy.
