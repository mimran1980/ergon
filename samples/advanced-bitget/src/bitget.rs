//! `BitgetIngestor` — the pure Bitget → normalized state machine.
//!
//! Owns WebSocket-frame-shaped borrowed events in, borrowed
//! [`NormalizedEventRef`] out through a generic callback. Callback errors
//! bubble unchanged (no boxing). Book publication is suppressed until a
//! valid snapshot arrives, and again after every disconnect.

use std::collections::BTreeMap;

use crate::counters::Counters;
use crate::decimal::{DecimalConvertError, parse_decimal_exact};
use crate::market::{Level, NormalizedEventRef, WireDec};

/// Borrowed Bitget event at the WebSocket seam. Price/size values stay as
/// text until `apply` parses them exactly.
#[derive(Debug, Clone, Copy)]
pub enum BitgetEventRef<'a> {
    BookSnapshot {
        symbol: &'a str,
        exchange_ts_ns: u64,
        bids: &'a [[&'a str; 2]],
        asks: &'a [[&'a str; 2]],
    },
    BookUpdate {
        symbol: &'a str,
        exchange_ts_ns: u64,
        bids: &'a [[&'a str; 2]],
        asks: &'a [[&'a str; 2]],
    },
    Trade {
        symbol: &'a str,
        exchange_ts_ns: u64,
        price: &'a str,
        size: &'a str,
        is_buy: bool,
    },
    Heartbeat,
}

/// Structured apply failure. `Emit` bubbles the callback error unchanged.
#[derive(Debug, PartialEq)]
pub enum ApplyError<E> {
    Emit(E),
    MalformedDecimal {
        value_kind: &'static str,
        error: DecimalConvertError,
    },
}

/// Ordered-book key: exact numeric ordering across mixed exponents.
type PriceKey = rust_decimal::Decimal;

fn price_key(d: WireDec) -> PriceKey {
    // Exponents come from parse_decimal_exact and are always ≤ 0, within
    // rust_decimal's scale range.
    rust_decimal::Decimal::from_i128_with_scale(
        i128::from(d.mantissa),
        u32::from(d.exponent.unsigned_abs()),
    )
}

#[derive(Default)]
pub struct BitgetIngestor {
    bids: BTreeMap<PriceKey, Level>,
    asks: BTreeMap<PriceKey, Level>,
    have_snapshot: bool,
    sequence: u64,
    counters: Counters,
    // Reused emission scratch — retains capacity across events.
    bid_scratch: Vec<Level>,
    ask_scratch: Vec<Level>,
}

impl BitgetIngestor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    /// Clear book state after a disconnect; publication stays suppressed
    /// until the next snapshot.
    pub fn on_disconnect(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.have_snapshot = false;
        self.counters.reconnects += 1;
    }

    /// Apply one event, emitting zero or one normalized event through `emit`.
    pub fn apply<E, F>(
        &mut self,
        event: BitgetEventRef<'_>,
        mut emit: F,
    ) -> Result<(), ApplyError<E>>
    where
        F: FnMut(NormalizedEventRef<'_>) -> Result<(), E>,
    {
        match event {
            BitgetEventRef::Heartbeat => Ok(()),
            BitgetEventRef::Trade {
                symbol,
                exchange_ts_ns,
                price,
                size,
                is_buy,
            } => {
                let price = parse_wire(price, "trade price", &mut self.counters)?;
                let size = parse_wire(size, "trade size", &mut self.counters)?;
                self.sequence += 1;
                self.counters.trades_emitted += 1;
                emit(NormalizedEventRef::Trade {
                    symbol,
                    exchange_ts_ns,
                    receive_ts_ns: now_ns(),
                    sequence: self.sequence,
                    price,
                    size,
                    is_buy,
                })
                .map_err(ApplyError::Emit)
            }
            BitgetEventRef::BookSnapshot {
                symbol,
                exchange_ts_ns,
                bids,
                asks,
            } => {
                // Parse both sides fully before touching state: a malformed
                // snapshot must not leave a half-applied book.
                let parsed_bids = parse_side(bids, "bid", &mut self.counters)?;
                let parsed_asks = parse_side(asks, "ask", &mut self.counters)?;
                self.bids.clear();
                self.asks.clear();
                for l in parsed_bids {
                    self.bids.insert(price_key(l.price), l);
                }
                for l in parsed_asks {
                    self.asks.insert(price_key(l.price), l);
                }
                self.have_snapshot = true;
                self.emit_book(symbol, exchange_ts_ns, emit)
            }
            BitgetEventRef::BookUpdate {
                symbol,
                exchange_ts_ns,
                bids,
                asks,
            } => {
                if !self.have_snapshot {
                    self.counters.updates_before_snapshot += 1;
                    return Ok(());
                }
                let parsed_bids = parse_side(bids, "bid", &mut self.counters)?;
                let parsed_asks = parse_side(asks, "ask", &mut self.counters)?;
                apply_levels(&mut self.bids, parsed_bids);
                apply_levels(&mut self.asks, parsed_asks);
                self.emit_book(symbol, exchange_ts_ns, emit)
            }
        }
    }

    fn emit_book<E, F>(
        &mut self,
        symbol: &str,
        exchange_ts_ns: u64,
        mut emit: F,
    ) -> Result<(), ApplyError<E>>
    where
        F: FnMut(NormalizedEventRef<'_>) -> Result<(), E>,
    {
        self.bid_scratch.clear();
        // Bids best-first: descending price.
        self.bid_scratch.extend(self.bids.values().rev().copied());
        self.ask_scratch.clear();
        // Asks best-first: ascending price.
        self.ask_scratch.extend(self.asks.values().copied());
        self.sequence += 1;
        self.counters.books_emitted += 1;
        emit(NormalizedEventRef::L2Book {
            symbol,
            exchange_ts_ns,
            receive_ts_ns: now_ns(),
            sequence: self.sequence,
            bids: &self.bid_scratch,
            asks: &self.ask_scratch,
        })
        .map_err(ApplyError::Emit)
    }
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn parse_wire<E>(
    s: &str,
    value_kind: &'static str,
    counters: &mut Counters,
) -> Result<WireDec, ApplyError<E>> {
    match parse_decimal_exact(s) {
        Ok((m, e)) => Ok(WireDec::new(m, e)),
        Err(error) => {
            counters.malformed_values += 1;
            Err(ApplyError::MalformedDecimal { value_kind, error })
        }
    }
}

fn parse_side<E>(
    raw: &[[&str; 2]],
    value_kind: &'static str,
    counters: &mut Counters,
) -> Result<Vec<Level>, ApplyError<E>> {
    raw.iter()
        .map(|[p, s]| {
            Ok(Level {
                price: parse_wire(p, value_kind, counters)?,
                size: parse_wire(s, value_kind, counters)?,
            })
        })
        .collect()
}

/// Apply update levels: size 0 deletes, otherwise insert/replace.
fn apply_levels(book: &mut BTreeMap<PriceKey, Level>, levels: Vec<Level>) {
    for l in levels {
        let key = price_key(l.price);
        if l.size.mantissa == 0 {
            book.remove(&key);
        } else {
            book.insert(key, l);
        }
    }
}
