//! Both arms of every cluster decode pair must meet a bad frame the same way.
//!
//! The head-to-head gate compares ergon with sbe-tool on equal work. An arm
//! that skips a frame where the other panics, or panics where the other skips,
//! has dropped (or added) a check, so the ratio measures the harness. This
//! feeds every truncation of each fixture, plus corrupt var-data length
//! prefixes, to the arm bodies `cluster_codec_bench` times
//! (`benches/decode_arms`) and to the session-event Ir probes, and requires
//! the same outcome from both arms: a panic, a skip, or equal values.

use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};

// sbe-tool's generated re-exports, most unused here (the bench allows them crate-wide).
#[allow(unused_imports)]
#[path = "../benches/reference_sbe/mod.rs"]
mod reference_sbe;

#[path = "../benches/decode_arms/mod.rs"]
mod decode_arms;

#[path = "../benches/cluster_perf_probe.rs"]
mod probe;

const HDR: usize = 8;

#[derive(Debug, PartialEq, Eq)]
enum Outcome<T> {
    Panicked,
    Skipped,
    Decoded(T),
}

fn outcome<T>(arm: impl FnOnce() -> Option<T>) -> Outcome<T> {
    match catch_unwind(AssertUnwindSafe(arm)) {
        Err(_) => Outcome::Panicked,
        Ok(None) => Outcome::Skipped,
        Ok(Some(values)) => Outcome::Decoded(values),
    }
}

/// The fixture, every truncation of it, and, when it has var data whose length
/// prefix sits at `prefix_at`, that prefix rewritten to off-by-one, zero, the
/// schema maximum and past it.
fn frames(fixture: &[u8], prefix_at: Option<usize>) -> Vec<Vec<u8>> {
    let mut frames: Vec<Vec<u8>> = (0..=fixture.len()).map(|len| fixture[..len].to_vec()).collect();
    if let Some(at) = prefix_at {
        let data_len = u32::try_from(fixture.len() - at - 4).unwrap_or(u32::MAX);
        for len in [0, data_len - 1, data_len + 1, 1 << 30, (1 << 30) + 1, u32::MAX] {
            let mut frame = fixture.to_vec();
            frame[at..at + 4].copy_from_slice(&len.to_le_bytes());
            frames.push(frame);
        }
    }
    frames
}

/// Runs both arms on every frame and returns a line per frame they disagree on.
fn disagreements<T: PartialEq + std::fmt::Debug>(
    pair: &str,
    frames: &[Vec<u8>],
    ergo: impl Fn(&[u8]) -> Option<T>,
    tool: impl Fn(&[u8]) -> Option<T>,
) -> Vec<String> {
    frames
        .iter()
        .filter_map(|frame| {
            let (e, t) = (outcome(|| ergo(frame)), outcome(|| tool(frame)));
            (e != t).then(|| format!("{pair}, {}-byte frame: ergon {e:?}, sbe-tool {t:?}", frame.len()))
        })
        .collect()
}

fn session_event_fixture() -> Result<Vec<u8>, Box<dyn Error>> {
    use ergo_aeron_cluster::cluster_codec_types::{EventCode, SessionEventEncoder, SessionEventFixedFields};
    const DETAIL: &[u8] = b"some-detail";
    let mut buf = vec![0u8; SessionEventEncoder::compute_length_with_header(DETAIL.len())];
    let len = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&SessionEventFixedFields {
            cluster_session_id: 1,
            correlation_id: 100,
            leadership_term_id: 5,
            leader_member_id: 0,
            code: EventCode::OK,
            version: Some(1),
            leader_heartbeat_timeout_ns: Some(1_000_000_000),
        })
        .detail(DETAIL)?
        .encoded_length_with_header();
    buf.truncate(len);
    Ok(buf)
}

fn new_leader_event_fixture() -> Result<Vec<u8>, Box<dyn Error>> {
    use ergo_aeron_cluster::cluster_codec_types::{NewLeaderEventEncoder, NewLeaderEventFixedFields};
    const ENDPOINTS: &[u8] = b"0=localhost:9000,1=localhost:9100";
    let mut buf = vec![0u8; NewLeaderEventEncoder::compute_length_with_header(ENDPOINTS.len())];
    let len = NewLeaderEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&NewLeaderEventFixedFields {
            cluster_session_id: 2,
            leadership_term_id: 9,
            leader_member_id: 1,
        })
        .ingress_endpoints(ENDPOINTS)?
        .encoded_length_with_header();
    buf.truncate(len);
    Ok(buf)
}

#[test]
fn decode_arms_meet_truncated_and_corrupt_frames_the_same_way() -> Result<(), Box<dyn Error>> {
    use ergo_aeron_cluster::cluster_codec_types::{
        NewLeaderEventDecoder, SessionEventDecoder, SessionMessageHeaderEncoder,
    };

    let mut header = vec![0u8; HDR + SessionMessageHeaderEncoder::BLOCK_LENGTH];
    let _ = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut header, 0)
        .leadership_term_id(99)
        .cluster_session_id(42)
        .timestamp(1_234_567_890);
    let event = session_event_fixture()?;
    let event_prefix = Some(HDR + SessionEventDecoder::BLOCK_LENGTH);
    let leader = new_leader_event_fixture()?;
    let leader_prefix = Some(HDR + NewLeaderEventDecoder::BLOCK_LENGTH);

    // The arms' panics are expected outcomes here, not failures to report.
    std::panic::set_hook(Box::new(|_| {}));
    let mut found = disagreements(
        "session_message_header",
        &frames(&header, None),
        |f| decode_arms::ergo_session_message_header(f, |v| v),
        |f| decode_arms::tool_session_message_header(f, |v| v),
    );
    found.extend(disagreements(
        "session_event",
        &frames(&event, event_prefix),
        |f| {
            decode_arms::ergo_session_event(f, |(a, b, c, d, code, detail)| {
                (a, b, c, d, code as i32, detail.to_vec())
            })
        },
        |f| {
            decode_arms::tool_session_event(f, |(a, b, c, d, code, detail)| {
                (a, b, c, d, code as i32, detail.to_vec())
            })
        },
    ));
    found.extend(disagreements(
        "new_leader_event",
        &frames(&leader, leader_prefix),
        |f| decode_arms::ergo_new_leader_event(f, |(a, b, c, eps)| (a, b, c, eps.to_vec())),
        |f| decode_arms::tool_new_leader_event(f, |(a, b, c, eps)| (a, b, c, eps.to_vec())),
    ));
    found.extend(disagreements(
        "session_event Ir probe",
        &frames(&event, event_prefix),
        |f| Some(probe::ergo_probe_decode_session_event(f)),
        |f| Some(probe::tool_probe_decode_session_event(f)),
    ));
    drop(std::panic::take_hook());

    assert!(
        found.is_empty(),
        "decode arms disagree on bad frames, so one arm skips a check the other pays for:\n{}",
        found.join("\n")
    );
    Ok(())
}
