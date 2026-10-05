//! One exchange's public market data. `EXCHANGE` picks the venue. No API keys.
//!
//! Quotes and trades go out on the md feeds, SBE. `ticker`, `spread`, and
//! `book_view` are event rows once a second with the book, not on every
//! quote. Counters, gauges, and histograms are updated on the callback and
//! published by the runtime's housekeeping. `book_update` is a checkpoint
//! trace.
//!
//! `kind` in `tables.yaml` chooses static or dynamic. Every row goes through
//! the runtime's persist, on the actor's thread. `tracing` spans, from any
//! thread, go through the bridge.
//!
//! The actor builds the runtime, its Aeron client, persist and the bridge in
//! `on_start`, not in `main`: Nautilus connects its venue clients first, and
//! an Aeron client whose conductor nothing runs is closed after its 10 s
//! liveness timeout. From then on the actor's callbacks and a 1 ms timer run
//! the runtime's duty cycle: a venue callback's takes the event time, and the
//! timer's runs the Aeron conductor and due housekeeping, so no callback pays
//! for them unless a millisecond has passed. On SIGTERM it closes the feeds
//! and persist's publication at once and exits.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::HashMap;
use std::num::NonZeroUsize;

use arrayvec::{ArrayString, ArrayVec};

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::Nanos;
use ergon_runtime::event::Value;
use ergon_runtime::metrics::{Counter, Gauge, Histogram};
use ergon_runtime::persist::Persist;
use ergon_runtime::rt::{self, Agent, Config, Ctx, Expiry, FeedId, Invoker, Out};
use ergon_runtime::subscription::Delivery;
use ergon_runtime::trace::Tracer;
use nautilus_binance::config::{BinanceDataClientConfig, BinanceSpotMarketDataMode};
use nautilus_binance::factories::BinanceDataClientFactory;
use nautilus_bybit::common::enums::BybitProductType;
use nautilus_bybit::config::BybitDataClientConfig;
use nautilus_bybit::factories::BybitDataClientFactory;
use nautilus_common::actor::{DataActor, DataActorConfig, DataActorCore};
use nautilus_common::enums::Environment;
use nautilus_common::nautilus_actor;
use nautilus_common::timer::TimeEvent;
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
    /// What `on_start` builds the runtime from: this service's name, and the
    /// registry, its directory.
    start: Option<(String, lab::Streams)>,
    /// The runtime and what hangs off it, from `on_start` on.
    live: Option<Live>,
}

/// What the actor builds in `on_start`.
#[derive(Debug)]
struct Live {
    t: Telemetry,
    feeds: Feeds,
    /// Persist's housekeeping, the Aeron conductor, SIGTERM, and the feeds'
    /// `Source` heartbeat: the runtime in embedded mode, driven from this
    /// actor's callbacks.
    rt: Invoker,
    housekeeping: Housekeeping,
}

/// md takes no feeds and keeps no timers of its own: its runtime only does
/// housekeeping and runs the Aeron conductor.
#[derive(Debug, Default)]
struct Housekeeping {
    /// This cycle runs for a venue callback, which has work to do. As in any
    /// cycle that found work, the conductor waits for the millisecond
    /// timer's cycle, or a millisecond, and due housekeeping for that cycle,
    /// or half a millisecond.
    venue: bool,
}

impl Agent for Housekeeping {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        Ok(())
    }

    fn on_message(&mut self, _ctx: &mut Ctx, _feed: FeedId, _msg: &[u8], _d: Delivery) {}

    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}

    fn poll(&mut self, _ctx: &mut Ctx) -> usize {
        usize::from(self.venue)
    }
}

impl Recorder {
    /// One duty cycle of the runtime, on this actor's thread, for a venue
    /// callback (`venue`) or the millisecond timer: the event time, then on
    /// the timer's cycle (or once a callback's has waited a millisecond) due
    /// housekeeping and the Aeron conductor. On SIGTERM (a restart, or a move
    /// to another node) close every publication at once, so subscribers turn
    /// to the next pod of this feed within seconds rather than after this
    /// client's timeout.
    fn invoke(&mut self, venue: bool) {
        let Some(live) = &mut self.live else {
            return;
        };
        live.housekeeping.venue = venue;
        live.rt.cycle(&mut live.housekeeping);
        if live.rt.is_stopping() {
            if let Err(error) = live.rt.finish(&mut live.housekeeping) {
                log::error!("runtime shutdown failed: {error}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    /// The runtime, on this actor's thread: the bus, persist and the
    /// `tracing` bridge, the feeds and the telemetry. The actor makes every
    /// record, on persist's exclusive publication; a `tracing` span from an
    /// adapter's worker thread goes through the bridge, on a publication of
    /// its own. Log lines go through Nautilus' logger.
    fn runtime(&mut self) -> Result<Live, Box<dyn std::error::Error>> {
        let (service, streams) = self.start.take().ok_or("the runtime starts once")?;
        let settings = Settings::from_env();
        let bus = Bus::connect(&settings)?;
        let persist = Persist::connect(schema::MARKET_SCHEMA, &bus, settings)?;
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(persist.layer()?),
        )?;
        // No thread of its own: the actor drives it (`Recorder::invoke`),
        // from every callback and a millisecond timer. It polls persist,
        // applies `tables.yaml`, refreshes the wall-clock offset, runs the
        // Aeron conductor, and stops on SIGTERM.
        let mut rt = Invoker::new(Config {
            persist: Some(persist),
            stop: rt::sigterm()?,
            directory: Box::new(streams),
            ..Config::new(bus)
        })?;
        let feeds = Feeds::open(&mut rt, &service)?;
        rt.start(&mut Housekeeping::default())?;
        let t = Telemetry::new(&self.instruments, rt.ctx_ref());
        Ok(Live {
            t,
            feeds,
            rt,
            housekeeping: Housekeeping::default(),
        })
    }
}

/// This exchange's feeds (config/streams.yaml): market data other
/// applications subscribe to, recorded by the archive of the node it runs on.
struct Feeds {
    /// Reliable: trades, order book changes and snapshots, bars, mark and
    /// index prices, funding rates.
    md: Out,
    /// Best effort: quotes (top of book).
    tob: Out,
    /// Book changes per `book_deltas` message: as many as one UDP frame holds.
    deltas_per_row: usize,
}

impl std::fmt::Debug for Feeds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Feeds")
            .field("md", &self.md)
            .field("tob", &self.tob)
            .finish()
    }
}

impl Feeds {
    /// Open `service`'s feeds from the registry, on this node, as
    /// exclusive publications of the actor's thread.
    fn open(rt: &mut Invoker, service: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = rt.ctx();
        let md = ctx.publish(service, "md")?;
        let tob = ctx.publish(service, "tob")?;
        // Room for the longest symbol and venue name in these feeds.
        let max = ctx.max_payload(md);
        let fits = |n: usize| BookDeltasEncoder::compute_length_with_header(n, 32, 16) <= max;
        let deltas_per_row = (1..=MAX_DELTAS_PER_ROW)
            .rev()
            .find(|&n| fits(n))
            .unwrap_or(1);
        log::info!("feeds {service}: {deltas_per_row} book changes a message");
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
    trades: Counter,
    quotes: Counter,
    books: Counter,
    deltas: Counter,
    /// The venue's timestamp to our handler, ns.
    trade_latency: Histogram,
    quote_latency: Histogram,
    /// How long sending a trade takes, ns.
    record_ns: Histogram,
    spread_bps: HashMap<InstrumentId, Gauge>,
    /// Each book change, from the venue's timestamp: `wire`, `convert`,
    /// `record`.
    book_update: Tracer,
}

impl Telemetry {
    fn new(instruments: &[InstrumentId], ctx: &Ctx) -> Self {
        let m = ctx.metrics();
        let kind = |k| m.counter("messages", &[("kind", k)]);
        let latency = |k| m.histogram("venue_to_local_ns", &[("kind", k)]);
        Self {
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
            book_update: ctx.tracer("book_update", &["wire", "convert", "record"], &["deltas"]),
        }
    }

    /// Ns from the venue's `ts_event` to `wall_now`, both wall clocks; 0
    /// when the venue's is ahead.
    fn since_venue(ts_event: u64, wall_now: Nanos) -> u64 {
        wall_now.since(Nanos::from_epoch(ts_event as i64)).max(0) as u64
    }
}

nautilus_actor!(Recorder);

impl DataActor for Recorder {
    fn on_start(&mut self) -> anyhow::Result<()> {
        if self.live.is_none() {
            match self.runtime() {
                Ok(live) => self.live = Some(live),
                Err(error) => {
                    log::error!("the runtime did not start: {error}");
                    std::process::exit(1);
                }
            }
        }
        // The runtime's duty cycle every millisecond, quiet venue or not.
        self.clock().set_timer(
            "ergon-runtime",
            std::time::Duration::from_millis(1),
            None,
            None,
            None,
            None,
            None,
        )?;
        // A `tracing` span, through the bridge: kept while `otel_traces` is
        // on for this app.
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

    /// Nautilus traps SIGTERM too, and stops the actor first: close every
    /// publication now, so subscribers turn to the next pod of this feed
    /// within seconds rather than after the node's shutdown or this
    /// client's timeout.
    fn on_stop(&mut self) -> anyhow::Result<()> {
        if let Some(live) = &mut self.live {
            live.rt.finish(&mut live.housekeeping)?;
        }
        Ok(())
    }

    fn on_time_event(&mut self, _event: &TimeEvent) -> anyhow::Result<()> {
        self.invoke(false);
        Ok(())
    }

    fn on_instrument(&mut self, i: &InstrumentAny) -> anyhow::Result<()> {
        self.invoke(true);
        if let Some(persist) = self.persist("instrument") {
            instrument_row(i, |row| persist.record_row("instrument", row));
        }
        self.spec(i)
    }

    fn on_instrument_status(&mut self, s: &InstrumentStatus) -> anyhow::Result<()> {
        self.invoke(true);
        if let Some(persist) = self.persist("instrument_status") {
            instrument_status_row(s, |row| persist.record_row("instrument_status", row));
        }
        Ok(())
    }

    fn on_trade(&mut self, t: &TradeTick) -> anyhow::Result<()> {
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        live.t.trades.inc();
        let latency = Telemetry::since_venue(t.ts_event.as_u64(), live.rt.ctx_ref().wall_now());
        live.t.trade_latency.record(latency);
        let ticker = self.tickers.entry(t.instrument_id).or_default();
        ticker.last = Some(t.price.as_f64());
        ticker.trades += 1;
        ticker.volume += t.size.as_f64();
        let (symbol, venue) = names(&t.instrument_id);
        let trade_id = t.trade_id.as_str().as_bytes();
        let len =
            TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
        let started = live.rt.ctx_ref().read();
        live.rt.ctx_ref().send(
            live.feeds.md,
            TradeEncoder::TEMPLATE_ID,
            len,
            |buf| -> anyhow::Result<_> {
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
            },
        )?;
        let took = live.rt.ctx_ref().read().since(started);
        live.t.record_ns.record(took.max(0) as u64);
        Ok(())
    }

    fn on_quote(&mut self, q: &QuoteTick) -> anyhow::Result<()> {
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        live.t.quotes.inc();
        let latency = Telemetry::since_venue(q.ts_event.as_u64(), live.rt.ctx_ref().wall_now());
        live.t.quote_latency.record(latency);
        let (symbol, venue) = names(&q.instrument_id);
        let len = QuoteEncoder::compute_length_with_header(symbol.len(), venue.len());
        live.rt.ctx_ref().send(
            live.feeds.tob,
            QuoteEncoder::TEMPLATE_ID,
            len,
            |buf| -> anyhow::Result<_> {
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
            },
        )?;
        // The hot path keeps a gauge. The `spread` table is an event row on
        // the once-a-second book, below.
        let (bid, ask) = (q.bid_price.as_f64(), q.ask_price.as_f64());
        if let Some(gauge) = live.t.spread_bps.get(&q.instrument_id) {
            gauge.set((ask - bid) / (ask + bid) * 2e4);
        }
        Ok(())
    }

    fn on_book(&mut self, book: &OrderBook) -> anyhow::Result<()> {
        self.invoke(true);
        self.spread(book);
        self.ticker(book);
        let Some(live) = &self.live else {
            return Ok(());
        };
        live.t.books.inc();
        if let Some(persist) = live.rt.ctx_ref().persist() {
            book_view(persist, book);
        }
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
        live.rt.ctx_ref().send(
            live.feeds.md,
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
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        live.t.deltas.inc();
        // A checkpoint trace from the venue's timestamp: `wire` (the venue
        // and the network), `convert` (rows built), `record` (published).
        let tracer = &live.t.book_update;
        // From the venue's time, placed by its wall-clock age: the `wire`
        // stage is true whatever this process's clock has drifted.
        let now = live.rt.ctx_ref().read();
        let mut trace = tracer.start(
            live.rt.ctx_ref().from_remote(d.ts_event.as_u64() as i64),
            tracer.next_id(),
        );
        trace.mark(now);
        trace.attr(0, d.deltas.len() as i64);
        let (symbol, venue) = names(&d.instrument_id);
        for (i, chunk) in d.deltas.chunks(live.feeds.deltas_per_row).enumerate() {
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
                trace.mark(live.rt.ctx_ref().read());
            }
            let deltas = &self.deltas;
            let len = BookDeltasEncoder::compute_length_with_header(
                deltas.len(),
                symbol.len(),
                venue.len(),
            );
            live.rt.ctx_ref().send(
                live.feeds.md,
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
        trace.mark(live.rt.ctx_ref().read());
        trace.finish();
        Ok(())
    }

    fn on_bar(&mut self, b: &Bar) -> anyhow::Result<()> {
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        let id = b.bar_type.instrument_id();
        let (symbol, venue) = names(&id);
        let spec = text::<96>(b.bar_type);
        let len = BarEncoder::compute_length_with_header(symbol.len(), venue.len(), spec.len());
        live.rt
            .ctx_ref()
            .send(live.feeds.md, BarEncoder::TEMPLATE_ID, len, |buf| {
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
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        self.tickers.entry(m.instrument_id).or_default().mark = Some(m.value.as_f64());
        let (symbol, venue) = names(&m.instrument_id);
        let len = MarkPriceEncoder::compute_length_with_header(symbol.len(), venue.len());
        live.rt
            .ctx_ref()
            .send(live.feeds.md, MarkPriceEncoder::TEMPLATE_ID, len, |buf| {
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
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        self.tickers.entry(x.instrument_id).or_default().index = Some(x.value.as_f64());
        let (symbol, venue) = names(&x.instrument_id);
        let len = IndexPriceEncoder::compute_length_with_header(symbol.len(), venue.len());
        live.rt
            .ctx_ref()
            .send(live.feeds.md, IndexPriceEncoder::TEMPLATE_ID, len, |buf| {
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
        self.invoke(true);
        let Some(live) = &self.live else {
            return Ok(());
        };
        self.tickers.entry(f.instrument_id).or_default().funding = f.rate.to_f64();
        let (symbol, venue) = names(&f.instrument_id);
        let len = FundingRateEncoder::compute_length_with_header(symbol.len(), venue.len());
        live.rt
            .ctx_ref()
            .send(live.feeds.md, FundingRateEncoder::TEMPLATE_ID, len, |buf| {
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
        self.invoke(true);
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
        let Some(persist) = self.persist(&table) else {
            return Ok(());
        };
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
        persist.record_row(
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
        let Some(persist) = self.persist(TABLE) else {
            return;
        };
        let (id, price, size) = (
            text::<64>(t.instrument_id),
            text::<48>(t.price),
            text::<48>(t.size),
        );
        let side = text::<16>(t.aggressor_side);
        persist.record_row(
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
        let Some(persist) = self.persist(TABLE) else {
            return;
        };
        let (id, open_interest) = (text::<64>(oi.instrument_id), text::<48>(oi.open_interest));
        persist.record_row(
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
        let Some(persist) = self.persist(TABLE) else {
            return;
        };
        persist.record_row(
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
        let Some(live) = &self.live else {
            return Ok(());
        };
        live.rt.ctx_ref().send(
            live.feeds.md,
            InstrumentSpecEncoder::TEMPLATE_ID,
            len,
            |buf| {
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
            },
        )
    }

    /// The runtime's persist, when it records `table` now.
    fn persist(&self, table: &str) -> Option<&Persist> {
        self.live
            .as_ref()?
            .rt
            .ctx_ref()
            .persist()
            .filter(|p| p.event_enabled(table))
    }

    /// One `spread` row a second: a row does not belong on the quote
    /// callback.
    fn spread(&self, book: &OrderBook) {
        let (Some(bid), Some(ask)) = (book.best_bid_price(), book.best_ask_price()) else {
            return;
        };
        let (bid, ask) = (bid.as_f64(), ask.as_f64());
        let mid = ask + bid;
        if mid == 0.0 {
            return;
        }
        if let Some(persist) = self.persist("spread") {
            spread_row(book.instrument_id, (ask - bid) / mid * 2e4, |row| {
                persist.record_row("spread", row);
            });
        }
    }

    /// One `ticker` row: the instrument's last second. Its columns are
    /// whatever this venue has, so a venue with more data adds columns to the
    /// table the moment it is deployed.
    fn ticker(&mut self, book: &OrderBook) {
        let Some(persist) = self
            .live
            .as_ref()
            .and_then(|live| live.rt.ctx_ref().persist())
            .filter(|p| p.event_enabled("ticker"))
        else {
            return;
        };
        let id = book.instrument_id;
        let volatility = id
            .symbol
            .as_str()
            .split('-')
            .next()
            .and_then(|base| self.volatility_of(base));
        let quote = (
            book.best_bid_price().map(|p| p.as_f64()),
            book.best_ask_price().map(|p| p.as_f64()),
        );
        let t = self.tickers.entry(id).or_default();
        ticker_row(id, quote, t, volatility, |row| {
            persist.record_row("ticker", row);
        });
    }
}

/// What a row function hands its sink: the row's fields in its columns'
/// order, absent values left out.
///
/// Each row keeps the fields, kinds and order of the `tracing` event it
/// replaces, because shape ids hash them: `%` fields were Display text and
/// `?` fields Debug text.
type Fields<'r, 'a> = &'r mut dyn Iterator<Item = (&'a str, Value<'a>)>;

/// `instrument`: an instrument's definition, its decimals as text so a tick
/// size like 0.00001 is kept exactly.
fn instrument_row(i: &InstrumentAny, row: impl FnOnce(Fields<'_, '_>)) {
    let id = i.id();
    let (instrument, venue) = (text::<64>(id), text::<16>(id.venue));
    let raw_symbol = text::<64>(i.raw_symbol());
    let class = text::<32>(format_args!("{:?}", i.instrument_class()));
    let base = i.base_currency().map(|c| text::<16>(c.code));
    let quote = text::<16>(i.quote_currency().code);
    let settlement = text::<16>(i.settlement_currency().code);
    let price_increment = text::<48>(i.price_increment());
    let size_increment = text::<48>(i.size_increment());
    let multiplier = text::<48>(i.multiplier());
    let min_quantity = i.min_quantity().map(text::<48>);
    let max_quantity = i.max_quantity().map(text::<48>);
    let (maker_fee, taker_fee) = (text::<48>(i.maker_fee()), text::<48>(i.taker_fee()));
    row(&mut [
        Some(("instrument", Value::Str(&instrument))),
        Some(("venue", Value::Str(&venue))),
        Some(("raw_symbol", Value::Str(&raw_symbol))),
        Some(("class", Value::Str(&class))),
        base.as_deref().map(|b| ("base_currency", Value::Str(b))),
        Some(("quote_currency", Value::Str(&quote))),
        Some(("settlement_currency", Value::Str(&settlement))),
        Some(("inverse", Value::Bool(i.is_inverse()))),
        Some(("price_increment", Value::Str(&price_increment))),
        Some(("size_increment", Value::Str(&size_increment))),
        Some(("multiplier", Value::Str(&multiplier))),
        min_quantity
            .as_deref()
            .map(|q| ("min_quantity", Value::Str(q))),
        max_quantity
            .as_deref()
            .map(|q| ("max_quantity", Value::Str(q))),
        Some(("maker_fee", Value::Str(&maker_fee))),
        Some(("taker_fee", Value::Str(&taker_fee))),
    ]
    .into_iter()
    .flatten());
}

/// `instrument_status`: a change in an instrument's trading state.
fn instrument_status_row(s: &InstrumentStatus, row: impl FnOnce(Fields<'_, '_>)) {
    let instrument = text::<64>(s.instrument_id);
    let venue = text::<16>(s.instrument_id.venue);
    let action = text::<32>(format_args!("{:?}", s.action));
    row(&mut [
        Some(("instrument", Value::Str(&instrument))),
        Some(("venue", Value::Str(&venue))),
        Some(("ts_event", Value::U64(s.ts_event.as_u64()))),
        Some(("action", Value::Str(&action))),
        s.reason.map(|r| ("reason", Value::Str(r.as_str()))),
        s.trading_event
            .map(|e| ("trading_event", Value::Str(e.as_str()))),
        s.is_trading.map(|v| ("is_trading", Value::Bool(v))),
        s.is_quoting.map(|v| ("is_quoting", Value::Bool(v))),
        s.is_short_sell_restricted
            .map(|v| ("is_short_sell_restricted", Value::Bool(v))),
    ]
    .into_iter()
    .flatten());
}

/// `spread`: the best bid and ask's spread, in basis points of the mid.
fn spread_row(id: InstrumentId, bps: f64, row: impl FnOnce(Fields<'_, '_>)) {
    let instrument = text::<64>(id);
    row(&mut [
        ("instrument", Value::Str(&instrument)),
        ("bps", Value::F64(bps)),
    ]
    .into_iter());
}

/// `ticker`: the instrument's last second, from the book's best bid and ask
/// and `t`, whose trade count and volume start again from 0.
fn ticker_row(
    id: InstrumentId,
    (bid, ask): (Option<f64>, Option<f64>),
    t: &mut Ticker,
    volatility: Option<f64>,
    row: impl FnOnce(Fields<'_, '_>),
) {
    let (instrument, venue) = (text::<64>(id), text::<16>(id.venue));
    let (trades, volume) = (std::mem::take(&mut t.trades), std::mem::take(&mut t.volume));
    row(&mut [
        Some(("instrument", Value::Str(&instrument))),
        Some(("venue", Value::Str(&venue))),
        bid.map(|v| ("bid", Value::F64(v))),
        ask.map(|v| ("ask", Value::F64(v))),
        t.last.map(|v| ("last", Value::F64(v))),
        Some(("trades", Value::U64(trades))),
        Some(("volume", Value::F64(volume))),
        t.mark.map(|v| ("mark_price", Value::F64(v))),
        t.index.map(|v| ("index_price", Value::F64(v))),
        t.funding.map(|v| ("funding_rate", Value::F64(v))),
        t.open_interest.map(|v| ("open_interest", Value::F64(v))),
        volatility.map(|v| ("volatility_index", Value::F64(v))),
    ]
    .into_iter()
    .flatten());
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
fn book_view(persist: &Persist, book: &OrderBook) {
    if !persist.event_enabled("book_view") {
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
    persist.record_value(
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

#[tokio::main(flavor = "current_thread")]
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

    // Checked before any venue connects; the actor's `on_start` builds the
    // runtime from them.
    let service = std::env::var("SERVICE").unwrap_or_else(|_| format!("md-{}", venue.name));
    lab::check_node_network()?;
    let streams = lab::Streams::load(lab::streams_path())?;
    node.add_actor(Recorder {
        core: DataActorCore::new(DataActorConfig::default()),
        venue,
        instruments,
        tickers: HashMap::new(),
        volatility: Vec::new(),
        venue_upper: venue.name.to_uppercase(),
        deltas: Vec::with_capacity(MAX_DELTAS_PER_ROW),
        start: Some((service, streams)),
        live: None,
    })?;
    node.run().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ergon_runtime::event::Kind;

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

    /// Each field a row function hands its sink: name, kind, and its text
    /// (a value that is not text, debug-printed).
    fn columns(fields: Fields<'_, '_>) -> Vec<(String, Kind, String)> {
        fields
            .map(|(name, value)| {
                let text = match value {
                    Value::Str(s) => s.to_owned(),
                    other => format!("{other:?}"),
                };
                (name.to_owned(), value.kind(), text)
            })
            .collect()
    }

    /// The names and kinds of `columns`.
    fn kinds(columns: &[(String, Kind, String)]) -> Vec<(&str, Kind)> {
        columns.iter().map(|(n, k, _)| (n.as_str(), *k)).collect()
    }

    /// The text of field `name`.
    fn text_of<'c>(columns: &'c [(String, Kind, String)], name: &str) -> Option<&'c str> {
        columns
            .iter()
            .find(|(n, ..)| n == name)
            .map(|(.., t)| t.as_str())
    }

    // The expected lists are the fields of the `tracing` events these rows
    // replace, in their order: their shape ids hash them.

    #[test]
    fn an_instrument_row_keeps_the_events_columns() -> anyhow::Result<()> {
        use nautilus_model::identifiers::Symbol;
        use nautilus_model::instruments::CurrencyPair;
        use nautilus_model::types::{Currency, Price, Quantity};

        let pair = CurrencyPair::builder()
            .instrument_id(InstrumentId::from("BTCUSDT.BINANCE"))
            .raw_symbol(Symbol::from("BTCUSDT"))
            .base_currency(Currency::from("BTC"))
            .quote_currency(Currency::from("USDT"))
            .price_precision(2)
            .size_precision(6)
            .price_increment(Price::from("0.01"))
            .size_increment(Quantity::from("0.000001"))
            .max_quantity(Quantity::from("9000"))
            .min_quantity(Quantity::from("0.000001"))
            .maker_fee(Decimal::new(1, 3))
            .taker_fee(Decimal::new(1, 3))
            .ts_event(nautilus_core::UnixNanos::default())
            .ts_init(nautilus_core::UnixNanos::default())
            .build()?;
        let mut got = Vec::new();
        instrument_row(&InstrumentAny::CurrencyPair(pair), |row| {
            got = columns(row);
        });
        assert_eq!(
            kinds(&got),
            [
                ("instrument", Kind::Str),
                ("venue", Kind::Str),
                ("raw_symbol", Kind::Str),
                ("class", Kind::Str),
                ("base_currency", Kind::Str),
                ("quote_currency", Kind::Str),
                ("settlement_currency", Kind::Str),
                ("inverse", Kind::Bool),
                ("price_increment", Kind::Str),
                ("size_increment", Kind::Str),
                ("multiplier", Kind::Str),
                ("min_quantity", Kind::Str),
                ("max_quantity", Kind::Str),
                ("maker_fee", Kind::Str),
                ("taker_fee", Kind::Str),
            ]
        );
        assert_eq!(text_of(&got, "instrument"), Some("BTCUSDT.BINANCE"));
        assert_eq!(text_of(&got, "class"), Some("Spot"), "Debug, as `?` was");
        assert_eq!(text_of(&got, "price_increment"), Some("0.01"));
        Ok(())
    }

    #[test]
    fn an_instrument_status_row_keeps_the_events_columns() {
        let status = InstrumentStatus::new(
            InstrumentId::from("BTCUSDT.BINANCE"),
            nautilus_model::enums::MarketStatusAction::Trading,
            nautilus_core::UnixNanos::from(1),
            nautilus_core::UnixNanos::from(2),
            Some("open".into()),
            Some("auction".into()),
            Some(true),
            Some(true),
            Some(false),
        );
        let mut got = Vec::new();
        instrument_status_row(&status, |row| got = columns(row));
        assert_eq!(
            kinds(&got),
            [
                ("instrument", Kind::Str),
                ("venue", Kind::Str),
                ("ts_event", Kind::U64),
                ("action", Kind::Str),
                ("reason", Kind::Str),
                ("trading_event", Kind::Str),
                ("is_trading", Kind::Bool),
                ("is_quoting", Kind::Bool),
                ("is_short_sell_restricted", Kind::Bool),
            ]
        );
        assert_eq!(
            text_of(&got, "action"),
            Some("Trading"),
            "Debug, as `?` was"
        );
        assert_eq!(text_of(&got, "venue"), Some("BINANCE"));
    }

    #[test]
    fn spread_and_ticker_rows_keep_the_events_columns() {
        let id = InstrumentId::from("BTC-PERPETUAL.DERIBIT");
        let mut got = Vec::new();
        spread_row(id, 1.5, |row| got = columns(row));
        assert_eq!(kinds(&got), [("instrument", Kind::Str), ("bps", Kind::F64)]);

        let mut ticker = Ticker {
            last: Some(100.0),
            trades: 3,
            volume: 2.5,
            mark: Some(100.5),
            index: Some(100.25),
            funding: Some(0.0001),
            open_interest: Some(7.0),
        };
        ticker_row(
            id,
            (Some(99.0), Some(101.0)),
            &mut ticker,
            Some(55.0),
            |row| {
                got = columns(row);
            },
        );
        assert_eq!(
            kinds(&got),
            [
                ("instrument", Kind::Str),
                ("venue", Kind::Str),
                ("bid", Kind::F64),
                ("ask", Kind::F64),
                ("last", Kind::F64),
                ("trades", Kind::U64),
                ("volume", Kind::F64),
                ("mark_price", Kind::F64),
                ("index_price", Kind::F64),
                ("funding_rate", Kind::F64),
                ("open_interest", Kind::F64),
                ("volatility_index", Kind::F64),
            ]
        );
        assert_eq!(
            (ticker.trades, ticker.volume.to_bits()),
            (0, 0f64.to_bits()),
            "the second's count and volume start again"
        );
        // A value the venue does not have is left out, not written as 0.
        ticker_row(
            id,
            (None, Some(101.0)),
            &mut Ticker::default(),
            None,
            |row| {
                got = columns(row);
            },
        );
        assert_eq!(
            kinds(&got),
            [
                ("instrument", Kind::Str),
                ("venue", Kind::Str),
                ("ask", Kind::F64),
                ("trades", Kind::U64),
                ("volume", Kind::F64),
            ]
        );
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
