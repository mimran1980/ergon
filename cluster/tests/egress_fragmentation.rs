#![allow(missing_docs)]
//! Fail-closed `parse_event` coverage (no Java harness). Live reassembly
//! lives in `ergo-aeron-cluster-test-harness`.

use ergo_aeron_cluster::cluster_codec_types::{EventCode, SessionEventEncoder};
use ergo_aeron_cluster::{ClusterError, EgressEvent, parse_event};

/// T-16: canonical `parse_event` path rejects short/truncated/invalid frames
/// without a live cluster (sync + async handshakes share this projection).
#[test]
fn parse_event_fail_closed_short_and_truncated() -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        matches!(parse_event(&[0u8; 4]), Err(ClusterError::ProtocolError { .. })),
        "short header must be ProtocolError"
    );

    let detail = b"ok";
    let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
    let mut buf = vec![0u8; len];
    let complete = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&ergo_aeron_cluster::cluster_codec_types::SessionEventFixedFields {
            cluster_session_id: 1,
            correlation_id: 2,
            leadership_term_id: 3,
            leader_member_id: 0,
            code: EventCode::OK,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(detail)?;
    let full = complete.as_bytes_with_header();
    // Drop tail so var-data length or body is incomplete.
    let cut = &full[..full.len().saturating_sub(2).max(8)];
    assert!(
        matches!(parse_event(cut), Err(ClusterError::ProtocolError { .. })),
        "truncated SessionEvent must be ProtocolError"
    );
    Ok(())
}

#[test]
fn parse_event_unknown_template_is_other_not_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let mut frame = vec![0u8; 24];
    frame[0] = 16;
    frame[2] = 0xAB;
    frame[3] = 0xCD;
    frame[4] = 111; // schema id (cluster codecs)
    match parse_event(&frame)? {
        EgressEvent::Other { template_id } => assert_eq!(template_id, 0xCDAB),
        other => panic!("expected Other, got {other:?}"),
    }
    Ok(())
}
