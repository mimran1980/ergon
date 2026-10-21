//! Time sources used generically: the calibrated clocks, and the manual and
//! stepped ones that make a test or a replay deterministic, all behind one
//! trait with no dynamic dispatch; the wall-clock offset; and the
//! calibration's parameters in their C layout.
#![allow(missing_docs)]

use ergon_clock::advanced::{
    Clock, CounterClock, MonotonicClock, PARAMS_VERSION, Params, Policy, SteppedClock, Trust,
    WallOffset, calibrate, monotonic_ns, realtime_ns,
};
use ergon_clock::{Config, Domain, Error, ManualClock, TimeSource, elapsed_ns};
use std::mem::{align_of, offset_of, size_of};

type R = Result<(), Box<dyn std::error::Error>>;

// ANCHOR: generic
/// What a component written against the trait sees: the elapsed time of a
/// piece of work, statically dispatched.
fn timed<C: TimeSource>(clock: &C, work: impl FnOnce()) -> u64 {
    let start = clock.now_ns_begin();
    work();
    elapsed_ns(start, clock.now_ns_ordered())
}

/// A manual clock reads what it was set to, and is advanced by hand: a
/// deterministic test of time-dependent code.
#[test]
fn a_manual_clock_reads_what_it_is_set_to() {
    let clock = ManualClock::new(1_000);
    assert_eq!(clock.now_ns(), 1_000);
    assert_eq!(clock.now_ns_ordered(), 1_000);
    let elapsed = timed(&clock, || clock.advance(250));
    assert_eq!(elapsed, 250);
    clock.set(5_000);
    assert_eq!(clock.now_ns(), 5_000);
}
// ANCHOR_END: generic

/// A stepped clock advances a fixed step at every read (the C transport's
/// per-thread step hook): each read is one step after the last.
#[test]
fn a_stepped_clock_advances_a_step_a_read() {
    let clock = SteppedClock::new(100, 7);
    assert_eq!(
        (clock.now_ns(), clock.now_ns(), clock.now_ns()),
        (100, 107, 114)
    );
    assert_eq!(timed(&clock, || {}), 7);
}

/// Elapsed time never goes negative: a start taken after `now` (another
/// core's read, a clock set back) is no time at all, not a wrapped one.
#[test]
fn elapsed_time_saturates_at_zero() {
    let clock = ManualClock::new(1_000);
    assert_eq!(clock.since_ns(2_000), 0);
    assert_eq!(ergon_clock::elapsed_ns(2_000, 1_000), 0);
    assert_eq!(ergon_clock::elapsed_ns(1_000, 2_500), 1_500);
}

// ANCHOR: time_sources
/// The calibrated clock, the counter alone and the fallback alone are all
/// time sources: a component generic over the trait runs on each.
#[test]
fn the_calibrated_clocks_are_time_sources() -> R {
    let clock = calibrate(&Config {
        trust: Trust::CrossCheck,
        ..Config::default()
    })?
    .clock;
    let sleep = || std::thread::sleep(std::time::Duration::from_millis(2));
    assert!(timed(&clock, sleep) >= 1_900_000);
    match clock {
        Clock::Counter(counter) => assert!(timed(&counter, sleep) >= 1_900_000),
        Clock::Monotonic(monotonic) => assert!(timed(&monotonic, sleep) >= 1_900_000),
    }
    Ok(())
}
// ANCHOR_END: time_sources

// ANCHOR: wall_offset
/// The wall-clock offset maps the clock to `CLOCK_REALTIME` (the domain of
/// kernel and NIC timestamps) within its stated error, which is never zero:
/// it includes both clocks' resolutions.
#[test]
fn the_wall_offset_maps_to_realtime() -> R {
    for policy in [Policy::Auto, Policy::Monotonic] {
        let clock = calibrate(&Config {
            policy,
            trust: Trust::CrossCheck,
            ..Config::default()
        })?
        .clock;
        let offset = WallOffset::measure(&clock);
        assert!(offset.error_ns() > 0);
        let wall = offset.to_realtime_ns(clock.now_ns());
        let kernel = realtime_ns();
        // the reads are apart by a few hundred ns; NTP may slew in between
        let apart = i128::from(wall) - i128::from(kernel);
        assert!(
            apart.unsigned_abs() <= u128::from(offset.error_ns()) + 100_000,
            "{policy:?}: {wall} against {kernel}, error {}",
            offset.error_ns()
        );
        assert_eq!(
            offset.from_realtime_ns(offset.to_realtime_ns(123_456_789)),
            123_456_789
        );
    }
    Ok(())
}
// ANCHOR_END: wall_offset

/// The fallback's resolution is the step its reads advance by, as a reader
/// sees it, not `clock_getres`'s answer: on Linux that is the timer
/// resolution, a jiffy (1 ms) on a kernel without high-resolution timers
/// whose clock reads advance by nanoseconds. That step is the clock's tick,
/// or a read's cost where that is longer, and the cost is that of the
/// reads the clock was made with: a few of the smallest steps seen in a
/// million reads after it, the step those usually see, or, where its
/// thread was slow then (in a loaded Docker VM, 6% of starts saw no read
/// of a 24 MHz counter closer than 4 ticks to the last), at most a
/// microsecond. A jiffy is a thousand of those.
#[test]
fn the_fallbacks_resolution_is_the_step_its_reads_advance_by() -> R {
    let clock = MonotonicClock::new(Domain::Monotonic)?;
    let mut steps = Vec::with_capacity(1_000_000);
    let mut previous = monotonic_ns();
    for _ in 0..1_000_000 {
        let now = monotonic_ns();
        if now > previous {
            steps.push(now - previous);
        }
        previous = now;
    }
    steps.sort_unstable();
    let (Some(&smallest), Some(&usual)) = (steps.first(), steps.get(steps.len() / 2)) else {
        return Err("the monotonic clock never advanced".into());
    };
    assert!(
        (1..=smallest.saturating_mul(4).max(usual).max(1_000)).contains(&clock.resolution_ns()),
        "resolution {} ns; reads advance by {smallest} ns at least, {usual} ns usually",
        clock.resolution_ns()
    );
    Ok(())
}

/// The C layout the DPDK transport reads: 48 bytes, 8-aligned, fields
/// where its header expects them.
#[test]
fn the_parameters_have_a_c_layout() {
    assert_eq!(size_of::<Params>(), 48);
    assert_eq!(align_of::<Params>(), 8);
    assert_eq!(offset_of!(Params, version), 0);
    assert_eq!(offset_of!(Params, kind), 4);
    assert_eq!(offset_of!(Params, domain), 8);
    assert_eq!(offset_of!(Params, base_ticks), 16);
    assert_eq!(offset_of!(Params, base_ns), 24);
    assert_eq!(offset_of!(Params, mult), 32);
    assert_eq!(offset_of!(Params, hz), 40);
}

/// A process adopts another's calibration from its parameters (the
/// transport and the application on one timebase), and gets the same
/// conversion; parameters it cannot use are refused, saying why.
#[test]
fn a_calibration_travels_as_parameters() -> R {
    let calibrated = calibrate(&Config {
        trust: Trust::CrossCheck,
        domain: Domain::Realtime,
        ..Config::default()
    })?;
    let params = calibrated.clock.params();
    assert_eq!(params.version, PARAMS_VERSION);
    match calibrated.clock {
        Clock::Counter(counter) => {
            let adopted = CounterClock::from_params(&params)?;
            assert_eq!(adopted, counter);
            assert_eq!(adopted.domain(), Domain::Realtime);
            let t = counter.ticks();
            assert_eq!(adopted.to_ns(t), counter.to_ns(t));
        }
        Clock::Monotonic(_) => {
            assert!(matches!(
                CounterClock::from_params(&params),
                Err(Error::InvalidParams(_))
            ));
        }
    }
    let good = Params {
        version: PARAMS_VERSION,
        kind: 1,
        domain: 0,
        reserved: 0,
        base_ticks: 0,
        base_ns: 0,
        mult: 1 << 32,
        hz: 1_000_000_000,
    };
    for bad in [
        Params { version: 0, ..good },
        Params { kind: 0, ..good },
        Params { domain: 7, ..good },
        Params { mult: 0, ..good },
        Params {
            mult: 1 << 63,
            ..good
        },
        Params {
            hz: 2_000_000_000,
            ..good
        },
    ] {
        assert!(
            matches!(
                CounterClock::from_params(&bad),
                Err(Error::InvalidParams(_))
            ),
            "{bad:?}"
        );
    }
    Ok(())
}
