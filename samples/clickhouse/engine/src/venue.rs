//! Historical L2 liquidity matched against the engine's immediate-or-cancel orders.
//! Each market update refreshes available liquidity; between updates executions
//! consume it. Reports contain the aggregate executed quantity and price.
use std::collections::BTreeMap;

use ergon_runtime::Error;
use ergon_runtime::rt::{Agent, Ctx, Expiry, FeedId, Out};
use ergon_runtime::subscription::Delivery;
use schema::market::{AnyMessage, BookAction, Side as MdSide};
use schema::trading::{
    Decimal9, ExecutionReportEncoder, ExecutionReportFixedFields, NewOrderDecoder, OrderStatus,
    Side,
};

use crate::matching::{MatchingBook, Order};
use crate::{Book, Change, SCALE, Spec};

#[derive(Default)]
struct Instrument {
    spec: Option<Spec>,
    book: Book,
}

/// A venue agent for a same-thread engine backtest.
pub struct Venue {
    orders: FeedId,
    exec: Out,
    md: Vec<FeedId>,
    instruments: BTreeMap<(u32, Vec<u8>), Instrument>,
    books: BTreeMap<String, MatchingBook>,
    liquidity_id: u64,
}

impl Venue {
    /// Subscribe to the historical market and this region's engine orders.
    ///
    /// # Errors
    /// A stream or publication cannot be opened.
    pub fn new(ctx: &mut Ctx, streams: &lab::Streams) -> Result<Self, Error> {
        let orders = ctx.subscribe(&format!("engine-{}", ctx.region()), "orders")?;
        let exec = ctx.publish(&format!("exch-sim-{}", ctx.region()), "exec")?;
        let names: Vec<_> = streams
            .services
            .keys()
            .filter(|s| s.starts_with("md-"))
            .cloned()
            .collect();
        let md = names
            .iter()
            .map(|s| ctx.subscribe(s, "md"))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            orders,
            exec,
            md,
            instruments: BTreeMap::new(),
            books: BTreeMap::new(),
            liquidity_id: 1_u64 << 63,
        })
    }

    fn market(&mut self, feed: FeedId, frame: &[u8], first: bool) {
        if first {
            for ((id, _), instrument) in &mut self.instruments {
                if *id == feed.0 {
                    instrument.book.reset();
                }
            }
        }
        let Ok(message) = AnyMessage::decode(frame, 0) else {
            return;
        };
        match message {
            AnyMessage::InstrumentSpec(d) => {
                let (Ok(symbol), Ok(base)) = (d.symbol(), d.base_as_str()) else {
                    return;
                };
                self.instruments
                    .entry((feed.0, symbol.to_vec()))
                    .or_default()
                    .spec = Some(Spec {
                    asset: base.into(),
                    multiplier: d.multiplier_value().mantissa() as f64 / SCALE,
                    inverse: d.inverse() != 0,
                });
            }
            AnyMessage::BookSnapshot(d) => {
                let (Ok(symbol), Ok(bids), Ok(asks)) = (d.symbol(), d.bids(), d.asks()) else {
                    return;
                };
                self.instruments
                    .entry((feed.0, symbol.to_vec()))
                    .or_default()
                    .book
                    .snapshot(
                        bids.map(|e| (e.price_value().mantissa(), e.size_value().mantissa())),
                        asks.map(|e| (e.price_value().mantissa(), e.size_value().mantissa())),
                    );
            }
            AnyMessage::BookDeltas(d) => {
                let (Ok(symbol), Ok(entries)) = (d.symbol(), d.deltas()) else {
                    return;
                };
                let book = &mut self
                    .instruments
                    .entry((feed.0, symbol.to_vec()))
                    .or_default()
                    .book;
                if !book.synced {
                    return;
                }
                for e in entries {
                    let bid = e.side() == MdSide::Buy;
                    let price = e.price_value().mantissa();
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
            }
            _ => return,
        }
        self.refresh();
    }

    fn refresh(&mut self) {
        self.books.clear();
        let mut discarded = Vec::new();
        for instrument in self.instruments.values().filter(|i| i.book.synced) {
            let Some(spec) = &instrument.spec else {
                continue;
            };
            let book = self.books.entry(spec.asset.clone()).or_default();
            for (buy, levels) in [
                (true, &instrument.book.bids),
                (false, &instrument.book.asks),
            ] {
                for (&price, &size) in levels {
                    let qty = (spec.base(size as f64 / SCALE, price as f64 / SCALE) * SCALE) as i64;
                    let id = self.liquidity_id;
                    self.liquidity_id += 1;
                    book.submit(
                        Order {
                            id,
                            buy,
                            price,
                            qty,
                        },
                        &mut discarded,
                    );
                }
            }
        }
    }

    fn order(&mut self, ctx: &Ctx, frame: &[u8]) {
        if frame.get(4..6) != Some(&NewOrderDecoder::SCHEMA_ID.to_le_bytes()[..]) {
            return;
        }
        let Ok(order) = NewOrderDecoder::decode(frame, 0) else {
            return;
        };
        let Ok(asset) = order.asset_as_str() else {
            return;
        };
        let book = self.books.entry(asset.into()).or_default();
        let mut fills = Vec::new();
        if !book.submit(
            Order {
                id: order.order_id(),
                buy: order.side() == Side::Buy,
                price: order.price_value().mantissa(),
                qty: order.qty_value().mantissa(),
            },
            &mut fills,
        ) {
            return;
        }
        book.cancel(order.order_id());
        let qty: i64 = fills.iter().map(|f| f.qty).sum();
        let price = if qty == 0 {
            order.price_value().mantissa()
        } else {
            (fills
                .iter()
                .map(|f| i128::from(f.price) * i128::from(f.qty))
                .sum::<i128>()
                / i128::from(qty)) as i64
        };
        let len = ExecutionReportEncoder::compute_length_with_header(asset.len());
        for status in [
            OrderStatus::New,
            if qty > 0 {
                OrderStatus::Filled
            } else {
                OrderStatus::Rejected
            },
        ] {
            let _ = ctx.send(self.exec, ExecutionReportEncoder::TEMPLATE_ID, len, |buf| {
                Ok::<_, schema::trading::sbe_rt::EncodeError>(
                    ExecutionReportEncoder::wrap_and_apply_header(buf, 0)
                        .fixed(&ExecutionReportFixedFields {
                            ts: ctx.now().epoch_ns() as u64,
                            order_id: order.order_id(),
                            status,
                            side: order.side(),
                            fill_price: Decimal9::new(price),
                            fill_qty: Decimal9::new(if status == OrderStatus::New {
                                0
                            } else {
                                qty
                            }),
                        })
                        .asset(asset.as_bytes())?
                        .encoded_length_with_header(),
                )
            });
        }
    }
}

impl Agent for Venue {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), Error> {
        Ok(())
    }
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, frame: &[u8], delivery: Delivery) {
        if feed == self.orders {
            self.order(ctx, frame);
        } else if self.md.contains(&feed) {
            self.market(feed, frame, delivery.first);
        }
    }
    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use ergon_runtime::rt::sim::{Sim, SimConfig};
    use lab::Streams;
    use schema::trading::{AnyMessage, NewOrderEncoder, NewOrderFixedFields};

    #[test]
    fn ioc_reports_executed_quantity_and_cancels_the_residual()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut config = SimConfig::new();
        config.region = crate::replay::REGION.into();
        let mut sim = Sim::new(config, Vec::new())?;
        let mut venue = Venue::new(sim.ctx(), &Streams::parse(crate::replay::STREAMS)?)?;
        let mut book = MatchingBook::default();
        assert!(book.submit(
            Order {
                id: 100,
                buy: false,
                price: 100,
                qty: 3
            },
            &mut Vec::new()
        ));
        venue.books.insert("BTC".into(), book);
        for id in [1, 2] {
            let mut frame = vec![0; NewOrderEncoder::compute_length_with_header(3)];
            let len = NewOrderEncoder::wrap_and_apply_header(&mut frame, 0)
                .fixed(&NewOrderFixedFields {
                    ts: 0,
                    tick_ts: 0,
                    order_id: id,
                    side: Side::Buy,
                    price: Decimal9::new(105),
                    qty: Decimal9::new(5),
                })
                .asset(b"BTC")?
                .encoded_length_with_header();
            assert_eq!(len, frame.len());
            venue.order(sim.ctx(), &frame);
        }
        let book = venue.books.get("BTC").ok_or("missing BTC book")?;
        let partial = book.account(1).ok_or("missing partial order")?;
        assert_eq!(
            (partial.filled, partial.resting, partial.cancelled),
            (3, 0, 2)
        );
        let unfilled = book.account(2).ok_or("missing unfilled order")?;
        assert_eq!(
            (unfilled.filled, unfilled.resting, unfilled.cancelled),
            (0, 0, 5)
        );
        let reports = sim
            .ctx()
            .captured()
            .records()
            .filter_map(|r| match AnyMessage::decode(r.frame, 0) {
                Ok(AnyMessage::ExecutionReport(report)) => Some((
                    report.order_id(),
                    report.status(),
                    report.fill_price_value().mantissa(),
                    report.fill_qty_value().mantissa(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(reports.contains(&(1, OrderStatus::Filled, 100, 3)));
        assert!(reports.contains(&(2, OrderStatus::Rejected, 105, 0)));
        Ok(())
    }
}
