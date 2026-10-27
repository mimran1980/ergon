//! `cluster-perf-probe` — stable-symbol wrappers for mechanism-level
//! measurement of cluster codec hot paths.
//!
//! Same rationale and driving mechanism as `sbe/benchmarks/src/bin/perf_probe.rs`
//! (see that file's header) applied to `ergo-aeron-cluster`: Criterion answers
//! "how long did it take on this machine, today"; this binary exists to answer
//! "did the call disappear" and "how many instructions did each arm actually
//! retire". Each probe is a named, `#[inline(never)]`, unmangled function that
//! performs exactly [`OPERATIONS`] logical operations and returns an observed
//! checksum.
//!
//! `scripts/run-cluster-instruction-probes.sh` drives it under raw Callgrind
//! with `--toggle-collect=<symbol>`, so setup and validation — which happen in
//! `main`, before any probe is entered — are never counted.
//!
//! Every probe is registered in [`PROBES`]. The registry is the manifest:
//! `--list` prints it, the driver script compares that against the checked-in
//! `cluster/probes.tsv`, and an unregistered probe name fails closed rather
//! than silently measuring nothing.
//!
//! Adding a probe: append a [`Probe`] entry under the relevant topic, add the
//! matching wrapper, and regenerate `cluster/probes.tsv` with `--list`.
//!
//! This is a sibling to the SBE lane, not a shared one: cluster codecs live in
//! a different package with a different fixture story
//! (`cluster/benches/reference_sbe/`, a plain module, not a separate crate),
//! matching this repo's existing split between `run-sbe-bench.sh` and the
//! `bench-cluster` wall-clock recipe.

#![allow(unsafe_code)]
#![allow(
    missing_docs,
    unused_variables,
    unused_imports,
    dead_code,
    unused_mut,
    unused_must_use
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

use std::hint::black_box;

/// sbe-tool 1.39.0 reference runtime — probe-private (forbid production imports).
mod reference_sbe;

/// Standard SBE message-header size (matches `cluster_codec_bench.rs`'s `HDR`).
const HDR: usize = 8;

/// Logical operations performed inside every probe. Fixed across probes so
/// normalised instruction counts are directly comparable between arms.
const OPERATIONS: usize = 10_000;

/// The exact fixture `cluster_codec_bench.rs` decodes for this pair — keep the
/// two in sync; this probe exists to give the same case a stable symbol.
const SESSION_EVENT_FIXTURE: [u8; 67] = [
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x11, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x73,
    0x6f, 0x6d, 0x65, 0x2d, 0x64, 0x65, 0x74, 0x61, 0x69, 0x6c,
];

/// Which codec a probe measures. Both arms of a pair must exist, or the
/// normalised comparison is meaningless.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    Ergon,
    SbeTool,
}

impl Arm {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ergon => "ergon",
            Self::SbeTool => "sbe-tool",
        }
    }
}

struct Probe {
    symbol: &'static str,
    arm: Arm,
    /// Maintained benchmark pair this probe corresponds to (matches the label
    /// `scripts/check-bench-gate.sh` uses for the cluster pairs table).
    pair: &'static str,
    /// Coarse grouping for selecting a related set. `--topic` selects on this.
    topic: &'static str,
    run: fn() -> u64,
}

/// Four maintained pairs plus one constructor diagnostic. `cluster_decode_session_event`
/// carries a documented no-LTO-only wall-clock allowance (see `check-bench-gate.sh`).
/// `cluster_encode_session_message_header` is the smallest maintained encode
/// workload — three fixed stores, amplified 10,000x per batch — so it is the
/// arm whose ratio sits closest to 1.00 and therefore the one that most needs
/// a mechanism check before any near-1.00 wall-clock result is read as a loss.
/// `cluster_encode_session_keep_alive_checked_wrap` is **not** a gate pair: it
/// reintroduces ergon's checked `wrap()` alone to price that constructor.
/// Append further cluster pairs here as they need the same mechanism check.
const PROBES: &[Probe] = &[
    Probe {
        symbol: "ergo_probe_decode_session_event",
        arm: Arm::Ergon,
        pair: "cluster_decode_session_event",
        topic: "decode",
        run: run_ergo_decode_session_event,
    },
    Probe {
        symbol: "tool_probe_decode_session_event",
        arm: Arm::SbeTool,
        pair: "cluster_decode_session_event",
        topic: "decode",
        run: run_tool_decode_session_event,
    },
    Probe {
        symbol: "ergo_probe_encode_session_message_header",
        arm: Arm::Ergon,
        pair: "cluster_encode_session_message_header",
        topic: "encode",
        run: run_ergo_encode_session_message_header,
    },
    Probe {
        symbol: "tool_probe_encode_session_message_header",
        arm: Arm::SbeTool,
        pair: "cluster_encode_session_message_header",
        topic: "encode",
        run: run_tool_encode_session_message_header,
    },
    Probe {
        symbol: "ergo_probe_encode_session_keep_alive",
        arm: Arm::Ergon,
        pair: "cluster_encode_session_keep_alive",
        topic: "encode",
        run: run_ergo_encode_session_keep_alive,
    },
    Probe {
        symbol: "tool_probe_encode_session_keep_alive",
        arm: Arm::SbeTool,
        pair: "cluster_encode_session_keep_alive",
        topic: "encode",
        run: run_tool_encode_session_keep_alive,
    },
    Probe {
        symbol: "ergo_probe_encode_claim_shaped",
        arm: Arm::Ergon,
        pair: "cluster_encode_claim_shaped_header_plus_app",
        topic: "encode",
        run: run_ergo_encode_claim_shaped,
    },
    Probe {
        symbol: "tool_probe_encode_claim_shaped",
        arm: Arm::SbeTool,
        pair: "cluster_encode_claim_shaped_header_plus_app",
        topic: "encode",
        run: run_tool_encode_claim_shaped,
    },
    Probe {
        symbol: "ergo_probe_encode_session_keep_alive_checked_wrap",
        arm: Arm::Ergon,
        pair: "cluster_encode_session_keep_alive_checked_wrap",
        topic: "encode",
        run: run_ergo_encode_session_keep_alive_checked_wrap,
    },
    Probe {
        symbol: "tool_probe_encode_session_keep_alive_checked_wrap",
        arm: Arm::SbeTool,
        pair: "cluster_encode_session_keep_alive_checked_wrap",
        topic: "encode",
        run: run_tool_encode_session_keep_alive_checked_wrap,
    },
];

// ─── Setup, performed once in main, never inside a probe ───────────────────

/// Untimed preflight both arms share: proves the two codecs decode the fixture
/// to identical values before either probe is trusted to measure anything.
fn assert_probe_correctness() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder as ErgoDecoder;
    use reference_sbe::{
        ReadBuf, message_header_codec::MessageHeaderDecoder, session_event_codec::SessionEventDecoder as ToolDecoder,
    };

    let ergo = ErgoDecoder::try_decode(&SESSION_EVENT_FIXTURE, 0)?;
    let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(&SESSION_EVENT_FIXTURE), 0);
    let mut tool = ToolDecoder::default().header(header, 0);

    assert_eq!(ergo.correlation_id(), tool.correlation_id());
    assert_eq!(ergo.cluster_session_id(), tool.cluster_session_id());
    assert_eq!(ergo.leadership_term_id(), tool.leadership_term_id());
    assert_eq!(ergo.leader_member_id(), tool.leader_member_id());
    assert_eq!(ergo.code() as u8, tool.code() as u8);
    let coords = tool.detail_decoder();
    assert_eq!(ergo.detail_slice()?, tool.detail_slice(coords));
    assert_encode_session_message_header_parity()?;
    assert_encode_session_keep_alive_parity()?;
    assert_encode_claim_shaped_parity()?;
    Ok(())
}

/// Untimed preflight for the encode pair: both arms must write byte-identical
/// bodies for the same three field values. Equal bytes is the value-level
/// "equal work" claim; the instruction probe then shows whether the code that
/// produced them retired the same instructions.
fn assert_encode_session_message_header_parity() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder as ErgoEncoder;
    use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder as ToolEncoder};

    let term = 5_i64;
    let session = 42_i64;
    let timestamp = 7_i64;
    let slot = SESSION_MESSAGE_HEADER_SLOT;

    let mut ergo = vec![0u8; slot];
    // SAFETY: `slot` is exactly `HDR + BLOCK_LENGTH`, which is `ENCODED_LENGTH`.
    let _ = unsafe { ErgoEncoder::wrap_unchecked(&mut ergo, 0) }
        .leadership_term_id(term)
        .cluster_session_id(session)
        .timestamp(timestamp);

    let mut tool = vec![0u8; slot];
    let wb = WriteBuf::new(&mut tool);
    let mut enc = ToolEncoder::default().wrap(wb, HDR);
    enc.leadership_term_id(term);
    enc.cluster_session_id(session);
    enc.timestamp(timestamp);

    assert_eq!(
        &ergo[..HDR],
        &[0u8; HDR],
        "encode probe: ergon header region must stay zero in body-only mode"
    );
    assert_eq!(
        &tool[..HDR],
        &[0u8; HDR],
        "encode probe: sbe-tool header region must stay zero without .header(0)"
    );
    assert_eq!(
        &ergo[HDR..],
        &tool[HDR..],
        "encode probe: session message header bodies differ"
    );
    Ok(())
}

/// Untimed preflight for the keep-alive pair: both arms must write
/// byte-identical bodies for the same two field values. Equal bytes is the
/// value-level "equal work" claim; the instruction probe then shows whether the
/// code that produced them retired the same instructions.
fn assert_encode_session_keep_alive_parity() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder as ErgoEncoder;
    use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder as ToolEncoder};

    let term = 5_i64;
    let session = 42_i64;
    let slot = SESSION_KEEP_ALIVE_SLOT;

    let mut ergo = vec![0u8; slot];
    // SAFETY: `slot` is exactly `HDR + BLOCK_LENGTH`, which is `ENCODED_LENGTH`.
    let _ = unsafe { ErgoEncoder::wrap_unchecked(&mut ergo, 0) }
        .leadership_term_id(term)
        .cluster_session_id(session);

    let mut tool = vec![0u8; slot];
    let wb = WriteBuf::new(&mut tool);
    let mut enc = ToolEncoder::default().wrap(wb, HDR);
    enc.leadership_term_id(term);
    enc.cluster_session_id(session);

    assert_eq!(
        &ergo[..HDR],
        &[0u8; HDR],
        "keep-alive probe: ergon header region must stay zero in body-only mode"
    );
    assert_eq!(
        &tool[..HDR],
        &[0u8; HDR],
        "keep-alive probe: sbe-tool header region must stay zero without .header(0)"
    );
    assert_eq!(
        &ergo[HDR..],
        &tool[HDR..],
        "keep-alive probe: session keep-alive bodies differ"
    );
    Ok(())
}

/// Untimed preflight for the claim-shaped pair: both arms must write
/// byte-identical header bodies and byte-identical app payloads for the same
/// three field values. Equal bytes is the value-level "equal work" claim; the
/// instruction probe then shows whether the code that produced them retired
/// the same instructions.
fn assert_encode_claim_shaped_parity() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder as ErgoEncoder;
    use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder as ToolEncoder};

    let term = 5_i64;
    let session = 42_i64;
    let timestamp = 0_i64;
    let hdr_slot = SESSION_MESSAGE_HEADER_SLOT;
    let total = SESSION_CLAIM_SHAPED_SLOT;

    let mut ergo = vec![0u8; total];
    {
        let slot = &mut ergo[..];
        // SAFETY: `hdr_slot` is `HDR + BLOCK_LENGTH`; `total` is that plus 32.
        let _ = unsafe { ErgoEncoder::wrap_unchecked(&mut slot[..hdr_slot], 0) }
            .leadership_term_id(term)
            .cluster_session_id(session)
            .timestamp(timestamp);
        slot[hdr_slot..].copy_from_slice(&CLAIM_APP_PAYLOAD);
    }

    let mut tool = vec![0u8; total];
    {
        let slot = &mut tool[..];
        let wb = WriteBuf::new(&mut slot[..hdr_slot]);
        let mut enc = ToolEncoder::default().wrap(wb, HDR);
        enc.leadership_term_id(term);
        enc.cluster_session_id(session);
        enc.timestamp(timestamp);
        slot[hdr_slot..].copy_from_slice(&CLAIM_APP_PAYLOAD);
    }

    assert_eq!(
        &ergo[..HDR],
        &[0u8; HDR],
        "claim-shaped probe: ergon header region must stay zero in body-only mode"
    );
    assert_eq!(
        &tool[..HDR],
        &[0u8; HDR],
        "claim-shaped probe: sbe-tool header region must stay zero without .header(0)"
    );
    assert_eq!(
        &ergo[HDR..hdr_slot],
        &tool[HDR..hdr_slot],
        "claim-shaped probe: session message header bodies differ"
    );
    assert_eq!(
        &ergo[hdr_slot..],
        &tool[hdr_slot..],
        "claim-shaped probe: app payloads differ"
    );
    Ok(())
}

// ─── Probes: SessionEvent decode ────────────────────────────────────────────
//
// Both bodies mirror `bench_decode_session_event`'s timed closures in
// `cluster_codec_bench.rs` exactly — same accessors, same order, same
// equal-work var-data bounds check — so the mechanism this measures is the
// mechanism the wall-clock gate measures.

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_decode_session_event(buf: &[u8]) -> u64 {
    use ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder;
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let dec = SessionEventDecoder::wrap(
            black_box(buf),
            0,
            SessionEventDecoder::BLOCK_LENGTH,
            SessionEventDecoder::SCHEMA_VERSION,
        );
        let cid = dec.correlation_id();
        let csid = dec.cluster_session_id();
        let ltid = dec.leadership_term_id();
        let lmid = dec.leader_member_id();
        let code = dec.code();
        // Slice only — matches sbe-tool `detail_slice`. Fail-closed skip
        // matches the bench arm's `continue` on a too-short buffer.
        let detail_len = match dec.detail_slice() {
            Ok(detail) => detail.len(),
            Err(_) => 0,
        };
        checksum = checksum
            .wrapping_add(cid as u64)
            .wrapping_add(csid as u64)
            .wrapping_add(ltid as u64)
            .wrapping_add(u64::from(lmid as u32))
            .wrapping_add(u64::from(code as u8))
            .wrapping_add(detail_len as u64);
    }
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_decode_session_event(buf: &[u8]) -> u64 {
    use reference_sbe::{ReadBuf, session_event_codec::SessionEventDecoder};
    let mut checksum = 0_u64;
    for _ in 0..OPERATIONS {
        let buf = black_box(buf);
        let block = ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::BLOCK_LENGTH;
        // Same extent check ergon `wrap()` performs — equal work, both arms.
        if buf.len() < HDR + block {
            continue;
        }
        let mut dec = SessionEventDecoder::default().wrap(
            ReadBuf::new(buf),
            HDR,
            block as u16,
            ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder::SCHEMA_VERSION,
        );
        let cid = dec.correlation_id();
        let csid = dec.cluster_session_id();
        let ltid = dec.leadership_term_id();
        let lmid = dec.leader_member_id();
        let code = dec.code();
        let coords = dec.detail_decoder();
        let (off, len) = coords;
        let detail_len = if len > 1_073_741_824 || off.saturating_add(len) > buf.len() {
            0
        } else {
            dec.detail_slice(coords).len()
        };
        checksum = checksum
            .wrapping_add(cid as u64)
            .wrapping_add(csid as u64)
            .wrapping_add(ltid as u64)
            .wrapping_add(u64::from(lmid as u32))
            .wrapping_add(u64::from(code as u8))
            .wrapping_add(detail_len as u64);
    }
    black_box(checksum)
}

fn run_ergo_decode_session_event() -> u64 {
    ergo_probe_decode_session_event(&SESSION_EVENT_FIXTURE)
}

fn run_tool_decode_session_event() -> u64 {
    tool_probe_decode_session_event(&SESSION_EVENT_FIXTURE)
}

// ─── Probes: SessionMessageHeader encode ─────────────────────────────────────
//
// Both bodies mirror `bench_encode_msg_header_ergo`'s timed closures in
// `cluster_codec_bench.rs` exactly — same wrap, same three setters, same
// per-slot stride, no MessageHeader write — so the mechanism this measures is
// the mechanism the wall-clock gate measures. The buffer is supplied by the
// `run_*` wrapper and is never allocated inside a probe.

/// Bytes each arm writes per encoded body: `HDR` reserved + `BLOCK_LENGTH`.
const SESSION_MESSAGE_HEADER_SLOT: usize = HDR + 24;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_session_message_header(buf: &mut [u8]) -> u64 {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder;
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_MESSAGE_HEADER_SLOT;
        if off + SESSION_MESSAGE_HEADER_SLOT > buf.len() {
            continue;
        }
        let term = black_box((i % 1000) as i64);
        let session = black_box(42_i64);
        let timestamp = black_box(0_i64);
        // SAFETY: the extent check above proves `HDR + BLOCK_LENGTH` bytes
        // remain, matching `wrap_unchecked`'s contract and the bench arm.
        let _ =
            unsafe { SessionMessageHeaderEncoder::wrap_unchecked(&mut buf[off..off + SESSION_MESSAGE_HEADER_SLOT], 0) }
                .leadership_term_id(term)
                .cluster_session_id(session)
                .timestamp(timestamp);
    }
    // Observe the written bytes so the stores cannot be dead-code eliminated.
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_encode_session_message_header(buf: &mut [u8]) -> u64 {
    use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_MESSAGE_HEADER_SLOT;
        if off + SESSION_MESSAGE_HEADER_SLOT > buf.len() {
            continue;
        }
        let term = black_box((i % 1000) as i64);
        let session = black_box(42_i64);
        let timestamp = black_box(0_i64);
        let wb = WriteBuf::new(&mut buf[off..off + SESSION_MESSAGE_HEADER_SLOT]);
        let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, HDR);
        enc.leadership_term_id(term);
        enc.cluster_session_id(session);
        enc.timestamp(timestamp);
    }
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

fn run_ergo_encode_session_message_header() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_MESSAGE_HEADER_SLOT];
    ergo_probe_encode_session_message_header(&mut buf)
}

fn run_tool_encode_session_message_header() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_MESSAGE_HEADER_SLOT];
    tool_probe_encode_session_message_header(&mut buf)
}

// ─── Probes: SessionKeepAlive encode ────────────────────────────────────────
//
// Both bodies of the maintained pair mirror `bench_encode_keep_alive_ergo`'s
// timed closures in `cluster_codec_bench.rs` exactly — same `wrap_unchecked`,
// same two setters, same per-slot stride, opaque field values, slice length
// visible — so the mechanism this measures is the mechanism the wall-clock gate
// measures. The buffer is supplied by the `run_*` wrapper and is never
// allocated inside a probe.

/// Bytes each arm writes per encoded body: `HDR` reserved + `BLOCK_LENGTH` (16).
const SESSION_KEEP_ALIVE_SLOT: usize = HDR + 16;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_session_keep_alive(buf: &mut [u8]) -> u64 {
    use ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder;
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_KEEP_ALIVE_SLOT;
        if off + SESSION_KEEP_ALIVE_SLOT > buf.len() {
            continue;
        }
        let term = black_box(5_i64);
        let session = black_box((i % 100) as i64);
        // SAFETY: the extent check above proves `HDR + BLOCK_LENGTH` bytes
        // remain, matching `wrap_unchecked`'s contract and the bench arm.
        let _ = unsafe { SessionKeepAliveEncoder::wrap_unchecked(&mut buf[off..off + SESSION_KEEP_ALIVE_SLOT], 0) }
            .leadership_term_id(term)
            .cluster_session_id(session);
    }
    // Observe the written bytes so the stores cannot be dead-code eliminated.
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_encode_session_keep_alive(buf: &mut [u8]) -> u64 {
    use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder};
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_KEEP_ALIVE_SLOT;
        if off + SESSION_KEEP_ALIVE_SLOT > buf.len() {
            continue;
        }
        let term = black_box(5_i64);
        let session = black_box((i % 100) as i64);
        let wb = WriteBuf::new(&mut buf[off..off + SESSION_KEEP_ALIVE_SLOT]);
        let mut enc = SessionKeepAliveEncoder::default().wrap(wb, HDR);
        enc.leadership_term_id(term);
        enc.cluster_session_id(session);
    }
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

fn run_ergo_encode_session_keep_alive() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_KEEP_ALIVE_SLOT];
    ergo_probe_encode_session_keep_alive(&mut buf)
}

fn run_tool_encode_session_keep_alive() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_KEEP_ALIVE_SLOT];
    tool_probe_encode_session_keep_alive(&mut buf)
}

// ─── Probes: claim-shaped write (SessionMessageHeader body + 32-byte app) ──
//
// Both bodies of the maintained pair mirror `bench_claim_shaped_write`'s timed
// closures in `cluster_codec_bench.rs` exactly — same `wrap_unchecked` on a
// subslice, same three setters, same 32-byte `copy_from_slice`, same 64-byte
// per-slot stride, opaque field values, slice length visible — so the
// mechanism this measures is the mechanism the wall-clock gate measures. The
// buffer is supplied by the `run_*` wrapper and is never allocated inside a
// probe.

/// Fixed 32-byte app payload copied after the header body slot, as in the bench.
const CLAIM_APP_PAYLOAD: [u8; 32] = [0xABu8; 32];
/// Bytes each arm writes per iteration: the header slot plus the app payload.
const SESSION_CLAIM_SHAPED_SLOT: usize = SESSION_MESSAGE_HEADER_SLOT + CLAIM_APP_PAYLOAD.len();

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_claim_shaped(buf: &mut [u8]) -> u64 {
    use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderEncoder;
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_CLAIM_SHAPED_SLOT;
        if off + SESSION_CLAIM_SHAPED_SLOT > buf.len() {
            continue;
        }
        let slot = &mut buf[off..off + SESSION_CLAIM_SHAPED_SLOT];
        let term = black_box(5_i64);
        let session = black_box(42_i64);
        let timestamp = black_box(0_i64);
        // SAFETY: `SESSION_MESSAGE_HEADER_SLOT` is `HDR + BLOCK_LENGTH`; the
        // extent check above proves `SESSION_CLAIM_SHAPED_SLOT` bytes remain,
        // matching `wrap_unchecked`'s contract and the bench arm.
        let _ = unsafe { SessionMessageHeaderEncoder::wrap_unchecked(&mut slot[..SESSION_MESSAGE_HEADER_SLOT], 0) }
            .leadership_term_id(term)
            .cluster_session_id(session)
            .timestamp(timestamp);
        slot[SESSION_MESSAGE_HEADER_SLOT..].copy_from_slice(&CLAIM_APP_PAYLOAD);
    }
    // Observe the written bytes so the stores cannot be dead-code eliminated.
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_encode_claim_shaped(buf: &mut [u8]) -> u64 {
    use reference_sbe::{WriteBuf, session_message_header_codec::SessionMessageHeaderEncoder};
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_CLAIM_SHAPED_SLOT;
        if off + SESSION_CLAIM_SHAPED_SLOT > buf.len() {
            continue;
        }
        let slot = &mut buf[off..off + SESSION_CLAIM_SHAPED_SLOT];
        let term = black_box(5_i64);
        let session = black_box(42_i64);
        let timestamp = black_box(0_i64);
        let wb = WriteBuf::new(&mut slot[..SESSION_MESSAGE_HEADER_SLOT]);
        let mut enc = SessionMessageHeaderEncoder::default().wrap(wb, HDR);
        enc.leadership_term_id(term);
        enc.cluster_session_id(session);
        enc.timestamp(timestamp);
        slot[SESSION_MESSAGE_HEADER_SLOT..].copy_from_slice(&CLAIM_APP_PAYLOAD);
    }
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

fn run_ergo_encode_claim_shaped() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_CLAIM_SHAPED_SLOT];
    ergo_probe_encode_claim_shaped(&mut buf)
}

fn run_tool_encode_claim_shaped() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_CLAIM_SHAPED_SLOT];
    tool_probe_encode_claim_shaped(&mut buf)
}

// ─── Probes: SessionKeepAlive constructor diagnostic ────────────────────────
//
// Not a maintained pair and not a gate pair. This replays the **pre-fix**
// keep-alive bench closures verbatim — checked `wrap()` on the ergon arm,
// opaque slice length on both — to price what that bench actually measured.
// The maintained pair above is the same workload with the constructor and the
// opacity controlled; the difference between the two ergon arms is the
// formatted panic payload that `wrap()` has to be able to build.
//
// With the slice length **visible** (the maintained pair's choice) ergon's
// `24 > buf.len().saturating_sub(msg_offset)` folds to `24 > 24`, the branch
// disappears, and checked `wrap` and `wrap_unchecked` compile the same. The
// cost only exists when the length is opaque — which is why the maintained
// benches keep it visible on both arms.

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ergo_probe_encode_session_keep_alive_checked_wrap(buf: &mut [u8]) -> u64 {
    use ergo_aeron_cluster::cluster_codec_types::SessionKeepAliveEncoder;
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_KEEP_ALIVE_SLOT;
        if off + SESSION_KEEP_ALIVE_SLOT > buf.len() {
            continue;
        }
        let term = black_box(5_i64);
        let session = black_box((i % 100) as i64);
        // Checked `wrap`, and the slice length kept opaque: the constructor's
        // `24 > buf.len()` proof cannot fold away. This is the mechanism the
        // pre-fix keep-alive bench arm measured.
        let _ = SessionKeepAliveEncoder::wrap(black_box(&mut buf[off..off + SESSION_KEEP_ALIVE_SLOT]), 0)
            .leadership_term_id(term)
            .cluster_session_id(session);
    }
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tool_probe_encode_session_keep_alive_checked_wrap(buf: &mut [u8]) -> u64 {
    // Mirrors the pre-fix bench's tool arm (not a forwarder of the maintained
    // arm): the instruction comparison needs both arms' full bodies in their
    // own symbol, and this arm must share the ergo arm's opaque slice length.
    use reference_sbe::{WriteBuf, session_keep_alive_codec::SessionKeepAliveEncoder};
    let mut checksum = 0_u64;
    for i in 0..OPERATIONS {
        let off = i * SESSION_KEEP_ALIVE_SLOT;
        if off + SESSION_KEEP_ALIVE_SLOT > buf.len() {
            continue;
        }
        let term = black_box(5_i64);
        let session = black_box((i % 100) as i64);
        // Slice length opaque, matching the ergo arm and the pre-fix bench:
        // sbe-tool's `put_bytes_at` slice-index checks survive too. Both arms
        // keep their own bounds checks; only ergon's carries a formatted panic.
        let wb = WriteBuf::new(black_box(&mut buf[off..off + SESSION_KEEP_ALIVE_SLOT]));
        let mut enc = SessionKeepAliveEncoder::default().wrap(wb, HDR);
        enc.leadership_term_id(term);
        enc.cluster_session_id(session);
    }
    let mut sum = 0_u64;
    for chunk in buf.chunks_exact(8) {
        sum = sum.wrapping_add(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
    }
    checksum = checksum.wrapping_add(sum);
    black_box(checksum)
}

fn run_ergo_encode_session_keep_alive_checked_wrap() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_KEEP_ALIVE_SLOT];
    ergo_probe_encode_session_keep_alive_checked_wrap(&mut buf)
}

fn run_tool_encode_session_keep_alive_checked_wrap() -> u64 {
    let mut buf = vec![0u8; OPERATIONS * SESSION_KEEP_ALIVE_SLOT];
    tool_probe_encode_session_keep_alive_checked_wrap(&mut buf)
}

// ─── CLI ────────────────────────────────────────────────────────────────────

fn print_manifest() {
    println!("symbol\tarm\tpair\ttopic\toperations");
    for probe in PROBES {
        println!(
            "{}\t{}\t{}\t{}\t{OPERATIONS}",
            probe.symbol,
            probe.arm.as_str(),
            probe.pair,
            probe.topic
        );
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: cluster-perf-probe --list | --probe SYMBOL | --topic NAME\n\
         \n\
         --list          print the registered probe manifest\n\
         --probe SYMBOL  run one registered probe and print its checksum\n\
         --topic NAME    run every probe in a topic"
    );
    std::process::exit(2)
}

/// Reject a registry that cannot support a fair comparison, before any
/// measurement is taken.
fn validate_registry() {
    let mut seen: Vec<&str> = Vec::new();
    for probe in PROBES {
        assert!(
            !seen.contains(&probe.symbol),
            "duplicate probe symbol {}: the driver could not tell the two apart",
            probe.symbol
        );
        seen.push(probe.symbol);
    }
    for probe in PROBES {
        let counterpart = PROBES
            .iter()
            .filter(|other| other.pair == probe.pair)
            .filter(|other| other.arm != probe.arm)
            .count();
        assert!(
            counterpart == 1,
            "probe {} has no single opposing arm for pair {} — a one-sided \
             probe cannot support a comparison",
            probe.symbol,
            probe.pair
        );
    }
}

fn run(probe: &Probe) {
    let checksum = (probe.run)();
    println!(
        "probe={} arm={} pair={} topic={} operations={OPERATIONS} checksum={checksum}",
        probe.symbol,
        probe.arm.as_str(),
        probe.pair,
        probe.topic
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    validate_registry();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--list" => {
            print_manifest();
        }
        [flag, symbol] if flag == "--probe" => {
            // Setup and validation happen here, outside every collected region.
            assert_probe_correctness()?;
            let Some(probe) = PROBES.iter().find(|p| p.symbol == symbol) else {
                eprintln!(
                    "unknown probe {symbol:?}; registered probes:\n{}",
                    PROBES
                        .iter()
                        .map(|p| format!("  {}", p.symbol))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                std::process::exit(2);
            };
            run(probe);
        }
        [flag, topic] if flag == "--topic" => {
            assert_probe_correctness()?;
            let selected: Vec<&Probe> = PROBES.iter().filter(|p| p.topic == topic).collect();
            if selected.is_empty() {
                eprintln!("no probes registered for topic {topic:?}");
                std::process::exit(2);
            }
            for probe in selected {
                run(probe);
            }
        }
        _ => usage(),
    }
    Ok(())
}
