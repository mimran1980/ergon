//! Protocol-level tests: compatibility, malformed input, edge cases.
//! Production constants live on ErgoSBE encoder associated consts.

#[cfg(test)]
mod tests {
    use crate::codecs::ergo_codecs::EventCode;
    use crate::codecs::ergo_codecs::{
        AdminRequestEncoder, AdminResponseEncoder, ChallengeEncoder, ChallengeResponseEncoder, NewLeaderEventEncoder,
        SessionCloseRequestEncoder, SessionConnectRequestEncoder, SessionEventEncoder, SessionKeepAliveEncoder,
        SessionMessageHeaderDecoder, SessionMessageHeaderEncoder,
    };

    #[test]
    fn test_template_ids_match_java_reference() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SessionMessageHeaderEncoder::TEMPLATE_ID, 1, "SessionMessageHeader");
        assert_eq!(SessionEventEncoder::TEMPLATE_ID, 2, "SessionEvent");
        assert_eq!(SessionConnectRequestEncoder::TEMPLATE_ID, 3, "SessionConnectRequest");
        assert_eq!(SessionCloseRequestEncoder::TEMPLATE_ID, 4, "SessionCloseRequest");
        assert_eq!(SessionKeepAliveEncoder::TEMPLATE_ID, 5, "SessionKeepAlive");
        assert_eq!(NewLeaderEventEncoder::TEMPLATE_ID, 6, "NewLeaderEvent");
        assert_eq!(ChallengeEncoder::TEMPLATE_ID, 7, "Challenge");
        assert_eq!(ChallengeResponseEncoder::TEMPLATE_ID, 8, "ChallengeResponse");
        assert_eq!(AdminRequestEncoder::TEMPLATE_ID, 26, "AdminRequest");
        assert_eq!(AdminResponseEncoder::TEMPLATE_ID, 27, "AdminResponse");
    
        Ok(())
    }

    #[test]
    fn test_schema_id_is_111() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SessionConnectRequestEncoder::SCHEMA_ID, 111);
    
        Ok(())
    }

    #[test]
    fn test_schema_version_is_16() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SessionConnectRequestEncoder::SCHEMA_VERSION, 16);
    
        Ok(())
    }

    #[test]
    fn test_message_header_is_8_bytes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SessionMessageHeaderEncoder::HEADER_TEMPLATE.len(), 8);
    
        Ok(())
    }

    #[test]
    fn test_session_message_header_is_24_bytes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SessionMessageHeaderEncoder::BLOCK_LENGTH, 24);
    
        Ok(())
    }

    #[test]
    fn test_session_event_body_is_44_bytes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(SessionEventEncoder::BLOCK_LENGTH, 44);
    
        Ok(())
    }

    #[test]
    fn test_decode_empty_buffer_returns_false_not_panic() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(!adapter.on_fragment(&[]).expect("should not error"));
    
        Ok(())
    }

    #[test]
    fn test_decode_truncated_header_returns_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(!adapter.on_fragment(&[0u8; 4]).expect("should not error"));
    
        Ok(())
    }

    #[test]
    fn test_decode_wrong_template_id_returns_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 32];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc.leadership_term_id(1).cluster_session_id(1).timestamp(1);
        buf[2] = 0xFF;
        buf[3] = 0xFF;
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(!adapter.on_fragment(&buf).expect("should not error"));
        Ok(())
    }

    #[test]
    fn test_decode_valid_template_id_routes() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 64];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc.leadership_term_id(1).cluster_session_id(1).timestamp(1);
        let bytes = enc.as_ref().to_vec();
        let mut adapter = crate::egress::EgressAdapter::new(crate::egress::NullListener);
        assert!(adapter.on_fragment(&bytes).expect("should not error"));
        Ok(())
    }

    #[test]
    fn test_event_code_values_match_java() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(EventCode::OK as i32, 0);
        assert_eq!(EventCode::ERROR as i32, 1);
        assert_eq!(EventCode::REDIRECT as i32, 2);
        assert_eq!(EventCode::AUTHENTICATIONREJECTED as i32, 3);
        assert_eq!(EventCode::CLOSED as i32, 4);
    
        Ok(())
    }

    #[test]
    fn test_roundtrip_keep_alive() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 64];
        let mut enc = SessionKeepAliveEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc.leadership_term_id(5).cluster_session_id(10);
        let bytes = enc.as_ref();
        assert_eq!(
            u16::from_le_bytes([bytes[2], bytes[3]]),
            SessionKeepAliveEncoder::TEMPLATE_ID
        );
        Ok(())
    }

    #[test]
    fn test_encode_decode_max_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 64];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc
            .leadership_term_id(i64::MAX)
            .cluster_session_id(i64::MAX)
            .timestamp(i64::MAX);
        let bytes = enc.as_ref().to_vec();
        let dec = SessionMessageHeaderDecoder::wrap_and_apply_header(&bytes, 0)?;
        assert_eq!(dec.leadership_term_id(), i64::MAX);
        assert_eq!(dec.cluster_session_id(), i64::MAX);
        assert_eq!(dec.timestamp(), i64::MAX);
        Ok(())
    }

    #[test]
    fn test_encode_decode_min_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0u8; 64];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)?;
        let _ = enc
            .leadership_term_id(i64::MIN)
            .cluster_session_id(i64::MIN)
            .timestamp(i64::MIN);
        let bytes = enc.as_ref().to_vec();
        let dec = SessionMessageHeaderDecoder::wrap_and_apply_header(&bytes, 0)?;
        assert_eq!(dec.leadership_term_id(), i64::MIN);
        assert_eq!(dec.cluster_session_id(), i64::MIN);
        assert_eq!(dec.timestamp(), i64::MIN);
        Ok(())
    }
}
