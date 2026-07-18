//! Leadership-aware order-book apply policy for the HA cluster sample.
//!
//! On leadership **release** (`NewLeader`, session reconnect, session close)
//! the book stops serving until a term-valid **snapshot** lands. Incremental
//! updates never apply across a term boundary or sequence gap.
//!
//! This module is pure logic (no Aeron / cluster transport) so it is unit-
//! testable offline. The cluster sample wires it to egress events later.

use crate::market::Level;

/// Why the book is not serving live prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleReason {
    /// Initial state or explicit freeze before any snapshot.
    NotReady,
    /// Leadership term changed or session released.
    LeadershipRelease,
    /// Sequence gap on the active term.
    SequenceGap { expected: u64, got: u64 },
    /// Message term does not match the book epoch.
    TermMismatch { book_term: i64, msg_term: i64 },
}

/// Outcome of applying a book message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Full snapshot installed; book is serving.
    SnapshotApplied,
    /// Increment applied while serving.
    IncrementApplied,
    /// Dropped because book is not serving (waiting for snapshot).
    DroppedNotServing,
    /// Term/sequence invalid — book marked stale; needs resync.
    ResyncRequired(StaleReason),
}

/// In-memory L2 book image owned by a follower.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookImage {
    pub symbol: String,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
    pub exchange_ts_ns: u64,
}

/// Leadership-aware book state machine.
#[derive(Debug, Clone)]
pub struct LeadershipAwareBook {
    leadership_term_id: Option<i64>,
    last_seq: u64,
    serving: bool,
    stale: StaleReason,
    image: BookImage,
}

impl Default for LeadershipAwareBook {
    fn default() -> Self {
        Self::new()
    }
}

impl LeadershipAwareBook {
    #[must_use]
    pub fn new() -> Self {
        Self {
            leadership_term_id: None,
            last_seq: 0,
            serving: false,
            stale: StaleReason::NotReady,
            image: BookImage::default(),
        }
    }

    #[must_use]
    pub fn is_serving(&self) -> bool {
        self.serving
    }

    #[must_use]
    pub fn stale_reason(&self) -> Option<StaleReason> {
        if self.serving {
            None
        } else {
            Some(self.stale)
        }
    }

    #[must_use]
    pub fn leadership_term_id(&self) -> Option<i64> {
        self.leadership_term_id
    }

    #[must_use]
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Live image only when serving; otherwise `None` (never silent last-good).
    #[must_use]
    pub fn live_image(&self) -> Option<&BookImage> {
        if self.serving {
            Some(&self.image)
        } else {
            None
        }
    }

    /// Frozen last image for debug/UI only — must not be treated as live.
    #[must_use]
    pub fn frozen_image(&self) -> &BookImage {
        &self.image
    }

    /// Call on `NewLeaderEvent`, session reconnect, or session release.
    pub fn on_leadership_release(&mut self) {
        self.serving = false;
        self.stale = StaleReason::LeadershipRelease;
        // Keep last image frozen for debug; do not clear term until snapshot
        // so TermMismatch can still be diagnosed mid-resync.
    }

    /// Install a full L2 snapshot for leadership term `term` at sequence `seq`.
    pub fn apply_snapshot(
        &mut self,
        term: i64,
        seq: u64,
        symbol: impl Into<String>,
        bids: Vec<Level>,
        asks: Vec<Level>,
        exchange_ts_ns: u64,
    ) -> ApplyOutcome {
        self.image = BookImage {
            symbol: symbol.into(),
            bids,
            asks,
            exchange_ts_ns,
        };
        self.leadership_term_id = Some(term);
        self.last_seq = seq;
        self.serving = true;
        ApplyOutcome::SnapshotApplied
    }

    /// Apply an incremental L2 update. Requires continuous sequence on the
    /// active term while serving.
    pub fn apply_increment(
        &mut self,
        term: i64,
        seq: u64,
        bids: Vec<Level>,
        asks: Vec<Level>,
        exchange_ts_ns: u64,
    ) -> ApplyOutcome {
        if !self.serving {
            return ApplyOutcome::DroppedNotServing;
        }
        let book_term = match self.leadership_term_id {
            Some(t) => t,
            None => {
                self.serving = false;
                self.stale = StaleReason::NotReady;
                return ApplyOutcome::ResyncRequired(StaleReason::NotReady);
            }
        };
        if term != book_term {
            self.serving = false;
            let reason = StaleReason::TermMismatch {
                book_term,
                msg_term: term,
            };
            self.stale = reason;
            return ApplyOutcome::ResyncRequired(reason);
        }
        let expected = self.last_seq.saturating_add(1);
        if seq != expected {
            self.serving = false;
            let reason = StaleReason::SequenceGap {
                expected,
                got: seq,
            };
            self.stale = reason;
            return ApplyOutcome::ResyncRequired(reason);
        }
        // Full-replace levels for simplicity (sample-grade L2 image).
        self.image.bids = bids;
        self.image.asks = asks;
        self.image.exchange_ts_ns = exchange_ts_ns;
        self.last_seq = seq;
        ApplyOutcome::IncrementApplied
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::WireDec;

    fn lvl(p: i64, s: i64) -> Level {
        Level {
            price: WireDec::new(p, -2),
            size: WireDec::new(s, -4),
        }
    }

    #[test]
    fn starts_not_serving() {
        let b = LeadershipAwareBook::new();
        assert!(!b.is_serving());
        assert_eq!(b.stale_reason(), Some(StaleReason::NotReady));
        assert!(b.live_image().is_none());
    }

    #[test]
    fn snapshot_enables_serving() -> Result<(), Box<dyn std::error::Error>> {
        let mut b = LeadershipAwareBook::new();
        let o = b.apply_snapshot(7, 10, "BTCUSDT", vec![lvl(100, 1)], vec![lvl(101, 2)], 1);
        assert_eq!(o, ApplyOutcome::SnapshotApplied);
        assert!(b.is_serving());
        assert_eq!(b.leadership_term_id(), Some(7));
        assert_eq!(b.last_seq(), 10);
        assert_eq!(b.live_image().map(|i| i.symbol.as_str()), Some("BTCUSDT"));
        Ok(())
    }

    #[test]
    fn increment_requires_continuous_seq() -> Result<(), Box<dyn std::error::Error>> {
        let mut b = LeadershipAwareBook::new();
        let _ = b.apply_snapshot(1, 5, "BTCUSDT", vec![], vec![], 0);
        let o = b.apply_increment(1, 6, vec![lvl(1, 1)], vec![], 1);
        assert_eq!(o, ApplyOutcome::IncrementApplied);
        assert_eq!(b.last_seq(), 6);
        Ok(())
    }

    #[test]
    fn sequence_gap_marks_stale() -> Result<(), Box<dyn std::error::Error>> {
        let mut b = LeadershipAwareBook::new();
        let _ = b.apply_snapshot(1, 5, "BTCUSDT", vec![lvl(1, 1)], vec![], 0);
        let o = b.apply_increment(1, 8, vec![], vec![], 0);
        assert_eq!(
            o,
            ApplyOutcome::ResyncRequired(StaleReason::SequenceGap {
                expected: 6,
                got: 8
            })
        );
        assert!(!b.is_serving());
        assert!(b.live_image().is_none());
        // Frozen image still available for debug, not live.
        assert_eq!(b.frozen_image().bids.len(), 1);
        Ok(())
    }

    #[test]
    fn term_mismatch_on_increment_resyncs() -> Result<(), Box<dyn std::error::Error>> {
        let mut b = LeadershipAwareBook::new();
        let _ = b.apply_snapshot(3, 1, "BTCUSDT", vec![], vec![], 0);
        let o = b.apply_increment(4, 2, vec![], vec![], 0);
        assert_eq!(
            o,
            ApplyOutcome::ResyncRequired(StaleReason::TermMismatch {
                book_term: 3,
                msg_term: 4
            })
        );
        assert!(!b.is_serving());
        Ok(())
    }

    #[test]
    fn leadership_release_stops_serving_until_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let mut b = LeadershipAwareBook::new();
        let _ = b.apply_snapshot(2, 1, "BTCUSDT", vec![lvl(9, 9)], vec![], 0);
        assert!(b.is_serving());
        b.on_leadership_release();
        assert!(!b.is_serving());
        assert_eq!(b.stale_reason(), Some(StaleReason::LeadershipRelease));
        // Increments dropped while not serving.
        let o = b.apply_increment(2, 2, vec![], vec![], 0);
        assert_eq!(o, ApplyOutcome::DroppedNotServing);
        // New term snapshot restores service.
        let o = b.apply_snapshot(3, 1, "BTCUSDT", vec![lvl(10, 1)], vec![], 2);
        assert_eq!(o, ApplyOutcome::SnapshotApplied);
        assert!(b.is_serving());
        assert_eq!(b.leadership_term_id(), Some(3));
        Ok(())
    }

    #[test]
    fn never_serves_stale_across_release() -> Result<(), Box<dyn std::error::Error>> {
        let mut b = LeadershipAwareBook::new();
        let _ = b.apply_snapshot(1, 1, "BTCUSDT", vec![lvl(100, 1)], vec![], 0);
        b.on_leadership_release();
        // Cross-term silent merge attempt must not re-enable serving.
        let o = b.apply_increment(1, 2, vec![lvl(99, 1)], vec![], 0);
        assert_eq!(o, ApplyOutcome::DroppedNotServing);
        assert!(b.live_image().is_none());
        Ok(())
    }
}
