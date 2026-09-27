//! A regional trading engine: one thread, one loop, the HFT model.
//!
//! It subscribes to every feed handler in its region (`md-*` in
//! config/streams.yaml, `REGION`), by name, so it follows them when they
//! move. Their `md` streams, and its exchange's fills, come through
//! persistent subscriptions (`Persistent`): a slow engine catches up from the
//! archive, and a restarted feed handler's new session is replayed from its
//! first message, so nothing is lost. Top of book (`tob`) is best effort.
//! From the `md` streams it keeps each instrument's L2 book (rebuilt
//! from every snapshot, and from scratch when a publisher restarts), and
//! per asset the aggregated best bid and ask in base quantity, the mid's
//! EMAs and a strategy. Its orders go to the region's dummy exchange
//! (`exch-sim`), whose fills come back on `exec`. Once a second it publishes
//! each asset's EMAs and aggregated book on `signals`; the node's archive
//! records them, so they become ClickHouse tables with no more code.
//!
//! Tick-to-trade is the checkpoint trace `tick_to_trade`, from the feed
//! handler's receive time through `feed`, `decode`, `book`, `signal`,
//! `decide` and `send`. Every tick counts in its stage histograms; one that
//! led to an order is kept, under the order's id, which the exchange's
//! `order_ack` trace shares.

use engine::{App, Book, Change, Emas, Spec, Strategy, aggregate};
use market::market::{
    BookAction, BookDeltasDecoder, BookSnapshotDecoder, InstrumentSpecDecoder, QuoteDecoder,
    Side as MdSide,
};
use market::trading::{
    AggBookAsksEntry, AggBookBidsEntry, AggBookEncoder, AggBookFixedFields, Decimal9, EmaEncoder,
    EmaFixedFields, ExecutionReportDecoder, NewOrderEncoder, NewOrderFixedFields, OrderStatus,
    Side,
};
use persist_client::clock::{Clock, Nanos};
use std::collections::HashMap;

use persist_client::feed::{Feed, Subscriber};
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
    let app = App::start(market::TRADING_SCHEMA)?;
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
    let mut subs = Vec::new();
    let mut venues = Vec::new();
    for (name, _) in s
        .services
        .iter()
        .filter(|(name, service)| name.starts_with("md-") && service.region == app.region)
    {
        let tob: Subscriber = app
            .persist
            .subscriber(&s.subscription(name, "tob", ip)?, s.stream(name, "tob")?);
        let md: Persistent = app.persist.persistent(s, name, "md", ip)?;
        subs.push((md, tob));
        let label = name.trim_start_matches("md-").to_uppercase();
        let l = [("venue", label.as_str())];
        venues.push(Venue {
            instruments: Vec::new(),
            sessions: 0,
            updated: Nanos(0),
            resyncs: metrics.counter("feed_resyncs", &l),
            age: metrics.gauge("book_age_ns", &l),
            live: metrics.gauge("feed_live", &l),
            tob: metrics.counter("tob_quotes", &l),
            tob_latency: metrics.histogram("tob_latency_ns", &l),
            label: venue_label(&label),
            name: label,
        });
    }
    log::info!(
        "{service}: {} venues ({}), publishing on {publication}",
        venues.len(),
        venues
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
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
        open: HashMap::new(),
        metrics: metrics.clone(),
    };
    loop {
        if app.stopping() {
            return Ok(());
        }
        let mut work = 0;
        for (i, (md, tob)) in subs.iter_mut().enumerate() {
            work += md.poll(|m, new| core.on_md(&t2t, i, m, new), LIMIT);
            work += tob.poll(|m, _| core.on_tob(i, m), LIMIT);
        }
        work += exec.poll(|m, _| core.on_exec(m), LIMIT);
        let now = core.clock.now();
        if core.every_second(now) {
            for (venue, (md, _)) in core.venues.iter().zip(&subs) {
                venue.live.set(f64::from(u8::from(md.is_live())));
            }
        }
        metrics.poll(now);
        app.idle.idle(work);
    }
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
    /// `name` as `agg_book`'s fixed 12 chars.
    label: [u8; 12],
    instruments: Vec<Instrument>,
    /// Publisher sessions seen: every one after the first is a restart.
    sessions: u64,
    updated: Nanos,
    resyncs: Counter,
    age: Gauge,
    /// 1 on the live stream, 0 replaying (catching up) or finding it.
    live: Gauge,
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
    /// Orders sent and not yet answered, by id: when sent. A fill applies
    /// once, to an order here; a replayed or repeated one is ignored.
    open: HashMap<u64, i64>,
    metrics: persist_client::metrics::Metrics,
}

/// A message's SBE header: `(template id, schema id)`.
fn header(m: &[u8]) -> Option<(u16, u16)> {
    let h = m.get(..8)?;
    Some((
        u16::from_le_bytes([h[2], h[3]]),
        u16::from_le_bytes([h[4], h[5]]),
    ))
}

fn venue_label(name: &str) -> [u8; 12] {
    let mut label = [0; 12];
    for (l, b) in label.iter_mut().zip(name.bytes()) {
        *l = b;
    }
    label
}

impl Core {
    /// One message of venue `v`'s `md` stream. Never panics: a panic here
    /// would abort the process inside Aeron's callback.
    fn on_md(&mut self, t2t: &Tracer, v: usize, m: &[u8], new_session: bool) {
        let received = self.clock.now();
        if new_session {
            self.new_session(v);
        }
        let Some((template, schema)) = header(m) else {
            return;
        };
        if schema != InstrumentSpecDecoder::SCHEMA_ID {
            return; // persist's `Source` messages
        }
        self.venues[v].updated = received;
        match template {
            BookDeltasDecoder::TEMPLATE_ID => self.on_deltas(t2t, v, m, received),
            BookSnapshotDecoder::TEMPLATE_ID => self.on_snapshot(t2t, v, m, received),
            InstrumentSpecDecoder::TEMPLATE_ID => self.on_spec(v, m),
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

    fn on_spec(&mut self, v: usize, m: &[u8]) {
        let Ok(d) = InstrumentSpecDecoder::decode(m, 0) else {
            return;
        };
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

    fn on_snapshot(&mut self, t2t: &Tracer, v: usize, m: &[u8], received: Nanos) {
        let Ok(d) = BookSnapshotDecoder::decode(m, 0) else {
            return;
        };
        let mut trace = t2t.start(Nanos::from_epoch(d.ts_init() as i64), t2t.next_id());
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

    fn on_deltas(&mut self, t2t: &Tracer, v: usize, m: &[u8], received: Nanos) {
        let Ok(d) = BookDeltasDecoder::decode(m, 0) else {
            return;
        };
        let mut trace = t2t.start(Nanos::from_epoch(d.ts_init() as i64), t2t.next_id());
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
                    Ok::<_, market::trading::sbe_rt::EncodeError>(
                        NewOrderEncoder::wrap_and_apply_header(buf, 0)
                            .fixed(&NewOrderFixedFields {
                                ts: order_id,
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
        if header(m).map(|(_, s)| s) != Some(QuoteDecoder::SCHEMA_ID) {
            return;
        }
        if let Ok(q) = QuoteDecoder::decode(m, 0) {
            let venue = &self.venues[v];
            venue.tob.inc();
            let age = self
                .clock
                .now()
                .since(Nanos::from_epoch(q.ts_init() as i64));
            venue.tob_latency.record(age.max(0) as u64);
        }
    }

    fn on_exec(&mut self, m: &[u8]) {
        if header(m).map(|(_, s)| s) != Some(ExecutionReportDecoder::SCHEMA_ID) {
            return;
        }
        let Ok(r) = ExecutionReportDecoder::decode(m, 0) else {
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
                Ok::<_, market::trading::sbe_rt::EncodeError>(
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
            let label = |venue: &str| {
                self.venues
                    .iter()
                    .find(|v| v.name == venue)
                    .map_or([0; 12], |v| v.label)
            };
            let bids = aggregate(books(), true, AGG_LEVELS);
            let asks = aggregate(books(), false, AGG_LEVELS);
            let len =
                AggBookEncoder::compute_length_with_header(bids.len(), asks.len(), name.len());
            let d9 = |x: f64| Decimal9::new((x * engine::SCALE).round() as i64);
            let _ = self
                .signals
                .record(AggBookEncoder::TEMPLATE_ID, len, |buf| {
                    Ok::<_, market::trading::sbe_rt::EncodeError>(
                        AggBookEncoder::wrap_and_apply_header(buf, 0)
                            .fixed(&AggBookFixedFields { ts })
                            .bids(bids.len() as u16, |g| {
                                for &(price, size, venue) in &bids {
                                    g.add_struct(&AggBookBidsEntry {
                                        price: Decimal9::new(price),
                                        size: d9(size),
                                        venue: label(venue),
                                    })?;
                                }
                                Ok(())
                            })?
                            .asks(asks.len() as u16, |g| {
                                for &(price, size, venue) in &asks {
                                    g.add_struct(&AggBookAsksEntry {
                                        price: Decimal9::new(price),
                                        size: d9(size),
                                        venue: label(venue),
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
