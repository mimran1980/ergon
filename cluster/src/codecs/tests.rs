//! Protocol codec round-trips using **ErgoSBE** production codecs only.
//! sbe-tool trees remain for head-to-head benches only.

use super::ergo_codecs::{
    ChallengeEncoder, EventCode, NewLeaderEventEncoder, SessionConnectRequestEncoder, SessionEventEncoder,
    SessionMessageHeaderDecoder, SessionMessageHeaderEncoder,
};
use super::ergo_rfq_codecs::{CreateRfqCommandDecoder, CreateRfqCommandEncoder, Side};

#[test]
fn test_session_message_header_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = vec![0u8; 256];
    let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0)?;
    let _ = enc.leadership_term_id(42).cluster_session_id(99).timestamp(1234567890);
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
    let _ = enc.correlation_id(42).response_stream_id(102).version(1);
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
    let _ = enc.correlation_id(5).cluster_session_id(2);
    let complete = enc.encoded_challenge(b"chal")?;
    assert!(complete.as_bytes_with_header().len() > 8);

    let mut data2 = vec![0u8; 256];
    let mut enc2 = NewLeaderEventEncoder::wrap_and_apply_header(&mut data2, 0)?;
    let _ = enc2.leadership_term_id(1).cluster_session_id(2).leader_member_id(0);
    let complete2 = enc2.ingress_endpoints(b"0=localhost:9000")?;
    assert!(complete2.as_bytes_with_header().len() > 8);
    Ok(())
}

/// ErgoSBE RFQ CreateRfqCommand vs residual sbe-tool bytes (wire parity).
#[test]
fn test_rfq_create_command_ergo_matches_sbe_tool() -> Result<(), Box<dyn std::error::Error>> {
    let mut corr = [b'_'; 36];
    corr[..14].copy_from_slice(b"create-rfq-001");
    let mut cusip = [0u8; 9];
    cusip.copy_from_slice(b"123456789");

    // ErgoSBE production encoder
    let mut ergo_buf = vec![0u8; 128];
    let mut enc = CreateRfqCommandEncoder::wrap_and_apply_header(&mut ergo_buf, 0)?;
    let _ = enc
        .correlation(corr)
        .expire_time_ms(60_000)
        .quantity(1000)
        .requester_side(Side::BUY)
        .cusip(cusip)
        .requester_user_id(500);
    let ergo_bytes = enc.as_ref()[..CreateRfqCommandEncoder::ENCODED_LENGTH].to_vec();

    // sbe-tool residual encoder (same schema 101)
    use super::rfq_codecs::{
        WriteBuf,
        create_rfq_command_codec::{self, CreateRfqCommandEncoder as SbeCreate},
        side::Side as SbeSide,
    };
    let mut sbe_buf = vec![0u8; 128];
    {
        let wb = WriteBuf::new(&mut sbe_buf);
        let mut enc = SbeCreate::default().wrap(wb, 8);
        enc.correlation(&corr);
        enc.expire_time_ms(60_000);
        enc.quantity(1000);
        enc.requester_side(SbeSide::BUY);
        enc.cusip(&cusip);
        enc.requester_user_id(500);
        let _ = enc.header(0);
    }
    let sbe_len = 8 + create_rfq_command_codec::SBE_BLOCK_LENGTH as usize;
    let sbe_bytes = &sbe_buf[..sbe_len];

    assert_eq!(
        ergo_bytes.as_slice(),
        sbe_bytes,
        "ErgoSBE RFQ CreateRfqCommand must match sbe-tool wire bytes"
    );

    // Decode with ErgoSBE
    let dec = CreateRfqCommandDecoder::wrap_and_apply_header(&ergo_bytes, 0)?;
    assert_eq!(dec.correlation(), corr);
    assert_eq!(dec.expire_time_ms(), 60_000);
    assert_eq!(dec.quantity(), 1000);
    assert_eq!(dec.requester_side(), Side::BUY);
    assert_eq!(dec.cusip(), cusip);
    assert_eq!(dec.requester_user_id(), 500);
    Ok(())
}
