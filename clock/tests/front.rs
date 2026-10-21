//! The front API: `calibrate` gives an owner clock on UTC by default,
//! nanoseconds since boot on request, followers for other threads, and a
//! health check by return value.
#![allow(missing_docs)]

use ergon_clock::advanced::{self, KIND_MONOTONIC, Policy, monotonic_ns, realtime_ns};
use ergon_clock::{Config, Domain, Follower, ManualClock, TimeSource, calibrate, elapsed_ns};
use std::time::{Duration, Instant};

type R = Result<(), Box<dyn std::error::Error>>;

/// 1 ms, ns.
const MS: i128 = 1_000_000;

/// Whether `read` (ns) lies within 1 ms of `[before, after]`.
fn near(before: i128, read: i128, after: i128) -> bool {
    (before - MS..=after + MS).contains(&read)
}

/// The default clock counts nanoseconds since 1970, UTC: it reads what
/// `CLOCK_REALTIME` reads.
#[test]
fn the_default_clock_counts_utc_nanoseconds() -> R {
    let clock = calibrate(&Config::default())?;
    assert_eq!(clock.report().domain, Domain::Realtime);
    let before = realtime_ns();
    let now = clock.now_ns();
    let ordered = clock.now_ns_ordered();
    let after = realtime_ns();
    for read in [now, ordered] {
        assert!(
            near(before.into(), read.into(), after.into()),
            "{read} against CLOCK_REALTIME {before}..{after}"
        );
    }
    // 2026 or later, in ns since 1970: not ns since boot
    assert!(now > 1_760_000_000_000_000_000, "{now}");
    Ok(())
}

/// Nanoseconds since boot are opt-in: `Domain::Monotonic` reads what the
/// platform's monotonic clock reads.
#[test]
fn nanoseconds_since_boot_are_opt_in() -> R {
    let clock = calibrate(&Config {
        domain: Domain::Monotonic,
        ..Config::default()
    })?;
    assert_eq!(clock.report().domain, Domain::Monotonic);
    let before = monotonic_ns();
    let now = clock.now_ns();
    let after = monotonic_ns();
    assert!(
        near(before.into(), now.into(), after.into()),
        "{now} against the monotonic clock {before}..{after}"
    );
    Ok(())
}

/// The UTC fallback is the monotonic clock plus a fixed offset, never a
/// read of `CLOCK_REALTIME` (which can step back): every read lies between
/// two monotonic reads plus the offset, exactly.
#[test]
fn a_utc_fallback_is_the_monotonic_clock_plus_an_offset() -> R {
    let clock = calibrate(&Config {
        policy: Policy::Monotonic,
        ..Config::default()
    })?;
    assert_eq!(clock.report().domain, Domain::Realtime);
    let p = advanced::segment(&clock).params();
    assert_eq!((p.kind, p.base_ticks, p.mult), (KIND_MONOTONIC, 0, 1 << 32));
    let off = p.base_ns;
    for _ in 0..100_000 {
        let m1 = monotonic_ns();
        let now = clock.now_ns();
        let m2 = monotonic_ns();
        assert!(
            m1.wrapping_add(off) <= now && now <= m2.wrapping_add(off),
            "{now} outside {m1}..{m2} + {off}"
        );
    }
    Ok(())
}

/// Another thread reads the owner's clock through a follower.
#[test]
fn a_follower_reads_the_owners_clock() -> R {
    let mut clock = calibrate(&Config::default())?;
    let follower = clock.follower();
    let (before, read, after) = std::thread::scope(|s| {
        s.spawn(move || {
            let before = realtime_ns();
            let read = follower.now_ns();
            (before, read, realtime_ns())
        })
        .join()
    })
    .map_err(|_| "the follower's thread panicked")?;
    assert!(near(before.into(), read.into(), after.into()), "{read}");
    assert!(elapsed_ns(read, clock.now_ns_ordered()) < 1_000_000_000);
    Ok(())
}

/// Code generic over `TimeSource` runs on a follower's thread, and in a
/// test on a manual clock.
#[test]
fn a_follower_is_a_time_source() -> R {
    fn age<T: TimeSource>(clock: &T, stamped_ns: u64) -> u64 {
        clock.since_ns(stamped_ns)
    }
    let mut clock = calibrate(&Config::default())?;
    let follower: Follower = clock.follower();
    let stamp = clock.now_ns();
    let on_thread = std::thread::scope(|s| s.spawn(move || age(&follower, stamp)).join())
        .map_err(|_| "the follower's thread panicked")?;
    assert!(on_thread < 1_000_000_000, "{on_thread}");
    let manual = ManualClock::new(1_000);
    manual.advance(500);
    assert_eq!(age(&manual, 1_000), 500);
    Ok(())
}

/// A clock just made is not stale: the calibration counts as its first
/// update (the fallback's base is boot, which must not read as one).
#[test]
fn a_fresh_fallback_clock_is_not_stale() -> R {
    let clock = calibrate(&Config {
        policy: Policy::Monotonic,
        ..Config::default()
    })?;
    let health = clock.health();
    assert!(health.utc);
    assert!(health.since_update_ns < 1_000_000_000, "{health:?}");
    assert_eq!(health.warning(), None, "{health:?}");
    Ok(())
}

/// Updated from a loop's idle time, the clock follows `CLOCK_REALTIME` and
/// never goes back. It reads the wall clock less the gap its health
/// reports (within 1 ms and the slew since the update): a host whose wall
/// clock steps (a VM's, resynced from its host) leaves a gap the clock
/// slews out at 0.5 ms a second, reported, never jumped.
#[test]
fn updates_keep_the_clock_on_utc_and_never_back() -> R {
    let mut clock = calibrate(&Config::default())?;
    let started = Instant::now();
    let mut last = clock.now_ns_ordered();
    let mut updates = 0_u32;
    let mut health = clock.health();
    while started.elapsed() < Duration::from_millis(2_500) {
        let now = clock.now_ns_ordered();
        assert!(now >= last, "back by {} ns", last - now);
        last = now;
        let next = clock.update();
        if next.since_update_ns < health.since_update_ns {
            updates += 1;
        }
        health = next;
    }
    assert!(updates >= 1, "no update in 2.5 s: {health:?}");
    assert!(health.since_update_ns < 1_500_000_000, "{health:?}");
    assert!(
        health.bound_ns >= health.gap_ns.unsigned_abs(),
        "{health:?}"
    );
    assert_eq!(
        health.warning().is_some(),
        health.gap_ns.unsigned_abs() >= 1_000_000,
        "{health:?}"
    );
    let before = realtime_ns();
    let now = clock.now_ns();
    let after = realtime_ns();
    let health = clock.health();
    let gap = i128::from(health.gap_ns);
    let slew = i128::from(health.since_update_ns / 2_000);
    assert!(
        near(
            i128::from(before) - gap - slew,
            now.into(),
            i128::from(after) - gap + slew
        ),
        "{now} against CLOCK_REALTIME {before}..{after}: {health:?}"
    );
    Ok(())
}

/// Without updates the health check says so, by return value: a clock
/// whose owner never calls `update` is reported stale after 5 s. Scripted
/// in the crate's unit tests; here only that a fresh clock's bound grows
/// with time and is never below its gap.
#[test]
fn the_health_bound_covers_the_gap() -> R {
    let clock = calibrate(&Config::default())?;
    let first = clock.health();
    std::thread::sleep(Duration::from_millis(20));
    let later = clock.health();
    assert!(later.since_update_ns > first.since_update_ns);
    assert!(later.bound_ns >= later.gap_ns.unsigned_abs());
    assert!(later.bound_ns >= first.bound_ns);
    Ok(())
}
