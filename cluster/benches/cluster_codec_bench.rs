//! Head-to-head cluster codec benchmarks: ergon vs sbe-tool 1.39.0.
//!
//! Both codecs encode byte-identical output (proved by 18/18 golden parity
//! tests); this measures speed on **equal work**. The sbe-tool codecs remain
//! only for these benches under `reference_sbe/`.
//!
//! Acceptance: 5-run median ergon/sbe-tool ratio ≤ 1.00 on every **maintained**
//! case:
//! - encode: session message header, keep-alive, **claim_shaped** (header + app)
//! - decode: session message header, session event (equal-work audit)
//!
//! Diagnostic-only (not a ≤1.00 gate): NewLeaderEvent decode, connect request.
#![allow(unused_must_use, unused_imports)]

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

/// sbe-tool 1.39.0 reference runtime — Criterion-private (forbid production imports).
mod reference_sbe;

const BATCH_SIZE: usize = 10_000;

/// Encode 10k SessionMessageHeaders via ergon.
fn bench_encode_msg_header_ergo(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/session_message_header");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * 32],
            |mut buf| {
                for i in 0..BATCH_SIZE {
                    let off = i * 32;
                    let _ = ergo_aeron_cluster::codecs::session::SessionMessageHeaderEncoder::wrap_and_apply_header(
                        &mut buf[off..off + 32],
                        0,
                    )
                    .unwrap()
                    .leadership_term_id((i % 1000) as i64)
                    .cluster_session_id(42)
                    .timestamp(0);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });
    g.bench_function("sbe-tool", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * 32],
            |mut buf| {
                use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
                for i in 0..BATCH_SIZE {
                    let off = i * 32;
                    let wb = WriteBuf::new(&mut buf[off..off + 32]);
                    let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
                    enc.leadership_term_id((i % 1000) as i64);
                    enc.cluster_session_id(42);
                    enc.timestamp(0);
                    let _ = enc.header(0);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });
    g.finish();
}

/// Encode 10k SessionKeepAlives via ergon.
fn bench_encode_keep_alive_ergo(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/session_keep_alive");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    let sz = 8 + ergo_aeron_cluster::codecs::session::SessionKeepAliveEncoder::BLOCK_LENGTH;
    g.bench_function("ergo-sbe", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * sz],
            |mut buf| {
                for i in 0..BATCH_SIZE {
                    let off = i * sz;
                    let _ = ergo_aeron_cluster::codecs::session::SessionKeepAliveEncoder::wrap_and_apply_header(
                        &mut buf[off..off + sz],
                        0,
                    )
                    .unwrap()
                    .leadership_term_id(5)
                    .cluster_session_id((i % 100) as i64);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });
    g.bench_function("sbe-tool", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * sz],
            |mut buf| {
                use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder};
                for i in 0..BATCH_SIZE {
                    let off = i * sz;
                    let wb = WriteBuf::new(&mut buf[off..off + sz]);
                    let mut enc = SessionKeepAliveEncoder::default().wrap(wb, 8);
                    enc.leadership_term_id(5);
                    enc.cluster_session_id((i % 100) as i64);
                    let _ = enc.header(0);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });
    g.finish();
}

/// Encode 10k SessionConnectRequests (with 33-byte channel, 9-byte creds,
/// empty client_info — the complete 78-byte message both codecs produce).
fn bench_encode_connect_request_ergo(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/session_connect_request");
    let channel = "aeron:udp?endpoint=localhost:9999";
    let creds = b"user:pass";
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * 128],
            |mut buf| {
                for i in 0..BATCH_SIZE {
                    let off = i * 128;
                    let mut enc =
                        ergo_aeron_cluster::codecs::session::SessionConnectRequestEncoder::wrap_and_apply_header(
                            &mut buf[off..off + 128],
                            0,
                        )
                        .unwrap();
                    let _ = enc.correlation_id(0).response_stream_id(102).version(0);
                    let _ = enc
                        .response_channel(channel.as_bytes())
                        .unwrap()
                        .encoded_credentials(creds)
                        .unwrap()
                        .client_info(b"")
                        .unwrap();
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });
    g.bench_function("sbe-tool", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * 128],
            |mut buf| {
                use reference_sbe::{WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder};
                for i in 0..BATCH_SIZE {
                    let off = i * 128;
                    let wb = WriteBuf::new(&mut buf[off..off + 128]);
                    let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
                    enc.correlation_id(0);
                    enc.response_stream_id(102);
                    enc.version(0);
                    enc.response_channel(channel.as_bytes());
                    enc.encoded_credentials(creds);
                    enc.client_info(b"");
                    let _ = enc.header(0);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
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

#[inline(always)]
fn sbe_tool_header_ok(template_id: u16, schema_id: u16, expected_tid: u16, expected_sid: u16) -> bool {
    // Same two comparisons ergon performs in wrap_and_apply_header.
    // Return value is black_box'd so LLVM cannot DCE the checks.
    black_box(template_id == expected_tid && schema_id == expected_sid)
}

fn bench_decode_msg_header(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/decode/session_message_header");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                let d = ergo_aeron_cluster::codecs::session::SessionMessageHeaderDecoder::wrap_and_apply_header(
                    black_box(&MSG_HDR_FIXTURE[..]),
                    0,
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
                // Equal bounds gate (Ergo checks pos+8 <= len before reading).
                if buf.len() < 8 {
                    continue;
                }
                let rb = ReadBuf::new(buf);
                let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
                if !sbe_tool_header_ok(
                    hdr.template_id(),
                    hdr.schema_id(),
                    MSG_HDR_TEMPLATE_ID,
                    MSG_HDR_SCHEMA_ID,
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

// ── Decode: SessionEvent (var-data "some-detail", 67 bytes) ─────────────

const SESSION_EVENT_FIXTURE: [u8; 67] = [
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x73,
    0x6f, 0x6d, 0x65, 0x2d, 0x64, 0x65, 0x74, 0x61, 0x69, 0x6c,
];

const SESSION_EVENT_TEMPLATE_ID: u16 = 2;
const SESSION_EVENT_SCHEMA_ID: u16 = 111;

fn bench_decode_session_event(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/decode/session_event");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                use ergo_aeron_cluster::codecs::session::SessionEventDecoder;
                let dec = SessionEventDecoder::wrap_and_apply_header(black_box(&SESSION_EVENT_FIXTURE[..]), 0).unwrap();
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
                if buf.len() < 8 {
                    continue;
                }
                let rb = ReadBuf::new(buf);
                let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
                if !sbe_tool_header_ok(
                    hdr.template_id(),
                    hdr.schema_id(),
                    SESSION_EVENT_TEMPLATE_ID,
                    SESSION_EVENT_SCHEMA_ID,
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

fn new_leader_fixture() -> Vec<u8> {
    use ergo_aeron_cluster::codecs::session::NewLeaderEventEncoder;
    let mut buf = vec![0u8; 256];
    let mut enc = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    let _ = enc.cluster_session_id(2).leadership_term_id(9).leader_member_id(1);
    let complete = enc.ingress_endpoints(b"0=localhost:9000,1=localhost:9100").unwrap();
    let len = complete.encoded_length_with_header();
    buf.truncate(len);
    buf
}

fn bench_decode_new_leader(c: &mut Criterion) {
    let fixture = new_leader_fixture();
    let mut g = c.benchmark_group("cluster/decode/new_leader_event");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                use ergo_aeron_cluster::codecs::session::NewLeaderEventDecoder;
                let dec = NewLeaderEventDecoder::wrap_and_apply_header(black_box(fixture.as_slice()), 0).unwrap();
                let csid = dec.cluster_session_id();
                let ltid = dec.leadership_term_id();
                let lmid = dec.leader_member_id();
                let (eps, _) = dec.into_ingress_endpoints().unwrap();
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
                if buf.len() < 8 {
                    continue;
                }
                let rb = ReadBuf::new(buf);
                let hdr = MessageHeaderDecoder::default().wrap(rb, 0);
                if !sbe_tool_header_ok(
                    hdr.template_id(),
                    hdr.schema_id(),
                    NEW_LEADER_TEMPLATE_ID,
                    NEW_LEADER_SCHEMA_ID,
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

// ── Claim-shaped write: SessionMessageHeader + fixed 32-byte app payload ─

const CLAIM_APP_PAYLOAD: [u8; 32] = [0xABu8; 32];

fn bench_claim_shaped_write(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/claim_shaped_header_plus_app");
    let total = 32 + CLAIM_APP_PAYLOAD.len(); // MSG_HDR_TOTAL + app
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * total],
            |mut buf| {
                for i in 0..BATCH_SIZE {
                    let off = i * total;
                    let slot = &mut buf[off..off + total];
                    let _ = ergo_aeron_cluster::codecs::session::SessionMessageHeaderEncoder::wrap_and_apply_header(
                        &mut slot[..32],
                        0,
                    )
                    .unwrap()
                    .leadership_term_id(5)
                    .cluster_session_id(42)
                    .timestamp(0);
                    slot[32..].copy_from_slice(&CLAIM_APP_PAYLOAD);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
    });
    g.bench_function("sbe-tool", |b| {
        b.iter_batched(
            || vec![0u8; BATCH_SIZE * total],
            |mut buf| {
                use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
                for i in 0..BATCH_SIZE {
                    let off = i * total;
                    let slot = &mut buf[off..off + total];
                    let wb = WriteBuf::new(&mut slot[..32]);
                    let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, 8);
                    enc.leadership_term_id(5);
                    enc.cluster_session_id(42);
                    enc.timestamp(0);
                    let _ = enc.header(0);
                    slot[32..].copy_from_slice(&CLAIM_APP_PAYLOAD);
                }
                black_box(&buf);
            },
            criterion::BatchSize::LargeInput,
        );
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
