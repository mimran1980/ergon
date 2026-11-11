//! Leadership-aware order-book recovery contract for the HA cluster sample.
//!
//! Every `L2Book` frame is a complete image ([`L2Snapshot`]), never a delta,
//! stamped with the publisher's leadership term (`epoch`) and a `sequence`
//! that orders images within that epoch. The book accepts a frame only when:
//!
//! 1. the session is up ([`LeadershipAwareBook::on_session_up`] ran since the
//!    last [`LeadershipAwareBook::on_session_lost`]);
//! 2. `epoch >= min_epoch`, where `min_epoch` is the highest of the session
//!    epochs reported and the epochs already applied;
//! 3. within the applied epoch, `sequence > last sequence`. A new epoch may
//!    restart at any sequence.
//!
//! A rejected frame changes nothing: a serving book keeps serving its image.
//! A sequence gap within an epoch is accepted, because the frame is a whole
//! image and the missed frames carried nothing it lacks; the outcome reports
//! how many were missed.
//!
//! The book serves nothing from the first observed session loss until a frame
//! is accepted on the re-established session. A session up on a newer term
//! also stops serving a held image from an older term, even with no loss
//! reported first. This client only reconnects to
//! a new leader, so the recovery frame carries the new, higher term, and any
//! frame from the lost term (a replay, or a recovery snapshot delayed in
//! flight) is rejected as a stale epoch.
//!
//! This module is pure logic (no Aeron / cluster transport) so it is unit-
//! testable offline; `follower` feeds it from cluster egress.

use crate::market::L2Snapshot;

/// Why the book is not serving live prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleReason {
    /// No frame accepted yet.
    NotReady,
    /// Session lost; waiting for a frame on the re-established session.
    SessionLost,
}

/// Why a frame was rejected. The book is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// The session is down (not yet up, or lost and not re-established).
    SessionDown,
    /// The frame's epoch is below the current epoch floor.
    StaleEpoch { min_epoch: i64, got: i64 },
    /// Same epoch as the image, but not newer (a replay or a late duplicate).
    OldSequence { epoch: i64, last: u64, got: u64 },
}

/// Outcome of offering one frame to the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Image installed; the book serves it. `missed` counts the sequence
    /// numbers skipped within the epoch (0 when contiguous or on a new epoch).
    Applied { missed: u64 },
    /// Frame dropped; see [`RejectReason`].
    Rejected(RejectReason),
}

/// Leadership-aware book state machine.
#[derive(Debug, Clone)]
pub struct LeadershipAwareBook {
    image: Option<L2Snapshot>,
    min_epoch: i64,
    session_up: bool,
    serving: bool,
    stale: StaleReason,
}

impl Default for LeadershipAwareBook {
    fn default() -> Self {
        Self::new()
    }
}

impl LeadershipAwareBook {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            image: None,
            min_epoch: i64::MIN,
            session_up: false,
            serving: false,
            stale: StaleReason::NotReady,
        }
    }

    #[must_use]
    pub const fn is_serving(&self) -> bool {
        self.serving
    }

    #[must_use]
    pub const fn stale_reason(&self) -> Option<StaleReason> {
        if self.serving { None } else { Some(self.stale) }
    }

    /// Lowest epoch a frame may carry.
    #[must_use]
    pub const fn min_epoch(&self) -> i64 {
        self.min_epoch
    }

    /// Live image only when serving; otherwise `None` (never silent last-good).
    #[must_use]
    pub const fn live_image(&self) -> Option<&L2Snapshot> {
        if self.serving {
            self.image.as_ref()
        } else {
            None
        }
    }

    /// Last accepted image, live or not, for debug/UI only.
    #[must_use]
    pub const fn frozen_image(&self) -> Option<&L2Snapshot> {
        self.image.as_ref()
    }

    /// The session is (re-)established on leadership term `epoch`: call it on
    /// connect and whenever the client is `Connected` again after a loss, with
    /// the client's current term. `NewLeader` is a loss
    /// ([`Self::on_session_lost`]), not an up. Frames below `epoch` are stale
    /// from now on, and a held image below it stops being served.
    pub fn on_session_up(&mut self, epoch: i64) {
        self.session_up = true;
        self.min_epoch = self.min_epoch.max(epoch);
        if self
            .image
            .as_ref()
            .is_some_and(|i| i.epoch < self.min_epoch)
        {
            self.serving = false;
            self.stale = StaleReason::SessionLost;
        }
    }

    /// The session is lost: the client left `Connected`, saw `NewLeader`, or
    /// the session closed. Stops serving until a frame is accepted after the
    /// next [`Self::on_session_up`].
    pub const fn on_session_lost(&mut self) {
        self.session_up = false;
        self.serving = false;
        self.stale = StaleReason::SessionLost;
    }

    /// Offer one complete image; see the module docs for the rules.
    pub fn apply(&mut self, snapshot: L2Snapshot) -> ApplyOutcome {
        if !self.session_up {
            return ApplyOutcome::Rejected(RejectReason::SessionDown);
        }
        if snapshot.epoch < self.min_epoch {
            return ApplyOutcome::Rejected(RejectReason::StaleEpoch {
                min_epoch: self.min_epoch,
                got: snapshot.epoch,
            });
        }
        // `min_epoch` covers every applied epoch, so only an equal epoch remains.
        let missed = match &self.image {
            Some(cur) if cur.epoch == snapshot.epoch => {
                if snapshot.sequence <= cur.sequence {
                    return ApplyOutcome::Rejected(RejectReason::OldSequence {
                        epoch: cur.epoch,
                        last: cur.sequence,
                        got: snapshot.sequence,
                    });
                }
                snapshot.sequence - cur.sequence - 1
            }
            _ => 0,
        };
        self.min_epoch = snapshot.epoch;
        self.image = Some(snapshot);
        self.serving = true;
        ApplyOutcome::Applied { missed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::{Level, WireDec};

    fn lvl(p: i64, s: i64) -> Level {
        Level {
            price: WireDec::new(p, -2),
            size: WireDec::new(s, -4),
        }
    }

    fn snap(epoch: i64, sequence: u64, bids: Vec<Level>) -> L2Snapshot {
        L2Snapshot {
            epoch,
            sequence,
            symbol: "BTCUSDT".to_owned(),
            exchange_ts_ns: 0,
            bids,
            asks: vec![],
        }
    }

    fn serving(epoch: i64, sequence: u64, bids: Vec<Level>) -> LeadershipAwareBook {
        let mut b = LeadershipAwareBook::new();
        b.on_session_up(epoch);
        assert_eq!(
            b.apply(snap(epoch, sequence, bids)),
            ApplyOutcome::Applied { missed: 0 }
        );
        b
    }

    #[test]
    fn starts_not_serving() {
        let b = LeadershipAwareBook::new();
        assert!(!b.is_serving());
        assert_eq!(b.stale_reason(), Some(StaleReason::NotReady));
        assert!(b.live_image().is_none());
    }

    #[test]
    fn rejects_frames_until_the_session_is_up() {
        let mut b = LeadershipAwareBook::new();
        assert_eq!(
            b.apply(snap(1, 1, vec![])),
            ApplyOutcome::Rejected(RejectReason::SessionDown)
        );
        assert!(b.frozen_image().is_none());
    }

    #[test]
    fn snapshot_enables_serving() {
        let b = serving(7, 10, vec![lvl(100, 1)]);
        assert!(b.is_serving());
        assert_eq!(b.min_epoch(), 7);
        assert_eq!(b.live_image(), Some(&snap(7, 10, vec![lvl(100, 1)])));
    }

    #[test]
    fn next_sequence_replaces_the_image() {
        let mut b = serving(1, 5, vec![]);
        let next = snap(1, 6, vec![lvl(1, 1)]);
        assert_eq!(b.apply(next.clone()), ApplyOutcome::Applied { missed: 0 });
        assert_eq!(b.live_image(), Some(&next));
    }

    #[test]
    fn sequence_gap_applies_and_reports_missed() {
        let mut b = serving(1, 5, vec![lvl(1, 1)]);
        let gap = snap(1, 8, vec![lvl(2, 1)]);
        assert_eq!(b.apply(gap.clone()), ApplyOutcome::Applied { missed: 2 });
        assert!(b.is_serving());
        assert_eq!(b.live_image(), Some(&gap));
    }

    #[test]
    fn old_or_duplicate_sequence_is_rejected_and_keeps_serving() {
        let mut b = serving(1, 5, vec![lvl(1, 1)]);
        for got in [5, 4] {
            assert_eq!(
                b.apply(snap(1, got, vec![])),
                ApplyOutcome::Rejected(RejectReason::OldSequence {
                    epoch: 1,
                    last: 5,
                    got
                })
            );
        }
        assert_eq!(b.live_image(), Some(&snap(1, 5, vec![lvl(1, 1)])));
    }

    #[test]
    fn older_epoch_is_rejected_and_newer_epoch_restarts_sequence() {
        let mut b = serving(3, 9, vec![]);
        assert_eq!(
            b.apply(snap(2, 10, vec![])),
            ApplyOutcome::Rejected(RejectReason::StaleEpoch {
                min_epoch: 3,
                got: 2
            })
        );
        assert!(b.is_serving());
        assert_eq!(
            b.apply(snap(4, 1, vec![])),
            ApplyOutcome::Applied { missed: 0 }
        );
        assert_eq!(b.min_epoch(), 4);
    }

    #[test]
    fn session_loss_stops_serving_until_a_frame_on_the_new_session() {
        let mut b = serving(2, 1, vec![lvl(9, 9)]);
        b.on_session_lost();
        assert!(!b.is_serving());
        assert_eq!(b.stale_reason(), Some(StaleReason::SessionLost));
        assert!(b.live_image().is_none());
        // Frozen image still available for debug, not live.
        assert_eq!(b.frozen_image(), Some(&snap(2, 1, vec![lvl(9, 9)])));

        assert_eq!(
            b.apply(snap(2, 2, vec![])),
            ApplyOutcome::Rejected(RejectReason::SessionDown)
        );
        b.on_session_up(3);
        assert_eq!(
            b.apply(snap(2, 3, vec![lvl(99, 1)])),
            ApplyOutcome::Rejected(RejectReason::StaleEpoch {
                min_epoch: 3,
                got: 2
            })
        );
        assert!(b.live_image().is_none());

        assert_eq!(
            b.apply(snap(3, 1, vec![lvl(10, 1)])),
            ApplyOutcome::Applied { missed: 0 }
        );
        assert_eq!(b.live_image(), Some(&snap(3, 1, vec![lvl(10, 1)])));
    }

    #[test]
    fn session_up_on_a_newer_term_without_a_loss_stops_serving() {
        let mut b = serving(1, 1, vec![lvl(9, 9)]);
        b.on_session_up(2);
        assert!(!b.is_serving());
        assert_eq!(b.stale_reason(), Some(StaleReason::SessionLost));
        assert!(b.live_image().is_none());
        assert_eq!(
            b.apply(snap(2, 1, vec![lvl(10, 1)])),
            ApplyOutcome::Applied { missed: 0 }
        );
        assert!(b.is_serving());
    }

    #[test]
    fn session_up_on_the_same_term_keeps_serving() {
        let mut b = serving(3, 1, vec![]);
        b.on_session_up(3);
        assert!(b.is_serving());
    }

    #[test]
    fn session_up_never_lowers_the_epoch_floor() {
        let mut b = serving(5, 1, vec![]);
        b.on_session_lost();
        b.on_session_up(4);
        assert_eq!(b.min_epoch(), 5);
        assert_eq!(
            b.apply(snap(4, 1, vec![])),
            ApplyOutcome::Rejected(RejectReason::StaleEpoch {
                min_epoch: 5,
                got: 4
            })
        );
    }
}
