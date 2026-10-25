//! Egress dispatch: decoded cluster→client messages to an [`EgressListener`].
//!
//! [`crate::AeronCluster::poll`] decodes each reassembled fragment once,
//! drops what belongs to another session, and calls the listener.

use crate::codecs::session::{AdminRequestType, AdminResponseCode, EventCode};
use crate::error::ClusterError;
use crate::fragment::Fragment;

/// Callbacks for egress (cluster→client) messages, passed to
/// [`crate::AeronCluster::poll`].
///
/// Only [`Self::on_message`] is required. The lifecycle callbacks default to
/// no-ops: the client acts on session events and new leaders itself, so
/// implement them only to observe them.
///
/// | Callback | Carries |
/// |---|---|
/// | `on_message` | application payload after the `SessionMessageHeader` |
/// | `on_message_with_reserved_value` | the same, plus the Aeron header's reserved value |
/// | `on_session_event` | OK, ERROR, REDIRECT, `AUTHENTICATION_REJECTED`, CLOSED ([`EventCode`]) |
/// | `on_new_leader` | a new leader's term, member id and `id=host:port,…` endpoints |
/// | `on_challenge` | an auth challenge after connect; reported only (connect answers its own through [`CredentialsSupplier`](crate::CredentialsSupplier)) |
/// | `on_admin_response` | the reply to [`send_admin_request_to_take_snapshot`](crate::AeronCluster::send_admin_request_to_take_snapshot) |
/// | `on_extension_message` | a frame of another template or schema; not an error |
///
/// Only this session's messages reach the listener: every event carrying
/// another `cluster_session_id` is dropped. Text fields are validated before
/// they reach a callback. A decode error or a listener panic is returned from
/// the poll as [`ClusterError`]: a panic becomes
/// [`ClusterError::ListenerPanicked`].
pub trait EgressListener {
    /// Application message from the clustered service. `buffer` is the raw
    /// payload bytes after the `SessionMessageHeader` (no SBE framing).
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]);

    /// [`Self::on_message`] plus the Aeron header's reserved value.
    ///
    /// The default calls [`Self::on_message`] and ignores `reserved_value`.
    /// Override this when the clustered service stamps a send timestamp.
    fn on_message_with_reserved_value(
        &mut self,
        cluster_session_id: i64,
        timestamp: i64,
        buffer: &[u8],
        reserved_value: i64,
    ) {
        let _ = reserved_value;
        self.on_message(cluster_session_id, timestamp, buffer);
    }

    /// A frame this client does not decode (unknown template or schema).
    ///
    /// `buffer` is the payload after the 8-byte SBE header. The default
    /// ignores it. A schema mismatch is not an error: the poll continues.
    fn on_extension_message(
        &mut self,
        acting_block_length: u16,
        template_id: u16,
        schema_id: u16,
        acting_version: u16,
        buffer: &[u8],
    ) {
        let _ = (acting_block_length, template_id, schema_id, acting_version, buffer);
    }

    /// Cluster→client session lifecycle event. `detail` is validated ASCII
    /// text. The default ignores it.
    fn on_session_event(
        &mut self,
        correlation_id: i64,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        code: EventCode,
        detail: &str,
    ) {
        let _ = (
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail,
        );
    }

    /// A new leader was elected. `ingress_endpoints` is a CSV of
    /// `member_id=host:port` pairs. The client reconnects itself; the
    /// default ignores the event.
    fn on_new_leader(
        &mut self,
        cluster_session_id: i64,
        leadership_term_id: i64,
        leader_member_id: i32,
        ingress_endpoints: &str,
    ) {
        let _ = (
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
        );
    }

    /// An auth challenge from the cluster: the opaque bytes of the
    /// `Challenge` message. The default ignores it.
    fn on_challenge(&mut self, correlation_id: i64, cluster_session_id: i64, encoded_challenge: &[u8]) {
        let _ = (correlation_id, cluster_session_id, encoded_challenge);
    }

    /// Response to an admin request (a snapshot). `message` is validated
    /// UTF-8; `payload` is the raw bytes after it. The default ignores it.
    fn on_admin_response(
        &mut self,
        cluster_session_id: i64,
        correlation_id: i64,
        request_type: AdminRequestType,
        response_code: AdminResponseCode,
        message: &str,
        payload: &[u8],
    ) {
        let _ = (
            cluster_session_id,
            correlation_id,
            request_type,
            response_code,
            message,
            payload,
        );
    }
}

/// A listener that ignores everything.
pub struct NullListener;

impl EgressListener for NullListener {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
}

/// Decode `data` and deliver it to `listener` when it is `session_id`'s.
#[cfg(test)]
pub(crate) fn deliver<L: EgressListener>(
    listener: &mut L,
    session_id: i64,
    data: &[u8],
    reserved_value: i64,
) -> Result<(), ClusterError> {
    match Fragment::decode(data)? {
        Some(frag) => dispatch(listener, session_id, frag, reserved_value),
        None => dispatch_extension(listener, data),
    }
}

/// Deliver a decoded fragment to `listener` unless it carries another
/// session's id. A listener panic becomes [`ClusterError::ListenerPanicked`].
#[inline]
pub(crate) fn dispatch<L: EgressListener>(
    listener: &mut L,
    session_id: i64,
    frag: Fragment<'_>,
    reserved_value: i64,
) -> Result<(), ClusterError> {
    if frag.cluster_session_id() != session_id {
        return Ok(());
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(listener, frag, reserved_value))).map_err(|_| {
        ClusterError::ListenerPanicked {
            context: "egress dispatch",
        }
    })
}

/// A frame of an unknown template. Nothing is delivered when `data` has no
/// SBE header.
pub(crate) fn dispatch_extension<L: EgressListener>(listener: &mut L, data: &[u8]) -> Result<(), ClusterError> {
    let Some(view) = crate::fragment::extension_view(data) else {
        return Ok(());
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        listener.on_extension_message(
            view.acting_block_length,
            view.template_id,
            view.schema_id,
            view.acting_version,
            view.payload,
        );
    }))
    .map_err(|_| ClusterError::ListenerPanicked {
        context: "egress extension",
    })
}

fn call<L: EgressListener>(listener: &mut L, frag: Fragment<'_>, reserved_value: i64) {
    match frag {
        Fragment::Message {
            cluster_session_id,
            timestamp,
            payload,
        } => listener.on_message_with_reserved_value(cluster_session_id, timestamp, payload, reserved_value),
        Fragment::SessionEvent {
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail,
            leader_heartbeat_timeout_ns: _,
        } => listener.on_session_event(
            correlation_id,
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            code,
            detail,
        ),
        Fragment::NewLeader {
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
        } => listener.on_new_leader(
            cluster_session_id,
            leadership_term_id,
            leader_member_id,
            ingress_endpoints,
        ),
        Fragment::Challenge {
            correlation_id,
            cluster_session_id,
            encoded_challenge,
        } => listener.on_challenge(correlation_id, cluster_session_id, encoded_challenge),
        Fragment::AdminResponse {
            cluster_session_id,
            correlation_id,
            request_type,
            response_code,
            message,
            payload,
        } => listener.on_admin_response(
            cluster_session_id,
            correlation_id,
            request_type,
            response_code,
            message,
            payload,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::session::EventCode;
    use crate::codecs::session::{SessionEventEncoder, SessionEventFixedFields};

    /// A listener that implements only `on_message`: every lifecycle
    /// callback has a no-op default, and the client filters by session.
    #[derive(Default)]
    struct OnlyMessages {
        seen: Vec<(i64, Vec<u8>)>,
    }

    impl EgressListener for OnlyMessages {
        fn on_message(&mut self, cluster_session_id: i64, _timestamp: i64, buffer: &[u8]) {
            self.seen.push((cluster_session_id, buffer.to_vec()));
        }
    }

    fn message(cluster_session_id: i64, payload: &[u8]) -> Vec<u8> {
        use crate::codecs::session::SessionMessageHeaderEncoder;
        let mut header = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH];
        SessionMessageHeaderEncoder::wrap_and_apply_header(&mut header, 0)
            .leadership_term_id(7)
            .cluster_session_id(cluster_session_id)
            .timestamp(100);
        let mut frame = header.to_vec();
        frame.extend_from_slice(payload);
        frame
    }

    #[test]
    fn a_listener_needs_only_on_message_and_sees_only_its_session() -> Result<(), Box<dyn std::error::Error>> {
        let mut listener = OnlyMessages::default();
        deliver(&mut listener, 42, &message(42, b"mine"), 0)?;
        deliver(&mut listener, 42, &message(99, b"theirs"), 0)?;
        let event = session_event(42)?;
        deliver(&mut listener, 42, &event, 0)?;
        assert_eq!(listener.seen, vec![(42, b"mine".to_vec())]);
        Ok(())
    }

    fn session_event(cluster_session_id: i64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let detail = b"ok";
        let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
        let mut buf = vec![0u8; len];
        let _ = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&SessionEventFixedFields {
                cluster_session_id,
                correlation_id: 1,
                leadership_term_id: 5,
                leader_member_id: 0,
                code: EventCode::OK,
                version: Some(1),
                leader_heartbeat_timeout_ns: None,
            })
            .detail(detail)?;
        Ok(buf)
    }

    #[test]
    fn test_foreign_session_session_event_ignored() -> Result<(), Box<dyn std::error::Error>> {
        // T4: Wrong-session SessionEvent must not call listener.
        let detail = b"ok";
        let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
        let mut buf = vec![0u8; len];
        let _ = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&SessionEventFixedFields {
                cluster_session_id: 99,
                correlation_id: 1,
                leadership_term_id: 5,
                leader_member_id: 0,
                code: EventCode::OK,
                version: Some(1),
                leader_heartbeat_timeout_ns: None,
            })
            .detail(detail)?;
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
        let mut listener = Rec { called: false };
        deliver(&mut listener, 42, &buf, 0)?;
        assert!(!listener.called, "wrong-session SessionEvent must be dropped");
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
        let mut listener = Rec {
            sid: 0,
            ts: 0,
            pl: vec![],
        };
        deliver(&mut listener, 42, &full, 0)?;
        assert_eq!(listener.sid, 42);
        assert_eq!(listener.ts, 100);
        assert_eq!(listener.pl, payload);
        Ok(())
    }

    #[test]
    fn test_on_new_leader_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::NewLeaderEventEncoder;
        let eps = b"0=localhost:9000,1=localhost:9001";
        let len = NewLeaderEventEncoder::compute_encoded_length_with_message_header(eps.len());
        let mut buf = vec![0u8; len];
        let _ = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&crate::codecs::session::NewLeaderEventFixedFields {
                leadership_term_id: 3,
                cluster_session_id: 42,
                leader_member_id: 1,
            })
            .ingress_endpoints(eps)?;
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
        let mut listener = Rec {
            sid: 0,
            ltid: 0,
            lmid: 0,
            eps: String::new(),
        };
        deliver(&mut listener, 42, &buf, 0)?;
        assert_eq!(listener.sid, 42);
        assert_eq!(listener.ltid, 3);
        assert_eq!(listener.lmid, 1);
        assert_eq!(listener.eps, "0=localhost:9000,1=localhost:9001");
        Ok(())
    }

    #[test]
    fn test_on_challenge_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        use crate::codecs::session::ChallengeEncoder;
        let chal = b"server-challenge-data";
        let len = ChallengeEncoder::compute_encoded_length_with_message_header(chal.len());
        let mut buf = vec![0u8; len];
        let _ = ChallengeEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&crate::codecs::session::ChallengeFixedFields {
                correlation_id: 99,
                cluster_session_id: 42,
            })
            .encoded_challenge(chal)?;
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
        let mut listener = Rec {
            cid: 0,
            sid: 0,
            chal: vec![],
        };
        deliver(&mut listener, 42, &buf, 0)?;
        assert_eq!(listener.cid, 99);
        assert_eq!(listener.sid, 42);
        assert_eq!(listener.chal, chal);
        Ok(())
    }

    proptest::proptest! {
        /// Arbitrary bytes through egress dispatch never panic.
        #[test]
        fn arbitrary_bytes_do_not_panic(data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..512)) {
            let _ = deliver(&mut NullListener, 42, &data, 0);
        }
    }
}
