//! Protocol-level tests: compatibility, malformed input, edge cases.

#[cfg(test)]
mod tests {
    use crate::codecs::cluster_codecs::{
        ReadBuf, WriteBuf,
        event_code::EventCode,
        message_header_codec::{ENCODED_LENGTH as HEADER_LEN, MessageHeaderDecoder, MessageHeaderEncoder},
        session_keep_alive_codec::{SBE_TEMPLATE_ID as KEEP_ALIVE_ID, SessionKeepAliveEncoder},
        session_message_header_codec::{SBE_TEMPLATE_ID as SESSION_MSG_HDR_ID, SessionMessageHeaderEncoder},
    };

    #[test]
    fn test_template_ids_match_java_reference() {
        assert_eq!(SESSION_MSG_HDR_ID, 1, "SessionMessageHeader");
        assert_eq!(
            crate::codecs::cluster_codecs::session_event_codec::SBE_TEMPLATE_ID,
            2,
            "SessionEvent"
        );
        assert_eq!(
            crate::codecs::cluster_codecs::session_connect_request_codec::SBE_TEMPLATE_ID,
            3,
            "SessionConnectRequest"
        );
        assert_eq!(
            crate::codecs::cluster_codecs::session_close_request_codec::SBE_TEMPLATE_ID,
            4,
            "SessionCloseRequest"
        );
        assert_eq!(KEEP_ALIVE_ID, 5, "SessionKeepAlive");
        assert_eq!(
            crate::codecs::cluster_codecs::new_leader_event_codec::SBE_TEMPLATE_ID,
            6,
            "NewLeaderEvent"
        );
        assert_eq!(
            crate::codecs::cluster_codecs::challenge_codec::SBE_TEMPLATE_ID,
            7,
            "Challenge"
        );
        assert_eq!(
            crate::codecs::cluster_codecs::challenge_response_codec::SBE_TEMPLATE_ID,
            8,
            "ChallengeResponse"
        );
        assert_eq!(
            crate::codecs::cluster_codecs::admin_request_codec::SBE_TEMPLATE_ID,
            26,
            "AdminRequest"
        );
        assert_eq!(
            crate::codecs::cluster_codecs::admin_response_codec::SBE_TEMPLATE_ID,
            27,
            "AdminResponse"
        );
    }

    #[test]
    fn test_schema_id_is_111() {
        assert_eq!(crate::codecs::cluster_codecs::SBE_SCHEMA_ID, 111);
    }

    #[test]
    fn test_schema_version_is_16() {
        assert_eq!(crate::codecs::cluster_codecs::SBE_SCHEMA_VERSION, 16);
    }

    #[test]
    fn test_message_header_is_8_bytes() {
        assert_eq!(HEADER_LEN, 8);
    }

    #[test]
    fn test_session_message_header_is_24_bytes() {
        assert_eq!(
            crate::codecs::cluster_codecs::session_message_header_codec::SBE_BLOCK_LENGTH,
            24
        );
    }

    #[test]
    fn test_session_event_body_is_44_bytes() {
        assert_eq!(crate::codecs::cluster_codecs::session_event_codec::SBE_BLOCK_LENGTH, 44);
    }

    // ── malformed input ──

    #[test]
    fn test_decode_empty_buffer_returns_false_not_panic() {
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(!adapter.on_fragment(&[]).expect("should not error"));
    }

    #[test]
    fn test_decode_truncated_header_returns_false() {
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(!adapter.on_fragment(&[0u8; 4]).expect("should not error"));
    }

    #[test]
    fn test_decode_wrong_template_id_returns_false() {
        let mut buf = vec![0u8; 32];
        {
            let wb = WriteBuf::new(&mut buf);
            let mut h = MessageHeaderEncoder::default().wrap(wb, 0);
            h.block_length(0);
            h.template_id(0xFFFF);
            h.schema_id(111);
            h.version(16);
        }
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(!adapter.on_fragment(&buf).expect("should not error"));
    }

    #[test]
    fn test_decode_valid_template_id_routes() {
        let mut buf = vec![0u8; 64];
        {
            let wb = WriteBuf::new(&mut buf);
            let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(1);
            enc.cluster_session_id(1);
            enc.timestamp(1);
            let _h = enc.header(0);
        }
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(adapter.on_fragment(&buf).expect("should not error"));
    }

    // ── event codes match Java ──

    #[test]
    fn test_event_code_values_match_java() {
        assert_eq!(EventCode::OK as i32, 0);
        assert_eq!(EventCode::ERROR as i32, 1);
        assert_eq!(EventCode::REDIRECT as i32, 2);
        assert_eq!(EventCode::AUTHENTICATION_REJECTED as i32, 3);
        assert_eq!(EventCode::CLOSED as i32, 4);
    }

    // ── keep-alive roundtrip ──

    #[test]
    fn test_roundtrip_keep_alive() {
        let mut buf = vec![0u8; 64];
        {
            let wb = WriteBuf::new(&mut buf);
            let mut enc = SessionKeepAliveEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(5);
            enc.cluster_session_id(10);
            let _h = enc.header(0);
        }
        let read_buf = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(read_buf, 0);
        assert_eq!(hdr.template_id(), KEEP_ALIVE_ID);
    }

    // ── extreme values ──

    #[test]
    fn test_encode_decode_max_values() {
        let mut buf = vec![0u8; 64];
        {
            let wb = WriteBuf::new(&mut buf);
            let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(i64::MAX);
            enc.cluster_session_id(i64::MAX);
            enc.timestamp(i64::MAX);
            let _h = enc.header(0);
        }
        let read_buf = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(read_buf, 0);
        let dec = crate::codecs::cluster_codecs::session_message_header_codec::SessionMessageHeaderDecoder::default()
            .header(hdr, 0);
        assert_eq!(dec.leadership_term_id(), i64::MAX);
        assert_eq!(dec.cluster_session_id(), i64::MAX);
        assert_eq!(dec.timestamp(), i64::MAX);
    }

    #[test]
    fn test_encode_decode_min_values() {
        let mut buf = vec![0u8; 64];
        {
            let wb = WriteBuf::new(&mut buf);
            let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            enc.leadership_term_id(i64::MIN);
            enc.cluster_session_id(i64::MIN);
            enc.timestamp(i64::MIN);
            let _h = enc.header(0);
        }
        let read_buf = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(read_buf, 0);
        let dec = crate::codecs::cluster_codecs::session_message_header_codec::SessionMessageHeaderDecoder::default()
            .header(hdr, 0);
        assert_eq!(dec.leadership_term_id(), i64::MIN);
        assert_eq!(dec.cluster_session_id(), i64::MIN);
        assert_eq!(dec.timestamp(), i64::MIN);
    }
}
