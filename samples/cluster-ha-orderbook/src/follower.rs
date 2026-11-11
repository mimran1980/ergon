//! Follower: decode cluster egress app payloads and offer them to
//! [`LeadershipAwareBook`], counting what happened.
//!
//! The caller reports the session: [`BookFollower::on_session_lost`] as soon as
//! it sees the loss (the client leaves `Connected`, `NewLeader` arrives, or the
//! session closes), and [`BookFollower::on_session_up`] with the client's
//! `leadership_term_id()` on connect and whenever the client is `Connected`
//! again. `NewLeader` is a loss, not an up. Every payload is a complete image;
//! see [`crate::ha_book`] for the accept and reject rules.

use crate::ha_book::{ApplyOutcome, LeadershipAwareBook};
use crate::market::{L2Snapshot, Level, WireDec};
use crate::normalized_app::{AppMessageDecoder, DecimalDecoder, L2BookDecoder};

/// Follower view of the HA book with apply counters.
#[derive(Default)]
pub struct BookFollower {
    book: LeadershipAwareBook,
    /// Frames installed.
    pub applied: u64,
    /// Frames dropped (session down, stale epoch, old sequence).
    pub rejected: u64,
    /// Sequence numbers skipped by applied frames within an epoch.
    pub missed: u64,
    /// Session losses reported.
    pub losses: u64,
}

impl BookFollower {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn book(&self) -> &LeadershipAwareBook {
        &self.book
    }

    /// The session is (re-)established on leadership term `epoch` (the
    /// client is `Connected`); see [`LeadershipAwareBook::on_session_up`].
    pub fn on_session_up(&mut self, epoch: i64) {
        self.book.on_session_up(epoch);
    }

    /// The session is lost; the book stops serving at once.
    pub fn on_session_lost(&mut self) {
        self.book.on_session_lost();
        self.losses += 1;
    }

    /// Offer one decoded image to the book.
    pub fn apply(&mut self, snapshot: L2Snapshot) -> ApplyOutcome {
        let o = self.book.apply(snapshot);
        match o {
            ApplyOutcome::Applied { missed } => {
                self.applied += 1;
                self.missed += missed;
            }
            ApplyOutcome::Rejected(_) => self.rejected += 1,
        }
        o
    }

    /// Decode an AppMessage payload (after SessionMessageHeader) carrying an
    /// `L2Book` image and offer it to the book.
    ///
    /// # Errors
    ///
    /// The payload is not a well-formed AppMessage(L2Book).
    pub fn on_app_payload(
        &mut self,
        payload: &[u8],
    ) -> Result<ApplyOutcome, Box<dyn std::error::Error>> {
        Ok(self.apply(decode_snapshot(payload)?))
    }
}

/// Decode an AppMessage payload (after SessionMessageHeader) into the
/// `L2Book` image it carries.
///
/// # Errors
///
/// The payload is not a well-formed AppMessage(L2Book).
pub fn decode_snapshot(payload: &[u8]) -> Result<L2Snapshot, Box<dyn std::error::Error>> {
    let app = AppMessageDecoder::try_decode(payload, 0)?;
    let (_name, after) = app.into_app_name()?;
    let (inner, _done) = after.into_payload()?;
    let book = L2BookDecoder::try_decode(inner, 0)?;
    let epoch = book.epoch();
    let sequence = book.sequence();
    let exchange_ts_ns = book.exchange_timestamp();

    let mut bids = Vec::new();
    let mut bid_iter = book.into_bids()?;
    for entry in &mut bid_iter {
        bids.push(level(entry.price_wire(), entry.size_wire()));
    }
    let mut asks = Vec::new();
    let mut ask_iter = bid_iter.into_asks()?;
    for entry in &mut ask_iter {
        asks.push(level(entry.price_wire(), entry.size_wire()));
    }
    let (symbol, _) = ask_iter.into_symbol_as_str()?;

    Ok(L2Snapshot {
        epoch,
        sequence,
        symbol: symbol.to_owned(),
        exchange_ts_ns,
        bids,
        asks,
    })
}

fn level(px: DecimalDecoder<'_>, sz: DecimalDecoder<'_>) -> Level {
    Level {
        price: WireDec::new(px.mantissa(), px.exponent()),
        size: WireDec::new(sz.mantissa(), sz.exponent()),
    }
}
