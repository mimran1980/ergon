//! Egress fragment dispatch to an [`EgressListener`].
//!
//! SessionEvent / NewLeader / SessionMessageHeader decoding uses the same
//! ErgoSBE equal-work path as [`crate::decode`].

// Production decode uses ErgoSBE.
use crate::codecs::ergo_codecs::{
    AdminRequestType, AdminResponseCode, AdminResponseDecoder, ChallengeDecoder, EventCode, MessageHeader,
};
use crate::codecs::ergo_codecs::{
    AdminResponseEncoder, ChallengeEncoder, NewLeaderEventEncoder, SessionEventEncoder, SessionMessageHeaderEncoder,
};
use crate::decode::{decode_new_leader_event, decode_session_event, decode_session_message_header};

/// SBE message frame header is always 8 bytes.
const HEADER_LEN: usize = 8;

/// Callbacks for egress (cluster→client) messages.
///
/// All methods are infallible — errors are buffered and surfaced on
/// the next `poll()` return. Panics must never unwind through this
/// trait; the adapter may be invoked under a C `fragment_handler`.
pub trait EgressListener {
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]);
    fn on_session_event(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        code: EventCode,
        detail: &str,
    );
    fn on_new_leader(
        &mut self,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        ingress_endpoints: &str,
    );
    fn on_challenge(&mut self, correlation_id: i64, cluster_session_id: i64, encoded_challenge: &[u8]);
    fn on_admin_response(
        &mut self,
        cluster_session_id: i64,
        correlation_id: i64,
        request_type: AdminRequestType,
        response_code: AdminResponseCode,
        message: &str,
        payload: &[u8],
    );
}

/// Dispatch egress fragments to an `EgressListener`.
pub struct EgressAdapter<L: EgressListener> {
    listener: L,
}

impl<L: EgressListener> EgressAdapter<L> {
    pub fn new(listener: L) -> Self {
        Self { listener }
    }

    /// Mutably borrow the wrapped listener (e.g. to inspect captured
    /// messages after a poll; a `&self` borrow is available as `listener()`).
    pub fn listener_mut(&mut self) -> &mut L {
        &mut self.listener
    }

    /// Decode and dispatch one egress fragment. Returns `true` if
    /// handled, `false` if the templateId is unrecognised.
    pub fn on_fragment(&mut self, data: &[u8]) -> Result<bool, crate::ClusterError> {
        let Some(template_id) = MessageHeader::peek_template_id(data) else {
        return Ok(false);
    };

    match template_id {
            SessionMessageHeaderEncoder::TEMPLATE_ID => {
                if data.len() < HEADER_LEN + 24 {
                    return Err(crate::ClusterError::ProtocolError {
                        reason: "session message too short".into(),
                    });
                }
                let body = decode_session_message_header(data)?;
                let payload = &data[HEADER_LEN + 24..];
                self.listener
                    .on_message(body.cluster_session_id, body.timestamp, payload);
                Ok(true)
            }
            SessionEventEncoder::TEMPLATE_ID => {
                let view = decode_session_event(data)?;
                let detail = std::str::from_utf8(view.detail).unwrap_or("<invalid utf-8>");
                self.listener.on_session_event(
                    view.correlation_id,
                    view.cluster_session_id,
                    view.leadership_term_id,
                    view.leader_member_id,
                    view.code,
                    detail,
                );
                Ok(true)
            }
            NewLeaderEventEncoder::TEMPLATE_ID => {
                let view = decode_new_leader_event(data)?;
                let endpoints = std::str::from_utf8(view.ingress_endpoints).unwrap_or("<invalid utf-8>");
                self.listener.on_new_leader(
                    view.cluster_session_id,
                    view.leadership_term_id,
                    view.leader_member_id,
                    endpoints,
                );
                Ok(true)
            }
            ChallengeEncoder::TEMPLATE_ID => {
                let decoder = ChallengeDecoder::wrap_and_apply_header(data, 0)?;
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let (challenge, _after_challenge) = decoder.into_encoded_challenge()?;
                self.listener.on_challenge(cid, csid, challenge);
                Ok(true)
            }
            AdminResponseEncoder::TEMPLATE_ID => {
                let decoder = AdminResponseDecoder::wrap_and_apply_header(data, 0)?;
                let csid = decoder.cluster_session_id();
                let cid = decoder.correlation_id();
                let rt = decoder.request_type();
                let rc = decoder.response_code();
                let (msg_bytes, after_msg) = decoder.into_message()?;
                let (payload_bytes, _after_payload) = after_msg.into_payload()?;
                let msg = std::str::from_utf8(msg_bytes).unwrap_or("<invalid utf-8>").to_string();
                let payload = payload_bytes.to_vec();
                self.listener.on_admin_response(csid, cid, rt, rc, &msg, &payload);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn listener(&self) -> &L {
        &self.listener
    }
}

/// A no-op listener for tests.
pub struct NullListener;

impl EgressListener for NullListener {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
    fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) {}
    fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_admin_response(&mut self, _: i64, _: i64, _: AdminRequestType, _: AdminResponseCode, _: &str, _: &[u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::ergo_codecs::{
        ChallengeEncoder, EventCode as ErgoEventCode, NewLeaderEventEncoder, SessionEventEncoder,
        SessionMessageHeaderEncoder,
    };

    #[derive(Default)]
    struct Rec {
        calls: usize,
        session_code: Option<EventCode>,
        detail: String,
        leader_endpoints: String,
        challenge: Vec<u8>,
        msg_csid: i64,
        msg_ts: i64,
        admin_msg: String,
    }

    impl EgressListener for Rec {
        fn on_message(&mut self, csid: i64, ts: i64, _buf: &[u8]) {
            self.calls += 1;
            self.msg_csid = csid;
            self.msg_ts = ts;
        }
        fn on_session_event(&mut self, _cid: i64, _sid: i64, _tid: i64, _mid: i32, code: EventCode, detail: &str) {
            self.calls += 1;
            self.session_code = Some(code);
            self.detail = detail.to_string();
        }
        fn on_new_leader(&mut self, _sid: i64, _tid: i64, _mid: i32, eps: &str) {
            self.calls += 1;
            self.leader_endpoints = eps.to_string();
        }
        fn on_challenge(&mut self, _cid: i64, _sid: i64, chal: &[u8]) {
            self.calls += 1;
            self.challenge = chal.to_vec();
        }
        fn on_admin_response(
            &mut self,
            _sid: i64,
            _cid: i64,
            _rt: AdminRequestType,
            _rc: AdminResponseCode,
            msg: &str,
            _pl: &[u8],
        ) {
            self.calls += 1;
            self.admin_msg = msg.to_string();
        }
    }

    #[test]
    fn test_dispatch_session_event_ok() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut data, 0)?;
        let _ = enc
            .cluster_session_id(7)
            .correlation_id(99)
            .leadership_term_id(3)
            .leader_member_id(0)
            .code(ErgoEventCode::OK)
            .version(1);
        let complete = enc.detail(b"ok")?;
        let bytes = complete.as_bytes_with_header().to_vec();

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&bytes)?);
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().session_code, Some(EventCode::OK));
        assert_eq!(adapter.listener().detail, "ok");

        Ok(())
    }

    #[test]
    fn test_dispatch_challenge() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = ChallengeEncoder::wrap_and_apply_header(&mut data, 0)?;
        let _ = enc.correlation_id(5).cluster_session_id(2);
        let complete = enc.encoded_challenge(b"chal-token")?;
        let bytes = complete.as_bytes_with_header().to_vec();

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&bytes)?);
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().challenge, b"chal-token");

        Ok(())
    }

    #[test]
    fn test_dispatch_new_leader() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 256];
        let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut data, 0)?;
        let _ = enc.leadership_term_id(10).cluster_session_id(99).leader_member_id(1);
        let complete = enc.ingress_endpoints(b"0=host:9000,1=host:9001")?;
        let bytes = complete.as_bytes_with_header().to_vec();

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&bytes)?);
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().leader_endpoints, "0=host:9000,1=host:9001");

        Ok(())
    }

    #[test]
    fn test_dispatch_session_message_header() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0)?;
        let _ = enc.leadership_term_id(1).cluster_session_id(42).timestamp(999);
        let bytes = enc.as_ref().to_vec();

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&bytes)?);
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().msg_csid, 42);
        assert_eq!(adapter.listener().msg_ts, 999);

        Ok(())
    }

    #[test]
    fn test_unknown_template_id_returns_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = vec![0u8; 128];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut data, 0)?;
        let _ = enc.leadership_term_id(1).cluster_session_id(1).timestamp(1);
        // Overwrite template_id at bytes 2-3 with 0 (unknown)
        data[2] = 0u8;
        data[3] = 0u8;

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(!adapter.on_fragment(&data)?);
        assert_eq!(adapter.listener().calls, 0);

        Ok(())
    }

    #[test]
    fn test_short_fragment_returns_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(!adapter.on_fragment(&[0u8; 4])?);
        assert_eq!(adapter.listener().calls, 0);

        Ok(())
    }
}
