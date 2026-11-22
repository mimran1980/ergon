//! Protocol codec round-trips using **ergon** production codecs only.
//! sbe-tool trees remain for head-to-head benches only.

use super::session::{
    ChallengeEncoder, ChallengeFixedFields, ClusterAction, ClusterActionRequestDecoder, ClusterActionRequestEncoder,
    ClusterActionRequestFixedFields, ClusterTimeUnit, EventCode, NewLeaderEventEncoder, NewLeaderEventFixedFields,
    SessionConnectRequestEncoder, SessionConnectRequestFixedFields, SessionEventEncoder, SessionEventFixedFields,
    SessionMessageHeaderDecoder, SessionMessageHeaderEncoder, SessionMessageHeaderFixedFields, SnapshotMark,
    SnapshotMarkerDecoder, SnapshotMarkerEncoder, SnapshotMarkerFixedFields,
};

#[test]
fn test_session_message_header_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 256];
    let enc =
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0).fixed(&SessionMessageHeaderFixedFields {
            leadership_term_id: 42,
            cluster_session_id: 99,
            timestamp: 1_234_567_890,
        });
    let bytes = enc.as_bytes_with_header().to_vec();
    let dec = SessionMessageHeaderDecoder::decode(&bytes, 0)?;
    assert_eq!(dec.leadership_term_id(), 42);
    assert_eq!(dec.cluster_session_id(), 99);
    assert_eq!(dec.timestamp(), 1_234_567_890);
    assert!(
        dec.get_metadata().remaining().is_empty(),
        "remaining must be empty after full decode"
    );
    Ok(())
}

#[test]
fn test_session_event_ok_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 256];
    let complete = SessionEventEncoder::wrap_and_apply_header(&mut data, 0)
        .fixed(&SessionEventFixedFields {
            cluster_session_id: 1,
            correlation_id: 100,
            leadership_term_id: 5,
            leader_member_id: 0,
            code: EventCode::OK,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(&[])?;
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
    let complete = SessionConnectRequestEncoder::wrap_and_apply_header(&mut data, 0)
        .fixed(&SessionConnectRequestFixedFields {
            correlation_id: 42,
            response_stream_id: 102,
            version: Some(1),
        })
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
    let complete = ChallengeEncoder::wrap_and_apply_header(&mut data, 0)
        .fixed(&ChallengeFixedFields {
            correlation_id: 5,
            cluster_session_id: 2,
        })
        .encoded_challenge(b"chal")?;
    assert!(complete.as_bytes_with_header().len() > 8);

    let mut data2 = [0u8; 256];
    let complete2 = NewLeaderEventEncoder::wrap_and_apply_header(&mut data2, 0)
        .fixed(&NewLeaderEventFixedFields {
            leadership_term_id: 1,
            cluster_session_id: 2,
            leader_member_id: 0,
        })
        .ingress_endpoints(b"0=localhost:9000")?;
    assert!(complete2.as_bytes_with_header().len() > 8);
    Ok(())
}

/// When a `SessionMessageHeader` is encoded alone (no app payload),
/// `remaining()` on the decoder must return an empty slice.
#[test]
fn test_remaining_empty_without_payload() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
    SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
        .leadership_term_id(7)
        .cluster_session_id(42)
        .timestamp(100);

    let dec = SessionMessageHeaderDecoder::decode(&buf, 0)?;
    assert_eq!(dec.leadership_term_id(), 7);
    assert_eq!(dec.cluster_session_id(), 42);
    assert_eq!(dec.timestamp(), 100);
    // No payload, so nothing remains.
    assert!(
        dec.get_metadata().remaining().is_empty(),
        "remaining must be empty when there is no payload"
    );
    Ok(())
}

/// When a `SessionMessageHeader` is followed by payload bytes,
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

    let dec = SessionMessageHeaderDecoder::decode(&buf, 0)?;
    assert_eq!(dec.get_metadata().remaining(), payload);
    Ok(())
}

/// `get_metadata().buffer()` returns the entire original frame (header + payload).
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

    let dec = SessionMessageHeaderDecoder::decode(&buf, 0)?;
    let meta = dec.get_metadata();
    assert_eq!(meta.buffer().len(), total);
    assert_eq!(meta.buffer(), buf.as_slice());
    Ok(())
}

/// After decoding a `SessionMessageHeader`, the remaining bytes can be decoded
/// via [`AnyMessage`] (they contain another SBE message with its own header).
#[test]
fn test_any_message_decode_chain_from_remaining() -> Result<(), Box<dyn std::error::Error>> {
    use super::session::{AnyMessage, SessionKeepAliveEncoder};

    // Build a buffer: SessionMessageHeader (32 bytes) + SessionKeepAlive (32 bytes).
    // Both lengths are constants, so a stack array fits.
    let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH + SessionKeepAliveEncoder::ENCODED_LENGTH];

    // First: SessionMessageHeader
    let enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0).fixed(&SessionMessageHeaderFixedFields {
        leadership_term_id: 7,
        cluster_session_id: 99,
        timestamp: 42,
    });

    // `into_remaining_mut()` gives the unwritten region for the next encoder.
    SessionKeepAliveEncoder::wrap_and_apply_header(enc.into_remaining_mut(), 0)
        .leadership_term_id(7)
        .cluster_session_id(99);

    // Decode the first message (SessionMessageHeader)
    let smh = SessionMessageHeaderDecoder::decode(&buf, 0)?;
    assert_eq!(smh.cluster_session_id(), 99);

    // get_metadata().remaining() is the SessionKeepAlive bytes
    let tail = smh.get_metadata().remaining();
    assert_eq!(tail.len(), SessionKeepAliveEncoder::ENCODED_LENGTH);

    // Decode the second message via AnyMessage
    let msg = AnyMessage::decode(tail, 0).map_err(|e| format!("AnyMessage decode: {e}"))?;
    match msg {
        AnyMessage::SessionKeepAlive(dec) => {
            assert_eq!(dec.leadership_term_id(), 7);
            assert_eq!(dec.cluster_session_id(), 99);
            // After fully decoding, remaining should be empty
            assert!(
                dec.get_metadata().remaining().is_empty(),
                "remaining after keep-alive must be empty"
            );
        }
        _other => panic!("expected SessionKeepAlive"),
    }
    Ok(())
}

// Schema 111 v18 (Aeron 1.54.2) adds the optional `termBaseLogPosition`
// (`position_t`, null -1) to ClusterActionRequest and SnapshotMarker. These
// goldens are the Java 1.54.2 encoders' output (aeron-all-1.54.2.jar) for the
// same inputs as the tests below.
const JAVA_CLUSTER_ACTION_REQUEST: [u8; 48] = [
    0x28, 0x00, 0x17, 0x00, 0x6f, 0x00, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf1, 0x53, 0x65, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
// flags and termBaseLogPosition null: i32::MIN and -1, not zero.
const JAVA_CLUSTER_ACTION_REQUEST_NULLS: [u8; 48] = [
    0x28, 0x00, 0x17, 0x00, 0x6f, 0x00, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf1, 0x53, 0x65, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x80, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];
const JAVA_SNAPSHOT_MARKER: [u8; 56] = [
    0x30, 0x00, 0x64, 0x00, 0x6f, 0x00, 0x12, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn cluster_action_request_fields(flags: Option<i32>, term_base: Option<i64>) -> ClusterActionRequestFixedFields {
    ClusterActionRequestFixedFields {
        leadership_term_id: 3,
        log_position: 1024,
        timestamp: 1_700_000_000,
        action: ClusterAction::SNAPSHOT,
        flags,
        term_base_log_position: term_base,
    }
}

/// Rewrites the header as an older sender's (block length, version) and drops
/// the fields that sender never wrote.
fn as_older_sender(frame: &[u8], block_length: u16, version: u16) -> Vec<u8> {
    let mut old = frame[..8 + usize::from(block_length)].to_vec();
    old[0..2].copy_from_slice(&block_length.to_le_bytes());
    old[6..8].copy_from_slice(&version.to_le_bytes());
    old
}

#[test]
fn cluster_action_request_v18_matches_java_and_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 64];
    let enc = ClusterActionRequestEncoder::wrap_and_apply_header(&mut data, 0)
        .fixed(&cluster_action_request_fields(Some(0), Some(512)));
    assert_eq!(enc.as_bytes_with_header(), &JAVA_CLUSTER_ACTION_REQUEST[..]);

    let dec = ClusterActionRequestDecoder::decode(&JAVA_CLUSTER_ACTION_REQUEST, 0)?;
    assert_eq!(dec.leadership_term_id(), 3);
    assert_eq!(dec.log_position(), 1024);
    assert_eq!(dec.action(), ClusterAction::SNAPSHOT);
    assert_eq!(dec.flags(), Some(0));
    assert_eq!(dec.term_base_log_position(), Some(512));
    Ok(())
}

#[test]
fn cluster_action_request_absent_term_base_writes_java_null() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 64];
    let enc = ClusterActionRequestEncoder::wrap_and_apply_header(&mut data, 0)
        .fixed(&cluster_action_request_fields(None, None));
    assert_eq!(enc.as_bytes_with_header(), &JAVA_CLUSTER_ACTION_REQUEST_NULLS[..]);

    let dec = ClusterActionRequestDecoder::decode(&JAVA_CLUSTER_ACTION_REQUEST_NULLS, 0)?;
    assert_eq!(dec.flags(), None);
    assert_eq!(dec.term_base_log_position(), None);

    // 0 is a real position, not the null.
    let mut zero = JAVA_CLUSTER_ACTION_REQUEST;
    zero[40..48].copy_from_slice(&0_i64.to_le_bytes());
    assert_eq!(
        ClusterActionRequestDecoder::decode(&zero, 0)?.term_base_log_position(),
        Some(0)
    );
    Ok(())
}

#[test]
fn cluster_action_request_from_v17_sender_has_no_term_base() -> Result<(), Box<dyn std::error::Error>> {
    let old = as_older_sender(&JAVA_CLUSTER_ACTION_REQUEST, 32, 17);
    let dec = ClusterActionRequestDecoder::decode(&old, 0)?;
    assert_eq!(dec.leadership_term_id(), 3);
    assert_eq!(dec.log_position(), 1024);
    assert_eq!(dec.timestamp(), 1_700_000_000);
    assert_eq!(dec.action(), ClusterAction::SNAPSHOT);
    assert_eq!(dec.flags(), Some(0));
    assert_eq!(dec.term_base_log_position(), None);
    Ok(())
}

#[test]
fn snapshot_marker_v18_matches_java_and_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = [0u8; 64];
    let enc = SnapshotMarkerEncoder::wrap_and_apply_header(&mut data, 0).fixed(&SnapshotMarkerFixedFields {
        type_id: 2,
        log_position: 4096,
        leadership_term_id: 7,
        index: 0,
        mark: SnapshotMark::BEGIN,
        time_unit: Some(ClusterTimeUnit::NANOS),
        app_version: Some(1),
        term_base_log_position: Some(2048),
    });
    assert_eq!(enc.as_bytes_with_header(), &JAVA_SNAPSHOT_MARKER[..]);

    let dec = SnapshotMarkerDecoder::decode(&JAVA_SNAPSHOT_MARKER, 0)?;
    assert_eq!(dec.term_base_log_position(), Some(2048));

    let old = as_older_sender(&JAVA_SNAPSHOT_MARKER, 40, 17);
    let dec = SnapshotMarkerDecoder::decode(&old, 0)?;
    assert_eq!(dec.type_id(), 2);
    assert_eq!(dec.log_position(), 4096);
    assert_eq!(dec.leadership_term_id(), 7);
    assert_eq!(dec.mark(), SnapshotMark::BEGIN);
    assert_eq!(dec.time_unit(), Some(ClusterTimeUnit::NANOS));
    assert_eq!(dec.app_version(), Some(1));
    assert_eq!(dec.term_base_log_position(), None);
    Ok(())
}
