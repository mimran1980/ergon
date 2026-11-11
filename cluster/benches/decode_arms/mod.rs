//! Per-message bodies of the cluster decode pairs' timed arms.
//!
//! `cluster_codec_bench` times these inside its loops, and
//! `cluster/tests/bench_decode_arms_agree.rs` feeds both arms of each pair the
//! same truncated and corrupted frames and requires the same outcome: a panic,
//! a fail-closed skip (`None`) or equal values. An arm that skips where the
//! other panics leaves out a check the other pays for, and the ratio then
//! measures the harness.
//!
//! Each arm hands its values to `observe` and returns its result, or `None`
//! when it skips the frame. `#[inline(always)]` keeps the timed loops what they
//! were when the bodies were written inline.

// Timed code, linted as the bench that times it (its casts and the sbe-tool
// calls stay as the gate measured them).
#![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]

use crate::reference_sbe;

/// Standard SBE message-header size.
const HDR: usize = 8;

#[inline(always)]
pub fn ergo_session_message_header<R>(buf: &[u8], observe: impl FnOnce((i64, i64, i64)) -> R) -> Option<R> {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder;
    let d = SessionMessageHeaderDecoder::wrap(
        buf,
        0,
        SessionMessageHeaderDecoder::BLOCK_LENGTH,
        SessionMessageHeaderDecoder::SCHEMA_VERSION,
    );
    Some(observe((d.leadership_term_id(), d.cluster_session_id(), d.timestamp())))
}

#[inline(always)]
pub fn tool_session_message_header<R>(buf: &[u8], observe: impl FnOnce((i64, i64, i64)) -> R) -> Option<R> {
    use reference_sbe::{ReadBuf, session_message_header_codec::SessionMessageHeaderDecoder};
    let block = ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::BLOCK_LENGTH;
    // Same extent check ergon wrap() performs, with the same outcome (panic).
    // No header identity.
    assert!(buf.len() >= HDR + block, "buffer too short for message body");
    let d = SessionMessageHeaderDecoder::default().wrap(
        ReadBuf::new(buf),
        HDR,
        block as u16,
        ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::SCHEMA_VERSION,
    );
    Some(observe((d.leadership_term_id(), d.cluster_session_id(), d.timestamp())))
}

/// SessionEvent values: correlation id, cluster session id, leadership term
/// id, leader member id, code, detail.
pub type SessionEvent<'d, C> = (i64, i64, i64, i32, C, &'d [u8]);

#[inline(always)]
pub fn ergo_session_event<R>(
    buf: &[u8],
    observe: impl FnOnce(SessionEvent<'_, ergo_aeron_cluster::cluster_codec_types::EventCode>) -> R,
) -> Option<R> {
    use ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder;
    // Use wrap() — extent-only check, no header field validation.
    // sbe-tool arm does equivalent manual checks.
    let dec = SessionEventDecoder::wrap(
        buf,
        0,
        SessionEventDecoder::BLOCK_LENGTH,
        SessionEventDecoder::SCHEMA_VERSION,
    );
    let cid = dec.correlation_id();
    let csid = dec.cluster_session_id();
    let ltid = dec.leadership_term_id();
    let lmid = dec.leader_member_id();
    let code = dec.code();
    // Slice only — matches sbe-tool `detail_slice`. `into_detail()` also
    // constructs the next stage and is more work. The skip matches the
    // sbe-tool arm's fail-closed skip (no panic path).
    let Ok(detail) = dec.detail_slice() else {
        return None;
    };
    Some(observe((cid, csid, ltid, lmid, code, detail)))
}

#[inline(always)]
pub fn tool_session_event<R>(
    buf: &[u8],
    observe: impl FnOnce(SessionEvent<'_, reference_sbe::event_code::EventCode>) -> R,
) -> Option<R> {
    use reference_sbe::{ReadBuf, session_event_codec::SessionEventDecoder};
    let block = ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::BLOCK_LENGTH;
    // Same extent check ergon wrap() performs, with the same outcome (panic):
    // a skip here would add a second back-edge. No header identity.
    assert!(buf.len() >= HDR + block, "buffer too short for message body");
    let mut dec = SessionEventDecoder::default().wrap(
        ReadBuf::new(buf),
        HDR,
        block as u16,
        ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::SCHEMA_VERSION,
    );
    // Field reads first (matches Ergo order: scalars then detail).
    let cid = dec.correlation_id();
    let csid = dec.cluster_session_id();
    let ltid = dec.leadership_term_id();
    let lmid = dec.leader_member_id();
    let code = dec.code();
    // Equal-work var-data gates, each a fail-closed skip as in the Ergo arm's
    // `detail_slice()`: length prefix in bounds, length cap, data in bounds.
    if buf.len() < HDR + block + 4 {
        return None;
    }
    let coords = dec.detail_decoder();
    let (off, len) = coords;
    if len > 1_073_741_824 || off.saturating_add(len) > buf.len() {
        return None;
    }
    let detail = dec.detail_slice(coords);
    Some(observe((cid, csid, ltid, lmid, code, detail)))
}

/// NewLeaderEvent values: cluster session id, leadership term id, leader
/// member id, ingress endpoints.
pub type NewLeaderEvent<'d> = (i64, i64, i32, &'d [u8]);

#[inline(always)]
pub fn ergo_new_leader_event<R>(buf: &[u8], observe: impl FnOnce(NewLeaderEvent<'_>) -> R) -> Option<R> {
    use ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder;
    let dec = NewLeaderEventDecoder::wrap(
        buf,
        0,
        NewLeaderEventDecoder::BLOCK_LENGTH,
        NewLeaderEventDecoder::SCHEMA_VERSION,
    );
    let csid = dec.cluster_session_id();
    let ltid = dec.leadership_term_id();
    let lmid = dec.leader_member_id();
    // Fail-closed skip, as the sbe-tool arm's var-data gates.
    let Ok(eps) = dec.ingress_endpoints_slice() else {
        return None;
    };
    Some(observe((csid, ltid, lmid, eps)))
}

#[inline(always)]
pub fn tool_new_leader_event<R>(buf: &[u8], observe: impl FnOnce(NewLeaderEvent<'_>) -> R) -> Option<R> {
    use reference_sbe::{ReadBuf, new_leader_event_codec::NewLeaderEventDecoder};
    let block = ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder::BLOCK_LENGTH;
    // Same extent check ergon wrap() performs, with the same outcome (panic).
    // No header identity.
    assert!(buf.len() >= HDR + block, "buffer too short for message body");
    let mut dec = NewLeaderEventDecoder::default().wrap(
        ReadBuf::new(buf),
        HDR,
        block as u16,
        ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder::SCHEMA_VERSION,
    );
    let csid = dec.cluster_session_id();
    let ltid = dec.leadership_term_id();
    let lmid = dec.leader_member_id();
    // Equal-work var-data gates, each a fail-closed skip as in the Ergo arm's
    // `ingress_endpoints_slice()`: length prefix in bounds, length cap, data in
    // bounds.
    if buf.len() < HDR + block + 4 {
        return None;
    }
    let coords = dec.ingress_endpoints_decoder();
    let (off, len) = coords;
    if len > 1_073_741_824 || off.saturating_add(len) > buf.len() {
        return None;
    }
    let eps = dec.ingress_endpoints_slice(coords);
    Some(observe((csid, ltid, lmid, eps)))
}
