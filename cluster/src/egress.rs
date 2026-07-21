//! Egress fragment dispatch to an [`EgressListener`].
//!
//! SessionEvent / NewLeader / SessionMessageHeader decoding uses the same
//! ErgoSBE equal-work path as [`crate::decode`].

use crate::codecs::session::SessionMessageHeaderEncoder;
use crate::codecs::session::{AdminRequestType, AdminResponseCode, AnyMessage, EventCode};

/// Decode var-data bytes as UTF-8 with a consistent sentinel on failure.
#[inline]
fn as_utf8_lossy(data: &[u8]) -> &str {
    std::str::from_utf8(data).unwrap_or("<invalid utf-8>")
}

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
///
/// Optionally filters application messages (`SessionMessageHeader`) by
/// `expected_session_id` — messages from other sessions are silently
/// dropped. Lifecycle events (SessionEvent, NewLeader, Challenge,
/// AdminResponse) are always dispatched regardless of the filter.
pub struct EgressAdapter<L: EgressListener> {
    listener: L,
    expected_session_id: Option<i64>,
}

impl<L: EgressListener> EgressAdapter<L> {
    pub fn new(listener: L) -> Self {
        Self {
            listener,
            expected_session_id: None,
        }
    }

    /// Create an adapter that only dispatches `SessionMessageHeader`
    /// messages matching `session_id`. Other message types are unaffected.
    pub fn with_session_filter(listener: L, session_id: i64) -> Self {
        Self {
            listener,
            expected_session_id: Some(session_id),
        }
    }

    /// Update the session-id filter (e.g. after a NewLeaderEvent assigns a new session).
    pub fn set_expected_session_id(&mut self, id: i64) {
        self.expected_session_id = Some(id);
    }

    /// Mutably borrow the wrapped listener (e.g. to inspect captured
    /// messages after a poll; a `&self` borrow is available as `listener()`).
    pub fn listener_mut(&mut self) -> &mut L {
        &mut self.listener
    }

    /// Decode and dispatch one egress fragment via `AnyMessage::decode` —
    /// schema validation and template-id dispatch are handled by the
    /// generated code. Returns `true` if handled, `false` for unknown types.
    ///
    /// Listener callbacks are wrapped in `std::panic::catch_unwind` so a
    /// panicking callback cannot unwind through the Aeron C fragment handler.
    /// Panics are surfaced as [`ClusterError::ListenerPanicked`] on the next
    /// poll return.
    pub fn on_fragment(&mut self, data: &[u8]) -> Result<bool, crate::ClusterError> {
        let msg = match AnyMessage::decode(data, 0) {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };

        // Wrap dispatch in catch_unwind — panics must not unwind through
        // the Aeron C fragment handler callback.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.dispatch_unchecked(data, msg)
        }));
        match result {
            Ok(r) => r,
            Err(_) => Err(crate::ClusterError::ListenerPanicked {
                context: "egress dispatch",
            }),
        }
    }

    /// Inner dispatch without panic safety (called via catch_unwind).
    fn dispatch_unchecked(&mut self, data: &[u8], msg: AnyMessage<'_>) -> Result<bool, crate::ClusterError> {

        match msg {
            AnyMessage::SessionMessageHeader(decoder) => {
                // Filter: drop messages not addressed to our session.
                if let Some(expected) = self.expected_session_id
                    && decoder.cluster_session_id() != expected {
                        return Ok(true); // handled (dropped), not an error
                    }
                if data.len() < SessionMessageHeaderEncoder::ENCODED_LENGTH {
                    return Err(crate::ClusterError::ProtocolError {
                        reason: "session message too short".into(),
                    });
                }
                let payload = &data[SessionMessageHeaderEncoder::ENCODED_LENGTH..];
                self.listener
                    .on_message(decoder.cluster_session_id(), decoder.timestamp(), payload);
                Ok(true)
            }
            AnyMessage::SessionEvent(decoder) => {
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let code = decoder.code();
                let detail = decoder
                    .into_detail()
                    .map(|(b, _)| as_utf8_lossy(b))
                    .unwrap_or("<invalid utf-8>");
                self.listener.on_session_event(cid, csid, ltid, lmid, code, detail);
                Ok(true)
            }
            AnyMessage::NewLeaderEvent(decoder) => {
                let csid = decoder.cluster_session_id();
                let ltid = decoder.leadership_term_id();
                let lmid = decoder.leader_member_id();
                let eps = decoder
                    .into_ingress_endpoints()
                    .map(|(b, _)| as_utf8_lossy(b))
                    .unwrap_or("<invalid utf-8>");
                self.listener.on_new_leader(csid, ltid, lmid, eps);
                Ok(true)
            }
            AnyMessage::Challenge(decoder) => {
                let cid = decoder.correlation_id();
                let csid = decoder.cluster_session_id();
                let chal = decoder.into_encoded_challenge().map(|(b, _)| b).unwrap_or(&[]);
                self.listener.on_challenge(cid, csid, chal);
                Ok(true)
            }
            AnyMessage::AdminResponse(decoder) => {
                let csid = decoder.cluster_session_id();
                let cid = decoder.correlation_id();
                let rt = decoder.request_type();
                let rc = decoder.response_code();
                let (msg_bytes, after_msg) =
                    decoder.into_message().map_err(|e| crate::ClusterError::ProtocolError {
                        reason: format!("admin response message: {e:?}"),
                    })?;
                let (payload_bytes, _) = after_msg
                    .into_payload()
                    .map_err(|e| crate::ClusterError::ProtocolError {
                        reason: format!("admin response payload: {e:?}"),
                    })?;
                let msg = as_utf8_lossy(msg_bytes).to_string();
                self.listener.on_admin_response(csid, cid, rt, rc, &msg, payload_bytes);
                Ok(true)
            }
            AnyMessage::Unknown { .. } => Ok(false),
            _ => Ok(false), // messages not in the cluster protocol
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
    use crate::codecs::session::{
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
        enc.correlation_id(5).cluster_session_id(2);
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
        enc.leadership_term_id(10).cluster_session_id(99).leader_member_id(1);
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
        enc.leadership_term_id(1).cluster_session_id(42).timestamp(999);
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
        enc.leadership_term_id(1).cluster_session_id(1).timestamp(1);
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
