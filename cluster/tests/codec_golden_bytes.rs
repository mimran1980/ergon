//! Wire-byte golden fixtures for the cluster protocol messages.
//!
//! Each `GOLDEN_*` constant is the exact SBE wire output for a fixed,
//! deterministic input, captured from the sbe-tool 1.39.0 codecs that
//! `cluster/src/codecs/cluster_codecs/` currently ship. The `parity_*` tests
//! re-encode each message with the current codec stack and assert byte-for-byte
//! equality with the constant.
//!
//! These constants are the **migration safety net**: when the cluster codecs
//! move to ErgoSBE generation (`docs/superpowers/plans/2026-07-18-ergosbe-
//! experimental-master-plan.md` §3), the `parity_*` encode bodies are rewritten
//! to the ErgoSBE API but the constants stay. If every `parity_*` test still
//! passes post-migration, wire parity with the Java reference is preserved.
//!
//! Capture provenance: 2026-07-18, aeron submodule 1.52.2, schema id 111 v16.

#![allow(unused_must_use)]

use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf, admin_request_type::AdminRequestType, admin_response_code::AdminResponseCode,
    admin_response_codec::AdminResponseEncoder, challenge_codec::ChallengeEncoder,
    challenge_response_codec::ChallengeResponseEncoder, event_code::EventCode,
    new_leader_event_codec::NewLeaderEventEncoder, session_close_request_codec::SessionCloseRequestEncoder,
    session_connect_request_codec::SessionConnectRequestEncoder, session_event_codec::SessionEventEncoder,
    session_keep_alive_codec::SessionKeepAliveEncoder, session_message_header_codec::SessionMessageHeaderEncoder,
};

const GOLDEN_SESSION_MESSAGE_HEADER: [u8; 32] = [
    0x18, 0x00, 0x01, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0xd2, 0x02, 0x96, 0x49, 0x00, 0x00, 0x00, 0x00,
];
const GOLDEN_SESSION_KEEP_ALIVE: [u8; 24] = [
    0x10, 0x00, 0x05, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
];
const GOLDEN_SESSION_CLOSE_REQUEST: [u8; 24] = [
    0x10, 0x00, 0x04, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2a, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
];
const GOLDEN_SESSION_EVENT: [u8; 67] = [
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x73,
    0x6f, 0x6d, 0x65, 0x2d, 0x64, 0x65, 0x74, 0x61, 0x69, 0x6c,
];
const GOLDEN_SESSION_CONNECT_REQUEST: [u8; 78] = [
    0x10, 0x00, 0x03, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x00, 0x00,
    0x00, 0x01, 0x00, 0x00, 0x00, 0x21, 0x00, 0x00, 0x00, 0x61, 0x65, 0x72, 0x6f, 0x6e, 0x3a, 0x75, 0x64, 0x70, 0x3f,
    0x65, 0x6e, 0x64, 0x70, 0x6f, 0x69, 0x6e, 0x74, 0x3d, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x68, 0x6f, 0x73, 0x74, 0x3a,
    0x39, 0x39, 0x39, 0x39, 0x09, 0x00, 0x00, 0x00, 0x75, 0x73, 0x65, 0x72, 0x3a, 0x70, 0x61, 0x73, 0x73, 0x00, 0x00,
    0x00, 0x00,
];
const GOLDEN_CHALLENGE: [u8; 49] = [
    0x10, 0x00, 0x07, 0x00, 0x6f, 0x00, 0x10, 0x00, 0xc8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x63, 0x68, 0x61, 0x6c, 0x6c, 0x65, 0x6e, 0x67, 0x65, 0x2d,
    0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x2d, 0x31, 0x32, 0x33, 0x34, 0x35,
];
const GOLDEN_CHALLENGE_RESPONSE: [u8; 42] = [
    0x10, 0x00, 0x08, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x2c, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x72, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x2d, 0x63,
    0x72, 0x65, 0x64, 0x73,
];
const GOLDEN_NEW_LEADER_EVENT: [u8; 82] = [
    0x14, 0x00, 0x06, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00, 0x30, 0x3d, 0x6c, 0x6f, 0x63, 0x61,
    0x6c, 0x68, 0x6f, 0x73, 0x74, 0x3a, 0x39, 0x30, 0x31, 0x30, 0x2c, 0x31, 0x3d, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x68,
    0x6f, 0x73, 0x74, 0x3a, 0x39, 0x30, 0x31, 0x31, 0x2c, 0x32, 0x3d, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x68, 0x6f, 0x73,
    0x74, 0x3a, 0x39, 0x30, 0x31, 0x32,
];
const GOLDEN_ADMIN_RESPONSE: [u8; 42] = [
    0x18, 0x00, 0x1b, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x6f, 0x6b,
    0x00, 0x00, 0x00, 0x00,
];

#[test]
fn parity_session_message_header() {
    let mut b = vec![0u8; 64];
    let wb = WriteBuf::new(&mut b);
    let mut e = SessionMessageHeaderEncoder::default().wrap(wb, 8);
    e.leadership_term_id(42);
    e.cluster_session_id(99);
    e.timestamp(1234567890);
    let _ = e.header(0);
    assert_eq!(&b[..32], &GOLDEN_SESSION_MESSAGE_HEADER[..]);
}

#[test]
fn parity_session_keep_alive() {
    let mut b = vec![0u8; 64];
    let wb = WriteBuf::new(&mut b);
    let mut e = SessionKeepAliveEncoder::default().wrap(wb, 8);
    e.leadership_term_id(5);
    e.cluster_session_id(10);
    let _ = e.header(0);
    assert_eq!(&b[..24], &GOLDEN_SESSION_KEEP_ALIVE[..]);
}

#[test]
fn parity_session_close_request() {
    let mut b = vec![0u8; 64];
    let wb = WriteBuf::new(&mut b);
    let mut e = SessionCloseRequestEncoder::default().wrap(wb, 8);
    e.leadership_term_id(7);
    e.cluster_session_id(42);
    let _ = e.header(0);
    assert_eq!(&b[..24], &GOLDEN_SESSION_CLOSE_REQUEST[..]);
}

#[test]
fn parity_session_event() {
    let detail: &[u8] = b"some-detail";
    let mut b = vec![0u8; 128];
    let wb = WriteBuf::new(&mut b);
    let mut e = SessionEventEncoder::default().wrap(wb, 8);
    e.cluster_session_id(1);
    e.correlation_id(100);
    e.leadership_term_id(5);
    e.leader_member_id(0);
    e.code(EventCode::OK);
    e.version(1);
    e.detail(detail);
    let _ = e.header(0);
    assert_eq!(&b[..67], &GOLDEN_SESSION_EVENT[..]);
}

#[test]
fn parity_session_connect_request() {
    let channel = "aeron:udp?endpoint=localhost:9999";
    let creds = b"user:pass";
    let mut b = vec![0u8; 256];
    let wb = WriteBuf::new(&mut b);
    let mut e = SessionConnectRequestEncoder::default().wrap(wb, 8);
    e.correlation_id(42);
    e.response_stream_id(102);
    e.version(1);
    e.response_channel(channel.as_bytes());
    e.encoded_credentials(creds);
    e.client_info(b"");
    let _ = e.header(0);
    assert_eq!(&b[..78], &GOLDEN_SESSION_CONNECT_REQUEST[..]);
}

#[test]
fn parity_challenge() {
    let tok = b"challenge-token-12345";
    let mut b = vec![0u8; 128];
    let wb = WriteBuf::new(&mut b);
    let mut e = ChallengeEncoder::default().wrap(wb, 8);
    e.correlation_id(200);
    e.cluster_session_id(5);
    e.encoded_challenge(tok);
    let _ = e.header(0);
    assert_eq!(&b[..49], &GOLDEN_CHALLENGE[..]);
}

#[test]
fn parity_challenge_response() {
    let rcreds = b"response-creds";
    let mut b = vec![0u8; 128];
    let wb = WriteBuf::new(&mut b);
    let mut e = ChallengeResponseEncoder::default().wrap(wb, 8);
    e.correlation_id(300);
    e.cluster_session_id(8);
    e.encoded_credentials(rcreds);
    let _ = e.header(0);
    assert_eq!(&b[..42], &GOLDEN_CHALLENGE_RESPONSE[..]);
}

#[test]
fn parity_new_leader_event() {
    let endpoints = "0=localhost:9010,1=localhost:9011,2=localhost:9012";
    let mut b = vec![0u8; 256];
    let wb = WriteBuf::new(&mut b);
    let mut e = NewLeaderEventEncoder::default().wrap(wb, 8);
    e.leadership_term_id(10);
    e.cluster_session_id(99);
    e.leader_member_id(1);
    e.ingress_endpoints(endpoints.as_bytes());
    let _ = e.header(0);
    assert_eq!(&b[..82], &GOLDEN_NEW_LEADER_EVENT[..]);
}

#[test]
fn parity_admin_response() {
    let msg = b"ok";
    let payload: &[u8] = b"";
    let mut b = vec![0u8; 128];
    let wb = WriteBuf::new(&mut b);
    let mut e = AdminResponseEncoder::default().wrap(wb, 8);
    e.cluster_session_id(1);
    e.correlation_id(2);
    e.request_type(AdminRequestType::SNAPSHOT);
    e.response_code(AdminResponseCode::OK);
    e.message(msg);
    e.payload(payload);
    let _ = e.header(0);
    assert_eq!(&b[..42], &GOLDEN_ADMIN_RESPONSE[..]);
}

// ── ErgoSBE parity (build.rs-generated `ergo_codecs`) ─────────────────────
//
// Same golden constants, ErgoSBE-generated encoders. If these pass alongside
// the sbe-tool `parity_*` tests above, ErgoSBE output is wire-identical to the
// Java reference for these messages.

use ergo_aeron_cluster::codecs::ergo_codecs::{
    SessionCloseRequestEncoder as EsmSessionCloseRequestEncoder, SessionKeepAliveEncoder as EsmSessionKeepAliveEncoder,
    SessionMessageHeaderEncoder as EsmSessionMessageHeaderEncoder,
};

#[test]
fn parity_ergo_session_message_header() {
    let mut b = vec![0u8; 64];
    let mut e = EsmSessionMessageHeaderEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.leadership_term_id(42);
    e.cluster_session_id(99);
    e.timestamp(1234567890);
    assert_eq!(e.as_ref(), &GOLDEN_SESSION_MESSAGE_HEADER[..]);
}

#[test]
fn parity_ergo_session_keep_alive() {
    let mut b = vec![0u8; 64];
    let mut e = EsmSessionKeepAliveEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.leadership_term_id(5);
    e.cluster_session_id(10);
    assert_eq!(e.as_ref(), &GOLDEN_SESSION_KEEP_ALIVE[..]);
}

#[test]
fn parity_ergo_session_close_request() {
    let mut b = vec![0u8; 64];
    let mut e = EsmSessionCloseRequestEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.leadership_term_id(7);
    e.cluster_session_id(42);
    assert_eq!(e.as_ref(), &GOLDEN_SESSION_CLOSE_REQUEST[..]);
}

use ergo_aeron_cluster::codecs::ergo_codecs::{
    AdminRequestType as ErgoAdminRequestType, AdminResponseCode as ErgoAdminResponseCode,
    AdminResponseEncoder as ErgoAdminResponseEncoder, ChallengeEncoder as ErgoChallengeEncoder,
    ChallengeResponseEncoder as ErgoChallengeResponseEncoder, EventCode as ErgoEventCode,
    NewLeaderEventEncoder as ErgoNewLeaderEventEncoder,
    SessionConnectRequestEncoder as ErgoSessionConnectRequestEncoder, SessionEventEncoder as ErgoSessionEventEncoder,
};

#[test]
fn parity_ergo_session_event() {
    let detail: &[u8] = b"some-detail";
    let mut b = vec![0u8; 128];
    let mut e = ErgoSessionEventEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.cluster_session_id(1);
    e.correlation_id(100);
    e.leadership_term_id(5);
    e.leader_member_id(0);
    e.code(ErgoEventCode::OK);
    e.version(1);
    let complete = e.detail(detail).unwrap();
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_SESSION_EVENT[..]);
}

#[test]
fn parity_ergo_session_connect_request() {
    let channel = "aeron:udp?endpoint=localhost:9999";
    let creds = b"user:pass";
    let mut b = vec![0u8; 256];
    let mut e = ErgoSessionConnectRequestEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.correlation_id(42);
    e.response_stream_id(102);
    e.version(1);
    let after_ch = e.response_channel(channel.as_bytes()).unwrap();
    let after_cred = after_ch.encoded_credentials(creds).unwrap();
    let complete = after_cred.client_info(b"").unwrap();
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_SESSION_CONNECT_REQUEST[..]);
}

#[test]
fn parity_ergo_challenge() {
    let tok = b"challenge-token-12345";
    let mut b = vec![0u8; 128];
    let mut e = ErgoChallengeEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.correlation_id(200);
    e.cluster_session_id(5);
    let complete = e.encoded_challenge(tok).unwrap();
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_CHALLENGE[..]);
}

#[test]
fn parity_ergo_challenge_response() {
    let rcreds = b"response-creds";
    let mut b = vec![0u8; 128];
    let mut e = ErgoChallengeResponseEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.correlation_id(300);
    e.cluster_session_id(8);
    let complete = e.encoded_credentials(rcreds).unwrap();
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_CHALLENGE_RESPONSE[..]);
}

#[test]
fn parity_ergo_new_leader_event() {
    let endpoints = "0=localhost:9010,1=localhost:9011,2=localhost:9012";
    let mut b = vec![0u8; 256];
    let mut e = ErgoNewLeaderEventEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.leadership_term_id(10);
    e.cluster_session_id(99);
    e.leader_member_id(1);
    let complete = e.ingress_endpoints(endpoints.as_bytes()).unwrap();
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_NEW_LEADER_EVENT[..]);
}

#[test]
fn parity_ergo_admin_response() {
    let msg = b"ok";
    let payload: &[u8] = b"";
    let mut b = vec![0u8; 128];
    let mut e = ErgoAdminResponseEncoder::wrap_and_apply_header(&mut b, 0).unwrap();
    e.cluster_session_id(1);
    e.correlation_id(2);
    e.request_type(ErgoAdminRequestType::SNAPSHOT);
    e.response_code(ErgoAdminResponseCode::OK);
    let after_msg = e.message(msg).unwrap();
    let complete = after_msg.payload(payload).unwrap();
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_ADMIN_RESPONSE[..]);
}
