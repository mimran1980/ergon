//! One region's trading engine, an [`Agent`] on the runtime's one thread.
//!
//! `md` streams and the exchange's fills come through persistent
//! subscriptions, so a slow or restarted engine catches up from the archive.
//! `tob` is best effort. The engine keeps each instrument's L2 book, and per
//! asset an aggregated book, EMAs, and a strategy. Orders go to `exch-sim`.
//! Fills come back on `exec`. Once a second (an aligned repeating timer) it
//! publishes `ema` and `agg_book` on `signals`. Each order has a one-shot
//! expiry timer, cancelled by its fill.
//!
//! It subscribes to every feed handler, in every region: the whole market,
//! the far venues as late as the network makes them.
//!
//! `tick_to_trade` is a checkpoint trace per venue, from the venue's event:
//! `venue <VENUE>` (the venue to md), `feed <from>→<here>` (md to this
//! engine, across regions or not), `decode`, `book`, `signal` (the EMAs),
//! `decide`, `send`. Every tick updates the stage histograms. A tick that
//! sends an order is kept under that order's id. `exch-sim` uses the same id.
//! Histograms beside it: `md_to_engine_ns`, `venue_to_engine_ns` and
//! `tick_to_order_ns` per venue and its region (`from`), `order_ack_ns` and
//! `order_fill_ns` per order.
//!
//! Time comes only from the runtime (`ctx.now`, `ctx.read`, `ctx.wall_ns`),
//! so the same code runs live, in replay and in a backtest; `clippy.toml`
//! rejects any other clock and any hash-ordered collection.

use crate::{Book, Change, Emas, FeedState, GapRule, Sequenced, Spec, Strategy, aggregate};
use ergon_runtime::clock::Nanos;
use ergon_runtime::metrics::{Counter, Gauge, Histogram};
use ergon_runtime::rt::{Agent, Ctx, Expiry, FeedId, Out};
use ergon_runtime::subscription::Delivery;
use ergon_runtime::timer::TimerId;
use ergon_runtime::trace::{Trace, TraceId, Tracer};
use ergon_runtime::{DetMap, Error};
use lab::{Streams, Watch};
use schema::market::{
    AnyMessage, BookAction, BookDeltasDecoder, BookSnapshotDecoder, InstrumentSpecDecoder,
    Side as MdSide,
};
use schema::trading::{
    AggBookEncoder, AggBookFixedFields, AnyMessage as TradingMessage, Decimal9, EmaEncoder,
    EmaFixedFields, NewOrderEncoder, NewOrderFixedFields, OrderStatus, Side,
};

/// Levels a side in each `agg_book` row.
const AGG_LEVELS: usize = 10;
const SECOND: i64 = 1_000_000_000;
const ORDERS: u64 = TraceId::namespace("order");
/// An order with no answer this long is given up (its exchange ignores
/// orders older than 10 s).
const ORDER_TIMEOUT_NS: i64 = 30 * SECOND;
/// The once-a-second timer's token.
const EVERY_SECOND: u64 = 1;
/// Live: read `streams.yaml` for new feed handlers.
const WATCH: u64 = 2;
/// An order's expiry timer: this bit and the order id.
const ORDER_EXPIRY: u64 = 1 << 62;
/// An instrument's stale timer: this bit, the venue index above bit 24 and
/// the instrument index below.
const STALE: u64 = 1 << 61;
/// A book with no update this long stops trading until its next one.
const STALE_AFTER_NS: i64 = 5 * SECOND;

/// What a feed is to this engine.
#[derive(Clone, Copy)]
enum Route {
    Md(usize),
    Tob(usize),
    Exec,
    Unknown,
}

/// The engine agent: build it with [`Engine::new`] on the runtime's context.
pub struct Engine {
    core: Core,
    /// A trace per venue, held apart from `Core`: a trace in flight borrows
    /// its tracer.
    tracers: Vec<Tracer>,
    /// By [`FeedId`].
    routes: Vec<Route>,
    /// The lab's registry: which feed handlers exist, and their regions.
    streams: Streams,
    /// Live: `streams.yaml`, followed so a new feed handler is subscribed to
    /// with no restart. A replay or backtest has a fixed registry.
    watch: Option<Watch>,
}

impl Engine {
    /// Publish `signals` and `orders`, subscribe to the exchange's fills and
    /// every feed handler in the registry. Timers are armed by [`Agent::start`]
    /// at the runtime's recorded startup checkpoint.
    ///
    /// # Errors
    ///
    /// The registry does not name a stream.
    pub fn new(ctx: &mut Ctx, streams: Streams) -> Result<Self, Error> {
        let service = format!("engine-{}", ctx.region());
        let signals = ctx.publish(&service, "signals")?;
        let orders = ctx.publish(&service, "orders")?;
        let exchange = format!("exch-sim-{}", ctx.region());
        let exec = ctx.subscribe(&exchange, "exec")?;
        let metrics = ctx.metrics().clone();
        let mut engine = Self {
            core: Core {
                venues: Vec::new(),
                assets: Vec::new(),
                signals,
                orders,
                sent: metrics.counter("orders", &[]),
                fills: metrics.counter("fills", &[]),
                expired: metrics.counter("orders_expired", &[]),
                open_gauge: metrics.gauge("orders_open", &[]),
                drift: metrics.gauge("clock_drift_ns", &[]),
                live: false,
                open: DetMap::default(),
                order_ack: metrics.histogram("order_ack_ns", &[]),
                order_fill: metrics.histogram("order_fill_ns", &[]),
                metrics,
            },
            tracers: Vec::new(),
            routes: Vec::new(),
            streams,
            watch: None,
        };
        engine.route(exec, Route::Exec);
        engine.add_venues(ctx)?;
        log::info!(
            "{service}: {} venues ({})",
            engine.core.venues.len(),
            engine
                .core
                .venues
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        Ok(engine)
    }

    fn route(&mut self, feed: FeedId, route: Route) {
        let i = feed.0 as usize;
        if self.routes.len() <= i {
            self.routes.resize(i + 1, Route::Unknown);
        }
        self.routes[i] = route;
    }

    /// Subscribe to every feed handler of the registry, in every region, not
    /// yet subscribed to: each engine sees the whole market, the far venues
    /// as late as the network makes them. Venues are only ever added: one
    /// taken out of the registry just goes quiet.
    fn add_venues(&mut self, ctx: &mut Ctx) -> Result<(), Error> {
        let feeds: Vec<(String, String)> = self
            .streams
            .services
            .iter()
            .filter(|(name, _)| name.starts_with("md-"))
            .map(|(name, service)| (name.clone(), service.region.clone()))
            .collect();
        for (name, region) in feeds {
            let label = name.trim_start_matches("md-").to_uppercase();
            if self.core.venues.iter().any(|v| v.name == label) {
                continue;
            }
            let v = self.core.venues.len();
            let tob = ctx.subscribe_live(&name, "tob")?;
            let md = ctx.subscribe(&name, "md")?;
            self.route(tob, Route::Tob(v));
            self.route(md, Route::Md(v));
            // A trace per venue, its stages naming the venue and the route,
            // so a slow `feed` reads as the region it crossed. Still
            // `tick_to_trade`: sampled by tables.yaml's rule of that name.
            self.tracers.push(ctx.tracer(
                "tick_to_trade",
                &[
                    &format!("venue {label}"),
                    &format!("feed {region}→{}", ctx.region()),
                    "decode",
                    "book",
                    "signal",
                    "decide",
                    "send",
                ],
                &[],
            ));
            // `from`: the feed's region, so each engine's view of every
            // region is its own series (Tokyo to London is not Tokyo to Tokyo).
            let metrics = &self.core.metrics;
            let l = [("venue", label.as_str()), ("from", region.as_str())];
            // How this venue's book `sequence` advances; most skip numbers.
            let gap = std::env::var(format!("GAP_RULE_{label}"))
                .ok()
                .and_then(|r| GapRule::parse(&r))
                .unwrap_or(GapRule::Monotonic);
            log::info!("{name}: subscribing");
            self.core.venues.push(Venue {
                instruments: Vec::new(),
                sessions: 0,
                updated: ctx.now(),
                is_live: false,
                gap,
                gaps: metrics.counter("feed_gaps", &l),
                stale_books: metrics.counter("book_stale", &l),
                resyncs: metrics.counter("feed_resyncs", &l),
                age: metrics.gauge("book_age_ns", &l),
                live: metrics.gauge("feed_live", &l),
                replayed: metrics.counter("feed_replayed", &l),
                tob: metrics.counter("tob_quotes", &l),
                tob_latency: metrics.histogram("tob_latency_ns", &l),
                md_latency: metrics.histogram("md_to_engine_ns", &l),
                venue_latency: metrics.histogram("venue_to_engine_ns", &l),
                tick_to_order: metrics.histogram("tick_to_order_ns", &l),
                name: label,
            });
        }
        Ok(())
    }
}

impl Engine {
    /// Follow `watch` (live): a feed handler added to the registry, in any
    /// region, is subscribed to within a second or two, with no restart.
    ///
    /// The registry is not an input the journal records: an exact replay
    /// (`backtest --source journal`) runs on the registry it is given
    /// (`--streams`, else `streams.yaml` as it is now), which must be the one
    /// the live run started with. It never subscribes a feed handler the live
    /// run added later, and stops with an error at that feed's first message.
    #[must_use]
    pub fn watching(mut self, watch: Watch) -> Self {
        self.watch = Some(watch);
        self
    }

    fn reread(&mut self, ctx: &mut Ctx) {
        let Some(streams) = self.watch.as_mut().and_then(Watch::changed) else {
            return;
        };
        // A new feed handler's name resolves through the new version.
        ctx.set_directory(Box::new(streams.clone()));
        self.streams = streams;
        if let Err(e) = self.add_venues(ctx) {
            log::error!("streams.yaml: {e}");
        }
    }
}

impl Agent for Engine {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), Error> {
        ctx.every_aligned(SECOND, EVERY_SECOND)
            .map_err(|e| Error::Config(format!("timer: {e}")))?;
        // Armed in every mode, so a live journal's firings of it replay
        // exactly; with no watch (a replay, a backtest) it does nothing.
        ctx.every(SECOND, WATCH)
            .map_err(|e| Error::Config(format!("timer: {e}")))?;
        Ok(())
    }

    #[inline]
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, msg: &[u8], delivery: Delivery) {
        match self.routes.get(feed.0 as usize) {
            Some(Route::Md(v)) => self.core.on_md(ctx, &self.tracers[*v], *v, msg, delivery),
            Some(Route::Tob(v)) => self.core.on_tob(ctx, *v, msg),
            Some(Route::Exec) => self.core.on_exec(ctx, msg),
            Some(Route::Unknown) | None => {}
        }
    }

    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        if timer.token == EVERY_SECOND {
            self.core.every_second(ctx);
        } else if timer.token == WATCH {
            self.reread(ctx);
        } else if timer.token & STALE != 0 {
            let token = timer.token & !STALE;
            let (v, i) = ((token >> 24) as usize, (token & 0xff_ffff) as usize);
            if let Some(venue) = self.core.venues.get_mut(v)
                && let Some(instrument) = venue.instruments.get_mut(i)
            {
                instrument.stale = None;
                if instrument.feed.tradable() {
                    instrument.feed.stale();
                    venue.stale_books.inc();
                }
            }
        } else if timer.token & ORDER_EXPIRY != 0
            && self
                .core
                .open
                .remove(&(timer.token & !ORDER_EXPIRY))
                .is_some()
        {
            self.core.expired.inc();
        }
    }
}

struct Instrument {
    symbol: Vec<u8>,
    /// Snapshot, gap and staleness state from the venue's sequence.
    feed: Sequenced,
    /// Fires when the book has had no update for `STALE_AFTER_NS`.
    stale: Option<TimerId>,
    spec: Option<Spec>,
    /// Index into `Core::assets`, once its spec arrived.
    asset: Option<usize>,
    book: Book,
}

struct Venue {
    /// BINANCE
    name: String,
    instruments: Vec<Instrument>,
    /// Publisher sessions seen: every one after the first is a restart.
    sessions: u64,
    updated: Nanos,
    /// The last `md` message was live, not caught up from the archive.
    is_live: bool,
    /// How its book `sequence` advances (`GAP_RULE_<VENUE>`).
    gap: GapRule,
    /// Books that missed a message and wait for a snapshot.
    gaps: Counter,
    /// Books that went quiet for `STALE_AFTER_NS`.
    stale_books: Counter,
    resyncs: Counter,
    age: Gauge,
    /// 1 on the live stream, 0 replaying (catching up) or finding it.
    live: Gauge,
    /// Messages caught up from the archive, not live.
    replayed: Counter,
    tob: Counter,
    tob_latency: Histogram,
    /// The feed handler's receive to this engine's, every live book message.
    md_latency: Histogram,
    /// The venue's event time to this engine's receive: the whole market data path.
    venue_latency: Histogram,
    /// The feed handler's receive to this engine's order, for ticks that send one.
    tick_to_order: Histogram,
}

struct Asset {
    name: String,
    emas: Emas,
    strategy: Strategy,
    mid: f64,
    position: Gauge,
    pnl: Gauge,
}

struct Core {
    venues: Vec<Venue>,
    assets: Vec<Asset>,
    signals: Out,
    orders: Out,
    sent: Counter,
    fills: Counter,
    expired: Counter,
    open_gauge: Gauge,
    /// The wall clock less this process's monotonic clock, ns.
    drift: Gauge,
    /// The message being handled is live, not replayed from the archive.
    live: bool,
    /// Orders sent and not yet answered, by id: when sent, and the expiry
    /// timer. A fill applies once, to an order here; a replayed or repeated
    /// one is ignored.
    open: DetMap<u64, (Nanos, TimerId)>,
    /// Order sent to the exchange's `New`, and to its `Filled`.
    order_ack: Histogram,
    order_fill: Histogram,
    metrics: ergon_runtime::metrics::Metrics,
}

impl Core {
    /// One message of venue `v`'s `md` stream. Never panics: a panic here
    /// would abort the process inside Aeron's callback.
    fn on_md(&mut self, ctx: &mut Ctx, t2t: &Tracer, v: usize, m: &[u8], delivery: Delivery) {
        let received = ctx.now();
        self.live = delivery.is_live();
        self.venues[v].is_live = self.live;
        if delivery.first {
            self.new_session(v);
        }
        // One decode. A different schema (the persist `Source` message) or a
        // short frame is not a book update.
        let Ok(msg) = AnyMessage::decode(m, 0) else {
            return;
        };
        // A replayed message is old news: it neither makes the book look
        // fresh nor counts as a tick; it is counted apart.
        if self.live {
            self.venues[v].updated = received;
        } else {
            self.venues[v].replayed.inc();
        }
        match msg {
            AnyMessage::BookDeltas(d) => self.on_deltas(ctx, t2t, v, d, received),
            AnyMessage::BookSnapshot(d) => self.on_snapshot(ctx, t2t, v, d, received),
            AnyMessage::InstrumentSpec(d) => self.on_spec(v, d),
            _ => {}
        }
    }

    #[cold]
    fn new_session(&mut self, v: usize) {
        let venue = &mut self.venues[v];
        venue.sessions += 1;
        if venue.sessions > 1 {
            // A restart or a move: its books resync from its next snapshot.
            log::info!("{}: new feed session; resyncing its books", venue.name);
            venue.resyncs.inc();
            for i in &mut venue.instruments {
                i.book.reset();
                i.feed = Sequenced::default();
            }
        }
    }

    fn instrument(&mut self, v: usize, symbol: &[u8]) -> usize {
        let instruments = &mut self.venues[v].instruments;
        instruments
            .iter()
            .position(|i| i.symbol == symbol)
            .unwrap_or_else(|| {
                instruments.push(Instrument {
                    symbol: symbol.to_vec(),
                    feed: Sequenced::default(),
                    stale: None,
                    spec: None,
                    asset: None,
                    book: Book::default(),
                });
                instruments.len() - 1
            })
    }

    fn on_spec(&mut self, v: usize, d: InstrumentSpecDecoder<'_>) {
        let (Ok(symbol), Ok(base)) = (d.symbol(), d.base_as_str()) else {
            return;
        };
        let spec = Spec {
            asset: base.to_owned(),
            multiplier: d.multiplier_value().mantissa() as f64 / crate::SCALE,
            inverse: d.inverse() != 0,
        };
        let i = self.instrument(v, symbol);
        if self.venues[v].instruments[i].spec.as_ref() == Some(&spec) {
            return;
        }
        let asset = match self.assets.iter().position(|a| a.name == spec.asset) {
            Some(a) => a,
            None => {
                let l = [("asset", spec.asset.as_str())];
                self.assets.push(Asset {
                    name: spec.asset.clone(),
                    emas: Emas::default(),
                    strategy: Strategy::default(),
                    mid: 0.0,
                    position: self.metrics.gauge("position", &l),
                    pnl: self.metrics.gauge("pnl", &l),
                });
                self.assets.len() - 1
            }
        };
        log::info!(
            "{} {}: {spec:?}",
            self.venues[v].name,
            String::from_utf8_lossy(symbol)
        );
        let instrument = &mut self.venues[v].instruments[i];
        instrument.spec = Some(spec);
        instrument.asset = Some(asset);
    }

    fn on_snapshot(
        &mut self,
        ctx: &mut Ctx,
        t2t: &Tracer,
        v: usize,
        d: BookSnapshotDecoder<'_>,
        received: Nanos,
    ) {
        // md's receive time, by its wall clock: the first stage is the
        // message's true age whatever this process's clock has drifted.
        let at = ctx.from_remote(d.ts_init() as i64);
        self.path_latency(ctx, v, at, d.ts_event(), received);
        // From the venue's event: the venue to md, then md to here.
        let mut trace = t2t.start(ctx.from_remote(d.ts_event() as i64), t2t.next_id());
        trace.mark(at);
        trace.mark(received);
        let (Ok(bids), Ok(asks), Ok(symbol)) = (d.bids(), d.asks(), d.symbol()) else {
            return;
        };
        let i = self.instrument(v, symbol);
        trace.mark(ctx.read());
        self.venues[v].instruments[i].feed.snapshot(d.sequence());
        self.touch(ctx, v, i);
        self.venues[v].instruments[i].book.snapshot(
            bids.map(|l| (l.price_value().mantissa(), l.size_value().mantissa())),
            asks.map(|l| (l.price_value().mantissa(), l.size_value().mantissa())),
        );
        self.tick(ctx, v, i, d.ts_init(), at, trace);
    }

    fn on_deltas(
        &mut self,
        ctx: &mut Ctx,
        t2t: &Tracer,
        v: usize,
        d: BookDeltasDecoder<'_>,
        received: Nanos,
    ) {
        // md's receive time, by its wall clock: the first stage is the
        // message's true age whatever this process's clock has drifted.
        let at = ctx.from_remote(d.ts_init() as i64);
        self.path_latency(ctx, v, at, d.ts_event(), received);
        // From the venue's event: the venue to md, then md to here.
        let mut trace = t2t.start(ctx.from_remote(d.ts_event() as i64), t2t.next_id());
        trace.mark(at);
        trace.mark(received);
        let (Ok(deltas), Ok(symbol)) = (d.deltas(), d.symbol()) else {
            return;
        };
        let i = self.instrument(v, symbol);
        trace.mark(ctx.read());
        let venue = &mut self.venues[v];
        let instrument = &mut venue.instruments[i];
        let was = instrument.feed.state;
        if !instrument.feed.deltas(venue.gap, d.sequence()) {
            if was != FeedState::Recovering && instrument.feed.state == FeedState::Recovering {
                // A missed message: the book is wrong until the next snapshot.
                venue.gaps.inc();
                instrument.book.reset();
            }
            return;
        }
        self.touch(ctx, v, i);
        let book = &mut self.venues[v].instruments[i].book;
        for e in deltas {
            let (bid, price) = (e.side() == MdSide::Buy, e.price_value().mantissa());
            book.apply(match e.action() {
                BookAction::Delete => Change::Delete { bid, price },
                BookAction::Clear => Change::Clear,
                _ => Change::Set {
                    bid,
                    price,
                    size: e.size_value().mantissa(),
                },
            });
        }
        self.tick(ctx, v, i, d.ts_init(), at, trace);
    }

    /// Instrument `i` of venue `v` was updated: re-arm its stale timer.
    fn touch(&mut self, ctx: &mut Ctx, v: usize, i: usize) {
        let instrument = &mut self.venues[v].instruments[i];
        if let Some(old) = instrument.stale.take() {
            ctx.cancel(old);
        }
        let token = STALE | ((v as u64) << 24) | i as u64;
        instrument.stale = ctx.after(STALE_AFTER_NS, token).ok();
    }

    /// Instrument `i` of venue `v` changed: its asset's aggregate, EMAs and
    /// strategy, and an order if it says so.
    fn tick(
        &mut self,
        ctx: &mut Ctx,
        v: usize,
        i: usize,
        tick_ts: u64,
        at: Nanos,
        mut trace: Trace<'_>,
    ) {
        let instrument = &self.venues[v].instruments[i];
        let Some(a) = instrument.asset.filter(|_| instrument.feed.tradable()) else {
            return;
        };
        // Catching up from the archive: the book is rebuilt, but a price
        // that may be a minute old moves no EMA, decides nothing, and is no
        // tick-to-trade sample.
        if !self.live {
            return;
        }
        let (mut bid, mut ask) = (None, None);
        for instrument in self.venues.iter().flat_map(|v| &v.instruments) {
            if instrument.asset == Some(a) && instrument.feed.tradable() {
                bid = bid.max(instrument.book.best_bid());
                ask = match (ask, instrument.book.best_ask()) {
                    (Some(x), Some(y)) => Some(i64::min(x, y)),
                    (x, y) => x.or(y),
                };
            }
        }
        trace.mark(ctx.read());
        let (Some(bid), Some(ask)) = (bid, ask) else {
            return;
        };
        // The event time, not a clock read: the input journal records it, so
        // an exact replay decides the same.
        let now = ctx.now();
        let asset = &mut self.assets[a];
        asset.mid = (bid + ask) as f64 / 2.0 / crate::SCALE;
        asset.emas.update(now.epoch_ns(), asset.mid);
        trace.mark(ctx.read());
        let decision = asset
            .strategy
            .decide(now.epoch_ns(), asset.mid, &asset.emas);
        trace.mark(ctx.read());
        if let Some(buy) = decision {
            let order_id = ctx.next_id();
            let asset = asset.name.as_bytes();
            let len = NewOrderEncoder::compute_length_with_header(asset.len());
            let sent = ctx.send(self.orders, NewOrderEncoder::TEMPLATE_ID, len, |buf| {
                Ok::<_, schema::trading::sbe_rt::EncodeError>(
                    NewOrderEncoder::wrap_and_apply_header(buf, 0)
                        .fixed(&NewOrderFixedFields {
                            // By the wall clock: the exchange compares it with its own.
                            ts: ctx.wall_ns().epoch_ns() as u64,
                            tick_ts,
                            order_id,
                            side: if buy { Side::Buy } else { Side::Sell },
                            // Marketable: the aggregated best on the other side.
                            price: Decimal9::new(if buy { ask } else { bid }),
                            qty: Decimal9::new((Strategy::QTY * crate::SCALE) as i64),
                        })
                        .asset(asset)?
                        .encoded_length_with_header(),
                )
            });
            let sent_at = ctx.read();
            trace.mark(sent_at);
            if sent.is_ok() {
                self.sent.inc();
                let ordered = sent_at.since(at).max(0) as u64;
                self.venues[v].tick_to_order.record(ordered);
                if let Ok(expiry) = ctx.after(ORDER_TIMEOUT_NS, ORDER_EXPIRY | order_id) {
                    self.open.insert(order_id, (sent_at, expiry));
                }
                trace.set_id(TraceId::new(ORDERS, order_id));
                trace.keep();
            }
        }
        trace.finish();
    }

    /// How old a live book message of venue `v` arrives: from the feed
    /// handler's receive (`at`) and from the venue's own event time. A
    /// message caught up from the archive is as old as the outage, not the path.
    fn path_latency(&self, ctx: &Ctx, v: usize, at: Nanos, ts_event: u64, received: Nanos) {
        if !self.live {
            return;
        }
        let venue = &self.venues[v];
        venue.md_latency.record(received.since(at).max(0) as u64);
        let event = ctx.from_remote(ts_event as i64);
        venue
            .venue_latency
            .record(received.since(event).max(0) as u64);
    }

    /// Best effort top of book: counted, and how old it arrives.
    fn on_tob(&self, ctx: &Ctx, v: usize, m: &[u8]) {
        let Ok(AnyMessage::Quote(q)) = AnyMessage::decode(m, 0) else {
            return;
        };
        let venue = &self.venues[v];
        venue.tob.inc();
        let age = ctx.now().since(ctx.from_remote(q.ts_init() as i64));
        venue.tob_latency.record(age.max(0) as u64);
    }

    fn on_exec(&mut self, ctx: &mut Ctx, m: &[u8]) {
        let Ok(TradingMessage::ExecutionReport(r)) = TradingMessage::decode(m, 0) else {
            return;
        };
        let sent = self.open.get(&r.order_id()).copied();
        let now = ctx.now();
        let age = |(sent, _): (Nanos, TimerId)| now.since(sent).max(0) as u64;
        match r.status() {
            OrderStatus::New => {
                if let Some(sent) = sent {
                    self.order_ack.record(age(sent));
                }
                return;
            }
            OrderStatus::Filled => {}
            OrderStatus::Rejected => {
                if let Some((_, expiry)) = self.open.remove(&r.order_id()) {
                    ctx.cancel(expiry);
                }
                return;
            }
            _ => return,
        }
        // Once: an exchange replaying its orders after a restart may answer
        // one again.
        let Some(open) = self.open.remove(&r.order_id()) else {
            return;
        };
        ctx.cancel(open.1);
        self.order_fill.record(age(open));
        let Ok(name) = r.asset_as_str() else {
            return;
        };
        if let Some(asset) = self.assets.iter_mut().find(|a| a.name == name) {
            asset.strategy.fill(
                r.side() == Side::Buy,
                r.fill_qty_value().mantissa() as f64 / crate::SCALE,
                r.fill_price_value().mantissa() as f64 / crate::SCALE,
            );
            self.fills.inc();
        }
    }

    /// Once a second: each asset's EMAs and aggregated book on `signals`,
    /// and the gauges.
    fn every_second(&mut self, ctx: &mut Ctx) {
        let now = ctx.now();
        self.open_gauge.set(self.open.len() as f64);
        self.drift.set(ctx.wall_ns().since(now) as f64);
        for venue in &self.venues {
            venue.age.set(now.since(venue.updated) as f64);
            venue.live.set(f64::from(u8::from(venue.is_live)));
        }
        for (a, asset) in self.assets.iter().enumerate() {
            asset.position.set(asset.strategy.position);
            asset.pnl.set(asset.strategy.pnl(asset.mid));
            if asset.mid == 0.0 {
                continue;
            }
            let ts = now.epoch_ns() as u64;
            let name = asset.name.as_bytes();
            let [ema5m, ema30m, ema1h, ema4h, ema12h, ema1d] = asset.emas.values;
            let len = EmaEncoder::compute_length_with_header(name.len());
            let _ = ctx.send(self.signals, EmaEncoder::TEMPLATE_ID, len, |buf| {
                Ok::<_, schema::trading::sbe_rt::EncodeError>(
                    EmaEncoder::wrap_and_apply_header(buf, 0)
                        .fixed(&EmaFixedFields {
                            ts,
                            mid: asset.mid,
                            ema5m,
                            ema30m,
                            ema1h,
                            ema4h,
                            ema12h,
                            ema1d,
                        })
                        .asset(name)?
                        .encoded_length_with_header(),
                )
            });
            let books = || {
                self.venues.iter().flat_map(move |v| {
                    v.instruments.iter().filter_map(move |i| {
                        let spec = i.spec.as_ref()?;
                        (i.asset == Some(a) && i.feed.tradable()).then_some((
                            spec,
                            &i.book,
                            v.name.as_str(),
                        ))
                    })
                })
            };
            let bids = aggregate::<AGG_LEVELS>(books(), true);
            let asks = aggregate::<AGG_LEVELS>(books(), false);
            let len =
                AggBookEncoder::compute_length_with_header(bids.len(), asks.len(), name.len());
            let d9 = |x: f64| Decimal9::new((x * crate::SCALE).round() as i64);
            let _ = ctx.send(self.signals, AggBookEncoder::TEMPLATE_ID, len, |buf| {
                Ok::<_, schema::trading::sbe_rt::EncodeError>(
                    AggBookEncoder::wrap_and_apply_header(buf, 0)
                        .fixed(&AggBookFixedFields { ts })
                        .bids(bids.len() as u16, |g| {
                            for &(price, size, venue) in &bids {
                                g.add_checked(|mut entry| {
                                    entry
                                        .price_wire(Decimal9::new(price))
                                        .size_wire(d9(size))
                                        .venue_str(venue)?;
                                    Ok(entry.complete())
                                })?;
                            }
                            Ok(())
                        })?
                        .asks(asks.len() as u16, |g| {
                            for &(price, size, venue) in &asks {
                                g.add_checked(|mut entry| {
                                    entry
                                        .price_wire(Decimal9::new(price))
                                        .size_wire(d9(size))
                                        .venue_str(venue)?;
                                    Ok(entry.complete())
                                })?;
                            }
                            Ok(())
                        })?
                        .asset(name)?
                        .encoded_length_with_header(),
                )
            });
        }
    }
}
