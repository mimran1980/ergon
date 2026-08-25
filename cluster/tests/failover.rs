#![allow(missing_docs)]
//! Failover / redirect tests.
//!
//! Unit cases pin T-16 redirect
//! resolution: missing or malformed leaders become `ReconnectFailed` shapes
//! immediately (no silent timeout loop).

use ergo_aeron_cluster::cluster_codec_types::{EventCode, SessionEventEncoder};
use ergo_aeron_cluster::poller::parse_leader_endpoint;
use ergo_aeron_cluster::{ClusterError, EgressEvent, parse_event};

/// T-16: redirect detail without the named leader member → ReconnectFailed shape.
#[test]
fn redirect_missing_leader_is_reconnect_failed() -> Result<(), Box<dyn std::error::Error>> {
    let detail = "0=localhost:9012,2=localhost:9212";
    let leader_member_id = 1;
    let err = parse_leader_endpoint(detail, leader_member_id)?
        .ok_or_else(|| ClusterError::ReconnectFailed {
            reason: format!("connect redirect listed no endpoint for leader member {leader_member_id}: {detail}"),
        })
        .expect_err("missing leader");
    assert!(matches!(err, ClusterError::ReconnectFailed { .. }));
    Ok(())
}

/// T-16: malformed endpoint map (no id=host entries) is a distinct `Err`,
/// not silently collapsed into "no leader" alongside a well-formed map.
#[test]
fn redirect_malformed_endpoints_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parse_leader_endpoint("not-a-map", 0).is_err());
    assert!(parse_leader_endpoint("=,=foo", 0).is_err());
    assert!(parse_leader_endpoint("abc=def", 0).is_err()); // non-numeric id
    Ok(())
}

/// T-16: a REDIRECT SessionEvent still decodes; resolution is a separate step.
#[test]
fn redirect_session_event_decodes_then_resolution_fails() -> Result<(), Box<dyn std::error::Error>> {
    let detail = b"0=localhost:9012"; // leader 1 absent
    let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
    let mut buf = vec![0u8; len];
    let complete = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&ergo_aeron_cluster::cluster_codec_types::SessionEventFixedFields {
            cluster_session_id: 9,
            correlation_id: 1,
            leadership_term_id: 2,
            leader_member_id: 1,
            code: EventCode::REDIRECT,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(detail)?;
    let bytes = complete.as_bytes_with_header();
    match parse_event(bytes)? {
        EgressEvent::SessionEvent {
            code,
            leader_member_id,
            detail,
            ..
        } => {
            assert_eq!(code, EventCode::REDIRECT);
            assert_eq!(leader_member_id, 1);
            assert_eq!(
                parse_leader_endpoint(&detail, leader_member_id)?,
                None,
                "leader 1 must be absent from {detail}"
            );
        }
        other => panic!("expected REDIRECT SessionEvent, got {other:?}"),
    }
    Ok(())
}
