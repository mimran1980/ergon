# Multi-Message & Framing

Ergon supports two framing approaches for adjacent messages, and an `AnyMessage`
dispatch enum for multi-message streams where the next type isn't known until
runtime.

Normal Cluster applications use `AeronCluster` and its egress listeners. The
protocol-codec examples below illustrate framing through the repository's
`cluster_codec_types` test/benchmark seam, which is not a stable consumer API.

## Two framing approaches

### 1. Back-to-back with encoded length

Pre-compute each message's exact size, lay them out at known offsets, and
validate after encoding. Safest when you know all messages ahead of time.

```rust,ignore
// Size every message from its own payload length.
let len_a = MsgAEncoder::compute_length_with_header(data_a.len());
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

// Both schemas are known, so their tails determine their message boundaries.
let wire = &buf[..len_a + len_b];
```

### 2. Stream / `remaining()` slot

Write sequentially; use `remaining()` to find where the next message starts.
Idiomatic for Aeron cluster sessions where a `SessionMessageHeader` is
immediately followed by application payload.

```rust,ignore
use ergo_aeron_cluster::cluster_codec_types::*;

let mut buf = [0u8; SessionMessageHeaderEncoder::compute_length_with_header()
    + SessionKeepAliveEncoder::compute_length_with_header()];

// Encode the outer message. `fixed()` writes the required body so a reused
// buffer cannot publish leftover bytes.
let tail = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&SessionMessageHeaderFixedFields {
        leadership_term_id: 7,
        cluster_session_id: 99,
        timestamp: 42,
    })
    .into_remaining_mut();

// into_remaining_mut() returns the unwritten tail.
let keep_alive_len = SessionKeepAliveEncoder::wrap_and_apply_header(tail, 0)
    .fixed(&SessionKeepAliveFixedFields {
        leadership_term_id: 7,
        cluster_session_id: 99,
    })
    .encoded_length_with_header();
assert_eq!(keep_alive_len, SessionKeepAliveEncoder::compute_length_with_header());

// Decode: remaining() gives bytes after the first message.
let smh = SessionMessageHeaderDecoder::decode(&buf, 0)?;
let tail = smh.get_metadata().remaining();
assert_eq!(tail.len(), SessionKeepAliveEncoder::ENCODED_LENGTH);
```

## AnyMessage dispatch

Cluster sessions multiplex many message types on a single stream.
`AnyMessage::try_decode` reads the 8-byte session SBE header, validates its
schema identity, and selects the template. For known templates it also checks
the acting fixed-body extent. Dynamic tails are checked when consumed:

```rust,ignore
use ergo_aeron_cluster::cluster_codec_types::*;

fn dispatch(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match AnyMessage::try_decode(data, 0)? {
        AnyMessage::SessionMessageHeader(decoder) => {
            // This wraps application payload. The application owns its
            // schema; do not recursively dispatch it as Cluster protocol.
            let payload = decoder.get_metadata().remaining();
            println!("application payload: {} bytes", payload.len());
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
            println!("challenge: {} bytes", chal.len());
            // respond to challenge...
        }
        AnyMessage::AdminResponse(decoder) => {
            let (msg, after) = decoder.into_message()?;
            let (payload, _) = after.into_payload()?;
            println!("admin response: {msg:?}, {} payload bytes", payload.len());
        }
        AnyMessage::SessionKeepAlive(_) => {
            // heartbeat — nothing to do
        }
        AnyMessage::Unknown { .. } => {
            // Produced by decode_frame when an external length is supplied.
        }
        _ => {
            // Other known session templates are not handled by this example.
        }
    }
    Ok(())
}
```

SBE headers do not carry a complete message length. `try_decode` returns
`UnknownTemplateLength` for an unknown template because its tail cannot be
located without the schema. Use `AnyMessage::decode_frame` / `FrameCursor`
with an external frame length when unknown templates must be preserved or
skipped. The high-level Cluster client can ignore an unknown protocol message
because Aeron already supplies the fragment boundary.

## Metadata

Every decoder exposes `get_metadata()` which returns a `Metadata` struct:

| Method | Returns |
|--------|---------|
| `buffer()` | The entire original `&[u8]` buffer |
| `remaining()` | Bytes after the acting fixed block (`&buffer[limit()..]`) |
| `message_offset()` | Absolute offset of this message's frame start within `buffer()` |
| `limit()` | End of the acting fixed block (not the full frame when tails follow) |

Metadata `remaining()` starts immediately after the fixed block. That is the
application payload for a fixed-only `SessionMessageHeader`; for a message
with groups or var-data, it starts at that message's first tail. To locate the
next message after a tailed message, complete its staged walk and use the
complete stage's `remaining()`. Calling the base decoder's full-length helper
instead requires a separate tail scan.
