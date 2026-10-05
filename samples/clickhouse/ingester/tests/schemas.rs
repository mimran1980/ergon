//! The lab's schemas through the ingester's writer, against the test
//! `ClickHouse` that `just test` starts (`CLICKHOUSE_TEST_URL`, default
//! `http://localhost:18123`, user and password `lab`). Each test works in its
//! own database, and fails, never skips, when the server is unreachable.

use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use ergon_runtime_server::{ClickHouse, Report, Writer};

type TestResult = Result<(), Box<dyn Error>>;

/// A private database, and a `tables.yaml` in a temp directory.
struct Lab {
    ch: ClickHouse,
    config: PathBuf,
}

impl Lab {
    fn new(test: &str, tables_yaml: &str) -> Result<Self, Box<dyn Error>> {
        let url = std::env::var("CLICKHOUSE_TEST_URL")
            .unwrap_or_else(|_| "http://localhost:18123".into());
        let db = format!("ingester_test_{test}");
        let ch = ClickHouse::new(&url, "lab", "lab", &db);
        ch.query(&format!("DROP DATABASE IF EXISTS {db}"))
            .map_err(|e| format!("ClickHouse at {url} is required (run `just test`): {e}"))?;
        let dir = std::env::temp_dir().join(format!("ingester-test-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let config = dir.join("tables.yaml");
        std::fs::write(&config, tables_yaml)?;
        Ok(Self { ch, config })
    }

    /// `sql` with `DB` naming this test's database.
    fn query(&self, sql: &str) -> Result<String, Box<dyn Error>> {
        Ok(self
            .ch
            .query(&sql.replace("DB", &self.ch.database))?
            .trim_end()
            .to_string())
    }
}

fn clean(report: &Report) -> Result<(), String> {
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(format!("unexpected errors: {:?}", report.errors))
    }
}

#[test]
fn market_schema_tables() -> TestResult {
    let tables = ergon_runtime_server::tables_from_schema(schema::MARKET_SCHEMA)?;
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "trade",
            "quote",
            "book_snapshot",
            "mark_price",
            "funding_rate",
            "index_price",
            "bar",
            "book_deltas",
            "instrument_spec"
        ]
    );
    let column = |table: &str, column: &str| {
        tables
            .iter()
            .find(|t| t.name == table)
            .and_then(|t| t.shape().columns.into_iter().find(|c| c.name == column))
            .map(|c| c.ch_type)
    };
    // Funding rates need more than nine decimals (Hyperliquid's have ten).
    assert_eq!(
        column("funding_rate", "rate").as_deref(),
        Some("Decimal(18, 18)")
    );
    // An enum inside a group: an array of its names.
    assert_eq!(
        column("book_deltas", "deltas.action").as_deref(),
        Some("Array(LowCardinality(String))")
    );
    let ch = ClickHouse::new("http://unused", "", "", "market");
    let book = tables
        .iter()
        .find(|t| t.name == "book_snapshot")
        .ok_or("book_snapshot")?;
    assert_eq!(
        ch.create_sql(&book.shape()),
        "CREATE TABLE IF NOT EXISTS `market`.`book_snapshot` (\n    `ts_event` DateTime64(9, 'UTC'),\n    `ts_init` DateTime64(9, 'UTC'),\n    `sequence` UInt64,\n    `bids.price` Array(Decimal(18, 9)),\n    `bids.size` Array(Decimal(18, 9)),\n    `asks.price` Array(Decimal(18, 9)),\n    `asks.size` Array(Decimal(18, 9)),\n    `symbol` String,\n    `venue` String,\n    inserted_at DateTime64(3, 'UTC') DEFAULT now64(3)\n)\nENGINE = MergeTree\nPARTITION BY toDate(`ts_event`)\nORDER BY (`symbol`, `venue`, `ts_event`)\nSETTINGS non_replicated_deduplication_window = 1000"
    );
    Ok(())
}

#[test]
fn trading_rows_round_trip_beside_market_rows() -> TestResult {
    use schema::trading::{
        AggBookEncoder, AggBookFixedFields, Decimal9, NewOrderEncoder, NewOrderFixedFields, Side,
    };
    let lab = Lab::new(
        "trading",
        "tables:\n  trade: { kind: static }\n  agg_book: { kind: dynamic }\n  new_order: { kind: static }\n",
    )?;
    let mut writer = Writer::new(
        &[schema::MARKET_SCHEMA, schema::TRADING_SCHEMA],
        lab.ch.clone(),
        &lab.config,
        Duration::ZERO,
    )?;
    let mut buf = [0u8; AggBookEncoder::compute_length_with_header(2, 1, 3)];
    let len = AggBookEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&AggBookFixedFields {
            ts: 1_700_000_000_000_000_000,
        })
        .bids(2, |g| {
            g.add_checked(|mut entry| {
                entry
                    .price_wire(Decimal9::new(100_000_000_000))
                    .size_wire(Decimal9::new(1_500_000_000))
                    .venue_str("BINANCE")?;
                Ok(entry.complete())
            })?;
            g.add_checked(|mut entry| {
                entry
                    .price_wire(Decimal9::new(99_000_000_000))
                    .size_wire(Decimal9::new(2_000_000_000))
                    .venue_str("HYPERLIQUID")?;
                Ok(entry.complete())
            })?;
            Ok(())
        })?
        .asks(1, |g| {
            g.add_checked(|mut entry| {
                entry
                    .price_wire(Decimal9::new(101_000_000_000))
                    .size_wire(Decimal9::new(250_000_000))
                    .venue_str("BINANCE")?;
                Ok(entry.complete())
            })?;
            Ok(())
        })?
        .asset(b"BTC")?
        .encoded_length_with_header();
    assert!(writer.push(
        schema::AnySchemaMessage::decode(&buf[..len], 0)?.as_bytes(),
        0
    ));
    let mut buf = [0u8; NewOrderEncoder::compute_length_with_header(3)];
    let len = NewOrderEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&NewOrderFixedFields {
            ts: 1_700_000_000_000_001_000,
            tick_ts: 1_700_000_000_000_000_000,
            order_id: 7,
            side: Side::Sell,
            price: Decimal9::new(100_000_000_000),
            qty: Decimal9::new(1_000_000),
        })
        .asset(b"BTC")?
        .encoded_length_with_header();
    assert!(writer.push(
        schema::AnySchemaMessage::decode(&buf[..len], 0)?.as_bytes(),
        0
    ));
    let mut trade = [0u8; schema::market::TradeEncoder::compute_length_with_header(3, 4, 1)];
    let len = schema::market::TradeEncoder::wrap_and_apply_header(&mut trade, 0)
        .fixed(&schema::market::TradeFixedFields {
            ts_event: 1_700_000_000_000_000_000,
            ts_init: 1_700_000_000_000_001_000,
            price: schema::market::Decimal9::new(100_000_000_000),
            size: schema::market::Decimal9::new(1_000_000_000),
            aggressor: schema::market::Side::Buy,
        })
        .symbol(b"BTC")?
        .venue(b"XNAS")?
        .trade_id(b"1")?
        .encoded_length_with_header();
    assert!(writer.push(
        schema::AnySchemaMessage::decode(&trade[..len], 0)?.as_bytes(),
        0
    ));
    clean(&writer.tick())?;
    assert_eq!(
        lab.query("SELECT symbol, venue, price, size FROM DB.trade FORMAT TSV")?,
        "BTC\tXNAS\t100\t1"
    );
    assert_eq!(
        lab.query("SELECT bids.price, bids.size, bids.venue, asks.venue, asset FROM DB.agg_book FORMAT TSV")?,
        "[100,99]\t[1.5,2]\t['BINANCE','HYPERLIQUID']\t['BINANCE']\tBTC"
    );
    assert_eq!(
        lab.query("SELECT order_id, side, price, qty, tick_ts FROM DB.new_order FORMAT TSV")?,
        "7\tSell\t100\t0.001\t2023-11-14 22:13:20.000000000"
    );
    Ok(())
}
