//! `ClaimPublisher` — typed SBE publication seam.
//!
//! Hides exact length computation, `try_claim_owned`, direct SBE encoding
//! into the claim, commit, and classified drop counters. Backpressure is one
//! immediate drop with no retry. Two adapters make this a real seam:
//! [`RecordingPublication`] for tests and [`AeronPublication`] for Rusteron.

use crate::config::APP_NAME;
use crate::market::NormalizedEventRef;
use crate::normalized_app::{
    AppMessageEncoder, Decimal, L2BookEncoder, Side, Source, TradeEncoder, sbe_rt,
};

/// Classified claim-drop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    Backpressured,
    NotConnected,
    AdminAction,
    Closed,
    MaxPosition,
}

/// Outcome of one publish attempt. No retry is ever performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Published,
    Dropped(DropReason),
    EncodeFailed,
    CommitFailed,
}

/// One Aeron-publication-shaped claim sink.
pub trait Publication {
    fn try_claim_and_commit<F>(&mut self, len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>;
}

// ── Recording adapter (tests/diagnostics) ─────────────────────────────

#[derive(Default)]
pub struct RecordingPublication {
    pub committed: Vec<Vec<u8>>,
    pub claimed_lengths: Vec<usize>,
    pub claim_attempts: usize,
    pub fail_with: Option<DropReason>,
    pub short_by: usize,
}

impl RecordingPublication {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn failing(reason: DropReason) -> Self {
        Self {
            fail_with: Some(reason),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn short(n: usize) -> Self {
        Self {
            short_by: n,
            ..Self::default()
        }
    }
}

impl Publication for RecordingPublication {
    fn try_claim_and_commit<F>(&mut self, len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>,
    {
        self.claim_attempts += 1;
        if let Some(reason) = self.fail_with {
            return PublishOutcome::Dropped(reason);
        }
        let mut buf = vec![0; len.saturating_sub(self.short_by)];
        match fill(&mut buf) {
            Ok(()) => {
                self.claimed_lengths.push(len);
                self.committed.push(buf);
                PublishOutcome::Published
            }
            Err(_) => PublishOutcome::EncodeFailed,
        }
    }
}

// ── Rusteron adapter ──────────────────────────────────────────────────

/// Rusteron [`Publication`] adapter over an exclusive IPC publication.
pub struct AeronPublication(pub rusteron_client::AeronExclusivePublication);

impl Publication for AeronPublication {
    fn try_claim_and_commit<F>(&mut self, len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>,
    {
        use rusteron_client::AeronOfferError as E;
        match self.0.try_claim_owned(len) {
            Ok(mut claim) => match fill(claim.data()) {
                Ok(()) => match claim.commit() {
                    Ok(_) => PublishOutcome::Published,
                    Err(_) => PublishOutcome::CommitFailed,
                },
                Err(_) => PublishOutcome::EncodeFailed,
            },
            Err(E::BackPressured) => PublishOutcome::Dropped(DropReason::Backpressured),
            Err(E::NotConnected) => PublishOutcome::Dropped(DropReason::NotConnected),
            Err(E::AdminAction) => PublishOutcome::Dropped(DropReason::AdminAction),
            Err(E::Closed) => PublishOutcome::Dropped(DropReason::Closed),
            Err(E::MaxPositionExceeded) => PublishOutcome::Dropped(DropReason::MaxPosition),
            Err(_) => PublishOutcome::Dropped(DropReason::Closed),
        }
    }
}

// ── ClaimPublisher ────────────────────────────────────────────────────

/// Simple counters for publication diagnostics.
#[derive(Default)]
pub struct Counters {
    pub published: u64,
    pub dropped_backpressure: u64,
    pub dropped_not_connected: u64,
    pub dropped_admin_action: u64,
    pub dropped_closed: u64,
    pub dropped_max_position: u64,
    pub encode_failures: u64,
    pub commit_failures: u64,
}

/// Publishes normalized events as SBE messages directly into Aeron claims.
pub struct ClaimPublisher<P> {
    typed: P,
    counters: Counters,
}

impl<P: Publication> ClaimPublisher<P> {
    pub fn new(typed: P) -> Self {
        Self {
            typed,
            counters: Counters::default(),
        }
    }

    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    pub fn into_adapter(self) -> P {
        self.typed
    }

    /// Publish one normalized event. Never retries.
    pub fn publish(&mut self, ev: &NormalizedEventRef<'_>) -> PublishOutcome {
        match ev {
            NormalizedEventRef::L2Book {
                symbol,
                exchange_ts_ns,
                receive_ts_ns,
                sequence,
                bids,
                asks,
            } => {
                let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
                    bids.len(),
                    asks.len(),
                    symbol.len(),
                );
                let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
                    APP_NAME.len(),
                    inner_len,
                );
                let outcome = self.typed.try_claim_and_commit(outer_len, |buf| {
                    let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0);
                    app.sent_ts(*receive_ts_ns);
                    let after = app.app_name(APP_NAME)?;
                    let _ = after.payload_with(
                        inner_len,
                        |payload| -> Result<(), sbe_rt::EncodeError> {
                            let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0);
                            enc.source(Source::Bitget)
                                .exchange_timestamp(*exchange_ts_ns)
                                .receive_timestamp(*receive_ts_ns)
                                .sequence(*sequence);
                            let after = enc.bids(bids.len() as u16, |g| {
                                for l in *bids {
                                    g.add(|e| -> Result<(), sbe_rt::EncodeError> {
                                        e.price_wire(Decimal::new(
                                            l.price.mantissa,
                                            l.price.exponent,
                                        ))
                                        .size_wire(Decimal::new(l.size.mantissa, l.size.exponent));
                                        Ok(())
                                    })?;
                                }
                                Ok::<(), sbe_rt::EncodeError>(())
                            })?;
                            let after = after.asks(asks.len() as u16, |g| {
                                for l in *asks {
                                    g.add(|e| -> Result<(), sbe_rt::EncodeError> {
                                        e.price_wire(Decimal::new(
                                            l.price.mantissa,
                                            l.price.exponent,
                                        ))
                                        .size_wire(Decimal::new(l.size.mantissa, l.size.exponent));
                                        Ok(())
                                    })?;
                                }
                                Ok::<(), sbe_rt::EncodeError>(())
                            })?;
                            let _complete = after.symbol(symbol.as_bytes())?;
                            Ok(())
                        },
                    )?;
                    Ok(())
                });
                self.count(outcome);
                outcome
            }
            NormalizedEventRef::Trade {
                symbol,
                exchange_ts_ns,
                receive_ts_ns,
                sequence,
                price,
                size,
                is_buy,
            } => {
                let inner_len =
                    TradeEncoder::compute_encoded_length_with_message_header(symbol.len());
                let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
                    APP_NAME.len(),
                    inner_len,
                );
                let outcome = self.typed.try_claim_and_commit(outer_len, |buf| {
                    let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0);
                    app.sent_ts(*receive_ts_ns);
                    let after = app.app_name(APP_NAME)?;
                    let _ = after.payload_with(
                        inner_len,
                        |payload| -> Result<(), sbe_rt::EncodeError> {
                            let mut enc = TradeEncoder::wrap_and_apply_header(payload, 0);
                            enc.source(Source::Bitget)
                                .exchange_timestamp(*exchange_ts_ns)
                                .receive_timestamp(*receive_ts_ns)
                                .trade_id(*sequence)
                                .price_wire(Decimal::new(price.mantissa, price.exponent))
                                .size_wire(Decimal::new(size.mantissa, size.exponent))
                                .side(if *is_buy { Side::Buy } else { Side::Sell });
                            let _complete = enc.symbol(symbol.as_bytes())?;
                            Ok(())
                        },
                    )?;
                    Ok(())
                });
                self.count(outcome);
                outcome
            }
        }
    }

    fn count(&mut self, outcome: PublishOutcome) {
        match outcome {
            PublishOutcome::Published => self.counters.published += 1,
            PublishOutcome::Dropped(DropReason::Backpressured) => {
                self.counters.dropped_backpressure += 1;
            }
            PublishOutcome::Dropped(DropReason::NotConnected) => {
                self.counters.dropped_not_connected += 1;
            }
            PublishOutcome::Dropped(DropReason::AdminAction) => {
                self.counters.dropped_admin_action += 1;
            }
            PublishOutcome::Dropped(DropReason::Closed) => self.counters.dropped_closed += 1,
            PublishOutcome::Dropped(DropReason::MaxPosition) => {
                self.counters.dropped_max_position += 1;
            }
            PublishOutcome::EncodeFailed => self.counters.encode_failures += 1,
            PublishOutcome::CommitFailed => self.counters.commit_failures += 1,
        }
    }
}

// ── MTU derivation ────────────────────────────────────────────────────

/// Worst-case typed claim length for an `AppMessage(L2Book)` with
/// [`MAX_BOOK_LEVELS`](crate::config::MAX_BOOK_LEVELS) levels per side and a
/// 16-byte symbol.
#[must_use]
pub fn worst_case_typed_claim_len() -> usize {
    let inner = L2BookEncoder::compute_encoded_length_with_message_header(
        crate::config::MAX_BOOK_LEVELS,
        crate::config::MAX_BOOK_LEVELS,
        16,
    );
    AppMessageEncoder::compute_encoded_length_with_message_header(APP_NAME.len(), inner)
}

/// IPC MTU sized so the largest maintained message fits one claim, rounded up
/// to a power of two and never below Aeron's 1408-byte default.
#[must_use]
pub fn derive_ipc_mtu() -> usize {
    (worst_case_typed_claim_len() + 32)
        .next_power_of_two()
        .max(1408)
}
