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

    #[test]
    fn test_parse_event_short_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_event(&[0u8; 4])?.is_none());

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
}
