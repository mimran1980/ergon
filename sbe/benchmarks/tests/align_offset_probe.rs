//! Fixed64 encode/decode at non-zero absolute offsets (`alignment_bench` contract).
#![allow(clippy::unwrap_used, clippy::panic)]
use ergo_sbe_benchmarks::codec_matrix::{Fixed64Decoder, Fixed64Encoder, Fixed64FixedFields};

fn encode_at(buffer: &mut [u8], offset: usize) -> usize {
    Fixed64Encoder::try_wrap_and_apply_header(buffer, offset)
        .unwrap()
        .fixed(&Fixed64FixedFields {
            value: 0x0102_0304_0506_0708,
            payload: [0x5a; 56],
        })
        .encoded_length_with_header()
}

#[test]
fn encode_decode_all_offsets_0_to_63() {
    for offset in 0usize..=63 {
        let mut storage = [0u8; 512];
        let len = encode_at(&mut storage, offset);
        assert_eq!(len, Fixed64Encoder::ENCODED_LENGTH, "offset={offset}");
        let dec = Fixed64Decoder::try_wrap_and_apply_header(&storage[..offset + len], offset)
            .unwrap_or_else(|e| panic!("offset={offset} decode: {e:?}"));
        assert_eq!(
            dec.value(),
            0x0102_0304_0506_0708,
            "offset={offset} value mismatch"
        );
        assert_eq!(
            dec.payload(),
            [0x5a; 56],
            "offset={offset} payload mismatch"
        );
    }
}
