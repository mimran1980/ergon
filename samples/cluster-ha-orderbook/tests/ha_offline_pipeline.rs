//! Offline HA pipeline: real try_claim-shaped publish + follower book policy.
//!
//! Proves H1 (SessionMessageHeader + AppMessage via claim path), H2/H3
//! (stale across release, resync via snapshot), without Java.

use cluster_ha_orderbook::follower::BookFollower;
use cluster_ha_orderbook::ha_book::ApplyOutcome;
use cluster_ha_orderbook::market::{Level, WireDec};
use cluster_ha_orderbook::publish::{
    ClusterBookPublisher, PublishOutcome, RecordingClaimIngress, SESSION_MSG_HDR_TEMPLATE_ID,
    app_payload, session_header_template_id,
};

fn lvl(p: i64, s: i64) -> Level {
    Level {
        price: WireDec::new(p, -2),
        size: WireDec::new(s, -4),
    }
}

#[test]
fn publish_try_claim_path_then_follower_serves_snapshot() -> Result<(), Box<dyn std::error::Error>>
{
    let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(3, 99));
    assert_eq!(
        pubr.publish_l2_snapshot("BTCUSDT", 1, 100, 110, &[lvl(500, 1)], &[lvl(501, 2)]),
        PublishOutcome::Published
    );
    let frame = &pubr.ingress().committed[0];
    assert_eq!(
        session_header_template_id(frame),
        Some(SESSION_MSG_HDR_TEMPLATE_ID)
    );
    let payload = app_payload(frame).expect("payload");

    let mut follower = BookFollower::new();
    assert!(!follower.book().is_serving());
    let o = follower.on_app_payload(3, payload)?;
    assert_eq!(o, ApplyOutcome::SnapshotApplied);
    assert!(follower.book().is_serving());
    let live = follower.book().live_image().expect("live");
    assert_eq!(live.symbol, "BTCUSDT");
    assert_eq!(live.bids.len(), 1);
    assert_eq!(live.asks.len(), 1);
    Ok(())
}

#[test]
fn leadership_release_never_serves_stale_then_resync() -> Result<(), Box<dyn std::error::Error>> {
    let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(1, 1));
    let _ = pubr.publish_l2_snapshot("BTCUSDT", 1, 1, 2, &[lvl(10, 1)], &[]);
    let frame1 = pubr.ingress().committed[0].clone();

    let mut follower = BookFollower::new();
    let _ = follower.on_app_payload(1, app_payload(&frame1).unwrap())?;
    assert!(follower.book().is_serving());

    // Simulate NewLeader / session release.
    follower.on_leadership_release();
    assert!(!follower.book().is_serving());
    assert!(follower.book().live_image().is_none());

    // Old-term increment must not re-enable serving.
    let o = follower.apply_increment(1, 2, vec![lvl(9, 1)], vec![], 3);
    assert_eq!(o, ApplyOutcome::DroppedNotServing);

    // New-term snapshot restores service (reference book).
    let o = follower.apply_snapshot(2, 1, "BTCUSDT", vec![lvl(11, 2)], vec![], 4);
    assert_eq!(o, ApplyOutcome::SnapshotApplied);
    assert!(follower.book().is_serving());
    assert_eq!(follower.book().leadership_term_id(), Some(2));
    let live = follower.book().live_image().unwrap();
    assert_eq!(live.bids[0].price.mantissa, 11);
    Ok(())
}

#[test]
fn failover_sequence_reference_equality() -> Result<(), Box<dyn std::error::Error>> {
    // Publisher stream on term 5, kill/release, republish snapshot on term 6.
    let mut pubr = ClusterBookPublisher::new(RecordingClaimIngress::new(5, 7));
    let _ = pubr.publish_l2_snapshot("ETHUSDT", 10, 50, 51, &[lvl(2000, 3)], &[lvl(2001, 4)]);
    let f1 = pubr.ingress().committed[0].clone();

    let mut follower = BookFollower::new();
    let _ = follower.on_app_payload(5, app_payload(&f1).unwrap())?;
    follower.on_leadership_release();

    // New leader term 6 full book (reference).
    let mut pubr2 = ClusterBookPublisher::new(RecordingClaimIngress::new(6, 7));
    let ref_bids = vec![lvl(1999, 5), lvl(1998, 6)];
    let ref_asks = vec![lvl(2002, 7)];
    let _ = pubr2.publish_l2_snapshot("ETHUSDT", 1, 60, 61, &ref_bids, &ref_asks);
    let f2 = &pubr2.ingress().committed[0];
    let o = follower.on_app_payload(6, app_payload(f2).unwrap())?;
    assert_eq!(o, ApplyOutcome::SnapshotApplied);
    let live = follower.book().live_image().unwrap();
    assert_eq!(live.bids.len(), 2);
    assert_eq!(live.asks.len(), 1);
    assert_eq!(live.bids[0].price.mantissa, 1999);
    assert_eq!(live.asks[0].price.mantissa, 2002);
    Ok(())
}
