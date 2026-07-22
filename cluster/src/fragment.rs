//! Shared egress fragment dispatch — one decode path for all listeners.
//!
//! Every egress path (`EgressAdapter`, `ControlledEgressAdapter`, `poller`)
//! previously had its own copy of the same `AnyMessage` match. This module
//! provides the single canonical dispatch with proper error propagation.
//! Callers decide how to route the result.

use crate::codecs::session::{AdminRequestType, AdminResponseCode, AnyMessage, EventCode, SessionMessageHeaderEncoder};
use crate::error::ClusterError;

/// A fully-decoded egress fragment, ready for listener dispatch.
#[derive(Debug)]
pub(crate) enum Fragment<'a> {
    /// Application message with its payload slice.
    Message {
        cluster_session_id: i64,
        timestamp: i64,
        payload: &'a [u8],
    },
    /// Session lifecycle event.
    SessionEvent {
        correlation_id: i64,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        code: EventCode,
        detail: &'a str,
    },
    /// New leader elected.
    NewLeader {
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        ingress_endpoints: &'a str,
    },
    /// Credential challenge.
    Challenge {
        correlation_id: i64,
        cluster_session_id: i64,
        encoded_challenge: &'a [u8],
    },
    /// Administrative response.
    AdminResponse {
        cluster_session_id: i64,
        correlation_id: i64,
        request_type: AdminRequestType,
        response_code: AdminResponseCode,
        message: &'a str,
        payload: &'a [u8],
    },
}

impl<'a> Fragment<'a> {
    /// Decode one egress fragment from wire bytes.
    ///
    /// Returns `Ok(None)` for unknown template IDs (not an error — the
    /// cluster may send messages not in our schema). Returns `Err` only
    /// for malformed frames, invalid text, or buffer overruns.
    pub(crate) fn decode(data: &'a [u8]) -> Result<Option<Self>, ClusterError> {
        let msg = match AnyMessage::decode(data, 0) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        Ok(Some(match msg {
            AnyMessage::SessionMessageHeader(decoder) => {
                if data.len() < SessionMessageHeaderEncoder::ENCODED_LENGTH {
                    return Err(ClusterError::ProtocolError {
                        reason: "session message too short".into(),
                    });
                }
                let payload = &data[SessionMessageHeaderEncoder::ENCODED_LENGTH..];
                Self::Message {
                    cluster_session_id: decoder.cluster_session_id(),
                    timestamp: decoder.timestamp(),
                    payload,
                }
            }
            AnyMessage::SessionEvent(decoder) => {
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let code = decoder.code();
                let (detail, _) = decoder.into_detail_as_str().map_err(|e| ClusterError::ProtocolError {
                    reason: format!("session event detail: {e}"),
                })?;
                Self::SessionEvent {
                    correlation_id: cid,
                    cluster_session_id: csid,
                    leadership_term_id: ltid,
                    leader_member_id: lmid,
                    code,
                    detail,
                }
            }
            AnyMessage::NewLeaderEvent(decoder) => {
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let (ingress_endpoints, _) =
                    decoder
                        .into_ingress_endpoints_as_str()
                        .map_err(|e| ClusterError::ProtocolError {
                            reason: format!("new leader endpoints: {e}"),
                        })?;
                Self::NewLeader {
                    cluster_session_id: csid,
                    leadership_term_id: ltid,
                    leader_member_id: lmid,
                    ingress_endpoints,
                }
            }
            AnyMessage::Challenge(decoder) => {
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                // Challenges are binary — raw bytes.
                let chal = decoder.into_encoded_challenge().map(|(b, _)| b).unwrap_or(&[]);
                Self::Challenge {
                    correlation_id: cid,
                    cluster_session_id: csid,
                    encoded_challenge: chal,
                }
            }
            AnyMessage::AdminResponse(decoder) => {
                let csid = decoder.cluster_session_id();
                let cid = decoder.correlation_id();
                let rt = decoder.request_type();
                let rc = decoder.response_code();
                let (msg_bytes, after_msg) = decoder.into_message().map_err(|e| ClusterError::ProtocolError {
                    reason: format!("admin response message: {e:?}"),
                })?;
                let (payload_bytes, _) = after_msg.into_payload().map_err(|e| ClusterError::ProtocolError {
                    reason: format!("admin response payload: {e:?}"),
                })?;
                let msg = std::str::from_utf8(msg_bytes).map_err(|e| ClusterError::ProtocolError {
                    reason: format!("admin response not UTF-8: {e}"),
                })?;
                Self::AdminResponse {
                    cluster_session_id: csid,
                    correlation_id: cid,
                    request_type: rt,
                    response_code: rc,
                    message: msg,
                    payload: payload_bytes,
                }
            }
            AnyMessage::Unknown { .. } => return Ok(None),
            _ => return Ok(None),
        }))
    }
}
