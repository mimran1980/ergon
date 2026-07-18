use ergo_aeron_cluster::codecs::cluster_codecs::{
    ReadBuf, WriteBuf, challenge_codec::ChallengeEncoder, event_code::EventCode,
    message_header_codec::MessageHeaderDecoder, new_leader_event_codec::NewLeaderEventEncoder,
    session_event_codec::SessionEventEncoder, session_keep_alive_codec::SessionKeepAliveEncoder,
    session_message_header_codec::SessionMessageHeaderEncoder,
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
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
        enc.leadership_term_id(ltid);
        enc.cluster_session_id(csid);
        enc.timestamp(ts);
        let _h = enc.header(0);

        let rb = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
        let dec = ergo_aeron_cluster::codecs::cluster_codecs::session_message_header_codec::SessionMessageHeaderDecoder::default().header(hdr, 0);
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
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionKeepAliveEncoder::default().wrap(wb, 8);
        enc.leadership_term_id(ltid);
        enc.cluster_session_id(csid);
        let _h = enc.header(0);

        let rb = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
        prop_assert_eq!(hdr.template_id(), 5); // KEEP_ALIVE
    }

    #[test]
    fn prop_challenge_roundtrip(
        cid in any::<i64>(),
        csid in any::<i64>(),
        data in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let mut buf = vec![0u8; 256];
        let wb = WriteBuf::new(&mut buf);
        let mut enc = ChallengeEncoder::default().wrap(wb, 8);
        enc.correlation_id(cid);
        enc.cluster_session_id(csid);
        enc.encoded_challenge(&data);
        let _h = enc.header(0);

        let rb = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
        let mut dec = ergo_aeron_cluster::codecs::cluster_codecs::challenge_codec::ChallengeDecoder::default().header(hdr, 0);
        prop_assert_eq!(dec.correlation_id(), cid);
        prop_assert_eq!(dec.cluster_session_id(), csid);
        let coords = dec.encoded_challenge_decoder();
        prop_assert_eq!(dec.encoded_challenge_slice(coords), &data[..]);
    }

    #[test]
    fn prop_new_leader_roundtrip(
        ltid in any::<i64>(),
        csid in any::<i64>(),
        mid in any::<i32>(),
        eps in "[a-z0-9=,:]{0,80}",
    ) {
        let mut buf = vec![0u8; 256];
        let wb = WriteBuf::new(&mut buf);
        let mut enc = NewLeaderEventEncoder::default().wrap(wb, 8);
        enc.leadership_term_id(ltid);
        enc.cluster_session_id(csid);
        enc.leader_member_id(mid);
        enc.ingress_endpoints(eps.as_bytes());
        let _h = enc.header(0);

        let rb = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
        let mut dec = ergo_aeron_cluster::codecs::cluster_codecs::new_leader_event_codec::NewLeaderEventDecoder::default().header(hdr, 0);
        prop_assert_eq!(dec.leadership_term_id(), ltid);
        prop_assert_eq!(dec.cluster_session_id(), csid);
        prop_assert_eq!(dec.leader_member_id(), mid);
        let coords = dec.ingress_endpoints_decoder();
        prop_assert_eq!(dec.ingress_endpoints_slice(coords), eps.as_bytes());
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
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionEventEncoder::default().wrap(wb, 8);
        enc.cluster_session_id(csid);
        enc.correlation_id(cid);
        enc.leadership_term_id(ltid);
        enc.leader_member_id(mid);
        enc.code(EventCode::OK);
        enc.version(1);
        enc.detail(detail.as_bytes());
        let _h = enc.header(0);

        let rb = ReadBuf::new(&buf);
        let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
        let dec = ergo_aeron_cluster::codecs::cluster_codecs::session_event_codec::SessionEventDecoder::default().header(hdr, 0);
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

    #[test]
    fn prop_codec_no_panic_on_arbitrary(encoded in prop::collection::vec(any::<u8>(), 0..128)) {
        // Decoding arbitrary bytes should never panic
        if encoded.len() >= 8 {
            let rb = ReadBuf::new(&encoded);
            let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
            let _id = hdr.template_id(); // safe
        }
    }
}
