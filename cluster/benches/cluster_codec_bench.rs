//! Head-to-head cluster codec benchmarks: ErgoSBE vs sbe-tool 1.39.0.
//!
//! Both codecs encode byte-identical output (proved by 18/18 golden parity
//! tests); this measures speed on equal work. The sbe-tool codecs are still
//! committed in `cluster_codecs/` — these benchmarks provide the acceptance
//! numbers before the sbe-tool codecs are deleted per the ErgoSBE migration.
//!
//! Acceptance: 5-run median ErgoSBE/sbe-tool ratio ≤ 1.00 on every case.
#![allow(unused_must_use, unused_imports)]

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

// Use `ergo_aeron_cluster::codecs::ergo_codecs` for the ErgoSBE codecs and
// `cluster_codecs` for the sbe-tool codecs. Both modules coexist in the crate
// during the migration transition.

const HFT_BATCH: usize = 10_000;

/// Encode 10k SessionMessageHeaders via ErgoSBE.
fn bench_encode_msg_header_ergo(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/session_message_header");
    g.throughput(Throughput::Elements(HFT_BATCH as u64));
    g.bench_function("ergosbe", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * 32],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * 32;
                    let _ =
                        ergo_aeron_cluster::codecs::ergo_codecs::SessionMessageHeaderEncoder::wrap_and_apply_header(
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
            || vec![0u8; HFT_BATCH * 32],
            |mut buf| {
                use ergo_aeron_cluster::codecs::cluster_codecs::{
                    WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder,
                };
                for i in 0..HFT_BATCH {
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

/// Encode 10k SessionKeepAlives via ErgoSBE.
fn bench_encode_keep_alive_ergo(c: &mut Criterion) {
    let mut g = c.benchmark_group("cluster/encode/session_keep_alive");
    g.throughput(Throughput::Elements(HFT_BATCH as u64));
    let sz = 8 + ergo_aeron_cluster::codecs::ergo_codecs::SessionKeepAliveEncoder::BLOCK_LENGTH;
    g.bench_function("ergosbe", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * sz],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * sz;
                    let _ = ergo_aeron_cluster::codecs::ergo_codecs::SessionKeepAliveEncoder::wrap_and_apply_header(
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
            || vec![0u8; HFT_BATCH * sz],
            |mut buf| {
                use ergo_aeron_cluster::codecs::cluster_codecs::{
                    WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder,
                };
                for i in 0..HFT_BATCH {
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
    g.throughput(Throughput::Elements(HFT_BATCH as u64));
    g.bench_function("ergosbe", |b| {
        b.iter_batched(
            || vec![0u8; HFT_BATCH * 128],
            |mut buf| {
                for i in 0..HFT_BATCH {
                    let off = i * 128;
                    let mut enc =
                        ergo_aeron_cluster::codecs::ergo_codecs::SessionConnectRequestEncoder::wrap_and_apply_header(
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
            || vec![0u8; HFT_BATCH * 128],
            |mut buf| {
                use ergo_aeron_cluster::codecs::cluster_codecs::{
                    WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder,
                };
                for i in 0..HFT_BATCH {
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

criterion_group!(
    benches,
    bench_encode_msg_header_ergo,
    bench_encode_keep_alive_ergo,
    bench_encode_connect_request_ergo,
);
criterion_main!(benches);
