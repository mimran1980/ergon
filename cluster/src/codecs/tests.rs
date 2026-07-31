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
    assert!(dec.remaining().is_empty(), "remaining must be empty after full decode");
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

/// When a SessionMessageHeader is encoded alone (no app payload),
/// `remaining()` on the decoder must return an empty slice.
#[test]
fn test_remaining_empty_without_payload() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
    SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
        .leadership_term_id(7)
        .cluster_session_id(42)
        .timestamp(100);

    let dec = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&buf, 0)?;
    assert_eq!(dec.leadership_term_id(), 7);
    assert_eq!(dec.cluster_session_id(), 42);
    assert_eq!(dec.timestamp(), 100);
    // No payload — remaining must be empty
    assert!(
        dec.remaining().is_empty(),
        "remaining must be empty when there is no payload"
    );
    Ok(())
}

/// When a SessionMessageHeader is followed by payload bytes,
/// `remaining()` returns exactly those bytes.
#[test]
fn test_remaining_returns_payload_after_header() -> Result<(), Box<dyn std::error::Error>> {
    let payload: &[u8] = b"hello-world-payload";
    let total = SessionMessageHeaderEncoder::ENCODED_LENGTH + payload.len();
    let mut buf = vec![0u8; total];

    SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
        .leadership_term_id(1)
        .cluster_session_id(2)
        .timestamp(3);
    buf[SessionMessageHeaderEncoder::ENCODED_LENGTH..].copy_from_slice(payload);

    let dec = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&buf, 0)?;
    assert_eq!(dec.remaining(), payload);
    Ok(())
}

/// `whole_buffer()` returns the entire original frame (header + payload).
#[test]
fn test_whole_buffer_returns_entire_frame() -> Result<(), Box<dyn std::error::Error>> {
    let payload: &[u8] = b"data";
    let total = SessionMessageHeaderEncoder::ENCODED_LENGTH + payload.len();
    let mut buf = vec![0u8; total];

    SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
        .leadership_term_id(1)
        .cluster_session_id(2)
        .timestamp(3);
    buf[SessionMessageHeaderEncoder::ENCODED_LENGTH..].copy_from_slice(payload);

    let dec = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&buf, 0)?;
    assert_eq!(dec.whole_buffer().len(), total);
    assert_eq!(dec.whole_buffer(), buf.as_slice());
    Ok(())
}

/// After decoding a SessionMessageHeader, the remaining bytes can be decoded
/// via [`AnyMessage`] (they contain another SBE message with its own header).
#[test]
fn test_any_message_decode_chain_from_remaining() -> Result<(), Box<dyn std::error::Error>> {
    use super::session::{AnyMessage, SessionKeepAliveEncoder};

    // Build a buffer: SessionMessageHeader (32 bytes) + SessionKeepAlive (32 bytes).
    // Both lengths are const — use a stack array.
    let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH + SessionKeepAliveEncoder::ENCODED_LENGTH];

    // First: SessionMessageHeader
    let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.leadership_term_id(7).cluster_session_id(99).timestamp(42);

    // remaining_mut() gives the unwritten region — chain the next encoder
    SessionKeepAliveEncoder::wrap_and_apply_header(enc.remaining_mut(), 0)
        .leadership_term_id(7)
        .cluster_session_id(99);

    // Decode the first message (SessionMessageHeader)
    let smh = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&buf, 0)?;
    assert_eq!(smh.cluster_session_id(), 99);

    // remaining() is the SessionKeepAlive bytes
    let tail = smh.remaining();
    assert_eq!(tail.len(), SessionKeepAliveEncoder::ENCODED_LENGTH);

    // Decode the second message via AnyMessage
    let msg = AnyMessage::decode(tail, 0).map_err(|e| format!("AnyMessage decode: {e}"))?;
    match msg {
        AnyMessage::SessionKeepAlive(dec) => {
            assert_eq!(dec.leadership_term_id(), 7);
            assert_eq!(dec.cluster_session_id(), 99);
            // After fully decoding, remaining should be empty
            assert!(dec.remaining().is_empty(), "remaining after keep-alive must be empty");
        }
        _other => panic!("expected SessionKeepAlive"),
    }
    Ok(())
}
