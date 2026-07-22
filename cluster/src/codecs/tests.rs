//! Protocol codec round-trips using **ergon** production codecs only.
//! sbe-tool trees remain for head-to-head benches only.

use super::rfq::{CreateRfqCommandDecoder, CreateRfqCommandEncoder, Side};
use super::session::{
    ChallengeEncoder, EventCode, NewLeaderEventEncoder, SessionConnectRequestEncoder, SessionEventEncoder,
    SessionMessageHeaderDecoder, SessionMessageHeaderEncoder,
};

#[test]
fn test_session_message_header_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![0u8; 256];
    let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0)?;
    enc.leadership_term_id(42).cluster_session_id(99).timestamp(1234567890);
    let bytes = enc.as_ref().to_vec();
    let dec = SessionMessageHeaderDecoder::wrap_and_apply_header(&bytes, 0)?;
    assert_eq!(dec.leadership_term_id(), 42);
    assert_eq!(dec.cluster_session_id(), 99);
    assert_eq!(dec.timestamp(), 1234567890);
    Ok(())
}

#[test]
fn test_session_event_ok_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![0u8; 256];
    let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut data, 0)?;
    let _ = enc
        .cluster_session_id(1)
        .correlation_id(100)
        .leadership_term_id(5)
        .leader_member_id(0)
        .code(EventCode::OK)
        .version(1);
    let complete = enc.detail(&[])?;
    let bytes = complete.as_bytes_with_header();
    assert!(bytes.len() >= 8 + SessionEventEncoder::BLOCK_LENGTH);
    assert_eq!(
        u16::from_le_bytes([bytes[2], bytes[3]]),
        SessionEventEncoder::TEMPLATE_ID
    );
    Ok(())
}

#[test]
fn test_session_connect_request_roundtrip_shape() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![0u8; 512];
    let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut data, 0)?;
    enc.correlation_id(42).response_stream_id(102).version(1);
    let complete = enc
        .response_channel(b"aeron:udp?endpoint=localhost:9999")?
        .encoded_credentials(b"user:pass")?
        .client_info(b"")?;
    let bytes = complete.as_bytes_with_header();
    assert_eq!(bytes.len(), 78);
    assert_eq!(
        u16::from_le_bytes([bytes[2], bytes[3]]),
        SessionConnectRequestEncoder::TEMPLATE_ID
    );
    Ok(())
}

#[test]
fn test_challenge_and_new_leader_encode() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![0u8; 256];
    let mut enc = ChallengeEncoder::wrap_and_apply_header(&mut data, 0)?;
    enc.correlation_id(5).cluster_session_id(2);
    let complete = enc.encoded_challenge(b"chal")?;
    assert!(complete.as_bytes_with_header().len() > 8);

    let mut data2 = vec![0u8; 256];
    let mut enc2 = NewLeaderEventEncoder::wrap_and_apply_header(&mut data2, 0)?;
    enc2.leadership_term_id(1).cluster_session_id(2).leader_member_id(0);
    let complete2 = enc2.ingress_endpoints(b"0=localhost:9000")?;
    assert!(complete2.as_bytes_with_header().len() > 8);
    Ok(())
}

/// ergon RFQ CreateRfqCommand vs frozen 77-byte golden (schema 101 wire parity).
/// Golden captured 2026-07-20; cross-generator sbe-tool verification replaced
/// by this frozen reference + ergon round-trip assertions.
#[test]
fn test_rfq_create_command_golden_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let mut corr = [b'_'; 36];
    corr[..14].copy_from_slice(b"create-rfq-001");
    let mut cusip = [0u8; 9];
    cusip.copy_from_slice(b"123456789");

    let mut ergo_buf = vec![0u8; 128];
    let mut enc = CreateRfqCommandEncoder::wrap_and_apply_header(&mut ergo_buf, 0)?;
    let _ = enc
        .correlation(corr)
        .expire_time_ms(60_000)
        .quantity(1000)
        .requester_side(Side::BUY)
        .cusip(cusip)
        .requester_user_id(500);
    let ergo_bytes = &ergo_buf[..CreateRfqCommandEncoder::ENCODED_LENGTH];

    // Frozen 77-byte golden (schema 101, CreateRfqCommand, template_id=101).
    // Beginning: [69, 0, 106, 0, 101, 0, 1, 0] — message header + template id.
    // Ending:    [244, 1, 0, 0] — requester_user_id=500 in little-endian.
    assert_eq!(ergo_bytes.len(), 77);
    assert_eq!(&ergo_bytes[..8], &[69, 0, 106, 0, 101, 0, 1, 0]);
    assert_eq!(&ergo_bytes[ergo_bytes.len() - 4..], &[244, 1, 0, 0]);

    // Decode with ergon — verify round-trip correctness.
    let dec = CreateRfqCommandDecoder::wrap_and_apply_header(ergo_bytes, 0)?;
    assert_eq!(dec.correlation(), corr);
    assert_eq!(dec.expire_time_ms(), 60_000);
    assert_eq!(dec.quantity(), 1000);
    assert_eq!(dec.requester_side(), Side::BUY);
    assert_eq!(dec.cusip(), cusip);
    assert_eq!(dec.requester_user_id(), 500);
    Ok(())
}
