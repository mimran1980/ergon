//! The WebSocket receive path, fed record by record without a driver: what
//! each delivery case copies, `Interest::Skip`, fragments as slices of the
//! receive buffer and their fallback to a copy, and when text is checked for
//! UTF-8. Every platform.
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod support;
mod ws_rig;

use std::collections::VecDeque;

use ergon_io::ws::{self, OP_BINARY, OP_CONT, OP_PING, OP_TEXT};
use ergon_io::{
    ConnId, Fragments, Handler, Interest, MAX_FRAGMENTS, RxMeta, WsHandler, WsOptions, WsStats,
    ws_conn::MIN_BUF,
};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngSeed, TestRunner};
use support::{Lab, R, frame};
use ws_rig::{WsRig, fragmented};

/// One delivered message: `slices` is 0 for `on_message`, else the
/// fragment count `on_fragments` saw.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Got {
    op: u8,
    data: Vec<u8>,
    slices: usize,
}

/// Decides each message from `plan` (in order), then `default`.
struct App {
    plan: VecDeque<Interest>,
    default: Interest,
    got: Vec<Got>,
    /// What `on_message_start` saw: opcode, first payload, fin.
    starts: Vec<(u8, Vec<u8>, bool)>,
}

impl App {
    const fn new(default: Interest) -> Self {
        Self {
            plan: VecDeque::new(),
            default,
            got: Vec::new(),
            starts: Vec::new(),
        }
    }

    fn planned(plan: Vec<Interest>) -> Self {
        Self {
            plan: plan.into(),
            ..Self::new(Interest::Keep)
        }
    }
}

impl Handler for App {}

impl WsHandler for App {
    fn on_message(&mut self, _: ConnId, op: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.got.push(Got {
            op,
            data: payload.to_vec(),
            slices: 0,
        });
    }

    fn on_message_start(
        &mut self,
        _: ConnId,
        op: u8,
        first: &[u8],
        fin: bool,
        _: &RxMeta,
    ) -> Interest {
        self.starts.push((op, first.to_vec(), fin));
        self.plan.pop_front().unwrap_or(self.default)
    }

    fn on_fragments(&mut self, _: ConnId, op: u8, m: &Fragments<'_>, _: &RxMeta, _: u16) {
        let mut data = vec![0u8; m.len()];
        let copied = m.copy_to(&mut data);
        let joined: Vec<u8> = m.iter().flatten().copied().collect();
        assert_eq!(copied, Some(m.len()));
        assert_eq!(joined, data, "iter and copy_to disagree");
        assert_eq!(m.iter().len(), m.count());
        assert_eq!(m.contiguous().is_some(), m.count() == 1);
        self.got.push(Got {
            op,
            data,
            slices: m.count(),
        });
    }
}

fn opts() -> WsOptions {
    WsOptions {
        ping_interval_ms: 0,
        ..WsOptions::default()
    }
}

fn rig(lab: &Lab, o: &WsOptions) -> R<WsRig> {
    WsRig::new(lab, o)
}

fn text(len: usize) -> Vec<u8> {
    (0..len).map(|i| b'a' + (i % 26) as u8).collect()
}

fn binary(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

const fn stats(r: &WsRig) -> WsStats {
    r.conn.stats()
}

// (a) unfragmented, one record.
#[test]
fn an_unfragmented_message_is_one_slice_with_no_copy() -> R {
    let lab = Lab::new()?;
    for mode in [Interest::Keep, Interest::Fragments] {
        let mut r = rig(&lab, &opts())?;
        let mut h = App::new(mode);
        let msg = binary(300);
        assert_eq!(
            r.feed_records(&frame(OP_BINARY, true, &msg)?, &[], &mut h)?,
            1
        );
        assert_eq!(
            h.got,
            vec![Got {
                op: OP_BINARY,
                data: msg.clone(),
                slices: 0
            }]
        );
        assert_eq!(h.starts, vec![(OP_BINARY, msg, true)]);
        let s = stats(&r);
        assert_eq!(
            (s.messages, s.span_copy_bytes, s.frag_copy_bytes, s.skipped),
            (1, 0, 0, 0)
        );
    }
    Ok(())
}

// (b) one frame split across records.
#[test]
fn a_frame_split_across_records_moves_only_the_later_part() -> R {
    let lab = Lab::new()?;
    for one_read in [false, true] {
        let mut r = rig(&lab, &opts())?;
        let mut h = App::new(Interest::Fragments);
        let msg = binary(4000);
        let f = frame(OP_BINARY, true, &msg)?;
        let cut = 1500;
        if one_read {
            r.feed_one_read(&f, &[cut], &mut h)?;
        } else {
            r.feed_records(&f, &[cut], &mut h)?;
        }
        assert_eq!(
            h.got,
            vec![Got {
                op: OP_BINARY,
                data: msg,
                slices: 0
            }]
        );
        let s = stats(&r);
        assert_eq!(s.span_copy_bytes, (f.len() - cut) as u64);
        assert_eq!((s.frag_copy_bytes, s.frag_fallbacks), (0, 0));
    }
    Ok(())
}

// (c) fragmented in 2..N frames.
#[test]
fn a_fragmented_message_arrives_as_slices_of_the_buffer() -> R {
    let lab = Lab::new()?;
    for parts in 2..=MAX_FRAGMENTS {
        for layout in 0..3 {
            let mut r = rig(&lab, &opts())?;
            let mut h = App::new(Interest::Fragments);
            let msg = binary(97 * parts);
            let stream = fragmented(OP_BINARY, &msg, parts)?;
            // 0: one record; 1: one record per frame (aligned); 2: frames
            // cut mid-way across records.
            let cuts: Vec<usize> = match layout {
                0 => vec![],
                1 => frame_ends(&stream)?,
                _ => (1..parts).map(|i| i * stream.len() / parts + 3).collect(),
            };
            r.feed_records(&stream, &cuts, &mut h)?;
            assert_eq!(
                h.got,
                vec![Got {
                    op: OP_BINARY,
                    data: msg,
                    slices: parts
                }],
                "parts {parts} layout {layout}"
            );
            let s = stats(&r);
            assert_eq!((s.frag_copy_bytes, s.frag_fallbacks, s.messages), (0, 0, 1));
            if layout < 2 {
                // Records that end on a frame boundary open in place.
                assert_eq!(s.span_copy_bytes, 0, "parts {parts} layout {layout}");
            }
            assert_eq!(r.conn.buffered(), 0, "the held bytes are released");
        }
    }
    Ok(())
}

/// Offsets just past each frame of `stream`.
fn frame_ends(stream: &[u8]) -> R<Vec<usize>> {
    let mut at = 0;
    let mut out = Vec::new();
    while at < stream.len() {
        let h = ws::parse_header(&stream[at..])?.ok_or("short frame")?;
        at += h.frame_len();
        out.push(at);
    }
    Ok(out)
}

#[test]
fn keep_still_reassembles_a_fragmented_message_by_copy() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, &opts())?;
    let mut h = App::new(Interest::Keep);
    let msg = text(5000);
    r.feed_records(&fragmented(OP_TEXT, &msg, 4)?, &[], &mut h)?;
    assert_eq!(
        h.got,
        vec![Got {
            op: OP_TEXT,
            data: msg.clone(),
            slices: 0
        }]
    );
    assert_eq!(h.starts.len(), 1);
    assert!(!h.starts[0].2, "the first frame is not the last");
    let s = stats(&r);
    assert_eq!((s.frag_copy_bytes, s.frag_fallbacks), (msg.len() as u64, 0));
    Ok(())
}

/// A message in three fragments with a Ping between the first two.
fn with_ping(op: u8, msg: &[u8]) -> R<Vec<u8>> {
    let third = msg.len() / 3;
    let mut s = frame(op, false, &msg[..third])?;
    s.extend_from_slice(&frame(OP_PING, true, b"hb")?);
    s.extend_from_slice(&frame(OP_CONT, false, &msg[third..2 * third])?);
    s.extend_from_slice(&frame(OP_CONT, true, &msg[2 * third..])?);
    Ok(s)
}

// (d) fragmented with an interleaved Ping.
#[test]
fn a_ping_inside_a_held_or_skipped_message_is_answered() -> R {
    let lab = Lab::new()?;
    for mode in [Interest::Fragments, Interest::Skip, Interest::Keep] {
        for cut in [None, Some(40)] {
            let mut r = rig(&lab, &opts())?;
            let mut h = App::new(mode);
            let msg = binary(900);
            let stream = with_ping(OP_BINARY, &msg)?;
            r.feed_records(&stream, cut.as_slice(), &mut h)?;
            let s = stats(&r);
            assert_eq!((s.pings, s.auto_frames), (1, 1), "{mode:?}");
            assert!(!r.conn.queued_tx().is_empty(), "the Pong is queued");
            match mode {
                Interest::Skip => {
                    assert_eq!(h.got, []);
                    assert_eq!(s.skipped, 1);
                }
                Interest::Fragments => {
                    assert_eq!(
                        h.got,
                        vec![Got {
                            op: OP_BINARY,
                            data: msg,
                            slices: 3
                        }]
                    );
                    assert_eq!(s.frag_copy_bytes, 0);
                }
                Interest::Keep => {
                    assert_eq!(
                        h.got,
                        vec![Got {
                            op: OP_BINARY,
                            data: msg,
                            slices: 0
                        }]
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_close_inside_a_held_message_ends_it_undelivered() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, &opts())?;
    let mut h = App::new(Interest::Fragments);
    let mut stream = frame(OP_BINARY, false, b"first half")?;
    stream.extend_from_slice(&frame(ws::OP_CLOSE, true, &1000u16.to_be_bytes())?);
    stream.extend_from_slice(&frame(OP_CONT, true, b"never delivered")?);
    r.feed_records(&stream, &[], &mut h)?;
    assert_eq!(h.got, []);
    assert!(r.conn.is_closing());
    assert_eq!(stats(&r).auto_frames, 1, "the Close is echoed");
    assert_eq!(r.conn.buffered(), 0, "the held bytes are dropped");
    Ok(())
}

// (e) fragmented larger than the buffer, and more fragments than the list.
#[test]
fn a_message_the_buffer_cannot_hold_falls_back_to_one_copy() -> R {
    let lab = Lab::new()?;
    let o = WsOptions {
        buf_size: MIN_BUF,
        max_message: 1 << 20,
        ..opts()
    };
    let mut r = rig(&lab, &o)?;
    let mut h = App::new(Interest::Fragments);
    let msg = binary(300 * 1024);
    r.feed_records(&fragmented(OP_BINARY, &msg, 10)?, &[], &mut h)?;
    assert_eq!(
        h.got,
        vec![Got {
            op: OP_BINARY,
            data: msg.clone(),
            slices: 1
        }]
    );
    let s = stats(&r);
    assert_eq!((s.frag_fallbacks, s.frag_copy_bytes), (1, msg.len() as u64));

    // The next message is held again.
    let small = binary(2000);
    r.feed_records(&fragmented(OP_BINARY, &small, 3)?, &[], &mut h)?;
    assert_eq!(
        h.got[1],
        Got {
            op: OP_BINARY,
            data: small,
            slices: 3
        }
    );
    assert_eq!(stats(&r).frag_fallbacks, 1);
    Ok(())
}

#[test]
fn compaction_moves_a_held_message_intact() -> R {
    let lab = Lab::new()?;
    let o = WsOptions {
        buf_size: MIN_BUF,
        ..opts()
    };
    let mut r = rig(&lab, &o)?;
    let mut h = App::new(Interest::Fragments);
    // A 60 KiB message and the first fragment in one read: the hold starts
    // 60 KiB in. Then one read per fragment until the tail runs short.
    let lead = binary(60 * 1024);
    let msg = binary(8 * 9000);
    let frags = fragmented(OP_BINARY, &msg, 8)?;
    let ends = frame_ends(&frags)?;
    let mut first = frame(OP_BINARY, true, &lead)?;
    first.extend_from_slice(&frags[..ends[0]]);
    r.feed_one_read(&first, &[], &mut h)?;
    for w in ends.windows(2) {
        r.feed_one_read(&frags[w[0]..w[1]], &[], &mut h)?;
    }
    assert_eq!(
        h.got[1],
        Got {
            op: OP_BINARY,
            data: msg,
            slices: 8
        }
    );
    let s = stats(&r);
    assert!(s.compact_bytes > 0, "the buffer was compacted: {s:?}");
    assert_eq!((s.frag_copy_bytes, s.frag_fallbacks), (0, 0));
    Ok(())
}

#[test]
fn more_fragments_than_the_list_holds_fall_back_to_one_copy() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, &opts())?;
    let mut h = App::new(Interest::Fragments);
    let msg = binary(10 * (MAX_FRAGMENTS + 1));
    r.feed_records(
        &fragmented(OP_BINARY, &msg, MAX_FRAGMENTS + 1)?,
        &[],
        &mut h,
    )?;
    assert_eq!(
        h.got,
        vec![Got {
            op: OP_BINARY,
            data: msg.clone(),
            slices: 1
        }]
    );
    let s = stats(&r);
    assert_eq!((s.frag_fallbacks, s.frag_copy_bytes), (1, msg.len() as u64));
    Ok(())
}

#[test]
fn a_held_message_over_max_message_closes_the_connection() -> R {
    let lab = Lab::new()?;
    for mode in [Interest::Fragments, Interest::Skip, Interest::Keep] {
        let o = WsOptions {
            max_message: 1000,
            ..opts()
        };
        let mut r = rig(&lab, &o)?;
        let mut h = App::new(mode);
        let e = r.feed_records(&fragmented(OP_BINARY, &binary(1500), 3)?, &[], &mut h);
        let msg = e.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("message too large"), "{mode:?}: {msg}");
        assert_eq!(h.got, []);
    }
    Ok(())
}

// (f) skipped variants.
#[test]
fn a_skipped_message_is_neither_copied_nor_delivered() -> R {
    let lab = Lab::new()?;
    let big = WsOptions {
        buf_size: MIN_BUF,
        max_message: 1 << 20,
        ..opts()
    };
    let cases: Vec<(&str, WsOptions, Vec<u8>, Vec<usize>)> = vec![
        (
            "unfragmented",
            opts(),
            frame(OP_TEXT, true, &text(300))?,
            vec![],
        ),
        (
            "split",
            opts(),
            frame(OP_TEXT, true, &text(4000))?,
            vec![1000],
        ),
        (
            "fragmented",
            opts(),
            fragmented(OP_TEXT, &text(4000), 5)?,
            vec![],
        ),
        (
            "fragmented, split",
            opts(),
            fragmented(OP_TEXT, &text(4000), 5)?,
            vec![333, 2100],
        ),
        (
            "larger than the buffer",
            big,
            fragmented(OP_BINARY, &binary(300 * 1024), 10)?,
            vec![],
        ),
        (
            "more fragments than the list",
            opts(),
            fragmented(OP_BINARY, &binary(400), MAX_FRAGMENTS + 5)?,
            vec![],
        ),
    ];
    for (name, o, stream, cuts) in cases {
        let mut r = rig(&lab, &o)?;
        let mut h = App::planned(vec![Interest::Skip]);
        r.feed_records(&stream, &cuts, &mut h)?;
        // Then a kept message, delivered normally.
        let after = binary(64);
        r.feed_records(&frame(OP_BINARY, true, &after)?, &[], &mut h)?;
        assert_eq!(
            h.got,
            vec![Got {
                op: OP_BINARY,
                data: after,
                slices: 0
            }],
            "{name}"
        );
        let s = stats(&r);
        assert_eq!(
            (s.skipped, s.messages, s.frag_copy_bytes, s.frag_fallbacks),
            (1, 1, 0, 0),
            "{name}"
        );
        assert_eq!(h.starts.len(), 2, "{name}: one decision per message");
    }
    Ok(())
}

// (g) UTF-8: checked only when kept.
#[test]
fn invalid_utf8_closes_only_when_the_message_is_kept() -> R {
    let lab = Lab::new()?;
    let check = WsOptions {
        validate_utf8: true,
        ..opts()
    };
    let bad = b"price \xff\xfe".to_vec();
    for parts in [1, 3] {
        let stream = fragmented(OP_TEXT, &bad, parts)?;
        for mode in [Interest::Keep, Interest::Fragments] {
            let mut r = rig(&lab, &check)?;
            let mut h = App::new(mode);
            let e = r
                .feed_records(&stream, &[], &mut h)
                .err()
                .map(|e| e.to_string());
            assert!(
                e.unwrap_or_default().contains("not UTF-8"),
                "{parts} {mode:?}"
            );
            assert_eq!(h.got, []);
        }
        // Interest::Skip: never read, so it does not close the connection.
        let mut r = rig(&lab, &check)?;
        let mut h = App::planned(vec![Interest::Skip]);
        r.feed_records(&stream, &[], &mut h)?;
        assert_eq!(stats(&r).skipped, 1);
        // Checking off: delivered as is.
        let mut r = rig(&lab, &opts())?;
        let mut h = App::new(Interest::Fragments);
        r.feed_records(&stream, &[], &mut h)?;
        assert_eq!(h.got.len(), 1);
        assert_eq!(h.got[0].data, bad);
    }
    Ok(())
}

#[test]
fn a_character_split_between_fragments_is_valid_utf8() -> R {
    let lab = Lab::new()?;
    let check = WsOptions {
        validate_utf8: true,
        ..opts()
    };
    // "€" is three bytes, "😀" four: cut every way between fragments.
    let msg = "a€b😀c".as_bytes().to_vec();
    for cut1 in 1..msg.len() {
        for cut2 in cut1..msg.len() {
            let mut stream = frame(OP_TEXT, false, &msg[..cut1])?;
            stream.extend_from_slice(&frame(OP_CONT, false, &msg[cut1..cut2])?);
            stream.extend_from_slice(&frame(OP_CONT, true, &msg[cut2..])?);
            let mut r = rig(&lab, &check)?;
            let mut h = App::new(Interest::Fragments);
            r.feed_records(&stream, &[], &mut h)?;
            assert_eq!(
                h.got,
                vec![Got {
                    op: OP_TEXT,
                    data: msg.clone(),
                    slices: 3
                }],
                "{cut1} {cut2}"
            );
        }
    }
    // A truncated character at the very end is invalid.
    let mut stream = frame(OP_TEXT, false, b"ok")?;
    stream.extend_from_slice(&frame(OP_CONT, true, &"€".as_bytes()[..2])?);
    let mut r = rig(&lab, &check)?;
    let mut h = App::new(Interest::Fragments);
    assert!(r.feed_records(&stream, &[], &mut h).is_err());
    Ok(())
}

#[test]
fn a_new_message_inside_a_held_one_is_refused() -> R {
    let lab = Lab::new()?;
    for mode in [Interest::Fragments, Interest::Skip] {
        let mut r = rig(&lab, &opts())?;
        let mut h = App::new(mode);
        let mut stream = frame(OP_BINARY, false, b"one")?;
        stream.extend_from_slice(&frame(OP_BINARY, true, b"two")?);
        let e = r
            .feed_records(&stream, &[], &mut h)
            .err()
            .map(|e| e.to_string());
        assert!(
            e.unwrap_or_default().contains("inside a fragmented"),
            "{mode:?}"
        );
    }
    Ok(())
}

/// One generated message: payload, fragment count, decision, and whether a
/// Ping follows its first fragment.
#[derive(Debug, Clone)]
struct Msg {
    op: u8,
    data: Vec<u8>,
    parts: usize,
    interest: Interest,
    ping: bool,
}

fn msg_strategy() -> impl Strategy<Value = Msg> {
    (
        prop::bool::ANY,
        0usize..20_000,
        1usize..8,
        prop_oneof![
            Just(Interest::Keep),
            Just(Interest::Fragments),
            Just(Interest::Skip)
        ],
        prop::bool::weighted(0.2),
        any::<u8>(),
    )
        .prop_map(|(is_text, len, parts, interest, ping, seed)| Msg {
            op: if is_text { OP_TEXT } else { OP_BINARY },
            data: (0..len)
                .map(|i| b' ' + ((i + usize::from(seed)) % 90) as u8)
                .collect(),
            parts,
            interest,
            ping: ping && parts > 1,
        })
}

fn stream_of(msgs: &[Msg]) -> R<Vec<u8>> {
    let mut out = Vec::new();
    for m in msgs {
        let f = fragmented(m.op, &m.data, m.parts)?;
        if m.ping {
            let first = ws::parse_header(&f)?.ok_or("short")?.frame_len();
            out.extend_from_slice(&f[..first]);
            out.extend_from_slice(&frame(OP_PING, true, b"p")?);
            out.extend_from_slice(&f[first..]);
        } else {
            out.extend_from_slice(&f);
        }
    }
    Ok(out)
}

fn prop_config() -> Config {
    Config {
        cases: 64,
        rng_seed: RngSeed::Fixed(0x7773_5f7a_6572_6f63),
        failure_persistence: None,
        ..Config::default()
    }
}

/// Random fragmentation, record cuts and read grouping always deliver
/// exactly the kept messages, in order and intact, and skip the rest.
#[test]
fn any_fragmentation_and_record_split_delivers_the_original_messages() -> R {
    let lab = Lab::new()?;
    let strategy = (
        prop::collection::vec(msg_strategy(), 1..12),
        prop::collection::vec(any::<prop::sample::Index>(), 0..24),
        prop::collection::vec(any::<prop::sample::Index>(), 0..6),
        prop::bool::ANY,
    );
    let compacted = std::cell::Cell::new(0u32);
    let mut runner = TestRunner::new(prop_config());
    runner
        .run(&strategy, |(msgs, cut_at, read_at, small)| {
            let o = WsOptions {
                buf_size: if small { MIN_BUF } else { 1 << 20 },
                max_message: 1 << 20,
                validate_utf8: true,
                ..opts()
            };
            let stream = stream_of(&msgs).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let cuts: Vec<usize> = cut_at.iter().map(|i| i.index(stream.len())).collect();
            let mut r = rig(&lab, &o).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let mut h = App::planned(msgs.iter().map(|m| m.interest).collect());
            let records = r
                .records(&stream, &cuts)
                .map_err(|e| TestCaseError::fail(e.to_string()))?;
            // Group consecutive records into reads at random boundaries.
            let mut bounds: Vec<usize> =
                read_at.iter().map(|i| i.index(records.len() + 1)).collect();
            bounds.push(records.len());
            bounds.sort_unstable();
            let mut from = 0;
            for b in bounds {
                if b > from {
                    let read = records[from..b].concat();
                    r.feed(&read, &mut h)
                        .map_err(|e| TestCaseError::fail(e.to_string()))?;
                    from = b;
                }
            }
            let want: Vec<(u8, Vec<u8>)> = msgs
                .iter()
                .filter(|m| m.interest != Interest::Skip)
                .map(|m| (m.op, m.data.clone()))
                .collect();
            let got: Vec<(u8, Vec<u8>)> = h.got.iter().map(|g| (g.op, g.data.clone())).collect();
            prop_assert_eq!(got, want);
            // Every generated message fits the buffer and the slice list:
            // a fragmented message kept as fragments is never copied.
            for (g, m) in h
                .got
                .iter()
                .zip(msgs.iter().filter(|m| m.interest != Interest::Skip))
            {
                let expect = match (m.interest, m.parts) {
                    (_, 1) | (Interest::Keep, _) => 0,
                    _ => m.parts,
                };
                prop_assert_eq!(g.slices, expect);
            }
            let s = r.conn.stats();
            let skipped = msgs.iter().filter(|m| m.interest == Interest::Skip).count() as u64;
            prop_assert_eq!(s.skipped, skipped);
            prop_assert_eq!(s.frag_fallbacks, 0);
            prop_assert_eq!(s.messages, msgs.len() as u64 - skipped);
            prop_assert_eq!(s.pings, msgs.iter().filter(|m| m.ping).count() as u64);
            prop_assert_eq!(h.starts.len(), msgs.len());
            prop_assert_eq!(r.conn.buffered(), 0);
            compacted.set(compacted.get() + u32::from(s.compact_bytes > 0));
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    assert!(compacted.get() > 0, "no case compacted the buffer");
    Ok(())
}

/// The generator is not vacuous: a fixed case runs every decision.
#[test]
fn the_generator_produces_every_decision_and_fragmentation() -> R {
    let mut runner = TestRunner::new(prop_config());
    let mut seen = [false; 3];
    let mut fragmented_msgs = 0;
    for _ in 0..64 {
        let m = msg_strategy()
            .new_tree(&mut runner)
            .map_err(|e| e.to_string())?
            .current();
        seen[m.interest as usize] = true;
        fragmented_msgs += usize::from(m.parts > 1);
    }
    assert_eq!(seen, [true; 3]);
    assert!(fragmented_msgs > 10);
    Ok(())
}
