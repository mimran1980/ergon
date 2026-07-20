//! Controlled egress polling — mirrors Java `ControlledEgressAdapter` /
//! `ControlledEgressListener`. Callbacks return a `ControlledPollAction`
//! so the application can apply backpressure (Abort) or stop (Break).

use crate::codecs::session::{
    AdminRequestType, AdminResponseCode, AnyMessage, EventCode,
    SessionMessageHeaderEncoder,
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

    /// Decode and dispatch one egress fragment via `AnyMessage::decode`.
    /// Returns the action the listener produced, or `Continue` for
    /// unrecognised / unparseable fragments.
    pub fn on_fragment(&mut self, data: &[u8]) -> ControlledPollAction {
        let Ok(msg) = AnyMessage::decode(data, 0) else {
            return ControlledPollAction::Continue;
        };

        match msg {
            AnyMessage::SessionMessageHeader(decoder) => {
                if data.len() < SessionMessageHeaderEncoder::ENCODED_LENGTH {
                    return ControlledPollAction::Continue;
                }
                let payload = &data[SessionMessageHeaderEncoder::ENCODED_LENGTH..];
                self.listener
                    .on_message(decoder.cluster_session_id(), decoder.timestamp(), payload)
            }
            AnyMessage::SessionEvent(decoder) => {
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let code = decoder.code();
                let detail = decoder.into_detail()
                    .map(|(b, _)| as_utf8_lossy(b))
                    .unwrap_or("<invalid utf-8>");
                self.listener.on_session_event(cid, csid, ltid, lmid, code, detail)
            }
            AnyMessage::NewLeaderEvent(decoder) => {
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let eps = decoder.into_ingress_endpoints()
                    .map(|(b, _)| as_utf8_lossy(b))
                    .unwrap_or("<invalid utf-8>");
                self.listener.on_new_leader(csid, ltid, lmid, eps)
            }
            AnyMessage::Challenge(decoder) => {
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let chal = decoder.into_encoded_challenge()
                    .map(|(b, _)| b)
                    .unwrap_or(&[]);
                self.listener.on_challenge(cid, csid, chal)
            }
            AnyMessage::AdminResponse(decoder) => {
                let csid = decoder.cluster_session_id();
                let cid = decoder.correlation_id();
                let rt = decoder.request_type();
                let rc = decoder.response_code();
                let (msg_bytes, after_msg) = match decoder.into_message() {
                    Ok(v) => v,
                    Err(_) => return ControlledPollAction::Continue,
                };
                let (pl, _) = match after_msg.into_payload() {
                    Ok(v) => v,
                    Err(_) => return ControlledPollAction::Continue,
                };
                let msg = as_utf8_lossy(msg_bytes).to_string();
                let pl = pl.to_vec();
                self.listener.on_admin_response(csid, cid, rt, rc, &msg, &pl)
            }
            AnyMessage::Unknown { .. } => ControlledPollAction::Continue,
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
    use crate::codecs::session::{
        ChallengeEncoder, EventCode as ErgoEventCode,
        NewLeaderEventEncoder, SessionEventEncoder, SessionMessageHeaderEncoder,
    };

    #[test]
    fn test_action_values_match_aeron() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ControlledPollAction::Continue as i32, 0);
        assert_eq!(ControlledPollAction::Abort as i32, 1);
        assert_eq!(ControlledPollAction::Break as i32, 2);
        Ok(())
    }

    #[test]
    fn test_short_fragment_returns_continue() -> Result<(), Box<dyn std::error::Error>> {
        let mut a = ControlledEgressAdapter::new(Rec::default());
        assert_eq!(a.on_fragment(&[0u8; 4]), ControlledPollAction::Continue);
        Ok(())
    }

    #[test]
    fn test_wrong_schema_id_returns_continue() -> Result<(), Box<dyn std::error::Error>> {
        let mut a = ControlledEgressAdapter::new(Rec::default());
        // Header with schema_id=0 (cluster schema is 111)
        let mut hdr = [0u8; 8];
        hdr[4..6].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(a.on_fragment(&hdr), ControlledPollAction::Continue);
        Ok(())
    }

    // ── Roundtrip: encode, dispatch through on_fragment, verify listener fields ──

    #[test]
    fn test_dispatch_session_event_ok() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut data, 0)?;
        enc.cluster_session_id(7).correlation_id(99).leadership_term_id(3)
            .leader_member_id(0).code(ErgoEventCode::OK).version(1);
        let complete = enc.detail(b"ok")?;
        let bytes = complete.as_bytes_with_header().to_vec();

        let mut adapter = ControlledEgressAdapter::new(Rec::default());
        let action = adapter.on_fragment(&bytes);
        assert_eq!(action, ControlledPollAction::Continue);
        let r = adapter.listener();
        assert_eq!(r.session_code, Some(EventCode::OK));
        assert_eq!(r.detail, "ok");
        assert_eq!(r.calls, 1);
        Ok(())
    }

    #[test]
    fn test_dispatch_challenge() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = ChallengeEncoder::wrap_and_apply_header(&mut data, 0)?;
        enc.correlation_id(5).cluster_session_id(2);
        let complete = enc.encoded_challenge(b"chal-token")?;
        let bytes = complete.as_bytes_with_header().to_vec();

        let mut adapter = ControlledEgressAdapter::new(Rec::default());
        let action = adapter.on_fragment(&bytes);
        assert_eq!(action, ControlledPollAction::Continue);
        assert_eq!(adapter.listener().challenge, b"chal-token");
        assert_eq!(adapter.listener().calls, 1);
        Ok(())
    }

    #[test]
    fn test_dispatch_new_leader() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 256];
        let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut data, 0)?;
        enc.leadership_term_id(10).cluster_session_id(99).leader_member_id(1);
        let complete = enc.ingress_endpoints(b"0=host:9000")?;
        let bytes = complete.as_bytes_with_header().to_vec();

        let mut adapter = ControlledEgressAdapter::new(Rec::default());
        let action = adapter.on_fragment(&bytes);
        assert_eq!(action, ControlledPollAction::Continue);
        assert_eq!(adapter.listener().leader_endpoints, "0=host:9000");
        assert_eq!(adapter.listener().calls, 1);
        Ok(())
    }

    #[test]
    fn test_dispatch_session_message_header() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0)?;
        enc.leadership_term_id(42).cluster_session_id(99).timestamp(1_000_000);
        let bytes = data[..SessionMessageHeaderEncoder::ENCODED_LENGTH + 4].to_vec();

        let mut adapter = ControlledEgressAdapter::new(Rec::default());
        let action = adapter.on_fragment(&bytes);
        assert_eq!(action, ControlledPollAction::Continue);
        let r = adapter.listener();
        assert_eq!(r.msg_csid, 99);
        assert_eq!(r.msg_ts, 1_000_000);
        assert_eq!(r.calls, 1);
        Ok(())
    }

    // ── Malformed data: silent error sites must return Continue ──

    #[test]
    fn test_bad_var_data_returns_continue() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = ControlledEgressAdapter::new(Rec::default());
        let mut data = vec![0u8; 128];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut data, 0)?;
        enc.cluster_session_id(1).correlation_id(2).leadership_term_id(3)
            .leader_member_id(0).code(ErgoEventCode::OK).version(1);
        let complete = enc.detail(b"ok")?;
        let mut bytes = complete.as_bytes_with_header().to_vec();
        // Corrupt the var-data length to point past buffer end
        let body_start = 8;
        let detail_len_offset = body_start + SessionEventEncoder::BLOCK_LENGTH;
        bytes[detail_len_offset] = 0xFF;
        bytes[detail_len_offset + 1] = 0xFF;
        bytes[detail_len_offset + 2] = 0xFF;
        bytes[detail_len_offset + 3] = 0xFF;
        assert_eq!(adapter.on_fragment(&bytes), ControlledPollAction::Continue);
        Ok(())
    }

    // ── Recording listener ────────────────────────────────────────────

    #[derive(Default)]
    struct Rec {
        calls: usize,
        session_code: Option<EventCode>,
        detail: String,
        leader_endpoints: String,
        challenge: Vec<u8>,
        msg_csid: i64,
        msg_ts: i64,
    }

    impl ControlledEgressListener for Rec {
        fn on_message(&mut self, csid: i64, ts: i64, _buf: &[u8]) -> ControlledPollAction {
            self.calls += 1;
            self.msg_csid = csid;
            self.msg_ts = ts;
            ControlledPollAction::Continue
        }
        fn on_session_event(
            &mut self, _cid: i64, _sid: i64, _tid: i64, _mid: i32,
            code: EventCode, detail: &str,
        ) -> ControlledPollAction {
            self.calls += 1;
            self.session_code = Some(code);
            self.detail = detail.to_string();
            ControlledPollAction::Continue
        }
        fn on_new_leader(
            &mut self, _sid: i64, _tid: i64, _mid: i32, eps: &str,
        ) -> ControlledPollAction {
            self.calls += 1;
            self.leader_endpoints = eps.to_string();
            ControlledPollAction::Continue
        }
        fn on_challenge(&mut self, _cid: i64, _sid: i64, chal: &[u8]) -> ControlledPollAction {
            self.calls += 1;
            self.challenge = chal.to_vec();
            ControlledPollAction::Continue
        }
        fn on_admin_response(
            &mut self, _sid: i64, _cid: i64, _rt: AdminRequestType,
            _rc: AdminResponseCode, msg: &str, _pl: &[u8],
        ) -> ControlledPollAction {
            self.calls += 1;
            self.detail = msg.to_string();
            ControlledPollAction::Continue
        }
    }
}
