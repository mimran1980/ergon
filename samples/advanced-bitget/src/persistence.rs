//! `ForegroundPersistor` — the Task 7/10 persistence seam.
//!
//! Owns ordered typed/dynamic matching by correlation sequence and row
//! conversion to exact ClickHouse `Decimal(38,18)` scaled integers. Database
//! writes go through the [`RowSink`] seam: [`InMemorySink`] for tests and a
//! ClickHouse adapter for the live pipeline.

use std::collections::VecDeque;

use ergo_clickhouse_persist::sbe::v2::DynamicRowV2Decoder;

use crate::counters::Counters;
use crate::decimal::{DecimalConvertError, to_clickhouse_decimal};
use crate::normalized_app::{AnyMessage, AppMessageDecoder, Side};

/// A decoded L2 book in wire (mantissa, exponent) form, used for exact
/// typed/dynamic comparison before conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WireBook {
    sequence: u64,
    exchange_ts_ns: u64,
    symbol: String,
    /// ((price m, e), (size m, e)) best-first.
    bids: Vec<((i64, i8), (i64, i8))>,
    asks: Vec<((i64, i8), (i64, i8))>,
}

/// One L2 book row with exact `Decimal(38,18)` scaled integers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2BookRow {
    pub sequence: u64,
    pub exchange_ts_ns: u64,
    pub symbol: String,
    pub bid_prices: Vec<i128>,
    pub bid_sizes: Vec<i128>,
    pub ask_prices: Vec<i128>,
    pub ask_sizes: Vec<i128>,
}

/// One trade row with exact `Decimal(38,18)` scaled integers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRow {
    pub trade_id: u64,
    pub exchange_ts_ns: u64,
    pub symbol: String,
    pub price: i128,
    pub size: i128,
    pub is_buy: bool,
}

/// Structured persistence failure.
#[derive(Debug)]
pub enum PersistError {
    /// SBE decode failure (malformed or wrong-schema bytes).
    Decode(String),
    /// AppMessage payload was itself an AppMessage or other infrastructure
    /// message — rejected by contract.
    RecursivePayload,
    /// Exact Decimal(38,18) conversion failed.
    Convert(DecimalConvertError),
    /// Database write failed.
    Sink(String),
}

impl core::fmt::Display for PersistError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(m) => write!(f, "decode failure: {m}"),
            Self::RecursivePayload => write!(f, "recursive AppMessage payload rejected"),
            Self::Convert(e) => write!(f, "decimal conversion failure: {e}"),
            Self::Sink(m) => write!(f, "sink failure: {m}"),
        }
    }
}

impl std::error::Error for PersistError {}

/// Database seam. Errors are strings — this is the cold path.
pub trait RowSink {
    fn insert_l2book_typed(&mut self, row: &L2BookRow) -> Result<(), String>;
    fn insert_l2book_dynamic(&mut self, row: &L2BookRow) -> Result<(), String>;
    fn insert_trade(&mut self, row: &TradeRow) -> Result<(), String>;
    fn flush(&mut self) -> Result<(), String>;
}

/// In-memory [`RowSink`] adapter for tests and diagnostics.
#[derive(Debug, Default)]
pub struct InMemorySink {
    pub l2book_typed: Vec<L2BookRow>,
    pub l2book_dynamic: Vec<L2BookRow>,
    pub trade: Vec<TradeRow>,
    pub flushes: usize,
}

impl RowSink for InMemorySink {
    fn insert_l2book_typed(&mut self, row: &L2BookRow) -> Result<(), String> {
        self.l2book_typed.push(row.clone());
        Ok(())
    }
    fn insert_l2book_dynamic(&mut self, row: &L2BookRow) -> Result<(), String> {
        self.l2book_dynamic.push(row.clone());
        Ok(())
    }
    fn insert_trade(&mut self, row: &TradeRow) -> Result<(), String> {
        self.trade.push(row.clone());
        Ok(())
    }
    fn flush(&mut self) -> Result<(), String> {
        self.flushes += 1;
        Ok(())
    }
}

/// Ordered bounded queue cap.
// ponytail: fixed cap; make configurable if live tuning ever needs it.
const MAX_QUEUE: usize = 1024;

pub struct ForegroundPersistor<S> {
    sink: S,
    typed_queue: VecDeque<WireBook>,
    dynamic_queue: VecDeque<WireBook>,
    counters: Counters,
}

impl<S: RowSink> ForegroundPersistor<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            typed_queue: VecDeque::new(),
            dynamic_queue: VecDeque::new(),
            counters: Counters::default(),
        }
    }

    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Handle one stream-1001 fragment: `AppMessage(payload = L2Book | Trade)`.
    ///
    /// # Errors
    ///
    /// Decode, recursive-payload, conversion, or sink failures. All decode
    /// failures increment `decode_failures`.
    pub fn on_typed(&mut self, bytes: &[u8]) -> Result<(), PersistError> {
        let result = self.decode_typed(bytes);
        if matches!(
            result,
            Err(PersistError::Decode(_) | PersistError::RecursivePayload)
        ) {
            self.counters.decode_failures += 1;
        }
        result
    }

    fn decode_typed(&mut self, bytes: &[u8]) -> Result<(), PersistError> {
        let dec_err =
            |e: crate::normalized_app::sbe_rt::DecodeError| PersistError::Decode(e.to_string());
        let app = AppMessageDecoder::wrap_and_apply_header(bytes, 0).map_err(dec_err)?;
        let (_name, after) = app.into_app_name().map_err(dec_err)?;
        let (frame, _complete) = after.into_payload_as_message().map_err(dec_err)?;
        match frame.message {
            AnyMessage::L2Book(book) => {
                let wire = decode_l2book(book).map_err(dec_err)?;
                self.typed_queue.push_back(wire);
                self.bound_queues();
                self.try_match()
            }
            AnyMessage::Trade(trade) => {
                let trade_id = trade.trade_id();
                let exchange_ts_ns = trade.exchange_timestamp();
                let price = (trade.price_wire().mantissa(), trade.price_wire().exponent());
                let size = (trade.size_wire().mantissa(), trade.size_wire().exponent());
                let is_buy = trade.side() == Side::Buy;
                let (sym, _) = trade.into_symbol().map_err(dec_err)?;
                let row = TradeRow {
                    trade_id,
                    exchange_ts_ns,
                    symbol: String::from_utf8_lossy(sym).into_owned(),
                    price: to_clickhouse_decimal(price.0, price.1)
                        .map_err(PersistError::Convert)?,
                    size: to_clickhouse_decimal(size.0, size.1).map_err(PersistError::Convert)?,
                    is_buy,
                };
                self.sink.insert_trade(&row).map_err(PersistError::Sink)?;
                self.counters.persisted_trades += 1;
                Ok(())
            }
            AnyMessage::AppMessage(_) => Err(PersistError::RecursivePayload),
            AnyMessage::Unknown { .. } => {
                Err(PersistError::Decode("unknown payload template".into()))
            }
        }
    }

    /// Handle one stream-1002 fragment: `DynamicRowV2`.
    ///
    /// # Errors
    ///
    /// Decode, conversion, or sink failures.
    pub fn on_dynamic(&mut self, bytes: &[u8]) -> Result<(), PersistError> {
        let result = self.decode_dynamic(bytes);
        if matches!(result, Err(PersistError::Decode(_))) {
            self.counters.decode_failures += 1;
        }
        result
    }

    fn decode_dynamic(&mut self, bytes: &[u8]) -> Result<(), PersistError> {
        let wire = decode_dynamic_book(bytes)?;
        self.dynamic_queue.push_back(wire);
        self.bound_queues();
        self.try_match()
    }

    /// Flush pending sink batches.
    ///
    /// # Errors
    ///
    /// Sink failures.
    pub fn flush(&mut self) -> Result<(), PersistError> {
        self.sink.flush().map_err(PersistError::Sink)
    }

    fn bound_queues(&mut self) {
        while self.typed_queue.len() > MAX_QUEUE {
            self.typed_queue.pop_front();
            self.counters.unmatched_dropped += 1;
        }
        while self.dynamic_queue.len() > MAX_QUEUE {
            self.dynamic_queue.pop_front();
            self.counters.unmatched_dropped += 1;
        }
    }

    /// Ordered correlation matching: equal heads are compared and persisted;
    /// the smaller unmatched head is counted and dropped.
    fn try_match(&mut self) -> Result<(), PersistError> {
        loop {
            let (Some(t), Some(d)) = (self.typed_queue.front(), self.dynamic_queue.front()) else {
                return Ok(());
            };
            if t.sequence == d.sequence {
                let t = self.typed_queue.pop_front().expect("front checked");
                let d = self.dynamic_queue.pop_front().expect("front checked");
                if t == d {
                    let row = to_row(&t).map_err(PersistError::Convert)?;
                    self.sink
                        .insert_l2book_typed(&row)
                        .map_err(PersistError::Sink)?;
                    self.counters.persisted_typed += 1;
                    self.sink
                        .insert_l2book_dynamic(&row)
                        .map_err(PersistError::Sink)?;
                    self.counters.persisted_dynamic += 1;
                } else {
                    self.counters.compare_failures += 1;
                }
            } else if t.sequence < d.sequence {
                self.typed_queue.pop_front();
                self.counters.unmatched_dropped += 1;
            } else {
                self.dynamic_queue.pop_front();
                self.counters.unmatched_dropped += 1;
            }
        }
    }
}

/// ((price m, e), (size m, e)) level pair in wire form.
type WireLevel = ((i64, i8), (i64, i8));

fn to_row(w: &WireBook) -> Result<L2BookRow, DecimalConvertError> {
    let conv = |pairs: &[WireLevel],
                pick: fn(&WireLevel) -> (i64, i8)|
     -> Result<Vec<i128>, DecimalConvertError> {
        pairs
            .iter()
            .map(|p| {
                let (m, e) = pick(p);
                to_clickhouse_decimal(m, e)
            })
            .collect()
    };
    Ok(L2BookRow {
        sequence: w.sequence,
        exchange_ts_ns: w.exchange_ts_ns,
        symbol: w.symbol.clone(),
        bid_prices: conv(&w.bids, |p| p.0)?,
        bid_sizes: conv(&w.bids, |p| p.1)?,
        ask_prices: conv(&w.asks, |p| p.0)?,
        ask_sizes: conv(&w.asks, |p| p.1)?,
    })
}

fn decode_l2book(
    book: crate::normalized_app::L2BookDecoder<'_>,
) -> Result<WireBook, crate::normalized_app::sbe_rt::DecodeError> {
    let sequence = book.sequence();
    let exchange_ts_ns = book.exchange_timestamp();
    let mut bids = Vec::new();
    let mut g = book.into_bids()?;
    for e in g.by_ref() {
        bids.push((
            (e.price().mantissa(), e.price().exponent()),
            (e.size().mantissa(), e.size().exponent()),
        ));
    }
    let after = g.finish()?;
    let mut asks = Vec::new();
    let mut g = after.into_asks()?;
    for e in g.by_ref() {
        asks.push((
            (e.price().mantissa(), e.price().exponent()),
            (e.size().mantissa(), e.size().exponent()),
        ));
    }
    let after = g.finish()?;
    let (symbol, _) = after.into_symbol()?;
    Ok(WireBook {
        sequence,
        exchange_ts_ns,
        symbol: String::from_utf8_lossy(symbol).into_owned(),
        bids,
        asks,
    })
}

/// Positional field ids as laid out by `ClaimPublisher`'s dynamic table:
/// 0 sequence, 1 exchange_ts, 2 symbol, 3-6 bid/ask price/size arrays.
fn decode_dynamic_book(bytes: &[u8]) -> Result<WireBook, PersistError> {
    let dec_err = |e: ergo_clickhouse_persist::sbe::v2::sbe_rt::DecodeError| {
        PersistError::Decode(e.to_string())
    };
    let dec = DynamicRowV2Decoder::wrap_and_apply_header(bytes, 0).map_err(dec_err)?;
    let dec = dec
        .into_row_metadata()
        .map_err(dec_err)?
        .finish()
        .map_err(dec_err)?;
    let dec = dec
        .into_int64_fields()
        .map_err(dec_err)?
        .finish()
        .map_err(dec_err)?;

    let mut sequence = 0u64;
    let mut exchange_ts_ns = 0u64;
    let mut g = dec.into_uint64_fields().map_err(dec_err)?;
    for e in g.by_ref() {
        match e.field_id() {
            0 => sequence = e.value(),
            1 => exchange_ts_ns = e.value(),
            _ => return Err(PersistError::Decode("unexpected uint64 field".into())),
        }
    }
    let dec = g.finish().map_err(dec_err)?;
    let dec = dec
        .into_float64_fields()
        .map_err(dec_err)?
        .finish()
        .map_err(dec_err)?;
    let dec = dec
        .into_bool_fields()
        .map_err(dec_err)?
        .finish()
        .map_err(dec_err)?;

    let mut string_lens: Vec<(u8, usize)> = Vec::new();
    let mut g = dec.into_string_fields().map_err(dec_err)?;
    for e in g.by_ref() {
        string_lens.push((e.field_id(), e.str_len() as usize));
    }
    let dec = g.finish().map_err(dec_err)?;
    let dec = dec
        .into_null_fields()
        .map_err(dec_err)?
        .finish()
        .map_err(dec_err)?;

    let mut bid_prices = Vec::new();
    let mut bid_sizes = Vec::new();
    let mut ask_prices = Vec::new();
    let mut ask_sizes = Vec::new();
    let mut g = dec.into_decimal_array_fields().map_err(dec_err)?;
    for e in g.by_ref() {
        let e = e.map_err(dec_err)?;
        let target = match e.field_id() {
            3 => &mut bid_prices,
            4 => &mut bid_sizes,
            5 => &mut ask_prices,
            6 => &mut ask_sizes,
            _ => {
                return Err(PersistError::Decode(
                    "unexpected decimal array field".into(),
                ));
            }
        };
        for v in e.values().map_err(dec_err)? {
            target.push((v.mantissa(), v.exponent()));
        }
    }
    let dec = g.finish().map_err(dec_err)?;
    let (symbols, _) = dec.into_symbol_table().map_err(dec_err)?;

    // Recorder writes no metadata, so the symbol table is exactly the
    // string field bytes in field order.
    let mut symbol = String::new();
    let mut off = 0usize;
    for (fid, len) in string_lens {
        let bytes = symbols
            .get(off..off + len)
            .ok_or_else(|| PersistError::Decode("symbol table too short".into()))?;
        if fid == 2 {
            symbol = String::from_utf8_lossy(bytes).into_owned();
        }
        off += len;
    }

    if bid_prices.len() != bid_sizes.len() || ask_prices.len() != ask_sizes.len() {
        return Err(PersistError::Decode(
            "mismatched level array lengths".into(),
        ));
    }
    let bids = bid_prices.into_iter().zip(bid_sizes).collect();
    let asks = ask_prices.into_iter().zip(ask_sizes).collect();
    Ok(WireBook {
        sequence,
        exchange_ts_ns,
        symbol,
        bids,
        asks,
    })
}
