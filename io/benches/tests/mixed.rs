//! The many-connection traffic schedule (`replay_server --mode mixed`).

use io_bench::mixed::{Schedule, Shape};

const SHAPE: Shape = Shape {
    slots: 100,
    trickle_min_us: 1_000,
    trickle_max_us: 10_000,
    burst_every_us: 2_000,
    burst_slots: 10,
    burst_len: 10,
    seed: 42,
};

/// The schedule's first `ms` milliseconds.
fn first(shape: Shape, ms: i64) -> Vec<io_bench::mixed::Send> {
    Schedule::new(shape)
        .take_while(|s| s.at_ns < ms * 1_000_000)
        .collect()
}

#[test]
fn the_same_seed_gives_the_same_traffic_and_time_never_goes_back() {
    let a = first(SHAPE, 200);
    assert_eq!(a, first(SHAPE, 200));
    assert_ne!(a, first(Shape { seed: 7, ..SHAPE }, 200));
    assert!(a.windows(2).all(|w| w[0].at_ns <= w[1].at_ns));
}

#[test]
fn every_slot_trickles_and_bursts_hit_distinct_slots() {
    // One second: each slot at least once (the longest mean is 10 ms), bursts
    // every 2 ms of 10 slots x 10 messages.
    let s = first(SHAPE, 1_000);
    let mut per_slot = [0usize; 100];
    for m in &s {
        per_slot[m.slot] += 1;
    }
    assert!(per_slot.iter().all(|&n| n >= 50), "{per_slot:?}");
    // The burst at 2 ms: 100 messages at one instant, ten per slot.
    let at: Vec<_> = s.iter().filter(|m| m.at_ns == 2_000_000).collect();
    assert_eq!(at.len(), 100);
    let mut slots: Vec<usize> = at.iter().map(|m| m.slot).collect();
    slots.dedup();
    assert_eq!(slots.len(), 10, "ten distinct slots, ten messages each");
    // Rates: about 18,000 trickle messages a second (100 slots, mean 5.5 ms)
    // and 50,000 burst messages.
    assert!((60_000..76_000).contains(&s.len()), "{}", s.len());
}

#[test]
fn trickle_means_stay_in_range() {
    let sched = Schedule::new(SHAPE);
    assert!(
        sched
            .trickle_means_ns()
            .iter()
            .all(|&m| (1_000_000..=10_000_000).contains(&m))
    );
}
