#![allow(missing_docs)]
//! Wire-byte golden fixtures for the cluster protocol messages.
//!
//! Each `GOLDEN_*` constant is the exact SBE wire output for a fixed,
//! deterministic input, originally captured from sbe-tool 1.39.0 and now
//! re-proven solely via **ergon** production encoders (`session`).
//! Protocol goldens no longer require sbe-tool at runtime.
//!
//! Capture provenance: 2026-07-18, aeron submodule 1.52.2, schema id 111 v16.

#![allow(unused_must_use)]

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
// leader_heartbeat_timeout_ns is optional; fixed(None) writes the schema null
// image (i64::MIN = 0x8000_0000_0000_0000 LE), not a zero-filled buffer.
const GOLDEN_SESSION_EVENT: [u8; 67] = [
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x0b, 0x00, 0x00, 0x00, 0x73,
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

// ── ergo-sbe production encoders (sole protocol path) ─────────────────────

use ergo_aeron_cluster::cluster_codec_types::{
    AdminRequestType as ErgoAdminRequestType, AdminResponseCode as ErgoAdminResponseCode,
    AdminResponseEncoder as ErgoAdminResponseEncoder, AdminResponseFixedFields as ErgoAdminResponseFixedFields,
    ChallengeEncoder as ErgoChallengeEncoder, ChallengeFixedFields as ErgoChallengeFixedFields,
    ChallengeResponseEncoder as ErgoChallengeResponseEncoder,
    ChallengeResponseFixedFields as ErgoChallengeResponseFixedFields, EventCode as ErgoEventCode,
    NewLeaderEventEncoder as ErgoNewLeaderEventEncoder, NewLeaderEventFixedFields as ErgoNewLeaderEventFixedFields,
    SessionCloseRequestEncoder as EsmSessionCloseRequestEncoder,
    SessionCloseRequestFixedFields as EsmSessionCloseRequestFixedFields,
    SessionConnectRequestEncoder as ErgoSessionConnectRequestEncoder,
    SessionConnectRequestFixedFields as ErgoSessionConnectRequestFixedFields,
    SessionEventEncoder as ErgoSessionEventEncoder, SessionEventFixedFields as ErgoSessionEventFixedFields,
    SessionKeepAliveEncoder as EsmSessionKeepAliveEncoder,
    SessionKeepAliveFixedFields as EsmSessionKeepAliveFixedFields,
    SessionMessageHeaderEncoder as EsmSessionMessageHeaderEncoder,
    SessionMessageHeaderFixedFields as EsmSessionMessageHeaderFixedFields,
};

#[test]
fn parity_ergo_session_message_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut b = [0u8; 64];
    let e =
        EsmSessionMessageHeaderEncoder::wrap_and_apply_header(&mut b, 0).fixed(&EsmSessionMessageHeaderFixedFields {
            leadership_term_id: 42,
            cluster_session_id: 99,
            timestamp: 1234567890,
        });
    assert_eq!(e.as_bytes_with_header(), &GOLDEN_SESSION_MESSAGE_HEADER[..]);

    Ok(())
}

#[test]
fn parity_ergo_session_keep_alive() -> Result<(), Box<dyn std::error::Error>> {
    let mut b = [0u8; 64];
    let e = EsmSessionKeepAliveEncoder::wrap_and_apply_header(&mut b, 0).fixed(&EsmSessionKeepAliveFixedFields {
        leadership_term_id: 5,
        cluster_session_id: 10,
    });
    assert_eq!(e.as_bytes_with_header(), &GOLDEN_SESSION_KEEP_ALIVE[..]);

    Ok(())
}

#[test]
fn parity_ergo_session_close_request() -> Result<(), Box<dyn std::error::Error>> {
    let mut b = [0u8; 64];
    let e = EsmSessionCloseRequestEncoder::wrap_and_apply_header(&mut b, 0).fixed(&EsmSessionCloseRequestFixedFields {
        leadership_term_id: 7,
        cluster_session_id: 42,
    });
    assert_eq!(e.as_bytes_with_header(), &GOLDEN_SESSION_CLOSE_REQUEST[..]);

    Ok(())
}

#[test]
fn parity_ergo_session_event() -> Result<(), Box<dyn std::error::Error>> {
    let detail: &[u8] = b"some-detail";
    let mut b = [0u8; 128];
    let complete = ErgoSessionEventEncoder::wrap_and_apply_header(&mut b, 0)
        .fixed(&ErgoSessionEventFixedFields {
            cluster_session_id: 1,
            correlation_id: 100,
            leadership_term_id: 5,
            leader_member_id: 0,
            code: ErgoEventCode::OK,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(detail)?;
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_SESSION_EVENT[..]);

    Ok(())
}

#[test]
fn parity_ergo_session_connect_request() -> Result<(), Box<dyn std::error::Error>> {
    let channel = "aeron:udp?endpoint=localhost:9999";
    let creds = b"user:pass";
    let mut b = [0u8; 256];
    let complete = ErgoSessionConnectRequestEncoder::wrap_and_apply_header(&mut b, 0)
        .fixed(&ErgoSessionConnectRequestFixedFields {
            correlation_id: 42,
            response_stream_id: 102,
            version: Some(1),
        })
        .response_channel(channel.as_bytes())?
        .encoded_credentials(creds)?
        .client_info(b"")?;
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_SESSION_CONNECT_REQUEST[..]);

    Ok(())
}

#[test]
fn parity_ergo_challenge() -> Result<(), Box<dyn std::error::Error>> {
    let tok = b"challenge-token-12345";
    let mut b = [0u8; 128];
    let complete = ErgoChallengeEncoder::wrap_and_apply_header(&mut b, 0)
        .fixed(&ErgoChallengeFixedFields {
            correlation_id: 200,
            cluster_session_id: 5,
        })
        .encoded_challenge(tok)?;
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_CHALLENGE[..]);

    Ok(())
}

#[test]
fn parity_ergo_challenge_response() -> Result<(), Box<dyn std::error::Error>> {
    let rcreds = b"response-creds";
    let mut b = [0u8; 128];
    let complete = ErgoChallengeResponseEncoder::wrap_and_apply_header(&mut b, 0)
        .fixed(&ErgoChallengeResponseFixedFields {
            correlation_id: 300,
            cluster_session_id: 8,
        })
        .encoded_credentials(rcreds)?;
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_CHALLENGE_RESPONSE[..]);

    Ok(())
}

#[test]
fn parity_ergo_new_leader_event() -> Result<(), Box<dyn std::error::Error>> {
    let endpoints = "0=localhost:9010,1=localhost:9011,2=localhost:9012";
    let mut b = [0u8; 256];
    let complete = ErgoNewLeaderEventEncoder::wrap_and_apply_header(&mut b, 0)
        .fixed(&ErgoNewLeaderEventFixedFields {
            leadership_term_id: 10,
            cluster_session_id: 99,
            leader_member_id: 1,
        })
        .ingress_endpoints(endpoints.as_bytes())?;
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_NEW_LEADER_EVENT[..]);

    Ok(())
}

#[test]
fn parity_ergo_admin_response() -> Result<(), Box<dyn std::error::Error>> {
    let msg = b"ok";
    let payload: &[u8] = b"";
    let mut b = [0u8; 128];
    let complete = ErgoAdminResponseEncoder::wrap_and_apply_header(&mut b, 0)
        .fixed(&ErgoAdminResponseFixedFields {
            cluster_session_id: 1,
            correlation_id: 2,
            request_type: ErgoAdminRequestType::SNAPSHOT,
            response_code: ErgoAdminResponseCode::OK,
        })
        .message(msg)?
        .payload(payload)?;
    assert_eq!(complete.as_bytes_with_header(), &GOLDEN_ADMIN_RESPONSE[..]);

    Ok(())
}
