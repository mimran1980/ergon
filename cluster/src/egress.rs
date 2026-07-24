//! Egress fragment dispatch to an [`EgressListener`].
//!
//! Uses the shared `Fragment::decode` path — the
//! same canonical dispatch used by the controlled and poller paths.

use crate::codecs::session::{AdminRequestType, AdminResponseCode, EventCode};
use crate::fragment::Fragment;

/// Callbacks for egress (cluster→client) messages.
///
/// All methods are infallible — errors are buffered and surfaced on
/// the next `poll()` return. Panics must never unwind through this
/// trait; the adapter wraps dispatch in `catch_unwind`.
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
    expected_session_id: Option<i64>,
}

impl<L: EgressListener> EgressAdapter<L> {
    pub fn new(listener: L) -> Self {
        Self {
            listener,
            expected_session_id: None,
        }
    }

    pub fn with_session_filter(listener: L, session_id: i64) -> Self {
        Self {
            listener,
            expected_session_id: Some(session_id),
        }
    }

    pub fn set_expected_session_id(&mut self, id: i64) {
        self.expected_session_id = Some(id);
    }

    pub fn listener_mut(&mut self) -> &mut L {
        &mut self.listener
    }

    pub fn listener(&self) -> &L {
        &self.listener
    }

    /// Decode and dispatch one egress fragment. Listener callbacks are
    /// wrapped in `catch_unwind` — panics become `ListenerPanicked`.
    /// Returns `Ok(true)` if dispatched, `Ok(false)` for unknown types.
    pub fn on_fragment(&mut self, data: &[u8]) -> Result<bool, crate::ClusterError> {
        let frag = match Fragment::decode(data)? {
            Some(f) => f,
            None => return Ok(false),
        };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.dispatch(frag);
        }));
        match result {
            Ok(()) => Ok(true),
            Err(_) => Err(crate::ClusterError::ListenerPanicked {
                context: "egress dispatch",
            }),
        }
    }

    fn dispatch(&mut self, frag: Fragment<'_>) {
        match frag {
            Fragment::Message {
                cluster_session_id,
                timestamp,
                payload,
            } => {
                if self
                    .expected_session_id
                    .is_some_and(|expected| cluster_session_id != expected)
                {
                    return;
                }
                self.listener.on_message(cluster_session_id, timestamp, payload);
            }
            Fragment::SessionEvent {
                correlation_id,
                cluster_session_id,
                leadership_term_id,
                leader_member_id,
                code,
                detail,
            } => {
                if self
                    .expected_session_id
                    .is_some_and(|expected| cluster_session_id != expected)
                {
                    return;
                }
                self.listener.on_session_event(
                    correlation_id,
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    code,
                    detail,
                );
            }
            Fragment::NewLeader {
                cluster_session_id,
                leadership_term_id,
                leader_member_id,
                ingress_endpoints,
            } => {
                if self
                    .expected_session_id
                    .is_some_and(|expected| cluster_session_id != expected)
                {
                    return;
                }
                self.listener.on_new_leader(
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    ingress_endpoints,
                );
            }
            Fragment::Challenge {
                correlation_id,
                cluster_session_id,
                encoded_challenge,
            } => {
                if self
                    .expected_session_id
                    .is_some_and(|expected| cluster_session_id != expected)
                {
                    return;
                }
                self.listener
                    .on_challenge(correlation_id, cluster_session_id, encoded_challenge);
            }
            Fragment::AdminResponse {
                cluster_session_id,
                correlation_id,
                request_type,
                response_code,
                message,
                payload,
            } => {
                if self
                    .expected_session_id
                    .is_some_and(|expected| cluster_session_id != expected)
                {
                    return;
                }
                self.listener.on_admin_response(
                    cluster_session_id,
                    correlation_id,
                    request_type,
                    response_code,
                    message,
                    payload,
                );
            }
        }
    }
}

/// No-op listener for tests.
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
    use crate::codecs::session::SessionEventEncoder;
    use crate::codecs::session::EventCode;

    #[test]
    fn test_foreign_session_session_event_ignored() -> Result<(), Box<dyn std::error::Error>> {
        // T4: Wrong-session SessionEvent must not call listener.
        let detail = b"ok";
        let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
        let mut buf = vec![0u8; len];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.cluster_session_id(99)
            .correlation_id(1)
            .leadership_term_id(5)
            .leader_member_id(0)
            .code(EventCode::OK)
            .version(1);
        let _ = enc.detail(detail)?;
        struct Rec {
            called: bool,
        }
        impl EgressListener for Rec {
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
            fn on_session_event(
                &mut self,
                _cid: i64,
                _csid: i64,
                _ltid: i64,
                _lmid: i32,
                _code: EventCode,
                _detail: &str,
            ) {
                self.called = true;
            }
            fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) {}
            fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
            fn on_admin_response(
                &mut self,
                _: i64,
                _: i64,
                _: AdminRequestType,
                _: AdminResponseCode,
                _: &str,
                _: &[u8],
            ) {
            }
        }
        // Expect session 42 — actual is 99 → filter must drop.
        let mut adapter = EgressAdapter::with_session_filter(Rec { called: false }, 42);
        adapter.on_fragment(&buf)?;
        assert!(!adapter.listener.called, "wrong-session SessionEvent must be dropped");
        Ok(())
    }
}
