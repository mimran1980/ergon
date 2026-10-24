//! The WebSocket receive path allocates nothing per message once warm: kept,
//! skipped, held as fragments, reassembled by copy, with Pings answered.
//! Its own test binary, for the counting allocator. Every platform.
#![allow(missing_docs, unsafe_code, clippy::cast_possible_truncation)]

mod support;
mod ws_rig;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use ergon_io::ws::{OP_BINARY, OP_CONT, OP_PING, OP_TEXT};
use ergon_io::{
    ConnId, Flow, Fragments, Handler, Interest, NonAscii, RxMeta, TextCheck, WsHandler, WsOptions,
};
use support::{Lab, R, frame};
use ws_rig::{WsRig, fragmented};

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);

// SAFETY: forwards to the system allocator; only adds a relaxed counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Keeps text, takes binary as fragments, skips what starts with `s`; sums
/// what it sees without allocating.
#[derive(Default)]
struct Sum {
    messages: u64,
    bytes: u64,
}

impl Handler for Sum {}

impl WsHandler for Sum {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.messages += 1;
        self.bytes += payload.len() as u64;
    }

    fn on_message_start(
        &mut self,
        _: ConnId,
        op: u8,
        first: &[u8],
        _: bool,
        _: &RxMeta,
    ) -> Interest {
        match (op, first.first()) {
            (_, Some(b's')) => Interest::Skip,
            (OP_BINARY, _) => Interest::Fragments,
            _ => Interest::Keep,
        }
    }

    fn on_fragments(&mut self, _: ConnId, _: u8, m: &Fragments<'_>, _: &RxMeta, _: u16) -> Flow {
        self.messages += 1;
        self.bytes += m.iter().map(|s| s.len() as u64).sum::<u64>();
        Flow::Continue
    }
}

/// One round, cut once mid-frame: a kept and a skipped text message, the
/// same fragmented, and a binary message held as fragments with a Ping
/// inside. Three delivered, two skipped.
fn round() -> R<(Vec<u8>, Vec<usize>)> {
    let kept: Vec<u8> = (0usize..900).map(|i| b'a' + (i % 26) as u8).collect();
    let mut skipped = b"s".to_vec();
    skipped.extend_from_slice(&kept);
    let bin: Vec<u8> = (0usize..3000).map(|i| (i % 251) as u8).collect();
    let mut s = frame(OP_TEXT, true, &kept)?;
    s.extend_from_slice(&frame(OP_TEXT, true, &skipped)?);
    s.extend_from_slice(&fragmented(OP_TEXT, &kept, 3)?);
    s.extend_from_slice(&fragmented(OP_TEXT, &skipped, 4)?);
    let cut = s.len() + 700;
    s.extend_from_slice(&frame(OP_BINARY, false, &bin[..1000])?);
    s.extend_from_slice(&frame(OP_PING, true, b"hb")?);
    s.extend_from_slice(&frame(OP_CONT, true, &bin[1000..])?);
    Ok((s, vec![cut]))
}

#[test]
fn the_steady_state_receive_path_does_not_allocate() -> R {
    let lab = Lab::new()?;
    let o = WsOptions {
        ping_interval_ms: 0,
        text_check: TextCheck::Utf8,
        ..WsOptions::default()
    };
    let mut r = WsRig::new(&lab, &o)?;
    let (stream, cuts) = round()?;
    let rounds = 200;
    // Seal everything first: forging records allocates, receiving must not.
    let mut reads = Vec::new();
    for _ in 0..rounds {
        reads.extend(r.records(&stream, &cuts)?);
    }
    let mut h = Sum::default();
    let warm = reads.len() / rounds;
    for rec in &reads[..warm] {
        r.feed(rec, &mut h)?;
    }
    r.conn.clear_tx();
    let before = ALLOCS.load(Ordering::Relaxed);
    for (i, rec) in reads[warm..].iter().enumerate() {
        r.feed(rec, &mut h)?;
        if i % warm == 0 {
            // The driver sends and empties the queue each poll.
            r.conn.clear_tx();
        }
    }
    let after = ALLOCS.load(Ordering::Relaxed);
    assert_eq!(after - before, 0, "allocations on the receive path");
    assert_eq!(h.messages, 3 * rounds as u64);
    let s = r.conn.stats();
    assert_eq!(
        (s.skipped, s.frag_fallbacks, s.pings),
        (2 * rounds as u64, 0, rounds as u64)
    );
    Ok(())
}

/// Streams every fragmented message and keeps the rest; sums without
/// allocating.
#[derive(Default)]
struct StreamSum {
    fragments: u64,
    messages: u64,
    bytes: u64,
}

impl Handler for StreamSum {}

impl WsHandler for StreamSum {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.messages += 1;
        self.bytes += payload.len() as u64;
    }

    fn on_message_start(&mut self, _: ConnId, _: u8, _: &[u8], _: bool, _: &RxMeta) -> Interest {
        Interest::Stream
    }

    fn on_fragment(
        &mut self,
        _: ConnId,
        _: u8,
        part: &[u8],
        _: u32,
        fin: bool,
        _: &RxMeta,
    ) -> Flow {
        self.fragments += 1;
        self.messages += u64::from(fin);
        self.bytes += part.len() as u64;
        Flow::Continue
    }
}

/// Streamed fragments under every text check, ASCII and not, with a Ping
/// inside: no allocation once warm.
#[test]
fn streaming_and_every_text_check_do_not_allocate() -> R {
    let lab = Lab::new()?;
    let checks = [
        TextCheck::None,
        TextCheck::Utf8,
        TextCheck::Ascii(NonAscii::DeliverUnchecked),
        TextCheck::Ascii(NonAscii::Skip),
    ];
    for check in checks {
        let o = WsOptions {
            ping_interval_ms: 0,
            text_check: check,
            ..WsOptions::default()
        };
        let mut r = WsRig::new(&lab, &o)?;
        let (stream, cuts) = round()?;
        let mut mixed = stream.clone();
        let text = "prix 3€ ".repeat(40);
        mixed.extend_from_slice(&fragmented(OP_TEXT, text.as_bytes(), 5)?);
        let rounds = 100;
        let mut reads = Vec::new();
        for _ in 0..rounds {
            reads.extend(r.records(&mixed, &cuts)?);
        }
        let mut h = StreamSum::default();
        let warm = reads.len() / rounds;
        for rec in &reads[..warm] {
            r.feed(rec, &mut h)?;
        }
        r.conn.clear_tx();
        let before = ALLOCS.load(Ordering::Relaxed);
        for (i, rec) in reads[warm..].iter().enumerate() {
            r.feed(rec, &mut h)?;
            if i % warm == 0 {
                r.conn.clear_tx();
            }
        }
        let after = ALLOCS.load(Ordering::Relaxed);
        assert_eq!(
            after - before,
            0,
            "{check:?}: allocations on the receive path"
        );
        assert!(h.fragments > 0 && h.messages > 0, "{check:?}");
    }
    Ok(())
}
