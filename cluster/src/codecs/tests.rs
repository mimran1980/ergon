use super::cluster_codecs::*;
use super::cluster_codecs::{
    challenge_codec::{self, ChallengeDecoder, ChallengeEncoder},
    event_code::EventCode,
    message_header_codec::MessageHeaderDecoder,
    new_leader_event_codec::{self, NewLeaderEventDecoder, NewLeaderEventEncoder},
    session_connect_request_codec::{self, SessionConnectRequestDecoder, SessionConnectRequestEncoder},
    session_event_codec::{self, SessionEventDecoder, SessionEventEncoder},
    session_message_header_codec::{self, SessionMessageHeaderDecoder, SessionMessageHeaderEncoder},
};

#[test]
fn test_session_message_header_roundtrip() {
    let mut data = vec![0u8; 256];
    let write_buf = WriteBuf::new(&mut data);

    let mut encoder = SessionMessageHeaderEncoder::default().wrap(write_buf, 8);
    encoder.leadership_term_id(42);
    encoder.cluster_session_id(99);
    encoder.timestamp(1234567890);
    let _header = encoder.header(0);

    let read_buf = ReadBuf::new(&data);
    let header_dec = MessageHeaderDecoder::default().wrap(read_buf, 0);
    let decoder = SessionMessageHeaderDecoder::default().header(header_dec, 0);

    assert_eq!(decoder.leadership_term_id(), 42);
    assert_eq!(decoder.cluster_session_id(), 99);
    assert_eq!(decoder.timestamp(), 1234567890);
}

#[test]
fn test_session_event_ok_roundtrip() {
    let mut data = vec![0u8; 256];
    let write_buf = WriteBuf::new(&mut data);

    let mut encoder = SessionEventEncoder::default().wrap(write_buf, 8);
    encoder.cluster_session_id(1);
    encoder.correlation_id(100);
    encoder.leadership_term_id(5);
    encoder.leader_member_id(0);
    encoder.code(EventCode::OK);
    encoder.version(1);
    encoder.detail(&[]);
    let _header = encoder.header(0);

    let read_buf = ReadBuf::new(&data);
    let header_dec = MessageHeaderDecoder::default().wrap(read_buf, 0);
    let mut decoder = SessionEventDecoder::default().header(header_dec, 0);

    assert_eq!(decoder.cluster_session_id(), 1);
    assert_eq!(decoder.correlation_id(), 100);
    assert_eq!(decoder.leadership_term_id(), 5);
    assert_eq!(decoder.leader_member_id(), 0);
    assert_eq!(decoder.code(), EventCode::OK);
    assert_eq!(decoder.version(), Some(1));
    let coords = decoder.detail_decoder();
    assert_eq!(decoder.detail_slice(coords).len(), 0);
}

#[test]
fn test_session_connect_request_roundtrip() {
    let mut data = vec![0u8; 512];
    let write_buf = WriteBuf::new(&mut data);

    let response_channel = "aeron:udp?endpoint=localhost:9999";
    let credentials = b"user:pass";

    let mut encoder = SessionConnectRequestEncoder::default().wrap(write_buf, 8);
    encoder.correlation_id(42);
    encoder.response_stream_id(102);
    encoder.version(1);
    encoder.response_channel(response_channel.as_bytes());
    encoder.encoded_credentials(credentials);
    let _header = encoder.header(0);

    let read_buf = ReadBuf::new(&data);
    let header_dec = MessageHeaderDecoder::default().wrap(read_buf, 0);
    let mut decoder = SessionConnectRequestDecoder::default().header(header_dec, 0);

    assert_eq!(decoder.correlation_id(), 42);
    assert_eq!(decoder.response_stream_id(), 102);
    assert_eq!(decoder.version(), Some(1));

    let ch_coords = decoder.response_channel_decoder();
    assert_eq!(decoder.response_channel_slice(ch_coords), response_channel.as_bytes());
    let cred_coords = decoder.encoded_credentials_decoder();
    assert_eq!(decoder.encoded_credentials_slice(cred_coords), credentials);
}

#[test]
fn test_challenge_roundtrip() {
    let mut data = vec![0u8; 256];
    let write_buf = WriteBuf::new(&mut data);

    let challenge_data = b"challenge-token-12345";

    let mut encoder = ChallengeEncoder::default().wrap(write_buf, 8);
    encoder.correlation_id(200);
    encoder.cluster_session_id(5);
    encoder.encoded_challenge(challenge_data);
    let _header = encoder.header(0);

    let read_buf = ReadBuf::new(&data);
    let header_dec = MessageHeaderDecoder::default().wrap(read_buf, 0);
    let mut decoder = ChallengeDecoder::default().header(header_dec, 0);

    assert_eq!(decoder.correlation_id(), 200);
    assert_eq!(decoder.cluster_session_id(), 5);
    let coords = decoder.encoded_challenge_decoder();
    assert_eq!(decoder.encoded_challenge_slice(coords), challenge_data);
}

#[test]
fn test_new_leader_event_roundtrip() {
    let mut data = vec![0u8; 512];
    let write_buf = WriteBuf::new(&mut data);

    let endpoints = "0=localhost:9010,1=localhost:9011,2=localhost:9012";

    let mut encoder = NewLeaderEventEncoder::default().wrap(write_buf, 8);
    encoder.leadership_term_id(10);
    encoder.cluster_session_id(99);
    encoder.leader_member_id(1);
    encoder.ingress_endpoints(endpoints.as_bytes());
    let _header = encoder.header(0);

    let read_buf = ReadBuf::new(&data);
    let header_dec = MessageHeaderDecoder::default().wrap(read_buf, 0);
    let mut decoder = NewLeaderEventDecoder::default().header(header_dec, 0);

    assert_eq!(decoder.leadership_term_id(), 10);
    assert_eq!(decoder.cluster_session_id(), 99);
    assert_eq!(decoder.leader_member_id(), 1);
    let coords = decoder.ingress_endpoints_decoder();
    assert_eq!(decoder.ingress_endpoints_slice(coords), endpoints.as_bytes());
}

#[test]
fn test_schema_constants() {
    assert_eq!(SBE_SCHEMA_ID, 111);
    assert_eq!(SBE_SCHEMA_VERSION, 16);

    assert_eq!(session_message_header_codec::SBE_TEMPLATE_ID, 1);
    assert_eq!(session_event_codec::SBE_TEMPLATE_ID, 2);
    assert_eq!(session_connect_request_codec::SBE_TEMPLATE_ID, 3);
    assert_eq!(session_close_request_codec::SBE_TEMPLATE_ID, 4);
    assert_eq!(session_keep_alive_codec::SBE_TEMPLATE_ID, 5);
    assert_eq!(challenge_codec::SBE_TEMPLATE_ID, 7);
    assert_eq!(challenge_response_codec::SBE_TEMPLATE_ID, 8);
    assert_eq!(new_leader_event_codec::SBE_TEMPLATE_ID, 6);
}
