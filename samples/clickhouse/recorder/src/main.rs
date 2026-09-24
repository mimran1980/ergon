//! Records public Binance and Bybit market data into ClickHouse.
//!
//! NautilusTrader connects to the exchanges (public streams, no API keys) and
//! calls this actor; each callback encodes one SBE message from
//! `schema/market.xml` and hands it to `persist`. Which tables are recorded is
//! decided by `config/tables.yaml`, re-read while this runs.

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/market.rs"]
mod market;

use std::num::NonZeroUsize;

use market::{
    BookSnapshotAsksEntry, BookSnapshotBidsEntry, BookSnapshotEncoder, BookSnapshotFixedFields,
    FundingRateEncoder, FundingRateFixedFields, MarkPriceEncoder, MarkPriceFixedFields,
    QuoteEncoder, QuoteFixedFields, Side, TradeEncoder, TradeFixedFields,
};
use nautilus_binance::config::{BinanceDataClientConfig, BinanceSpotMarketDataMode};
use nautilus_binance::factories::BinanceDataClientFactory;
use nautilus_bybit::common::enums::BybitProductType;
use nautilus_bybit::config::BybitDataClientConfig;
use nautilus_bybit::factories::BybitDataClientFactory;
use nautilus_common::actor::{DataActor, DataActorConfig, DataActorCore};
use nautilus_common::enums::Environment;
use nautilus_common::nautilus_actor;
use nautilus_live::node::LiveNode;
use nautilus_model::data::{FundingRateUpdate, MarkPriceUpdate, QuoteTick, TradeTick};
use nautilus_model::enums::{AggressorSide, BookType};
use nautilus_model::identifiers::{InstrumentId, TraderId};
use nautilus_model::orderbook::OrderBook;
use persist::{Persist, Settings};
use rust_decimal::prelude::ToPrimitive;

const SCHEMA: &str = include_str!("../../schema/market.xml");
const INSTRUMENTS: [&str; 4] = [
    "BTCUSDT.BINANCE",
    "ETHUSDT.BINANCE",
    "BTCUSDT-LINEAR.BYBIT",
    "ETHUSDT-LINEAR.BYBIT",
];
/// Levels per side in each `book_snapshot` row.
const BOOK_LEVELS: usize = 10;

#[derive(Debug)]
struct Recorder {
    core: DataActorCore,
    persist: Persist,
}

nautilus_actor!(Recorder);

impl DataActor for Recorder {
    fn on_start(&mut self) -> anyhow::Result<()> {
        let second = NonZeroUsize::try_from(1000)?;
        for id in INSTRUMENTS.map(InstrumentId::from) {
            let bybit = id.venue.as_str() == "BYBIT";
            self.subscribe_trades(id, None, None);
            self.subscribe_quotes(id, None, None);
            // Bybit only streams depths 1/50/200/1000; Binance 5/10/20.
            let depth = if bybit { 50 } else { 20 };
            self.subscribe_book_at_interval(
                id,
                BookType::L2_MBP,
                NonZeroUsize::new(depth),
                second,
                None,
                None,
            );
            if bybit {
                self.subscribe_mark_prices(id, None, None);
                self.subscribe_funding_rates(id, None, None);
            }
        }
        Ok(())
    }

    fn on_trade(&mut self, t: &TradeTick) -> anyhow::Result<()> {
        let (symbol, venue) = names(&t.instrument_id);
        let trade_id = t.trade_id.as_str().as_bytes();
        let len =
            TradeEncoder::compute_length_with_header(symbol.len(), venue.len(), trade_id.len());
        self.persist.record(TradeEncoder::TEMPLATE_ID, len, |buf| {
            Ok(TradeEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&TradeFixedFields {
                    ts_event: t.ts_event.as_u64(),
                    ts_init: t.ts_init.as_u64(),
                    price: t.price.as_f64(),
                    size: t.size.as_f64(),
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
        self.persist.record(QuoteEncoder::TEMPLATE_ID, len, |buf| {
            Ok(QuoteEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&QuoteFixedFields {
                    ts_event: q.ts_event.as_u64(),
                    ts_init: q.ts_init.as_u64(),
                    bid_price: q.bid_price.as_f64(),
                    ask_price: q.ask_price.as_f64(),
                    bid_size: q.bid_size.as_f64(),
                    ask_size: q.ask_size.as_f64(),
                })
                .symbol(symbol)?
                .venue(venue)?
                .encoded_length_with_header())
        })
    }

    fn on_book(&mut self, book: &OrderBook) -> anyhow::Result<()> {
        // Sizing a snapshot walks the book: skip all of it when the table is off.
        if !self.persist.enabled(BookSnapshotEncoder::TEMPLATE_ID) {
            return Ok(());
        }
        let now = self.core.timestamp_ns().as_u64();
        let (symbol, venue) = names(&book.instrument_id);
        let bids = book.bids(Some(BOOK_LEVELS)).count();
        let asks = book.asks(Some(BOOK_LEVELS)).count();
        let len =
            BookSnapshotEncoder::compute_length_with_header(bids, asks, symbol.len(), venue.len());
        self.persist
            .record(BookSnapshotEncoder::TEMPLATE_ID, len, |buf| {
                Ok(BookSnapshotEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&BookSnapshotFixedFields {
                        ts_event: book.ts_last.as_u64(),
                        ts_init: now,
                        sequence: book.sequence,
                    })
                    .bids(bids as u16, |g| {
                        for l in book.bids(Some(BOOK_LEVELS)) {
                            g.add_struct(&BookSnapshotBidsEntry {
                                price: l.price.value.as_f64(),
                                size: l.size(),
                            })?;
                        }
                        Ok(())
                    })?
                    .asks(asks as u16, |g| {
                        for l in book.asks(Some(BOOK_LEVELS)) {
                            g.add_struct(&BookSnapshotAsksEntry {
                                price: l.price.value.as_f64(),
                                size: l.size(),
                            })?;
                        }
                        Ok(())
                    })?
                    .symbol(symbol)?
                    .venue(venue)?
                    .encoded_length_with_header())
            })
    }

    fn on_mark_price(&mut self, m: &MarkPriceUpdate) -> anyhow::Result<()> {
        let (symbol, venue) = names(&m.instrument_id);
        let len = MarkPriceEncoder::compute_length_with_header(symbol.len(), venue.len());
        self.persist
            .record(MarkPriceEncoder::TEMPLATE_ID, len, |buf| {
                Ok(MarkPriceEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&MarkPriceFixedFields {
                        ts_event: m.ts_event.as_u64(),
                        ts_init: m.ts_init.as_u64(),
                        price: m.value.as_f64(),
                    })
                    .symbol(symbol)?
                    .venue(venue)?
                    .encoded_length_with_header())
            })
    }

    fn on_funding_rate(&mut self, f: &FundingRateUpdate) -> anyhow::Result<()> {
        let (symbol, venue) = names(&f.instrument_id);
        let len = FundingRateEncoder::compute_length_with_header(symbol.len(), venue.len());
        self.persist
            .record(FundingRateEncoder::TEMPLATE_ID, len, |buf| {
                Ok(FundingRateEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&FundingRateFixedFields {
                        ts_event: f.ts_event.as_u64(),
                        ts_init: f.ts_init.as_u64(),
                        rate: f.rate.to_f64().unwrap_or(f64::NAN),
                        interval_minutes: f.interval,
                        next_funding_ts: f.next_funding_ns.map(|t| t.as_u64()),
                    })
                    .symbol(symbol)?
                    .venue(venue)?
                    .encoded_length_with_header())
            })
    }
}

/// An instrument's symbol and venue: the var-data every message ends with.
fn names(id: &InstrumentId) -> (&[u8], &[u8]) {
    (id.symbol.as_str().as_bytes(), id.venue.as_str().as_bytes())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let binance = BinanceDataClientConfig {
        spot_market_data_mode: BinanceSpotMarketDataMode::Json,
        ..Default::default()
    };
    let bybit = BybitDataClientConfig {
        product_types: vec![BybitProductType::Linear],
        ..Default::default()
    };
    let mut node = LiveNode::builder(TraderId::from("RECORDER-001"), Environment::Live)?
        .add_data_client(
            None,
            Box::new(BinanceDataClientFactory::new()),
            Box::new(binance),
        )?
        .add_data_client(
            None,
            Box::new(BybitDataClientFactory::new()),
            Box::new(bybit),
        )?
        .build()?;

    // After `build()`, so persist's log lines go through Nautilus' logger.
    let (persist, writer) = Persist::start(SCHEMA, Settings::from_env())?;
    node.add_actor(Recorder {
        core: DataActorCore::new(DataActorConfig::default()),
        persist,
    })?;
    node.run().await?;
    writer.stop();
    Ok(())
}
