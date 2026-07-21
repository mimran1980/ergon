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
        // Bids best-first: descending price, truncated to the maintained depth.
        self.bid_scratch.extend(
            self.bids
                .values()
                .rev()
                .take(crate::config::MAX_BOOK_LEVELS)
                .copied(),
        );
        self.ask_scratch.clear();
        // Asks best-first: ascending price, truncated to the maintained depth.
        self.ask_scratch.extend(
            self.asks
                .values()
                .take(crate::config::MAX_BOOK_LEVELS)
                .copied(),
        );
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
        Ok(wd) => Ok(WireDec::new(wd.mantissa, wd.exponent)),
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

// ── WebSocket frame parsing adapter ─────────────────────────────────────

/// Frame-level parse failure (JSON shape, not market data values).
#[derive(Debug)]
pub enum FrameError {
    /// The text is not valid JSON.
    Json(serde_json::Error),
    /// Valid JSON with an unrecognised shape.
    UnknownShape,
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "frame is not valid JSON: {e}"),
            Self::UnknownShape => write!(f, "unrecognised frame shape"),
        }
    }
}

impl std::error::Error for FrameError {}

/// Snapshot vs incremental update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookAction {
    Snapshot,
    Update,
}

/// One parsed trade (owned strings from the JSON edge).
#[derive(Debug)]
pub struct ParsedTrade {
    pub exchange_ts_ns: u64,
    pub price: String,
    pub size: String,
    pub is_buy: bool,
}

/// An owned, parsed WebSocket frame. Convert to borrowed events and feed the
/// ingestor with [`apply_to`](Self::apply_to).
#[derive(Debug)]
pub enum ParsedFrame {
    Book {
        action: BookAction,
        symbol: String,
        exchange_ts_ns: u64,
        bids: Vec<[String; 2]>,
        asks: Vec<[String; 2]>,
    },
    Trades {
        symbol: String,
        trades: Vec<ParsedTrade>,
    },
    /// Subscribe acks, pongs, and other non-market frames.
    Control,
}

impl ParsedFrame {
    /// Apply this frame's event(s) to the ingestor.
    ///
    /// # Errors
    ///
    /// Bubbles [`ApplyError`] from the ingestor unchanged.
    pub fn apply_to<E, F>(&self, ing: &mut BitgetIngestor, mut emit: F) -> Result<(), ApplyError<E>>
    where
        F: FnMut(NormalizedEventRef<'_>) -> Result<(), E>,
    {
        match self {
            Self::Control => Ok(()),
            Self::Book {
                action,
                symbol,
                exchange_ts_ns,
                bids,
                asks,
            } => {
                let bid_refs: Vec<[&str; 2]> = bids
                    .iter()
                    .map(|l| [l[0].as_str(), l[1].as_str()])
                    .collect();
                let ask_refs: Vec<[&str; 2]> = asks
                    .iter()
                    .map(|l| [l[0].as_str(), l[1].as_str()])
                    .collect();
                let event = match action {
                    BookAction::Snapshot => BitgetEventRef::BookSnapshot {
                        symbol,
                        exchange_ts_ns: *exchange_ts_ns,
                        bids: &bid_refs,
                        asks: &ask_refs,
                    },
                    BookAction::Update => BitgetEventRef::BookUpdate {
                        symbol,
                        exchange_ts_ns: *exchange_ts_ns,
                        bids: &bid_refs,
                        asks: &ask_refs,
                    },
                };
                ing.apply(event, &mut emit)
            }
            Self::Trades { symbol, trades } => {
                for t in trades {
                    ing.apply(
                        BitgetEventRef::Trade {
                            symbol,
                            exchange_ts_ns: t.exchange_ts_ns,
                            price: &t.price,
                            size: &t.size,
                            is_buy: t.is_buy,
                        },
                        &mut emit,
                    )?;
                }
                Ok(())
            }
        }
    }
}

/// Bitget v2 public WS wire shapes (JSON edge only).
mod wire {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Frame {
        pub event: Option<String>,
        pub action: Option<String>,
        pub arg: Option<Arg>,
        pub data: Option<serde_json::Value>,
    }

    #[derive(Deserialize)]
    pub struct Arg {
        pub channel: Option<String>,
        #[serde(rename = "instId")]
        pub inst_id: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct BookData {
        pub bids: Option<Vec<[String; 2]>>,
        pub asks: Option<Vec<[String; 2]>>,
        pub ts: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct TradeData {
        pub ts: Option<String>,
        pub price: String,
        pub size: String,
        pub side: String,
    }
}

fn ms_str_to_ns(ts: Option<&str>) -> u64 {
    ts.and_then(|s| s.parse::<u64>().ok())
        .map_or(0, |ms| ms * 1_000_000)
}

/// Parse one Bitget public WebSocket text frame.
///
/// Market data values stay as exact strings; only the JSON envelope is
/// interpreted here. `pong` and event acks parse as [`ParsedFrame::Control`].
///
/// # Errors
///
/// [`FrameError::Json`] for invalid JSON, [`FrameError::UnknownShape`] for
/// valid JSON that is neither a control frame nor a known channel.
pub fn parse_frame(text: &str) -> Result<ParsedFrame, FrameError> {
    if text == "pong" || text == "ping" {
        return Ok(ParsedFrame::Control);
    }
    let frame: wire::Frame = serde_json::from_str(text).map_err(FrameError::Json)?;
    if frame.event.is_some() {
        return Ok(ParsedFrame::Control);
    }
    let (Some(action), Some(arg), Some(data)) = (frame.action, frame.arg, frame.data) else {
        return Err(FrameError::UnknownShape);
    };
    let symbol = arg.inst_id.unwrap_or_default();
    match arg.channel.as_deref() {
        Some("books") | Some("books1") | Some("books5") | Some("books15") => {
            let mut books: Vec<wire::BookData> =
                serde_json::from_value(data).map_err(FrameError::Json)?;
            let Some(book) = books.pop() else {
                return Err(FrameError::UnknownShape);
            };
            Ok(ParsedFrame::Book {
                action: if action == "snapshot" {
                    BookAction::Snapshot
                } else {
                    BookAction::Update
                },
                symbol,
                exchange_ts_ns: ms_str_to_ns(book.ts.as_deref()),
                bids: book.bids.unwrap_or_default(),
                asks: book.asks.unwrap_or_default(),
            })
        }
        Some("trade") => {
            let trades: Vec<wire::TradeData> =
                serde_json::from_value(data).map_err(FrameError::Json)?;
            Ok(ParsedFrame::Trades {
                symbol,
                trades: trades
                    .into_iter()
                    .map(|t| ParsedTrade {
                        exchange_ts_ns: ms_str_to_ns(t.ts.as_deref()),
                        price: t.price,
                        size: t.size,
                        is_buy: t.side == "buy",
                    })
                    .collect(),
            })
        }
        _ => Err(FrameError::UnknownShape),
    }
}
