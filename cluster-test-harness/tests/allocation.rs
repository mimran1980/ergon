#![allow(missing_docs)]
#![allow(
    unsafe_code,
    reason = "a counting global allocator needs `unsafe impl GlobalAlloc`"
)]
//! A warm session's egress polls and ingress claims allocate nothing on the
//! polling thread. A fake cluster on an in-process driver plays the leader.
//!
//! Only Rust allocations on the measuring thread count: Aeron's C `malloc`
//! and the drivers' own threads are outside the counter.

#[allow(dead_code, reason = "these tests use only the own-driver launcher")]
mod common;
#[allow(dead_code, reason = "each test file drives part of the fake cluster")]
mod fake;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::error::Error;
use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};

use ergo_aeron_cluster::cluster_codec_types::SessionMessageHeaderDecoder;
use ergo_aeron_cluster::egress::{ControlledAction, ControlledEgressListener, EgressListener};
use ergo_aeron_cluster::{AeronCluster, IngressSessionDecorator, SessionBuilder};
use rusteron_client::AeronExclusivePublication;
use serial_test::serial;

use common::launch_own_driver;
use fake::{CLIENT_EGRESS, CLUSTER_SESSION_ID, FakeCluster, MEMBER_INGRESS, wait_until};

thread_local! {
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `try_with`: a thread being torn down has no thread-locals left.
        let _ = MEASURING.try_with(|measuring| {
            if measuring.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: forwards the caller's layout unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System.alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Runs `f`, returning its result and the allocations it made on this thread.
fn measured<T>(f: impl FnOnce() -> T) -> (T, u64) {
    ALLOCATIONS.with(|count| count.set(0));
    MEASURING.with(|measuring| measuring.set(true));
    let out = f();
    MEASURING.with(|measuring| measuring.set(false));
    (out, ALLOCATIONS.with(Cell::get))
}

/// Messages per measured round: one 64k egress term holds them all.
const MESSAGES: usize = 64;
const PAYLOAD: &[u8; 24] = b"allocation-free-hot-path";

#[derive(Default)]
struct Count(usize);

impl EgressListener for Count {
    fn on_message(&mut self, _: i64, _: i64, buffer: &[u8]) {
        if buffer == PAYLOAD {
            self.0 += 1;
        }
    }
}

#[derive(Default)]
struct ControlledCount(usize);

impl ControlledEgressListener for ControlledCount {
    fn on_message(&mut self, _: i64, _: i64, buffer: &[u8]) -> ControlledAction {
        if buffer == PAYLOAD {
            self.0 += 1;
        }
        ControlledAction::Continue
    }
}

/// The leader sends `MESSAGES` session messages to the client.
fn send_egress(egress: &AeronExclusivePublication) -> Result<(), Box<dyn Error>> {
    let mut decorator = IngressSessionDecorator::new();
    decorator
        .cluster_session_id(CLUSTER_SESSION_ID)
        .leadership_term_id(1);
    for _ in 0..MESSAGES {
        wait_until(
            || decorator.offer_exclusive(egress, PAYLOAD).is_ok(),
            "an egress offer",
        )?;
    }
    Ok(())
}

/// Polls with `poll` until `delivered` reaches `MESSAGES`, returning the
/// allocations the polls made.
fn poll_all(
    mut poll: impl FnMut() -> Result<(), Box<dyn Error>>,
    delivered: impl Fn() -> usize,
) -> Result<u64, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut allocations = 0;
    while delivered() < MESSAGES {
        let (polled, allocated) = measured(&mut poll);
        allocations += allocated;
        polled?;
        if Instant::now() >= deadline {
            return Err(format!("{} of {MESSAGES} messages polled", delivered()).into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(delivered(), MESSAGES);
    Ok(allocations)
}

fn poll_round(
    client: &mut AeronCluster,
    egress: &AeronExclusivePublication,
) -> Result<u64, Box<dyn Error>> {
    send_egress(egress)?;
    let delivered = Cell::new(0);
    let mut count = Count::default();
    poll_all(
        || {
            client.poll(&mut count, 10)?;
            delivered.set(count.0);
            Ok(())
        },
        || delivered.get(),
    )
}

fn controlled_round(
    client: &mut AeronCluster,
    egress: &AeronExclusivePublication,
) -> Result<u64, Box<dyn Error>> {
    send_egress(egress)?;
    let delivered = Cell::new(0);
    let mut count = ControlledCount::default();
    poll_all(
        || {
            client.controlled_poll_egress(&mut count, 10)?;
            delivered.set(count.0);
            Ok(())
        },
        || delivered.get(),
    )
}

/// `MESSAGES` claims and commits, then the member's count of them.
fn claim_round(client: &mut AeronCluster, cluster: &FakeCluster) -> Result<u64, Box<dyn Error>> {
    let mut allocations = 0;
    for _ in 0..MESSAGES {
        let (committed, allocated) = measured(|| -> Result<i64, Box<dyn Error>> {
            let mut claim = client.try_claim(PAYLOAD.len())?;
            claim.payload_mut().copy_from_slice(black_box(PAYLOAD));
            Ok(claim.commit()?)
        });
        allocations += allocated;
        assert!(committed? > 0);
    }
    let mut received = 0;
    wait_until(
        || {
            let _ = cluster.members[0].poll_fn(
                |data, _| {
                    if let Ok(header) = SessionMessageHeaderDecoder::decode(data, 0)
                        && header.cluster_session_id() == CLUSTER_SESSION_ID
                        && data.get(AeronCluster::SESSION_HEADER_LENGTH..)
                            == Some(PAYLOAD.as_slice())
                    {
                        received += 1;
                    }
                },
                MESSAGES,
            );
            received >= MESSAGES
        },
        "the claimed messages at the leader",
    )?;
    assert_eq!(received, MESSAGES);
    Ok(allocations)
}

/// A warm session polls egress (plain and controlled) and claims ingress
/// without a heap allocation on its thread.
#[test]
#[serial]
fn warm_poll_and_claim_do_not_allocate() -> Result<(), Box<dyn Error>> {
    // The counter must see an allocation, or a zero below proves nothing.
    let (_, allocated) = measured(|| black_box(Vec::<u8>::with_capacity(1)));
    assert!(allocated > 0, "the counting allocator counted nothing");

    let client_driver = launch_own_driver("allocation-client")?;
    let cluster = FakeCluster::launch()?;
    let builder = SessionBuilder::default()
        .ingress_channel(MEMBER_INGRESS[0])?
        .egress_channel(CLIENT_EGRESS)?
        .message_timeout(Duration::from_secs(10))?;
    let (mut client, egress) = cluster.connect(builder, &client_driver.dir)?;

    // Warm-up: the first round of each path may settle lazy state.
    poll_round(&mut client, &egress)?;
    controlled_round(&mut client, &egress)?;
    claim_round(&mut client, &cluster)?;

    assert_eq!(
        poll_round(&mut client, &egress)?,
        0,
        "AeronCluster::poll allocated"
    );
    assert_eq!(
        controlled_round(&mut client, &egress)?,
        0,
        "AeronCluster::controlled_poll_egress allocated"
    );
    assert_eq!(
        claim_round(&mut client, &cluster)?,
        0,
        "AeronCluster::try_claim + commit allocated"
    );
    client.close()?;
    Ok(())
}
