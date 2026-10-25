//! The u32 LE length frame both TCP arms write: its bytes, and decoding from
//! any split of the stream.

use ergon_latency_core::Error;
use ergon_latency_core::frame::{Deframer, FRAME_HEADER, decode, encode, header};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_frame_is_a_u32_le_length_then_the_payload() -> R {
    let mut out = [0u8; 16];
    assert_eq!(encode(b"abc", &mut out)?, FRAME_HEADER + 3);
    assert_eq!(&out[..7], &[3, 0, 0, 0, b'a', b'b', b'c']);
    assert_eq!(header(0x0102_0304)?, [4, 3, 2, 1]);
    assert!(matches!(
        encode(b"abc", &mut out[..6]),
        Err(Error::FrameBuffer { need: 7, have: 6 })
    ));
    Ok(())
}

#[test]
fn decode_waits_for_a_whole_frame() -> R {
    let mut out = [0u8; 16];
    let n = encode(b"hello", &mut out)?;
    for cut in 0..n {
        assert_eq!(decode(&out[..cut], 64)?, None, "cut at {cut}");
    }
    assert_eq!(decode(&out[..n], 64)?, Some((&b"hello"[..], n)));
    Ok(())
}

#[test]
fn an_oversized_length_is_refused_before_its_payload_arrives() {
    let len = header(65).unwrap_or_default();
    assert!(matches!(
        decode(&len, 64),
        Err(Error::FrameTooLarge { len: 65, max: 64 })
    ));
}

#[test]
fn the_deframer_yields_every_frame_from_any_split() -> R {
    let payloads: [&[u8]; 4] = [b"one", b"", b"three33", &[7u8; 64]];
    let mut stream = Vec::new();
    for p in payloads {
        let mut buf = [0u8; 80];
        let n = encode(p, &mut buf)?;
        stream.extend_from_slice(&buf[..n]);
    }
    for chunk in 1..=stream.len() {
        let mut d = Deframer::new(64)?;
        let mut got: Vec<Vec<u8>> = Vec::new();
        for part in stream.chunks(chunk) {
            let mut rest = part;
            while !rest.is_empty() {
                let took = d.push(rest)?;
                rest = &rest[took..];
                while let Some(frame) = d.next_frame()? {
                    got.push(frame.to_vec());
                }
            }
        }
        let want: Vec<Vec<u8>> = payloads.iter().map(|p| p.to_vec()).collect();
        assert_eq!(got, want, "chunk {chunk}");
        assert_eq!(d.buffered(), 0);
    }
    Ok(())
}

#[test]
fn the_deframer_reads_straight_into_its_spare_room() -> R {
    let mut d = Deframer::new(64)?;
    let mut wire = [0u8; 16];
    let n = encode(b"xyz", &mut wire)?;
    let spare = d.spare();
    assert!(spare.len() >= n);
    spare[..n].copy_from_slice(&wire[..n]);
    d.filled(n)?;
    assert_eq!(d.next_frame()?, Some(&b"xyz"[..]));
    assert_eq!(d.next_frame()?, None);
    let room = d.spare().len();
    assert!(matches!(d.filled(room + 1), Err(Error::FrameBuffer { .. })));
    Ok(())
}
