//! `EgressPoller` — single-fragment poller used during the connect
//! handshake. Captures the next `SessionEvent`, `Challenge`, or
//! `NewLeaderEvent` so the caller can react.

use crate::codecs::ergo_codecs::{
    ChallengeDecoder, EventCode, MessageHeader, NewLeaderEventDecoder, SessionEventDecoder,
};
use crate::codecs::ergo_codecs::{ChallengeEncoder, NewLeaderEventEncoder, SessionEventEncoder};

/// SBE message frame header is always 8 bytes.
const HEADER_LEN: usize = 8;

/// One captured egress event from a poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressEvent {
    /// A SessionEvent — connect result / state change.
    SessionEvent {
        correlation_id: i64,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        code: EventCode,
        detail: String,
    },
    /// An auth challenge.
    Challenge {
        correlation_id: i64,
        cluster_session_id: i64,
        encoded_challenge: Vec<u8>,
    },
    /// A new leader was elected.
    NewLeader {
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        ingress_endpoints: String,
    },
    /// An unrecognised / non-connect-phase message.
    Other { template_id: u16 },
}

/// Parse a single egress fragment into an `EgressEvent`.
pub fn parse_event(data: &[u8]) -> Option<EgressEvent> {
    if data.len() < HEADER_LEN {
        return None;
    }
    let header = MessageHeader(data[..HEADER_LEN].try_into().unwrap());
    let tid = header.template_id();

    match tid {
        SessionEventEncoder::TEMPLATE_ID => {
            let decoder = SessionEventDecoder::wrap_and_apply_header(data, 0).ok()?;
            let cid = decoder.correlation_id();
            let csid = decoder.cluster_session_id();
            let ltid = decoder.leadership_term_id();
            let lmid = decoder.leader_member_id();
            let code = decoder.code();
            let (detail_bytes, _) = decoder.into_detail().ok()?;
            let detail = std::str::from_utf8(detail_bytes).unwrap_or("").to_string();
            Some(EgressEvent::SessionEvent {
                correlation_id: cid,
                cluster_session_id: csid,
                leadership_term_id: ltid,
                leader_member_id: lmid,
                code,
                detail,
            })
        }
        ChallengeEncoder::TEMPLATE_ID => {
            let decoder = ChallengeDecoder::wrap_and_apply_header(data, 0).ok()?;
            let cid = decoder.correlation_id();
            let csid = decoder.cluster_session_id();
            let (chal, _) = decoder.into_encoded_challenge().ok()?;
            Some(EgressEvent::Challenge {
                correlation_id: cid,
                cluster_session_id: csid,
                encoded_challenge: chal.to_vec(),
            })
        }
        NewLeaderEventEncoder::TEMPLATE_ID => {
            let decoder = NewLeaderEventDecoder::wrap_and_apply_header(data, 0).ok()?;
            let csid = decoder.cluster_session_id();
            let ltid = decoder.leadership_term_id();
            let lmid = decoder.leader_member_id();
            let (eps_bytes, _) = decoder.into_ingress_endpoints().ok()?;
            Some(EgressEvent::NewLeader {
                cluster_session_id: csid,
                leadership_term_id: ltid,
                leader_member_id: lmid,
                ingress_endpoints: std::str::from_utf8(eps_bytes).unwrap_or("").to_string(),
            })
        }
        other => Some(EgressEvent::Other { template_id: other }),
    }
}

/// Parse a REDIRECT SessionEvent detail into `(leader_member_id, endpoint)`.
///
/// Java format: `"0=host:port,1=host:port,2=host:port"` with the leader
/// first. Returns the leader's (memberId, endpoint).
pub fn parse_redirect_leader(detail: &str) -> Option<(i32, String)> {
    let first = detail.split(',').next()?;
    let (id_str, ep) = first.split_once('=')?;
    let id: i32 = id_str.trim().parse().ok()?;
    Some((id, ep.trim().to_string()))
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

    #[test]
    fn test_parse_redirect_leader() -> Result<(), Box<dyn std::error::Error>> {
        let d = "0=localhost:9010,1=localhost:9011,2=localhost:9012";
        let (id, ep) = parse_redirect_leader(d).unwrap();
        assert_eq!(id, 0);
        assert_eq!(ep, "localhost:9010");

        Ok(())
    }

    #[test]
    fn test_parse_redirect_leader_single() -> Result<(), Box<dyn std::error::Error>> {
        let (id, ep) = parse_redirect_leader("3=host:9999").unwrap();
        assert_eq!(id, 3);
        assert_eq!(ep, "host:9999");

        Ok(())
    }

    #[test]
    fn test_parse_redirect_leader_malformed() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_redirect_leader("garbage").is_none());
        assert!(parse_redirect_leader("").is_none());

        Ok(())
    }

    #[test]
    fn test_parse_event_short_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_event(&[0u8; 4]).is_none());

        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_picks_by_id_not_position() -> Result<(), Box<dyn std::error::Error>> {
        // Node 0 (the dead leader) is listed first; the new leader is member 1.
        let eps = "0=localhost:9012,1=localhost:9112,2=localhost:9212";
        assert_eq!(parse_leader_endpoint(eps, 1).unwrap(), "localhost:9112");
        assert_eq!(parse_leader_endpoint(eps, 2).unwrap(), "localhost:9212");
        // A position-based parse would wrongly return localhost:9012 here.
        assert_ne!(parse_leader_endpoint(eps, 1).unwrap(), "localhost:9012");
        Ok(())
    }

    #[test]
    fn test_parse_leader_endpoint_missing_member() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_leader_endpoint("0=localhost:9012", 5).is_none());
        assert!(parse_leader_endpoint("", 0).is_none());

        Ok(())
    }
}
