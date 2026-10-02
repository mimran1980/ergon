//! One thread, one loop, for one region.
//!
//! `md` streams and the exchange's fills come through `Persistent`, so a slow
//! or restarted engine catches up from the archive. `tob` is best effort.
//! The loop keeps each instrument's L2 book, and per asset an aggregated book,
//! EMAs, and a strategy. Orders go to `exch-sim`. Fills come back on `exec`.
//! Once a second it publishes `ema` and `agg_book` on `signals`.
//!
//! `tick_to_trade` is a checkpoint trace: `feed`, `decode`, `book`, `signal`,
//! `decide`, `send`. Every tick updates the stage histograms. A tick that
//! sends an order is kept under that order's id. `exch-sim` uses the same id.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use engine::{App, Book, Change, Emas, Spec, Strategy, aggregate};
use persist_client::clock::{Clock, Nanos};
use schema::market::{
    AnyMessage, BookAction, BookDeltasDecoder, BookSnapshotDecoder, InstrumentSpecDecoder,
    Side as MdSide,
};
use schema::trading::{
    AggBookEncoder, AggBookFixedFields, AnyMessage as TradingMessage, Decimal9, EmaEncoder,
    EmaFixedFields, NewOrderEncoder, NewOrderFixedFields, OrderStatus, Side,
};
use std::collections::HashMap;

use persist_client::feed::{Delivery, Feed, Subscriber};
use persist_client::metrics::{Counter, Gauge, Histogram};
use persist_client::persistent::Persistent;
use persist_client::trace::{Trace, TraceId, Tracer};

/// Messages taken from one subscription per loop.
const LIMIT: usize = 64;
/// Levels a side in each `agg_book` row.
const AGG_LEVELS: usize = 10;
const SECOND: i64 = 1_000_000_000;
const ORDERS: u64 = TraceId::namespace("order");
/// An order with no answer this long is given up (its exchange ignores
/// orders older than 10 s).
const ORDER_TIMEOUT_NS: i64 = 30 * SECOND;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::start(schema::TRADING_SCHEMA)?;
    let service = format!("engine-{}", app.region);
    let (s, ip) = (&app.streams, app.host_ip.as_str());
    let publication = s.publication(&service, ip)?;
    let signals = app
        .persist
        .feed(&publication, s.stream(&service, "signals")?)?;
    let orders = app
        .persist
        .feed(&publication, s.stream(&service, "orders")?)?;
    let exchange = format!("exch-sim-{}", app.region);
    let mut exec = app.persist.persistent(s, &exchange, "exec", ip)?;
    let metrics = app.persist.metrics();
    let (mut subs, mut venues) = (Vec::new(), Vec::new());
    add_venues(&app, s, &metrics, &mut subs, &mut venues)?;
    log::info!(
        "{service}: {} venues ({}), publishing on {publication}",
        venues.len(),
        venues
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    // A feed handler added to the registry in this region is subscribed to
    // within a second or two, with no restart.
    let watch = persist_client::streams::Watch::spawn(&app.streams_path)?;
    // Held apart from `core`: a trace in flight borrows it.
    let t2t = app.persist.tracer(
        "tick_to_trade",
        &["feed", "decode", "book", "signal", "decide", "send"],
        &[],
    );
    let mut core = Core {
        venues,
        assets: Vec::new(),
        signals,
        orders,
        clock: Clock::new(),
        next_second: 0,
        sent: metrics.counter("orders", &[]),
        fills: metrics.counter("fills", &[]),
        expired: metrics.counter("orders_expired", &[]),
        open_gauge: metrics.gauge("orders_open", &[]),
        drift: metrics.gauge("clock_drift_ns", &[]),
        live: false,
        open: HashMap::new(),
        metrics: metrics.clone(),
    };
    loop {
        if app.stopping() {
            return Ok(());
        }
        let mut work = 0;
        for (i, (md, tob)) in subs.iter_mut().enumerate() {
            work += md.poll(|m, delivery| core.on_md(&t2t, i, m, delivery), LIMIT);
            work += tob.poll(|m, _| core.on_tob(i, m), LIMIT);
        }
        work += exec.poll(|m, _| core.on_exec(m), LIMIT);
        let now = core.clock.now();
        if core.every_second(now) {
            for (venue, (md, _)) in core.venues.iter().zip(&subs) {
                venue.live.set(f64::from(u8::from(md.is_live())));
            }
            if let Some(streams) = watch.changed()
                && let Err(e) = add_venues(&app, &streams, &metrics, &mut subs, &mut core.venues)
            {
                log::error!("streams.yaml: {e}");
            }
        }
        metrics.poll(now);
        app.idle.idle(work);
    }
}

/// Subscribe to every feed handler of `streams` in this region not yet
/// subscribed to. Venues are only ever added: one taken out of the registry
/// just goes quiet.
fn add_venues(
    app: &App,
    streams: &persist_client::streams::Streams,
    metrics: &persist_client::metrics::Metrics,
    subs: &mut Vec<(Persistent, Subscriber)>,
    venues: &mut Vec<Venue>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ip = app.host_ip.as_str();
    for (name, _) in streams
        .services
        .iter()
        .filter(|(name, service)| name.starts_with("md-") && service.region == app.region)
    {
        let label = name.trim_start_matches("md-").to_uppercase();
        if venues.iter().any(|v| v.name == label) {
            continue;
        }
        let tob = app.persist.subscriber(
            &streams.subscription(name, "tob", ip)?,
            streams.stream(name, "tob")?,
        );
        let md = app.persist.persistent(streams, name, "md", ip)?;
        subs.push((md, tob));
        let l = [("venue", label.as_str())];
        log::info!("{name}: subscribing");
        venues.push(Venue {
            instruments: Vec::new(),
            sessions: 0,
            updated: Nanos(0),
            resyncs: metrics.counter("feed_resyncs", &l),
            age: metrics.gauge("book_age_ns", &l),
            live: metrics.gauge("feed_live", &l),
            replayed: metrics.counter("feed_replayed", &l),
            tob: metrics.counter("tob_quotes", &l),
            tob_latency: metrics.histogram("tob_latency_ns", &l),
            name: label,
        });
    }
    Ok(())
}

struct Instrument {
    symbol: Vec<u8>,
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
    resyncs: Counter,
    age: Gauge,
    /// 1 on the live stream, 0 replaying (catching up) or finding it.
    live: Gauge,
    /// Messages caught up from the archive, not live.
    replayed: Counter,
    tob: Counter,
    tob_latency: Histogram,
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
    signals: Feed,
    orders: Feed,
    clock: Clock,
    next_second: i64,
    sent: Counter,
    fills: Counter,
    expired: Counter,
    open_gauge: Gauge,
    /// The wall clock less this process's monotonic clock, ns.
    drift: Gauge,
    /// The message being handled is live, not replayed from the archive.
    live: bool,
    /// Orders sent and not yet answered, by id: when sent. A fill applies
    /// once, to an order here; a replayed or repeated one is ignored.
    open: HashMap<u64, i64>,
    metrics: persist_client::metrics::Metrics,
}

impl Core {
    /// One message of venue `v`'s `md` stream. Never panics: a panic here
    /// would abort the process inside Aeron's callback.
    fn on_md(&mut self, t2t: &Tracer, v: usize, m: &[u8], delivery: Delivery) {
        let received = self.clock.now();
        self.live = delivery.is_live();
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
            AnyMessage::BookDeltas(d) => self.on_deltas(t2t, v, d, received),
            AnyMessage::BookSnapshot(d) => self.on_snapshot(t2t, v, d, received),
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
            multiplier: d.multiplier_value().mantissa() as f64 / engine::SCALE,
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

    fn on_snapshot(&mut self, t2t: &Tracer, v: usize, d: BookSnapshotDecoder<'_>, received: Nanos) {
        // md's receive time, by its wall clock: the first stage is the
        // message's true age whatever this process's clock has drifted.
        let at = self.clock.from_remote(d.ts_init() as i64, received);
        let mut trace = t2t.start(at, t2t.next_id());
        trace.mark(received);
        let (Ok(bids), Ok(asks), Ok(symbol)) = (d.bids(), d.asks(), d.symbol()) else {
            return;
        };
        let i = self.instrument(v, symbol);
        trace.mark(self.clock.now());
        self.venues[v].instruments[i].book.snapshot(
            bids.map(|l| (l.price_value().mantissa(), l.size_value().mantissa())),
            asks.map(|l| (l.price_value().mantissa(), l.size_value().mantissa())),
        );
        self.tick(v, i, d.ts_init(), trace);
    }

    fn on_deltas(&mut self, t2t: &Tracer, v: usize, d: BookDeltasDecoder<'_>, received: Nanos) {
        // md's receive time, by its wall clock: the first stage is the
        // message's true age whatever this process's clock has drifted.
        let at = self.clock.from_remote(d.ts_init() as i64, received);
        let mut trace = t2t.start(at, t2t.next_id());
        trace.mark(received);
        let (Ok(deltas), Ok(symbol)) = (d.deltas(), d.symbol()) else {
            return;
        };
        let i = self.instrument(v, symbol);
        trace.mark(self.clock.now());
        let book = &mut self.venues[v].instruments[i].book;
        if !book.synced {
            return; // until its first snapshot
        }
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
        self.tick(v, i, d.ts_init(), trace);
    }

    /// Instrument `i` of venue `v` changed: its asset's aggregate, EMAs and
    /// strategy, and an order if it says so.
    fn tick(&mut self, v: usize, i: usize, tick_ts: u64, mut trace: Trace<'_>) {
        let Some(a) = self.venues[v].instruments[i].asset else {
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
            if instrument.asset == Some(a) {
                bid = bid.max(instrument.book.best_bid());
                ask = match (ask, instrument.book.best_ask()) {
                    (Some(x), Some(y)) => Some(i64::min(x, y)),
                    (x, y) => x.or(y),
                };
            }
        }
        let now = self.clock.now();
        trace.mark(now);
        let (Some(bid), Some(ask)) = (bid, ask) else {
            return;
        };
        let asset = &mut self.assets[a];
        asset.mid = (bid + ask) as f64 / 2.0 / engine::SCALE;
        asset.emas.update(now.epoch_ns(), asset.mid);
        trace.mark(self.clock.now());
        let decision = asset
            .strategy
            .decide(now.epoch_ns(), asset.mid, &asset.emas);
        trace.mark(self.clock.now());
        if let Some(buy) = decision {
            let order_id = self.clock.now().epoch_ns() as u64; // ponytail: unique while one engine sends per ns
            let asset = asset.name.as_bytes();
            let len = NewOrderEncoder::compute_length_with_header(asset.len());
            let sent = self
                .orders
                .record(NewOrderEncoder::TEMPLATE_ID, len, |buf| {
                    Ok::<_, schema::trading::sbe_rt::EncodeError>(
                        NewOrderEncoder::wrap_and_apply_header(buf, 0)
                            .fixed(&NewOrderFixedFields {
                                // By the wall clock: the exchange compares it with its own.
                                ts: self.clock.wall().epoch_ns() as u64,
                                tick_ts,
                                order_id,
                                side: if buy { Side::Buy } else { Side::Sell },
                                // Marketable: the aggregated best on the other side.
                                price: Decimal9::new(if buy { ask } else { bid }),
                                qty: Decimal9::new((Strategy::QTY * engine::SCALE) as i64),
                            })
                            .asset(asset)?
                            .encoded_length_with_header(),
                    )
                });
            trace.mark(self.clock.now());
            if sent.is_ok() {
                self.sent.inc();
                self.open.insert(order_id, order_id as i64);
                trace.set_id(TraceId::new(ORDERS, order_id));
                trace.keep();
            }
        }
        trace.finish();
    }

    /// Best effort top of book: counted, and how old it arrives.
    fn on_tob(&mut self, v: usize, m: &[u8]) {
        let Ok(AnyMessage::Quote(q)) = AnyMessage::decode(m, 0) else {
            return;
        };
        let venue = &self.venues[v];
        venue.tob.inc();
        let age = self
            .clock
            .wall()
            .since(Nanos::from_epoch(q.ts_init() as i64));
        venue.tob_latency.record(age.max(0) as u64);
    }

    fn on_exec(&mut self, m: &[u8]) {
        let Ok(TradingMessage::ExecutionReport(r)) = TradingMessage::decode(m, 0) else {
            return;
        };
        match r.status() {
            OrderStatus::Filled => {}
            OrderStatus::Rejected => {
                self.open.remove(&r.order_id());
                return;
            }
            _ => return,
        }
        // Once: an exchange replaying its orders after a restart may answer
        // one again.
        if self.open.remove(&r.order_id()).is_none() {
            return;
        }
        let Ok(name) = r.asset_as_str() else {
            return;
        };
        if let Some(asset) = self.assets.iter_mut().find(|a| a.name == name) {
            asset.strategy.fill(
                r.side() == Side::Buy,
                r.fill_qty_value().mantissa() as f64 / engine::SCALE,
                r.fill_price_value().mantissa() as f64 / engine::SCALE,
            );
            self.fills.inc();
        }
    }

    /// Once a second: each asset's EMAs and aggregated book on `signals`,
    /// the gauges, and orders given up. Whether it was time.
    fn every_second(&mut self, now: Nanos) -> bool {
        if now.epoch_ns() < self.next_second {
            return false;
        }
        self.next_second = (now.epoch_ns() / SECOND + 1) * SECOND;
        let before = self.open.len();
        self.open
            .retain(|_, sent| now.epoch_ns() - *sent < ORDER_TIMEOUT_NS);
        self.expired.add((before - self.open.len()) as u64);
        self.open_gauge.set(self.open.len() as f64);
        self.drift.set(self.clock.wall().since(now) as f64);
        for venue in &self.venues {
            venue.age.set(now.since(venue.updated) as f64);
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
            let _ = self.signals.record(EmaEncoder::TEMPLATE_ID, len, |buf| {
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
                        (i.asset == Some(a)).then_some((spec, &i.book, v.name.as_str()))
                    })
                })
            };
            let bids = aggregate(books(), true, AGG_LEVELS);
            let asks = aggregate(books(), false, AGG_LEVELS);
            let len =
                AggBookEncoder::compute_length_with_header(bids.len(), asks.len(), name.len());
            let d9 = |x: f64| Decimal9::new((x * engine::SCALE).round() as i64);
            let _ = self
                .signals
                .record(AggBookEncoder::TEMPLATE_ID, len, |buf| {
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
        true
    }
}
