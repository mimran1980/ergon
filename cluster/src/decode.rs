//! Equal-work egress decode helpers for production ErgoSBE codecs.
//!
//! These wrap the same `wrap_and_apply_header` path used by [`crate::egress`]
//! and the maintained Criterion decode benches (template_id + schema_id checks
//! in release). Prefer them over residual sbe-tool decoders for new call sites.

use crate::codecs::ergo_codecs::{
    EventCode, NewLeaderEventDecoder, SessionEventDecoder, SessionMessageHeaderDecoder,
};
use crate::error::ClusterError;

/// Fixed fields from a decoded `SessionMessageHeader` (schema 111, template 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionMessageHeaderView {
    /// Leadership term id from the header body.
    pub leadership_term_id: i64,
    /// Cluster session id.
    pub cluster_session_id: i64,
    /// Timestamp (epoch ns as carried on the wire).
    pub timestamp: i64,
}

/// Fixed fields + detail var-data from a `SessionEvent` (template 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEventView<'a> {
    /// Correlation id for the connect / async event.
    pub correlation_id: i64,
    /// Cluster session id.
    pub cluster_session_id: i64,
    /// Leadership term id.
    pub leadership_term_id: i64,
    /// Leader member id.
    pub leader_member_id: i32,
    /// Event code.
    pub code: EventCode,
    /// Detail string bytes (ASCII / UTF-8 as sent by the cluster).
    pub detail: &'a [u8],
}

/// Fields from a `NewLeaderEvent` (template for leadership change).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLeaderEventView<'a> {
    /// Cluster session id.
    pub cluster_session_id: i64,
    /// New leadership term id.
    pub leadership_term_id: i64,
    /// New leader member id.
    pub leader_member_id: i32,
    /// Ingress endpoints string bytes.
    pub ingress_endpoints: &'a [u8],
}

/// Decode a full SessionMessageHeader frame (8-byte SBE header + 24-byte body).
///
/// # Errors
///
/// Returns [`ClusterError::ProtocolError`] when the ErgoSBE decoder rejects the
/// buffer (too short, wrong template/schema, …).
#[inline]
pub fn decode_session_message_header(data: &[u8]) -> Result<SessionMessageHeaderView, ClusterError> {
    let d = SessionMessageHeaderDecoder::wrap_and_apply_header(data, 0).map_err(decode_err)?;
    Ok(SessionMessageHeaderView {
        leadership_term_id: d.leadership_term_id(),
        cluster_session_id: d.cluster_session_id(),
        timestamp: d.timestamp(),
    })
}

/// Decode a SessionEvent frame including the `detail` var-data field.
///
/// # Errors
///
/// Protocol / bounds errors from ErgoSBE become [`ClusterError::ProtocolError`].
#[inline]
pub fn decode_session_event(data: &[u8]) -> Result<SessionEventView<'_>, ClusterError> {
    let d = SessionEventDecoder::wrap_and_apply_header(data, 0).map_err(decode_err)?;
    let correlation_id = d.correlation_id();
    let cluster_session_id = d.cluster_session_id();
    let leadership_term_id = d.leadership_term_id();
    let leader_member_id = d.leader_member_id();
    let code = d.code();
    let (detail, _) = d.into_detail().map_err(decode_err)?;
    Ok(SessionEventView {
        correlation_id,
        cluster_session_id,
        leadership_term_id,
        leader_member_id,
        code,
        detail,
    })
}

/// Decode a NewLeaderEvent frame including ingress endpoints var-data.
///
/// # Errors
///
/// Protocol / bounds errors from ErgoSBE become [`ClusterError::ProtocolError`].
#[inline]
pub fn decode_new_leader_event(data: &[u8]) -> Result<NewLeaderEventView<'_>, ClusterError> {
    let d = NewLeaderEventDecoder::wrap_and_apply_header(data, 0).map_err(decode_err)?;
    let cluster_session_id = d.cluster_session_id();
    let leadership_term_id = d.leadership_term_id();
    let leader_member_id = d.leader_member_id();
    let (ingress_endpoints, _) = d.into_ingress_endpoints().map_err(decode_err)?;
    Ok(NewLeaderEventView {
        cluster_session_id,
        leadership_term_id,
        leader_member_id,
        ingress_endpoints,
    })
}

fn decode_err<E: std::fmt::Debug>(e: E) -> ClusterError {
    ClusterError::ProtocolError {
        reason: format!("sbe decode: {e:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::ergo_codecs::{
        EventCode, NewLeaderEventEncoder, SessionEventEncoder, SessionMessageHeaderEncoder,
    };

    #[test]
    fn session_message_header_roundtrip_view() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 32];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc
            .leadership_term_id(7)
            .cluster_session_id(99)
            .timestamp(1_234_567_890);
        let view = decode_session_message_header(&buf)?;
        assert_eq!(view.leadership_term_id, 7);
        assert_eq!(view.cluster_session_id, 99);
        assert_eq!(view.timestamp, 1_234_567_890);
        Ok(())
    }

    #[test]
    fn session_event_roundtrip_view() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 256];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc
            .cluster_session_id(1)
            .correlation_id(100)
            .leadership_term_id(5)
            .leader_member_id(0)
            .code(EventCode::OK)
            .version(1);
        let complete = enc.detail(b"some-detail")?;
        let bytes = complete.as_bytes_with_header();
        let view = decode_session_event(bytes)?;
        assert_eq!(view.correlation_id, 100);
        assert_eq!(view.cluster_session_id, 1);
        assert_eq!(view.leadership_term_id, 5);
        assert_eq!(view.code, EventCode::OK);
        assert_eq!(view.detail, b"some-detail");
        Ok(())
    }

    #[test]
    fn new_leader_roundtrip_view() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 256];
        let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc
            .cluster_session_id(2)
            .leadership_term_id(9)
            .leader_member_id(1);
        let complete = enc.ingress_endpoints(b"0=localhost:9000")?;
        let bytes = complete.as_bytes_with_header();
        let view = decode_new_leader_event(bytes)?;
        assert_eq!(view.cluster_session_id, 2);
        assert_eq!(view.leadership_term_id, 9);
        assert_eq!(view.leader_member_id, 1);
        assert_eq!(view.ingress_endpoints, b"0=localhost:9000");
        Ok(())
    }

    #[test]
    fn short_buffer_is_protocol_error() -> Result<(), Box<dyn std::error::Error>> {
        let err = decode_session_message_header(&[0u8; 4]).unwrap_err();
        match err {
            ClusterError::ProtocolError { reason } => assert!(reason.contains("sbe decode")),
            other => panic!("expected ProtocolError, got {other:?}"),
        }
    
        Ok(())
    }
}
