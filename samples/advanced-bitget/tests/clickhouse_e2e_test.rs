//! Live ClickHouse E2E — the full pipeline with real inserts and queries.
//!
//! Publisher bytes → Aeron IPC (SHARED driver) → ForegroundPersistor with the
//! ClickHouse sink → SELECT the exact rows back from `l2book_typed`,
//! `l2book_dynamic`, and `trade`.
//!
//! Requires an already-running ClickHouse at 127.0.0.1:8123 (external
//! Docker). Run via `just test-clickhouse-live`; the recipe performs the
//! preflight. These tests FAIL (not skip) when ClickHouse is unreachable.

use rusteron_client::cformat;
use std::time::Duration;

use advanced_bitget::config::{CHANNEL, STREAM_DYNAMIC, STREAM_TYPED};
use advanced_bitget::market::{Level, NormalizedEventRef, WireDec};
use advanced_bitget::persistence::{ClickHouseRowSink, ForegroundPersistor};
use advanced_bitget::publication::{AeronPublication, ClaimPublisher};

const ENDPOINT: &str = "http://127.0.0.1:8123";

fn ch_query(sql: &str) -> String {
    let (user, password) = advanced_bitget::persistence::clickhouse_credentials();
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(ENDPOINT)
        .header("X-ClickHouse-User", user)
        .header("X-ClickHouse-Key", password)
        .body(sql.to_string())
        .send()
        .expect("ClickHouse must be running (just test-clickhouse-live)");
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    assert!(
        status.is_success(),
        "query failed [{status}]: {sql}\n{text}"
    );
    text
}

fn lvl(pm: i64, pe: i8, sm: i64, se: i8) -> Level {
    Level {
        price: WireDec::new(pm, pe),
        size: WireDec::new(sm, se),
    }
}

#[test]
#[ignore = "requires live ClickHouse — run via just test-clickhouse-live"]
fn e2e_ipc_to_clickhouse_exact_rows() -> Result<(), Box<dyn std::error::Error>> {
    // ── Clean slate ────────────────────────────────────────────────────
    for t in ["l2book_typed", "l2book_dynamic", "trade"] {
        ch_query(&format!("DROP TABLE IF EXISTS {t}"));
    }

    // ── Aeron IPC (SHARED driver) ──────────────────────────────────────
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch_with(|ctx| {
        ctx.set_threading_mode(
            rusteron_media_driver::bindings::aeron_threading_mode_t::AERON_THREADING_MODE_SHARED,
        )?;
        Ok(())
    })
    .expect("driver");
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    ctx.set_dir(&cformat!("{}", driver.dir()))
        .expect("dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
    aeron.start().expect("start");
    let ch = CHANNEL;
    let pub_typed = aeron
        .async_add_exclusive_publication(&ch, STREAM_TYPED)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let pub_dyn = aeron
        .async_add_exclusive_publication(&ch, STREAM_DYNAMIC)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let sub_typed = aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            &ch, STREAM_TYPED, None, None,
        )
        .expect("sub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let sub_dyn = aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            &ch, STREAM_DYNAMIC, None, None,
        )
        .expect("sub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");

    // ── Publish one book + one trade through the real publisher ───────
    let mut publisher = ClaimPublisher::new(AeronPublication(pub_typed), AeronPublication(pub_dyn))
        .expect("publisher");
    let bids = [lvl(500005, -1, 15, -1), lvl(500000, -1, 20, -1)];
    let asks = [lvl(500015, -1, 30, -1)];
    publisher.publish(&NormalizedEventRef::L2Book {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_000,
        receive_ts_ns: 1_700_000_000_000_000_100,
        sequence: 7,
        bids: &bids,
        asks: &asks,
    });
    publisher.publish(&NormalizedEventRef::Trade {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1_700_000_000_000_000_200,
        receive_ts_ns: 1_700_000_000_000_000_300,
        sequence: 9,
        price: WireDec::new(500005, -1),
        size: WireDec::new(25, -2),
        is_buy: true,
    });
    assert_eq!(
        publisher.counters().published,
        3,
        "book typed + book dynamic + trade"
    );

    // ── Consume through the real persistor with the ClickHouse sink ───
    let sink = ClickHouseRowSink::connect(ENDPOINT).expect("clickhouse connect + table create");
    let mut persistor = ForegroundPersistor::new(sink);
    let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");

    fn on_typed(
        p: &mut ForegroundPersistor<ClickHouseRowSink>,
        buf: &[u8],
        _h: rusteron_client::AeronHeader,
    ) {
        p.on_typed(buf).expect("typed decode+persist");
    }
    fn on_dynamic(
        p: &mut ForegroundPersistor<ClickHouseRowSink>,
        buf: &[u8],
        _h: rusteron_client::AeronHeader,
    ) {
        p.on_dynamic(buf).expect("dynamic decode+persist");
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while (persistor.counters().persisted_typed < 1 || persistor.counters().persisted_trades < 1)
        && std::time::Instant::now() < deadline
    {
        asm.poll(&sub_typed, &mut persistor, on_typed, 16)
            .expect("poll");
        asm.poll(&sub_dyn, &mut persistor, on_dynamic, 16)
            .expect("poll");
        std::thread::sleep(Duration::from_millis(1));
    }
    persistor.flush().expect("flush");
    let c = persistor.counters();
    assert_eq!(
        (c.persisted_typed, c.persisted_dynamic, c.persisted_trades),
        (1, 1, 1),
        "matched book persisted to both tables plus one trade"
    );

    // ── Query the exact rows back ──────────────────────────────────────
    for table in ["l2book_typed", "l2book_dynamic"] {
        let row = ch_query(&format!(
            "SELECT sequence, exchange_ts, symbol, \
             arrayMap(x -> toString(x), bid_prices), \
             arrayMap(x -> toString(x), bid_sizes), \
             arrayMap(x -> toString(x), ask_prices), \
             arrayMap(x -> toString(x), ask_sizes) \
             FROM {table} FORMAT TabSeparated"
        ));
        let row = row.trim();
        assert_eq!(
            row,
            "7\t1700000000000000000\tBTCUSDT\t\
             ['50000.5','50000']\t['1.5','2']\t['50001.5']\t['3']",
            "exact {table} row mismatch"
        );
    }

    let trade = ch_query(
        "SELECT trade_id, exchange_ts, symbol, toString(price), toString(size), is_buy \
         FROM trade FORMAT TabSeparated",
    );
    assert_eq!(
        trade.trim(),
        "9\t1700000000000000200\tBTCUSDT\t50000.5\t0.25\ttrue",
        "exact trade row mismatch"
    );
    Ok(())
}

#[test]
#[ignore = "requires live ClickHouse — run via just test-clickhouse-live"]
fn batched_inserts_flush_on_threshold_and_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    use advanced_bitget::persistence::{RowSink, TradeRow};

    ch_query("DROP TABLE IF EXISTS trade");
    let mut sink = ClickHouseRowSink::connect(ENDPOINT).expect("connect");

    // 300 rows: crosses the 256-row automatic batch flush once; the rest
    // drain on the explicit flush.
    for i in 0..300u64 {
        sink.insert_trade(&TradeRow {
            trade_id: i,
            exchange_ts_ns: i,
            symbol: "BTCUSDT".into(),
            price: 1_500_000_000_000_000_000,
            size: 250_000_000_000_000_000,
            is_buy: i % 2 == 0,
        })
        .expect("insert");
    }
    sink.flush().expect("flush");

    let count = ch_query("SELECT count() FROM trade FORMAT TabSeparated");
    assert_eq!(count.trim(), "300");
    let extremes = ch_query(
        "SELECT min(trade_id), max(trade_id), toString(any(price)) FROM trade FORMAT TabSeparated",
    );
    assert_eq!(extremes.trim(), "0\t299\t1.5");

    Ok(())
}
