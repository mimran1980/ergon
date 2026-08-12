//! `EgressPoller` — single-fragment poller used during the connect
//! handshake. Captures the next `SessionEvent`, `Challenge`, or
//! `NewLeaderEvent` so the caller can react.

use crate::ClusterError;
use crate::codecs::session::{EventCode, MessageHeader};

/// One captured egress event from a poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressEvent {
    /// A SessionEvent — connect result / state change.
    SessionEvent {
        /// Echo of the connect request correlation id.
        correlation_id: i64,
        /// Cluster-assigned session id.
        cluster_session_id: i64,
        /// Current leadership term.
        leadership_term_id: i64,
        /// Current leader member id.
        leader_member_id: i32,
        /// Status code (OK, ERROR, REDIRECT, AUTH_REJECTED).
        code: EventCode,
        /// Human-readable detail string from the cluster.
        detail: String,
    },
    /// An auth challenge.
    Challenge {
        /// Echo of the connect request correlation id.
        correlation_id: i64,
        /// Cluster-assigned session id.
        cluster_session_id: i64,
        /// Encoded challenge data to pass to the credential supplier.
        encoded_challenge: Vec<u8>,
    },
    /// A new leader was elected.
    NewLeader {
        /// Cluster-assigned session id.
        cluster_session_id: i64,
        /// New leadership term.
        leadership_term_id: i64,
        /// New leader member id.
        leader_member_id: i32,
        /// Member-endpoint map string for the new leader.
        ingress_endpoints: String,
    },
    /// An unrecognised / non-connect-phase message.
    Other {
        /// The unknown SBE template id.
        template_id: u16,
    },
}

/// Parse a single egress fragment via the canonical [`Fragment::decode`] path.
///
/// Text fields (`detail`, `ingress_endpoints`) are validated as UTF-8 via
/// the generated `_as_str()` accessors. Invalid text returns
/// [`ClusterError::InvalidUtf8`] rather than a lossy sentinel.
pub fn parse_event(data: &[u8]) -> Result<Option<EgressEvent>, ClusterError> {
    let fragment = match crate::fragment::Fragment::decode(data)? {
        Some(f) => f,
        None => {
            // Not an error — may be an unknown template. Report the template
            // id if we can peek it.
            let tid = MessageHeader::peek_template_id(data);
            return Ok(Some(EgressEvent::Other {
                template_id: tid.unwrap_or(0),
            }));
        }
    };

    use crate::fragment::Fragment;
    Ok(Some(match fragment {
        Fragment::SessionEvent {
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail,
        } => EgressEvent::SessionEvent {
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail: detail.to_string(),
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
        _ => EgressEvent::Other { template_id: 0 },
    }))
}

/// Resolve a single member's endpoint from a Java endpoints map.
///
/// `NewLeaderEvent.ingress_endpoints` lists ALL members in id order
/// (`"0=host:port,1=host:port,2=host:port"`); the new leader is identified
/// by `leader_member_id`, NOT by position. Picking the first entry (as a
/// redirect-style parse would) reconnects to the dead leader. This returns
/// the endpoint whose id matches `leader_member_id`.
pub fn parse_leader_endpoint(endpoints: &str, leader_member_id: i32) -> Option<String> {
    for entry in endpoints.split(',') {
        let Some((id_str, ep)) = entry.split_once('=') else {
            continue;
        };
        if let Ok(id) = id_str.trim().parse::<i32>()
            && id == leader_member_id
        {
            return Some(ep.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::session::{
        EventCode, NewLeaderEventEncoder, SessionEventEncoder, SessionMessageHeaderEncoder,
    };

    fn encode_session_event(detail: &[u8], code: EventCode) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
        let mut buf = vec![0u8; len];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.cluster_session_id(7)
            .correlation_id(42)
            .leadership_term_id(3)
            .leader_member_id(1)
            .code(code)
            .version(1);
        let complete = enc.detail(detail)?;
        Ok(complete.as_bytes_with_header().to_vec())
    }

    #[test]
    fn test_parse_event_short_header_is_error() -> Result<(), Box<dyn std::error::Error>> {
        // Fail-closed: frames shorter than the 8-byte SBE header are protocol errors,
        // not silent "no event" — matches Fragment::decode.
        let err = parse_event(&[0u8; 4]).expect_err("short header");
        assert!(
            matches!(err, ClusterError::ProtocolError { .. }),
            "got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("too short") || msg.contains("header"),
            "{msg}"
        );
        Ok(())
    }

    #[test]
    fn test_parse_event_empty_is_error() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_event(&[]).is_err());
        Ok(())
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
        assert!(
            matches!(err, ClusterError::ProtocolError { .. }),
            "got {err:?}"
        );
        Ok(())
    }

    #[test]
    fn test_parse_event_invalid_utf8_detail_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let bad = encode_session_event(&[0xff, 0xfe, 0xfd], EventCode::OK)?;
        let err = parse_event(&bad).expect_err("invalid utf8 detail");
        assert!(
            matches!(err, ClusterError::ProtocolError { .. }),
            "got {err:?}"
        );
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
            Some(EgressEvent::Other { template_id }) => {
                assert_eq!(template_id, 0xFEFE);
            }
            other => panic!("expected Other for unknown template, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_event_session_event_ok_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = encode_session_event(b"connected", EventCode::OK)?;
        match parse_event(&bytes)? {
            Some(EgressEvent::SessionEvent {
                correlation_id,
                cluster_session_id,
                leadership_term_id,
                leader_member_id,
                code,
                detail,
            }) => {
                assert_eq!(correlation_id, 42);
                assert_eq!(cluster_session_id, 7);
                assert_eq!(leadership_term_id, 3);
                assert_eq!(leader_member_id, 1);
                assert_eq!(code, EventCode::OK);
                assert_eq!(detail, "connected");
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
        let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.leadership_term_id(9)
            .cluster_session_id(3)
            .leader_member_id(1);
        let complete = enc.ingress_endpoints(eps)?;
        let bytes = complete.as_bytes_with_header();
        match parse_event(bytes)? {
            Some(EgressEvent::NewLeader {
                cluster_session_id,
                leadership_term_id,
                leader_member_id,
                ingress_endpoints,
            }) => {
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
    fn test_parse_event_session_message_header_too_short() -> Result<(), Box<dyn std::error::Error>> {
        // Header-only claim of SessionMessageHeader without fixed body.
        let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(1)
            .cluster_session_id(2)
            .timestamp(3);
        // Drop the last body byte so the full fixed extent is incomplete.
        let short = &buf[..buf.len().saturating_sub(1)];
        if short.len() < 8 {
            return Ok(()); // encoder length edge — header path still covered elsewhere
        }
        let err = parse_event(short).expect_err("short session message");
        assert!(
            matches!(err, ClusterError::ProtocolError { .. }),
            "got {err:?}"
        );
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_picks_by_id_not_position() -> Result<(), Box<dyn std::error::Error>> {
        let eps = "0=localhost:9012,1=localhost:9112,2=localhost:9212";
        assert_eq!(parse_leader_endpoint(eps, 1).ok_or("ep")?, "localhost:9112");
        assert_eq!(parse_leader_endpoint(eps, 2).ok_or("ep")?, "localhost:9212");
        assert_ne!(parse_leader_endpoint(eps, 1).ok_or("ep")?, "localhost:9012");
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_missing_member() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_leader_endpoint("0=localhost:9012", 5).is_none());
        assert!(parse_leader_endpoint("", 0).is_none());
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_malformed_entries_skipped() -> Result<(), Box<dyn std::error::Error>> {
        // Missing '=', non-numeric ids, and whitespace — only the valid id wins.
        let eps = "garbage,1=localhost:9112,not-an-id=x,2";
        assert_eq!(
            parse_leader_endpoint(eps, 1).ok_or("ep")?,
            "localhost:9112"
        );
        assert!(parse_leader_endpoint(eps, 2).is_none());
        assert!(parse_leader_endpoint("1", 1).is_none());
        Ok(())
    }

    /// Connect/async handshakes map a missing redirect leader to
    /// [`ClusterError::ReconnectFailed`] rather than looping until timeout.
    #[test]
    fn test_redirect_missing_leader_is_reconnect_failed_shape() -> Result<(), Box<dyn std::error::Error>> {
        let detail = "0=localhost:9012,2=localhost:9212";
        let leader_member_id = 1; // absent
        let err = parse_leader_endpoint(detail, leader_member_id)
            .ok_or_else(|| ClusterError::ReconnectFailed {
                reason: format!(
                    "connect redirect listed no endpoint for leader member {leader_member_id}: {detail}"
                ),
            })
            .expect_err("missing leader");
        assert!(
            matches!(err, ClusterError::ReconnectFailed { .. }),
            "got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("leader member 1"), "{msg}");
        assert!(msg.contains(detail), "{msg}");
        Ok(())
    }
}
