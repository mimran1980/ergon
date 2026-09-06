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
    0x2c, 0x00, 0x02, 0x00, 0x6f, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00,
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
    fn as_str(self) -> &'static str {
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

/// One pair today: `cluster_decode_session_event`, the pair carrying a
/// documented no-LTO-only wall-clock allowance (see `check-bench-gate.sh`).
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
];

// ─── Setup, performed once in main, never inside a probe ───────────────────

/// Untimed preflight both arms share: proves the two codecs decode the fixture
/// to identical values before either probe is trusted to measure anything.
fn assert_probe_correctness() {
    use ergo_aeron_cluster::cluster_codec_types::SessionEventDecoder as ErgoDecoder;
    use reference_sbe::{
        ReadBuf, message_header_codec::MessageHeaderDecoder, session_event_codec::SessionEventDecoder as ToolDecoder,
    };

    let ergo = ErgoDecoder::decode(&SESSION_EVENT_FIXTURE, 0).expect("fixture must decode");
    let header = MessageHeaderDecoder::default().wrap(ReadBuf::new(&SESSION_EVENT_FIXTURE), 0);
    let mut tool = ToolDecoder::default().header(header, 0);

    assert_eq!(ergo.correlation_id(), tool.correlation_id());
    assert_eq!(ergo.cluster_session_id(), tool.cluster_session_id());
    assert_eq!(ergo.leadership_term_id(), tool.leadership_term_id());
    assert_eq!(ergo.leader_member_id(), tool.leader_member_id());
    assert_eq!(ergo.code() as u8, tool.code() as u8);
    let coords = tool.detail_decoder();
    assert_eq!(ergo.detail_slice().unwrap(), tool.detail_slice(coords));
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

fn main() {
    validate_registry();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--list" => {
            print_manifest();
        }
        [flag, symbol] if flag == "--probe" => {
            // Setup and validation happen here, outside every collected region.
            assert_probe_correctness();
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
            assert_probe_correctness();
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
}
