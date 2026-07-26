//! Follower: apply cluster egress app payloads into [`LeadershipAwareBook`].
//!
//! On `NewLeader` / session release, marks book stale. Snapshots resume serving.

use crate::ha_book::{ApplyOutcome, LeadershipAwareBook};
use crate::market::{Level, WireDec};
use crate::normalized_app::{AppMessageDecoder, L2BookDecoder};

/// Follower view of the HA book with apply counters.
pub struct BookFollower {
    book: LeadershipAwareBook,
    pub applied_snapshots: u64,
    pub applied_increments: u64,
    pub dropped: u64,
    pub resyncs: u64,
}

impl Default for BookFollower {
    fn default() -> Self {
        Self::new()
    }
}

impl BookFollower {
    #[must_use]
    pub fn new() -> Self {
        Self {
            book: LeadershipAwareBook::new(),
            applied_snapshots: 0,
            applied_increments: 0,
            dropped: 0,
            resyncs: 0,
        }
    }

    #[must_use]
    pub fn book(&self) -> &LeadershipAwareBook {
        &self.book
    }

    /// Call when egress delivers NewLeaderEvent or session is released.
    pub fn on_leadership_release(&mut self) {
        self.book.on_leadership_release();
        self.resyncs += 1;
    }

    /// Apply a full L2 snapshot for the given leadership term.
    pub fn apply_snapshot(
        &mut self,
        term: i64,
        seq: u64,
        symbol: &str,
        bids: Vec<Level>,
        asks: Vec<Level>,
        exchange_ts_ns: u64,
    ) -> ApplyOutcome {
        let o = self
            .book
            .apply_snapshot(term, seq, symbol, bids, asks, exchange_ts_ns);
        if o == ApplyOutcome::SnapshotApplied {
            self.applied_snapshots += 1;
        }
        o
    }

    /// Apply an increment; may resync.
    pub fn apply_increment(
        &mut self,
        term: i64,
        seq: u64,
        bids: Vec<Level>,
        asks: Vec<Level>,
        exchange_ts_ns: u64,
    ) -> ApplyOutcome {
        let o = self
            .book
            .apply_increment(term, seq, bids, asks, exchange_ts_ns);
        match o {
            ApplyOutcome::IncrementApplied => self.applied_increments += 1,
            ApplyOutcome::DroppedNotServing => self.dropped += 1,
            ApplyOutcome::ResyncRequired(_) => self.resyncs += 1,
            ApplyOutcome::SnapshotApplied => {}
        }
        o
    }

    /// Decode an AppMessage payload (after SessionMessageHeader) as L2Book
    /// and apply as snapshot when not serving, else as increment.
    pub fn on_app_payload(
        &mut self,
        term: i64,
        payload: &[u8],
    ) -> Result<ApplyOutcome, Box<dyn std::error::Error>> {
        let app = AppMessageDecoder::try_wrap_and_apply_header(payload, 0)?;
        let (_name, after) = app.into_app_name()?;
        let (inner, _done) = after.into_payload()?;
        let book = L2BookDecoder::try_wrap_and_apply_header(inner, 0)?;
        let exchange_ts = book.exchange_timestamp();
        let seq = book.sequence();

        let mut bids_dec = book.into_bids()?;
        let mut bids = Vec::with_capacity(bids_dec.remaining());
        for entry in &mut bids_dec {
            let px = entry.price_wire();
            let sz = entry.size_wire();
            bids.push(Level {
                price: WireDec::new(px.mantissa(), px.exponent()),
                size: WireDec::new(sz.mantissa(), sz.exponent()),
            });
        }
        let after_bids = bids_dec.finish()?;
        let mut asks_dec = after_bids.into_asks()?;
        let mut asks = Vec::with_capacity(asks_dec.remaining());
        for entry in &mut asks_dec {
            let px = entry.price_wire();
            let sz = entry.size_wire();
            asks.push(Level {
                price: WireDec::new(px.mantissa(), px.exponent()),
                size: WireDec::new(sz.mantissa(), sz.exponent()),
            });
        }
        let after_asks = asks_dec.finish()?;
        let (sym_bytes, _) = after_asks.into_symbol()?;
        let symbol = std::str::from_utf8(sym_bytes).unwrap_or("");

        let outcome = if self.book.is_serving() {
            self.apply_increment(term, seq, bids, asks, exchange_ts)
        } else {
            self.apply_snapshot(term, seq, symbol, bids, asks, exchange_ts)
        };
        Ok(outcome)
    }
}
