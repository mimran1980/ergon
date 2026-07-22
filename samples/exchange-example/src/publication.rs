//! `ClaimPublisher` — the Task 7/9 publication seam.
//!
//! Hides exact length computation, `try_claim_owned`, direct SBE encoding
//! into the claim, commit, and classified drop counters. Backpressure is one
//! immediate drop with no retry. Two adapters make this a real seam:
//! [`RecordingPublication`] for tests and [`AeronPublication`] for Rusteron.

use ergo_clickhouse_persist::ColumnType;
use ergo_clickhouse_persist::dynamic::{
    DynamicRecorderBuilder, DynamicRecorderError, DynamicRecorderV2, DynamicValueRef,
};

use crate::config::APP_NAME;
use crate::counters::Counters;
use crate::market::NormalizedEventRef;
use crate::normalized_app::{
    AppMessageEncoder, Decimal, L2BookEncoder, Side, Source, TradeEncoder, sbe_rt,
};

/// Classified claim-drop reason (maps 1:1 onto Aeron claim results).
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
    /// Claim exactly `len` bytes, run `fill` to encode directly into the
    /// claim, then commit. Encoding failure aborts the claim.
    fn try_claim_and_commit<F>(&mut self, len: usize, fill: F) -> PublishOutcome
    where
        F: FnOnce(&mut [u8]) -> Result<(), sbe_rt::EncodeError>;
}

// ── Recording adapter (tests/diagnostics) ─────────────────────────────

/// In-memory [`Publication`] adapter: captures committed claims, records
/// claim lengths and attempts, and can simulate any drop reason.
#[derive(Default)]
pub struct RecordingPublication {
    pub committed: Vec<Vec<u8>>,
    pub claimed_lengths: Vec<usize>,
    pub claim_attempts: usize,
    pub fail_with: Option<DropReason>,
    /// Hand the fill closure a buffer this many bytes SHORTER than the
    /// claimed length, simulating an encode failure inside the claim.
    pub short_by: usize,
}

impl RecordingPublication {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An adapter whose every claim fails with `reason`.
    #[must_use]
    pub fn failing(reason: DropReason) -> Self {
        Self {
            fail_with: Some(reason),
            ..Self::default()
        }
    }

    /// An adapter whose claims come up `n` bytes short, forcing the encoder
    /// inside the claim to fail.
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
        let mut buf = vec![0u8; len.saturating_sub(self.short_by)];
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
                // Claim auto-aborts on drop; nothing partial is published.
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

/// Publishes normalized events as SBE messages directly into Aeron claims.
///
/// - L2 books → `AppMessage(payload = L2Book)` on the typed stream plus one
///   `DynamicRowV2` (same correlation sequence) on the dynamic stream.
/// - Trades → `AppMessage(payload = Trade)` on the typed stream.
pub struct ClaimPublisher<P> {
    typed: P,
    dynamic: P,
    recorder: DynamicRecorderV2,
    counters: Counters,
    // Reused decimal-array scratch (capacity retained across publishes).
    bid_prices: Vec<(i64, i8)>,
    bid_sizes: Vec<(i64, i8)>,
    ask_prices: Vec<(i64, i8)>,
    ask_sizes: Vec<(i64, i8)>,
}

fn decimal_38_18_array() -> ColumnType {
    ColumnType::Array(Box::new(ColumnType::Decimal {
        precision: 38,
        scale: 18,
    }))
}

impl<P: Publication> ClaimPublisher<P> {
    /// # Errors
    ///
    /// Returns the recorder construction error if the dynamic table
    /// definition is rejected.
    pub fn new(typed: P, dynamic: P) -> Result<Self, DynamicRecorderError> {
        let recorder = DynamicRecorderBuilder::new("l2book_dynamic")
            .field("sequence", ColumnType::UInt64)
            .field("exchange_ts", ColumnType::UInt64)
            .field("symbol", ColumnType::String)
            .field("bid_prices", decimal_38_18_array())
            .field("bid_sizes", decimal_38_18_array())
            .field("ask_prices", decimal_38_18_array())
            .field("ask_sizes", decimal_38_18_array())
            .build_v2()?;
        Ok(Self {
            typed,
            dynamic,
            recorder,
            counters: Counters::default(),
            bid_prices: Vec::new(),
            bid_sizes: Vec::new(),
            ask_prices: Vec::new(),
            ask_sizes: Vec::new(),
        })
    }

    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    /// Schema id of the dynamic L2 row table definition.
    pub fn dynamic_schema_id(&self) -> u32 {
        self.recorder.schema_id()
    }

    /// Consume the publisher and return both adapters (test inspection).
    pub fn into_adapters(self) -> (P, P) {
        (self.typed, self.dynamic)
    }

    /// Publish the `DynamicSchemaV2` message on the dynamic stream. Called
    /// once after both subscribers are connected and before live ingestion.
    pub fn publish_schema(&mut self) -> PublishOutcome {
        let len = self.recorder.schema_encoded_length();
        let recorder = &self.recorder;
        let outcome = self.dynamic.try_claim_and_commit(len, |buf| {
            recorder
                .schema_into(buf)
                .map(|_| ())
                .map_err(|_| sbe_rt::EncodeError::BufferTooShort {
                    needed: len,
                    available: buf.len(),
                })
        });
        if outcome == PublishOutcome::Published {
            self.counters.schemas_published += 1;
        }
        self.count(outcome);
        outcome
    }

    /// Publish one normalized event. Never retries; every claim outcome is
    /// classified into `counters`.
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
                let typed_outcome = self.typed.try_claim_and_commit(outer_len, |buf| {
                    let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0)?;
                    app.sent_ts(*receive_ts_ns);
                    let after = app.app_name(APP_NAME)?;
                    let _ = after.payload_with(
                        inner_len,
                        |payload| -> Result<(), sbe_rt::EncodeError> {
                            let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
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
                                        .size_wire(Decimal::new(
                                            l.size.mantissa,
                                            l.size.exponent,
                                        ));
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
                                        .size_wire(Decimal::new(
                                            l.size.mantissa,
                                            l.size.exponent,
                                        ));
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
                self.count(typed_outcome);
                if typed_outcome != PublishOutcome::Published {
                    return typed_outcome;
                }

                // Dynamic V2 row with the same correlation sequence.
                self.bid_prices.clear();
                self.bid_sizes.clear();
                self.ask_prices.clear();
                self.ask_sizes.clear();
                for l in *bids {
                    self.bid_prices.push((l.price.mantissa, l.price.exponent));
                    self.bid_sizes.push((l.size.mantissa, l.size.exponent));
                }
                for l in *asks {
                    self.ask_prices.push((l.price.mantissa, l.price.exponent));
                    self.ask_sizes.push((l.size.mantissa, l.size.exponent));
                }
                let values = [
                    DynamicValueRef::UInt64(*sequence),
                    DynamicValueRef::UInt64(*exchange_ts_ns),
                    DynamicValueRef::String(symbol),
                    DynamicValueRef::DecimalArray(&self.bid_prices),
                    DynamicValueRef::DecimalArray(&self.bid_sizes),
                    DynamicValueRef::DecimalArray(&self.ask_prices),
                    DynamicValueRef::DecimalArray(&self.ask_sizes),
                ];
                let Ok(row_len) = self.recorder.compute_encoded_length(&values) else {
                    self.counters.encode_failures += 1;
                    return PublishOutcome::EncodeFailed;
                };
                let recorder = &self.recorder;
                let dyn_outcome = self.dynamic.try_claim_and_commit(row_len, |buf| {
                    recorder.record_into(buf, &values).map(|_| ()).map_err(|_| {
                        sbe_rt::EncodeError::BufferTooShort {
                            needed: row_len,
                            available: buf.len(),
                        }
                    })
                });
                self.count(dyn_outcome);
                dyn_outcome
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
                    let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0)?;
                    app.sent_ts(*receive_ts_ns);
                    let after = app.app_name(APP_NAME)?;
                    let _ = after.payload_with(
                        inner_len,
                        |payload| -> Result<(), sbe_rt::EncodeError> {
                            let mut enc = TradeEncoder::wrap_and_apply_header(payload, 0)?;
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

/// Worst-case typed claim length: an `AppMessage(L2Book)` with
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

/// IPC MTU sized so the largest maintained message fits one claim
/// (claim limit is MTU minus the 32-byte data frame header), rounded up to a
/// power of two and never below Aeron's 1408-byte default.
#[must_use]
pub fn derive_ipc_mtu() -> usize {
    (worst_case_typed_claim_len() + 32)
        .next_power_of_two()
        .max(1408)
}
