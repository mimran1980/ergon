//! The open-loop sender's schedule: when each slot is due, which stream it
//! goes to, and how late sends are counted and flagged.

use ergon_latency_core::Error;
use ergon_latency_core::clock::TscClock;
use ergon_latency_core::pace::{LATE_NS, PaceStats, Pacer};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn slot_k_goes_to_stream_k_mod_n_with_its_own_sequence() -> R {
    let clock = TscClock::new();
    let pacer = Pacer::new(&clock, 1_000, 1_000_000, 3, 0, 9)?;
    let slots: Vec<_> = (0..7).map(|k| pacer.slot(k)).collect();
    let streams: Vec<u32> = slots.iter().map(|s| s.stream).collect();
    let seqs: Vec<u64> = slots.iter().map(|s| s.seq).collect();
    assert_eq!(streams, [0, 1, 2, 0, 1, 2, 0]);
    assert_eq!(seqs, [0, 0, 0, 1, 1, 1, 2]);
    Ok(())
}

#[test]
fn the_schedule_is_start_plus_k_intervals_without_drift() -> R {
    let clock = TscClock::new();
    let start = 5_000;
    // 3 msg/s: an interval of a third of a second, which no whole number of
    // ticks is, so a per-slot rounding would drift
    let pacer = Pacer::new(&clock, start, 3, 1, 0, 3_000_001)?;
    let second = clock.ns_to_ticks(1_000_000_000);
    let due = pacer.due(3_000_000) - start;
    // a million seconds in, within a tick per second of the clock's own rate
    let want = second * 1_000_000;
    assert!(due.abs_diff(want) <= 1_000_000, "{due} vs {want}");
    assert_eq!(pacer.due(0), start);
    Ok(())
}

#[test]
fn nothing_is_due_before_its_time_and_poll_does_not_advance() -> R {
    let clock = TscClock::new();
    let pacer = Pacer::new(&clock, 1_000_000, 1_000, 1, 0, 2)?;
    assert!(pacer.poll(999_999).is_none());
    let slot = pacer.poll(1_000_000).ok_or("slot 0 due")?;
    assert_eq!((slot.index, slot.intended_tsc), (0, 1_000_000));
    assert_eq!(pacer.poll(1_000_000).map(|s| s.index), Some(0));
    Ok(())
}

#[test]
fn a_send_on_time_is_not_late_and_one_past_100_us_is() -> R {
    let clock = TscClock::new();
    let start = 10_000_000;
    let mut pacer = Pacer::new(&clock, start, 1_000, 1, 0, 3)?;
    let late = clock.ns_to_ticks(LATE_NS);
    let s0 = pacer.slot(0);
    pacer.sent(&clock, &s0, s0.intended_tsc + late / 2)?;
    let s1 = pacer.slot(1);
    pacer.sent(&clock, &s1, s1.intended_tsc + late * 2)?;
    let stats = pacer.stats();
    assert_eq!((stats.sent, stats.late), (2, 1));
    let lag = clock.delta_ns(s1.intended_tsc, s1.intended_tsc + late * 2);
    assert_eq!(stats.max_send_lag_ns, lag);
    Ok(())
}

#[test]
fn warm_up_sends_count_as_sent_but_never_as_late() -> R {
    let clock = TscClock::new();
    let mut pacer = Pacer::new(&clock, 0, 1_000, 1, 2, 1)?;
    let way_late = clock.ns_to_ticks(1_000_000_000);
    for k in 0..2 {
        let s = pacer.slot(k);
        assert!(!s.measured);
        pacer.sent(&clock, &s, s.intended_tsc + way_late)?;
    }
    let s = pacer.slot(2);
    assert!(s.measured);
    pacer.sent(&clock, &s, s.intended_tsc)?;
    let stats = pacer.stats();
    assert_eq!(
        stats,
        PaceStats {
            scheduled: 3,
            sent: 3,
            late: 0,
            max_send_lag_ns: 0,
            censored: 0
        }
    );
    assert!(pacer.done());
    assert!(pacer.poll(u64::MAX).is_none());
    Ok(())
}

#[test]
fn a_slot_sent_out_of_turn_is_refused() -> R {
    let clock = TscClock::new();
    let mut pacer = Pacer::new(&clock, 0, 1_000, 1, 0, 3)?;
    let s1 = pacer.slot(1);
    assert!(matches!(
        pacer.sent(&clock, &s1, 0),
        Err(Error::SlotOutOfTurn {
            expected: 0,
            got: 1
        })
    ));
    Ok(())
}

#[test]
fn more_than_one_percent_late_flags_the_point() {
    let base = PaceStats {
        scheduled: 1000,
        sent: 1000,
        late: 10,
        max_send_lag_ns: 0,
        censored: 0,
    };
    assert!(!base.flagged());
    assert!(PaceStats { late: 11, ..base }.flagged());
    assert!(
        !PaceStats {
            sent: 0,
            late: 0,
            ..base
        }
        .flagged()
    );
}

#[test]
fn slots_due_but_never_sent_by_the_end_are_censored() -> R {
    let clock = TscClock::new();
    let mut pacer = Pacer::new(&clock, 0, 1_000, 1, 1, 9)?;
    let s0 = pacer.slot(0);
    pacer.sent(&clock, &s0, 0)?;
    let s1 = pacer.slot(1);
    pacer.sent(&clock, &s1, s1.intended_tsc)?;
    // the run ends just after slot 4 was due: 2, 3 and 4 are owed, 5.. are not yet due
    let owed = pacer.finish(pacer.due(4) + 1);
    assert_eq!(owed, 2..5);
    assert_eq!(pacer.stats().censored, 3);
    Ok(())
}

#[test]
fn a_zero_rate_or_no_streams_is_refused() {
    let clock = TscClock::new();
    assert!(matches!(
        Pacer::new(&clock, 0, 0, 1, 0, 1),
        Err(Error::Schedule(_))
    ));
    assert!(matches!(
        Pacer::new(&clock, 0, 1, 0, 0, 1),
        Err(Error::Schedule(_))
    ));
}

#[test]
fn wait_busy_spins_until_the_slot_is_due() -> R {
    let clock = TscClock::new();
    let start = clock.raw() + clock.ns_to_ticks(2_000_000);
    let pacer = Pacer::new(&clock, start, 1_000, 1, 0, 1)?;
    let slot = pacer.wait(&clock).ok_or("a slot")?;
    assert!(clock.raw() >= slot.intended_tsc);
    assert_eq!(slot.index, 0);
    Ok(())
}

/// `with_burst(b)`: b consecutive slots share one due time and one stream,
/// with consecutive sequence numbers there; the average rate is unchanged
/// (group g is due when slot g × b would be without bursts).
#[test]
fn a_burst_sends_b_slots_at_once_to_one_stream() -> R {
    let clock = TscClock::new();
    let plain = Pacer::new(&clock, 1_000, 1_000, 2, 0, 12)?;
    let pacer = Pacer::new(&clock, 1_000, 1_000, 2, 0, 12)?.with_burst(4)?;
    let slots: Vec<_> = (0..12).map(|k| pacer.slot(k)).collect();
    let streams: Vec<u32> = slots.iter().map(|s| s.stream).collect();
    let seqs: Vec<u64> = slots.iter().map(|s| s.seq).collect();
    assert_eq!(streams, [0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0]);
    assert_eq!(seqs, [0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7]);
    for k in 0..12 {
        assert_eq!(pacer.due(k), plain.due(k / 4 * 4), "slot {k}");
    }
    assert_eq!(pacer.burst(), 4);
    assert!(Pacer::new(&clock, 0, 1, 1, 0, 1)?.with_burst(0).is_err());
    Ok(())
}
