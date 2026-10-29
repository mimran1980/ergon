//! One decode of an egress fragment, shared by the poll's state tracking
//! and listener dispatch.

use crate::codecs::session::sbe_rt::DecodeError;
use crate::codecs::session::{AdminRequestType, AdminResponseCode, AnyMessage, EventCode, SessionMessageHeaderEncoder};
use crate::error::ClusterError;

/// Every SBE frame begins with an 8-byte message header (blockLength,
/// templateId, schemaId, version). Per the SBE specification § Message
/// Header, this field is fixed and never varies.
const SBE_MESSAGE_HEADER_LENGTH: usize = 8;

/// A [`ClusterError::ProtocolError`] for a malformed frame. Out of line so
/// the per-fragment decode keeps the formatting off its hot path.
#[cold]
#[inline(never)]
fn protocol_error(reason: std::fmt::Arguments<'_>) -> ClusterError {
    ClusterError::ProtocolError {
        reason: reason.to_string(),
    }
}

/// A fully-decoded egress fragment, ready for listener dispatch.
#[derive(Debug, Clone, Copy)]
pub enum Fragment<'a> {
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
        /// Leader heartbeat from the event, when the cluster sent one.
        leader_heartbeat_timeout_ns: Option<i64>,
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
    /// The session the fragment is for.
    pub(crate) const fn cluster_session_id(&self) -> i64 {
        match self {
            Self::Message { cluster_session_id, .. }
            | Self::SessionEvent { cluster_session_id, .. }
            | Self::NewLeader { cluster_session_id, .. }
            | Self::Challenge { cluster_session_id, .. }
            | Self::AdminResponse { cluster_session_id, .. } => *cluster_session_id,
        }
    }

    /// Decode one egress fragment from wire bytes.
    ///
    /// Returns `Ok(None)` for unknown template IDs (not an error — the
    /// cluster may send messages not in our schema). Returns `Err` for
    /// malformed frames (including frames too short to contain a complete
    /// 8-byte SBE header), invalid text, or buffer overruns.
    pub(crate) fn decode(data: &'a [u8]) -> Result<Option<Self>, ClusterError> {
        if data.len() < SBE_MESSAGE_HEADER_LENGTH {
            return Err(protocol_error(format_args!(
                "frame too short for SBE header: {} bytes (need at least {})",
                data.len(),
                SBE_MESSAGE_HEADER_LENGTH
            )));
        }
        let msg = match AnyMessage::decode(data, 0) {
            Ok(m) => m,
            Err(DecodeError::UnknownTemplateLength { .. }) => return Ok(None),
            Err(e) => {
                return Err(protocol_error(format_args!("fragment decode: {e}")));
            }
        };

        Ok(Some(match msg {
            AnyMessage::SessionMessageHeader(decoder) => {
                // Explicit size guard: AnyMessage::decode only validates the
                // 8-byte frame header, not the full body. Reject truncated data
                // before slicing the payload.
                if data.len() < SessionMessageHeaderEncoder::ENCODED_LENGTH {
                    return Err(protocol_error(format_args!("session message too short")));
                }
                let payload = decoder.get_metadata().remaining();
                Self::Message {
                    cluster_session_id: decoder.cluster_session_id(),
                    timestamp: decoder.timestamp(),
                    payload,
                }
            }
            AnyMessage::SessionEvent(decoder) => {
                let correlation_id = decoder.correlation_id();
                let cluster_session_id = decoder.cluster_session_id();
                let leadership_term_id = decoder.leadership_term_id();
                let leader_member_id = decoder.leader_member_id();
                let code = decoder.code();
                // Read before `into_detail_as_str` consumes the decoder.
                let leader_heartbeat_timeout_ns = decoder.leader_heartbeat_timeout_ns();
                let (detail, _) = decoder
                    .into_detail_as_str()
                    .map_err(|e| protocol_error(format_args!("session event detail: {e}")))?;
                Self::SessionEvent {
                    correlation_id,
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    code,
                    detail,
                    leader_heartbeat_timeout_ns,
                }
            }
            AnyMessage::NewLeaderEvent(decoder) => {
                let cluster_session_id = decoder.cluster_session_id();
                let leadership_term_id = decoder.leadership_term_id();
                let leader_member_id = decoder.leader_member_id();
                let (ingress_endpoints, _) = decoder
                    .into_ingress_endpoints_as_str()
                    .map_err(|e| protocol_error(format_args!("new leader endpoints: {e}")))?;
                Self::NewLeader {
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    ingress_endpoints,
                }
            }
            AnyMessage::Challenge(decoder) => {
                let correlation_id = decoder.correlation_id();
                let cluster_session_id = decoder.cluster_session_id();
                let (encoded_challenge, _next) = decoder
                    .into_encoded_challenge()
                    .map_err(|e| protocol_error(format_args!("challenge payload: {e}")))?;
                Self::Challenge {
                    correlation_id,
                    cluster_session_id,
                    encoded_challenge,
                }
            }
            AnyMessage::AdminResponse(decoder) => {
                let cluster_session_id = decoder.cluster_session_id();
                let correlation_id = decoder.correlation_id();
                let request_type = decoder.request_type();
                let response_code = decoder.response_code();
                let (msg_bytes, after_msg) = decoder
                    .into_message()
                    .map_err(|e| protocol_error(format_args!("admin response message: {e:?}")))?;
                let (payload, _) = after_msg
                    .into_payload()
                    .map_err(|e| protocol_error(format_args!("admin response payload: {e:?}")))?;
                let message = std::str::from_utf8(msg_bytes)
                    .map_err(|e| protocol_error(format_args!("admin response not UTF-8: {e}")))?;
                Self::AdminResponse {
                    cluster_session_id,
                    correlation_id,
                    request_type,
                    response_code,
                    message,
                    payload,
                }
            }
            // AnyMessage::Unknown and the templates not projected here
            _ => return Ok(None),
        }))
    }
}

/// Header fields of a frame the schema does not decode, for an extension hook.
pub struct ExtensionView<'a> {
    /// Acting block length from the SBE header.
    pub acting_block_length: u16,
    /// Template id from the SBE header.
    pub template_id: u16,
    /// Schema id from the SBE header.
    pub schema_id: u16,
    /// Acting schema version from the SBE header.
    pub acting_version: u16,
    /// Bytes after the 8-byte header.
    pub payload: &'a [u8],
}

/// Split an 8-byte SBE header from its payload. `None` when `data` is shorter
/// than a header; [`Fragment::decode`] already reports that as an error.
pub fn extension_view(data: &[u8]) -> Option<ExtensionView<'_>> {
    if data.len() < SBE_MESSAGE_HEADER_LENGTH {
        return None;
    }
    Some(ExtensionView {
        acting_block_length: u16::from_le_bytes([data[0], data[1]]),
        template_id: u16::from_le_bytes([data[2], data[3]]),
        schema_id: u16::from_le_bytes([data[4], data[5]]),
        acting_version: u16::from_le_bytes([data[6], data[7]]),
        payload: &data[SBE_MESSAGE_HEADER_LENGTH..],
    })
}
