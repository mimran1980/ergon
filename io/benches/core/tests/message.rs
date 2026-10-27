//! The 64-byte message every arm sends: its exact bytes, and what decoding
//! refuses.

use ergon_latency_core::Error;
use ergon_latency_core::message::{Kind, MESSAGE_LEN, Message, echo_in_place, template};

type R = Result<(), Box<dyn std::error::Error>>;

const SAMPLE: Message = Message {
    seq: 0x0102_0304_0506_0708,
    send_tsc: 0x1112_1314_1516_1718,
    intended_tsc: 0x2122_2324_2526_2728,
    stream: 0x3132_3334,
    kind: Kind::Oneway,
};

/// `seq` u64 LE @0, `send_tsc` @8, `intended_tsc` @16, `stream` u32 @24, `kind` u32 @28,
/// 32..64 zero.
#[rustfmt::skip]
const GOLDEN: [u8; MESSAGE_LEN] = [
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
    0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11,
    0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21,
    0x34, 0x33, 0x32, 0x31,
    0x02, 0x00, 0x00, 0x00,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[test]
fn the_message_encodes_to_the_golden_bytes() -> R {
    assert_eq!(SAMPLE.to_bytes(), GOLDEN);
    let mut out = [0xffu8; 80];
    assert_eq!(SAMPLE.encode(&mut out)?, MESSAGE_LEN);
    assert_eq!(out[..MESSAGE_LEN], GOLDEN);
    // past the message, the buffer is left alone
    assert!(out[MESSAGE_LEN..].iter().all(|&b| b == 0xff));
    assert_eq!(Message::decode(&GOLDEN)?, SAMPLE);
    Ok(())
}

#[test]
fn the_kinds_are_one_two_three_so_a_zeroed_buffer_is_refused() {
    assert_eq!(Kind::Ping as u32, 1);
    assert_eq!(Kind::Oneway as u32, 2);
    assert_eq!(Kind::Echo as u32, 3);
    assert!(matches!(
        Message::decode(&[0u8; MESSAGE_LEN]),
        Err(Error::MessageKind(0))
    ));
    let mut bad = GOLDEN;
    bad[28] = 4;
    assert!(matches!(Message::decode(&bad), Err(Error::MessageKind(4))));
}

#[test]
fn a_short_long_or_padded_buffer_is_refused() {
    assert!(matches!(
        Message::decode(&GOLDEN[..63]),
        Err(Error::MessageLength(63))
    ));
    let mut long = [0u8; 65];
    long[..MESSAGE_LEN].copy_from_slice(&GOLDEN);
    assert!(matches!(
        Message::decode(&long),
        Err(Error::MessageLength(65))
    ));
    let mut padded = GOLDEN;
    padded[63] = 1;
    assert!(matches!(Message::decode(&padded), Err(Error::MessageTail)));
    let mut short = [0u8; 63];
    assert!(matches!(
        SAMPLE.encode(&mut short),
        Err(Error::MessageLength(63))
    ));
}

#[test]
fn an_echo_rewrites_only_the_kind_of_a_ping() -> R {
    let ping = Message {
        kind: Kind::Ping,
        ..SAMPLE
    };
    let mut buf = ping.to_bytes();
    let echoed = echo_in_place(&mut buf)?;
    assert_eq!(
        echoed,
        Message {
            kind: Kind::Echo,
            ..SAMPLE
        }
    );
    let mut want = ping.to_bytes();
    want[28] = 3;
    assert_eq!(buf, want);
    // an echo or a one-way message is never echoed
    assert!(matches!(
        echo_in_place(&mut buf),
        Err(Error::NotAPing(Kind::Echo))
    ));
    let mut oneway = SAMPLE.to_bytes();
    assert!(matches!(
        echo_in_place(&mut oneway),
        Err(Error::NotAPing(Kind::Oneway))
    ));
    assert_eq!(oneway, GOLDEN);
    Ok(())
}

#[test]
fn the_template_is_a_zero_ping() -> R {
    let t = template();
    assert_eq!(
        Message::decode(&t)?,
        Message {
            seq: 0,
            send_tsc: 0,
            intended_tsc: 0,
            stream: 0,
            kind: Kind::Ping
        }
    );
    Ok(())
}
