//! Head-to-head cluster codec benchmarks: ergon vs sbe-tool 1.39.0.
//!
//! Both codecs encode byte-identical **body** output; this measures speed on
//! **equal work**. The sbe-tool codecs remain only for these benches under
//! `reference_sbe/`.
#![allow(missing_docs)]
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
//! Acceptance: the ergon/sbe-tool ratio, over 5 invocations each taken in both
//! arm orders (see `Pair`), within its ceiling in `check-bench-gate.sh` on every
//! **maintained** case:
//! - encode: session message header, keep-alive, **claim_shaped** (body + app)
//! - decode: session message header, session event (equal-work audit)
//!
//! Diagnostic-only (not a ≤1.00 gate): NewLeaderEvent decode, connect request.
#![allow(unused_must_use, unused_imports)]
// The head-to-head gate's measured loops (the unchecked tier's `unsafe`,
// `#[inline(always)]`, the casts both arms make) stay as the gate measured
// them: as sbe/benchmarks' benches and cluster_perf_probe, no workspace lints
#![allow(unsafe_code)]
#![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Bencher, Criterion, SamplingMode, Throughput, criterion_group, criterion_main, measurement::WallTime};

/// sbe-tool 1.39.0 reference runtime — Criterion-private (forbid production imports).
mod reference_sbe;

/// The decode pairs' per-message arm bodies, shared with the outcome-parity test.
mod decode_arms;

/// Sampling shared with the SBE parity suite. Criterion's default `Linear` mode
/// ramps a sample to tens of milliseconds of work; under that mode one arm of
/// an A/A pair took long-sample spikes and the other did not. Flat mode over a
/// short budget keeps samples under a few milliseconds. The rationale and the
/// A/A evidence are in `perf_parity_bench`'s `parity_criterion`.
fn parity_group<'c, 'f>(c: &'c mut Criterion, name: &str) -> Pair<'c, 'f> {
    let mut group = c.benchmark_group(name);
    group.sampling_mode(SamplingMode::Flat);
    Pair {
        group,
        arms: Vec::with_capacity(2),
    }
}

type Arm<'f> = Box<dyn FnMut(&mut Bencher<'_, WallTime>) + 'f>;

/// One head-to-head group. Its arms run in the order `ERGON_BENCH_ARM_ORDER`
/// names: unset or `forward` as registered, `reversed` backwards. Whichever arm
/// of a Criterion group runs first is measured differently from the one after
/// it, and the pairs are close enough for that to decide a verdict, so
/// `just bench-cluster` takes every invocation in both orders and the gate
/// combines the two ratios (`scripts/check-bench-gate.sh`, `verify_arm_order`).
/// The A/A controls go through the same wrapper, so their band describes the
/// same measurement.
///
/// Arms are held until [`Pair::finish`], so anything an arm borrows must be
/// declared before the group.
struct Pair<'c, 'f> {
    group: criterion::BenchmarkGroup<'c, WallTime>,
    arms: Vec<(&'static str, Arm<'f>)>,
}

const BATCH_SIZE: usize = 10_000;
/// Standard SBE message-header size (same as `message_header_codec::ENCODED_LENGTH`).
const HDR: usize = 8;

/// Per-slot buffer view shared by both arms of every encode pair.
///
/// The extent is proved once — by the allocation (`BATCH_SIZE * SLOT`) and the
/// parity assert in the gated function — and the timed region must not re-prove
/// it. A safe `buf[off..off + slot]` index emits a bounds check every
/// iteration, and rustc does not fuse that check the same way twice: on aarch64
/// it compiled the sbe-tool arm's as `adds`+`b.hs` and the ergon arm's as
/// `add`+`cmp`+`b.lo`, one extra instruction per iteration. That measured as a
/// 4.6% wall-clock gap on `cluster_encode_session_keep_alive` while the two hot
/// loops were otherwise instruction-identical, so the ratio was reading the
/// harness. Both arms index through this helper so the timed loops retire the
/// same instructions. Guarded mechanically by
/// `fairness_policy_test::encode_arms_index_the_buffer_without_a_per_iteration_bounds_check`.
///
/// # Safety
///
/// `base.add(off..off + len)` must be in bounds of the allocation the caller
/// owns, and the returned view must not be aliased by any other live `&mut`.
#[inline(always)]
const unsafe fn slot_slice<'a>(base: *mut u8, off: usize, len: usize) -> &'a mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(base.add(off), len) }
}

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

// Below the parity helpers on purpose: `fairness_policy_test` requires an exact
// assertion before the file's first `.bench_*` call.
impl<'f> Pair<'_, 'f> {
    fn throughput(&mut self, throughput: Throughput) {
        self.group.throughput(throughput);
    }

    fn bench_function(&mut self, id: &'static str, arm: impl FnMut(&mut Bencher<'_, WallTime>) + 'f) {
        self.arms.push((id, Box::new(arm)));
    }

    fn finish(self) {
        let Self { mut group, mut arms } = self;
        match std::env::var("ERGON_BENCH_ARM_ORDER").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("forward") => {}
            Ok("reversed") => arms.reverse(),
            other => panic!("ERGON_BENCH_ARM_ORDER must be `forward` or `reversed`, got {other:?}"),
        }
        for (id, mut arm) in arms {
            group.bench_function(id, |b| arm(b));
        }
        group.finish();
    }
}

fn assert_session_message_header_encode_parity() {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder as ErgoEncoder;
    use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder as ToolEncoder};

    let body = ErgoEncoder::BLOCK_LENGTH;
    let slot = HDR + body;

    let mut ergo = vec![0u8; slot];
    let _ = ErgoEncoder::wrap(&mut ergo, 0)
        .leadership_term_id(5)
        .cluster_session_id(42)
        .timestamp(0);
    let ergo_body = ErgoEncoder::BLOCK_LENGTH;

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
    let _ = ErgoEncoder::wrap(&mut ergo, 0)
        .leadership_term_id(5)
        .cluster_session_id(42);
    let ergo_body = ErgoEncoder::BLOCK_LENGTH;

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
    let mut g = parity_group(c, "cluster/encode/session_message_header");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    // `const`, not a captured `let`. Arms are boxed `dyn FnMut` (see `Pair`), so
    // a captured length is an opaque load and sbe-tool's `put_bytes_at` keeps a
    // bounds check per field that ergon's unchecked setters do not: unequal loops.
    const SLOT: usize = HDR + ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::BLOCK_LENGTH;
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * SLOT];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * SLOT;
                let term = black_box((i % 1000) as i64);
                let session = black_box(42_i64);
                let timestamp = black_box(0_i64);
                // SAFETY: each slice is `HDR + BLOCK_LENGTH` (`ENCODED_LENGTH`),
                // within the `BATCH_SIZE * SLOT` allocation, and disjoint across
                // iterations. `wrap` proves that extent and then panics; sbe-tool
                // `wrap` does not, so the panic check is unequal work. The parity
                // assert above already ran `wrap` on this shape. The slice length
                // stays visible so both arms can drop per-field checks; the values
                // are opaque so neither arm folds 42 and 0 into a constant vector
                // store.
                let view = unsafe { slot_slice(base, off, SLOT) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::wrap_unchecked(view, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session)
                .timestamp(timestamp);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * SLOT];
        b.iter(|| {
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * SLOT;
                let term = black_box((i % 1000) as i64);
                let session = black_box(42_i64);
                let timestamp = black_box(0_i64);
                // SAFETY: same extent proof as the ergo arm — `BATCH_SIZE * SLOT`
                // allocation, disjoint per iteration.
                let wb = WriteBuf::new(unsafe { slot_slice(base, off, SLOT) });
                let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, HDR);
                enc.leadership_term_id(term);
                enc.cluster_session_id(session);
                enc.timestamp(timestamp);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

/// Encode 10k SessionKeepAlive **bodies** (no MessageHeader write).
fn bench_encode_keep_alive_ergo(c: &mut Criterion) {
    assert_session_keep_alive_encode_parity();
    let mut g = parity_group(c, "cluster/encode/session_keep_alive");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    // `const`, not a captured `let`: see `bench_encode_msg_header_ergo`.
    const SLOT: usize = HDR + ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::BLOCK_LENGTH;
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * SLOT];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * SLOT;
                let term = black_box(5_i64);
                let session = black_box((i % 100) as i64);
                // SAFETY: each slice is `HDR + BLOCK_LENGTH` (`ENCODED_LENGTH`),
                // within the `BATCH_SIZE * SLOT` allocation, and disjoint across
                // iterations. `wrap` proves that extent and then panics; sbe-tool
                // `wrap` does not, so the panic check is unequal work. The parity
                // assert above already ran `wrap` on this shape. The slice length
                // stays visible so both arms can drop per-field checks; the values
                // are opaque so neither arm folds 5 into a constant store.
                let view = unsafe { slot_slice(base, off, SLOT) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::wrap_unchecked(view, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * SLOT];
        b.iter(|| {
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder};
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * SLOT;
                let term = black_box(5_i64);
                let session = black_box((i % 100) as i64);
                // SAFETY: same extent proof as the ergo arm — `BATCH_SIZE * SLOT`
                // allocation, disjoint per iteration.
                let wb = WriteBuf::new(unsafe { slot_slice(base, off, SLOT) });
                let mut enc = SessionKeepAliveEncoder::default().wrap(wb, HDR);
                enc.leadership_term_id(term);
                enc.cluster_session_id(session);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

/// A/A control for the encode harness.
///
/// Both arms run the identical keep-alive encode chain — same codec, same
/// slice helper, same alignment, same opaque values. The only thing that
/// differs between them is what differs between a real pair's arms minus the
/// codec, so their ratio is the harness's own null distribution: any deviation
/// from 1.0 is not codec work. Which harness or host effect produces that
/// deviation is not measured.
///
/// Read it beside every maintained pair. A real pair whose ratio falls inside
/// this band has **no established winner** — inconclusive, not an ergon loss
/// and not an ergon win. Ceilings are unchanged; this only says whether a
/// ceiling breach is resolved.
///
/// `fairness_policy_test::aa_control_arms_run_identical_work` keeps the two
/// bodies textually identical.
fn bench_aa_control_keep_alive(c: &mut Criterion) {
    assert_session_keep_alive_encode_parity();
    let mut g = parity_group(c, "cluster/control/aa_encode_keep_alive");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    const SLOT: usize = HDR + ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::BLOCK_LENGTH;
    g.bench_function("arm_a", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * SLOT];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * SLOT;
                let term = black_box(5_i64);
                let session = black_box((i % 100) as i64);
                // SAFETY: each slice is `HDR + BLOCK_LENGTH` (`ENCODED_LENGTH`),
                // within the `BATCH_SIZE * SLOT` allocation, and disjoint across
                // iterations. The parity assert above already ran `wrap` on this
                // shape. The slice length stays visible so both arms can drop
                // per-field checks; the values are opaque so neither arm folds 5
                // into a constant store.
                let view = unsafe { slot_slice(base, off, SLOT) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::wrap_unchecked(view, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session);
            }
            black_box(&buf);
        });
    });
    g.bench_function("arm_b", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * SLOT];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * SLOT;
                let term = black_box(5_i64);
                let session = black_box((i % 100) as i64);
                // SAFETY: each slice is `HDR + BLOCK_LENGTH` (`ENCODED_LENGTH`),
                // within the `BATCH_SIZE * SLOT` allocation, and disjoint across
                // iterations. The parity assert above already ran `wrap` on this
                // shape. The slice length stays visible so both arms can drop
                // per-field checks; the values are opaque so neither arm folds 5
                // into a constant store.
                let view = unsafe { slot_slice(base, off, SLOT) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder::wrap_unchecked(view, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

/// A/A control for the claim-shaped encode harness — the null for
/// `cluster_encode_claim_shaped_header_plus_app`, which is the largest of the
/// maintained encode loops and the one most exposed to code placement.
///
/// Both arms run the identical claim-shaped write: two disjoint `slot_slice`
/// views, three chained setters, a 32-byte payload copy. See
/// [`bench_aa_control_keep_alive`] for what the control means and
/// `fairness_policy_test::aa_control_arms_run_identical_work` for the lock.
fn bench_aa_control_claim_shaped(c: &mut Criterion) {
    let mut g = parity_group(c, "cluster/control/aa_encode_claim_shaped");
    const HDR_SLOT: usize = HDR + ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::BLOCK_LENGTH; // 32
    const TOTAL: usize = HDR_SLOT + CLAIM_APP_PAYLOAD.len();
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("arm_a", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * TOTAL];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * TOTAL;
                let term = black_box(5_i64);
                let session = black_box(42_i64);
                let timestamp = black_box(0_i64);
                // SAFETY: `HDR_SLOT` is exactly `HDR + BLOCK_LENGTH`. The header
                // and app regions are disjoint within one `TOTAL` slot, and slots
                // are disjoint across iterations. The values are opaque so
                // neither arm folds them into constant stores.
                let hdr = unsafe { slot_slice(base, off, HDR_SLOT) };
                let app = unsafe { slot_slice(base, off + HDR_SLOT, CLAIM_APP_PAYLOAD.len()) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::wrap_unchecked(hdr, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session)
                .timestamp(timestamp);
                app.copy_from_slice(&CLAIM_APP_PAYLOAD);
            }
            black_box(&buf);
        });
    });
    g.bench_function("arm_b", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * TOTAL];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * TOTAL;
                let term = black_box(5_i64);
                let session = black_box(42_i64);
                let timestamp = black_box(0_i64);
                // SAFETY: `HDR_SLOT` is exactly `HDR + BLOCK_LENGTH`. The header
                // and app regions are disjoint within one `TOTAL` slot, and slots
                // are disjoint across iterations. The values are opaque so
                // neither arm folds them into constant stores.
                let hdr = unsafe { slot_slice(base, off, HDR_SLOT) };
                let app = unsafe { slot_slice(base, off + HDR_SLOT, CLAIM_APP_PAYLOAD.len()) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::wrap_unchecked(hdr, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session)
                .timestamp(timestamp);
                app.copy_from_slice(&CLAIM_APP_PAYLOAD);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

/// Encode 10k SessionConnectRequest **bodies** (no MessageHeader write).
fn bench_encode_connect_request_ergo(c: &mut Criterion) {
    use ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestFixedFields;

    let mut g = parity_group(c, "cluster/encode/session_connect_request");
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
// Equal-work decode: both arms wrap at a proven extent and read the same
// fields. Neither parses or validates template_id / schema_id / version.
// Ergo wrap() is extent-only; sbe-tool wrap(body_offset, acting_*) matches
// that — MessageHeaderDecoder + identity checks would be extra work.

const MSG_HDR_FIXTURE: [u8; 32] = [
    0x18, 0x00, 0x01, 0x00, 0x6f, 0x00, 0x11, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0xd2, 0x02, 0x96, 0x49, 0x00, 0x00, 0x00, 0x00,
];

fn bench_decode_msg_header(c: &mut Criterion) {
    {
        use reference_sbe::{
            ReadBuf, message_header_codec::MessageHeaderDecoder,
            session_message_header_codec::SessionMessageHeaderDecoder,
        };
        let ergo =
            ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder::decode(&MSG_HDR_FIXTURE, 0).unwrap();
        let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(&MSG_HDR_FIXTURE), 0);
        let tool = SessionMessageHeaderDecoder::default().header(header, 0);
        assert_eq!(ergo.leadership_term_id(), tool.leadership_term_id());
        assert_eq!(ergo.cluster_session_id(), tool.cluster_session_id());
        assert_eq!(ergo.timestamp(), tool.timestamp());
    }
    let mut g = parity_group(c, "cluster/decode/session_message_header");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            for _ in 0..BATCH_SIZE {
                decode_arms::ergo_session_message_header(black_box(&MSG_HDR_FIXTURE[..]), |v| {
                    black_box(v);
                });
            }
        });
    });
    g.bench_function("sbe-tool", |b| {
        b.iter(|| {
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            for _ in 0..BATCH_SIZE {
                decode_arms::tool_session_message_header(black_box(&MSG_HDR_FIXTURE[..]), |v| {
                    black_box(v);
                });
            }
        });
    });
    g.finish();
}

// ── Decode: SessionEvent (fixed + var-data detail) ───────────────────────

const SESSION_EVENT_FIXTURE: [u8; 67] = [
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x11, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x73,
    0x6f, 0x6d, 0x65, 0x2d, 0x64, 0x65, 0x74, 0x61, 0x69, 0x6c,
];

fn bench_decode_session_event(c: &mut Criterion) {
    {
        use reference_sbe::{
            ReadBuf, message_header_codec::MessageHeaderDecoder, session_event_codec::SessionEventDecoder,
        };
        let ergo =
            ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::decode(&SESSION_EVENT_FIXTURE, 0).unwrap();
        let ergo_correlation_id = ergo.correlation_id();
        let ergo_cluster_session_id = ergo.cluster_session_id();
        let ergo_leadership_term_id = ergo.leadership_term_id();
        let ergo_leader_member_id = ergo.leader_member_id();
        let ergo_code = ergo.code() as u8;
        let ergo_detail = ergo.detail_slice().unwrap();

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
    let mut g = parity_group(c, "cluster/decode/session_event");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            for _ in 0..BATCH_SIZE {
                decode_arms::ergo_session_event(black_box(&SESSION_EVENT_FIXTURE[..]), |v| {
                    black_box(v);
                });
            }
        });
    });
    g.bench_function("sbe-tool", |b| {
        b.iter(|| {
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            for _ in 0..BATCH_SIZE {
                decode_arms::tool_session_event(black_box(&SESSION_EVENT_FIXTURE[..]), |v| {
                    black_box(v);
                });
            }
        });
    });
    g.finish();
}

// ── Decode: NewLeaderEvent (var-data endpoints) ──────────────────────────

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
        let ergo = ergo_aeron_cluster::cluster_codec_types::NewLeaderEventDecoder::decode(&fixture, 0).unwrap();
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
    let mut g = parity_group(c, "cluster/decode/new_leader_event");
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                decode_arms::ergo_new_leader_event(black_box(fixture.as_slice()), |v| {
                    black_box(v);
                });
            }
        });
    });
    g.bench_function("sbe-tool", |b| {
        b.iter(|| {
            for _ in 0..BATCH_SIZE {
                decode_arms::tool_new_leader_event(black_box(fixture.as_slice()), |v| {
                    black_box(v);
                });
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
    let mut g = parity_group(c, "cluster/encode/claim_shaped_header_plus_app");
    // `const`, not a captured `let`: see `bench_encode_msg_header_ergo`.
    const HDR_SLOT: usize = HDR + ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::BLOCK_LENGTH; // 32
    const TOTAL: usize = HDR_SLOT + CLAIM_APP_PAYLOAD.len();
    {
        use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder as ErgoEncoder;
        use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder as ToolEncoder};

        let mut ergo = vec![0u8; TOTAL];
        let _ = ErgoEncoder::wrap(&mut ergo[..HDR_SLOT], 0)
            .leadership_term_id(5)
            .cluster_session_id(42)
            .timestamp(0);
        let ergo_body = ErgoEncoder::BLOCK_LENGTH;
        ergo[HDR_SLOT..].copy_from_slice(&CLAIM_APP_PAYLOAD);

        let mut sbe_tool = vec![0u8; TOTAL];
        let mut tool = ToolEncoder::default().wrap(WriteBuf::new(&mut sbe_tool[..HDR_SLOT]), HDR);
        tool.leadership_term_id(5).cluster_session_id(42).timestamp(0);
        let tool_body = tool.encoded_length();
        sbe_tool[HDR_SLOT..].copy_from_slice(&CLAIM_APP_PAYLOAD);

        assert_eq!(ergo_body, tool_body, "claim body length");
        assert_eq!(ergo_body, ErgoEncoder::BLOCK_LENGTH);
        assert_body_parity(
            "claim-shaped write",
            &ergo[..HDR_SLOT],
            &sbe_tool[..HDR_SLOT],
            ergo_body,
        );
        assert_eq!(&ergo[HDR_SLOT..], &sbe_tool[HDR_SLOT..], "claim app payload");
    }
    g.throughput(Throughput::Elements(BATCH_SIZE as u64));
    g.bench_function("ergo-sbe", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * TOTAL];
        b.iter(|| {
            // Same 64-byte alignment as the sbe-tool arm. The no-LTO
            // ratio otherwise follows where the loop header lands.
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * TOTAL;
                let term = black_box(5_i64);
                let session = black_box(42_i64);
                let timestamp = black_box(0_i64);
                // SAFETY: `HDR_SLOT` is exactly `HDR + BLOCK_LENGTH`. The header
                // and app regions are disjoint within one `TOTAL` slot, and slots
                // are disjoint across iterations. `wrap` proves the header extent
                // and then panics; sbe-tool `wrap` does not, so the panic check is
                // unequal work. The parity assert above already ran `wrap` on this
                // shape. The slice length stays visible so both arms can drop
                // per-field checks; the values are opaque so neither arm folds them
                // into constant stores.
                let hdr = unsafe { slot_slice(base, off, HDR_SLOT) };
                let app = unsafe { slot_slice(base, off + HDR_SLOT, CLAIM_APP_PAYLOAD.len()) };
                let _ = unsafe {
                    ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder::wrap_unchecked(hdr, 0)
                }
                .leadership_term_id(term)
                .cluster_session_id(session)
                .timestamp(timestamp);
                app.copy_from_slice(&CLAIM_APP_PAYLOAD);
            }
            black_box(&buf);
        });
    });
    g.bench_function("sbe-tool", |b| {
        let mut buf = vec![0u8; BATCH_SIZE * TOTAL];
        b.iter(|| {
            unsafe {
                core::arch::asm!(".p2align 6", options(nomem, nostack, preserves_flags));
            }
            use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
            let base = buf.as_mut_ptr();
            for i in 0..BATCH_SIZE {
                let off = i * TOTAL;
                let term = black_box(5_i64);
                let session = black_box(42_i64);
                let timestamp = black_box(0_i64);
                // SAFETY: same extent and disjointness proof as the ergo arm.
                let hdr = unsafe { slot_slice(base, off, HDR_SLOT) };
                let app = unsafe { slot_slice(base, off + HDR_SLOT, CLAIM_APP_PAYLOAD.len()) };
                let wb = WriteBuf::new(hdr);
                let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, HDR);
                enc.leadership_term_id(term);
                enc.cluster_session_id(session);
                enc.timestamp(timestamp);
                app.copy_from_slice(&CLAIM_APP_PAYLOAD);
            }
            black_box(&buf);
        });
    });
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_millis(500))
        .sample_size(200);
    targets = bench_encode_msg_header_ergo,
        bench_encode_keep_alive_ergo,
        bench_aa_control_keep_alive,
        bench_aa_control_claim_shaped,
        bench_encode_connect_request_ergo,
        bench_decode_msg_header,
        bench_decode_session_event,
        bench_decode_new_leader,
        bench_claim_shaped_write
}
criterion_main!(benches);
