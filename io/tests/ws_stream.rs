//! Text checks (`TextCheck`: none, UTF-8, trusted ASCII, checked ASCII with
//! its non-ASCII policy)
//! and streamed fragments (`Interest::Stream`), fed record by record without
//! a driver: what each mode delivers and flags in `RxMeta::text`, that a
//! streamed fragment is handed over as it arrives with nothing held or
//! copied, `Flow::SkipRest`, incremental UTF-8 across fragments, and the
//! defaults that keep a message a handler did not take. Every platform.
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod support;
mod ws_rig;

use ergon_io::ws::{OP_BINARY, OP_CLOSE, OP_CONT, OP_PING, OP_TEXT};
use ergon_io::{
    AsciiText, ConnId, Flow, Fragments, Handler, Interest, NonAscii, RxMeta, TextCheck, TextStatus,
    WsHandler, WsOptions, WsStats,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};
use support::{Lab, R, frame};
use ws_rig::{WsRig, fragmented};

/// One delivery: through which method, the bytes, and the text status.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Got {
    Message(u8, Vec<u8>, TextStatus),
    Fragments(u8, Vec<u8>, TextStatus),
    Fragment {
        op: u8,
        part: Vec<u8>,
        index: u32,
        fin: bool,
        text: TextStatus,
    },
}

/// Takes every message as `interest`; streamed fragments get `flow`, or
/// `stop_at` returns `SkipRest` from that fragment on. `buffered` records
/// what the connection held while each fragment was handed over.
struct App {
    interest: Interest,
    stop_at: Option<u32>,
    got: Vec<Got>,
}

impl App {
    const fn new(interest: Interest) -> Self {
        Self {
            interest,
            stop_at: None,
            got: Vec::new(),
        }
    }

    /// Every delivered message joined, with the statuses seen.
    fn messages(&self) -> Vec<(Vec<u8>, Vec<TextStatus>)> {
        let mut out = Vec::new();
        let mut open: Option<(Vec<u8>, Vec<TextStatus>)> = None;
        for g in &self.got {
            match g {
                Got::Message(_, d, t) | Got::Fragments(_, d, t) => out.push((d.clone(), vec![*t])),
                Got::Fragment {
                    part,
                    index,
                    fin,
                    text,
                    ..
                } => {
                    if *index == 0 {
                        open = Some((Vec::new(), Vec::new()));
                    }
                    if let Some((d, t)) = open.as_mut() {
                        d.extend_from_slice(part);
                        t.push(*text);
                    }
                    if *fin {
                        out.extend(open.take());
                    }
                }
            }
        }
        out
    }
}

impl Handler for App {}

impl WsHandler for App {
    fn on_message(&mut self, _: ConnId, op: u8, payload: &[u8], meta: &RxMeta, _: u16) {
        self.got.push(Got::Message(op, payload.to_vec(), meta.text));
    }

    fn on_message_start(&mut self, _: ConnId, _: u8, _: &[u8], _: bool, m: &RxMeta) -> Interest {
        assert_eq!(
            m.text,
            TextStatus::Unchecked,
            "not checked before the decision"
        );
        self.interest
    }

    fn on_fragments(
        &mut self,
        _: ConnId,
        op: u8,
        m: &Fragments<'_>,
        meta: &RxMeta,
        _: u16,
    ) -> Flow {
        let data: Vec<u8> = m.iter().flatten().copied().collect();
        self.got.push(Got::Fragments(op, data, meta.text));
        Flow::Continue
    }

    fn on_fragment(
        &mut self,
        _: ConnId,
        op: u8,
        part: &[u8],
        index: u32,
        fin: bool,
        meta: &RxMeta,
    ) -> Flow {
        self.got.push(Got::Fragment {
            op,
            part: part.to_vec(),
            index,
            fin,
            text: meta.text,
        });
        match self.stop_at {
            Some(k) if index >= k => Flow::SkipRest,
            _ => Flow::Continue,
        }
    }
}

fn opts(text_check: TextCheck) -> WsOptions {
    WsOptions {
        ping_interval_ms: 0,
        text_check,
        ..WsOptions::default()
    }
}

fn ascii(len: usize) -> Vec<u8> {
    (0..len).map(|i| b'a' + (i % 26) as u8).collect()
}

const fn stats(r: &WsRig) -> WsStats {
    r.conn.stats()
}

const DELIVER: TextCheck = TextCheck::AsciiChecked(NonAscii::DeliverUnchecked);

// ---- TextCheck ------------------------------------------------------------

/// What each mode flags, for unfragmented and fragmented text and binary,
/// under every interest that delivers.
#[test]
fn each_text_mode_flags_what_it_checked() -> R {
    let lab = Lab::new()?;
    let cases = [
        (
            TextCheck::None,
            TextStatus::Unchecked,
            TextStatus::Unchecked,
        ),
        (TextCheck::Utf8, TextStatus::Utf8, TextStatus::Utf8),
        (
            TextCheck::Ascii,
            TextStatus::AssumedAscii,
            TextStatus::AssumedAscii,
        ),
        (DELIVER, TextStatus::Ascii, TextStatus::NotAscii),
    ];
    let pure = ascii(500);
    let mut mixed = ascii(500);
    mixed[250..252].copy_from_slice("é".as_bytes());
    for (check, on_pure, on_mixed) in cases {
        for interest in [Interest::Keep, Interest::Fragments, Interest::Stream] {
            for parts in [1, 3] {
                for (msg, want) in [(&pure, on_pure), (&mixed, on_mixed)] {
                    let mut r = WsRig::new(&lab, &opts(check))?;
                    let mut h = App::new(interest);
                    r.feed_records(&fragmented(OP_TEXT, msg, parts)?, &[], &mut h)?;
                    r.feed_records(&fragmented(OP_BINARY, msg, parts)?, &[], &mut h)?;
                    let got = h.messages();
                    let what = format!("{check:?} {interest:?} {parts} parts");
                    assert_eq!(got.len(), 2, "{what}");
                    assert_eq!(&got[0].0, msg, "{what}");
                    // A streamed text fragment is flagged by its own bytes:
                    // the fragment holding "é" is NotAscii, the others Ascii.
                    if interest == Interest::Stream && parts > 1 && check == DELIVER {
                        let not = got[0].1.iter().filter(|&&t| t == TextStatus::NotAscii);
                        assert_eq!(not.count(), usize::from(msg == &mixed), "{what}");
                    } else {
                        assert!(
                            got[0].1.iter().all(|&t| t == want),
                            "{what}: {:?}",
                            got[0].1
                        );
                    }
                    assert!(
                        got[1].1.iter().all(|&t| t == TextStatus::Unchecked),
                        "{what}: binary is never checked"
                    );
                    let s = stats(&r);
                    let non_ascii = u64::from(check == DELIVER && msg == &mixed);
                    assert_eq!(s.non_ascii, non_ascii, "{what}");
                }
            }
        }
    }
    Ok(())
}

/// Under `Ascii`, a non-ASCII byte anywhere (every offset of the scan's
/// blocks and tails, every fragment) meets the policy: `Close` closes,
/// `Skip` drops the message and keeps the next, `DeliverUnchecked` delivers
/// it flagged.
#[test]
fn a_non_ascii_byte_meets_the_policy() -> R {
    let lab = Lab::new()?;
    let len = 3 * 37;
    let next = ascii(64);
    for interest in [Interest::Keep, Interest::Fragments, Interest::Stream] {
        for at in (0..len).step_by(7) {
            let mut bad = ascii(len);
            bad[at] = 0x80 | (at as u8 & 0x3F);
            let stream = fragmented(OP_TEXT, &bad, 3)?;
            let mut then = stream.clone();
            then.extend_from_slice(&frame(OP_TEXT, true, &next)?);
            let what = format!("{interest:?} byte at {at}");

            let mut r = WsRig::new(&lab, &opts(TextCheck::AsciiChecked(NonAscii::Close)))?;
            let mut h = App::new(interest);
            let e = r
                .feed_records(&then, &[], &mut h)
                .err()
                .map(|e| e.to_string());
            assert!(e.is_some_and(|e| e.contains("not ASCII")), "{what}");
            assert!(
                h.messages().iter().all(|(d, _)| d != &bad),
                "{what}: the message was delivered"
            );

            let mut r = WsRig::new(&lab, &opts(TextCheck::AsciiChecked(NonAscii::Skip)))?;
            let mut h = App::new(interest);
            r.feed_records(&then, &[], &mut h)?;
            let got = h.messages();
            assert_eq!(got.last().map(|g| &g.0), Some(&next), "{what}: next kept");
            assert!(
                got.iter().all(|(d, _)| d != &bad),
                "{what}: delivered whole"
            );
            let s = stats(&r);
            assert_eq!((s.non_ascii, s.skipped, s.messages), (1, 1, 1), "{what}");

            let mut r = WsRig::new(&lab, &opts(DELIVER))?;
            let mut h = App::new(interest);
            r.feed_records(&then, &[], &mut h)?;
            let got = h.messages();
            assert_eq!(got.len(), 2, "{what}");
            assert_eq!(got[0].0, bad, "{what}");
            assert!(got[0].1.contains(&TextStatus::NotAscii), "{what}");
            assert_eq!(got[1].1, vec![TextStatus::Ascii], "{what}");
            assert_eq!((stats(&r).non_ascii, stats(&r).skipped), (1, 0), "{what}");
        }
    }
    Ok(())
}

/// `Stream` under `AsciiChecked(Skip)`: a later fragment that is not ASCII
/// ends the message mid-way. The fragments before it were delivered, no
/// `fin` fragment follows and no callback reports the drop; `skipped`
/// counts it once its last frame arrives, and the next message starts
/// again at index 0. One record per frame, and all in one read.
#[test]
fn a_streamed_non_ascii_fragment_ends_the_message_without_fin() -> R {
    let lab = Lab::new()?;
    let skip = TextCheck::AsciiChecked(NonAscii::Skip);
    for bad_at in 1..4u32 {
        let mut first = ascii(400);
        first[bad_at as usize * 100 + 50] = 0xC3;
        let next = ascii(300);
        let mut wire = fragmented(OP_TEXT, &first, 4)?;
        wire.extend_from_slice(&fragmented(OP_TEXT, &next, 3)?);
        for per_frame in [true, false] {
            let what = format!("non-ASCII fragment {bad_at}, one read per frame {per_frame}");
            let mut r = WsRig::new(&lab, &opts(skip))?;
            let mut h = App::new(Interest::Stream);
            if per_frame {
                let mut at = 0;
                while at < wire.len() {
                    let f = ergon_io::ws::parse_header(&wire[at..])?.ok_or("short")?;
                    r.feed_records(&wire[at..at + f.frame_len()], &[], &mut h)?;
                    at += f.frame_len();
                }
            } else {
                r.feed_records(&wire, &[], &mut h)?;
            }
            let seen: Vec<(u32, bool, usize)> = h
                .got
                .iter()
                .map(|g| match g {
                    Got::Fragment {
                        index, fin, part, ..
                    } => Ok((*index, *fin, part.len())),
                    other => Err(format!("{what}: not a fragment: {other:?}")),
                })
                .collect::<Result<_, _>>()?;
            let mut want: Vec<(u32, bool, usize)> = (0..bad_at).map(|i| (i, false, 100)).collect();
            want.extend([(0, false, 100), (1, false, 100), (2, true, 100)]);
            assert_eq!(seen, want, "{what}");
            let delivered: Vec<u8> = h.got[..bad_at as usize]
                .iter()
                .flat_map(|g| match g {
                    Got::Fragment { part, .. } => part.clone(),
                    _ => Vec::new(),
                })
                .collect();
            assert_eq!(delivered, first[..bad_at as usize * 100], "{what}");
            assert_eq!(
                h.messages(),
                vec![(next.clone(), vec![TextStatus::Ascii; 3])]
            );
            let s = stats(&r);
            assert_eq!(
                (s.skipped, s.non_ascii, s.frag_copy_bytes),
                (1, 1, 0),
                "{what}"
            );
        }
    }
    Ok(())
}

/// A skipped message is never read: invalid text in it closes nothing,
/// whatever the check.
#[test]
fn a_skipped_message_is_never_checked() -> R {
    let lab = Lab::new()?;
    let bad = b"price \xff\xfe".to_vec();
    for check in [TextCheck::Utf8, TextCheck::AsciiChecked(NonAscii::Close)] {
        for parts in [1, 3] {
            let mut r = WsRig::new(&lab, &opts(check))?;
            let mut h = App::new(Interest::Skip);
            r.feed_records(&fragmented(OP_TEXT, &bad, parts)?, &[], &mut h)?;
            let s = stats(&r);
            assert_eq!(
                (s.skipped, s.non_ascii, h.got.len()),
                (1, 0, 0),
                "{check:?}"
            );
        }
    }
    Ok(())
}

/// `Ascii` is trust, not a check: bytes that are not ASCII (not even
/// UTF-8) are delivered unchanged, flagged `AssumedAscii`, under every
/// interest and fragmentation; nothing closes, drops or counts them.
#[test]
fn trusted_ascii_delivers_any_bytes_unchanged() -> R {
    let lab = Lab::new()?;
    let mut bad = ascii(300);
    bad[17] = 0xFF;
    bad[150..152].copy_from_slice("\u{e9}".as_bytes());
    bad[299] = 0x80;
    let next = ascii(64);
    for interest in [Interest::Keep, Interest::Fragments, Interest::Stream] {
        for parts in [1, 3] {
            let what = format!("{interest:?} {parts} parts");
            let mut stream = fragmented(OP_TEXT, &bad, parts)?;
            stream.extend_from_slice(&frame(OP_TEXT, true, &next)?);
            let mut r = WsRig::new(&lab, &opts(TextCheck::Ascii))?;
            let mut h = App::new(interest);
            r.feed_records(&stream, &[], &mut h)?;
            let got = h.messages();
            assert_eq!(got.len(), 2, "{what}");
            assert_eq!(got[0].0, bad, "{what}: delivered unchanged");
            assert_eq!(got[1].0, next, "{what}");
            for (_, texts) in &got {
                assert!(
                    texts.iter().all(|&t| t == TextStatus::AssumedAscii),
                    "{what}: {texts:?}"
                );
            }
            let s = stats(&r);
            assert_eq!((s.messages, s.non_ascii, s.skipped), (2, 0, 0), "{what}");
        }
    }
    // `Stream` without `on_fragment`: joined, still unread.
    let mut r = WsRig::new(&lab, &opts(TextCheck::Ascii))?;
    let mut h = OnlyMessages::default();
    r.feed_records(&fragmented(OP_TEXT, &bad, 4)?, &[], &mut h)?;
    assert_eq!(h.got, vec![(bad, TextStatus::AssumedAscii)]);
    assert_eq!(stats(&r).non_ascii, 0);
    Ok(())
}

// ANCHOR: trusted
/// A feed known to send ASCII, under `TextCheck::Ascii`: the payload is
/// never scanned; the handler reads it by byte through `AsciiText`.
#[derive(Default)]
struct Prices {
    last: Vec<u8>,
}

impl Handler for Prices {}

impl WsHandler for Prices {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        let t = AsciiText::new(payload);
        let Some(at) = t.find(b'=') else { return };
        if let Some(price) = t.slice(at + 1..) {
            self.last.clear();
            self.last.extend_from_slice(price.as_bytes());
        }
    }
}
// ANCHOR_END: trusted

#[test]
fn the_book_trusted_example_reads_by_byte() -> R {
    let lab = Lab::new()?;
    let mut r = WsRig::new(&lab, &opts(TextCheck::Ascii))?;
    let mut h = Prices::default();
    r.feed_records(&frame(OP_TEXT, true, b"px=101.25")?, &[], &mut h)?;
    assert_eq!(h.last, b"101.25");
    Ok(())
}

// ---- Interest::Stream -----------------------------------------------------

/// Each fragment, one read each, reaches `on_fragment` within its own read,
/// borrowed: nothing is copied and nothing stays in the buffer between
/// fragments, so a message of any size costs no buffer space.
#[test]
fn a_streamed_fragment_is_handed_over_as_it_arrives() -> R {
    let lab = Lab::new()?;
    let msg = ascii(10 * 9000);
    let stream = fragmented(OP_TEXT, &msg, 10)?;
    let mut ends = Vec::new();
    let mut at = 0;
    while at < stream.len() {
        let h = ergon_io::ws::parse_header(&stream[at..])?.ok_or("short")?;
        at += h.frame_len();
        ends.push(at);
    }
    for check in [TextCheck::None, TextCheck::Utf8, TextCheck::Ascii, DELIVER] {
        let mut r = WsRig::new(&lab, &opts(check))?;
        let mut h = App::new(Interest::Stream);
        let mut from = 0;
        for (k, &end) in ends.iter().enumerate() {
            r.feed_records(&stream[from..end], &[], &mut h)?;
            from = end;
            assert_eq!(h.got.len(), k + 1, "{check:?}: fragment {k} arrived");
            assert_eq!(r.conn.buffered(), 0, "{check:?}: held after fragment {k}");
            let Some(Got::Fragment { index, fin, .. }) = h.got.last() else {
                return Err("not a fragment".into());
            };
            assert_eq!((*index, *fin), (k as u32, k == 9));
        }
        let want = match check {
            TextCheck::None => TextStatus::Unchecked,
            TextCheck::Utf8 => TextStatus::Utf8,
            TextCheck::Ascii => TextStatus::AssumedAscii,
            TextCheck::AsciiChecked(_) => TextStatus::Ascii,
        };
        assert_eq!(h.messages(), vec![(msg.clone(), vec![want; 10])]);
        let s = stats(&r);
        assert_eq!(
            (
                s.messages,
                s.frag_copy_bytes,
                s.frag_fallbacks,
                s.span_copy_bytes
            ),
            (1, 0, 0, 0),
            "{check:?}"
        );
    }
    Ok(())
}

/// More fragments and more bytes than `Fragments` could hold stream fine:
/// no fallback, no copy; `max_message` still applies.
#[test]
fn streaming_has_no_hold_limit_but_max_message_holds() -> R {
    let lab = Lab::new()?;
    let small_buf = WsOptions {
        buf_size: ergon_io::ws_conn::MIN_BUF,
        max_message: 1 << 20,
        ..opts(TextCheck::None)
    };
    let mut r = WsRig::new(&lab, &small_buf)?;
    let mut h = App::new(Interest::Stream);
    let msg: Vec<u8> = (0..400 * 1024usize).map(|i| (i % 251) as u8).collect();
    r.feed_records(&fragmented(OP_BINARY, &msg, 100)?, &[], &mut h)?;
    assert_eq!(h.messages().len(), 1);
    assert!(h.messages()[0].0 == msg, "payload differs");
    let st = stats(&r);
    assert_eq!((st.frag_fallbacks, st.frag_copy_bytes), (0, 0));

    let small = WsOptions {
        max_message: 1000,
        ..opts(TextCheck::None)
    };
    let mut r = WsRig::new(&lab, &small)?;
    let mut h = App::new(Interest::Stream);
    let e = r.feed_records(&fragmented(OP_BINARY, &ascii(1500), 3)?, &[], &mut h);
    assert!(
        e.err()
            .is_some_and(|e| e.to_string().contains("message too large"))
    );
    Ok(())
}

/// `SkipRest` drops the rest: later fragments are neither delivered nor
/// checked (invalid UTF-8 there closes nothing) nor copied, and the next
/// message arrives as usual.
#[test]
fn skip_rest_drops_the_remaining_fragments() -> R {
    let lab = Lab::new()?;
    let mut msg = ascii(900);
    msg[700] = 0xFF;
    for stop in 0..3u32 {
        let mut r = WsRig::new(&lab, &opts(TextCheck::Utf8))?;
        let mut h = App::new(Interest::Stream);
        h.stop_at = Some(stop);
        let mut s = fragmented(OP_TEXT, &msg, 4)?;
        s.extend_from_slice(&frame(OP_TEXT, true, b"next")?);
        r.feed_records(&s, &[], &mut h)?;
        let frags = h
            .got
            .iter()
            .filter(|g| matches!(g, Got::Fragment { .. }))
            .count();
        assert_eq!(frags, stop as usize + 1, "stop at {stop}");
        assert_eq!(
            h.got.last(),
            Some(&Got::Message(OP_TEXT, b"next".to_vec(), TextStatus::Utf8))
        );
        let st = stats(&r);
        assert_eq!((st.skipped, st.messages, st.frag_copy_bytes), (1, 1, 0));
    }
    Ok(())
}

/// A Ping between streamed fragments is answered; a Close ends the message
/// with no `fin` fragment.
#[test]
fn control_frames_inside_a_streamed_message_are_handled() -> R {
    let lab = Lab::new()?;
    let msg = ascii(900);
    let f = fragmented(OP_TEXT, &msg, 3)?;
    let first = ergon_io::ws::parse_header(&f)?.ok_or("short")?.frame_len();
    let mut s = f[..first].to_vec();
    s.extend_from_slice(&frame(OP_PING, true, b"hb")?);
    s.extend_from_slice(&f[first..]);
    let mut r = WsRig::new(&lab, &opts(DELIVER))?;
    let mut h = App::new(Interest::Stream);
    r.feed_records(&s, &[40], &mut h)?;
    assert_eq!(h.messages(), vec![(msg, vec![TextStatus::Ascii; 3])]);
    assert_eq!((stats(&r).pings, stats(&r).auto_frames), (1, 1));

    let mut s = f[..first].to_vec();
    s.extend_from_slice(&frame(OP_CLOSE, true, &1000u16.to_be_bytes())?);
    s.extend_from_slice(&f[first..]);
    let mut r = WsRig::new(&lab, &opts(TextCheck::None))?;
    let mut h = App::new(Interest::Stream);
    r.feed_records(&s, &[], &mut h)?;
    assert_eq!(h.got.len(), 1, "only the fragment before the Close");
    assert!(h.messages().is_empty(), "no fin fragment");
    assert!(r.conn.is_closing());
    assert_eq!(r.conn.buffered(), 0);
    Ok(())
}

/// UTF-8 is checked fragment by fragment, a character cut between two
/// fragments carried across (at most 3 bytes): every cut of a mixed text
/// is valid; an invalid byte closes at the fragment that holds it, before
/// that fragment is delivered; a character left open at the end closes.
#[test]
fn utf8_is_checked_across_streamed_fragments() -> R {
    let lab = Lab::new()?;
    let msg = "a€b😀c".as_bytes().to_vec();
    for cut1 in 1..msg.len() {
        for cut2 in cut1..msg.len() {
            let mut s = frame(OP_TEXT, false, &msg[..cut1])?;
            s.extend_from_slice(&frame(OP_CONT, false, &msg[cut1..cut2])?);
            s.extend_from_slice(&frame(OP_CONT, true, &msg[cut2..])?);
            let mut r = WsRig::new(&lab, &opts(TextCheck::Utf8))?;
            let mut h = App::new(Interest::Stream);
            r.feed_records(&s, &[], &mut h)?;
            assert_eq!(
                h.messages(),
                vec![(msg.clone(), vec![TextStatus::Utf8; 3])],
                "{cut1} {cut2}"
            );
        }
    }
    let mut s = frame(OP_TEXT, false, b"ok")?;
    s.extend_from_slice(&frame(OP_CONT, false, b"\xff")?);
    s.extend_from_slice(&frame(OP_CONT, true, b"never")?);
    let mut r = WsRig::new(&lab, &opts(TextCheck::Utf8))?;
    let mut h = App::new(Interest::Stream);
    let e = r.feed_records(&s, &[], &mut h).err().map(|e| e.to_string());
    assert!(e.is_some_and(|e| e.contains("not UTF-8")));
    assert_eq!(h.got.len(), 1, "the invalid fragment is not delivered");

    let mut s = frame(OP_TEXT, false, b"ok")?;
    s.extend_from_slice(&frame(OP_CONT, true, &"€".as_bytes()[..2])?);
    let mut r = WsRig::new(&lab, &opts(TextCheck::Utf8))?;
    let mut h = App::new(Interest::Stream);
    assert!(r.feed_records(&s, &[], &mut h).is_err());
    Ok(())
}

/// A character cut by a fragment's end is checked as far as it goes: a
/// fragment whose first bytes cannot continue it is refused before it is
/// handed over flagged UTF-8, not one fragment later.
#[test]
fn a_bad_continuation_of_a_cut_character_is_refused_at_once() -> R {
    let lab = Lab::new()?;
    // (piece ending in a cut lead, the next piece, both never valid)
    let cases: [(&[u8], &[u8]); 4] = [
        (b"a\xe2", b"A"),
        (b"a\xf0\x9f", b"Z"),
        // Overlong and surrogate starts: the second byte is out of range.
        (b"a\xe0", b"\x80"),
        (b"a\xed", b"\xa0"),
    ];
    for (first, second) in cases {
        let mut s = frame(OP_TEXT, false, first)?;
        s.extend_from_slice(&frame(OP_CONT, false, second)?);
        s.extend_from_slice(&frame(OP_CONT, true, b"never")?);
        let mut r = WsRig::new(&lab, &opts(TextCheck::Utf8))?;
        let mut h = App::new(Interest::Stream);
        let e = r.feed_records(&s, &[], &mut h).err().map(|e| e.to_string());
        assert!(
            e.is_some_and(|e| e.contains("not UTF-8")),
            "{first:?} {second:?}"
        );
        assert_eq!(h.got.len(), 1, "{first:?} {second:?}: delivered as UTF-8");
    }
    Ok(())
}

// ---- Defaults keep what a handler did not take ----------------------------

/// Implements only `on_message` and asks for `Stream`.
#[derive(Default)]
struct OnlyMessages {
    got: Vec<(Vec<u8>, TextStatus)>,
}

impl Handler for OnlyMessages {}

impl WsHandler for OnlyMessages {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], meta: &RxMeta, _: u16) {
        self.got.push((payload.to_vec(), meta.text));
    }

    fn on_message_start(&mut self, _: ConnId, _: u8, _: &[u8], _: bool, _: &RxMeta) -> Interest {
        Interest::Stream
    }
}

/// `Stream` without `on_fragment`: the message is copied as it arrives and
/// delivered whole, checked, through `on_message`; the copy is counted.
#[test]
fn stream_without_on_fragment_arrives_through_on_message() -> R {
    let lab = Lab::new()?;
    let msg = ascii(3000);
    let mut r = WsRig::new(&lab, &opts(DELIVER))?;
    let mut h = OnlyMessages::default();
    r.feed_records(&fragmented(OP_TEXT, &msg, 4)?, &[], &mut h)?;
    assert_eq!(h.got, vec![(msg.clone(), TextStatus::Ascii)]);
    let s = stats(&r);
    assert_eq!(
        (s.messages, s.unhandled, s.frag_copy_bytes),
        (1, 1, msg.len() as u64)
    );
    Ok(())
}

/// `Stream` without `on_fragment` on text that is not ASCII: the message
/// is counted once in `non_ascii`, not once by the streamed first fragment
/// and again by the joined message.
#[test]
fn an_unhandled_stream_counts_a_non_ascii_message_once() -> R {
    let lab = Lab::new()?;
    let msg = "x€".repeat(500).into_bytes();
    let mut r = WsRig::new(&lab, &opts(DELIVER))?;
    let mut h = OnlyMessages::default();
    r.feed_records(&fragmented(OP_TEXT, &msg, 4)?, &[], &mut h)?;
    assert_eq!(h.got, vec![(msg, TextStatus::NotAscii)]);
    let s = stats(&r);
    assert_eq!((s.messages, s.unhandled, s.non_ascii), (1, 1, 1));
    Ok(())
}

// ---- Random fragmentation -------------------------------------------------

/// Text that is ASCII, or ASCII with multi-byte characters, cut into
/// fragments at any byte (so characters are split between fragments).
fn text_strategy() -> impl Strategy<Value = (Vec<u8>, usize, bool)> {
    let pieces = prop::collection::vec(
        prop_oneof![
            8 => Just("abc{\"x\":1}".to_owned()),
            1 => Just("é".to_owned()),
            1 => Just("€".to_owned()),
            1 => Just("😀".to_owned()),
        ],
        0..400,
    );
    (pieces, 1usize..9).prop_map(|(p, parts)| {
        let s = p.concat();
        let is_ascii = s.is_ascii();
        (s.into_bytes(), parts, is_ascii)
    })
}

/// Any fragmentation, any record cuts, every mode and interest: the
/// message arrives intact, flagged as its bytes are, with no copy outside
/// `Keep`.
#[test]
fn any_fragmentation_keeps_text_and_flags_it_correctly() -> R {
    let lab = Lab::new()?;
    let strategy = (
        prop::collection::vec(text_strategy(), 1..6),
        prop::collection::vec(any::<prop::sample::Index>(), 0..12),
        0usize..4,
        prop_oneof![
            Just(Interest::Keep),
            Just(Interest::Fragments),
            Just(Interest::Stream)
        ],
    );
    let mut runner = TestRunner::new(Config {
        cases: 64,
        rng_seed: RngSeed::Fixed(0x7465_7874_6368_6b31),
        failure_persistence: None,
        ..Config::default()
    });
    let fail = |e: Box<dyn std::error::Error + Send + Sync>| TestCaseError::fail(e.to_string());
    runner
        .run(&strategy, |(msgs, cuts, mode, interest)| {
            let check = [TextCheck::None, TextCheck::Utf8, TextCheck::Ascii, DELIVER][mode];
            let mut stream = Vec::new();
            for (m, parts, _) in &msgs {
                stream.extend_from_slice(&fragmented(OP_TEXT, m, *parts).map_err(fail)?);
            }
            let cuts: Vec<usize> = cuts.iter().map(|i| i.index(stream.len().max(1))).collect();
            let mut r = WsRig::new(&lab, &opts(check)).map_err(fail)?;
            let mut h = App::new(interest);
            r.feed_records(&stream, &cuts, &mut h).map_err(fail)?;
            let got = h.messages();
            prop_assert_eq!(got.len(), msgs.len());
            for ((data, texts), (m, parts, is_ascii)) in got.iter().zip(&msgs) {
                prop_assert_eq!(data, m);
                let streamed = interest == Interest::Stream && *parts > 1;
                match check {
                    TextCheck::None => {
                        prop_assert!(texts.iter().all(|&t| t == TextStatus::Unchecked));
                    }
                    TextCheck::Utf8 => {
                        prop_assert!(texts.iter().all(|&t| t == TextStatus::Utf8));
                    }
                    TextCheck::Ascii => {
                        prop_assert!(texts.iter().all(|&t| t == TextStatus::AssumedAscii));
                    }
                    TextCheck::AsciiChecked(_) if streamed => {
                        // Per fragment: Ascii exactly when its bytes are.
                        prop_assert_eq!(texts.contains(&TextStatus::NotAscii), !is_ascii);
                    }
                    TextCheck::AsciiChecked(_) => {
                        let want = if *is_ascii {
                            TextStatus::Ascii
                        } else {
                            TextStatus::NotAscii
                        };
                        prop_assert_eq!(texts, &vec![want]);
                    }
                }
            }
            let s = r.conn.stats();
            if interest != Interest::Keep {
                prop_assert_eq!(s.frag_copy_bytes, 0);
            }
            prop_assert_eq!(r.conn.buffered(), 0);
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ANCHOR: stream
/// Reads only the messages it needs from a feed of large fragmented ones:
/// the kind is in the first fragment, so every other message is dropped
/// there (`SkipRest`: its other fragments are never copied or checked).
/// With `TextCheck::AsciiChecked`, `meta.text` says each fragment is ASCII: any
/// byte index is a character boundary.
#[derive(Default)]
struct FillReader {
    fills: usize,
    bytes: usize,
    stopped: usize,
    unchecked: usize,
}

impl Handler for FillReader {}

impl WsHandler for FillReader {
    // Unfragmented messages arrive whole.
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        if payload.starts_with(br#"{"kind":"fill""#) {
            self.fills += 1;
            self.bytes += payload.len();
        }
    }

    fn on_message_start(&mut self, _: ConnId, _: u8, _: &[u8], _: bool, _: &RxMeta) -> Interest {
        Interest::Stream
    }

    // Fragmented ones arrive a fragment at a time, nothing held.
    fn on_fragment(
        &mut self,
        _: ConnId,
        _: u8,
        part: &[u8],
        index: u32,
        fin: bool,
        meta: &RxMeta,
    ) -> Flow {
        if index == 0 && !part.starts_with(br#"{"kind":"fill""#) {
            self.stopped += 1;
            return Flow::SkipRest;
        }
        if meta.text != TextStatus::Ascii {
            self.unchecked += 1;
        }
        self.bytes += part.len();
        self.fills += usize::from(fin);
        Flow::Continue
    }
}
// ANCHOR_END: stream

#[test]
fn the_book_example_streams_only_what_it_needs() -> R {
    let lab = Lab::new()?;
    let mut r = WsRig::new(&lab, &opts(DELIVER))?;
    let mut h = FillReader::default();
    let fill = format!(r#"{{"kind":"fill","rows":"{}"}}"#, "x".repeat(5000));
    let other = format!(r#"{{"kind":"book","rows":"{}"}}"#, "y".repeat(5000));
    let mut s = fragmented(OP_TEXT, fill.as_bytes(), 4)?;
    s.extend_from_slice(&fragmented(OP_TEXT, other.as_bytes(), 4)?);
    s.extend_from_slice(&frame(OP_TEXT, true, fill.as_bytes())?);
    r.feed_records(&s, &[], &mut h)?;
    assert_eq!((h.fills, h.bytes), (2, 2 * fill.len()));
    assert_eq!((h.stopped, h.unchecked), (1, 0));
    let st = stats(&r);
    assert_eq!((st.skipped, st.frag_copy_bytes, st.messages), (1, 0, 2));
    Ok(())
}
