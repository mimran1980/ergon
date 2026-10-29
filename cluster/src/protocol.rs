//! Connect-protocol decisions shared by the blocking and polled handshakes.
//!
//! These match the Java client: protocol version 0.3.0, a new-leader timeout
//! taken from the leader heartbeat when the caller did not set one, and the
//! publication failures that end or fail over a session.

use crate::codecs::session::{
    ChallengeResponseEncoder, ChallengeResponseFixedFields, EventCode, SessionConnectRequestEncoder,
    SessionConnectRequestFixedFields,
};
use crate::error::{ClusterError, PublicationFailure};
use crate::poller::EgressEvent;
use crate::state::SessionState;

/// `PROTOCOL_SEMANTIC_VERSION` for 0.3.0: `(major << 16) | (minor << 8) | patch`.
pub const PROTOCOL_SEMANTIC_VERSION: i32 = 3 << 8;

/// Java `ClusterConfig.LEADER_HEARTBEAT_TIMEOUT_DEFAULT_NS` (10 seconds).
pub const LEADER_HEARTBEAT_TIMEOUT_DEFAULT_NS: u64 = 10_000_000_000;

/// Java `AeronCluster.Configuration.CLIENT_NAME_DEFAULT`.
pub const DEFAULT_CLIENT_NAME: &str = "cluster-client";

/// The new-leader timeout in milliseconds once connect has an OK.
///
/// A caller-supplied timeout wins. Otherwise the timeout is twice the leader
/// heartbeat from that OK, and twice the Java default heartbeat when the
/// cluster omits the field.
pub fn conclude_new_leader_timeout_ms(configured: Option<u64>, heartbeat_ns: Option<i64>) -> u64 {
    if let Some(ms) = configured {
        return ms;
    }
    let ns = heartbeat_ns
        .filter(|ns| *ns > 0)
        .map_or(LEADER_HEARTBEAT_TIMEOUT_DEFAULT_NS, i64::cast_unsigned);
    (ns.saturating_mul(2) / 1_000_000).max(1)
}

/// Session state after an ingress publication failure while `Connected`.
///
/// `NotConnected` and `Closed` wait for a new leader. `MaxPositionExceeded`
/// closes the session. Any other failure, and any failure outside
/// `Connected`, leaves the state alone.
pub fn ingress_failure_state(state: SessionState, failure: PublicationFailure) -> SessionState {
    if state != SessionState::Connected {
        return state;
    }
    match failure {
        PublicationFailure::NotConnected | PublicationFailure::Closed => SessionState::AwaitingNewLeader,
        PublicationFailure::MaxPositionExceeded => SessionState::PendingClose,
        _ => state,
    }
}

/// Java `clientInfo`: `name=<client name> <package> <version>`.
pub fn client_info_bytes(client_name: &str) -> Vec<u8> {
    format!(
        "name={client_name} {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .into_bytes()
}

/// One `SessionConnectRequest`. The caller re-offers the same bytes, so the
/// correlation id stays the one the cluster will echo.
pub fn encode_connect_request(
    correlation_id: i64,
    response_stream_id: i32,
    response_channel: &[u8],
    credentials: &[u8],
    client_info: &[u8],
) -> Result<Vec<u8>, ClusterError> {
    let len = SessionConnectRequestEncoder::compute_encoded_length_with_message_header(
        response_channel.len(),
        credentials.len(),
        client_info.len(),
    );
    let mut buf = vec![0u8; len];
    let _ = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&SessionConnectRequestFixedFields {
            correlation_id,
            response_stream_id,
            version: Some(PROTOCOL_SEMANTIC_VERSION),
        })
        .response_channel(response_channel)?
        .encoded_credentials(credentials)?
        .client_info(client_info)?;
    Ok(buf)
}

/// One `ChallengeResponse`. `correlation_id` is a new id, not the challenge's.
pub fn encode_challenge_response(
    correlation_id: i64,
    cluster_session_id: i64,
    credentials: &[u8],
) -> Result<Vec<u8>, ClusterError> {
    let len = ChallengeResponseEncoder::compute_encoded_length_with_message_header(credentials.len());
    let mut buf = vec![0u8; len];
    let _ = ChallengeResponseEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&ChallengeResponseFixedFields {
            correlation_id,
            cluster_session_id,
        })
        .encoded_credentials(credentials)?;
    Ok(buf)
}

/// What a connect handshake does with one egress event.
pub enum HandshakeEvent {
    /// SessionEvent(OK) for this attempt.
    Established {
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        leader_heartbeat_timeout_ns: Option<i64>,
        image_session_id: Option<i32>,
    },
    /// The cluster rejected the credentials.
    AuthRejected,
    /// Follow the leader at `endpoint`.
    Redirect { endpoint: String },
    /// Answer this challenge, then wait for an OK carrying the new correlation.
    Challenge {
        cluster_session_id: i64,
        encoded_challenge: Vec<u8>,
    },
    /// Not a response to the attempt in flight.
    Ignore,
}

/// Classify one parsed egress event against the correlation of the request
/// the handshake is waiting on. A mismatched id is [`HandshakeEvent::Ignore`].
pub fn classify_handshake(
    event: EgressEvent,
    expected_correlation: i64,
    image_session_id: Option<i32>,
) -> Result<HandshakeEvent, ClusterError> {
    match event {
        EgressEvent::SessionEvent {
            correlation_id,
            code,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            detail,
            leader_heartbeat_timeout_ns,
        } => {
            if correlation_id != expected_correlation {
                return Ok(HandshakeEvent::Ignore);
            }
            match code {
                EventCode::OK => Ok(HandshakeEvent::Established {
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    leader_heartbeat_timeout_ns,
                    image_session_id,
                }),
                EventCode::AUTHENTICATIONREJECTED => Ok(HandshakeEvent::AuthRejected),
                EventCode::REDIRECT => {
                    let endpoint =
                        crate::poller::parse_leader_endpoint(&detail, leader_member_id)?.ok_or_else(|| {
                            ClusterError::ReconnectFailed {
                                reason: format!(
                                    "connect redirect listed no endpoint for leader member {leader_member_id}: {detail}"
                                ),
                            }
                        })?;
                    Ok(HandshakeEvent::Redirect { endpoint })
                }
                _ => Ok(HandshakeEvent::Ignore),
            }
        }
        EgressEvent::Challenge {
            correlation_id,
            cluster_session_id,
            encoded_challenge,
        } => {
            if correlation_id != expected_correlation {
                return Ok(HandshakeEvent::Ignore);
            }
            Ok(HandshakeEvent::Challenge {
                cluster_session_id,
                encoded_challenge,
            })
        }
        _ => Ok(HandshakeEvent::Ignore),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_0_3_0() {
        assert_eq!(PROTOCOL_SEMANTIC_VERSION, 768);
    }

    #[test]
    fn configured_new_leader_timeout_wins() {
        assert_eq!(conclude_new_leader_timeout_ms(Some(5_000), Some(10_000_000_000)), 5_000);
    }

    #[test]
    fn missing_timeout_is_twice_the_leader_heartbeat() {
        assert_eq!(conclude_new_leader_timeout_ms(None, Some(10_000_000_000)), 20_000);
        assert_eq!(conclude_new_leader_timeout_ms(None, Some(1_500_000_000)), 3_000);
    }

    #[test]
    fn missing_heartbeat_uses_the_ten_second_default() {
        assert_eq!(conclude_new_leader_timeout_ms(None, None), 20_000);
        assert_eq!(conclude_new_leader_timeout_ms(None, Some(0)), 20_000);
        assert_eq!(conclude_new_leader_timeout_ms(None, Some(-1)), 20_000);
    }

    #[test]
    fn ingress_failures_match_java() {
        let connected = SessionState::Connected;
        assert_eq!(
            ingress_failure_state(connected, PublicationFailure::NotConnected),
            SessionState::AwaitingNewLeader
        );
        assert_eq!(
            ingress_failure_state(connected, PublicationFailure::Closed),
            SessionState::AwaitingNewLeader
        );
        assert_eq!(
            ingress_failure_state(connected, PublicationFailure::MaxPositionExceeded),
            SessionState::PendingClose
        );
        assert_eq!(
            ingress_failure_state(connected, PublicationFailure::BackPressured),
            connected
        );
        assert_eq!(
            ingress_failure_state(SessionState::AwaitingNewLeader, PublicationFailure::NotConnected),
            SessionState::AwaitingNewLeader
        );
    }

    #[test]
    fn connect_request_carries_the_protocol_version() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::AnyMessage;

        let buf = encode_connect_request(99, 102, b"aeron:udp?endpoint=localhost:1", b"creds", b"info")?;
        let AnyMessage::SessionConnectRequest(decoder) = AnyMessage::decode(&buf, 0)? else {
            return Err("expected SessionConnectRequest".into());
        };
        assert_eq!(decoder.correlation_id(), 99);
        assert_eq!(decoder.version(), Some(PROTOCOL_SEMANTIC_VERSION));
        Ok(())
    }

    #[test]
    fn mismatched_correlation_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let event = EgressEvent::SessionEvent {
            correlation_id: 1,
            cluster_session_id: 7,
            leadership_term_id: 3,
            leader_member_id: 0,
            code: EventCode::OK,
            detail: String::new(),
            leader_heartbeat_timeout_ns: Some(10_000_000_000),
        };
        assert!(matches!(classify_handshake(event, 2, None)?, HandshakeEvent::Ignore));
        Ok(())
    }
}
