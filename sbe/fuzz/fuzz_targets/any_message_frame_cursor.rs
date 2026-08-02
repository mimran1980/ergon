#![no_main]

use ergo_sbe_fuzz::l3_codec::{AnyMessage, FrameCursor, FramingPolicy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let pos = data.first().map_or(0, |byte| usize::from(*byte)) % data.len().saturating_add(1);
    let available = data.len().saturating_sub(pos);
    let frame_len = data
        .get(1..9)
        .and_then(|bytes| bytes.try_into().ok())
        .and_then(|bytes| usize::try_from(u64::from_le_bytes(bytes)).ok())
        .unwrap_or(available)
        .min(available);

    let _ = AnyMessage::try_decode(data, pos);
    let _ = AnyMessage::decode_frame(data, pos, frame_len);

    for policy in [
        FramingPolicy::LengthPrefixU16,
        FramingPolicy::LengthPrefixU32,
        FramingPolicy::Fixed(frame_len),
    ] {
        let mut cursor = FrameCursor::new(data, policy);
        for _ in 0..256 {
            match cursor.next() {
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            }
        }
    }
});
