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
use std::time::Duration;

use market::sbe_rt::EncodeError;
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
    buf: Vec<u8>,
}

nautilus_actor!(Recorder);

impl DataActor for Recorder {
    fn on_start(&mut self) -> anyhow::Result<()> {
        let second = NonZeroUsize::try_from(1000)?;
        for id in INSTRUMENTS.map(InstrumentId::from) {
            self.subscribe_trades(id, None, None);
            self.subscribe_quotes(id, None, None);
            // Bybit only streams depths 1/50/200/1000; Binance 5/10/20.
            let depth = if id.venue.as_str() == "BYBIT" { 50 } else { 20 };
            self.subscribe_book_at_interval(
                id,
                BookType::L2_MBP,
                NonZeroUsize::new(depth),
                second,
                None,
                None,
            );
            if id.venue.as_str() == "BYBIT" {
                self.subscribe_mark_prices(id, None, None);
                self.subscribe_funding_rates(id, None, None);
            }
        }
        Ok(())
    }

    fn on_trade(&mut self, t: &TradeTick) -> anyhow::Result<()> {
        self.record(TradeEncoder::TEMPLATE_ID, |buf| {
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
                .symbol(t.instrument_id.symbol.as_str().as_bytes())?
                .venue(t.instrument_id.venue.as_str().as_bytes())?
                .trade_id(t.trade_id.as_str().as_bytes())?
                .encoded_length_with_header())
        })
    }

    fn on_quote(&mut self, q: &QuoteTick) -> anyhow::Result<()> {
        self.record(QuoteEncoder::TEMPLATE_ID, |buf| {
            Ok(QuoteEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&QuoteFixedFields {
                    ts_event: q.ts_event.as_u64(),
                    ts_init: q.ts_init.as_u64(),
                    bid_price: q.bid_price.as_f64(),
                    ask_price: q.ask_price.as_f64(),
                    bid_size: q.bid_size.as_f64(),
                    ask_size: q.ask_size.as_f64(),
                })
                .symbol(q.instrument_id.symbol.as_str().as_bytes())?
                .venue(q.instrument_id.venue.as_str().as_bytes())?
                .encoded_length_with_header())
        })
    }

    fn on_book(&mut self, book: &OrderBook) -> anyhow::Result<()> {
        let now = self.core.timestamp_ns().as_u64();
        self.record(BookSnapshotEncoder::TEMPLATE_ID, |buf| {
            let bids = book.bids(Some(BOOK_LEVELS)).count() as u16;
            let asks = book.asks(Some(BOOK_LEVELS)).count() as u16;
            Ok(BookSnapshotEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&BookSnapshotFixedFields {
                    ts_event: book.ts_last.as_u64(),
                    ts_init: now,
                    sequence: book.sequence,
                })
                .bids(bids, |g| {
                    for l in book.bids(Some(BOOK_LEVELS)) {
                        g.add_struct(&BookSnapshotBidsEntry {
                            price: l.price.value.as_f64(),
                            size: l.size(),
                        })?;
                    }
                    Ok(())
                })?
                .asks(asks, |g| {
                    for l in book.asks(Some(BOOK_LEVELS)) {
                        g.add_struct(&BookSnapshotAsksEntry {
                            price: l.price.value.as_f64(),
                            size: l.size(),
                        })?;
                    }
                    Ok(())
                })?
                .symbol(book.instrument_id.symbol.as_str().as_bytes())?
                .venue(book.instrument_id.venue.as_str().as_bytes())?
                .encoded_length_with_header())
        })
    }

    fn on_mark_price(&mut self, m: &MarkPriceUpdate) -> anyhow::Result<()> {
        self.record(MarkPriceEncoder::TEMPLATE_ID, |buf| {
            Ok(MarkPriceEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&MarkPriceFixedFields {
                    ts_event: m.ts_event.as_u64(),
                    ts_init: m.ts_init.as_u64(),
                    price: m.value.as_f64(),
                })
                .symbol(m.instrument_id.symbol.as_str().as_bytes())?
                .venue(m.instrument_id.venue.as_str().as_bytes())?
                .encoded_length_with_header())
        })
    }

    fn on_funding_rate(&mut self, f: &FundingRateUpdate) -> anyhow::Result<()> {
        self.record(FundingRateEncoder::TEMPLATE_ID, |buf| {
            Ok(FundingRateEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&FundingRateFixedFields {
                    ts_event: f.ts_event.as_u64(),
                    ts_init: f.ts_init.as_u64(),
                    rate: f.rate.to_f64().unwrap_or(f64::NAN),
                    interval_minutes: f.interval,
                    next_funding_ts: f.next_funding_ns.map(|t| t.as_u64()),
                })
                .symbol(f.instrument_id.symbol.as_str().as_bytes())?
                .venue(f.instrument_id.venue.as_str().as_bytes())?
                .encoded_length_with_header())
        })
    }
}

impl Recorder {
    /// Encode into the reused buffer and queue it — only if the table is on.
    fn record(
        &mut self,
        template: u16,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, EncodeError>,
    ) -> anyhow::Result<()> {
        if self.persist.enabled(template) {
            let len = encode(&mut self.buf)?;
            self.persist.record(&self.buf[..len]);
        }
        Ok(())
    }
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
    let (persist, writer) = Persist::start(SCHEMA, Settings::from_env(), Duration::from_secs(1))?;
    node.add_actor(Recorder {
        core: DataActorCore::new(DataActorConfig::default()),
        persist,
        buf: vec![0; 64 * 1024],
    })?;
    node.run().await?;
    writer.stop();
    Ok(())
}
