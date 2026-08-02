//! Head-to-head cluster codec benchmarks: ergon vs sbe-tool 1.39.0.
//!
//! Both codecs encode byte-identical **body** output; this measures speed on
//! **equal work**. The sbe-tool codecs remain only for these benches under
//! `reference_sbe/`.
//!
//! ## Header equal-work (mandatory)
//!
//! Match sbe-tool. Do **not** always call ergon `wrap_and_apply_header`.
//!
//! Official sbe-tool Rust encode (see `simple-binary-encoding/rust/benches/car_benchmark.rs`):
//! ```text
//! enc = enc.wrap(WriteBuf::new(buf), message_header_codec::ENCODED_LENGTH); // body @ 8
//! enc = enc.header(0).parent()?; // optional: write MessageHeader at 0, then body setters
//! // … body field setters …
//! let body_len = enc.encoded_length(); // body only — does NOT include the 8-byte header
//! ```
//!
//! These cluster **encode** gates time **body-only** work on both arms:
//! - sbe-tool: `wrap(buf, 8)` + field setters — **no** `.header(0)`
//! - ergon: `wrap(buf, 0)` + field setters — **no** `wrap_and_apply_header`
//!
//! Body bytes live at absolute offsets `[8 .. 8+BLOCK)`. Header region `[0..8)`
//! stays zero. Length asserts use body length only (`encoded_length()`), never
//! a synthetic `8 + body` that pretends a header was written.
//!
//! If a future case needs full wire (header + body), both arms must apply the
//! header: ergon `wrap_and_apply_header` ↔ sbe-tool `wrap(8)` then
//! `header(0).parent()` **before** body setters (official order).
//!
//! Acceptance: 5-run median ergon/sbe-tool ratio ≤ 1.00 on every **maintained**
//! case:
//! - encode: session message header, keep-alive, **claim_shaped** (body + app)
//! - decode: session message header, session event (equal-work audit)
//!
//! Diagnostic-only (not a ≤1.00 gate): NewLeaderEvent decode, connect request.
#![allow(unused_must_use, unused_imports)]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

/// sbe-tool 1.39.0 reference runtime — Criterion-private (forbid production imports).
mod reference_sbe;

const BATCH_SIZE: usize = 10_000;
/// Standard SBE message-header size (same as `message_header_codec::ENCODED_LENGTH`).
const HDR: usize = 8;

fn assert_body_parity(case: &str, ergo: &[u8], sbe_tool: &[u8], body_len: usize) {
    assert_eq!(
        &ergo[..HDR],
        &[0u8; HDR],
        "{case}: ergon header region must stay zero in body-only mode"
    );
    assert_eq!(
        &sbe_tool[..HDR],
        &[0u8; HDR],
        "{case}: sbe-tool header region must stay zero without .header(0)"
    );
    assert_eq!(
        &ergo[HDR..HDR + body_len],
        &sbe_tool[HDR..HDR + body_len],
        "{case}: body bytes differ"
    );
}

fn assert_session_message_header_encode_parity() {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder as ErgoEncoder;
    use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder as ToolEncoder};

    let body = ErgoEncoder::BLOCK_LENGTH;
    let slot = HDR + body;

    let mut ergo = vec![0u8; slot];
    let ergo_body = ErgoEncoder::wrap(&mut ergo, 0)
        .leadership_term_id(5)
        .cluster_session_id(42)
        .timestamp(0)
        .encoded_length();

    let mut sbe_tool = vec![0u8; slot];
    let mut tool = ToolEncoder::default().wrap(WriteBuf::new(&mut sbe_tool), HDR);
    tool.leadership_term_id(5).cluster_session_id(42).timestamp(0);
    let tool_body = tool.encoded_length();
    // Intentionally no tool.header(0) — body-only equal work with ergon wrap.

    assert_eq!(ergo_body, body, "session message header ergon body length");
    assert_eq!(tool_body, body, "session message header sbe-tool body length");
    assert_eq!(ergo_body, tool_body, "session message header body lengths");
    assert_body_parity("session message header", &ergo, &sbe_tool, body);
}

fn assert_session_keep_alive_encode_parity() {
    use ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder as ErgoEncoder;
    use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder as ToolEncoder};

    let body = ErgoEncoder::BLOCK_LENGTH;
    let slot = HDR + body;

    let mut ergo = vec![0u8; slot];
    let ergo_body = ErgoEncoder::wrap(&mut ergo, 0)
        .leadership_term_id(5)
        .cluster_session_id(42)
        .encoded_length();

    let mut sbe_tool = vec![0u8; slot];
    let mut tool = ToolEncoder::default().wrap(WriteBuf::new(&mut sbe_tool), HDR);
    tool.leadership_term_id(5).cluster_session_id(42);
    let tool_body = tool.encoded_length();

    assert_eq!(ergo_body, body, "keep-alive ergon body length");
    assert_eq!(tool_body, body, "keep-alive sbe-tool body length");
    assert_eq!(ergo_body, tool_body, "keep-alive body lengths");
    assert_body_parity("session keep-alive", &ergo, &sbe_tool, body);
}

fn assert_session_connect_request_encode_parity(channel: &[u8], credentials: &[u8]) {
    use ergo_aeron_cluster::cluster_codec_types::{
        SessionConnectRequestEncoder as ErgoEncoder, SessionConnectRequestFixedFields,
    };
    use reference_sbe::{WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder as ToolEncoder};

    // Full frame capacity (header reserved + body) — header bytes left zero.
    let frame_len = ErgoEncoder::compute_length_with_header(channel.len(), credentials.len(), 0);
    let fixed = SessionConnectRequestFixedFields {
        correlation_id: 0,
        response_stream_id: 102,
        version: Some(0),
    };

    let mut ergo = vec![0u8; frame_len];
    let ergo_body = ErgoEncoder::wrap(&mut ergo, 0)
        .fixed(&fixed)
        .response_channel(channel)
        .unwrap()
        .encoded_credentials(credentials)
        .unwrap()
        .client_info(b"")
        .unwrap()
        .encoded_length();

    let mut sbe_tool = vec![0u8; frame_len];
    let mut tool = ToolEncoder::default().wrap(WriteBuf::new(&mut sbe_tool), HDR);
    tool.correlation_id(0)
        .response_stream_id(102)
        .version(0)
        .response_channel(channel)
        .encoded_credentials(credentials)
        .client_info(b"");
    let tool_body = tool.encoded_length();

    assert_eq!(ergo_body, tool_body, "session connect body lengths");
    assert_eq!(
        ergo_body + HDR,
        frame_len,
        "body + reserved header region == compute_length_with_header capacity"
    );
    assert_body_parity("session connect request", &ergo, &sbe_tool, ergo_body);
}

/// Encode 10k SessionMessageHeader **bodies** (no MessageHeader write).
fn bench_encode_msg_header_ergo(c: &mut Criterion) {
    assert_session_message_header_encode_parity();
    let mut g = c.benchmark_group("cluster/encode/session_message_header");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    let slot = HDR + ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::BLOCK_LENGTH;
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * slot];
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                let off = i * slot;
                let _ = ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::wrap(
                    black_box(&mut buf[off..off + slot]),
                    0,
                )
                .unwrap()
                .leadership_term_id((i % 1000) as i64)
                .cluster_session_id(42)
                .timestamp(0);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * slot];
        b.iter(|| {
            use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
            for i in 0..BATCH_SIZE {
                let off = i * slot;
                let wb = WriteBuf::new(black_box(&mut buf[off..off + slot]));
                let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, HDR);
                enc.leadership_term_id((i % 1000) as i64);
                enc.cluster_session_id(42);
                enc.timestamp(0);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

/// Encode 10k SessionKeepAlive **bodies** (no MessageHeader write).
fn bench_encode_keep_alive_ergo(c: &mut Criterion) {
    assert_session_keep_alive_encode_parity();
    let mut g = c.benchmark_group("cluster/encode/session_keep_alive");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    let slot = HDR + ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::BLOCK_LENGTH;
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * slot];
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                let off = i * slot;
                let _ = ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::wrap(
                    black_box(&mut buf[off..off + slot]),
                    0,
                )
                .unwrap()
                .leadership_term_id(5)
                .cluster_session_id((i % 100) as i64);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * slot];
        b.iter(|| {
            use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder};
            for i in 0..BATCH_SIZE {
                let off = i * slot;
                let wb = WriteBuf::new(black_box(&mut buf[off..off + slot]));
                let mut enc = SessionKeepAliveEncoder::default().wrap(wb, HDR);
                enc.leadership_term_id(5);
                enc.cluster_session_id((i % 100) as i64);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

/// Encode 10k SessionConnectRequest **bodies** (no MessageHeader write).
fn bench_encode_connect_request_ergo(c: &mut Criterion) {
    use ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestFixedFields;

    let mut g = c.benchmark_group("cluster/encode/session_connect_request");
    let channel = "aeron:udp?endpoint=localhost:9999";
    let creds = b"user:pass";
    let fixed = SessionConnectRequestFixedFields {
        correlation_id: 0,
        response_stream_id: 102,
        version: Some(0),
    };
    assert_session_connect_request_encode_parity(channel.as_bytes(), creds);
    let frame_len = ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestEncoder::compute_length_with_header(
        channel.len(),
        creds.len(),
        0,
    );
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * frame_len];
        b.iter(|| {
            let channel = black_box(channel.as_bytes());
            let creds = black_box(creds.as_slice());
            for i in 0..BATCH_SIZE {
                let off = i * frame_len;
                let len = ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestEncoder::wrap(
                    black_box(&mut buf[off..off + frame_len]),
                    0,
                )
                .unwrap()
                .fixed(black_box(&fixed))
                .response_channel(channel)
                .unwrap()
                .encoded_credentials(creds)
                .unwrap()
                .client_info(b"")
                .unwrap()
                .encoded_length();
                black_box(len);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * frame_len];
        b.iter(|| {
            use reference_sbe::{WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder};
            let channel = black_box(channel.as_bytes());
            let creds = black_box(creds.as_slice());
            for i in 0..BATCH_SIZE {
                let off = i * frame_len;
                let wb = WriteBuf::new(black_box(&mut buf[off..off + frame_len]));
                let mut enc = SessionConnectRequestEncoder::default().wrap(wb, HDR);
                enc.correlation_id(0);
                enc.response_stream_id(102);
                enc.version(0);
                enc.response_channel(channel);
                enc.encoded_credentials(creds);
                enc.client_info(b"");
                let len = enc.encoded_length(); // body only
                black_box(len);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

// ── Decode: SessionMessageHeader (fixed, 32 bytes) ──────────────────────
//
// Equal-work audit (2026-07-18): production ergon `wrap_and_apply_header`
// always checks buffer length + template_id + schema_id. sbe-tool's
// `header()` only `debug_assert`s template_id (elided in release). Without
// matching release checks the sbe-tool arm was under-worked (~1.17× unfair).

const MSG_HDR_FIXTURE: [u8; 32] = [
    0x18, 0x00, 0x01, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0xd2, 0x02, 0x96, 0x49, 0x00, 0x00, 0x00, 0x00,
];

const MSG_HDR_TEMPLATE_ID: u16 = 1;
const MSG_HDR_SCHEMA_ID: u16 = 111;
const MSG_HDR_SCHEMA_VERSION: u16 = 16;

#[inline(always)]
fn sbe_tool_header_ok(
    template_id: u16,
    schema_id: u16,
    version: u16,
    expected_tid: u16,
    expected_sid: u16,
    expected_ver: u16,
) -> bool {
    // Same three comparisons ergon decode() performs: template_id, schema_id, version.
    // Return value is black_box'd so LLVM cannot DCE the checks.
    black_box(template_id == expected_tid && schema_id == expected_sid && version <= expected_ver)
}

fn bench_decode_msg_header(c: &mut Criterion) {
    {
        use reference_sbe::{
            ReadBuf, message_header_codec::MessageHeaderDecoder,
            session_message_header_codec::SessionMessageHeaderDecoder,
        };
        let ergo = ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::decode(&MSG_HDR_FIXTURE, 0);
        let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(&MSG_HDR_FIXTURE), 0);
        let tool = SessionMessageHeaderDecoder::default().header(header, 0);
        assert_eq!(ergo.leadership_term_id(), tool.leadership_term_id());
        assert_eq!(ergo.cluster_session_id(), tool.cluster_session_id());
        assert_eq!(ergo.timestamp(), tool.timestamp());
    }
    let mut g = c.benchmark_group("cluster/decode/session_message_header");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                let d = ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::wrap(
                    black_box(&MSG_HDR_FIXTURE[..]),
                    0,
                    ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::BLOCK_LENGTH,
                    ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::SCHEMA_VERSION,
                )
                .unwrap();
                black_box((d.leadership_term_id(), d.cluster_session_id(), d.timestamp()));
            }
        });
    });
    g.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use reference_sbe::{
                ReadBuf, message_header_codec::MessageHeaderDecoder,
                session_message_header_codec::SessionMessageHeaderDecoder,
            };
            for _ in 0..BATCH_SIZE {
                let buf = black_box(&MSG_HDR_FIXTURE[..]);
                // Equal-work: same extent check ergon wrap() performs.
                if buf.len() < 8 + ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::BLOCK_LENGTH {
                    continue;
                }
                let rb = ReadBuf::new(buf);
                let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
                if !sbe_tool_header_ok(
                    hdr.template_id(),
                    hdr.schema_id(),
                    hdr.version(),
                    MSG_HDR_TEMPLATE_ID,
                    MSG_HDR_SCHEMA_ID,
                    MSG_HDR_SCHEMA_VERSION,
                ) {
                    continue;
                }
                let d = SessionMessageHeaderDecoder::default().header(hdr, 0);
                black_box((d.leadership_term_id(), d.cluster_session_id(), d.timestamp()));
            }
        });
    });
    g.finish();
}

// ── Decode: SessionEvent (fixed + var-data detail) ───────────────────────

const SESSION_EVENT_FIXTURE: [u8; 67] = [
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x73,
    0x6f, 0x6d, 0x65, 0x2d, 0x64, 0x65, 0x74, 0x61, 0x69, 0x6c,
];

const SESSION_EVENT_TEMPLATE_ID: u16 = 2;
const SESSION_EVENT_SCHEMA_ID: u16 = 111;
const SESSION_EVENT_SCHEMA_VERSION: u16 = 16;

fn bench_decode_session_event(c: &mut Criterion) {
    {
        use reference_sbe::{
            ReadBuf, message_header_codec::MessageHeaderDecoder, session_event_codec::SessionEventDecoder,
        };
        let ergo = ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::decode(&SESSION_EVENT_FIXTURE, 0);
        let ergo_correlation_id = ergo.correlation_id();
        let ergo_cluster_session_id = ergo.cluster_session_id();
        let ergo_leadership_term_id = ergo.leadership_term_id();
        let ergo_leader_member_id = ergo.leader_member_id();
        let ergo_code = ergo.code() as u8;
        let (ergo_detail, _) = ergo.into_detail().unwrap();

        let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(&SESSION_EVENT_FIXTURE), 0);
        let mut tool = SessionEventDecoder::default().header(header, 0);
        assert_eq!(ergo_correlation_id, tool.correlation_id());
        assert_eq!(ergo_cluster_session_id, tool.cluster_session_id());
        assert_eq!(ergo_leadership_term_id, tool.leadership_term_id());
        assert_eq!(ergo_leader_member_id, tool.leader_member_id());
        assert_eq!(ergo_code, tool.code() as u8);
        let coords = tool.detail_decoder();
        assert_eq!(ergo_detail, tool.detail_slice(coords));
    }
    let mut g = c.benchmark_group("cluster/decode/session_event");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                use ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder;
                // Use wrap() — extent-only check, no header field validation.
                // sbe-tool arm does equivalent manual checks.
                let dec = SessionEventDecoder::wrap(
                    black_box(&SESSION_EVENT_FIXTURE[..]),
                    0,
                    SessionEventDecoder::BLOCK_LENGTH,
                    SessionEventDecoder::SCHEMA_VERSION,
                )
                .unwrap();
                let cid = dec.correlation_id();
                let csid = dec.cluster_session_id();
                let ltid = dec.leadership_term_id();
                let lmid = dec.leader_member_id();
                let code = dec.code();
                let (detail, _) = dec.into_detail().unwrap();
                black_box((cid, csid, ltid, lmid, code, detail));
            }
        });
    });
    g.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use reference_sbe::{
                ReadBuf, message_header_codec::MessageHeaderDecoder, session_event_codec::SessionEventDecoder,
            };
            for _ in 0..BATCH_SIZE {
                let buf = black_box(&SESSION_EVENT_FIXTURE[..]);
                // Same extent check ergon decode() performs: header + block must fit.
                if buf.len() < 8 + ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::BLOCK_LENGTH {
                    continue;
                }
                let rb = ReadBuf::new(buf);
                let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
                if !sbe_tool_header_ok(
                    hdr.template_id(),
                    hdr.schema_id(),
                    hdr.version(),
                    SESSION_EVENT_TEMPLATE_ID,
                    SESSION_EVENT_SCHEMA_ID,
                    SESSION_EVENT_SCHEMA_VERSION,
                ) {
                    continue;
                }
                let mut dec = SessionEventDecoder::default().header(hdr, 0);
                // Field reads first (matches Ergo order: scalars then detail).
                let cid = dec.correlation_id();
                let csid = dec.cluster_session_id();
                let ltid = dec.leadership_term_id();
                let lmid = dec.leader_member_id();
                let code = dec.code();
                let coords = dec.detail_decoder();
                // Equal-work var-data gates: length cap + slice bounds (Ergo into_detail).
                let (off, len) = coords;
                if len > 1_073_741_824 || off.saturating_add(len) > buf.len() {
                    continue;
                }
                let detail = dec.detail_slice(coords);
                black_box((cid, csid, ltid, lmid, code, detail));
            }
        });
    });
    g.finish();
}

// ── Decode: NewLeaderEvent (var-data endpoints) ──────────────────────────

const NEW_LEADER_TEMPLATE_ID: u16 = 3;
const NEW_LEADER_SCHEMA_ID: u16 = 111;
const NEW_LEADER_SCHEMA_VERSION: u16 = 16;

fn new_leader_fixture() -> Vec<u8> {
    use ergo_aeron_cluster::cluster_codec_types::{NewLeaderEventEncoder, NewLeaderEventFixedFields};

    const ENDPOINTS: &[u8] = b"0=localhost:9000,1=localhost:9100";
    let expected_len = NewLeaderEventEncoder::compute_length_with_header(ENDPOINTS.len());
    let mut buf = vec![0u8; expected_len];
    let len = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&NewLeaderEventFixedFields {
            cluster_session_id: 2,
            leadership_term_id: 9,
            leader_member_id: 1,
        })
        .ingress_endpoints(ENDPOINTS)
        .unwrap()
        .encoded_length_with_header();
    assert_eq!(len, expected_len);
    buf
}

fn bench_decode_new_leader(c: &mut Criterion) {
    let fixture = new_leader_fixture();
    {
        use reference_sbe::{
            ReadBuf, message_header_codec::MessageHeaderDecoder, new_leader_event_codec::NewLeaderEventDecoder,
        };
        let ergo = ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder::decode(&fixture, 0);
        let ergo_cluster_session_id = ergo.cluster_session_id();
        let ergo_leadership_term_id = ergo.leadership_term_id();
        let ergo_leader_member_id = ergo.leader_member_id();
        let ergo_endpoints = ergo.ingress_endpoints_slice().unwrap();

        let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(&fixture), 0);
        let mut tool = NewLeaderEventDecoder::default().header(header, 0);
        assert_eq!(ergo_cluster_session_id, tool.cluster_session_id());
        assert_eq!(ergo_leadership_term_id, tool.leadership_term_id());
        assert_eq!(ergo_leader_member_id, tool.leader_member_id());
        let coordinates = tool.ingress_endpoints_decoder();
        assert_eq!(ergo_endpoints, tool.ingress_endpoints_slice(coordinates));
    }
    let mut g = c.benchmark_group("cluster/decode/new_leader_event");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                use ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder;
                let dec = NewLeaderEventDecoder::wrap(
                    black_box(fixture.as_slice()),
                    0,
                    NewLeaderEventDecoder::BLOCK_LENGTH,
                    NewLeaderEventDecoder::SCHEMA_VERSION,
                )
                .unwrap();
                let csid = dec.cluster_session_id();
                let ltid = dec.leadership_term_id();
                let lmid = dec.leader_member_id();
                let eps = dec.ingress_endpoints_slice().unwrap();
                black_box((csid, ltid, lmid, eps));
            }
        });
    });
    g.bench_function("sbe-tool", |b| {
        b.iter(|| {
            use reference_sbe::{
                ReadBuf, message_header_codec::MessageHeaderDecoder, new_leader_event_codec::NewLeaderEventDecoder,
            };
            for _ in 0..BATCH_SIZE {
                let buf = black_box(fixture.as_slice());
                // Equal-work: same extent check ergon wrap() performs.
                if buf.len() < 8 + ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder::BLOCK_LENGTH {
                    continue;
                }
                let rb = ReadBuf::new(buf);
                let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
                if !sbe_tool_header_ok(
                    hdr.template_id(),
                    hdr.schema_id(),
                    hdr.version(),
                    NEW_LEADER_TEMPLATE_ID,
                    NEW_LEADER_SCHEMA_ID,
                    NEW_LEADER_SCHEMA_VERSION,
                ) {
                    continue;
                }
                let mut dec = NewLeaderEventDecoder::default().header(hdr, 0);
                let csid = dec.cluster_session_id();
                let ltid = dec.leadership_term_id();
                let lmid = dec.leader_member_id();
                let coords = dec.ingress_endpoints_decoder();
                let (off, len) = coords;
                if len > 1_073_741_824 || off.saturating_add(len) > buf.len() {
                    continue;
                }
                let eps = dec.ingress_endpoints_slice(coords);
                black_box((csid, ltid, lmid, eps));
            }
        });
    });
    g.finish();
}

// ── Claim-shaped write: SessionMessageHeader body + fixed 32-byte app ────
//
// Body-only session frame (header region zero) + app payload after the 32-byte
// slot. Equal work on both arms — no MessageHeader write.

const CLAIM_APP_PAYLOAD: [u8; 32] = [0xABu8; 32];

fn bench_claim_shaped_write(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/claim_shaped_header_plus_app");
    let hdr_slot = HDR + ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::BLOCK_LENGTH; // 32
    let total = hdr_slot + CLAIM_APP_PAYLOAD.len();
    {
        use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder as ErgoEncoder;
        use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder as ToolEncoder};

        let mut ergo = vec![0u8; total];
        let ergo_body = ErgoEncoder::wrap(&mut ergo[..hdr_slot], 0)
            .leadership_term_id(5)
            .cluster_session_id(42)
            .timestamp(0)
            .encoded_length();
        ergo[hdr_slot..].copy_from_slice(&CLAIM_APP_PAYLOAD);

        let mut sbe_tool = vec![0u8; total];
        let mut tool = ToolEncoder::default().wrap(WriteBuf::new(&mut sbe_tool[..hdr_slot]), HDR);
        tool.leadership_term_id(5).cluster_session_id(42).timestamp(0);
        let tool_body = tool.encoded_length();
        sbe_tool[hdr_slot..].copy_from_slice(&CLAIM_APP_PAYLOAD);

        assert_eq!(ergo_body, tool_body, "claim body length");
        assert_eq!(ergo_body, ErgoEncoder::BLOCK_LENGTH);
        assert_body_parity(
            "claim-shaped write",
            &ergo[..hdr_slot],
            &sbe_tool[..hdr_slot],
            ergo_body,
        );
        assert_eq!(&ergo[hdr_slot..], &sbe_tool[hdr_slot..], "claim app payload");
    }
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * total];
        b.iter(|| {
            for i in 0..BATCH_SIZE {
                let off = i * total;
                let slot = &mut buf[off..off + total];
                let _ = ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::wrap(
                    black_box(&mut slot[..hdr_slot]),
                    0,
                )
                .unwrap()
                .leadership_term_id(5)
                .cluster_session_id(42)
                .timestamp(0);
                slot[hdr_slot..].copy_from_slice(&CLAIM_APP_PAYLOAD);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * total];
        b.iter(|| {
            use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
            for i in 0..BATCH_SIZE {
                let off = i * total;
                let slot = &mut buf[off..off + total];
                let wb = WriteBuf::new(black_box(&mut slot[..hdr_slot]));
                let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, HDR);
                enc.leadership_term_id(5);
                enc.cluster_session_id(42);
                enc.timestamp(0);
                slot[hdr_slot..].copy_from_slice(&CLAIM_APP_PAYLOAD);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_encode_msg_header_ergo,
    bench_encode_keep_alive_ergo,
    bench_encode_connect_request_ergo,
    bench_decode_msg_header,
    bench_decode_session_event,
    bench_decode_new_leader,
    bench_claim_shaped_write,
);
criterion_main!(benches);
