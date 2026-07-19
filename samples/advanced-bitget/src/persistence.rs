//! `ForegroundPersistor` — the Task 7/10 persistence seam.
//!
//! Owns ordered typed/dynamic matching by correlation sequence and row
//! conversion to exact ClickHouse `Decimal(38,18)` scaled integers. Database
//! writes go through the [`RowSink`] seam: [`InMemorySink`] for tests and a
//! ClickHouse adapter for the live pipeline.

use std::collections::VecDeque;
use std::time::Duration;

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
        // Dispatch by schema + template id: template 3 is the schema
        // announcement, template 4 a row; anything else is rejected.
        use ergo_clickhouse_persist::sbe::v2::{DynamicRowV2Decoder, DynamicSchemaV2Decoder};
        if bytes.len() < 8 {
            return Err(PersistError::Decode("short dynamic fragment".into()));
        }
        let template = u16::from_le_bytes([bytes[2], bytes[3]]);
        let schema = u16::from_le_bytes([bytes[4], bytes[5]]);
        match (schema, template) {
            (DynamicSchemaV2Decoder::SCHEMA_ID, DynamicSchemaV2Decoder::TEMPLATE_ID) => {
                let dec = DynamicSchemaV2Decoder::wrap_and_apply_header(bytes, 0)
                    .map_err(|e| PersistError::Decode(e.to_string()))?;
                let _ = dec.schema_id();
                self.counters.schemas_seen += 1;
                Ok(())
            }
            (DynamicRowV2Decoder::SCHEMA_ID, DynamicRowV2Decoder::TEMPLATE_ID) => {
                let wire = decode_dynamic_book(bytes)?;
                self.dynamic_queue.push_back(wire);
                self.bound_queues();
                self.try_match()
            }
            (s, t) => Err(PersistError::Decode(format!(
                "unknown schema/template combination {s}/{t} on dynamic stream"
            ))),
        }
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
            (e.price_wire().mantissa(), e.price_wire().exponent()),
            (e.size_wire().mantissa(), e.size_wire().exponent()),
        ));
    }
    let after = g.finish()?;
    let mut asks = Vec::new();
    let mut g = after.into_asks()?;
    for e in g.by_ref() {
        asks.push((
            (e.price_wire().mantissa(), e.price_wire().exponent()),
            (e.size_wire().mantissa(), e.size_wire().exponent()),
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

// ── ClickHouse adapter ────────────────────────────────────────────────

/// Render a `Decimal(38,18)` scaled integer as an exact decimal literal.
#[must_use]
pub fn dec38_18(v: i128) -> String {
    const SCALE: u128 = 1_000_000_000_000_000_000;
    let sign = if v < 0 { "-" } else { "" };
    let a = v.unsigned_abs();
    let mut s = format!("{sign}{}.{:018}", a / SCALE, a % SCALE);
    // Trim trailing zeros (keep at least one fractional digit trimmed to none).
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

fn dec_array(vals: &[i128]) -> String {
    let inner: Vec<String> = vals.iter().map(|v| format!("'{}'", dec38_18(*v))).collect();
    format!("[{}]", inner.join(","))
}

/// ClickHouse credentials from `CLICKHOUSE_USER`/`CLICKHOUSE_PASSWORD`,
/// defaulting to the local sample container (`default`/`ergo-sbe`).
#[must_use]
pub fn clickhouse_credentials() -> (String, String) {
    (
        std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".into()),
        std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| "ergosbe".into()),
    )
}

/// Live [`RowSink`] over the ClickHouse HTTP interface. Batches rows and
/// inserts on size threshold or [`flush`](RowSink::flush); every response is
/// checked.
pub struct ClickHouseRowSink {
    endpoint: String,
    user: String,
    password: String,
    client: reqwest::blocking::Client,
    typed: Vec<String>,
    dynamic: Vec<String>,
    trades: Vec<String>,
}

/// Rows per table buffered before an automatic insert.
// ponytail: fixed threshold; add a time threshold knob if live tuning needs it.
const BATCH_ROWS: usize = 256;

impl ClickHouseRowSink {
    /// Connect to an already-running ClickHouse, verify it responds, and
    /// create the three tables if absent. Never starts Docker.
    ///
    /// # Errors
    ///
    /// Connection or DDL failures, with the endpoint in the message.
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        let ping = client
            .get(format!("{endpoint}/ping"))
            .send()
            .map_err(|e| format!("ClickHouse unreachable at {endpoint}: {e}"))?;
        if !ping.status().is_success() {
            return Err(format!(
                "ClickHouse ping failed at {endpoint}: {}",
                ping.status()
            ));
        }
        let (user, password) = clickhouse_credentials();
        let sink = Self {
            endpoint: endpoint.to_string(),
            user,
            password,
            client,
            typed: Vec::new(),
            dynamic: Vec::new(),
            trades: Vec::new(),
        };
        for table in ["l2book_typed", "l2book_dynamic"] {
            sink.execute(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (\
                 sequence UInt64, exchange_ts UInt64, symbol String, \
                 bid_prices Array(Decimal(38,18)), bid_sizes Array(Decimal(38,18)), \
                 ask_prices Array(Decimal(38,18)), ask_sizes Array(Decimal(38,18))\
                 ) ENGINE = MergeTree ORDER BY sequence"
            ))?;
        }
        sink.execute(
            "CREATE TABLE IF NOT EXISTS trade (\
             trade_id UInt64, exchange_ts UInt64, symbol String, \
             price Decimal(38,18), size Decimal(38,18), is_buy Bool\
             ) ENGINE = MergeTree ORDER BY trade_id",
        )?;
        Ok(sink)
    }

    fn execute(&self, sql: &str) -> Result<(), String> {
        let resp = self
            .client
            .post(&self.endpoint)
            .header("X-ClickHouse-User", &self.user)
            .header("X-ClickHouse-Key", &self.password)
            .body(sql.to_string())
            .send()
            .map_err(|e| format!("ClickHouse request failed: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().unwrap_or_default();
            Err(format!("ClickHouse error [{status}]: {body}"))
        }
    }

    fn book_values(row: &L2BookRow) -> String {
        format!(
            "({},{},'{}',{},{},{},{})",
            row.sequence,
            row.exchange_ts_ns,
            row.symbol.replace('\'', ""),
            dec_array(&row.bid_prices),
            dec_array(&row.bid_sizes),
            dec_array(&row.ask_prices),
            dec_array(&row.ask_sizes),
        )
    }

    fn insert_batch(&self, table: &str, rows: &[String]) -> Result<(), String> {
        if rows.is_empty() {
            return Ok(());
        }
        self.execute(&format!("INSERT INTO {table} VALUES {}", rows.join(",")))
    }

    fn flush_table(
        &mut self,
        which: fn(&mut Self) -> &mut Vec<String>,
        table: &str,
    ) -> Result<(), String> {
        let rows = std::mem::take(which(self));
        self.insert_batch(table, &rows)
    }
}

impl RowSink for ClickHouseRowSink {
    fn insert_l2book_typed(&mut self, row: &L2BookRow) -> Result<(), String> {
        self.typed.push(Self::book_values(row));
        if self.typed.len() >= BATCH_ROWS {
            self.flush_table(|s| &mut s.typed, "l2book_typed")?;
        }
        Ok(())
    }

    fn insert_l2book_dynamic(&mut self, row: &L2BookRow) -> Result<(), String> {
        self.dynamic.push(Self::book_values(row));
        if self.dynamic.len() >= BATCH_ROWS {
            self.flush_table(|s| &mut s.dynamic, "l2book_dynamic")?;
        }
        Ok(())
    }

    fn insert_trade(&mut self, row: &TradeRow) -> Result<(), String> {
        self.trades.push(format!(
            "({},{},'{}','{}','{}',{})",
            row.trade_id,
            row.exchange_ts_ns,
            row.symbol.replace('\'', ""),
            dec38_18(row.price),
            dec38_18(row.size),
            row.is_buy,
        ));
        if self.trades.len() >= BATCH_ROWS {
            self.flush_table(|s| &mut s.trades, "trade")?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        self.flush_table(|s| &mut s.typed, "l2book_typed")?;
        self.flush_table(|s| &mut s.dynamic, "l2book_dynamic")?;
        self.flush_table(|s| &mut s.trades, "trade")
    }
}
