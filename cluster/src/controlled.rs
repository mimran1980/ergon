//! Controlled egress polling — mirrors Java `ControlledEgressAdapter` /
//! `ControlledEgressListener`. Callbacks return a `ControlledPollAction`
//! so the application can apply backpressure (Abort) or stop (Break).

use crate::codecs::ergo_codecs::{
    AdminRequestType, AdminResponseCode, AdminResponseDecoder, ChallengeDecoder, EventCode, MessageHeader,
    NewLeaderEventDecoder, SessionEventDecoder, SessionMessageHeaderDecoder,
};
use crate::codecs::ergo_codecs::{
    AdminResponseEncoder, ChallengeEncoder, NewLeaderEventEncoder, SessionEventEncoder, SessionMessageHeaderEncoder,
};

/// Decode var-data bytes as UTF-8 with a consistent sentinel on failure.
#[inline]
fn as_utf8_lossy(data: &[u8]) -> &str {
    std::str::from_utf8(data).unwrap_or("<invalid utf-8>")
}

/// Action returned by a `ControlledEgressListener`.
///
/// Mirrors Aeron's `ControlledFragmentHandler.Action`:
/// - `Continue` — keep dispatching fragments
/// - `Abort` — stop dispatching and re-deliver this fragment next poll
/// - `Break` — stop dispatching; do not re-deliver
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ControlledPollAction {
    Continue = 0,
    Abort = 1,
    Break = 2,
}

/// Controlled variant of `EgressListener`. Each callback returns an
/// action so the caller can apply flow control.
pub trait ControlledEgressListener {
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]) -> ControlledPollAction;
    fn on_session_event(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        code: EventCode,
        detail: &str,
    ) -> ControlledPollAction;
    fn on_new_leader(
        &mut self,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        ingress_endpoints: &str,
    ) -> ControlledPollAction;
    fn on_challenge(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        encoded_challenge: &[u8],
    ) -> ControlledPollAction;
    fn on_admin_response(
        &mut self,
        cluster_session_id: i64,
        correlation_id: i64,
        request_type: AdminRequestType,
        response_code: AdminResponseCode,
        message: &str,
        payload: &[u8],
    ) -> ControlledPollAction;
}

/// Dispatch egress fragments to a `ControlledEgressListener`.
pub struct ControlledEgressAdapter<L: ControlledEgressListener> {
    listener: L,
}

impl<L: ControlledEgressListener> ControlledEgressAdapter<L> {
    pub fn new(listener: L) -> Self {
        Self { listener }
    }

    /// Decode and dispatch one egress fragment. Returns the action the
    /// listener produced (or `Continue` for unrecognised template IDs).
    pub fn on_fragment(&mut self, data: &[u8]) -> ControlledPollAction {
        let Some(template_id) = MessageHeader::peek_for_schema(data, SessionMessageHeaderEncoder::SCHEMA_ID) else {
        return ControlledPollAction::Continue;
    };

    match template_id {
            SessionMessageHeaderEncoder::TEMPLATE_ID => {
                if data.len() < SessionMessageHeaderEncoder::ENCODED_LENGTH {
                    return ControlledPollAction::Continue;
                }
                let Ok(body) = SessionMessageHeaderDecoder::wrap_and_apply_header(data, 0) else {
                    return ControlledPollAction::Continue;
                };
                let payload = &data[SessionMessageHeaderEncoder::ENCODED_LENGTH..];
                self.listener
                    .on_message(body.cluster_session_id(), body.timestamp(), payload)
            }
            SessionEventEncoder::TEMPLATE_ID => {
                let Ok(decoder) = SessionEventDecoder::wrap_and_apply_header(data, 0) else {
                    return ControlledPollAction::Continue;
                };
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let code = decoder.code();
                let Ok((detail_bytes, _)) = decoder.into_detail() else {
                    return ControlledPollAction::Continue;
                };
                let detail = as_utf8_lossy(detail_bytes);
                self.listener.on_session_event(cid, csid, ltid, lmid, code, detail)
            }
            NewLeaderEventEncoder::TEMPLATE_ID => {
                let Ok(decoder) = NewLeaderEventDecoder::wrap_and_apply_header(data, 0) else {
                    return ControlledPollAction::Continue;
                };
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let Ok((eps_bytes, _)) = decoder.into_ingress_endpoints() else {
                    return ControlledPollAction::Continue;
                };
                let eps = as_utf8_lossy(eps_bytes);
                self.listener.on_new_leader(csid, ltid, lmid, eps)
            }
            ChallengeEncoder::TEMPLATE_ID => {
                let Ok(decoder) = ChallengeDecoder::wrap_and_apply_header(data, 0) else {
                    return ControlledPollAction::Continue;
                };
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let Ok((chal, _)) = decoder.into_encoded_challenge() else {
                    return ControlledPollAction::Continue;
                };
                self.listener.on_challenge(cid, csid, chal)
            }
            AdminResponseEncoder::TEMPLATE_ID => {
                let Ok(decoder) = AdminResponseDecoder::wrap_and_apply_header(data, 0) else {
                    return ControlledPollAction::Continue;
                };
                let csid = decoder.cluster_session_id();
                let cid = decoder.correlation_id();
                let rt = decoder.request_type();
                let rc = decoder.response_code();
                let Ok((msg_bytes, after_msg)) = decoder.into_message() else {
                    return ControlledPollAction::Continue;
                };
                let Ok((pl, _)) = after_msg.into_payload() else {
                    return ControlledPollAction::Continue;
                };
                let msg = as_utf8_lossy(msg_bytes).to_string();
                let pl = pl.to_vec();
                self.listener.on_admin_response(csid, cid, rt, rc, &msg, &pl)
            }
            _ => ControlledPollAction::Continue,
        }
    }

    pub fn listener(&self) -> &L {
        &self.listener
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_values_match_aeron() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ControlledPollAction::Continue as i32, 0);
        assert_eq!(ControlledPollAction::Abort as i32, 1);
        assert_eq!(ControlledPollAction::Break as i32, 2);

        Ok(())
    }

    #[test]
    fn test_short_fragment_returns_continue() -> Result<(), Box<dyn std::error::Error>> {
        let mut a = ControlledEgressAdapter::new(NoOp);
        assert_eq!(a.on_fragment(&[0u8; 4]), ControlledPollAction::Continue);

        Ok(())
    }

    struct NoOp;
    impl ControlledEgressListener for NoOp {
        fn on_message(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledPollAction {
            ControlledPollAction::Continue
        }
        fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) -> ControlledPollAction {
            ControlledPollAction::Continue
        }
        fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) -> ControlledPollAction {
            ControlledPollAction::Continue
        }
        fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledPollAction {
            ControlledPollAction::Continue
        }
        fn on_admin_response(
            &mut self,
            _: i64,
            _: i64,
            _: AdminRequestType,
            _: AdminResponseCode,
            _: &str,
            _: &[u8],
        ) -> ControlledPollAction {
            ControlledPollAction::Continue
        }
    }
}
