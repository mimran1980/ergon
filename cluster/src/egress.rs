use crate::codecs::cluster_codecs::{
    ReadBuf,
    admin_request_type::AdminRequestType,
    admin_response_code::AdminResponseCode,
    admin_response_codec::{AdminResponseDecoder, SBE_TEMPLATE_ID as ADMIN_RESPONSE_ID},
    challenge_codec::{ChallengeDecoder, SBE_TEMPLATE_ID as CHALLENGE_ID},
    event_code::EventCode,
    message_header_codec::{ENCODED_LENGTH as HEADER_LEN, MessageHeaderDecoder},
    new_leader_event_codec::{NewLeaderEventDecoder, SBE_TEMPLATE_ID as NEW_LEADER_ID},
    session_event_codec::{SBE_TEMPLATE_ID as SESSION_EVENT_ID, SessionEventDecoder},
    session_message_header_codec::{SBE_TEMPLATE_ID as SESSION_MSG_HEADER_ID, SessionMessageHeaderDecoder},
};

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
        if data.len() < HEADER_LEN {
            return Ok(false);
        }

        let read_buf = ReadBuf::new(data);
        let header = MessageHeaderDecoder::default().wrap(read_buf, 0);
        let template_id = header.template_id();

        match template_id {
            SESSION_MSG_HEADER_ID => {
                if data.len() < HEADER_LEN + 24 {
                    return Err(crate::ClusterError::ProtocolError {
                        reason: "session message too short".into(),
                    });
                }
                let body = SessionMessageHeaderDecoder::default().header(header, 0);
                let payload = &data[HEADER_LEN + 24..];
                self.listener
                    .on_message(body.cluster_session_id(), body.timestamp(), payload);
                Ok(true)
            }
            SESSION_EVENT_ID => {
                let mut decoder = SessionEventDecoder::default().header(header, 0);
                let coords = decoder.detail_decoder();
                let detail = std::str::from_utf8(decoder.detail_slice(coords)).unwrap_or("<invalid utf-8>");
                self.listener.on_session_event(
                    decoder.correlation_id(),
                    decoder.cluster_session_id(),
                    decoder.leadership_term_id(),
                    decoder.leader_member_id(),
                    decoder.code(),
                    detail,
                );
                Ok(true)
            }
            NEW_LEADER_ID => {
                let mut decoder = NewLeaderEventDecoder::default().header(header, 0);
                let coords = decoder.ingress_endpoints_decoder();
                let endpoints =
                    std::str::from_utf8(decoder.ingress_endpoints_slice(coords)).unwrap_or("<invalid utf-8>");
                self.listener.on_new_leader(
                    decoder.cluster_session_id(),
                    decoder.leadership_term_id(),
                    decoder.leader_member_id(),
                    endpoints,
                );
                Ok(true)
            }
            CHALLENGE_ID => {
                let mut decoder = ChallengeDecoder::default().header(header, 0);
                let coords = decoder.encoded_challenge_decoder();
                let challenge = decoder.encoded_challenge_slice(coords);
                self.listener
                    .on_challenge(decoder.correlation_id(), decoder.cluster_session_id(), challenge);
                Ok(true)
            }
            ADMIN_RESPONSE_ID => {
                let mut decoder = AdminResponseDecoder::default().header(header, 0);
                let csid = decoder.cluster_session_id();
                let cid = decoder.correlation_id();
                let rt = decoder.request_type();
                let rc = decoder.response_code();
                // Extract var-data coordinates first (&mut self calls)
                let msg_coords = decoder.message_decoder();
                let payload_coords = decoder.payload_decoder();
                // Then extract slices (&self calls) and convert to owned
                let msg = std::str::from_utf8(decoder.message_slice(msg_coords))
                    .unwrap_or("<invalid utf-8>")
                    .to_string();
                let payload = decoder.payload_slice(payload_coords).to_vec();
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
    use crate::codecs::cluster_codecs::{
        WriteBuf, challenge_codec::ChallengeEncoder, event_code::EventCode,
        new_leader_event_codec::NewLeaderEventEncoder, session_event_codec::SessionEventEncoder,
        session_message_header_codec::SessionMessageHeaderEncoder,
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
    fn test_dispatch_session_event_ok() {
        let mut data = vec![0u8; 128];
        {
            let wb = WriteBuf::new(&mut data);
            let mut enc = SessionEventEncoder::default().wrap(wb, 8);
            enc.cluster_session_id(7);
            enc.correlation_id(99);
            enc.leadership_term_id(3);
            enc.leader_member_id(0);
            enc.code(EventCode::OK);
            enc.version(1);
            enc.detail(b"ok");
            let _h = enc.header(0);
            // Encoder chain dropped here; data is written.
        }

        let mut adapter = EgressAdapter::new(Rec::default());
        // Pass the full buffer; decoders bound themselves via SBE_BLOCK_LENGTH
        assert!(adapter.on_fragment(&data).expect("decode failure"));
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().session_code, Some(EventCode::OK));
        assert_eq!(adapter.listener().detail, "ok");
    }

    #[test]
    fn test_dispatch_challenge() {
        let mut data = vec![0u8; 128];
        {
            let wb = WriteBuf::new(&mut data);
            let mut enc = ChallengeEncoder::default().wrap(wb, 8);
            enc.correlation_id(5);
            enc.cluster_session_id(2);
            enc.encoded_challenge(b"chal-token");
            let _h = enc.header(0);
        }

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&data).expect("decode failure"));
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().challenge, b"chal-token");
    }

    #[test]
    fn test_dispatch_new_leader() {
        let mut data = vec![0u8; 256];
        {
            let wb = WriteBuf::new(&mut data);
            let mut enc = NewLeaderEventEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(10);
            enc.cluster_session_id(99);
            enc.leader_member_id(1);
            enc.ingress_endpoints(b"0=host:9000,1=host:9001");
            let _h = enc.header(0);
        }

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&data).expect("decode failure"));
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().leader_endpoints, "0=host:9000,1=host:9001");
    }

    #[test]
    fn test_dispatch_session_message_header() {
        let mut data = vec![0u8; 128];
        {
            let wb = WriteBuf::new(&mut data);
            let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(1);
            enc.cluster_session_id(42);
            enc.timestamp(999);
            let _h = enc.header(0);
        }

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(adapter.on_fragment(&data).expect("decode failure"));
        assert_eq!(adapter.listener().calls, 1);
        assert_eq!(adapter.listener().msg_csid, 42);
        assert_eq!(adapter.listener().msg_ts, 999);
    }

    #[test]
    fn test_unknown_template_id_returns_false() {
        // Encode a valid SessionMessageHeader, then corrupt template_id
        let mut data = vec![0u8; 128];
        {
            let wb = WriteBuf::new(&mut data);
            let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(1);
            enc.cluster_session_id(1);
            enc.timestamp(1);
            let _h = enc.header(0);
        }
        // Overwrite template_id at bytes 2-3 with 0 (unknown)
        data[2] = 0u8;
        data[3] = 0u8;

        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(!adapter.on_fragment(&data).expect("decode failure"));
        assert_eq!(adapter.listener().calls, 0);
    }

    #[test]
    fn test_short_fragment_returns_false() {
        let mut adapter = EgressAdapter::new(Rec::default());
        assert!(!adapter.on_fragment(&[0u8; 4]).expect("decode failure"));
        assert_eq!(adapter.listener().calls, 0);
    }
}
