//! Per-stream loss, reorder and duplicate accounting from each message's
//! sequence number.

use ergon_latency_core::Error;
use ergon_latency_core::loss::{Arrival, LossCounts, LossTracker};

type R = Result<(), Box<dyn std::error::Error>>;

fn feed(t: &mut LossTracker, stream: u32, seqs: &[u64]) -> Result<Vec<Arrival>, Error> {
    seqs.iter().map(|&s| t.on_message(stream, s)).collect()
}

#[test]
fn an_in_order_stream_loses_nothing() -> R {
    let mut t = LossTracker::new(1, 64)?;
    let arrivals = feed(&mut t, 0, &[0, 1, 2, 3])?;
    assert!(arrivals.iter().all(|a| *a == Arrival::InOrder));
    assert_eq!(
        t.finish(0, 4)?,
        LossCounts {
            received: 4,
            lost: 0,
            reordered: 0,
            duplicate: 0,
            stale: 0
        }
    );
    Ok(())
}

#[test]
fn a_gap_is_lost_unless_it_arrives_late() -> R {
    let mut t = LossTracker::new(1, 64)?;
    assert_eq!(
        feed(&mut t, 0, &[0, 3])?,
        [Arrival::InOrder, Arrival::Gap { missing: 2 }]
    );
    assert_eq!(t.on_message(0, 1)?, Arrival::Reordered);
    assert_eq!(t.on_message(0, 1)?, Arrival::Duplicate);
    assert_eq!(t.on_message(0, 3)?, Arrival::Duplicate);
    // 2 never came; 4 was owed and never came either
    assert_eq!(
        t.finish(0, 5)?,
        LossCounts {
            received: 3,
            lost: 2,
            reordered: 1,
            duplicate: 2,
            stale: 0
        }
    );
    Ok(())
}

#[test]
fn an_arrival_older_than_the_window_is_stale_and_stays_lost() -> R {
    let mut t = LossTracker::new(1, 64)?;
    t.on_message(0, 0)?;
    t.on_message(0, 200)?;
    assert_eq!(t.on_message(0, 5)?, Arrival::Stale);
    let c = t.finish(0, 201)?;
    assert_eq!((c.received, c.lost, c.stale), (2, 199, 1));
    Ok(())
}

#[test]
fn a_first_message_past_zero_is_a_gap_from_zero() -> R {
    let mut t = LossTracker::new(1, 64)?;
    assert_eq!(t.on_message(0, 2)?, Arrival::Gap { missing: 2 });
    assert_eq!(t.on_message(0, 0)?, Arrival::Reordered);
    assert_eq!(t.finish(0, 3)?.lost, 1);
    Ok(())
}

#[test]
fn streams_are_tracked_apart_and_summed() -> R {
    let mut t = LossTracker::new(2, 64)?;
    feed(&mut t, 0, &[0, 1, 2])?;
    feed(&mut t, 1, &[0, 2, 2])?;
    assert_eq!(t.finish(1, 3)?.lost, 1);
    let total = t.totals(&[3, 3])?;
    assert_eq!(
        total,
        LossCounts {
            received: 5,
            lost: 1,
            reordered: 0,
            duplicate: 1,
            stale: 0
        }
    );
    assert!(matches!(
        t.totals(&[3]),
        Err(Error::Streams { streams: 2, got: 1 })
    ));
    Ok(())
}

#[test]
fn an_unknown_stream_or_bad_window_is_refused() -> R {
    let mut t = LossTracker::new(2, 64)?;
    assert!(matches!(
        t.on_message(2, 0),
        Err(Error::Stream {
            stream: 2,
            streams: 2
        })
    ));
    assert!(matches!(LossTracker::new(1, 100), Err(Error::Window(100))));
    assert!(matches!(LossTracker::new(1, 32), Err(Error::Window(32))));
    assert!(matches!(
        LossTracker::new(0, 64),
        Err(Error::Streams { streams: 0, got: 0 })
    ));
    Ok(())
}

#[test]
fn a_jump_wider_than_the_window_clears_it() -> R {
    let mut t = LossTracker::new(1, 64)?;
    feed(&mut t, 0, &[0, 1, 2])?;
    assert_eq!(t.on_message(0, 10_000)?, Arrival::Gap { missing: 9_997 });
    // within the window behind 10_000, nothing is marked as seen
    assert_eq!(t.on_message(0, 9_990)?, Arrival::Reordered);
    let c = t.finish(0, 10_001)?;
    assert_eq!((c.received, c.lost, c.reordered), (5, 9_996, 1));
    Ok(())
}
