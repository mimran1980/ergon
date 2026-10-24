//! The HTTP response path, fed record by record: chunked bodies cut at every
//! byte, each body byte moved at most once, a single chunk borrowed, the
//! head parsed once, the ASCII head rule and the body text check. Every
//! platform: no driver.
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod http_rig;
mod support;

use ergon_io::{
    ConnId, Error, Handler, HttpHandler, HttpOptions, NonAscii, NonAsciiHead, Response, RxMeta,
    TextCheck, TextStatus,
};
use http_rig::HttpRig;
use support::{Lab, R};

/// Status, body, every header and whether the head was ASCII.
type Seen = (u16, Vec<u8>, Vec<(String, Vec<u8>)>, bool);

#[derive(Default)]
struct Collect {
    responses: Vec<Seen>,
    /// What the body check found for each response (`RxMeta::text`).
    text: Vec<TextStatus>,
}

impl Handler for Collect {}

impl HttpHandler for Collect {
    fn on_response(&mut self, _: ConnId, _: u64, r: &Response<'_>, meta: &RxMeta) {
        self.text.push(meta.text);
        self.responses.push((
            r.status,
            r.body.to_vec(),
            r.headers
                .iter()
                .map(|h| (h.name.to_owned(), h.value.to_vec()))
                .collect(),
            r.head_ascii,
        ));
    }
}

fn body_of(len: usize) -> Vec<u8> {
    (0..len).map(|i| b'a' + (i % 26) as u8).collect()
}

/// A chunked response carrying `body` in chunks of `sizes` (the last size
/// repeats until the body is used up), then `0`, `trailer` and CRLF.
fn chunked(body: &[u8], sizes: &[usize], trailer: &str) -> (Vec<u8>, usize) {
    let mut out =
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nX-Kind: chunked\r\n\r\n".to_vec();
    let head = out.len();
    let mut at = 0;
    let mut i = 0;
    while at < body.len() {
        let n = sizes[i.min(sizes.len() - 1)].min(body.len() - at);
        out.extend_from_slice(format!("{n:x}\r\n").as_bytes());
        out.extend_from_slice(&body[at..at + n]);
        out.extend_from_slice(b"\r\n");
        at += n;
        i += 1;
    }
    out.extend_from_slice(format!("0\r\n{trailer}\r\n").as_bytes());
    (out, head)
}

fn rig(lab: &Lab, buf_size: usize) -> R<HttpRig> {
    HttpRig::new(
        lab,
        HttpOptions {
            buf_size,
            max_in_flight: 1024,
            ..HttpOptions::default()
        },
    )
}

fn ask(r: &mut HttpRig) -> R {
    r.conn.request("GET", "/x", &[], &[])?;
    r.conn.clear_tx();
    Ok(())
}

/// One chunked response cut into two records at every byte offset: the body
/// arrives whole, the head is parsed once when it came whole in the first
/// record, and the copies stay within one pass over head and body.
#[test]
fn chunked_bodies_cut_at_every_offset_move_each_byte_at_most_once() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, 1 << 20)?;
    let body = body_of(1500);
    for (sizes, trailer) in [
        (&[700usize, 1, 300, 499][..], ""),
        (&[16][..], "X-Sum: 1\r\n"),
        (&[1500][..], ""),
    ] {
        let (resp, head) = chunked(&body, sizes, trailer);
        for cut in 1..resp.len() {
            let mut h = Collect::default();
            ask(&mut r)?;
            let s0 = r.conn.stats();
            r.feed_records(&resp, &[cut], &mut h)?;
            let s1 = r.conn.stats();
            assert_eq!(h.responses.len(), 1, "cut {cut}");
            assert_eq!(h.responses[0].1, body, "cut {cut}");
            assert_eq!(
                h.responses[0].2,
                vec![
                    ("Transfer-Encoding".to_owned(), b"chunked".to_vec()),
                    ("X-Kind".to_owned(), b"chunked".to_vec())
                ],
                "cut {cut}"
            );
            // A whole head in the first record: the head, then each body
            // byte, copied once as it streams in. A cut head: the response
            // is whole when the head is, so one chunk is borrowed and
            // several are copied once.
            let copied = s1.chunk_copy_bytes - s0.chunk_copy_bytes;
            let want = if cut >= head {
                head + body.len()
            } else if sizes.len() == 1 && sizes[0] >= body.len() {
                0
            } else {
                body.len()
            };
            assert_eq!(copied, want as u64, "sizes {sizes:?} cut {cut}");
            let parses = s1.head_parses - s0.head_parses;
            assert_eq!(parses, if cut >= head { 1 } else { 2 }, "cut {cut}");
            // A whole head lets the body stream out, so the window empties
            // and the second record is read where it landed.
            if cut >= head {
                assert_eq!(s1.span_copy_bytes, s0.span_copy_bytes, "cut {cut}");
            }
            assert_eq!(r.conn.in_flight(), 0);
        }
    }
    Ok(())
}

/// 4 KiB, 64 KiB and 1 MB chunked bodies (4 KiB chunks) in 16 KiB records:
/// each body byte is copied once, never once per record.
#[test]
fn large_chunked_bodies_in_full_records_copy_each_byte_once() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, 4 << 20)?;
    for len in [4096, 64 << 10, 1_000_000] {
        let body = body_of(len);
        let (resp, head) = chunked(&body, &[4096], "");
        let mut h = Collect::default();
        ask(&mut r)?;
        let s0 = r.conn.stats();
        r.feed_records(&resp, &[], &mut h)?;
        let s1 = r.conn.stats();
        assert_eq!(h.responses.len(), 1);
        assert_eq!(h.responses[0].1, body, "{len}");
        // One record holds the whole 4 KiB response (one chunk: borrowed);
        // the others stream: the head and each body byte once.
        let copied = s1.chunk_copy_bytes - s0.chunk_copy_bytes;
        let want = if resp.len() <= http_rig::RECORD {
            0
        } else {
            head + len
        };
        assert_eq!(copied, want as u64, "{len}");
        assert_eq!(s1.head_parses - s0.head_parses, 1, "{len}");
        // Each record is read in place: nothing is left in the window for
        // the next one to be moved next to.
        assert_eq!(s1.span_copy_bytes, s0.span_copy_bytes, "{len}");
    }
    Ok(())
}

/// A body sent as one chunk, whole in the window, is a slice of the receive
/// buffer: nothing is copied.
#[test]
fn a_single_chunk_whole_in_the_window_is_borrowed() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, 1 << 20)?;
    for (len, trailer) in [(0, ""), (1, ""), (5000, ""), (16_000, "X-Sum: 9\r\n")] {
        let body = body_of(len);
        let (resp, _) = chunked(&body, &[len.max(1)], trailer);
        let mut h = Collect::default();
        ask(&mut r)?;
        let s0 = r.conn.stats();
        // One read, the response cut wherever 16 KiB records cut it.
        let all = r.records(&resp, &[])?.concat();
        r.feed(&all, &mut h)?;
        let s1 = r.conn.stats();
        assert_eq!(h.responses.len(), 1);
        assert_eq!(h.responses[0].1, body, "{len}");
        assert_eq!(s1.chunk_copy_bytes - s0.chunk_copy_bytes, 0, "{len}");
    }
    Ok(())
}

/// Several chunks whole in the window: the body is copied once.
#[test]
fn several_chunks_whole_in_the_window_are_copied_once() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, 1 << 20)?;
    let body = body_of(3000);
    let (resp, _) = chunked(&body, &[1000], "");
    let mut h = Collect::default();
    ask(&mut r)?;
    let s0 = r.conn.stats();
    r.feed_records(&resp, &[], &mut h)?;
    let s1 = r.conn.stats();
    assert_eq!(h.responses[0].1, body);
    assert_eq!(s1.chunk_copy_bytes - s0.chunk_copy_bytes, 3000);
    Ok(())
}

/// A `Content-Length` response whose head and body arrive in separate
/// records: the head is parsed once, its headers survive the body's
/// arrival and the buffer's compaction, and the body is borrowed.
#[test]
fn a_head_and_body_in_separate_records_parse_the_head_once() -> R {
    let lab = Lab::new()?;
    // The smallest buffer: the 64 KiB bodies force compaction.
    let mut r = rig(&lab, 160 << 10)?;
    for len in [0usize, 1, 4096, 64 << 10] {
        let body = body_of(len);
        let mut resp = format!(
            "HTTP/1.1 201 Created\r\nContent-Length: {len}\r\nX-Echo: abc\r\nX-Two: d e\r\n\r\n"
        )
        .into_bytes();
        let head = resp.len();
        resp.extend_from_slice(&body);
        // Twice in a row, so the second starts mid-buffer.
        let mut two = resp.clone();
        two.extend_from_slice(&resp);
        for cut in [head - 1, head, head + 1, head + len / 2, resp.len() + head] {
            let mut h = Collect::default();
            ask(&mut r)?;
            ask(&mut r)?;
            let s0 = r.conn.stats();
            r.feed_records(&two, &[cut, resp.len()], &mut h)?;
            let s1 = r.conn.stats();
            assert_eq!(h.responses.len(), 2, "{len} cut {cut}");
            for got in &h.responses {
                assert_eq!(got.0, 201);
                assert_eq!(got.1, body, "{len} cut {cut}");
                assert_eq!(
                    got.2,
                    vec![
                        ("Content-Length".to_owned(), len.to_string().into_bytes()),
                        ("X-Echo".to_owned(), b"abc".to_vec()),
                        ("X-Two".to_owned(), b"d e".to_vec())
                    ]
                );
            }
            assert_eq!(s1.chunk_copy_bytes, s0.chunk_copy_bytes);
            // A head cut in two is parsed twice (the first try is partial).
            let split = |at: usize| u64::from(cut > at && cut < at + head);
            let want = 2 + split(0) + split(resp.len());
            assert_eq!(s1.head_parses - s0.head_parses, want, "{len} cut {cut}");
        }
    }
    Ok(())
}

/// The status line and headers are ASCII by spec: a non-ASCII byte is
/// flagged by default (delivered, counted) and closes the connection when
/// asked to.
#[test]
fn a_non_ascii_head_is_flagged_by_default_and_rejected_on_request() -> R {
    let lab = Lab::new()?;
    let bad = b"HTTP/1.1 200 OK\xe9\r\nContent-Length: 2\r\nX-V: \xff\r\n\r\nhi";
    let good = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi";
    let mut r = rig(&lab, 1 << 20)?;
    let mut h = Collect::default();
    // The second time, the body comes in a later record: the flag is kept
    // with the head parsed earlier.
    let bad_head = bad.len() - 2;
    for (resp, cut) in [(&bad[..], 0), (&bad[..], bad_head), (&good[..], 0)] {
        ask(&mut r)?;
        r.feed_records(resp, &[cut], &mut h)?;
    }
    assert_eq!(
        h.responses.iter().map(|g| g.3).collect::<Vec<_>>(),
        vec![false, false, true]
    );
    assert_eq!(h.responses[0].2[1], ("X-V".to_owned(), vec![0xff]));
    assert_eq!(h.responses[1].2[1], ("X-V".to_owned(), vec![0xff]));
    assert_eq!(r.conn.stats().non_ascii_heads, 2);

    let mut r = HttpRig::new(
        &lab,
        HttpOptions {
            non_ascii_head: NonAsciiHead::Reject,
            ..HttpOptions::default()
        },
    )?;
    let mut h = Collect::default();
    ask(&mut r)?;
    r.feed_records(good, &[], &mut h)?;
    ask(&mut r)?;
    let rec = r.records(bad, &[])?.concat();
    let e = r
        .conn
        .feed(ConnId::from_parts(0, 0), &rec, &RxMeta::default(), &mut h);
    assert!(matches!(e, Err(Error::Http(_))), "{e:?}");
    assert_eq!(h.responses.len(), 1, "the good one only");
    Ok(())
}

/// `body_check` takes the WebSocket text check: off by default; UTF-8 and
/// ASCII under `NonAscii::Close` close on a failure; ASCII under
/// `NonAscii::DeliverUnchecked` delivers the body flagged. `RxMeta::text`
/// says what the check found.
#[test]
fn the_body_check_is_off_by_default_and_flags_or_closes() -> R {
    let lab = Lab::new()?;
    assert_eq!(HttpOptions::default().body_check, TextCheck::None);
    let latin = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n\u{e9}".as_bytes();
    let broken = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n\xe9";
    let ascii = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\na";
    let close = TextCheck::Ascii(NonAscii::Close);
    let flag = TextCheck::Ascii(NonAscii::DeliverUnchecked);
    // (check, response, what a delivered body is flagged; None: closed)
    for (check, resp, want) in [
        (TextCheck::None, &broken[..], Some(TextStatus::Unchecked)),
        (TextCheck::Utf8, latin, Some(TextStatus::Utf8)),
        (TextCheck::Utf8, &broken[..], None),
        (close, latin, None),
        (close, ascii, Some(TextStatus::Ascii)),
        (flag, latin, Some(TextStatus::NotAscii)),
        (flag, ascii, Some(TextStatus::Ascii)),
    ] {
        let mut r = HttpRig::new(
            &lab,
            HttpOptions {
                body_check: check,
                ..HttpOptions::default()
            },
        )?;
        let mut h = Collect::default();
        ask(&mut r)?;
        let rec = r.records(resp, &[])?.concat();
        let got = r
            .conn
            .feed(ConnId::from_parts(0, 0), &rec, &RxMeta::default(), &mut h);
        assert_eq!(got.is_ok(), want.is_some(), "{check:?} {got:?}");
        assert_eq!(h.text, want.into_iter().collect::<Vec<_>>(), "{check:?}");
        if want.is_none() {
            assert!(matches!(got, Err(Error::Http(_))), "{got:?}");
        }
    }
    Ok(())
}

/// Dropping a response would leave its request unanswered on a live
/// connection: `NonAscii::Skip` is a WebSocket policy, refused for bodies.
#[test]
fn the_body_check_refuses_skip() -> R {
    let lab = Lab::new()?;
    let got = HttpRig::new(
        &lab,
        HttpOptions {
            body_check: TextCheck::Ascii(NonAscii::Skip),
            ..HttpOptions::default()
        },
    );
    let e = got.err().ok_or("Skip accepted")?;
    assert!(e.to_string().contains("Skip"), "{e}");
    Ok(())
}

/// Chunk framing the old parser accepted is still accepted, cut at every
/// offset; what it refused is still refused.
#[test]
fn chunk_framing_is_accepted_and_refused_as_before() -> R {
    let lab = Lab::new()?;
    let ok: &[&[u8]] = &[
        b"5;name=value\r\nhello\r\n0\r\n\r\n",
        b"5 ;ext\r\nhello\r\n0\r\n\r\n",
        b"5\t\r\nhello\r\n0\r\n\r\n",
        b"5\r\nhello\r\n0\r\nA: 1\r\nB: 2\r\n\r\n",
        b"0005\r\nhello\r\n000\r\n\r\n",
        b"5\r\nhe\rlo\r\n0\r\n\r\n",
    ];
    let bad: &[&[u8]] = &[
        b"5\r\nhelloXX0\r\n\r\n",
        b"ffffffffffffffff\r\nx",
        b"5x\r\nhello\r\n0\r\n\r\n",
        b"5\r\rhello\r\n0\r\n\r\n",
        b"5 5\r\nhello\r\n0\r\n\r\n",
    ];
    let head = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
    for (cases, accept) in [(ok, true), (bad, false)] {
        for enc in cases {
            let mut resp = head.to_vec();
            resp.extend_from_slice(enc);
            for cut in 1..resp.len() {
                let mut r = rig(&lab, 1 << 20)?;
                let mut h = Collect::default();
                ask(&mut r)?;
                let got = r.feed_records(&resp, &[cut], &mut h);
                let enc_s = String::from_utf8_lossy(enc);
                if accept {
                    got.map_err(|e| format!("{enc_s:?} cut {cut}: {e}"))?;
                    let want: &[u8] = if enc.starts_with(b"5\r\nhe\r") {
                        b"he\rlo"
                    } else {
                        b"hello"
                    };
                    assert_eq!(h.responses.len(), 1, "{enc_s:?} cut {cut}");
                    assert_eq!(h.responses[0].1, want, "{enc_s:?} cut {cut}");
                } else {
                    assert!(h.responses.is_empty(), "{enc_s:?} cut {cut}");
                    let e = got
                        .err()
                        .ok_or_else(|| format!("{enc_s:?} cut {cut} accepted"))?;
                    assert!(e.to_string().starts_with("http:"), "{enc_s:?}: {e}");
                }
            }
        }
    }
    Ok(())
}

/// A non-ASCII byte anywhere in a header value is flagged, at every offset
/// of a 40-byte value (the ASCII scan reads 16 bytes per step).
#[test]
fn a_non_ascii_byte_at_any_offset_of_the_head_is_flagged() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, 1 << 20)?;
    let mut h = Collect::default();
    for at in 0..40 {
        let mut value = vec![b'v'; 40];
        value[at] = 0x80;
        let mut resp = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nX-V: ".to_vec();
        resp.extend_from_slice(&value);
        resp.extend_from_slice(b"\r\n\r\n");
        ask(&mut r)?;
        r.feed_records(&resp, &[], &mut h)?;
    }
    assert!(h.responses.iter().all(|g| !g.3));
    assert_eq!(r.conn.stats().non_ascii_heads, 40);
    Ok(())
}

/// Pipelined responses back to back: a chunked body still arriving ends
/// mid-record and the next responses (a `Content-Length` one, a one-chunk
/// one) follow in the same record. Cut into three records at many pairs of
/// offsets, every response arrives whole, with its own headers, in order.
#[test]
fn a_streamed_chunked_body_followed_by_more_responses_in_the_same_record() -> R {
    let lab = Lab::new()?;
    let mut r = rig(&lab, 1 << 20)?;
    let first = body_of(1500);
    let (mut stream, _) = chunked(&first, &[700, 1, 300, 499], "X-Sum: 1\r\n");
    let second = b"0123456789";
    stream.extend_from_slice(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nX-Kind: length\r\n\r\n");
    stream.extend_from_slice(second);
    let third = body_of(40);
    let (single, _) = chunked(&third, &[40], "");
    stream.extend_from_slice(&single);
    let len = stream.len();
    let mut cuts = Vec::new();
    for x in 1..len {
        cuts.push(vec![x]);
    }
    for x in (1..len).step_by(7) {
        for y in (x + 1..len).step_by(11) {
            cuts.push(vec![x, y]);
        }
    }
    for cut in &cuts {
        let mut h = Collect::default();
        for _ in 0..3 {
            ask(&mut r)?;
        }
        r.feed_records(&stream, cut, &mut h)?;
        assert_eq!(h.responses.len(), 3, "cuts {cut:?}");
        assert_eq!(h.responses[0].1, first, "cuts {cut:?}");
        assert_eq!(
            h.responses[0].2[1],
            ("X-Kind".to_owned(), b"chunked".to_vec())
        );
        assert_eq!(h.responses[1].1, second, "cuts {cut:?}");
        assert_eq!(
            h.responses[1].2[1],
            ("X-Kind".to_owned(), b"length".to_vec())
        );
        assert_eq!(h.responses[2].1, third, "cuts {cut:?}");
        assert_eq!(r.conn.in_flight(), 0, "cuts {cut:?}");
    }
    Ok(())
}
