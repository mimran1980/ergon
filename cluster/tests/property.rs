use ergo_aeron_cluster::cluster_codec_types::{
    ChallengeDecoder, ChallengeEncoder, EventCode, NewLeaderEventDecoder, NewLeaderEventEncoder, SessionEventDecoder,
    SessionEventEncoder, SessionKeepAliveEncoder, SessionMessageHeaderDecoder, SessionMessageHeaderEncoder,
};

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_session_message_header_roundtrip(
        ltid in any::<i64>(),
        csid in any::<i64>(),
        ts in any::<i64>(),
    ) {
        let mut buf = vec![0u8; 128];
        let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.leadership_term_id(ltid).cluster_session_id(csid).timestamp(ts);
        let bytes = enc.as_ref().to_vec();
        let dec = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&bytes, 0).unwrap();
        prop_assert_eq!(dec.leadership_term_id(), ltid);
        prop_assert_eq!(dec.cluster_session_id(), csid);
        prop_assert_eq!(dec.timestamp(), ts);
    }

    #[test]
    fn prop_session_keep_alive_roundtrip(
        ltid in any::<i64>(),
        csid in any::<i64>(),
    ) {
        let mut buf = vec![0u8; 128];
        let mut enc = SessionKeepAliveEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.leadership_term_id(ltid).cluster_session_id(csid);
        let bytes = enc.as_ref();
        prop_assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 5); // KEEP_ALIVE
    }

    #[test]
    fn prop_challenge_roundtrip(
        cid in any::<i64>(),
        csid in any::<i64>(),
        data in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let mut buf = vec![0u8; 256];
        let mut enc = ChallengeEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.correlation_id(cid).cluster_session_id(csid);
        let complete = enc.encoded_challenge(&data).unwrap();
        let bytes = complete.as_bytes_with_header();

        // Decode: skip header bytes (8), decode the body
        let dec = ergo_aeron_cluster::cluster_codec_types::ChallengeDecoder::try_wrap_and_apply_header(bytes, 0).unwrap();
        prop_assert_eq!(dec.correlation_id(), cid);
        prop_assert_eq!(dec.cluster_session_id(), csid);
        let (chal, _) = dec.into_encoded_challenge().unwrap();
        prop_assert_eq!(chal, &data[..]);
    }

    #[test]
    fn prop_new_leader_roundtrip(
        ltid in any::<i64>(),
        csid in any::<i64>(),
        mid in any::<i32>(),
        eps in "[a-z0-9=,:]{0,80}",
    ) {
        let mut buf = vec![0u8; 256];
        let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.leadership_term_id(ltid).cluster_session_id(csid).leader_member_id(mid);
        let complete = enc.ingress_endpoints(eps.as_bytes()).unwrap();
        let bytes = complete.as_bytes_with_header();

        let dec = ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder::try_wrap_and_apply_header(bytes, 0).unwrap();
        prop_assert_eq!(dec.leadership_term_id(), ltid);
        prop_assert_eq!(dec.cluster_session_id(), csid);
        prop_assert_eq!(dec.leader_member_id(), mid);
        let (eps_out, _) = dec.into_ingress_endpoints().unwrap();
        prop_assert_eq!(eps_out, eps.as_bytes());
    }

    #[test]
    fn prop_session_event_roundtrip(
        csid in any::<i64>(),
        cid in any::<i64>(),
        ltid in any::<i64>(),
        mid in any::<i32>(),
        detail in "[a-zA-Z0-9 ]{0,40}",
    ) {
        let mut buf = vec![0u8; 256];
        let mut enc = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0);
        enc.cluster_session_id(csid).correlation_id(cid).leadership_term_id(ltid)
            .leader_member_id(mid).code(EventCode::OK).version(1);
        let complete = enc.detail(detail.as_bytes()).unwrap();
        let bytes = complete.as_bytes_with_header();

        let dec = ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::try_wrap_and_apply_header(bytes, 0).unwrap();
        prop_assert_eq!(dec.cluster_session_id(), csid);
        prop_assert_eq!(dec.correlation_id(), cid);
        prop_assert_eq!(dec.leadership_term_id(), ltid);
        prop_assert_eq!(dec.leader_member_id(), mid);
        prop_assert_eq!(dec.code(), EventCode::OK);
    }

    #[test]
    fn prop_arbitrary_bytes_dont_panic(data in prop::collection::vec(any::<u8>(), 0..256)) {
        // Feeding arbitrary bytes through the egress adapter must never panic
        use ergo_aeron_cluster::egress::{EgressAdapter, NullListener};
        let mut adapter = EgressAdapter::new(NullListener);
        let _ = adapter.on_fragment(&data); // must not panic
    }

    /// Feed random bytes to SessionMessageHeaderDecoder Display/Debug — no panic.
    #[test]
    fn fuzz_session_message_header_display_debug(data in prop::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 && let Ok(dec) = SessionMessageHeaderDecoder::try_wrap_and_apply_header(&data, 0) {
                let _ = format!("{dec}");
                let _ = format!("{dec:?}");
        }
    }

    /// Feed random bytes to SessionEventDecoder Display/Debug — no panic.
    #[test]
    fn fuzz_session_event_display_debug(data in prop::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 && let Ok(dec) = SessionEventDecoder::try_wrap_and_apply_header(&data, 0) {
                let _ = format!("{dec}");
                let _ = format!("{dec:?}");
        }
    }

    /// Feed random bytes to NewLeaderEventDecoder Display/Debug — no panic.
    #[test]
    fn fuzz_new_leader_display_debug(data in prop::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 && let Ok(dec) = NewLeaderEventDecoder::try_wrap_and_apply_header(&data, 0) {
                let _ = format!("{dec}");
                let _ = format!("{dec:?}");
        }
    }

    /// Feed random bytes to ChallengeDecoder Display/Debug — no panic.
    #[test]
    fn fuzz_challenge_display_debug(data in prop::collection::vec(any::<u8>(), 0..512)) {
        if data.len() >= 8 && let Ok(dec) = ChallengeDecoder::try_wrap_and_apply_header(&data, 0) {
                let _ = format!("{dec}");
                let _ = format!("{dec:?}");
        }
    }

    /// Feed random bytes to controlled egress adapter — no panic.
    #[test]
    fn fuzz_controlled_egress_no_panic(data in prop::collection::vec(any::<u8>(), 0..512)) {
        use ergo_aeron_cluster::controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
        struct NoOp;
        impl ControlledEgressListener for NoOp {
            fn on_message(&mut self, _: i64, _: i64, _: &[u8]) -> ControlledPollAction {
                ControlledPollAction::Continue
            }
        }
        let mut adapter = ControlledEgressAdapter::new(NoOp);
        let _ = adapter.on_fragment(&data);
    }

    #[test]
    fn prop_codec_no_panic_on_arbitrary(encoded in prop::collection::vec(any::<u8>(), 0..128)) {
        // Decoding arbitrary bytes should never panic — AnyMessage::decode is safe
        if encoded.len() >= 8 {
            let _ = ergo_aeron_cluster::cluster_codec_types::AnyMessage::decode(&encoded, 0);
        }
    }
}
