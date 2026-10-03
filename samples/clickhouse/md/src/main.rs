//! One exchange's public market data. `EXCHANGE` picks the venue. No API keys.
//!
//! Quotes and trades are SBE `record` calls. `ticker`, `spread`, and
//! `book_view` go out once a second with the book, not on every quote.
//! Counters, gauges, and histograms are updated on the callback and published
//! from the metrics thread. `book_update` is a checkpoint trace.
//!
//! `kind` in `tables.yaml` chooses static or dynamic. The handle is installed
//! once in `main`. With none installed, the free functions do nothing.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::HashMap;
use std::num::NonZeroUsize;

use arrayvec::{ArrayString, ArrayVec};

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
use nautilus_hyperliquid::data_types::{HyperliquidOpenInterest, HyperliquidPublicTrade};
use nautilus_hyperliquid::factories::HyperliquidDataClientFactory;
use nautilus_kraken::common::enums::KrakenProductType;
use nautilus_kraken::config::KrakenDataClientConfig;
use nautilus_kraken::factories::KrakenDataClientFactory;
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
use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::event::Value;
use ergon_runtime::metrics::{Counter, Gauge, Histogram};
use ergon_runtime::persist::Persist;
use ergon_runtime::trace::Tracer;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use schema::market::{
    BarEncoder, BarFixedFields, BookAction, BookDeltasDeltasEntry, BookDeltasEncoder,
    BookDeltasFixedFields, BookSnapshotAsksEntry, BookSnapshotBidsEntry, BookSnapshotEncoder,
    BookSnapshotFixedFields, Decimal9, FundingRateEncoder, FundingRateFixedFields,
    IndexPriceEncoder, IndexPriceFixedFields, InstrumentSpecEncoder, InstrumentSpecFixedFields,
    MarkPriceEncoder, MarkPriceFixedFields, QuoteEncoder, QuoteFixedFields, Rate, Side,
    TradeEncoder, TradeFixedFields, TryToSbe,
};
use tracing_subscriber::layer::SubscriberExt;

/// Levels per side in each `book_snapshot` row.
const BOOK_LEVELS: usize = 10;
/// At most this many book changes per `book_deltas` message; fewer when one
/// UDP frame holds fewer (see [`Feeds::open`]).
const MAX_DELTAS_PER_ROW: usize = 1000;

/// An exchange this recorder can follow.
#[derive(Debug)]
struct Venue {
    /// `EXCHANGE`, and the Nautilus client id upper-cased.
    name: &'static str,
    /// The default instruments: the venue's most traded, so the feeds carry
    /// real volume. `INSTRUMENTS` (comma-separated) replaces them.
    instruments: &'static [&'static str],
    /// A book depth the venue streams.
    depth: usize,
    /// Perpetual swaps: mark price, index price and funding rate.
    perpetual: bool,
    /// The venue streams one-minute bars.
    bars: bool,
    /// Data only this venue offers: `(type, metadata key, metadata value)`.
    custom: &'static [(&'static str, &'static str, &'static str)],
}

const VENUES: [Venue; 6] = [
    Venue {
        name: "binance",
        instruments: &[
            "BTCUSDT.BINANCE",
            "ETHUSDT.BINANCE",
            "SOLUSDT.BINANCE",
            "BNBUSDT.BINANCE",
            "XRPUSDT.BINANCE",
            "DOGEUSDT.BINANCE",
            "ADAUSDT.BINANCE",
            "TRXUSDT.BINANCE",
            "LINKUSDT.BINANCE",
            "AVAXUSDT.BINANCE",
        ],
        depth: 20, // spot streams 5/10/20
        perpetual: false,
        bars: true,
        custom: &[],
    },
    Venue {
        name: "bybit",
        instruments: &[
            "BTCUSDT-LINEAR.BYBIT",
            "ETHUSDT-LINEAR.BYBIT",
            "SOLUSDT-LINEAR.BYBIT",
            "BNBUSDT-LINEAR.BYBIT",
            "XRPUSDT-LINEAR.BYBIT",
            "DOGEUSDT-LINEAR.BYBIT",
            "ADAUSDT-LINEAR.BYBIT",
            "TRXUSDT-LINEAR.BYBIT",
            "LINKUSDT-LINEAR.BYBIT",
            "AVAXUSDT-LINEAR.BYBIT",
        ],
        depth: 50, // 1/50/200/1000
        perpetual: true,
        bars: true,
        custom: &[],
    },
    Venue {
        name: "okx",
        instruments: &[
            "BTC-USDT-SWAP.OKX",
            "ETH-USDT-SWAP.OKX",
            "SOL-USDT-SWAP.OKX",
            "BNB-USDT-SWAP.OKX",
            "XRP-USDT-SWAP.OKX",
            "DOGE-USDT-SWAP.OKX",
            "ADA-USDT-SWAP.OKX",
            "TRX-USDT-SWAP.OKX",
            "LINK-USDT-SWAP.OKX",
            "AVAX-USDT-SWAP.OKX",
        ],
        depth: 50, // 50 or 400
        perpetual: true,
        bars: true,
        custom: &[],
    },
    Venue {
        name: "deribit",
        instruments: &[
            "BTC-PERPETUAL.DERIBIT",
            "ETH-PERPETUAL.DERIBIT",
            "SOL_USDC-PERPETUAL.DERIBIT",
            "BNB_USDC-PERPETUAL.DERIBIT",
            "XRP_USDC-PERPETUAL.DERIBIT",
            "DOGE_USDC-PERPETUAL.DERIBIT",
            "ADA_USDC-PERPETUAL.DERIBIT",
            "TRX_USDC-PERPETUAL.DERIBIT",
            "LINK_USDC-PERPETUAL.DERIBIT",
            "AVAX_USDC-PERPETUAL.DERIBIT",
        ],
        depth: 20, // 1/10/20
        perpetual: true,
        bars: true,
        custom: &[
            ("DeribitVolatilityIndex", "index_name", "btc_usd"),
            ("DeribitVolatilityIndex", "index_name", "eth_usd"),
        ],
    },
    Venue {
        name: "hyperliquid",
        instruments: &[
            "BTC-USD-PERP.HYPERLIQUID",
            "ETH-USD-PERP.HYPERLIQUID",
            "SOL-USD-PERP.HYPERLIQUID",
            "BNB-USD-PERP.HYPERLIQUID",
            "XRP-USD-PERP.HYPERLIQUID",
            "DOGE-USD-PERP.HYPERLIQUID",
            "ADA-USD-PERP.HYPERLIQUID",
            "TRX-USD-PERP.HYPERLIQUID",
            "LINK-USD-PERP.HYPERLIQUID",
            "AVAX-USD-PERP.HYPERLIQUID",
        ],
        depth: 20,
        perpetual: true,
        bars: true,
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
    Venue {
        name: "kraken",
        // Kraken Futures' linear perpetuals; Kraken calls bitcoin XBT.
        instruments: &[
            "PF_XBTUSD.KRAKEN",
            "PF_ETHUSD.KRAKEN",
            "PF_SOLUSD.KRAKEN",
            "PF_BNBUSD.KRAKEN",
            "PF_XRPUSD.KRAKEN",
            "PF_DOGEUSD.KRAKEN",
            "PF_ADAUSD.KRAKEN",
            "PF_TRXUSD.KRAKEN",
            "PF_LINKUSD.KRAKEN",
            "PF_AVAXUSD.KRAKEN",
        ],
        depth: 25,
        perpetual: true,
        bars: false, // Kraken Futures streams no bars
        custom: &[],
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
    instruments: Vec<InstrumentId>,
    tickers: HashMap<InstrumentId, Ticker>,
    /// Deribit's volatility index (DVOL) by index name, e.g. `btc_usd`: a
    /// handful, scanned, so a lookup by asset allocates nothing.
    volatility: Vec<(String, f64)>,
    /// `EXCHANGE` upper-cased once: every custom row's `venue`.
    venue_upper: String,
    /// One `book_deltas` row's changes, reused.
    deltas: Vec<BookDeltasDeltasEntry>,
    t: Telemetry,
    feeds: Feeds,
}

/// This exchange's feeds (config/streams.yaml): market data other
/// applications subscribe to, recorded by the archive of the node it runs on.
struct Feeds {
    /// Reliable: trades, order book changes and snapshots, bars, mark and
    /// index prices, funding rates.
    md: ergon_runtime::publication::Publication,
    /// Best effort: quotes (top of book).
    tob: ergon_runtime::publication::Publication,
    /// Book changes per `book_deltas` message: as many as one UDP frame holds.
    deltas_per_row: usize,
}

impl std::fmt::Debug for Feeds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Feeds")
            .field("md", &self.md.stream_id())
            .field("tob", &self.tob.stream_id())
            .finish()
    }
}

impl Feeds {
    /// Open `service`'s feeds from the registry, on the bus's node.
    fn open(bus: &Bus, service: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let streams = ergon_runtime::streams::Streams::load(
            std::env::var("PERSIST_STREAMS").unwrap_or_else(|_| "config/streams.yaml".into()),
        )?;
        ergon_runtime::streams::check_node_network()?;
        let md = bus.publish(&streams, service, "md")?;
        let tob = bus.publish(&streams, service, "tob")?;
        // Room for the longest symbol and venue name in these feeds.
        let fits =
            |n: usize| BookDeltasEncoder::compute_length_with_header(n, 32, 16) <= md.max_payload();
        let deltas_per_row = (1..=MAX_DELTAS_PER_ROW)
            .rev()
            .find(|&n| fits(n))
            .unwrap_or(1);
        log::info!(
            "feeds {service}: md stream {}, tob stream {}, from {}; {deltas_per_row} book changes a message",
            md.stream_id(),
            tob.stream_id(),
            bus.host_ip()
        );
        Ok(Self {
            md,
            tob,
            deltas_per_row,
        })
    }
}

/// The recorder's own metrics and trace. Made once; each update is a load
/// and a store on this thread.
#[derive(Debug)]
struct Telemetry {
    clock: Clock,
    trades: Counter,
    quotes: Counter,
    books: Counter,
    deltas: Counter,
    /// The venue's timestamp to our handler, ns.
    trade_latency: Histogram,
    quote_latency: Histogram,
    /// How long `ergon_runtime::persist::record` of a trade takes, ns.
    record_ns: Histogram,
    spread_bps: HashMap<InstrumentId, Gauge>,
    /// Each book change, from the venue's timestamp: `wire`, `convert`,
    /// `record`.
    book_update: Tracer,
}

impl Telemetry {
    fn new(instruments: &[InstrumentId]) -> Self {
        let m = ergon_runtime::persist::metrics();
        let kind = |k| m.counter("messages", &[("kind", k)]);
        let latency = |k| m.histogram("venue_to_local_ns", &[("kind", k)]);
        Self {
            clock: Clock::new(),
            trades: kind("trade"),
            quotes: kind("quote"),
            books: kind("book"),
            deltas: kind("book_deltas"),
            trade_latency: latency("trade"),
            quote_latency: latency("quote"),
            record_ns: m.histogram("record_ns", &[("table", "trade")]),
            spread_bps: instruments
                .iter()
                .map(|&id| {
                    let gauge = m.gauge("spread_bps", &[("instrument", &id.to_string())]);
                    (id, gauge)
                })
                .collect(),
            book_update: ergon_runtime::persist::tracer(
                "book_update",
                &["wire", "convert", "record"],
                &["deltas"],
            ),
        }
    }

    /// Ns from the venue's `ts_event` to now, both wall clocks; 0 when the
    /// venue's is ahead.
    fn since_venue(&self, ts_event: u64) -> u64 {
        self.clock
            .wall()
            .since(Nanos::from_epoch(ts_event as i64))
            .max(0) as u64
    }
}

nautilus_actor!(Recorder);

impl DataActor for Recorder {
    fn on_start(&mut self) -> anyhow::Result<()> {
        // A `tracing` span: recorded while `otel_traces` is on for this app.
        let _span = tracing::info_span!("subscribe", venue = self.venue.name).entered();
        let second = NonZeroUsize::try_from(1000)?;
        let depth = NonZeroUsize::new(self.venue.depth);
        for id in self.instruments.clone() {
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
            if self.venue.bars {
                self.subscribe_bars(
                    BarType::from(format!("{id}-1-MINUTE-LAST-EXTERNAL")),
                    None,
                    None,
                );
            }
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
        self.spec(i)
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
        self.t.trades.inc();
        let latency = self.t.since_venue(t.ts_event.as_u64());
        self.t.trade_latency.record(latency);
        let started = self.t.clock.cached();
        let ticker = self.tickers.entry(t.instrument_id).or_default();
        ticker.last = Some(t.price.as_f64());
        ticker.trades += 1;
        ticker.volume += t.size.as_f64();
        let (symbol, venue) = names(&t.instrument_id);
        let trade_id = t.trade_id.as_str().as_bytes();
        let len =
            TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
        self.feeds
            .md
            .record(TradeEncoder::TEMPLATE_ID, len, |buf| -> anyhow::Result<_> {
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
            })?;
        let took = self.t.clock.now().since(started);
        self.t.record_ns.record(took.max(0) as u64);
        Ok(())
    }

    fn on_quote(&mut self, q: &QuoteTick) -> anyhow::Result<()> {
        self.t.quotes.inc();
        let latency = self.t.since_venue(q.ts_event.as_u64());
        self.t.quote_latency.record(latency);
        let (symbol, venue) = names(&q.instrument_id);
        let len = QuoteEncoder::compute_length_with_header(symbol.len(), venue.len());
        self.feeds
            .tob
            .record(QuoteEncoder::TEMPLATE_ID, len, |buf| -> anyhow::Result<_> {
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
        // The hot path keeps a gauge. The `spread` table is a tracing event
        // on the once-a-second book, below.
        let (bid, ask) = (q.bid_price.as_f64(), q.ask_price.as_f64());
        if let Some(gauge) = self.t.spread_bps.get(&q.instrument_id) {
            gauge.set((ask - bid) / (ask + bid) * 2e4);
        }
        Ok(())
    }

    fn on_book(&mut self, book: &OrderBook) -> anyhow::Result<()> {
        self.t.books.inc();
        self.spread(book);
        self.ticker(book);
        book_view(book);
        // Published whether or not `book_snapshot` is persisted: a
        // subscriber that joins or falls behind resyncs its book from it.
        if let Some(instrument) = self.cache().instrument(&book.instrument_id) {
            self.spec(&instrument)?;
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
        self.feeds.md.record(
            BookSnapshotEncoder::TEMPLATE_ID,
            len,
            |buf| -> anyhow::Result<_> {
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
            },
        )
    }

    fn on_book_deltas(&mut self, d: &OrderBookDeltas) -> anyhow::Result<()> {
        self.t.deltas.inc();
        // A checkpoint trace from the venue's timestamp: `wire` (the venue
        // and the network), `convert` (rows built), `record` (published).
        let tracer = &self.t.book_update;
        // From the venue's time, placed by its wall-clock age: the `wire`
        // stage is true whatever this process's clock has drifted.
        let now = self.t.clock.now();
        let mut trace = tracer.start(
            self.t.clock.from_remote(d.ts_event.as_u64() as i64, now),
            tracer.next_id(),
        );
        trace.mark(now);
        trace.attr(0, d.deltas.len() as i64);
        let (symbol, venue) = names(&d.instrument_id);
        for (i, chunk) in d.deltas.chunks(self.feeds.deltas_per_row).enumerate() {
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
            if i == 0 {
                trace.mark(self.t.clock.now());
            }
            let deltas = &self.deltas;
            let len = BookDeltasEncoder::compute_length_with_header(
                deltas.len(),
                symbol.len(),
                venue.len(),
            );
            self.feeds.md.record(
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
        trace.mark(self.t.clock.now());
        trace.finish();
        Ok(())
    }

    fn on_bar(&mut self, b: &Bar) -> anyhow::Result<()> {
        let id = b.bar_type.instrument_id();
        let (symbol, venue) = names(&id);
        let spec = text::<96>(b.bar_type);
        let len = BarEncoder::compute_length_with_header(symbol.len(), venue.len(), spec.len());
        self.feeds.md.record(BarEncoder::TEMPLATE_ID, len, |buf| {
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
        self.feeds
            .md
            .record(MarkPriceEncoder::TEMPLATE_ID, len, |buf| {
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
        self.feeds
            .md
            .record(IndexPriceEncoder::TEMPLATE_ID, len, |buf| {
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
        self.feeds
            .md
            .record(FundingRateEncoder::TEMPLATE_ID, len, |buf| {
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
        // The types this recorder subscribes to, written field by field with
        // no allocation: a public trade arrives with every Hyperliquid trade.
        if let Some(t) = any.downcast_ref::<HyperliquidPublicTrade>() {
            self.public_trade(t);
            return Ok(());
        }
        if let Some(oi) = any.downcast_ref::<HyperliquidOpenInterest>() {
            self.tickers
                .entry(oi.instrument_id)
                .or_default()
                .open_interest = oi.open_interest.to_f64();
            self.open_interest(oi);
            return Ok(());
        }
        if let Some(v) = any.downcast_ref::<DeribitVolatilityIndex>() {
            match self
                .volatility
                .iter_mut()
                .find(|(name, _)| *name == v.index_name)
            {
                Some((_, value)) => *value = v.volatility,
                None => self.volatility.push((v.index_name.clone(), v.volatility)),
            }
            self.volatility_index(v);
            return Ok(());
        }
        // Any other custom type: rare, so a JSON round trip is fine here.
        let table = ergon_runtime::persist::snake_case(data.data.type_name());
        if !ergon_runtime::persist::event_enabled(&table) {
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
        ergon_runtime::persist::record_row(
            &table,
            scalars
                .chain(nested.iter().map(|(k, v)| (*k, Value::Str(v))))
                .chain([("venue", Value::Str(&self.venue_upper))]),
        );
        Ok(())
    }
}

impl Recorder {
    /// `hyperliquid_public_trade`: the columns and text the generic JSON path
    /// wrote, with the decimals and ids formatted on the stack.
    fn public_trade(&self, t: &HyperliquidPublicTrade) {
        const TABLE: &str = "hyperliquid_public_trade";
        if !ergon_runtime::persist::event_enabled(TABLE) {
            return;
        }
        let (id, price, size) = (
            text::<64>(t.instrument_id),
            text::<48>(t.price),
            text::<48>(t.size),
        );
        let side = text::<16>(t.aggressor_side);
        ergon_runtime::persist::record_row(
            TABLE,
            [
                ("instrument_id", Value::Str(&id)),
                ("price", Value::Str(&price)),
                ("size", Value::Str(&size)),
                ("aggressor_side", Value::Str(&side)),
                ("trade_id", Value::Str(&t.trade_id)),
                ("buyer", Value::Str(&t.buyer)),
                ("seller", Value::Str(&t.seller)),
                ("hash", Value::Str(&t.hash)),
                ("ts_event", Value::I64(t.ts_event.as_u64() as i64)),
                ("ts_init", Value::I64(t.ts_init.as_u64() as i64)),
                ("venue", Value::Str(&self.venue_upper)),
            ],
        );
    }

    /// `hyperliquid_open_interest`, as [`Self::public_trade`].
    fn open_interest(&self, oi: &HyperliquidOpenInterest) {
        const TABLE: &str = "hyperliquid_open_interest";
        if !ergon_runtime::persist::event_enabled(TABLE) {
            return;
        }
        let (id, open_interest) = (text::<64>(oi.instrument_id), text::<48>(oi.open_interest));
        ergon_runtime::persist::record_row(
            TABLE,
            [
                ("instrument_id", Value::Str(&id)),
                ("open_interest", Value::Str(&open_interest)),
                ("ts_event", Value::I64(oi.ts_event.as_u64() as i64)),
                ("ts_init", Value::I64(oi.ts_init.as_u64() as i64)),
                ("venue", Value::Str(&self.venue_upper)),
            ],
        );
    }

    /// `deribit_volatility_index`, as [`Self::public_trade`].
    fn volatility_index(&self, v: &DeribitVolatilityIndex) {
        const TABLE: &str = "deribit_volatility_index";
        if !ergon_runtime::persist::event_enabled(TABLE) {
            return;
        }
        ergon_runtime::persist::record_row(
            TABLE,
            [
                ("index_name", Value::Str(&v.index_name)),
                ("ts_event", Value::I64(v.ts_event.as_u64() as i64)),
                ("ts_init", Value::I64(v.ts_init.as_u64() as i64)),
                ("volatility", Value::F64(v.volatility)),
                ("venue", Value::Str(&self.venue_upper)),
            ],
        );
    }

    /// The DVOL of `base` (`BTC` reads `btc_usd`), with no allocation.
    fn volatility_of(&self, base: &str) -> Option<f64> {
        self.volatility
            .iter()
            .find(|(name, _)| {
                name.len() == base.len() + 4
                    && name.ends_with("_usd")
                    && name[..base.len()].eq_ignore_ascii_case(base)
            })
            .map(|&(_, value)| value)
    }

    /// How to read `i`'s sizes, on the md feed: when it is loaded, and before
    /// each book snapshot, so a subscriber that joins late or resyncs has it.
    fn spec(&self, i: &InstrumentAny) -> anyhow::Result<()> {
        let id = i.id();
        let (symbol, venue) = names(&id);
        let base = asset(i.base_currency().map_or("", |c| c.code.as_str()));
        let quote = i.quote_currency().code.as_str();
        let len = InstrumentSpecEncoder::compute_length_with_header(
            symbol.len(),
            venue.len(),
            base.len(),
            quote.len(),
        );
        let ts = self.core.timestamp_ns().as_u64();
        self.feeds
            .md
            .record(InstrumentSpecEncoder::TEMPLATE_ID, len, |buf| {
                Ok(InstrumentSpecEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&InstrumentSpecFixedFields {
                        ts_event: ts,
                        multiplier: d9(i.multiplier().as_decimal())?,
                        inverse: u8::from(i.is_inverse()),
                    })
                    .symbol(symbol)?
                    .venue(venue)?
                    .base(base.as_bytes())?
                    .quote(quote.as_bytes())?
                    .encoded_length_with_header())
            })
    }

    /// One `spread` row a second. A `tracing` event builds a table from its
    /// fields, and it does not belong on the quote callback.
    fn spread(&self, book: &OrderBook) {
        let (Some(bid), Some(ask)) = (book.best_bid_price(), book.best_ask_price()) else {
            return;
        };
        let (bid, ask) = (bid.as_f64(), ask.as_f64());
        let mid = ask + bid;
        if mid == 0.0 {
            return;
        }
        tracing::info!(table = "spread", instrument = %book.instrument_id, bps = (ask - bid) / mid * 2e4);
    }

    /// One `ticker` row: the instrument's last second. Its columns are
    /// whatever this venue has, so a venue with more data adds columns to the
    /// table the moment it is deployed.
    fn ticker(&mut self, book: &OrderBook) {
        if !ergon_runtime::persist::event_enabled("ticker") {
            return;
        }
        let id = book.instrument_id;
        let volatility = id
            .symbol
            .as_str()
            .split('-')
            .next()
            .and_then(|base| self.volatility_of(base));
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
    if !ergon_runtime::persist::event_enabled("book_view") {
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
    ergon_runtime::persist::record_value(
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
    d.try_to_sbe().map_err(|e| anyhow::anyhow!("{e}: {d}"))
}

/// `d` exactly, as the schema's `Rate` (mantissa x 10^-18): funding rates
/// carry more decimals than `Decimal9` holds.
fn rate(d: Decimal) -> anyhow::Result<Rate> {
    d.try_to_sbe().map_err(|e| anyhow::anyhow!("{e}: {d}"))
}

/// `x` as text for a row's text field on the hot path: on the stack, `N`
/// bytes being generous for what is formatted here (ids, decimals, bar
/// types). Longer text is allocated rather than cut: a value is never lost.
fn text<const N: usize>(x: impl std::fmt::Display) -> Text<N> {
    use std::fmt::Write as _;
    let mut s = ArrayString::new();
    if write!(s, "{x}").is_ok() {
        Text::Stack(s)
    } else {
        Text::Heap(x.to_string())
    }
}

/// [`text`]'s result: a `&str` either way.
enum Text<const N: usize> {
    Stack(ArrayString<N>),
    Heap(String),
}

impl<const N: usize> std::ops::Deref for Text<N> {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            Self::Stack(s) => s,
            Self::Heap(s) => s,
        }
    }
}

/// The best [`BOOK_LEVELS`] levels of one side as `(price, size)`, on the stack.
fn levels<'a>(
    side: impl Iterator<Item = &'a BookLevel>,
) -> anyhow::Result<ArrayVec<(Decimal9, Decimal9), BOOK_LEVELS>> {
    side.take(BOOK_LEVELS)
        .map(|l| Ok((d9(l.price.value.as_decimal())?, d9(l.size_decimal())?)))
        .collect()
}

/// The asset `code` names: some venues call bitcoin XBT.
fn asset(code: &str) -> &str {
    if code == "XBT" { "BTC" } else { code }
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
    let instruments: Vec<InstrumentId> = match std::env::var("INSTRUMENTS") {
        Ok(list) => list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(InstrumentId::from)
            .collect(),
        Err(_) => venue
            .instruments
            .iter()
            .map(|&s| InstrumentId::from(s))
            .collect(),
    };
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
        "kraken" => builder.add_data_client(
            None,
            Box::new(KrakenDataClientFactory::new()),
            Box::new(KrakenDataClientConfig {
                product_type: KrakenProductType::Futures,
                ..Default::default()
            }),
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
    // `ergon_runtime::persist::record` and friends, with no handle to pass around.
    let settings = Settings::from_env();
    let bus = Bus::connect(&settings)?;
    let persist = Persist::connect(schema::MARKET_SCHEMA, &bus, settings)?;
    persist.install();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;
    let service = std::env::var("SERVICE").unwrap_or_else(|_| format!("md-{}", venue.name));
    let feeds = Feeds::open(&bus, &service)?;
    // SIGTERM (a restart, or a move to another node): close every
    // publication at once, so subscribers turn to the next pod of this feed
    // within seconds rather than after this client's timeout.
    let closing = bus.clone();
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            term.recv().await;
            log::info!("SIGTERM: closing the feeds");
            closing.shutdown();
            std::process::exit(0);
        }
    });
    // Publishes the metrics and applies `tables.yaml`. A busy-spinning
    // application calls `poll` from its own loop instead, with the time it
    // has; Nautilus owns this one's.
    let polled = persist.clone();
    let idle = ergon_runtime::idle::Idle::from_env("IDLE", ergon_runtime::idle::Idle::Sleep)?;
    std::thread::Builder::new()
        .name("persist".into())
        .spawn(move || {
            let clock = Clock::new();
            loop {
                polled.poll(clock.now());
                idle.idle(0);
            }
        })?;
    node.add_actor(Recorder {
        core: DataActorCore::new(DataActorConfig::default()),
        venue,
        t: Telemetry::new(&instruments),
        instruments,
        tickers: HashMap::new(),
        volatility: Vec::new(),
        venue_upper: venue.name.to_uppercase(),
        deltas: Vec::with_capacity(MAX_DELTAS_PER_ROW),
        feeds,
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
    fn bitcoin_is_one_asset_whatever_the_venue_calls_it() {
        assert_eq!(asset("XBT"), "BTC");
        assert_eq!(asset("BTC"), "BTC");
        assert_eq!(asset("ETH"), "ETH");
    }

    #[test]
    fn text_stays_on_the_stack_and_never_cuts() {
        assert!(matches!(text::<8>(42), Text::Stack(ref s) if s.as_str() == "42"));
        let long = text::<4>("BTC-USD-PERP.HYPERLIQUID");
        assert!(matches!(long, Text::Heap(_)));
        assert_eq!(&*long, "BTC-USD-PERP.HYPERLIQUID");
    }

    #[test]
    fn every_venue_parses() -> anyhow::Result<()> {
        for venue in &VENUES {
            for id in venue.instruments.iter().map(|&s| InstrumentId::from(s)) {
                assert_eq!(id.venue.as_str(), venue.name.to_uppercase());
                let bars = BarType::from(format!("{id}-1-MINUTE-LAST-EXTERNAL").as_str());
                assert_eq!(bars.instrument_id(), id);
            }
        }
        Ok(())
    }
}
