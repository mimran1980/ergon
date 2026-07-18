//! Controlled egress polling — mirrors Java `ControlledEgressAdapter` /
//! `ControlledEgressListener`. Callbacks return a `ControlledPollAction`
//! so the application can apply backpressure (Abort) or stop (Break).

use crate::codecs::cluster_codecs::{
    ReadBuf, admin_request_type::AdminRequestType, admin_response_code::AdminResponseCode,
    admin_response_codec::SBE_TEMPLATE_ID as ADMIN_RESPONSE_ID, challenge_codec::SBE_TEMPLATE_ID as CHALLENGE_ID,
    event_code::EventCode, message_header_codec::ENCODED_LENGTH as HEADER_LEN,
    new_leader_event_codec::SBE_TEMPLATE_ID as NEW_LEADER_ID, session_event_codec::SBE_TEMPLATE_ID as SESSION_EVENT_ID,
    session_message_header_codec::SBE_TEMPLATE_ID as SESSION_MSG_HEADER_ID,
};

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
        if data.len() < HEADER_LEN {
            return ControlledPollAction::Continue;
        }
        let read_buf = ReadBuf::new(data);
        let header =
            crate::codecs::cluster_codecs::message_header_codec::MessageHeaderDecoder::default().wrap(read_buf, 0);
        let template_id = header.template_id();

        match template_id {
            SESSION_MSG_HEADER_ID => {
                if data.len() < HEADER_LEN + 24 {
                    return ControlledPollAction::Continue;
                }
                let body =
                    crate::codecs::cluster_codecs::session_message_header_codec::SessionMessageHeaderDecoder::default()
                        .header(header, 0);
                let payload = &data[HEADER_LEN + 24..];
                self.listener
                    .on_message(body.cluster_session_id(), body.timestamp(), payload)
            }
            SESSION_EVENT_ID => {
                use crate::codecs::cluster_codecs::session_event_codec::SessionEventDecoder;
                let mut dec = SessionEventDecoder::default().header(header, 0);
                let coords = dec.detail_decoder();
                let detail = std::str::from_utf8(dec.detail_slice(coords)).unwrap_or("<bad utf8>");
                self.listener.on_session_event(
                    dec.correlation_id(),
                    dec.cluster_session_id(),
                    dec.leadership_term_id(),
                    dec.leader_member_id(),
                    dec.code(),
                    detail,
                )
            }
            NEW_LEADER_ID => {
                use crate::codecs::cluster_codecs::new_leader_event_codec::NewLeaderEventDecoder;
                let mut dec = NewLeaderEventDecoder::default().header(header, 0);
                let coords = dec.ingress_endpoints_decoder();
                let eps = std::str::from_utf8(dec.ingress_endpoints_slice(coords)).unwrap_or("<bad utf8>");
                self.listener.on_new_leader(
                    dec.cluster_session_id(),
                    dec.leadership_term_id(),
                    dec.leader_member_id(),
                    eps,
                )
            }
            CHALLENGE_ID => {
                use crate::codecs::cluster_codecs::challenge_codec::ChallengeDecoder;
                let mut dec = ChallengeDecoder::default().header(header, 0);
                let coords = dec.encoded_challenge_decoder();
                let chal = dec.encoded_challenge_slice(coords);
                self.listener
                    .on_challenge(dec.correlation_id(), dec.cluster_session_id(), chal)
            }
            ADMIN_RESPONSE_ID => {
                use crate::codecs::cluster_codecs::admin_response_codec::AdminResponseDecoder;
                let mut dec = AdminResponseDecoder::default().header(header, 0);
                let mc = dec.message_decoder();
                let pc = dec.payload_decoder();
                let msg = std::str::from_utf8(dec.message_slice(mc))
                    .unwrap_or("<bad utf8>")
                    .to_string();
                let pl = dec.payload_slice(pc).to_vec();
                self.listener.on_admin_response(
                    dec.cluster_session_id(),
                    dec.correlation_id(),
                    dec.request_type(),
                    dec.response_code(),
                    &msg,
                    &pl,
                )
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
    fn test_action_values_match_aeron() {
        assert_eq!(ControlledPollAction::Continue as i32, 0);
        assert_eq!(ControlledPollAction::Abort as i32, 1);
        assert_eq!(ControlledPollAction::Break as i32, 2);
    }

    #[test]
    fn test_short_fragment_returns_continue() {
        let mut a = ControlledEgressAdapter::new(NoOp);
        assert_eq!(a.on_fragment(&[0u8; 4]), ControlledPollAction::Continue);
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
