//! Every hot-path call of the core allocates nothing once built: stamping and
//! decoding a message, pacing, recording latency, spikes and stalls, loss
//! tracking and deframing. Its own test binary, for the counting allocator.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::frame::{Deframer, encode};
use ergon_latency_core::hist::PointLatency;
use ergon_latency_core::loss::LossTracker;
use ergon_latency_core::message::{Kind, MESSAGE_LEN, Message, echo_in_place};
use ergon_latency_core::pace::Pacer;
use ergon_latency_core::row::{Spikes, Stalls};

type R = Result<(), Box<dyn std::error::Error>>;

struct Counting;

thread_local! {
    /// This thread's allocations: tests running in parallel do not count
    /// each other's.
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    // A const-initialised `Cell` has no destructor, so this never fails;
    // `try_with` keeps the allocator from panicking regardless.
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
}

fn allocs() -> u64 {
    ALLOCS.with(Cell::get)
}

// SAFETY: forwards to the system allocator; only adds a thread-local counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count();
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn the_counter_sees_an_allocation() {
    let before = allocs();
    let v = std::hint::black_box(vec![1u8; 32]);
    assert!(allocs() > before);
    drop(v);
}

#[test]
fn the_hot_path_allocates_nothing() -> R {
    const N: u64 = 20_000;
    let clock = TscClock::new();
    let start = clock.raw();
    let mut pacer = Pacer::new(&clock, start, 1_000_000_000, 8, 100, N)?;
    let mut latency = PointLatency::new()?;
    let mut loss = LossTracker::new(8, 1024)?;
    let mut spikes = Spikes::new(0, 1024);
    let mut stalls = Stalls::new(0, 1024, start);
    let mut deframer = Deframer::new(256)?;
    let mut wire = [0u8; MESSAGE_LEN + 4];
    let mut sink = 0u64;

    let before = allocs();
    while let Some(slot) = pacer.poll(u64::MAX) {
        let now = clock.raw();
        let ping = Message {
            seq: slot.seq,
            send_tsc: now,
            intended_tsc: slot.intended_tsc,
            stream: slot.stream,
            kind: Kind::Ping,
        };
        let mut buf = ping.to_bytes();
        pacer.sent(&clock, &slot, now)?;
        echo_in_place(&mut buf)?;
        let n = encode(&buf, &mut wire)?;
        let mut rest = &wire[..n];
        while !rest.is_empty() {
            let took = deframer.push(rest)?;
            rest = &rest[took..];
            while let Some(frame) = deframer.next_frame()? {
                let echo = Message::decode(frame)?;
                let recv = clock.raw();
                latency.record(&clock, &echo, recv);
                loss.on_message(echo.stream, echo.seq)?;
                spikes.record(
                    echo.stream,
                    clock.delta_ns(start, echo.intended_tsc),
                    clock.delta_ns(echo.intended_tsc, recv),
                    echo.seq,
                );
                stalls.pass(&clock, recv);
                sink = sink.wrapping_add(echo.seq);
            }
        }
    }
    let owed = pacer.finish(clock.raw());
    for k in owed {
        latency.record_owed(&clock, pacer.slot(k).intended_tsc, None, clock.raw());
    }
    let during = allocs() - before;
    std::hint::black_box(sink);
    assert_eq!(during, 0, "{during} allocations over {N} messages");
    assert_eq!(pacer.stats().sent, N + 100);
    assert_eq!(latency.from_intended.summary().count, N + 100);
    Ok(())
}
