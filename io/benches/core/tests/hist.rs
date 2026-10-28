//! A point's latency histograms: exact min and max beside `HdrHistogram`'s
//! percentiles, censored values, and the CSV the analysis merges.

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::hist::{HISTOGRAM_MAX_NS, Latency, PointLatency, Summary};
use ergon_latency_core::message::{Kind, Message};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn min_and_max_are_exact_not_bucket_edges() -> R {
    let mut l = Latency::new()?;
    for v in [12_345u64, 1_000_003, 777] {
        l.record(v);
    }
    let s = l.summary();
    assert_eq!((s.min, s.max, s.count, s.censored), (777, 1_000_003, 3, 0));
    // every percentile lies within [min, max]
    for p in [s.p50, s.p90, s.p99, s.p999, s.p9999] {
        assert!((s.min..=s.max).contains(&p), "{p} in {s:?}");
    }
    Ok(())
}

#[test]
fn percentiles_follow_the_distribution() -> R {
    let mut l = Latency::new()?;
    for v in 1..=100_000u64 {
        l.record(v * 10);
    }
    let s = l.summary();
    let near = |got: u64, want: u64| got.abs_diff(want) * 1000 <= want;
    assert!(near(s.p50, 500_000), "{s:?}");
    assert!(near(s.p90, 900_000), "{s:?}");
    assert!(near(s.p99, 990_000), "{s:?}");
    assert!(near(s.p999, 999_000), "{s:?}");
    assert!(near(s.p9999, 999_900), "{s:?}");
    assert_eq!((s.min, s.max), (10, 1_000_000));
    assert!((s.mean - 500_005.0).abs() < 500.0, "{s:?}");
    Ok(())
}

#[test]
fn an_empty_histogram_summarises_to_zeros() -> R {
    assert_eq!(Latency::new()?.summary(), Summary::default());
    Ok(())
}

#[test]
fn a_value_at_the_bound_or_owed_is_censored() -> R {
    let mut l = Latency::new()?;
    l.record(HISTOGRAM_MAX_NS + 5);
    l.record_censored(4_000);
    l.record(10);
    let s = l.summary();
    assert_eq!((s.count, s.censored), (3, 2));
    assert_eq!(s.max, HISTOGRAM_MAX_NS);
    Ok(())
}

#[test]
fn the_csv_is_value_ns_count_per_recorded_value() -> R {
    let mut l = Latency::new()?;
    l.record(100);
    l.record(100);
    l.record(2_000);
    assert_eq!(l.csv(), "value_ns,count\n100,2\n2000,1\n");
    Ok(())
}

#[test]
fn a_point_records_from_the_intended_and_from_the_actual_send() -> R {
    let clock = TscClock::new();
    let intended = 1_000_000u64;
    let send = intended + clock.ns_to_ticks(30_000);
    let recv = send + clock.ns_to_ticks(20_000);
    let m = Message {
        seq: 0,
        send_tsc: send,
        intended_tsc: intended,
        stream: 0,
        kind: Kind::Echo,
    };
    let mut p = PointLatency::new()?;
    p.record(&clock, &m, recv);
    let (from_intended, from_send) = (p.from_intended.summary(), p.from_send.summary());
    assert_eq!(from_intended.max, clock.delta_ns(intended, recv));
    assert_eq!(from_send.max, clock.delta_ns(send, recv));
    // owed at the end: from intended always, from send only when it went
    let end = recv + clock.ns_to_ticks(1_000_000);
    p.record_owed(&clock, intended, None, end);
    p.record_owed(&clock, intended, Some(send), end);
    assert_eq!(
        (
            p.from_intended.summary().count,
            p.from_intended.summary().censored
        ),
        (3, 2)
    );
    assert_eq!(
        (p.from_send.summary().count, p.from_send.summary().censored),
        (2, 1)
    );
    Ok(())
}

#[test]
fn add_merges_two_histograms_as_one() -> R {
    let (mut a, mut b, mut both) = (Latency::new()?, Latency::new()?, Latency::new()?);
    for v in [500u64, 1_000, 2_000] {
        a.record(v);
        both.record(v);
    }
    for v in [100u64, 9_000, 3_000] {
        b.record(v);
        both.record(v);
    }
    b.record_censored(70_000);
    both.record_censored(70_000);
    a.add(&b)?;
    let (got, want) = (a.summary(), both.summary());
    assert_eq!(got, want);
    assert_eq!(
        (got.min, got.max, got.count, got.censored),
        (100, 70_000, 7, 1)
    );
    assert_eq!(a.csv(), both.csv());
    Ok(())
}
