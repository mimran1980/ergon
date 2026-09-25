//! Records public Binance and Bybit market data into ClickHouse.
//!
//! NautilusTrader connects to the exchanges (public streams, no API keys) and
//! calls this actor; each callback encodes one SBE message from
//! `schema/market.xml` straight into an Aeron publication (`persist-client`).
//! The ingester (`persist-server`) takes it from the Aeron Archive into
//! ClickHouse. Which tables are recorded is decided by `config/tables.yaml`,
//! re-read while this runs.

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
#[path = "generated/market.rs"]
mod market;

use std::num::NonZeroUsize;

use arrayvec::ArrayVec;

use market::{
    BookSnapshotAsksEntry, BookSnapshotBidsEntry, BookSnapshotEncoder, BookSnapshotFixedFields,
    Decimal9, FundingRateEncoder, FundingRateFixedFields, MarkPriceEncoder, MarkPriceFixedFields,
    QuoteEncoder, QuoteFixedFields, Side, TradeEncoder, TradeFixedFields, TryToSbe,
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
use nautilus_model::orderbook::{BookLevel, OrderBook};
use persist_client::{Persist, Settings};
use rust_decimal::Decimal;
use tracing_subscriber::layer::SubscriberExt;

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
        })
    }

    fn on_quote(&mut self, q: &QuoteTick) -> anyhow::Result<()> {
        let (symbol, venue) = names(&q.instrument_id);
        let len = QuoteEncoder::compute_length_with_header(symbol.len(), venue.len());
        self.persist
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
        // A derived signal needs no schema: one event, and `spread` in
        // tables.yaml, make a table whose columns are these fields.
        let (bid, ask) = (q.bid_price.as_f64(), q.ask_price.as_f64());
        tracing::info!(table = "spread", instrument = %q.instrument_id, bps = (ask - bid) / (ask + bid) * 2e4);
        Ok(())
    }

    fn on_book(&mut self, book: &OrderBook) -> anyhow::Result<()> {
        // Reading a snapshot walks the book: skip all of it when the table is off.
        if !self.persist.enabled(BookSnapshotEncoder::TEMPLATE_ID) {
            return Ok(());
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
        self.persist
            .record(BookSnapshotEncoder::TEMPLATE_ID, len, |buf| {
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
                        price: d9(m.value.as_decimal())?,
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
                        rate: d9(f.rate)?,
                        interval_minutes: f.interval,
                        next_funding_ts: f.next_funding_ns.map(|t| t.as_u64()),
                    })
                    .symbol(symbol)?
                    .venue(venue)?
                    .encoded_length_with_header())
            })
    }
}

/// `d` exactly, as the schema's `Decimal9` (mantissa x 10^-9, stored as
/// ClickHouse `Decimal(18, 9)`), by the generated conversion. More than nine
/// decimals, or more than +-9.2 billion, is an error, never a rounded value.
fn d9(d: Decimal) -> anyhow::Result<Decimal9> {
    d.try_to_sbe().map_err(anyhow::Error::msg)
}

/// The best [`BOOK_LEVELS`] levels of one side as `(price, size)`, on the stack.
fn levels<'a>(
    side: impl Iterator<Item = &'a BookLevel>,
) -> anyhow::Result<ArrayVec<(Decimal9, Decimal9), BOOK_LEVELS>> {
    side.take(BOOK_LEVELS)
        .map(|l| Ok((d9(l.price.value.as_decimal())?, d9(l.size_decimal())?)))
        .collect()
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
    let persist = Persist::connect(SCHEMA, Settings::from_env())?;
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;
    node.add_actor(Recorder {
        core: DataActorCore::new(DataActorConfig::default()),
        persist,
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
        Ok(())
    }
}
