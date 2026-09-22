//! PyO3 extension: prepared Python table handles backed by the same
//! single-threaded recorder the native Rust API uses.
//!
//! Python evaluates arguments eagerly — the recommended temporary-table
//! pattern is an explicit `enabled()` guard before constructing optional
//! state (see `apps/market-recorder`).

use ergo_clickhouse_persist::config::{self, PermanentTables};
use ergo_clickhouse_persist::persist::{EncodeError, Persistable, RecordOutcome, RowWriter};
use ergo_clickhouse_persist::protocol::Policy;
use ergo_clickhouse_persist::registration::{RecorderConfig, RecorderSession, TransportConfig};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};
use ergo_market_schema::diagnostics::BookDebugDomain;
use ergo_market_schema::market_data::{
    BookDeltaAsksEntryDomain, BookDeltaBidsEntryDomain, BookDeltaDomain, BookFlags, QuoteDomain,
    Side, TradeDomain, TradeEncoder,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

fn map_err(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn outcome_code(o: RecordOutcome) -> u8 {
    match o {
        RecordOutcome::Published => 1,
        RecordOutcome::Disabled => 2,
        RecordOutcome::Dropped(_) => 3,
        RecordOutcome::Invalid(_) => 4,
    }
}

fn side_from_u8(v: u8) -> Side {
    match v {
        1 => Side::Buy,
        2 => Side::Sell,
        _ => Side::Unknown,
    }
}

fn flags_from_u8(v: u8) -> BookFlags {
    match v {
        1 => BookFlags::Snapshot,
        2 => BookFlags::EndOfBatch,
        4 => BookFlags::Resync,
        _ => BookFlags::None,
    }
}

fn encode_sbe<E: std::fmt::Display>(
    len: usize,
    encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
) -> PyResult<Vec<u8>> {
    let mut buf = vec![0u8; len];
    let n = encode(&mut buf).map_err(map_err)?;
    buf.truncate(n);
    Ok(buf)
}

const PERMANENT: &[&str] = &[
    "trades",
    "quotes",
    "bars",
    "l2_books",
    "raw_exchange_messages",
    "sbe_messages",
    "instruments",
    "order_book_deltas",
    "book_snapshots",
    "funding_rates",
    "mark_prices",
    "index_prices",
    "open_interest",
    "liquidations",
    "market_status",
];

/// Exact-value trade row. Prices/quantities are 1e-8 mantissas, never floats.
#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PyTrade {
    #[pyo3(get, set)]
    pub venue: String,
    #[pyo3(get, set)]
    pub instrument_id: u32,
    #[pyo3(get, set)]
    pub price_raw: i64,
    #[pyo3(get, set)]
    pub quantity_raw: i64,
    #[pyo3(get, set)]
    pub side: u8,
    #[pyo3(get, set)]
    pub event_time_ns: u64,
    #[pyo3(get, set)]
    pub exchange_trade_id: u64,
}

#[pymethods]
impl PyTrade {
    #[new]
    #[pyo3(signature = (venue, instrument_id, price_raw, quantity_raw, side, event_time_ns, exchange_trade_id))]
    fn new(
        venue: String,
        instrument_id: u32,
        price_raw: i64,
        quantity_raw: i64,
        side: u8,
        event_time_ns: u64,
        exchange_trade_id: u64,
    ) -> Self {
        Self {
            venue,
            instrument_id,
            price_raw,
            quantity_raw,
            side,
            event_time_ns,
            exchange_trade_id,
        }
    }
}

const TRADE_NAMES: &[&str] = &[
    "venue",
    "instrument_id",
    "price",
    "quantity",
    "side",
    "exchange_event_time_ns",
    "exchange_trade_id",
];

static TRADE_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn trade_schema() -> &'static RowSchema {
    TRADE_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::decimal(18, 8),
            ValueSchema::decimal(18, 8),
            ValueSchema::scalar(TypeCode::U8),
            ValueSchema::scalar(TypeCode::U64),
            ValueSchema::scalar(TypeCode::U64),
        ])
    })
}

impl Persistable for PyTrade {
    fn schema() -> &'static RowSchema {
        trade_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(trade_schema().nullmap_len() + 4 + self.venue.len() + 4 + 16 + 16 + 1 + 8 + 8)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_str(&self.venue)?;
        out.write_u32(self.instrument_id)?;
        out.write_decimal_i128(i128::from(self.price_raw))?;
        out.write_decimal_i128(i128::from(self.quantity_raw))?;
        out.write_u8(self.side)?;
        out.write_u64(self.event_time_ns)?;
        out.write_u64(self.exchange_trade_id)?;
        Ok(())
    }
}

impl PyTrade {
    fn to_domain(&self) -> TradeDomain {
        TradeDomain {
            instrument_id: self.instrument_id,
            price: Some(self.price_raw),
            quantity: Some(self.quantity_raw),
            side: side_from_u8(self.side),
            event_time_ns: self.event_time_ns,
            exchange_trade_id: self.exchange_trade_id,
        }
    }
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PyQuote {
    #[pyo3(get, set)]
    pub venue: String,
    #[pyo3(get, set)]
    pub instrument_id: u32,
    #[pyo3(get, set)]
    pub bid_price_raw: i64,
    #[pyo3(get, set)]
    pub bid_qty_raw: i64,
    #[pyo3(get, set)]
    pub ask_price_raw: i64,
    #[pyo3(get, set)]
    pub ask_qty_raw: i64,
    #[pyo3(get, set)]
    pub event_time_ns: u64,
}

#[pymethods]
impl PyQuote {
    #[new]
    fn new(
        venue: String,
        instrument_id: u32,
        bid_price_raw: i64,
        bid_qty_raw: i64,
        ask_price_raw: i64,
        ask_qty_raw: i64,
        event_time_ns: u64,
    ) -> Self {
        Self {
            venue,
            instrument_id,
            bid_price_raw,
            bid_qty_raw,
            ask_price_raw,
            ask_qty_raw,
            event_time_ns,
        }
    }
}

static QUOTE_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn quote_schema() -> &'static RowSchema {
    QUOTE_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::decimal(18, 8),
            ValueSchema::decimal(18, 8),
            ValueSchema::decimal(18, 8),
            ValueSchema::decimal(18, 8),
            ValueSchema::scalar(TypeCode::U64),
        ])
    })
}

const QUOTE_NAMES: &[&str] = &[
    "venue",
    "instrument_id",
    "bid_price",
    "bid_quantity",
    "ask_price",
    "ask_quantity",
    "event_time_ns",
];

impl Persistable for PyQuote {
    fn schema() -> &'static RowSchema {
        quote_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(quote_schema().nullmap_len() + 4 + self.venue.len() + 4 + 16 * 4 + 8)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_str(&self.venue)?;
        out.write_u32(self.instrument_id)?;
        out.write_decimal_i128(i128::from(self.bid_price_raw))?;
        out.write_decimal_i128(i128::from(self.bid_qty_raw))?;
        out.write_decimal_i128(i128::from(self.ask_price_raw))?;
        out.write_decimal_i128(i128::from(self.ask_qty_raw))?;
        out.write_u64(self.event_time_ns)?;
        Ok(())
    }
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PyRawFrame {
    #[pyo3(get, set)]
    pub payload: Vec<u8>,
    #[pyo3(get, set)]
    pub capture_mode: u8,
    #[pyo3(get, set)]
    pub connection_id: u32,
    #[pyo3(get, set)]
    pub reconnect_generation: u32,
    #[pyo3(get, set)]
    pub receive_sequence: u64,
    #[pyo3(get, set)]
    pub venue: String,
    #[pyo3(get, set)]
    pub product: u8,
}

static RAW_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn raw_schema() -> &'static RowSchema {
    RAW_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::Bytes),
            ValueSchema::scalar(TypeCode::U8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::U64),
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::U8),
        ])
    })
}

const RAW_NAMES: &[&str] = &[
    "payload",
    "capture_mode",
    "connection_id",
    "reconnect_generation",
    "receive_sequence",
    "venue",
    "product",
];

impl Persistable for PyRawFrame {
    fn schema() -> &'static RowSchema {
        raw_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(raw_schema().nullmap_len()
            + 4
            + self.payload.len()
            + 1
            + 4
            + 4
            + 8
            + 4
            + self.venue.len()
            + 1)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_bytes(&self.payload)?;
        out.write_u8(self.capture_mode)?;
        out.write_u32(self.connection_id)?;
        out.write_u32(self.reconnect_generation)?;
        out.write_u64(self.receive_sequence)?;
        out.write_str(&self.venue)?;
        out.write_u8(self.product)?;
        Ok(())
    }
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PySbeMessage {
    pub payload: Vec<u8>,
    pub schema_id: u16,
    pub template_id: u16,
    pub origin: u8,
}

static SBE_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn sbe_schema() -> &'static RowSchema {
    SBE_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::Bytes),
            ValueSchema::scalar(TypeCode::U16),
            ValueSchema::scalar(TypeCode::U16),
            ValueSchema::scalar(TypeCode::U8),
        ])
    })
}

impl Persistable for PySbeMessage {
    fn schema() -> &'static RowSchema {
        sbe_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(sbe_schema().nullmap_len() + 4 + self.payload.len() + 2 + 2 + 1)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_bytes(&self.payload)?;
        out.write_u16(self.schema_id)?;
        out.write_u16(self.template_id)?;
        out.write_u8(self.origin)?;
        Ok(())
    }
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PyL2Book {
    pub venue: String,
    pub instrument: String,
    pub instrument_id: u32,
    pub product: u8,
    pub generation: u32,
    pub source_sequence: u64,
    pub validity: u8,
    pub event_time_ns: u64,
    pub bids: Vec<(i64, i64)>,
    pub asks: Vec<(i64, i64)>,
}

static L2_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn l2_schema() -> &'static RowSchema {
    L2_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::U8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::U64),
            ValueSchema::scalar(TypeCode::U8),
            ValueSchema::scalar(TypeCode::U64),
            ValueSchema {
                ty: TypeCode::Decimal,
                precision: 18,
                scale: 8,
                nullable: false,
                is_array: true,
            },
            ValueSchema {
                ty: TypeCode::Decimal,
                precision: 18,
                scale: 8,
                nullable: false,
                is_array: true,
            },
            ValueSchema {
                ty: TypeCode::Decimal,
                precision: 18,
                scale: 8,
                nullable: false,
                is_array: true,
            },
            ValueSchema {
                ty: TypeCode::Decimal,
                precision: 18,
                scale: 8,
                nullable: false,
                is_array: true,
            },
        ])
    })
}

const L2_NAMES: &[&str] = &[
    "venue",
    "instrument",
    "instrument_id",
    "product",
    "generation",
    "source_sequence",
    "validity",
    "event_time_ns",
    "bids__price",
    "bids__quantity",
    "asks__price",
    "asks__quantity",
];

fn pack_decimals(levels: &[(i64, i64)], price: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(levels.len() * 16);
    for (p, q) in levels {
        let v = i128::from(if price { *p } else { *q });
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

impl Persistable for PyL2Book {
    fn schema() -> &'static RowSchema {
        l2_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        let n = self.bids.len() + self.asks.len();
        Ok(l2_schema().nullmap_len()
            + 4
            + self.venue.len()
            + 4
            + self.instrument.len()
            + 4
            + 1
            + 4
            + 8
            + 1
            + 8
            + 4 * 4
            + n * 2 * 16)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_str(&self.venue)?;
        out.write_str(&self.instrument)?;
        out.write_u32(self.instrument_id)?;
        out.write_u8(self.product)?;
        out.write_u32(self.generation)?;
        out.write_u64(self.source_sequence)?;
        out.write_u8(self.validity)?;
        out.write_u64(self.event_time_ns)?;
        let bp = pack_decimals(&self.bids, true);
        let bq = pack_decimals(&self.bids, false);
        let ap = pack_decimals(&self.asks, true);
        let aq = pack_decimals(&self.asks, false);
        out.write_array(TypeCode::Decimal, 16, &bp, self.bids.len())?;
        out.write_array(TypeCode::Decimal, 16, &bq, self.bids.len())?;
        out.write_array(TypeCode::Decimal, 16, &ap, self.asks.len())?;
        out.write_array(TypeCode::Decimal, 16, &aq, self.asks.len())?;
        Ok(())
    }
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct PyFunding {
    pub venue: String,
    pub instrument: String,
    pub instrument_id: u32,
    pub rate_raw: i64,
    pub interval_sec: u32,
    pub event_time_ns: u64,
}

static FUNDING_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn funding_schema() -> &'static RowSchema {
    FUNDING_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::Utf8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::decimal(18, 8),
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::U64),
        ])
    })
}

impl Persistable for PyFunding {
    fn schema() -> &'static RowSchema {
        funding_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(funding_schema().nullmap_len()
            + 4
            + self.venue.len()
            + 4
            + self.instrument.len()
            + 4
            + 16
            + 4
            + 8)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_str(&self.venue)?;
        out.write_str(&self.instrument)?;
        out.write_u32(self.instrument_id)?;
        out.write_decimal_i128(i128::from(self.rate_raw))?;
        out.write_u32(self.interval_sec)?;
        out.write_u64(self.event_time_ns)?;
        Ok(())
    }
}

const FUNDING_NAMES: &[&str] = &[
    "venue",
    "instrument",
    "instrument_id",
    "rate",
    "interval_sec",
    "event_time_ns",
];
const SBE_NAMES: &[&str] = &["payload", "schema_id", "template_id", "origin"];
const PRICE_NAMES: &[&str] = &["instrument_id", "price", "event_time_ns"];

#[pyclass(unsendable)]
#[derive(Clone, Copy)]
pub struct PyPricePoint {
    pub instrument_id: u32,
    pub price_raw: i64,
    pub event_time_ns: u64,
}

static PRICE_SCHEMA: std::sync::OnceLock<RowSchema> = std::sync::OnceLock::new();
fn price_schema() -> &'static RowSchema {
    PRICE_SCHEMA.get_or_init(|| {
        RowSchema::from_vec(vec![
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::decimal(18, 8),
            ValueSchema::scalar(TypeCode::U64),
        ])
    })
}

impl Persistable for PyPricePoint {
    fn schema() -> &'static RowSchema {
        price_schema()
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(price_schema().nullmap_len() + 4 + 16 + 8)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_u32(self.instrument_id)?;
        out.write_decimal_i128(i128::from(self.price_raw))?;
        out.write_u64(self.event_time_ns)?;
        Ok(())
    }
}

#[pyclass(unsendable)]
pub struct PyTable {
    inner: PyTableKind,
}

enum PyTableKind {
    Trade(ergo_clickhouse_persist::recorder::PreparedTable<PyTrade>),
    Quote(ergo_clickhouse_persist::recorder::PreparedTable<PyQuote>),
    Raw(ergo_clickhouse_persist::recorder::PreparedTable<PyRawFrame>),
    Sbe(ergo_clickhouse_persist::recorder::PreparedTable<PySbeMessage>),
    L2(ergo_clickhouse_persist::recorder::PreparedTable<PyL2Book>),
    Funding(ergo_clickhouse_persist::recorder::PreparedTable<PyFunding>),
    Price(ergo_clickhouse_persist::recorder::PreparedTable<PyPricePoint>),
    Debug(ergo_clickhouse_persist::recorder::PreparedTable<BookDebugDomain>),
}

impl PyTable {
    fn slot_enabled(&self) -> bool {
        use ergo_clickhouse_persist::recorder::EnableSlot;
        let word = match &self.inner {
            PyTableKind::Trade(t) => t.slot().load(),
            PyTableKind::Quote(t) => t.slot().load(),
            PyTableKind::Raw(t) => t.slot().load(),
            PyTableKind::Sbe(t) => t.slot().load(),
            PyTableKind::L2(t) => t.slot().load(),
            PyTableKind::Funding(t) => t.slot().load(),
            PyTableKind::Price(t) => t.slot().load(),
            PyTableKind::Debug(t) => t.slot().load(),
        };
        EnableSlot::is_enabled_word(word)
    }

    fn set_enabled(&self, on: bool) {
        match &self.inner {
            PyTableKind::Trade(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::Quote(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::Raw(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::Sbe(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::L2(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::Funding(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::Price(t) => toggle(t.slot(), t.policy_id(), on),
            PyTableKind::Debug(t) => toggle(t.slot(), t.policy_id(), on),
        }
    }
}

fn toggle(slot: &ergo_clickhouse_persist::recorder::SharedSlot, policy_id: u16, on: bool) {
    if on {
        slot.set_enabled(policy_id, 1);
    } else {
        slot.set_disabled();
    }
}

#[pymethods]
impl PyTable {
    fn enabled(&self) -> bool {
        self.slot_enabled()
    }

    fn enable(&self) {
        self.set_enabled(true);
    }

    fn disable(&self) {
        self.set_enabled(false);
    }

    fn record_trade(&self, writer: &mut PyWriter, trade: &PyTrade) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::Trade(t) => t,
            _ => return Err(PyValueError::new_err("not a trade table")),
        };
        let outcome = writer.0.borrow_mut().record(t, trade, trade.event_time_ns);
        Ok(outcome_code(outcome))
    }

    fn record_quote(&self, writer: &mut PyWriter, quote: &PyQuote) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::Quote(t) => t,
            _ => return Err(PyValueError::new_err("not a quote table")),
        };
        let outcome = writer.0.borrow_mut().record(t, quote, quote.event_time_ns);
        Ok(outcome_code(outcome))
    }

    #[allow(clippy::too_many_arguments)]
    fn record_frame(
        &self,
        writer: &mut PyWriter,
        payload: &[u8],
        capture_mode: u8,
        connection_id: u32,
        reconnect_generation: u32,
        receive_sequence: u64,
        venue: &str,
        product: u8,
        captured_at_ns: u64,
    ) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::Raw(t) => t,
            _ => return Err(PyValueError::new_err("not a raw table")),
        };
        let row = PyRawFrame {
            payload: payload.to_vec(),
            capture_mode,
            connection_id,
            reconnect_generation,
            receive_sequence,
            venue: venue.to_string(),
            product,
        };
        let outcome = writer.0.borrow_mut().record(t, &row, captured_at_ns);
        Ok(outcome_code(outcome))
    }

    fn record_bytes(
        &self,
        writer: &mut PyWriter,
        payload: &[u8],
        captured_at_ns: u64,
    ) -> PyResult<u8> {
        self.record_frame(
            writer,
            payload,
            1,
            0,
            0,
            captured_at_ns,
            "",
            0,
            captured_at_ns,
        )
    }

    fn record_sbe(
        &self,
        writer: &mut PyWriter,
        payload: &[u8],
        schema_id: u16,
        template_id: u16,
        origin: u8,
        captured_at_ns: u64,
    ) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::Sbe(t) => t,
            _ => return Err(PyValueError::new_err("not an sbe table")),
        };
        let row = PySbeMessage {
            payload: payload.to_vec(),
            schema_id,
            template_id,
            origin,
        };
        let outcome = writer.0.borrow_mut().record(t, &row, captured_at_ns);
        Ok(outcome_code(outcome))
    }

    #[allow(clippy::too_many_arguments)]
    fn record_l2(
        &self,
        writer: &mut PyWriter,
        venue: &str,
        instrument: &str,
        instrument_id: u32,
        product: u8,
        generation: u32,
        source_sequence: u64,
        validity: u8,
        event_time_ns: u64,
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>,
    ) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::L2(t) => t,
            _ => return Err(PyValueError::new_err("not an l2 table")),
        };
        let row = PyL2Book {
            venue: venue.to_string(),
            instrument: instrument.to_string(),
            instrument_id,
            product,
            generation,
            source_sequence,
            validity,
            event_time_ns,
            bids,
            asks,
        };
        let outcome = writer.0.borrow_mut().record(t, &row, event_time_ns);
        Ok(outcome_code(outcome))
    }

    #[allow(clippy::too_many_arguments)]
    fn record_funding(
        &self,
        writer: &mut PyWriter,
        venue: &str,
        instrument: &str,
        instrument_id: u32,
        rate_raw: i64,
        interval_sec: u32,
        event_time_ns: u64,
    ) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::Funding(t) => t,
            _ => return Err(PyValueError::new_err("not a funding table")),
        };
        let row = PyFunding {
            venue: venue.to_string(),
            instrument: instrument.to_string(),
            instrument_id,
            rate_raw,
            interval_sec,
            event_time_ns,
        };
        let outcome = writer.0.borrow_mut().record(t, &row, event_time_ns);
        Ok(outcome_code(outcome))
    }

    fn record_price(
        &self,
        writer: &mut PyWriter,
        instrument_id: u32,
        price_raw: i64,
        event_time_ns: u64,
    ) -> PyResult<u8> {
        let t = match &self.inner {
            PyTableKind::Price(t) => t,
            _ => return Err(PyValueError::new_err("not a price table")),
        };
        let row = PyPricePoint {
            instrument_id,
            price_raw,
            event_time_ns,
        };
        let outcome = writer.0.borrow_mut().record(t, &row, event_time_ns);
        Ok(outcome_code(outcome))
    }

    fn record_debug(
        &self,
        writer: &mut PyWriter,
        instrument_id: u32,
        update_id: u64,
        buffered_updates: u32,
        sync_state: u8,
        captured_at_ns: u64,
    ) -> PyResult<u8> {
        if !self.slot_enabled() {
            return Ok(2);
        }
        let t = match &self.inner {
            PyTableKind::Debug(t) => t,
            _ => return Err(PyValueError::new_err("not a debug table")),
        };
        let row = BookDebugDomain {
            instrument_id,
            update_id,
            buffered_updates,
            sync_state,
            captured_at_ns,
        };
        let outcome = writer.0.borrow_mut().record(t, &row, captured_at_ns);
        Ok(outcome_code(outcome))
    }
}

#[pyclass(unsendable)]
pub struct PyWriter(pub Rc<RefCell<ergo_clickhouse_persist::recorder::Writer>>);

#[pymethods]
impl PyWriter {
    fn published(&self) -> u64 {
        self.0.borrow().counters().published
    }

    /// Next sequence this writer will use; the session-end boundary reports
    /// the last one consumed.
    fn sequence(&self) -> u64 {
        self.0.borrow().sequence()
    }
    fn dropped(&self) -> u64 {
        self.0.borrow().counters().dropped
    }
    fn invalid(&self) -> u64 {
        self.0.borrow().counters().invalid
    }

    fn drain(&mut self) -> Vec<Vec<u8>> {
        self.0.borrow_mut().drain_permanent()
    }
}

#[pyclass(unsendable)]
pub struct PyRecorderSession {
    inner: RecorderSession,
    process: String,
    instance: String,
    /// The Aeron publication, when the session publishes over Aeron. Kept so
    /// the caller can wait for the Archive's recording subscription to present
    /// an image before declaring: an offer to a publication with no subscriber
    /// is rejected, and the first thing a session sends is its declaration.
    publication: Option<ergo_clickhouse_persist::recorder::SharedPublication>,
}

#[pymethods]
impl PyRecorderSession {
    /// Connect a recorder session.
    ///
    /// `aeron` selects the transport: `None` keeps the in-memory ring (the
    /// lab and the unit tests), while `Some((channel, stream_id))` publishes
    /// over Aeron — which is what the recorder pod does, so its records reach
    /// the pod's Archive instead of a ring nothing drains.
    #[staticmethod]
    #[pyo3(signature = (process, instance, aeron=None))]
    fn connect(process: String, instance: String, aeron: Option<(String, i32)>) -> PyResult<Self> {
        // Sized from the protocol's own limit rather than a guess:
        // `MAX_RECORD_BYTES` is the largest record any slot must hold, and the
        // ring overwrites its oldest slot (counting the drop), so the slot
        // count is a retention choice, not a capacity requirement. The
        // previous `4096 x 1 MiB` was a hard 4 GiB allocation at session
        // construction — enough to OOMKill the recorder container.
        let memory = || TransportConfig::Memory {
            slots: 256,
            slot_bytes: ergo_clickhouse_persist::protocol::limits::MAX_RECORD_BYTES,
        };
        let (config, inner) = match &aeron {
            Some((channel, stream_id)) => {
                // ONE publication shared by the registration sink and the data
                // transport: the ingester resolves layouts from declarations on
                // the stream, so a declaration and the rows that depend on it
                // must arrive in that order, which two publications could not
                // guarantee.
                let publication = std::rc::Rc::new(std::cell::RefCell::new(
                    ergo_clickhouse_persist::recorder::AeronPublication::connect(
                        channel, *stream_id,
                    )
                    .map_err(map_err)?,
                ));
                let sink = Box::new(ergo_clickhouse_persist::registration::AeronSink::new(
                    std::rc::Rc::clone(&publication),
                ));
                let config = RecorderConfig {
                    process: process.clone(),
                    instance: instance.clone(),
                    build: env!("CARGO_PKG_VERSION").into(),
                    diagnostics_quota_bytes_per_sec: 1024 * 1024,
                    permanent: TransportConfig::AeronShared(publication),
                    // Diagnostics get their own stream so quota exhaustion on a
                    // temporary table cannot delay permanent recording.
                    diagnostics: TransportConfig::Aeron {
                        channel: channel.clone(),
                        stream_id: stream_id + 1,
                    },
                    ..RecorderConfig::default()
                };
                let session =
                    RecorderSession::connect_with(config.clone(), sink).map_err(map_err)?;
                (config, session)
            }
            None => {
                let config = RecorderConfig {
                    process: process.clone(),
                    instance: instance.clone(),
                    build: env!("CARGO_PKG_VERSION").into(),
                    diagnostics_quota_bytes_per_sec: 1024 * 1024,
                    permanent: memory(),
                    diagnostics: memory(),
                    ..RecorderConfig::default()
                };
                let session = RecorderSession::connect(config.clone()).map_err(map_err)?;
                (config, session)
            }
        };
        let _ = config;
        Ok(Self {
            inner,
            process,
            instance,
            publication: match &aeron {
                Some(_) => Some(std::rc::Rc::clone(match &config.permanent {
                    TransportConfig::AeronShared(p) => p,
                    _ => unreachable!("aeron transport selects AeronShared"),
                })),
                None => None,
            },
        })
    }

    /// Wait until the publication has a subscriber (the Archive's recording
    /// image). Returns immediately for the in-memory transport.
    fn wait_connected(&self, timeout_secs: f64) -> bool {
        let Some(publication) = &self.publication else {
            return true;
        };
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs_f64(timeout_secs.max(0.0));
        while std::time::Instant::now() < deadline {
            if publication.borrow().is_connected() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        publication.borrow().is_connected()
    }

    /// Start an Archive recording of `channel`/`stream_id` on the pod-local
    /// Archive, so the publication this session creates has a subscriber.
    ///
    /// Only the pod's own process can do this: the Archive records the
    /// pod-local Aeron driver over IPC, which is shared memory and therefore
    /// unreachable from another pod. Idempotent — an existing recording for
    /// the stream is left alone.
    #[staticmethod]
    #[pyo3(signature = (channel, stream_id, aeron_dir, control_port=4001))]
    fn ensure_recording(
        channel: String,
        stream_id: i32,
        aeron_dir: String,
        control_port: u16,
    ) -> PyResult<Option<i64>> {
        use ergo_clickhouse_persist::ingest::archive::{ArchiveClient, ArchiveEndpoints};
        let endpoints = ArchiveEndpoints {
            control: format!("aeron:udp?endpoint=localhost:{control_port}"),
            control_response: format!("aeron:udp?endpoint=localhost:{}", control_port + 1),
            recording_events: format!("aeron:udp?endpoint=localhost:{}", control_port + 2),
            replay: "aeron:udp?endpoint=localhost:0".into(),
            recorded_channel: channel.clone(),
            recorded_stream_id: stream_id,
        };
        let client = ArchiveClient::connect(&endpoints, std::path::Path::new(&aeron_dir))
            .map_err(map_err)?;
        // The Archive's catalog lives on a PVC and survives the pod, so a
        // recording for this stream often already exists. Only an *active* one
        // is reusable: a stopped recording has no subscription, and returning
        // early on it leaves the publication with no subscriber — which is
        // exactly the state the caller's connection wait then times out on.
        if client.current().is_some_and(|r| r.is_active()) {
            return Ok(None); // already recording
        }
        let subscription_id = client
            .archive
            .start_recording(
                &rusteron_archive::cformat!("{channel}"),
                stream_id,
                rusteron_archive::SOURCE_LOCATION_LOCAL,
                true,
            )
            .map_err(map_err)?;
        Ok(Some(subscription_id))
    }

    /// Stop the recording started by [`Self::ensure_recording`].
    ///
    /// A bounded replay needs a stop position, and the replay source refuses an
    /// active recording, so a producer that has finished its burst must close
    /// the recording before an ingester can replay it.
    #[staticmethod]
    #[pyo3(signature = (subscription_id, aeron_dir, control_port=4001))]
    fn stop_recording(subscription_id: i64, aeron_dir: String, control_port: u16) -> PyResult<()> {
        use ergo_clickhouse_persist::ingest::archive::{ArchiveClient, ArchiveEndpoints};
        let endpoints = ArchiveEndpoints {
            control: format!("aeron:udp?endpoint=localhost:{control_port}"),
            control_response: format!("aeron:udp?endpoint=localhost:{}", control_port + 1),
            recording_events: format!("aeron:udp?endpoint=localhost:{}", control_port + 2),
            replay: "aeron:udp?endpoint=localhost:0".into(),
            recorded_channel: String::new(),
            recorded_stream_id: 0,
        };
        let client = ArchiveClient::connect(&endpoints, std::path::Path::new(&aeron_dir))
            .map_err(map_err)?;
        client
            .archive
            .stop_recording_subscription(subscription_id)
            .map(|_| ())
            .map_err(map_err)
    }

    fn metadata(&mut self, key: &str, value: &str) -> PyResult<()> {
        self.inner.metadata(key, value).map_err(map_err)
    }

    fn declare_session_start(&mut self) -> PyResult<()> {
        self.inner.declare_session_start().map_err(map_err)
    }

    /// Emit the session-end boundary on the wire. The recorder calls this on
    /// shutdown with the writer's final sequence; without it the declaration
    /// existed in Rust but was never produced by any producer.
    fn declare_session_end(&mut self, final_sequence: u64) -> PyResult<()> {
        self.inner
            .declare_session_end(final_sequence)
            .map_err(map_err)
    }

    fn intern_symbol(&mut self, value: &str) -> PyResult<u32> {
        Ok(self.inner.intern_symbol(value).map_err(map_err)?.0)
    }

    fn writer(&mut self) -> PyResult<PyWriter> {
        let w = self.inner.writer().map_err(map_err)?;
        Ok(PyWriter(Rc::new(RefCell::new(w))))
    }

    fn trade_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Trade(prepare(&mut self.inner, name, temporary, TRADE_NAMES)?),
        })
    }
    fn quote_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Quote(prepare(&mut self.inner, name, temporary, QUOTE_NAMES)?),
        })
    }
    fn raw_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Raw(prepare(&mut self.inner, name, temporary, RAW_NAMES)?),
        })
    }
    fn sbe_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Sbe(prepare(&mut self.inner, name, temporary, SBE_NAMES)?),
        })
    }
    fn l2_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::L2(prepare(&mut self.inner, name, temporary, L2_NAMES)?),
        })
    }
    fn funding_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Funding(prepare(&mut self.inner, name, temporary, FUNDING_NAMES)?),
        })
    }
    fn price_table(&mut self, name: &str, temporary: bool) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Price(prepare(&mut self.inner, name, temporary, PRICE_NAMES)?),
        })
    }
    fn debug_table(&mut self, name: &str) -> PyResult<PyTable> {
        Ok(PyTable {
            inner: PyTableKind::Debug(prepare(
                &mut self.inner,
                name,
                true,
                &[
                    "instrument_id",
                    "update_id",
                    "buffered_updates",
                    "sync_state",
                    "captured_at_ns",
                ],
            )?),
        })
    }

    fn encode_trade_sbe(&self, trade: &PyTrade) -> PyResult<Vec<u8>> {
        let d = trade.to_domain();
        let mut buf = [0u8; TradeEncoder::ENCODED_LENGTH];
        let n = d.encode(&mut buf).map_err(map_err)?;
        Ok(buf[..n].to_vec())
    }

    fn encode_quote_sbe(&self, quote: &PyQuote) -> PyResult<Vec<u8>> {
        let d = QuoteDomain {
            instrument_id: quote.instrument_id,
            bid_price: Some(quote.bid_price_raw),
            bid_quantity: Some(quote.bid_qty_raw),
            ask_price: Some(quote.ask_price_raw),
            ask_quantity: Some(quote.ask_qty_raw),
            event_time_ns: quote.event_time_ns,
        };
        let len = d.encoded_length_with_header().map_err(map_err)?;
        encode_sbe(len, |buf| d.encode(buf))
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_book_delta_sbe(
        &self,
        instrument_id: u32,
        generation: u32,
        source_sequence: u64,
        flags: u8,
        event_time_ns: u64,
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>,
    ) -> PyResult<Vec<u8>> {
        let d = BookDeltaDomain {
            instrument_id,
            generation,
            source_sequence: if source_sequence == 0 {
                None
            } else {
                Some(source_sequence)
            },
            flags: flags_from_u8(flags),
            event_time_ns,
            bids: bids
                .into_iter()
                .map(|(price, quantity)| BookDeltaBidsEntryDomain { price, quantity })
                .collect(),
            asks: asks
                .into_iter()
                .map(|(price, quantity)| BookDeltaAsksEntryDomain { price, quantity })
                .collect(),
        };
        let len = d.encoded_length_with_header().map_err(map_err)?;
        encode_sbe(len, |buf| d.encode(buf))
    }

    fn apply_config(&mut self, yaml: &[u8], table_name: &str, table: &PyTable) -> PyResult<bool> {
        let cfg = config::validate(yaml, PERMANENT as PermanentTables).map_err(map_err)?;
        let effective = cfg.resolve(&self.process, &self.instance, table_name);
        table.set_enabled(effective.enabled);
        Ok(effective.enabled)
    }

    /// How many policy declarations this session has made.
    ///
    /// The observable for PLAN §5's policy versioning: a retention change must
    /// **add** a policy (a new id carrying the new values, leaving the old one
    /// describing what already-written rows used), and an unchanged config
    /// must add nothing. Without this the versioning behaviour was
    /// unobservable from the Python side, so nothing could have caught it
    /// regressing.
    fn declared_policy_count(&self) -> PyResult<usize> {
        Ok(self.inner.catalog().policies().len())
    }

    /// Publish a table's configured retention to the session, before the table
    /// is declared.
    ///
    /// PLAN §5: the policy a table declares has to carry what the operator
    /// configured, because that declaration is what the ingester persists and
    /// freezes into its layout binding. Applying only the `enabled` flag — all
    /// `apply_config` did — left every temporary table declaring zeros, so its
    /// rows expired at their own capture time and the configured 24h/7d
    /// retention was never used by anything.
    /// Returns `true` when the resolved retention differs from what was last
    /// recorded for this table — the signal that the change has to be
    /// versioned (a new policy id bound to a new layout), because the ingester
    /// freezes a layout→storage binding on first sight and never recomputes it.
    fn apply_retention(&mut self, yaml: &[u8], table_name: &str) -> PyResult<bool> {
        let cfg = config::validate(yaml, PERMANENT as PermanentTables).map_err(map_err)?;
        let effective = cfg.resolve(&self.process, &self.instance, table_name);
        Ok(self.inner.set_retention(
            table_name,
            u64::try_from(effective.row_ttl.as_nanos()).unwrap_or(u64::MAX),
            u64::try_from(effective.idle_table_ttl.as_nanos()).unwrap_or(u64::MAX),
        ))
    }

    fn export(&mut self, writer: &mut PyWriter, path: &str) -> PyResult<u32> {
        use ergo_clickhouse_persist::protocol::Declaration;
        use std::io::Write;
        let mut file = std::fs::File::create(path).map_err(map_err)?;
        let mut nframes = 0u32;
        let mut write_frame = |bytes: &[u8]| -> PyResult<()> {
            let len = u32::try_from(bytes.len()).map_err(map_err)?;
            file.write_all(&len.to_le_bytes()).map_err(map_err)?;
            file.write_all(bytes).map_err(map_err)?;
            nframes += 1;
            Ok(())
        };
        let mut buf = vec![0u8; 1024 * 1024];
        for policy in self.inner.catalog().policies() {
            let decl = Declaration::Policy(*policy);
            let n = decl.encode(&mut buf).map_err(map_err)?;
            write_frame(&buf[..n])?;
        }
        for layout in self.inner.catalog().layouts() {
            let decl = Declaration::Layout(layout.clone());
            let n = decl.encode(&mut buf).map_err(map_err)?;
            write_frame(&buf[..n])?;
        }
        for frame in writer.drain() {
            write_frame(&frame)?;
        }
        Ok(nframes)
    }
}

fn prepare<T: Persistable>(
    session: &mut RecorderSession,
    name: &str,
    temporary: bool,
    names: &[&str],
) -> PyResult<ergo_clickhouse_persist::recorder::PreparedTable<T>> {
    let policy = if temporary {
        Policy::Temporary
    } else {
        Policy::Permanent
    };
    let t = session
        .table_named::<T>(name, policy, names)
        .map_err(map_err)?;
    if !temporary {
        t.slot().set_enabled(t.policy_id(), 1);
    }
    Ok(t)
}

#[pymodule]
fn ergo_recorder(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRecorderSession>()?;
    m.add_class::<PyTable>()?;
    m.add_class::<PyWriter>()?;
    m.add_class::<PyTrade>()?;
    m.add_class::<PyQuote>()?;
    Ok(())
}
