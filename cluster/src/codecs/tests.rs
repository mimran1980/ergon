//! Protocol codec round-trips using **ergon** production codecs only.
//! sbe-tool trees remain for head-to-head benches only.

use super::session::{
    ChallengeEncoder, EventCode, NewLeaderEventEncoder, SessionConnectRequestEncoder, SessionEventEncoder,
    SessionMessageHeaderDecoder, SessionMessageHeaderEncoder,
};

#[test]
fn test_session_message_header_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 256];
    let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0);
    enc.leadership_term_id(42).cluster_session_id(99).timestamp(1234567890);
    let bytes = enc.as_ref().to_vec();
    let dec = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&bytes, 0)?;
    assert_eq!(dec.leadership_term_id(), 42);
    assert_eq!(dec.cluster_session_id(), 99);
    assert_eq!(dec.timestamp(), 1234567890);
    Ok(())
}

#[test]
fn test_session_event_ok_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 256];
    let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut data, 0);
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
    let mut data = [0u8; 512];
    let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut data, 0);
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
    let mut data = [0u8; 256];
    let mut enc = ChallengeEncoder::wrap_and_apply_header(&mut data, 0);
    enc.correlation_id(5).cluster_session_id(2);
    let complete = enc.encoded_challenge(b"chal")?;
    assert!(complete.as_bytes_with_header().len() > 8);

    let mut data2 = [0u8; 256];
    let mut enc2 = NewLeaderEventEncoder::wrap_and_apply_header(&mut data2, 0);
    enc2.leadership_term_id(1).cluster_session_id(2).leader_member_id(0);
    let complete2 = enc2.ingress_endpoints(b"0=localhost:9000")?;
    assert!(complete2.as_bytes_with_header().len() > 8);
    Ok(())
}
