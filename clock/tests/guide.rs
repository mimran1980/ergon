//! The book's examples and the README's claims, run on every platform:
//! `book/src/clock/overview.md` includes `busy_loop`, `ordered_read`,
//! `calibrate` and `fake_time` by their anchors.
#![allow(missing_docs)]

use ergon_clock::advanced::discipline::{Discipline, DisciplineConfig, Phase};
use ergon_clock::advanced::share::{ClockFollower, ClockPublisher, SharedClock};
use ergon_clock::advanced::{self, KernelSource, Sampler, realtime_ns, realtime_res_ns};
use ergon_clock::{Config, Domain, ManualClock, Report, TimeSource, calibrate, elapsed_ns};
use std::hint::black_box;
use std::time::Duration;

type R = Result<(), Box<dyn std::error::Error>>;

// ANCHOR: busy_loop
/// A busy-spin loop: calibrate once at start-up, stamp each event with
/// `now_ns`, time its handling up to `now_ns_ordered` and keep the slowest,
/// and keep the clock on UTC from idle time. The event's stamp starts this
/// one because the question is how long from arrival to done; a measurement
/// of the work alone starts at `now_ns_begin`.
#[test]
fn a_busy_spin_loop() -> R {
    let mut clock = calibrate(&Config::default())?; // start-up only: 40-70 ms, plus a short time-sync probe (50 ms chronyd answer timeout; duration not measured)
    let mut slowest_ns = 0;
    let mut to_log = Vec::new(); // a logger thread prints these in a real program
    for turn in 0..10_000_u64 {
        if let Some(event) = poll(turn) {
            let received_ns = clock.now_ns(); // the event's stamp: UTC ns
            black_box(handle(black_box(event))); // the work, kept between the reads
            let took_ns = elapsed_ns(received_ns, clock.now_ns_ordered()); // waits for it
            slowest_ns = slowest_ns.max(took_ns);
        } else {
            let health = clock.update(); // idle: real work at most once a second
            // Log only where a sample was taken: a gap or a stale clock
            // lasts minutes, and `update` is called every idle turn.
            if health.sampled
                && let Some(warning) = health.warning()
            {
                to_log.push(warning); // never print here
            }
        }
    }
    for warning in &to_log {
        eprintln!("{}", warning.line()); // off the hot path
    }
    assert!(slowest_ns < 1_000_000_000, "{slowest_ns} ns");
    Ok(())
}

/// Stands in for the network: an event every tenth turn.
fn poll(turn: u64) -> Option<u64> {
    turn.is_multiple_of(10).then_some(turn)
}

/// Stands in for the work an event needs.
fn handle(event: u64) -> u64 {
    (0..event % 64).map(black_box).sum()
}
// ANCHOR_END: busy_loop

// ANCHOR: ordered_read
/// The end of a measurement must be ordered: 16 loads that miss the
/// caches, each waiting for the one before. The begin too: an unordered
/// read lets the work start before the stamp and shortens the interval.
#[test]
fn the_end_of_a_measurement_is_ordered() -> R {
    let clock = calibrate(&Config::default())?;
    let ring = ring(1 << 23); // 32 MB of slots in one random cycle
    let start_ns = clock.now_ns_begin();
    let slot = black_box(chase(&ring, black_box(start_ns))); // 16 cache misses
    let early_ns = elapsed_ns(start_ns, clock.now_ns()); // may run before they finish
    let full_ns = elapsed_ns(start_ns, clock.now_ns_ordered()); // waits for them
    assert!(full_ns >= early_ns && full_ns > 0, "{slot}: {full_ns}");
    Ok(())
}
// ANCHOR_END: ordered_read

/// One cycle through `n` slots in a random order (Sattolo's shuffle, a
/// fixed seed): each hop is a load that depends on the last and, over
/// 32 MB, misses the caches.
fn ring(n: u32) -> Vec<u32> {
    let mut ring: Vec<u32> = (0..n).collect();
    let mut x = 0x9e37_79b9_7f4a_7c15_u64;
    for i in (1..n).rev() {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let j = x.wrapping_mul(0x2545_f491_4f6c_dd1d) % u64::from(i);
        ring.swap(i as usize, usize::try_from(j).unwrap_or(0));
    }
    ring
}

/// 16 dependent loads from a slot `seed` picks, so none starts before the
/// read that gave the seed.
fn chase(ring: &[u32], seed: u64) -> usize {
    let len = u64::try_from(ring.len()).unwrap_or(u64::MAX).max(1);
    let mut i = usize::try_from(seed % len).unwrap_or(0);
    for _ in 0..16 {
        i = ring[i] as usize;
    }
    i
}

// ANCHOR: calibrate
/// Calibrate once at start-up, log what was chosen, give each other thread
/// a follower.
#[test]
fn calibrate_once_at_start_up() -> R {
    let mut clock = calibrate(&Config::default())?; // prints a line per problem
    println!("clock: {}", clock.report()); // one line: what was chosen and why
    for warning in clock.report().warnings() {
        println!("{}", warning.line()); // the same lines `calibrate` printed
    }
    // Another thread reads through its follower. No lock, no `Arc`.
    let follower = clock.follower();
    let stamp = std::thread::scope(|s| s.spawn(move || follower.now_ns()).join());
    assert!(stamp.is_ok());
    Ok(())
}

/// From housekeeping, every few seconds: a file read, never on the hot path.
fn counter_still_trusted(report: &Report) -> bool {
    match report.recheck() {
        Ok(()) => true,
        Err(why) => {
            eprintln!("clock: the kernel dropped the counter ({why}): calibrate again");
            false
        }
    }
}
// ANCHOR_END: calibrate

#[test]
fn the_recheck_passes_while_nothing_changed() -> R {
    let clock = calibrate(&Config::default())?;
    assert!(counter_still_trusted(clock.report()), "{}", clock.report());
    Ok(())
}

/// Event stamps, deadlines and ages: the plain (unordered) read.
#[test]
fn stamp_events_with_now_ns() -> R {
    let clock = calibrate(&Config::default())?;
    let received_ns = clock.now_ns(); // stamp the event: ns since 1970, UTC
    let deadline_ns = received_ns + 1_000_000_000; // answer within 1 s
    assert!(clock.now_ns() < deadline_ns);
    let age_ns = clock.since_ns(received_ns); // never negative
    assert!(age_ns < 1_000_000_000);
    Ok(())
}

/// How long a short section took: ordered reads at both ends, in raw
/// ticks, converted once at the end.
#[test]
fn time_a_short_section() -> R {
    let clock = calibrate(&Config::default())?;
    let raw = advanced::segment(&clock); // the fixed segment: raw ticks
    let start = raw.ticks_begin(); // waits for the work before it, and holds the work behind it
    let sum: u64 = (0..1_000_u64).map(black_box).sum(); // the work
    let end = raw.ticks_ordered(); // waits for the work to finish
    let took_ns = raw.ticks_to_ns(end.saturating_sub(start));
    assert!(black_box(sum) > 0 && took_ns < 1_000_000_000);
    Ok(())
}

/// A long section (well over a microsecond): both ends are still the
/// ordered reads. The begin's ordering costs a few nanoseconds and is not
/// the measurement here, but dropping it is still wrong.
#[test]
fn time_a_long_section() -> R {
    let clock = calibrate(&Config::default())?;
    let start_ns = clock.now_ns_begin();
    std::thread::sleep(Duration::from_millis(1)); // the work
    let took_ns = elapsed_ns(start_ns, clock.now_ns_ordered());
    assert!(took_ns >= 990_000, "{took_ns} ns");
    Ok(())
}

// ANCHOR: fake_time
/// Code that reads the time takes any `TimeSource`.
fn is_stale(clock: &impl TimeSource, stamped_ns: u64) -> bool {
    clock.since_ns(stamped_ns) > 5_000_000 // older than 5 ms
}

/// In a test: time moves only when the test moves it.
#[test]
fn test_with_a_manual_clock() {
    let clock = ManualClock::new(1_000);
    assert!(!is_stale(&clock, 1_000));
    clock.advance(5_000_001);
    assert!(is_stale(&clock, 1_000));
}

/// Live: the same function, given the calibrated clock.
#[test]
fn run_live_with_the_calibrated_clock() -> R {
    let clock = calibrate(&Config::default())?;
    assert!(!is_stale(&clock, clock.now_ns()));
    Ok(())
}
// ANCHOR_END: fake_time

/// The machinery under `Clock::update`, for code that keeps its own owner
/// thread: one owner disciplines a fixed segment and publishes it; every
/// other thread follows.
#[test]
fn discipline_one_clock_and_share_it() -> R {
    // nanoseconds since the UNIX epoch
    let config = Config {
        domain: Domain::Realtime,
        ..Config::default()
    };
    let mut clock = advanced::calibrate(&config)?.clock;
    let mut discipline = Discipline::new(&clock, DisciplineConfig::default())?;
    let mut sampler = Sampler::for_clock(&clock, realtime_res_ns()?);
    let shared = SharedClock::new(); // a `static` in a real program
    let mut publisher = ClockPublisher::new(&shared).ok_or("already has a publisher")?;

    // The owner: the start update before anything is stamped, then
    // `Phase::Running` once a second from housekeeping.
    for phase in [Phase::Start, Phase::Running] {
        let reader = clock; // the copy the sample reads through
        let mut source = KernelSource(&reader);
        let update = discipline.sample_and_apply(&mut clock, &mut sampler, &mut source, phase);
        publisher.publish(&clock);
        assert!(update.error_ns > 0);
    }

    // Any other thread: one acquire load and a compare a read.
    let owner = clock;
    let followed = std::thread::scope(|s| {
        s.spawn(|| {
            let mut follower = ClockFollower::new(&shared, owner);
            let before = realtime_ns();
            let wall_ns = follower.clock().now_ns(); // epoch ns, as published
            (before, wall_ns, realtime_ns(), follower.clock().params())
        })
        .join()
    });
    let (before, wall_ns, after, params) = followed.map_err(|_| "the follower panicked")?;
    assert_eq!(params, clock.params());
    // on the kernel's wall clock, within a millisecond
    let near = (i128::from(before) - 1_000_000)..=(i128::from(after) + 1_000_000);
    assert!(near.contains(&i128::from(wall_ns)), "{wall_ns}: {near:?}");
    Ok(())
}
