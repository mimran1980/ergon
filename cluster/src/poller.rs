//! A single-fragment poller for the connect handshake. It captures the next
//! `SessionEvent`, `Challenge` or `NewLeaderEvent` so the caller can react.

use crate::ClusterError;
use crate::codecs::session::{EventCode, MessageHeader};

/// An egress event captured by one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressEvent {
    /// A `SessionEvent`: the connect result or a state change.
    SessionEvent {
        /// The connect request's correlation id, echoed back.
        correlation_id: i64,
        /// The cluster-assigned session id.
        cluster_session_id: i64,
        /// The current leadership term.
        leadership_term_id: i64,
        /// The current leader's member id.
        leader_member_id: i32,
        /// The status code (OK, ERROR, REDIRECT, `AUTH_REJECTED`).
        code: EventCode,
        /// The cluster's detail text.
        detail: String,
        /// The leader heartbeat the cluster advertised, when the event has one.
        leader_heartbeat_timeout_ns: Option<i64>,
    },
    /// An authentication challenge.
    Challenge {
        /// The connect request's correlation id, echoed back.
        correlation_id: i64,
        /// The cluster-assigned session id.
        cluster_session_id: i64,
        /// The challenge bytes for the credentials supplier.
        encoded_challenge: Vec<u8>,
    },
    /// A new leader was elected.
    NewLeader {
        /// The cluster-assigned session id.
        cluster_session_id: i64,
        /// The new leadership term.
        leadership_term_id: i64,
        /// The new leader's member id.
        leader_member_id: i32,
        /// The new leader's member-endpoint map.
        ingress_endpoints: String,
    },
    /// A message the handshake does not use.
    Other {
        /// The SBE template id.
        template_id: u16,
    },
}

/// Parses one egress fragment through `Fragment::decode`, which is
/// crate-private and so not linked here.
///
/// The generated `_as_str()` accessors validate the text fields (`detail`,
/// `ingress_endpoints`) as UTF-8. Invalid text returns
/// [`ClusterError::ProtocolError`] rather than a lossy value, so wire input
/// fails closed.
pub fn parse_event(data: &[u8]) -> Result<EgressEvent, ClusterError> {
    use crate::fragment::Fragment;

    // Both the unknown-template path and the not-projected path below report
    // the real template id.
    let template_id = MessageHeader::peek_template_id(data).unwrap_or(0);

    let Some(fragment) = Fragment::decode(data)? else {
        // The cluster may send templates outside our schema.
        return Ok(EgressEvent::Other { template_id });
    };

    Ok(match fragment {
        Fragment::SessionEvent {
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail,
            leader_heartbeat_timeout_ns,
        } => EgressEvent::SessionEvent {
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail: detail.to_string(),
            leader_heartbeat_timeout_ns,
        },
        Fragment::Challenge {
            correlation_id,
            cluster_session_id,
            encoded_challenge,
        } => EgressEvent::Challenge {
            correlation_id,
            cluster_session_id,
            encoded_challenge: encoded_challenge.to_vec(),
        },
        Fragment::NewLeader {
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
        } => EgressEvent::NewLeader {
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ingress_endpoints: ingress_endpoints.to_string(),
        },
        // Decoded, but the handshake has no use for them. Listed instead of
        // `_` so a new `Fragment` variant fails to compile here rather than
        // silently becoming `Other`.
        Fragment::Message { .. } | Fragment::AdminResponse { .. } => EgressEvent::Other { template_id },
    })
}

/// Finds one member's endpoint in a Java endpoints map.
///
/// `NewLeaderEvent.ingress_endpoints` lists every member in id order
/// (`"0=host:port,1=host:port,2=host:port"`). The new leader is the one with
/// `leader_member_id`, not the first entry: a redirect-style parse of the first
/// entry would reconnect to the dead leader.
///
/// The map goes through [`crate::endpoints::parse_ingress_endpoints`], which
/// rejects bad input. A malformed map (missing `=`, non-numeric id, empty
/// endpoint, empty map, duplicate id) is `Err`. A well-formed map without the
/// requested leader is `Ok(None)`, and a match is `Ok(Some(endpoint))`.
pub fn parse_leader_endpoint(endpoints: &str, leader_member_id: i32) -> Result<Option<String>, ClusterError> {
    let parsed = crate::endpoints::parse_ingress_endpoints(endpoints)?;
    Ok(parsed
        .into_iter()
        .find(|e| e.member_id == leader_member_id)
        .map(|e| e.endpoint))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::codecs::session::{
        EventCode, NewLeaderEventEncoder, NewLeaderEventFixedFields, SessionEventEncoder, SessionEventFixedFields,
        SessionMessageHeaderEncoder,
    };

    fn encode_session_event(detail: &[u8], code: EventCode) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
        let mut buf = vec![0u8; len];
        let complete = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&SessionEventFixedFields {
                cluster_session_id: 7,
                correlation_id: 42,
                leadership_term_id: 3,
                leader_member_id: 1,
                code,
                version: Some(1),
                leader_heartbeat_timeout_ns: None,
            })
            .detail(detail)?;
        Ok(complete.as_bytes_with_header().to_vec())
    }

    #[test]
    fn test_parse_event_short_header_is_error() {
        // A frame shorter than the 8-byte SBE header is a protocol error, not "no event".
        let err = parse_event(&[0u8; 4]).expect_err("short header");
        assert!(matches!(err, ClusterError::ProtocolError { .. }), "got {err:?}");
        let msg = err.to_string();
        assert!(msg.contains("too short") || msg.contains("header"), "{msg}");
    }

    #[test]
    fn test_parse_event_empty_is_error() {
        assert!(parse_event(&[]).is_err());
    }

    #[test]
    fn test_parse_event_truncated_body_is_error() -> Result<(), Box<dyn std::error::Error>> {
        // Valid 8-byte header claiming SessionEvent template, but body cut short.
        let full = encode_session_event(b"ok", EventCode::OK)?;
        let truncated = &full[..full.len().saturating_sub(4).max(8)];
        // If truncation still leaves a complete frame, shrink harder to header+partial.
        let truncated = if truncated.len() >= full.len() {
            &full[..8 + 4]
        } else {
            truncated
        };
        let err = parse_event(truncated).expect_err("truncated body");
        assert!(matches!(err, ClusterError::ProtocolError { .. }), "got {err:?}");
        Ok(())
    }

    #[test]
    fn test_parse_event_invalid_utf8_detail_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let bad = encode_session_event(&[0xff, 0xfe, 0xfd], EventCode::OK)?;
        let err = parse_event(&bad).expect_err("invalid utf8 detail");
        assert!(matches!(err, ClusterError::ProtocolError { .. }), "got {err:?}");
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("utf") || msg.contains("detail") || msg.contains("invalid"),
            "{msg}"
        );
        Ok(())
    }

    #[test]
    fn test_parse_event_unknown_template_is_other() -> Result<(), Box<dyn std::error::Error>> {
        // 8-byte header: correct schema id 111, unknown template id.
        let mut hdr = [0u8; 8];
        hdr[0] = 16; // blockLength
        hdr[1] = 0;
        hdr[2] = 0xFE; // template_id low
        hdr[3] = 0xFE; // template_id high → 0xFEFE
        hdr[4] = 111; // schema_id = 111 (cluster codecs)
        hdr[5] = 0;
        hdr[6] = 0; // version
        hdr[7] = 0;
        let mut frame = hdr.to_vec();
        frame.extend_from_slice(&[0u8; 16]);
        let ev = parse_event(&frame)?;
        match ev {
            EgressEvent::Other { template_id } => {
                assert_eq!(template_id, 0xFEFE);
            }
            other => panic!("expected Other for unknown template, got {other:?}"),
        }
        Ok(())
    }

    /// Java `EgressPollerTest.shouldIgnoreUnknownMessageSchema`: a frame of
    /// another schema during connect is not an error.
    #[test]
    fn test_parse_event_another_schema_is_other() -> Result<(), Box<dyn std::error::Error>> {
        let mut frame = encode_session_event(b"ok", EventCode::OK)?;
        frame[4..6].copy_from_slice(&999u16.to_le_bytes());
        assert_eq!(
            parse_event(&frame)?,
            EgressEvent::Other {
                template_id: SessionEventEncoder::TEMPLATE_ID
            }
        );
        Ok(())
    }

    #[test]
    fn test_parse_event_session_event_ok_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = encode_session_event(b"connected", EventCode::OK)?;
        match parse_event(&bytes)? {
            EgressEvent::SessionEvent {
                correlation_id,
                cluster_session_id,
                leadership_term_id,
                leader_member_id,
                code,
                detail,
                leader_heartbeat_timeout_ns,
            } => {
                assert_eq!(correlation_id, 42);
                assert_eq!(cluster_session_id, 7);
                assert_eq!(leadership_term_id, 3);
                assert_eq!(leader_member_id, 1);
                assert_eq!(code, EventCode::OK);
                assert_eq!(detail, "connected");
                assert_eq!(leader_heartbeat_timeout_ns, None);
            }
            other => panic!("expected SessionEvent, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_event_new_leader_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let eps = b"0=localhost:9012,1=localhost:9112";
        let len = NewLeaderEventEncoder::compute_encoded_length_with_message_header(eps.len());
        let mut buf = vec![0u8; len];
        let complete = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&NewLeaderEventFixedFields {
                leadership_term_id: 9,
                cluster_session_id: 3,
                leader_member_id: 1,
            })
            .ingress_endpoints(eps)?;
        let bytes = complete.as_bytes_with_header();
        match parse_event(bytes)? {
            EgressEvent::NewLeader {
                cluster_session_id,
                leadership_term_id,
                leader_member_id,
                ingress_endpoints,
            } => {
                assert_eq!(cluster_session_id, 3);
                assert_eq!(leadership_term_id, 9);
                assert_eq!(leader_member_id, 1);
                assert_eq!(ingress_endpoints, "0=localhost:9012,1=localhost:9112");
            }
            other => panic!("expected NewLeader, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_event_session_message_header_too_short() {
        // Header-only claim of SessionMessageHeader without fixed body.
        let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(1)
            .cluster_session_id(2)
            .timestamp(3);
        // Drop the last body byte so the full fixed extent is incomplete.
        let short = &buf[..buf.len().saturating_sub(1)];
        if short.len() < 8 {
            return; // too short for a header, which another test covers
        }
        let err = parse_event(short).expect_err("short session message");
        assert!(matches!(err, ClusterError::ProtocolError { .. }), "got {err:?}");
    }

    #[test]
    fn test_parse_event_known_unprojected_template_keeps_template_id() -> Result<(), Box<dyn std::error::Error>> {
        // A well-formed application message decodes to `Fragment::Message`,
        // which the handshake has no use for. It must still report its real
        // template id. The arm once hardcoded 0, so callers could not tell
        // which frame arrived.
        let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(1)
            .cluster_session_id(2)
            .timestamp(3);
        match parse_event(&buf)? {
            EgressEvent::Other { template_id } => {
                assert_eq!(
                    template_id,
                    SessionMessageHeaderEncoder::TEMPLATE_ID,
                    "unprojected fragment must keep its template id, not 0"
                );
            }
            other => panic!("expected Other for unprojected template, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_picks_by_id_not_position() -> Result<(), Box<dyn std::error::Error>> {
        let eps = "0=localhost:9012,1=localhost:9112,2=localhost:9212";
        assert_eq!(parse_leader_endpoint(eps, 1)?.ok_or("ep")?, "localhost:9112");
        assert_eq!(parse_leader_endpoint(eps, 2)?.ok_or("ep")?, "localhost:9212");
        assert_ne!(parse_leader_endpoint(eps, 1)?.ok_or("ep")?, "localhost:9012");
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_missing_member() -> Result<(), Box<dyn std::error::Error>> {
        // A well-formed map that lacks the requested member is `Ok(None)`, unlike
        // a malformed map, which is `Err`.
        assert_eq!(parse_leader_endpoint("0=localhost:9012", 5)?, None);
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_malformed_map_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        // One malformed entry rejects the whole map, even beside a well-formed
        // leader entry. An empty map is malformed too: it has no member to resolve.
        for eps in [
            "garbage,1=localhost:9112,not-an-id=x,2",
            "1",
            "=,=foo",
            "",
            "not-a-map",
            "abc=def",
        ] {
            assert!(
                parse_leader_endpoint(eps, 1).is_err(),
                "expected {eps:?} to be rejected, not silently treated as \"no leader\""
            );
        }
        // Clean map still resolves by id.
        assert_eq!(
            parse_leader_endpoint("0=a:1,1=localhost:9112", 1)?.ok_or("ep")?,
            "localhost:9112"
        );
        Ok(())
    }

    /// Connect/async handshakes map a missing redirect leader to
    /// [`ClusterError::ReconnectFailed`] rather than looping until timeout.
    #[test]
    fn test_redirect_missing_leader_is_reconnect_failed_shape() -> Result<(), Box<dyn std::error::Error>> {
        let detail = "0=localhost:9012,2=localhost:9212";
        let leader_member_id = 1; // absent
        let err = parse_leader_endpoint(detail, leader_member_id)?
            .ok_or_else(|| ClusterError::ReconnectFailed {
                reason: format!("connect redirect listed no endpoint for leader member {leader_member_id}: {detail}"),
            })
            .expect_err("missing leader");
        assert!(matches!(err, ClusterError::ReconnectFailed { .. }), "got {err:?}");
        let msg = err.to_string();
        assert!(msg.contains("leader member 1"), "{msg}");
        assert!(msg.contains(detail), "{msg}");
        Ok(())
    }
}

/// Redirect resolution after a decoded REDIRECT (T-16), moved from the
/// integration tests when the parser became crate-private.
#[cfg(test)]
#[allow(clippy::panic)]
mod fail_closed {
    use super::*;
    use crate::codecs::session::{EventCode, SessionEventEncoder};

    /// T-16: a REDIRECT `SessionEvent` still decodes; resolution is a separate step.
    #[test]
    fn redirect_session_event_decodes_then_resolution_fails() -> Result<(), Box<dyn std::error::Error>> {
        let detail = b"0=localhost:9012"; // leader 1 absent
        let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
        let mut buf = vec![0u8; len];
        let complete = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&crate::codecs::session::SessionEventFixedFields {
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
}
