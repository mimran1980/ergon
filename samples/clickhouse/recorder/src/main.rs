//! Records one exchange's public market data into ClickHouse.
//!
//! `EXCHANGE` picks the venue (see [`VENUES`]); each runs as its own pod.
//! NautilusTrader connects to it (public streams, no API keys) and calls this
//! actor with everything the venue publishes for two instruments. All three
//! ways of recording appear here:
//!
//! * **SBE, static table** (`trade`, `quote`): a message in
//!   `schema/market.xml`, encoded straight into the Aeron term buffer by
//!   `persist_client::record`. The table is created once and never altered.
//! * **SBE, dynamic table** (`book_deltas`, `book_snapshot`, `bar`,
//!   `mark_price`, `index_price`, `funding_rate`): the same, but the table
//!   gains a column when the message gains a field.
//! * **Events** (`spread`, `ticker`, `instrument`, `instrument_status`, and
//!   each kind of venue-specific data, in a table named after its type): a
//!   `tracing::info!(table = …)` or `persist_client::record_row`, whose
//!   fields are the columns.
//! * **Any Rust value** (`book_view`): `persist_client::record_value` of a
//!   `Serialize` struct, nested as deep as it goes: its nested struct,
//!   slices of structs (arrays), `Option` and enum become columns.
//!
//! Static or dynamic is `kind` in `config/tables.yaml`, not code. The
//! handle is installed once in `main`; the free functions do nothing where
//! none is installed, as in this file's tests.
//!
//! The ingester (`persist-server`) takes it from the Aeron Archive into
//! ClickHouse. Which tables are recorded is decided by `config/tables.yaml`,
//! re-read while this runs.

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/market.rs"]
mod market;

use std::collections::HashMap;
use std::num::NonZeroUsize;

use arrayvec::ArrayVec;

use market::{
    BarEncoder, BarFixedFields, BookAction, BookDeltasDeltasEntry, BookDeltasEncoder,
    BookDeltasFixedFields, BookSnapshotAsksEntry, BookSnapshotBidsEntry, BookSnapshotEncoder,
    BookSnapshotFixedFields, Decimal9, FundingRateEncoder, FundingRateFixedFields,
    IndexPriceEncoder, IndexPriceFixedFields, MarkPriceEncoder, MarkPriceFixedFields, QuoteEncoder,
    QuoteFixedFields, Side, TradeEncoder, TradeFixedFields, TryToSbe,
};
use nautilus_binance::config::{BinanceDataClientConfig, BinanceSpotMarketDataMode};
use nautilus_binance::factories::BinanceDataClientFactory;
use nautilus_bybit::common::enums::BybitProductType;
use nautilus_bybit::config::BybitDataClientConfig;
use nautilus_bybit::factories::BybitDataClientFactory;
use nautilus_common::actor::{DataActor, DataActorConfig, DataActorCore};
use nautilus_common::enums::Environment;
use nautilus_common::nautilus_actor;
use nautilus_core::Params;
use nautilus_deribit::config::DeribitDataClientConfig;
use nautilus_deribit::data_types::DeribitVolatilityIndex;
use nautilus_deribit::factories::DeribitDataClientFactory;
use nautilus_hyperliquid::config::HyperliquidDataClientConfig;
use nautilus_hyperliquid::data_types::HyperliquidOpenInterest;
use nautilus_hyperliquid::factories::HyperliquidDataClientFactory;
use nautilus_live::node::LiveNode;
use nautilus_model::data::{
    Bar, BarType, CustomData, DataType, FundingRateUpdate, IndexPriceUpdate, InstrumentStatus,
    MarkPriceUpdate, OrderBookDeltas, QuoteTick, TradeTick,
};
use nautilus_model::enums::{AggressorSide, BookType, OrderSide};
use nautilus_model::identifiers::{ClientId, InstrumentId, TraderId};
use nautilus_model::instruments::{Instrument, InstrumentAny};
use nautilus_model::orderbook::{BookLevel, OrderBook};
use nautilus_okx::OKXInstrumentType;
use nautilus_okx::config::OKXDataClientConfig;
use nautilus_okx::factories::OKXDataClientFactory;
use persist_client::event::Value;
use persist_client::{Persist, Settings};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use tracing_subscriber::layer::SubscriberExt;

const SCHEMA: &str = include_str!("../../schema/market.xml");
/// Levels per side in each `book_snapshot` row.
const BOOK_LEVELS: usize = 10;
/// Book changes per `book_deltas` row: a whole snapshot would not fit one.
const DELTAS_PER_ROW: usize = 1000;

/// An exchange this recorder can follow.
#[derive(Debug)]
struct Venue {
    /// `EXCHANGE`, and the Nautilus client id upper-cased.
    name: &'static str,
    instruments: [&'static str; 2],
    /// A book depth the venue streams.
    depth: usize,
    /// Perpetual swaps: mark price, index price and funding rate.
    perpetual: bool,
    /// Data only this venue offers: `(type, metadata key, metadata value)`.
    custom: &'static [(&'static str, &'static str, &'static str)],
}

const VENUES: [Venue; 5] = [
    Venue {
        name: "binance",
        instruments: ["BTCUSDT.BINANCE", "ETHUSDT.BINANCE"],
        depth: 20, // spot streams 5/10/20
        perpetual: false,
        custom: &[],
    },
    Venue {
        name: "bybit",
        instruments: ["BTCUSDT-LINEAR.BYBIT", "ETHUSDT-LINEAR.BYBIT"],
        depth: 50, // 1/50/200/1000
        perpetual: true,
        custom: &[],
    },
    Venue {
        name: "okx",
        instruments: ["BTC-USDT-SWAP.OKX", "ETH-USDT-SWAP.OKX"],
        depth: 50, // 50 or 400
        perpetual: true,
        custom: &[],
    },
    Venue {
        name: "deribit",
        instruments: ["BTC-PERPETUAL.DERIBIT", "ETH-PERPETUAL.DERIBIT"],
        depth: 20, // 1/10/20
        perpetual: true,
        custom: &[
            ("DeribitVolatilityIndex", "index_name", "btc_usd"),
            ("DeribitVolatilityIndex", "index_name", "eth_usd"),
        ],
    },
    Venue {
        name: "hyperliquid",
        instruments: ["BTC-USD-PERP.HYPERLIQUID", "ETH-USD-PERP.HYPERLIQUID"],
        depth: 20,
        perpetual: true,
        custom: &[
            (
                "HyperliquidOpenInterest",
                "instrument_id",
                "BTC-USD-PERP.HYPERLIQUID",
            ),
            (
                "HyperliquidOpenInterest",
                "instrument_id",
                "ETH-USD-PERP.HYPERLIQUID",
            ),
            // Every trade with the buyer's and seller's addresses.
            (
                "HyperliquidPublicTrade",
                "instrument_id",
                "BTC-USD-PERP.HYPERLIQUID",
            ),
            (
                "HyperliquidPublicTrade",
                "instrument_id",
                "ETH-USD-PERP.HYPERLIQUID",
            ),
        ],
    },
];

/// What one `ticker` row reports for an instrument. The venue decides which
/// of these it has, and so which columns its rows bring.
#[derive(Debug, Default)]
struct Ticker {
    last: Option<f64>,
    trades: u64,
    volume: f64,
    mark: Option<f64>,
    index: Option<f64>,
    funding: Option<f64>,
    open_interest: Option<f64>,
}

#[derive(Debug)]
struct Recorder {
    core: DataActorCore,
    venue: &'static Venue,
    tickers: HashMap<InstrumentId, Ticker>,
    /// Deribit's volatility index (DVOL) by index name, e.g. `btc_usd`.
    volatility: HashMap<String, f64>,
    /// One `book_deltas` row's changes, reused.
    deltas: Vec<BookDeltasDeltasEntry>,
}

nautilus_actor!(Recorder);

impl DataActor for Recorder {
    fn on_start(&mut self) -> anyhow::Result<()> {
        let second = NonZeroUsize::try_from(1000)?;
        let depth = NonZeroUsize::new(self.venue.depth);
        for id in self.venue.instruments.map(InstrumentId::from) {
            // The definition as loaded now; `on_instrument` records changes.
            let loaded = self.cache().instrument(&id);
            if let Some(instrument) = loaded {
                self.on_instrument(&instrument)?;
            }
            self.subscribe_instrument(id, None, None);
            self.subscribe_instrument_status(id, None, None);
            self.subscribe_trades(id, None, None);
            self.subscribe_quotes(id, None, None);
            self.subscribe_book_deltas(id, BookType::L2_MBP, depth, None, false, None);
            // Also drives the `ticker` row, once a second.
            self.subscribe_book_at_interval(id, BookType::L2_MBP, depth, second, None, None);
            self.subscribe_bars(
                BarType::from(format!("{id}-1-MINUTE-LAST-EXTERNAL")),
                None,
                None,
            );
            if self.venue.perpetual {
                self.subscribe_mark_prices(id, None, None);
                self.subscribe_index_prices(id, None, None);
                self.subscribe_funding_rates(id, None, None);
            }
        }
        let client = ClientId::from(self.venue.name.to_uppercase().as_str());
        for &(type_name, key, value) in self.venue.custom {
            let mut metadata = Params::new();
            metadata.insert(key.to_string(), value.into());
            self.subscribe_data(
                DataType::new(type_name, Some(metadata), None),
                Some(client),
                None,
            );
        }
        Ok(())
    }

    fn on_instrument(&mut self, i: &InstrumentAny) -> anyhow::Result<()> {
        // Decimals as text, so a tick size like 0.00001 is kept exactly.
        tracing::info!(
            table = "instrument",
            instrument = %i.id(),
            venue = %i.id().venue,
            raw_symbol = %i.raw_symbol(),
            class = ?i.instrument_class(),
            base_currency = i.base_currency().map(|c| c.code.to_string()),
            quote_currency = %i.quote_currency().code,
            settlement_currency = %i.settlement_currency().code,
            inverse = i.is_inverse(),
            price_increment = %i.price_increment(),
            size_increment = %i.size_increment(),
            multiplier = %i.multiplier(),
            min_quantity = i.min_quantity().map(|q| q.to_string()),
            max_quantity = i.max_quantity().map(|q| q.to_string()),
            maker_fee = %i.maker_fee(),
            taker_fee = %i.taker_fee(),
        );
        Ok(())
    }

    fn on_instrument_status(&mut self, s: &InstrumentStatus) -> anyhow::Result<()> {
        tracing::info!(
            table = "instrument_status",
            instrument = %s.instrument_id,
            venue = %s.instrument_id.venue,
            ts_event = s.ts_event.as_u64(),
            action = ?s.action,
            reason = s.reason.map(|r| r.as_str()),
            trading_event = s.trading_event.map(|e| e.as_str()),
            is_trading = s.is_trading,
            is_quoting = s.is_quoting,
            is_short_sell_restricted = s.is_short_sell_restricted,
        );
        Ok(())
    }

    fn on_trade(&mut self, t: &TradeTick) -> anyhow::Result<()> {
        let ticker = self.tickers.entry(t.instrument_id).or_default();
        ticker.last = Some(t.price.as_f64());
        ticker.trades += 1;
        ticker.volume += t.size.as_f64();
        let (symbol, venue) = names(&t.instrument_id);
        let trade_id = t.trade_id.as_str().as_bytes();
        let len =
            TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
        persist_client::record(TradeEncoder::TEMPLATE_ID, len, |buf| {
            Ok(TradeEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&TradeFixedFields {
                    ts_event: t.ts_event.as_u64(),
                    ts_init: t.ts_init.as_u64(),
                    price: d9(t.price.as_decimal())?,
                    size: d9(t.size.as_decimal())?,
                    aggressor: match t.aggressor_side {
                        AggressorSide::Buy => Side::Buy,
                        AggressorSide::Sell => Side::Sell,
                        AggressorSide::NoAggressor => Side::NoSide,
                    },
                })
                .symbol(symbol)?
                .venue(venue)?
                .trade_id(trade_id)?
                .encoded_length_with_header())
        })
    }

    fn on_quote(&mut self, q: &QuoteTick) -> anyhow::Result<()> {
        let (symbol, venue) = names(&q.instrument_id);
        let len = QuoteEncoder::compute_length_with_header(symbol.len(), venue.len());
        persist_client::record(QuoteEncoder::TEMPLATE_ID, len, |buf| -> anyhow::Result<_> {
            Ok(QuoteEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&QuoteFixedFields {
                    ts_event: q.ts_event.as_u64(),
                    ts_init: q.ts_init.as_u64(),
                    bid_price: d9(q.bid_price.as_decimal())?,
                    ask_price: d9(q.ask_price.as_decimal())?,
                    bid_size: d9(q.bid_size.as_decimal())?,
                    ask_size: d9(q.ask_size.as_decimal())?,
                })
                .symbol(symbol)?
                .venue(venue)?
                .encoded_length_with_header())
        })?;
        // A derived signal needs no schema: one event, and `spread` in
        // tables.yaml, make a table whose columns are these fields.
        let (bid, ask) = (q.bid_price.as_f64(), q.ask_price.as_f64());
        tracing::info!(table = "spread", instrument = %q.instrument_id, bps = (ask - bid) / (ask + bid) * 2e4);
        Ok(())
    }

    fn on_book(&mut self, book: &OrderBook) -> anyhow::Result<()> {
        self.ticker(book);
        book_view(book);
        // Reading a snapshot walks the book: skip all of it when the table is off.
        if !persist_client::enabled(BookSnapshotEncoder::TEMPLATE_ID) {
            return Ok(());
        }
        let now = self.core.timestamp_ns().as_u64();
        let (symbol, venue) = names(&book.instrument_id);
        let (bids, asks) = (levels(book.bids(None))?, levels(book.asks(None))?);
        let len = BookSnapshotEncoder::compute_length_with_header(
            bids.len(),
            asks.len(),
            symbol.len(),
            venue.len(),
        );
        persist_client::record(BookSnapshotEncoder::TEMPLATE_ID, len, |buf| {
            Ok(BookSnapshotEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&BookSnapshotFixedFields {
                    ts_event: book.ts_last.as_u64(),
                    ts_init: now,
                    sequence: book.sequence,
                })
                .bids(bids.len() as u16, |g| {
                    for &(price, size) in &bids {
                        g.add_struct(&BookSnapshotBidsEntry { price, size })?;
                    }
                    Ok(())
                })?
                .asks(asks.len() as u16, |g| {
                    for &(price, size) in &asks {
                        g.add_struct(&BookSnapshotAsksEntry { price, size })?;
                    }
                    Ok(())
                })?
                .symbol(symbol)?
                .venue(venue)?
                .encoded_length_with_header())
        })
    }

    fn on_book_deltas(&mut self, d: &OrderBookDeltas) -> anyhow::Result<()> {
        if !persist_client::enabled(BookDeltasEncoder::TEMPLATE_ID) {
            return Ok(());
        }
        let (symbol, venue) = names(&d.instrument_id);
        for chunk in d.deltas.chunks(DELTAS_PER_ROW) {
            self.deltas.clear();
            for delta in chunk {
                self.deltas.push(BookDeltasDeltasEntry {
                    action: match delta.action {
                        nautilus_model::enums::BookAction::Add => BookAction::Add,
                        nautilus_model::enums::BookAction::Update => BookAction::Update,
                        nautilus_model::enums::BookAction::Delete => BookAction::Delete,
                        nautilus_model::enums::BookAction::Clear => BookAction::Clear,
                    },
                    side: match delta.order.side {
                        Some(OrderSide::Buy) => Side::Buy,
                        Some(OrderSide::Sell) => Side::Sell,
                        _ => Side::NoSide,
                    },
                    price: d9(delta.order.price.as_decimal())?,
                    size: d9(delta.order.size.as_decimal())?,
                });
            }
            let deltas = &self.deltas;
            let len = BookDeltasEncoder::compute_length_with_header(
                deltas.len(),
                symbol.len(),
                venue.len(),
            );
            persist_client::record(
                BookDeltasEncoder::TEMPLATE_ID,
                len,
                |buf| -> anyhow::Result<_> {
                    Ok(BookDeltasEncoder::wrap_and_apply_header(buf, 0)
                        .fixed(&BookDeltasFixedFields {
                            ts_event: d.ts_event.as_u64(),
                            ts_init: d.ts_init.as_u64(),
                            sequence: d.sequence,
                        })
                        .deltas(deltas.len() as u16, |g| {
                            for entry in deltas {
                                g.add_struct(entry)?;
                            }
                            Ok(())
                        })?
                        .symbol(symbol)?
                        .venue(venue)?
                        .encoded_length_with_header())
                },
            )?;
        }
        Ok(())
    }

    fn on_bar(&mut self, b: &Bar) -> anyhow::Result<()> {
        let id = b.bar_type.instrument_id();
        let (symbol, venue) = names(&id);
        let spec = b.bar_type.to_string();
        let len = BarEncoder::compute_length_with_header(symbol.len(), venue.len(), spec.len());
        persist_client::record(BarEncoder::TEMPLATE_ID, len, |buf| {
            Ok(BarEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&BarFixedFields {
                    ts_event: b.ts_event.as_u64(),
                    ts_init: b.ts_init.as_u64(),
                    open: d9(b.open.as_decimal())?,
                    high: d9(b.high.as_decimal())?,
                    low: d9(b.low.as_decimal())?,
                    close: d9(b.close.as_decimal())?,
                    volume: d9(b.volume.as_decimal())?,
                })
                .symbol(symbol)?
                .venue(venue)?
                .bar_type(spec.as_bytes())?
                .encoded_length_with_header())
        })
    }

    fn on_mark_price(&mut self, m: &MarkPriceUpdate) -> anyhow::Result<()> {
        self.tickers.entry(m.instrument_id).or_default().mark = Some(m.value.as_f64());
        let (symbol, venue) = names(&m.instrument_id);
        let len = MarkPriceEncoder::compute_length_with_header(symbol.len(), venue.len());
        persist_client::record(MarkPriceEncoder::TEMPLATE_ID, len, |buf| {
            Ok(MarkPriceEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&MarkPriceFixedFields {
                    ts_event: m.ts_event.as_u64(),
                    ts_init: m.ts_init.as_u64(),
                    price: d9(m.value.as_decimal())?,
                })
                .symbol(symbol)?
                .venue(venue)?
                .encoded_length_with_header())
        })
    }

    fn on_index_price(&mut self, x: &IndexPriceUpdate) -> anyhow::Result<()> {
        self.tickers.entry(x.instrument_id).or_default().index = Some(x.value.as_f64());
        let (symbol, venue) = names(&x.instrument_id);
        let len = IndexPriceEncoder::compute_length_with_header(symbol.len(), venue.len());
        persist_client::record(IndexPriceEncoder::TEMPLATE_ID, len, |buf| {
            Ok(IndexPriceEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&IndexPriceFixedFields {
                    ts_event: x.ts_event.as_u64(),
                    ts_init: x.ts_init.as_u64(),
                    price: d9(x.value.as_decimal())?,
                })
                .symbol(symbol)?
                .venue(venue)?
                .encoded_length_with_header())
        })
    }

    fn on_funding_rate(&mut self, f: &FundingRateUpdate) -> anyhow::Result<()> {
        self.tickers.entry(f.instrument_id).or_default().funding = f.rate.to_f64();
        let (symbol, venue) = names(&f.instrument_id);
        let len = FundingRateEncoder::compute_length_with_header(symbol.len(), venue.len());
        persist_client::record(FundingRateEncoder::TEMPLATE_ID, len, |buf| {
            Ok(FundingRateEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&FundingRateFixedFields {
                    ts_event: f.ts_event.as_u64(),
                    ts_init: f.ts_init.as_u64(),
                    rate: rate(f.rate)?,
                    interval_minutes: f.interval,
                    next_funding_ts: f.next_funding_ns.map(|t| t.as_u64()),
                })
                .symbol(symbol)?
                .venue(venue)?
                .encoded_length_with_header())
        })
    }

    /// Venue-specific data: into `ticker` where it fits, and every field, as
    /// the venue sends it, into a table named after its type
    /// (`HyperliquidPublicTrade` -> `hyperliquid_public_trade`).
    fn on_data(&mut self, data: &CustomData) -> anyhow::Result<()> {
        let any = data.data.as_any();
        if let Some(oi) = any.downcast_ref::<HyperliquidOpenInterest>() {
            self.tickers
                .entry(oi.instrument_id)
                .or_default()
                .open_interest = oi.open_interest.to_f64();
        } else if let Some(v) = any.downcast_ref::<DeribitVolatilityIndex>() {
            self.volatility.insert(v.index_name.clone(), v.volatility);
        }
        let table = persist_client::snake_case(data.data.type_name());
        if !persist_client::event_enabled(&table) {
            return Ok(());
        }
        // ponytail: a JSON round trip per row; these arrive a few a second.
        let serde_json::Value::Object(fields) = serde_json::from_str(&data.data.to_json()?)? else {
            return Ok(());
        };
        let nested: Vec<(&str, String)> = fields
            .iter()
            .filter(|(_, v)| v.is_object() || v.is_array())
            .map(|(k, v)| (k.as_str(), v.to_string()))
            .collect();
        let scalars = fields.iter().filter_map(|(k, v)| {
            let value = match v {
                serde_json::Value::Bool(b) => Value::Bool(*b),
                serde_json::Value::Number(n) => n
                    .as_i64()
                    .map(Value::I64)
                    .or_else(|| n.as_u64().map(Value::U64))
                    .or_else(|| n.as_f64().map(Value::F64))?,
                // Decimals arrive as text and stay text: exact, and
                // `toDecimal64(x, 9)` in a query when a number is wanted.
                serde_json::Value::String(s) => Value::Str(s),
                _ => return None,
            };
            (k != "type").then_some((k.as_str(), value))
        });
        let venue = self.venue.name.to_uppercase();
        persist_client::record_row(
            &table,
            scalars
                .chain(nested.iter().map(|(k, v)| (*k, Value::Str(v))))
                .chain([("venue", Value::Str(&venue))]),
        );
        Ok(())
    }
}

impl Recorder {
    /// One `ticker` row: the instrument's last second. Its columns are
    /// whatever this venue has, so a venue with more data adds columns to the
    /// table the moment it is deployed.
    fn ticker(&mut self, book: &OrderBook) {
        if !persist_client::event_enabled("ticker") {
            return;
        }
        let id = book.instrument_id;
        let volatility = id
            .symbol
            .as_str()
            .split('-')
            .next()
            .and_then(|base| self.volatility.get(&format!("{}_usd", base.to_lowercase())))
            .copied();
        let t = self.tickers.entry(id).or_default();
        tracing::info!(
            table = "ticker",
            instrument = %id,
            venue = %id.venue,
            bid = book.best_bid_price().map(|p| p.as_f64()),
            ask = book.best_ask_price().map(|p| p.as_f64()),
            last = t.last,
            trades = std::mem::take(&mut t.trades),
            volume = std::mem::take(&mut t.volume),
            mark_price = t.mark,
            index_price = t.index,
            funding_rate = t.funding,
            open_interest = t.open_interest,
            volatility_index = volatility,
        );
    }
}

/// A nested Rust value recorded as it is, with no schema: `record_value`
/// makes `book_view` from the struct. `spread.bps`, `bids.price` as
/// `Array(Nullable(Float64))`, `regime` and `regime.Wide.bps`, and a NULL
/// `imbalance` when a side is empty.
#[derive(serde::Serialize)]
struct BookView<'a> {
    instrument: &'a str,
    mid: f64,
    spread: Spread,
    bids: &'a [Level],
    asks: &'a [Level],
    /// Bid size over total size at the top 5 levels; `None` if both are empty.
    imbalance: Option<f64>,
    regime: Regime,
}

#[derive(serde::Serialize)]
struct Spread {
    bps: f64,
    ticks: f64,
}

#[derive(Clone, Copy, serde::Serialize)]
struct Level {
    price: f64,
    size: f64,
}

#[derive(serde::Serialize)]
enum Regime {
    Tight,
    Wide { bps: f64 },
}

/// One `book_view` row: the book's top 5 levels a side, and what they say.
fn book_view(book: &OrderBook) {
    if !persist_client::event_enabled("book_view") {
        return;
    }
    let top = |side: &mut dyn Iterator<Item = &BookLevel>| -> ArrayVec<Level, 5> {
        side.take(5)
            .map(|l| Level {
                price: l.price.value.as_f64(),
                size: l.size(),
            })
            .collect()
    };
    let (bids, asks) = (top(&mut book.bids(None)), top(&mut book.asks(None)));
    let (Some(bid), Some(ask)) = (bids.first(), asks.first()) else {
        return;
    };
    let mid = (bid.price + ask.price) / 2.0;
    let bps = (ask.price - bid.price) / mid * 1e4;
    let bid_size: f64 = bids.iter().map(|l| l.size).sum();
    let total = bid_size + asks.iter().map(|l| l.size).sum::<f64>();
    // The price precision is the tick size here: the best bid's.
    let precision = book
        .bids(None)
        .next()
        .map_or(0, |l| l.price.value.precision);
    let tick = 10f64.powi(-i32::from(precision));
    persist_client::record_value(
        "book_view",
        &BookView {
            instrument: book.instrument_id.symbol.as_str(),
            mid,
            spread: Spread {
                bps,
                ticks: (ask.price - bid.price) / tick,
            },
            bids: &bids,
            asks: &asks,
            imbalance: (total > 0.0).then(|| bid_size / total),
            regime: if bps < 1.0 {
                Regime::Tight
            } else {
                Regime::Wide { bps }
            },
        },
    );
}

/// `d` exactly, as the schema's `Decimal9` (mantissa x 10^-9, stored as
/// ClickHouse `Decimal(18, 9)`), by the generated conversion. More than nine
/// decimals, or more than +-9.2 billion, is an error, never a rounded value.
fn d9(d: Decimal) -> anyhow::Result<Decimal9> {
    d.try_to_sbe().map_err(anyhow::Error::msg)
}

/// `d` exactly, as the schema's `Rate` (mantissa x 10^-18): funding rates
/// carry more decimals than `Decimal9` holds.
fn rate(d: Decimal) -> anyhow::Result<market::Rate> {
    d.try_to_sbe().map_err(anyhow::Error::msg)
}

/// The best [`BOOK_LEVELS`] levels of one side as `(price, size)`, on the stack.
fn levels<'a>(
    side: impl Iterator<Item = &'a BookLevel>,
) -> anyhow::Result<ArrayVec<(Decimal9, Decimal9), BOOK_LEVELS>> {
    side.take(BOOK_LEVELS)
        .map(|l| Ok((d9(l.price.value.as_decimal())?, d9(l.size_decimal())?)))
        .collect()
}

/// An instrument's symbol and venue: the var-data every message ends with.
fn names(id: &InstrumentId) -> (&[u8], &[u8]) {
    (id.symbol.as_str().as_bytes(), id.venue.as_str().as_bytes())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exchange = std::env::var("EXCHANGE").unwrap_or_else(|_| "binance".into());
    let venue = VENUES.iter().find(|v| v.name == exchange).ok_or_else(|| {
        let names: Vec<_> = VENUES.iter().map(|v| v.name).collect();
        format!("EXCHANGE={exchange}: expected one of {names:?}")
    })?;
    let trader = TraderId::from(format!("RECORDER-{}", venue.name.to_uppercase()).as_str());
    let builder = LiveNode::builder(trader, Environment::Live)?;
    let builder = match venue.name {
        "binance" => builder.add_data_client(
            None,
            Box::new(BinanceDataClientFactory::new()),
            Box::new(BinanceDataClientConfig {
                spot_market_data_mode: BinanceSpotMarketDataMode::Json,
                ..Default::default()
            }),
        )?,
        "bybit" => builder.add_data_client(
            None,
            Box::new(BybitDataClientFactory::new()),
            Box::new(BybitDataClientConfig {
                product_types: vec![BybitProductType::Linear],
                ..Default::default()
            }),
        )?,
        "okx" => builder.add_data_client(
            None,
            Box::new(OKXDataClientFactory::new()),
            Box::new(OKXDataClientConfig {
                instrument_types: vec![OKXInstrumentType::Swap],
                ..Default::default()
            }),
        )?,
        "deribit" => builder.add_data_client(
            None,
            Box::new(DeribitDataClientFactory::new()),
            Box::new(DeribitDataClientConfig::default()),
        )?,
        _ => builder.add_data_client(
            None,
            Box::new(HyperliquidDataClientFactory::new()),
            Box::new(HyperliquidDataClientConfig::default()),
        )?,
    };
    let mut node = builder.build()?;

    // After `build()`, so persist's log lines go through Nautilus' logger.
    // Installed for the process: every callback records through
    // `persist_client::record` and friends, with no handle to pass around.
    let persist = Persist::connect(SCHEMA, Settings::from_env())?;
    persist.install();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;
    node.add_actor(Recorder {
        core: DataActorCore::new(DataActorConfig::default()),
        venue,
        tickers: HashMap::new(),
        volatility: HashMap::new(),
        deltas: Vec::with_capacity(DELTAS_PER_ROW),
    })?;
    node.run().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d9_is_exact_or_an_error() -> anyhow::Result<()> {
        assert_eq!(d9(Decimal::new(15, 1))?.mantissa(), 1_500_000_000);
        // Nautilus' 16-digit fixed point with trailing zeros.
        assert_eq!(
            d9(Decimal::from_i128_with_scale(
                650_001_200_000_000_000_000,
                16
            ))?
            .mantissa(),
            65_000_120_000_000
        );
        assert!(
            d9(Decimal::new(1, 10)).is_err(),
            "a tenth decimal is not rounded away"
        );
        assert!(
            d9(Decimal::new(10_000_000_000, 0)).is_err(),
            "too large for Decimal(18, 9)"
        );
        // Hyperliquid's BTC funding rate: ten decimals, one too many for d9.
        let funding = Decimal::new(112_659, 10);
        assert!(d9(funding).is_err());
        assert_eq!(rate(funding)?.mantissa(), 11_265_900_000_000);
        Ok(())
    }

    #[test]
    fn every_venue_parses() -> anyhow::Result<()> {
        for venue in &VENUES {
            for id in venue.instruments.map(InstrumentId::from) {
                assert_eq!(id.venue.as_str(), venue.name.to_uppercase());
                let bars = BarType::from(format!("{id}-1-MINUTE-LAST-EXTERNAL").as_str());
                assert_eq!(bars.instrument_id(), id);
            }
        }
        Ok(())
    }
}
