//! The live exchange agent, also reusable in composite runtime applications.
use ergon_runtime::Error;
use ergon_runtime::metrics::Counter;
use ergon_runtime::rt::{Agent, Ctx, Expiry, FeedId, Out};
use ergon_runtime::subscription::Delivery;
use ergon_runtime::trace::{TraceId, Tracer};
use schema::trading::sbe_rt::EncodeError;
use schema::trading::{
    ExecutionReportEncoder, ExecutionReportFixedFields, NewOrderDecoder, OrderStatus,
};

const ORDERS: u64 = TraceId::namespace("order");
/// Orders older than this when they arrive are ignored.
const ORDER_TTL_NS: i64 = 10_000_000_000;

/// The live dummy exchange: acknowledges and fills at the submitted limit.
pub struct Exchange {
    exec: Out,
    orders: FeedId,
    ack: Tracer,
    filled: Counter,
    stale: Counter,
}

impl Exchange {
    /// Open this region's order subscription and execution publication.
    ///
    /// # Errors
    /// The configured streams or publications cannot be opened.
    pub fn new(ctx: &mut Ctx) -> Result<Self, Error> {
        let service = format!("exch-sim-{}", ctx.region());
        let engine = format!("engine-{}", ctx.region());
        let exec = ctx.publish(&service, "exec")?;
        let orders = ctx.subscribe_from_start(&engine, "orders")?;
        let metrics = ctx.metrics().clone();
        log::info!("{service}: filling {engine}'s orders");
        Ok(Self {
            exec,
            orders,
            ack: ctx.tracer("order_ack", &["wire", "match", "ack"], &[]),
            filled: metrics.counter("orders_filled", &[]),
            stale: metrics.counter("orders_stale", &[]),
        })
    }
}

impl Agent for Exchange {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), Error> {
        Ok(())
    }

    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, m: &[u8], delivery: Delivery) {
        if feed != self.orders || m.get(4..6) != Some(&NewOrderDecoder::SCHEMA_ID.to_le_bytes()[..])
        {
            return; // persist's `Source` messages
        }
        let Ok(o) = NewOrderDecoder::decode(m, 0) else {
            return;
        };
        let Ok(asset) = o.asset() else {
            return;
        };
        // The order's age by the wall clock, which stamped it.
        let now = ctx.now();
        let placed = ctx.from_remote(o.ts() as i64);
        if now.since(placed) > ORDER_TTL_NS {
            self.stale.inc();
            return;
        }
        let mut trace = self.ack.start(placed, TraceId::new(ORDERS, o.order_id()));
        trace.mark(now);
        trace.mark(ctx.read()); // matching: everything fills
        for (status, qty) in [
            (OrderStatus::New, 0),
            (OrderStatus::Filled, o.qty_value().mantissa()),
        ] {
            let len = ExecutionReportEncoder::compute_length_with_header(asset.len());
            let _ = ctx.send(self.exec, ExecutionReportEncoder::TEMPLATE_ID, len, |buf| {
                Ok::<_, EncodeError>(
                    ExecutionReportEncoder::wrap_and_apply_header(buf, 0)
                        .fixed(&ExecutionReportFixedFields {
                            ts: now.epoch_ns() as u64,
                            order_id: o.order_id(),
                            status,
                            side: o.side(),
                            fill_price: o.price_value(),
                            fill_qty: schema::trading::Decimal9::new(qty),
                        })
                        .asset(asset)?
                        .encoded_length_with_header(),
                )
            });
        }
        trace.mark(ctx.read());
        // A replayed order's wire stage is how long this exchange was down,
        // not a latency: answered, never traced.
        if delivery.is_live() {
            trace.keep();
            trace.finish();
        }
        self.filled.inc();
    }

    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}
}
