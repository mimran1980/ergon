//! Egress fragment dispatch to an [`EgressListener`].
//!
//! Uses the shared `Fragment::decode` path — the
//! same canonical dispatch used by the controlled and poller paths.

use crate::codecs::session::{AdminRequestType, AdminResponseCode, EventCode};
use crate::fragment::Fragment;

/// Callbacks for egress (cluster→client) messages received via
/// [`crate::AeronCluster::poll_egress`]. Implement this trait and pass it to an
/// [`EgressAdapter`].
///
/// # Lifecycle
///
/// - **`on_session_event`** delivers the cluster's response to connect,
///   keep-alive, and close requests via [`EventCode`] (OK, ERROR, REDIRECT,
///   AUTHENTICATION_REJECTED, CLOSED). The `detail` field is validated ASCII
///   text from the schema and carries human-readable context (e.g. the
///   redirect leader endpoint list).
/// - **`on_new_leader`** fires when the cluster elects a new leader. The
///   `ingress_endpoints` CSV is the same format that
///   [`SessionBuilder::ingress_endpoints`](crate::SessionBuilder::ingress_endpoints)
///   accepts. The client handles reconnection internally — implement this to
///   observe leadership changes.
/// - **`on_challenge`** delivers an auth challenge from the cluster.
///   Respond with credentials via
///   [`CredentialsSupplier::on_challenge`](crate::CredentialsSupplier::on_challenge);
///   the client handles the protocol-level `ChallengeResponse` send.
/// - **`on_admin_response`** carries the cluster's reply to an admin request
///   (e.g. snapshot) initiated by
///   [`AeronCluster::send_admin_request_to_take_snapshot`](crate::AeronCluster::send_admin_request_to_take_snapshot).
/// - **`on_message`** delivers application payloads from the clustered
///   service. Filter by `cluster_session_id` if your process manages multiple
///   sessions; the adapter already drops message for foreign sessions when
///   `expected_session_id` is set, so this callback only sees messages for
///   the configured session.
///
/// # Error handling
///
/// All methods are **infallible** — they return `()`. Protocol decode errors
/// and listener panics are buffered and surfaced as
/// [`ClusterError`](crate::ClusterError) on the next
/// [`poll_egress`](crate::AeronCluster::poll_egress) return. Do not `panic!`
/// inside these callbacks; the adapter wraps dispatch in `catch_unwind` and
/// returns [`ClusterError::ListenerPanicked`](crate::ClusterError::ListenerPanicked).
///
/// # Session filtering
///
/// When you call
/// [`EgressAdapter::with_session_filter`] or
/// [`set_expected_session_id`](EgressAdapter::set_expected_session_id),
/// the adapter drops **all** event types (messages, session events,
/// new-leader notifications, challenges, admin responses) that carry a
/// `cluster_session_id` different from the configured one. This prevents a
/// multi-tenant process from acting on another session's lifecycle events.
pub trait EgressListener {
    /// Application message from the clustered service. `buffer` is the raw
    /// payload bytes after the `SessionMessageHeader` (no SBE framing).
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]);

    /// Cluster→client session lifecycle event. See [`EventCode`] for the
    /// possible codes. `detail` is validated ASCII text.
    fn on_session_event(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        code: EventCode,
        detail: &str,
    );

    /// A new leader was elected. `ingress_endpoints` is a CSV of
    /// `member_id=host:port` pairs. The client reconnects automatically;
    /// implement this to observe the transition.
    fn on_new_leader(
        &mut self,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        ingress_endpoints: &str,
    );

    /// An auth challenge from the cluster. `encoded_challenge` is the
    /// opaque challenge bytes from the `Challenge` SBE message.
    fn on_challenge(&mut self, correlation_id: i64, cluster_session_id: i64, encoded_challenge: &[u8]);

    /// Response to an admin request (e.g. snapshot). `message` is
    /// validated UTF-8 text; `payload` is raw bytes after the message
    /// field.
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

    /// Consume the adapter and return the inner listener.
    pub fn into_listener(self) -> L {
        self.listener
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
    use crate::codecs::session::EventCode;
    use crate::codecs::session::SessionEventEncoder;

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

    #[test]
    fn test_on_message_dispatch_and_values() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::SessionMessageHeaderEncoder;
        let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH + 5];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
            .leadership_term_id(7)
            .cluster_session_id(42)
            .timestamp(100);
        let payload = b"hello";
        let mut full = Vec::from(&buf[..SessionMessageHeaderEncoder::ENCODED_LENGTH]);
        full.extend_from_slice(payload);
        struct Rec {
            sid: i64,
            ts: i64,
            pl: Vec<u8>,
        }
        impl EgressListener for Rec {
            fn on_message(&mut self, s: i64, t: i64, b: &[u8]) {
                self.sid = s;
                self.ts = t;
                self.pl = b.to_vec();
            }
            fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
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
        let mut adapter = EgressAdapter::new(Rec {
            sid: 0,
            ts: 0,
            pl: vec![],
        });
        adapter.on_fragment(&full)?;
        assert_eq!(adapter.listener.sid, 42);
        assert_eq!(adapter.listener.ts, 100);
        assert_eq!(adapter.listener.pl, payload);
        Ok(())
    }

    #[test]
    fn test_on_new_leader_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::NewLeaderEventEncoder;
        let eps = b"0=localhost:9000,1=localhost:9001";
        let len = NewLeaderEventEncoder::compute_encoded_length_with_message_header(eps.len());
        let mut buf = vec![0u8; len];
        let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.leadership_term_id(3).cluster_session_id(42).leader_member_id(1);
        let _ = enc.ingress_endpoints(eps)?;
        struct Rec {
            sid: i64,
            ltid: i64,
            lmid: i32,
            eps: String,
        }
        impl EgressListener for Rec {
            fn on_new_leader(&mut self, s: i64, l: i64, m: i32, e: &str) {
                self.sid = s;
                self.ltid = l;
                self.lmid = m;
                self.eps = e.to_string();
            }
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
            fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
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
        let mut adapter = EgressAdapter::new(Rec {
            sid: 0,
            ltid: 0,
            lmid: 0,
            eps: String::new(),
        });
        adapter.on_fragment(&buf)?;
        assert_eq!(adapter.listener.sid, 42);
        assert_eq!(adapter.listener.ltid, 3);
        assert_eq!(adapter.listener.lmid, 1);
        assert_eq!(adapter.listener.eps, "0=localhost:9000,1=localhost:9001");
        Ok(())
    }

    #[test]
    fn test_on_challenge_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::ChallengeEncoder;
        let chal = b"server-challenge-data";
        let len = ChallengeEncoder::compute_encoded_length_with_message_header(chal.len());
        let mut buf = vec![0u8; len];
        let mut enc = ChallengeEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(99).cluster_session_id(42);
        let _ = enc.encoded_challenge(chal)?;
        struct Rec {
            cid: i64,
            sid: i64,
            chal: Vec<u8>,
        }
        impl EgressListener for Rec {
            fn on_challenge(&mut self, c: i64, s: i64, d: &[u8]) {
                self.cid = c;
                self.sid = s;
                self.chal = d.to_vec();
            }
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
            fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
            fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) {}
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
        let mut adapter = EgressAdapter::new(Rec {
            cid: 0,
            sid: 0,
            chal: vec![],
        });
        adapter.on_fragment(&buf)?;
        assert_eq!(adapter.listener.cid, 99);
        assert_eq!(adapter.listener.sid, 42);
        assert_eq!(adapter.listener.chal, chal);
        Ok(())
    }
}
