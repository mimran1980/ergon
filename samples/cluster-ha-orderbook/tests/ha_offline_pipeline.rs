//! Offline HA pipeline: real try_claim-shaped publish + follower recovery contract.
//!
//! Every frame goes through the wire path (claim, SessionMessageHeader,
//! AppMessage(L2Book), decode), without Java: snapshots, replayed old frames,
//! sequence gaps, session loss and reconnects, and delayed recovery snapshots.

use std::error::Error;

use cluster_ha_orderbook::follower::BookFollower;
use cluster_ha_orderbook::ha_book::{ApplyOutcome, RejectReason, StaleReason};
use cluster_ha_orderbook::market::{L2Snapshot, Level, WireDec};
use cluster_ha_orderbook::publish::{
    ClusterBookPublisher, PublishOutcome, RecordingClaimIngress, SESSION_MSG_HDR_TEMPLATE_ID,
    app_payload, session_header_template_id,
};

const fn lvl(p: i64, s: i64) -> Level {
    Level {
        price: WireDec::new(p, -2),
        size: WireDec::new(s, -4),
    }
}

fn snap(epoch: i64, sequence: u64, bids: Vec<Level>, asks: Vec<Level>) -> L2Snapshot {
    L2Snapshot {
        epoch,
        sequence,
        symbol: "BTCUSDT".to_owned(),
        exchange_ts_ns: sequence * 10,
        bids,
        asks,
    }
}

/// Publish `s` through the claim path and return the full frame (header + payload).
fn publish_frame(s: &L2Snapshot) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(s.epoch, 99));
    if pubr.publish_snapshot(s, s.exchange_ts_ns + 1) != PublishOutcome::Published {
        return Err("publish failed".into());
    }
    pubr.into_ingress()
        .committed
        .pop()
        .ok_or_else(|| "no committed frame".into())
}

/// Publish `s`, then deliver its app payload to `follower` as egress would.
fn deliver(follower: &mut BookFollower, s: &L2Snapshot) -> Result<ApplyOutcome, Box<dyn Error>> {
    let frame = publish_frame(s)?;
    let payload = app_payload(&frame).ok_or("missing payload")?;
    follower.on_app_payload(payload)
}

fn serving_follower(first: &L2Snapshot) -> Result<BookFollower, Box<dyn Error>> {
    let mut follower = BookFollower::new();
    follower.on_session_up(first.epoch);
    assert_eq!(
        deliver(&mut follower, first)?,
        ApplyOutcome::Applied { missed: 0 }
    );
    Ok(follower)
}

#[test]
fn publish_try_claim_path_then_follower_serves_snapshot() -> Result<(), Box<dyn Error>> {
    let s = snap(3, 1, vec![lvl(500, 1)], vec![lvl(501, 2)]);
    let frame = publish_frame(&s)?;
    assert_eq!(
        session_header_template_id(&frame),
        Some(SESSION_MSG_HDR_TEMPLATE_ID)
    );

    let mut follower = BookFollower::new();
    follower.on_session_up(3);
    assert!(!follower.book().is_serving());
    let o = follower.on_app_payload(app_payload(&frame).ok_or("missing payload")?)?;
    assert_eq!(o, ApplyOutcome::Applied { missed: 0 });
    assert!(follower.book().is_serving());
    assert_eq!(follower.book().live_image(), Some(&s));
    Ok(())
}

#[test]
fn frames_before_the_first_session_are_rejected() -> Result<(), Box<dyn Error>> {
    let mut follower = BookFollower::new();
    let o = deliver(&mut follower, &snap(1, 1, vec![lvl(10, 1)], vec![]))?;
    assert_eq!(o, ApplyOutcome::Rejected(RejectReason::SessionDown));
    assert_eq!(follower.book().stale_reason(), Some(StaleReason::NotReady));
    assert!(follower.book().live_image().is_none());
    assert_eq!(follower.rejected, 1);
    Ok(())
}

#[test]
fn replayed_old_frames_are_rejected_and_book_keeps_serving() -> Result<(), Box<dyn Error>> {
    let s1 = snap(1, 1, vec![lvl(10, 1)], vec![]);
    let s2 = snap(1, 2, vec![lvl(11, 1)], vec![]);
    let mut follower = serving_follower(&s1)?;
    assert_eq!(
        deliver(&mut follower, &s2)?,
        ApplyOutcome::Applied { missed: 0 }
    );

    for (replay, got) in [(&s1, 1), (&s2, 2)] {
        assert_eq!(
            deliver(&mut follower, replay)?,
            ApplyOutcome::Rejected(RejectReason::OldSequence {
                epoch: 1,
                last: 2,
                got
            })
        );
    }
    assert!(follower.book().is_serving());
    assert_eq!(follower.book().live_image(), Some(&s2));
    assert_eq!((follower.applied, follower.rejected), (2, 2));
    Ok(())
}

#[test]
fn sequence_gap_applies_the_complete_image_and_counts_missed() -> Result<(), Box<dyn Error>> {
    let mut follower = serving_follower(&snap(1, 1, vec![lvl(10, 1)], vec![]))?;
    let s4 = snap(1, 4, vec![lvl(13, 1)], vec![lvl(14, 1)]);
    assert_eq!(
        deliver(&mut follower, &s4)?,
        ApplyOutcome::Applied { missed: 2 }
    );
    assert!(follower.book().is_serving());
    assert_eq!(follower.book().live_image(), Some(&s4));
    assert_eq!(follower.missed, 2);

    let s5 = snap(1, 5, vec![lvl(15, 1)], vec![]);
    assert_eq!(
        deliver(&mut follower, &s5)?,
        ApplyOutcome::Applied { missed: 0 }
    );
    assert_eq!(follower.missed, 2);
    Ok(())
}

#[test]
fn session_loss_never_serves_stale_then_recovers() -> Result<(), Box<dyn Error>> {
    let mut follower = serving_follower(&snap(1, 1, vec![lvl(10, 1)], vec![]))?;

    follower.on_session_lost();
    assert!(!follower.book().is_serving());
    assert_eq!(
        follower.book().stale_reason(),
        Some(StaleReason::SessionLost)
    );
    assert!(follower.book().live_image().is_none());
    assert_eq!(follower.losses, 1);

    // A newer old-term frame still in flight while the session is down.
    let o = deliver(&mut follower, &snap(1, 2, vec![lvl(9, 1)], vec![]))?;
    assert_eq!(o, ApplyOutcome::Rejected(RejectReason::SessionDown));
    assert!(!follower.book().is_serving());

    // New leader on term 2: the old term can no longer restore serving.
    follower.on_session_up(2);
    let o = deliver(&mut follower, &snap(1, 3, vec![lvl(9, 1)], vec![]))?;
    assert_eq!(
        o,
        ApplyOutcome::Rejected(RejectReason::StaleEpoch {
            min_epoch: 2,
            got: 1
        })
    );
    assert!(!follower.book().is_serving());

    let reference = snap(2, 1, vec![lvl(11, 2)], vec![]);
    assert_eq!(
        deliver(&mut follower, &reference)?,
        ApplyOutcome::Applied { missed: 0 }
    );
    assert!(follower.book().is_serving());
    assert_eq!(follower.book().live_image(), Some(&reference));
    Ok(())
}

#[test]
fn failover_sequence_reference_equality() -> Result<(), Box<dyn Error>> {
    // Term 5 at sequence 10, then a full book on term 6 that restarts at 1.
    let mut follower = serving_follower(&snap(5, 10, vec![lvl(2000, 3)], vec![lvl(2001, 4)]))?;
    follower.on_session_lost();
    follower.on_session_up(6);

    let reference = snap(6, 1, vec![lvl(1999, 5), lvl(1998, 6)], vec![lvl(2002, 7)]);
    assert_eq!(
        deliver(&mut follower, &reference)?,
        ApplyOutcome::Applied { missed: 0 }
    );
    // Replaced, never merged with the term-5 levels.
    assert_eq!(follower.book().live_image(), Some(&reference));
    Ok(())
}

#[test]
fn repeated_reconnects_keep_raising_the_epoch_floor() -> Result<(), Box<dyn Error>> {
    let mut follower = serving_follower(&snap(1, 7, vec![lvl(10, 1)], vec![]))?;

    // Lost, reconnected on term 2, lost again before term 2 sent anything.
    follower.on_session_lost();
    follower.on_session_up(2);
    follower.on_session_lost();
    let o = deliver(&mut follower, &snap(2, 1, vec![lvl(20, 1)], vec![]))?;
    assert_eq!(o, ApplyOutcome::Rejected(RejectReason::SessionDown));

    follower.on_session_up(3);
    let o = deliver(&mut follower, &snap(2, 1, vec![lvl(20, 1)], vec![]))?;
    assert_eq!(
        o,
        ApplyOutcome::Rejected(RejectReason::StaleEpoch {
            min_epoch: 3,
            got: 2
        })
    );
    assert!(!follower.book().is_serving());

    let recovery = snap(3, 1, vec![lvl(30, 1)], vec![]);
    assert_eq!(
        deliver(&mut follower, &recovery)?,
        ApplyOutcome::Applied { missed: 0 }
    );
    assert_eq!(follower.book().live_image(), Some(&recovery));
    assert_eq!(follower.losses, 2);
    Ok(())
}

#[test]
fn delayed_recovery_snapshots_are_rejected() -> Result<(), Box<dyn Error>> {
    let mut follower = serving_follower(&snap(1, 1, vec![lvl(10, 1)], vec![]))?;
    follower.on_session_lost();
    follower.on_session_up(2);
    let recovery = snap(2, 1, vec![lvl(20, 1)], vec![]);
    assert_eq!(
        deliver(&mut follower, &recovery)?,
        ApplyOutcome::Applied { missed: 0 }
    );
    let latest = snap(2, 2, vec![lvl(21, 1)], vec![]);
    assert_eq!(
        deliver(&mut follower, &latest)?,
        ApplyOutcome::Applied { missed: 0 }
    );

    // The same recovery snapshot delivered late (duplicate or retransmit).
    assert_eq!(
        deliver(&mut follower, &recovery)?,
        ApplyOutcome::Rejected(RejectReason::OldSequence {
            epoch: 2,
            last: 2,
            got: 1
        })
    );
    // A recovery snapshot for the abandoned term 1, arriving after term 2 serves.
    assert_eq!(
        deliver(&mut follower, &snap(1, 9, vec![lvl(19, 1)], vec![]))?,
        ApplyOutcome::Rejected(RejectReason::StaleEpoch {
            min_epoch: 2,
            got: 1
        })
    );
    assert!(follower.book().is_serving());
    assert_eq!(follower.book().live_image(), Some(&latest));

    // After the next failover, term 2's recovery snapshot is stale as well.
    follower.on_session_lost();
    follower.on_session_up(3);
    assert_eq!(
        deliver(&mut follower, &recovery)?,
        ApplyOutcome::Rejected(RejectReason::StaleEpoch {
            min_epoch: 3,
            got: 2
        })
    );
    assert!(follower.book().live_image().is_none());
    Ok(())
}
