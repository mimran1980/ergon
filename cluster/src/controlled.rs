//! Controlled egress polling — mirrors Java `ControlledEgressAdapter` /
//! `ControlledEgressListener`. Callbacks return a `ControlledPollAction`
//! so the application can apply backpressure (Abort) or stop (Break).
//!
//! Uses the shared `Fragment::decode` path — the
//! same canonical dispatch used by the regular egress path.

use crate::ClusterError;
use crate::codecs::session::{AdminRequestType, AdminResponseCode, EventCode};
use crate::fragment::Fragment;

/// Action returned by a `ControlledEgressListener`.
///
/// Mirrors Aeron's `ControlledFragmentHandler.Action`:
/// - `Continue` — keep dispatching fragments
/// - `Abort` — stop dispatching and re-deliver this fragment next poll
/// - `Break` — stop dispatching; do not re-deliver
/// - `Commit` — commit the current position and continue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledPollAction {
    /// Keep dispatching fragments.
    Continue,
    /// Stop and re-deliver this fragment next poll.
    Abort,
    /// Stop; do not re-deliver this fragment.
    Break,
    /// Commit the current position and continue.
    Commit,
}

/// Controlled variant of `EgressListener`. Only `on_message` returns an
/// action — lifecycle, challenge, and admin callbacks default to no-ops.
pub trait ControlledEgressListener {
    /// Application message from the cluster. Return the desired poll action.
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]) -> ControlledPollAction;

    /// Session lifecycle event (connect result, state change). Default: no-op.
    fn on_session_event(
        &mut self,
        _correlation_id: i64,
        _cluster_session_id: i64,
        _leadership_term_id: i64,
        _leader_member_id: i32,
        _code: EventCode,
        _detail: &str,
    ) {
    }
    /// New leader elected. Default: no-op.
    fn on_new_leader(
        &mut self,
        _cluster_session_id: i64,
        _leadership_term_id: i64,
        _leader_member_id: i32,
        _ingress_endpoints: &str,
    ) {
    }
    /// Auth challenge from the cluster. Default: no-op.
    fn on_challenge(&mut self, _correlation_id: i64, _cluster_session_id: i64, _encoded_challenge: &[u8]) {}
    /// Administrative response (heartbeat ack, etc.). Default: no-op.
    fn on_admin_response(
        &mut self,
        _cluster_session_id: i64,
        _correlation_id: i64,
        _request_type: AdminRequestType,
        _response_code: AdminResponseCode,
        _message: &str,
        _payload: &[u8],
    ) {
    }
}

/// Dispatch egress fragments to a `ControlledEgressListener`.
pub struct ControlledEgressAdapter<L: ControlledEgressListener> {
    listener: L,
    expected_session_id: Option<i64>,
}

impl<L: ControlledEgressListener> ControlledEgressAdapter<L> {
    /// Create an adapter that dispatches all fragments (no session filter).
    pub fn new(listener: L) -> Self {
        Self {
            listener,
            expected_session_id: None,
        }
    }

    /// Create an adapter that only dispatches fragments matching `session_id`.
    /// Messages from other sessions are silently dropped.
    pub fn with_session_filter(listener: L, session_id: i64) -> Self {
        Self {
            listener,
            expected_session_id: Some(session_id),
        }
    }

    /// Update the session filter after construction.
    pub fn set_expected_session_id(&mut self, id: i64) {
        self.expected_session_id = Some(id);
    }

    /// Immutable borrow of the inner listener.
    pub fn listener(&self) -> &L {
        &self.listener
    }

    /// Mutable listener accessor — allows state updates between polls.
    pub fn listener_mut(&mut self) -> &mut L {
        &mut self.listener
    }

    /// Consume the adapter and return the inner listener.
    pub fn into_listener(self) -> L {
        self.listener
    }

    /// Decode and dispatch one egress fragment.
    ///
    /// Decode / protocol errors are returned to the caller (not
    /// silently swallowed as `Continue`). The app-level backpressure
    /// path still returns `Abort` on success.
    pub fn on_fragment(&mut self, data: &[u8]) -> Result<ControlledPollAction, ClusterError> {
        let frag = match Fragment::decode(data)? {
            Some(f) => f,
            None => return Ok(ControlledPollAction::Continue),
        };
        self.dispatch_fragment(frag)
    }

    /// Dispatch a pre-decoded [`Fragment`]. Use when the caller already
    /// decoded (e.g. one decode shared between state tracking and dispatch
    /// on the poll path).
    #[inline]
    pub(crate) fn dispatch_fragment(&mut self, frag: Fragment<'_>) -> Result<ControlledPollAction, ClusterError> {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.dispatch_controlled(frag)));
        match result {
            Ok(r) => r,
            Err(_) => Err(crate::ClusterError::ListenerPanicked {
                context: "controlled dispatch",
            }),
        }
    }

    fn dispatch_controlled(&mut self, frag: Fragment<'_>) -> Result<ControlledPollAction, ClusterError> {
        Ok(match frag {
            Fragment::Message {
                cluster_session_id,
                timestamp,
                payload,
            } => {
                if let Some(expected) = self.expected_session_id
                    && cluster_session_id != expected
                {
                    return Ok(ControlledPollAction::Continue);
                }
                self.listener.on_message(cluster_session_id, timestamp, payload)
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
                    return Ok(ControlledPollAction::Continue);
                }
                self.listener.on_session_event(
                    correlation_id,
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    code,
                    detail,
                );
                ControlledPollAction::Continue
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
                    return Ok(ControlledPollAction::Continue);
                }
                self.listener.on_new_leader(
                    cluster_session_id,
                    leadership_term_id,
                    leader_member_id,
                    ingress_endpoints,
                );
                ControlledPollAction::Continue
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
                    return Ok(ControlledPollAction::Continue);
                }
                self.listener
                    .on_challenge(correlation_id, cluster_session_id, encoded_challenge);
                ControlledPollAction::Continue
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
                    return Ok(ControlledPollAction::Continue);
                }
                self.listener.on_admin_response(
                    cluster_session_id,
                    correlation_id,
                    request_type,
                    response_code,
                    message,
                    payload,
                );
                ControlledPollAction::Continue
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::session::SessionMessageHeaderEncoder;

    #[test]
    fn test_all_actions_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
        let actions = [
            ControlledPollAction::Continue,
            ControlledPollAction::Abort,
            ControlledPollAction::Break,
            ControlledPollAction::Commit,
        ];
        for (i, a) in actions.iter().enumerate() {
            for (j, b) in actions.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "{a:?} and {b:?} must not be equal");
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_controlled_session_message_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.cluster_session_id(42).leadership_term_id(1).timestamp(100);
        let payload = b"hello";
        let data = &buf[..SessionMessageHeaderEncoder::ENCODED_LENGTH];
        let mut full = Vec::from(data);
        full.extend_from_slice(payload);

        struct Rec {
            session_id: i64,
            ts: i64,
            pl: Vec<u8>,
        }
        impl ControlledEgressListener for Rec {
            fn on_message(&mut self, sid: i64, ts: i64, buf: &[u8]) -> ControlledPollAction {
                self.session_id = sid;
                self.ts = ts;
                self.pl = buf.to_vec();
                ControlledPollAction::Continue
            }
        }

        let mut adapter = ControlledEgressAdapter::new(Rec {
            session_id: 0,
            ts: 0,
            pl: vec![],
        });
        let action = adapter.on_fragment(&full)?;
        assert_eq!(action, ControlledPollAction::Continue);
        assert_eq!(adapter.listener.session_id, 42);
        assert_eq!(adapter.listener.ts, 100);
        assert_eq!(adapter.listener.pl, payload);
        Ok(())
    }

    #[test]
    fn test_controlled_session_filter_drops_foreign_session() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.cluster_session_id(99).leadership_term_id(1).timestamp(0);
        let mut full = Vec::from(&buf[..SessionMessageHeaderEncoder::ENCODED_LENGTH]);
        full.extend_from_slice(b"x");

        struct Rec(bool);
        impl ControlledEgressListener for Rec {
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledPollAction {
                self.0 = true;
                ControlledPollAction::Continue
            }
        }

        let mut adapter = ControlledEgressAdapter::with_session_filter(Rec(false), 42);
        let action = adapter.on_fragment(&full)?;
        assert_eq!(action, ControlledPollAction::Continue);
        assert!(!adapter.listener.0, "foreign session message must be dropped");
        Ok(())
    }

    // Controlled dispatch lifecycle-event tests (~230 lines covering all 6
    // message types) live in the shared `codecs::tests` module. Those test
    // patterns remain valid because the Fragment::decode output is identical
    // to the previous inline dispatch — the observable behaviour is unchanged.

    #[test]
    fn test_invalid_frame_returns_err() -> Result<(), Box<dyn std::error::Error>> {
        // T2: Controlled adapter must return Err on malformed frames, not
        // silently Continue.  Craft a SessionMessageHeader with correct
        // template_id but truncated data (< ENCODED_LENGTH = 32).
        let hdr: [u8; 8] = [16, 0, 1, 0, 111, 0, 16, 0]; // template=1 schema=111
        let short = &hdr[..]; // 8 bytes, well below ENCODED_LENGTH (32)
        struct Rec;
        impl ControlledEgressListener for Rec {
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledPollAction {
                ControlledPollAction::Continue
            }
        }
        let mut adapter = ControlledEgressAdapter::new(Rec);
        let result = adapter.on_fragment(short);
        assert!(result.is_err(), "truncated frame must yield Err, got {result:?}");
        Ok(())
    }

    #[test]
    fn test_foreign_session_new_leader_ignored() -> Result<(), Box<dyn std::error::Error>> {
        // T3: Wrong-session NewLeader must not call listener or affect state.
        // Encode a real NewLeaderEvent (enc, ingress_endpoints)
        use crate::codecs::session::NewLeaderEventEncoder;
        let eps = b"0=localhost:9000";
        let len = NewLeaderEventEncoder::compute_encoded_length_with_message_header(eps.len());
        let mut buf = vec![0u8; len];
        let _ = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&crate::codecs::session::NewLeaderEventFixedFields {
                leadership_term_id: 1,
                cluster_session_id: 99,
                leader_member_id: 0,
            })
            .ingress_endpoints(eps)?;
        struct Rec {
            called: bool,
        }
        impl ControlledEgressListener for Rec {
            fn on_new_leader(&mut self, _csid: i64, _ltid: i64, _lmid: i32, _eps: &str) {
                self.called = true;
            }
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledPollAction {
                ControlledPollAction::Continue
            }
        }
        // Expect session 42 — actual is 99 → filter must drop.
        let mut adapter = ControlledEgressAdapter::with_session_filter(Rec { called: false }, 42);
        let action = adapter.on_fragment(&buf)?;
        assert_eq!(action, ControlledPollAction::Continue);
        assert!(!adapter.listener.called, "wrong-session NewLeader must be dropped");
        Ok(())
    }
}
