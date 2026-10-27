//! The WebSocket send path: the frames `WsConn` queues are byte-identical
//! to the reference encoder (`ws::encode_frame` into a zeroed buffer, the
//! path before 0.1.35), record by record, for every payload length from 1
//! to 16 KiB and for frames spread over several records; `ws::push_frame`
//! against the same reference; and (Linux) a fresh connection's first
//! sends take no page faults. Every platform but the last.
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod support;
mod ws_rig;

use ergon_io::WsOptions;
use ergon_io::ws::{self, OP_BINARY, OP_PING, OP_PONG, OP_TEXT};
use support::{Lab, R};
use ws_rig::WsRig;

/// Largest payload one record carries with its frame header.
const ONE_RECORD: usize = (1 << 14) - 8;

fn opts() -> WsOptions {
    WsOptions {
        ping_interval_ms: 0,
        ..WsOptions::default()
    }
}

fn payload(len: usize, seed: usize) -> Vec<u8> {
    (0..len)
        .map(|i| ((i * 31 + seed * 7) % 251) as u8)
        .collect()
}

/// The reference: a zeroed buffer, `encode_frame`, which copies and then
/// masks in place.
fn reference(op: u8, data: &[u8], mask: [u8; 4]) -> R<Vec<u8>> {
    let mut out = vec![0u8; ws::header_len(data.len(), true) + data.len()];
    let n = ws::encode_frame(op, data, Some(mask), &mut out)?;
    out.truncate(n);
    Ok(out)
}

/// How the reference path cut a frame of `len` payload bytes into
/// records: one record up to `ONE_RECORD`, else the header and as much as
/// fills 16 KiB, then 16 KiB per record.
fn reference_cuts(len: usize) -> Vec<usize> {
    let hl = ws::header_len(len, true);
    if len <= ONE_RECORD {
        return vec![hl + len];
    }
    let first = ((1 << 14) - hl).min(len);
    let mut cuts = vec![hl + first];
    let mut rest = len - first;
    while rest > 0 {
        let part = rest.min(1 << 14);
        cuts.push(part);
        rest -= part;
    }
    cuts
}

/// Queues `data` as `op`, reads the records back and compares them with
/// the reference encoder under the mask the connection chose.
fn check_one(r: &mut WsRig, op: u8, data: &[u8]) -> R {
    r.conn.send_frame(op, data)?;
    let records = r.sent_records()?;
    let lens: Vec<usize> = records.iter().map(Vec::len).collect();
    assert_eq!(
        lens,
        reference_cuts(data.len()),
        "{} B: record cuts",
        data.len()
    );
    let wire = records.concat();
    let h = ws::parse_header(&wire)?.ok_or("short frame")?;
    let mask = h.mask.ok_or("client frame not masked")?;
    let want = reference(op, data, mask)?;
    assert!(
        wire == want,
        "{} B op {op}: bytes differ from the reference encoder",
        data.len()
    );
    Ok(())
}

#[test]
fn every_length_up_to_one_record_matches_the_reference_encoder() -> R {
    let lab = Lab::new()?;
    let mut r = WsRig::new(&lab, &opts())?;
    for len in 1..=(1 << 14) {
        let op = if len % 2 == 0 { OP_TEXT } else { OP_BINARY };
        check_one(&mut r, op, &payload(len, len))?;
    }
    Ok(())
}

#[test]
fn frames_over_several_records_match_the_reference_encoder() -> R {
    let lab = Lab::new()?;
    let mut r = WsRig::new(&lab, &opts())?;
    let lens = [
        ONE_RECORD + 1,
        (1 << 14) - 4,
        1 << 14,
        (1 << 14) + 1,
        65_535,
        65_536,
        65_537,
        100_003,
        (1 << 20) + 5,
    ];
    for (i, len) in lens.into_iter().enumerate() {
        check_one(&mut r, OP_BINARY, &payload(len, i))?;
    }
    Ok(())
}

#[test]
fn control_frames_and_empty_payloads_match_the_reference_encoder() -> R {
    let lab = Lab::new()?;
    let mut r = WsRig::new(&lab, &opts())?;
    for len in 0..=ws::MAX_CONTROL {
        for op in [OP_PING, OP_PONG] {
            check_one(&mut r, op, &payload(len, 3))?;
        }
    }
    check_one(&mut r, OP_TEXT, b"")?;
    Ok(())
}

/// `push_frame` appends exactly what the reference writes, behind bytes
/// already in the vector, for every header form and every tail length.
#[test]
fn push_frame_matches_encode_frame() -> R {
    let mut lens: Vec<usize> = (0..300).collect();
    lens.extend([65_535, 65_536, 65_537, 70_001]);
    let mut out = Vec::new();
    for len in lens {
        let data = payload(len, len);
        let mask = (len as u32).wrapping_mul(0x9E37_79B9).to_le_bytes();
        out.clear();
        out.extend_from_slice(b"prefix");
        ws::push_frame(&mut out, OP_BINARY, &data, mask);
        assert_eq!(&out[..6], b"prefix");
        assert!(out[6..] == reference(OP_BINARY, &data, mask)?, "{len} B");
    }
    Ok(())
}

/// `push_masked` at any key offset equals masking the piece in place.
#[test]
fn push_masked_at_an_offset_matches_apply_mask_at() {
    let mask = [0x11, 0x22, 0x33, 0x44];
    for len in 0..70 {
        for offset in 0..8 {
            let data = payload(len, offset);
            let mut want = data.clone();
            ws::apply_mask_at(&mut want, mask, offset);
            let mut out = vec![9u8; 3];
            ws::push_masked(&mut out, &data, mask, offset);
            assert_eq!(&out[..3], &[9, 9, 9]);
            assert_eq!(&out[3..], &want[..], "{len} B at {offset}");
        }
    }
}

/// Minor faults of this thread so far (`/proc/thread-self/stat`, field 10).
#[cfg(target_os = "linux")]
fn minor_faults() -> R<u64> {
    let s = std::fs::read_to_string("/proc/thread-self/stat")?;
    let close = s.rfind(')').ok_or("no comm")?;
    let f: Vec<&str> = s[close + 1..].split_whitespace().collect();
    Ok(f.get(7).ok_or("short stat")?.parse()?)
}

/// The send queue is written once when the connection is made: queueing
/// 60 KB on a fresh connection takes no page faults (64 KiB of untouched
/// queue would take one per page).
#[cfg(target_os = "linux")]
#[test]
fn the_first_sends_take_no_page_faults() -> R {
    let lab = Lab::new()?;
    let mut r = WsRig::new(&lab, &opts())?;
    let data = payload(4000, 1);
    // Warm the code path (its pages fault too) on a 10-byte frame, which
    // touches only the queue's first page; then empty the queue.
    r.conn.send_binary(&data[..10])?;
    r.conn.clear_tx();
    let before = minor_faults()?;
    for _ in 0..15 {
        r.conn.send_binary(&data)?;
    }
    let after = minor_faults()?;
    assert!(r.conn.queued_tx().len() > 60_000);
    assert!(
        after - before <= 2,
        "{} minor faults queueing 60 KB on a fresh connection",
        after - before
    );
    Ok(())
}
