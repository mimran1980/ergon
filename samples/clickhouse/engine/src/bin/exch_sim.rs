//! A region's dummy exchange: it takes its engine's orders and fills each
//! at once, at its limit price, answering `New` then `Filled` on `exec`.
//!
//! Its orders come through a persistent subscription that starts from the
//! beginning of the engine's recording: after a restart it answers the
//! orders sent while it was down. Orders older than [`ORDER_TTL_NS`] are
//! ignored, as a real exchange never sees them: some it answered before the
//! restart, the rest its engine has given up on. An order it answered just
//! before a restart is answered again; the engine applies a fill once.
//! Its `order_ack` trace (`wire`, `match`, `ack`) shares the order's id, so
//! Grafana shows it under the engine's tick-to-trade trace of that order.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use ergon_runtime::app::App;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::trace::TraceId;
use schema::trading::sbe_rt::EncodeError;
use schema::trading::{
    ExecutionReportEncoder, ExecutionReportFixedFields, NewOrderDecoder, OrderStatus,
};

const ORDERS: u64 = TraceId::namespace("order");
/// Orders older than this when they arrive are ignored.
const ORDER_TTL_NS: i64 = 10_000_000_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::start(schema::TRADING_SCHEMA)?;
    let s = &app.streams;
    let service = format!("exch-sim-{}", app.region);
    let engine = format!("engine-{}", app.region);
    let exec = app.bus.publish(s, &service, "exec")?;
    let mut orders = app.bus.subscribe(s, &engine, "orders")?.from_start();
    let ack = app
        .persist
        .tracer("order_ack", &["wire", "match", "ack"], &[]);
    let (clock, metrics) = (Clock::new(), app.metrics());
    let filled = metrics.counter("orders_filled", &[]);
    let stale = metrics.counter("orders_stale", &[]);
    log::info!("{service}: filling {engine}'s orders");
    loop {
        if app.stopping() {
            return Ok(());
        }
        let work = orders.poll(
            |m, delivery| {
                if m.get(4..6) != Some(&NewOrderDecoder::SCHEMA_ID.to_le_bytes()[..]) {
                    return; // persist's `Source` messages
                }
                let Ok(o) = NewOrderDecoder::decode(m, 0) else {
                    return;
                };
                let Ok(asset) = o.asset() else {
                    return;
                };
                // The order's age by the wall clock, which stamped it.
                let now = clock.now();
                if clock.wall().since(Nanos::from_epoch(o.ts() as i64)) > ORDER_TTL_NS {
                    stale.inc();
                    return;
                }
                let mut trace = ack.start(
                    clock.from_remote(o.ts() as i64, now),
                    TraceId::new(ORDERS, o.order_id()),
                );
                trace.mark(now);
                trace.mark(clock.now()); // matching: everything fills
                for (status, qty) in [
                    (OrderStatus::New, 0),
                    (OrderStatus::Filled, o.qty_value().mantissa()),
                ] {
                    let len = ExecutionReportEncoder::compute_length_with_header(asset.len());
                    let _ = exec.record(ExecutionReportEncoder::TEMPLATE_ID, len, |buf| {
                        Ok::<_, EncodeError>(
                            ExecutionReportEncoder::wrap_and_apply_header(buf, 0)
                                .fixed(&ExecutionReportFixedFields {
                                    ts: clock.now().epoch_ns() as u64,
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
                trace.mark(clock.now());
                // A replayed order's wire stage is how long this exchange
                // was down, not a latency: answered, never traced.
                if delivery.is_live() {
                    trace.keep();
                    trace.finish();
                }
                filled.inc();
            },
            64,
        );
        app.persist.poll(clock.now());
        app.idle.idle(work);
    }
}
