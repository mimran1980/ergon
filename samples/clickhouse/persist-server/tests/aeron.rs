//! Application -> Aeron Archive -> ingester -> ClickHouse, for real.
//!
//! Needs the test ClickHouse (`CLICKHOUSE_TEST_URL`) and an Aeron
//! `ArchivingMediaDriver` on `AERON_TEST_DIR` (default
//! `/tmp/persist-test-aeron`); `just test` starts both. Every test fails,
//! never skips, when they are missing. Each test records its own stream.

use std::error::Error;
use std::time::{Duration, Instant};

use persist_client::{Drops, Persist};
use persist_server::{ClickHouse, Ingester, Report};

#[path = "support/lab.rs"]
mod lab;
#[path = "support/v1.rs"]
mod v1;

use lab::Lab;

type TestResult = Result<(), Box<dyn Error>>;

/// 1 MiB terms, the test archive's segment length, so a test fills segments
/// and sees them purged. (The default channel's 16 MiB terms would need far
/// more records.)
const CHANNEL: &str = "aeron:ipc?term-length=1m";

fn aeron_dir() -> String {
    std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into())
}

/// Test `n`'s stream in this run, so recordings left by other runs are never
/// replayed into it. Even, because each replay uses the next stream id.
fn stream(n: i32) -> i32 {
    10_000 + (std::process::id() % 100_000) as i32 * 16 + n * 2
}

fn client(lab: &Lab, stream_id: i32) -> Result<Persist, Box<dyn Error>> {
    let settings = persist_client::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        // Several tests connect before anything records the stream, and one
        // checks that those records are dropped.
        subscriber_timeout: Duration::ZERO,
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: "test-app".into(),
        ..persist_client::Settings::new(&lab.config)
    };
    Ok(Persist::connect(v1::SCHEMA, settings)?)
}

fn ingester(lab: &Lab, ch: ClickHouse, stream_id: i32) -> Result<Ingester, Box<dyn Error>> {
    let settings = persist_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        ..persist_server::Settings::new(ch, &lab.config, lab.dir.join("checkpoint"))
    };
    Ok(Ingester::connect(&[v1::SCHEMA], settings).map_err(|e| {
        format!(
            "an ArchivingMediaDriver on {} is required (run `just test`): {e}",
            aeron_dir()
        )
    })?)
}

fn wait_until(what: &str, mut done: impl FnMut() -> Result<bool, Box<dyn Error>>) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done()? {
        if Instant::now() > deadline {
            return Err(format!("timed out waiting for {what}").into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

/// Record `n` messages, 100 a millisecond, as a live feed would: a flat-out
/// burst larger than half a term outruns the archive and is dropped.
fn record(persist: &Persist, n: usize) -> TestResult {
    for i in 0..n {
        persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
        if i % 100 == 99 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(persist.dropped(), 0);
    Ok(())
}

/// Tick until `table` holds `rows`; every report is kept.
fn ingest(
    ingester: &mut Ingester,
    lab: &Lab,
    table: &str,
    rows: usize,
) -> Result<Vec<Report>, Box<dyn Error>> {
    let mut reports = Vec::new();
    wait_until(&format!("{rows} rows in {table}"), || {
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        reports.push(report);
        let count = lab
            .query(&format!("SELECT count() FROM DB.{table}"))
            .unwrap_or_default();
        Ok(count == rows.to_string())
    })?;
    Ok(reports)
}

fn purged(reports: &[Report]) -> Vec<&str> {
    reports
        .iter()
        .flat_map(|r| r.purged.iter().map(String::as_str))
        .collect()
}

#[test]
fn recorded_messages_reach_clickhouse_and_the_archive_is_purged() -> TestResult {
    let lab = Lab::new("aeron_e2e", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(0);

    // ClickHouse is down: the archive keeps everything and nothing is purged.
    let down = ClickHouse::new("http://127.0.0.1:9", "lab", "lab", &lab.ch.database);
    let mut ingester_a = ingester(&lab, down, stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    record(&persist, 10_000)?;
    for _ in 0..3 {
        let report = ingester_a.tick()?;
        assert!(!report.errors.is_empty());
        assert!(
            report.inserted.is_empty() && report.purged.is_empty(),
            "{report:?}"
        );
    }
    assert!(!lab.dir.join("checkpoint").exists());
    drop(ingester_a);

    // ClickHouse is back: every record arrives, and the segments that hold
    // only inserted records are deleted.
    let mut ingester_b = ingester(&lab, lab.ch.clone(), stream_id)?;
    let reports = ingest(&mut ingester_b, &lab, "shapes", 10_000)?;
    let deleted = purged(&reports);
    assert!(
        deleted
            .iter()
            .any(|p| p.contains("deleted the segments before position")),
        "{deleted:?}"
    );
    drop(ingester_b);

    // The ingester restarts while the application keeps recording: it resumes
    // at its checkpoint, so nothing is lost or written twice.
    record(&persist, 10_000)?;
    let mut ingester_c = ingester(&lab, lab.ch.clone(), stream_id)?;
    ingest(&mut ingester_c, &lab, "shapes", 20_000)?;

    // The application exits: its recording stops, and once all of it is in
    // ClickHouse it is deleted.
    drop(persist);
    let mut reports = Vec::new();
    wait_until("the stopped recording to be deleted", || {
        let report = ingester_c.tick()?;
        let done = report
            .purged
            .iter()
            .any(|p| p.contains("all of it is in ClickHouse"));
        reports.push(report);
        Ok(done)
    })?;
    assert_eq!(lab.query("SELECT count() FROM DB.shapes")?, "20000");
    // Across two ingesters and an application restart of none: every SBE
    // row is attributed, through the source id in each frame.
    assert_eq!(
        lab.query("SELECT host, pod, app, count() FROM DB.shapes GROUP BY ALL FORMAT TSV")?,
        "test-host\ttest-pod\ttest-app\t20000"
    );
    Ok(())
}

/// One call site, used twice.
fn emit_signal(edge: f64) {
    tracing::info!(table = "signal", edge);
}

#[test]
fn a_shape_aeron_could_not_take_is_sent_with_the_next_row() -> TestResult {
    use tracing_subscriber::layer::SubscriberExt;

    let lab = Lab::new(
        "aeron_shape_retry",
        "tables:\n  signal: { kind: dynamic }\n",
    )?;
    let stream_id = stream(9);
    // No ingester has the archive record this stream yet: the shape, and so
    // the row, cannot be published.
    let persist = client(&lab, stream_id)?;
    let subscriber = tracing_subscriber::registry().with(persist.layer());
    tracing::subscriber::with_default(subscriber, || emit_signal(1.0));
    assert_eq!(persist.dropped(), 1);

    // Recording now: the shape was never marked sent, so it goes first.
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    let subscriber = tracing_subscriber::registry().with(persist.layer());
    tracing::subscriber::with_default(subscriber, || emit_signal(2.0));
    ingest(&mut ingester, &lab, "signal", 1)?;
    assert_eq!(lab.query("SELECT edge FROM DB.signal")?, "2");
    Ok(())
}

#[test]
fn nested_values_become_array_columns() -> TestResult {
    #[derive(serde::Serialize)]
    struct Level {
        price: f64,
        orders: Vec<u64>,
    }
    #[derive(serde::Serialize)]
    enum Regime {
        Calm,
        Volatile { vol: f64 },
    }
    #[derive(serde::Serialize)]
    struct Book {
        symbol: &'static str,
        spread: Spread,
        bids: Vec<Level>,
        regime: Regime,
        note: Option<&'static str>,
    }
    #[derive(serde::Serialize)]
    struct Spread {
        bps: f64,
    }

    let lab = Lab::new("aeron_nested", "tables:\n  book: { kind: dynamic }\n")?;
    let stream_id = stream(10);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    persist.record_value(
        "book",
        &Book {
            symbol: "BTCUSDT",
            spread: Spread { bps: 1.5 },
            bids: vec![
                Level {
                    price: 100.5,
                    orders: vec![7, 8],
                },
                Level {
                    price: 100.0,
                    orders: vec![],
                },
            ],
            regime: Regime::Calm,
            note: None,
        },
    );
    // Another variant, and a note: the shape grows, and the table with it.
    persist.record_value(
        "book",
        &Book {
            symbol: "ETHUSDT",
            spread: Spread { bps: 2.0 },
            bids: vec![],
            regime: Regime::Volatile { vol: 0.4 },
            note: Some("wide"),
        },
    );
    ingest(&mut ingester, &lab, "book", 2)?;
    assert_eq!(persist.dropped(), 0);
    assert_eq!(
        lab.query("SELECT name, type FROM system.columns WHERE database = 'DB' AND table = 'book' ORDER BY name FORMAT TSV")?,
        [
            "app\tLowCardinality(String)",
            "bids.orders\tArray(Array(Nullable(UInt64)))",
            "bids.price\tArray(Nullable(Float64))",
            "host\tLowCardinality(String)",
            "inserted_at\tDateTime64(3, \\'UTC\\')",
            "note\tNullable(String)",
            "pod\tLowCardinality(String)",
            "regime\tNullable(String)",
            "regime.Volatile.vol\tNullable(Float64)",
            "spread.bps\tNullable(Float64)",
            "symbol\tNullable(String)",
            "ts\tDateTime64(9, \\'UTC\\')",
        ]
        .join("\n")
    );
    assert_eq!(
        lab.query("SELECT symbol, spread.bps, bids.price, bids.orders, regime, regime.Volatile.vol, note FROM DB.book ORDER BY symbol FORMAT TSV")?,
        "BTCUSDT\t1.5\t[100.5,100]\t[[7,8],[]]\tCalm\t\\N\t\\N\nETHUSDT\t2\t[]\t[]\tVolatile\t0.4\twide"
    );
    Ok(())
}

#[test]
fn lists_of_different_lengths_in_one_struct_are_inserted() -> TestResult {
    #[derive(serde::Serialize)]
    struct Stats {
        bids: Vec<f64>,
        asks: Vec<f64>,
    }
    #[derive(serde::Serialize)]
    enum Side {
        Quiet { levels: Vec<u32> },
        Busy { levels: Vec<u32>, trades: Vec<u32> },
    }
    #[derive(serde::Serialize)]
    struct Snap {
        stats: Stats,
        side: Side,
    }

    let lab = Lab::new("aeron_lists", "tables:\n  snap: { kind: dynamic }\n")?;
    let stream_id = stream(11);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    persist.record_value(
        "snap",
        &Snap {
            stats: Stats {
                bids: vec![1.0],
                asks: vec![2.0, 3.0, 4.0],
            },
            side: Side::Quiet { levels: vec![1, 2] },
        },
    );
    persist.record_value(
        "snap",
        &Snap {
            stats: Stats {
                bids: vec![],
                asks: vec![5.0],
            },
            side: Side::Busy {
                levels: vec![3],
                trades: vec![4, 5, 6],
            },
        },
    );
    // Rejected by ClickHouse, these would stay queued and fail `ingest`.
    ingest(&mut ingester, &lab, "snap", 2)?;
    assert_eq!(
        lab.query("SELECT stats_bids, stats_asks, side, side_Quiet_levels, side_Busy_trades FROM DB.snap ORDER BY length(stats_asks) DESC FORMAT TSV")?,
        "[1]\t[2,3,4]\tQuiet\t[1,2]\t[]\n[]\t[5]\tBusy\t[]\t[4,5,6]"
    );
    Ok(())
}

#[test]
fn rows_built_at_run_time_become_tables() -> TestResult {
    use persist_client::event::Value;

    let lab = Lab::new(
        "aeron_rows",
        "tables:\n  venue_stats: { kind: dynamic }\n  off: { kind: dynamic, enabled: false }\n",
    )?;
    let stream_id = stream(6);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    persist.record_row(
        "venue_stats",
        [("venue", Value::Str("DERIBIT")), ("dvol", Value::F64(41.5))],
    );
    // Another venue brings a column the first one did not have.
    persist.record_row(
        "venue_stats",
        [
            ("venue", Value::Str("HYPERLIQUID")),
            ("open_interest", Value::U64(7)),
        ],
    );
    persist.record_row("off", [("x", Value::I64(1))]);
    ingest(&mut ingester, &lab, "venue_stats", 2)?;
    assert_eq!(
        lab.query("SELECT venue, dvol, open_interest FROM DB.venue_stats ORDER BY ts FORMAT TSV")?,
        "DERIBIT\t41.5\t\\N\nHYPERLIQUID\t\\N\t7"
    );
    assert_eq!(lab.query("EXISTS TABLE DB.off")?, "0");
    Ok(())
}

#[test]
fn tracing_events_become_tables() -> TestResult {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::{Layer, filter::LevelFilter};

    let lab = Lab::new(
        "aeron_events",
        "tables:\n  signal: { kind: dynamic }\n  fixed: { kind: static }\n  quiet: { kind: dynamic, enabled: false }\n",
    )?;
    let stream_id = stream(4);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    // Beside a log layer that shows only warnings: its filter is its own, so
    // persist still sees every event.
    let subscriber = tracing_subscriber::registry()
        .with(persist.layer())
        .with(tracing_subscriber::fmt::layer().with_filter(LevelFilter::WARN));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(table = "signal", instrument = "BTCUSDT", edge = 0.25, n = 3);
        // A new field is a new column; a missing one is NULL.
        tracing::info!(table = "signal", instrument = %"ETHUSDT", edge = 0.5, flag = true);
        tracing::info!(table = "fixed", a = 1);
        tracing::info!(table = "quiet", x = 1); // disabled
        tracing::info!(no_table = 1); // not a record
    });
    ingest(&mut ingester, &lab, "signal", 2)?;
    assert_eq!(
        lab.query("SELECT instrument, edge, n, flag FROM DB.signal ORDER BY ts FORMAT TSV")?,
        "BTCUSDT\t0.25\t3\t\\N\nETHUSDT\t0.5\t\\N\ttrue"
    );
    assert_eq!(
        lab.query("SELECT name, type FROM system.columns WHERE database = 'DB' AND table = 'signal' ORDER BY position FORMAT TSV")?,
        "ts\tDateTime64(9, \\'UTC\\')\ninstrument\tNullable(String)\nedge\tNullable(Float64)\nn\tNullable(Int64)\nflag\tNullable(Bool)\nhost\tLowCardinality(String)\npod\tLowCardinality(String)\napp\tLowCardinality(String)\ninserted_at\tDateTime64(3, \\'UTC\\')"
    );
    assert_eq!(
        lab.query("SELECT DISTINCT host, pod, app FROM DB.signal FORMAT TSV")?,
        "test-host\ttest-pod\ttest-app",
        "every row names who recorded it"
    );
    assert_eq!(lab.query("SELECT count() FROM DB.fixed")?, "1");
    assert_eq!(lab.query("EXISTS TABLE DB.quiet")?, "0");

    // A static event table is never altered: a new field is reported, with the fix.
    tracing::subscriber::with_default(tracing_subscriber::registry().with(persist.layer()), || {
        tracing::info!(table = "fixed", a = 2, b = 3);
    });
    let mut problems = Vec::new();
    wait_until("a row with a new field in a static table", || {
        let report = ingester.tick()?;
        problems.extend(report.problems);
        Ok(lab.query("SELECT count() FROM DB.fixed")? == "2")
    })?;
    assert_eq!(
        problems,
        [
            "fixed: static table is missing column b; not writing it. Fix: ALTER TABLE `persist_test_aeron_events`.`fixed` ADD COLUMN IF NOT EXISTS `b` Nullable(Int64)"
        ]
    );

    // A value that cannot be written as its column's type is reported, not
    // silently zeroed.
    tracing::subscriber::with_default(tracing_subscriber::registry().with(persist.layer()), || {
        tracing::info!(table = "signal", instrument = "SOLUSDT", edge = "wide");
    });
    let mut errors = Vec::new();
    wait_until("the row with a mistyped value", || {
        errors.extend(ingester.tick()?.errors);
        Ok(lab.query("SELECT count() FROM DB.signal")? == "3")
    })?;
    assert_eq!(
        errors,
        [
            "signal: 1 value(s) did not match their column's type (set by the first shape seen); wrote NULL"
        ]
    );
    Ok(())
}

#[test]
fn enabled_follows_the_config_file() -> TestResult {
    let lab = Lab::new(
        "aeron_toggle",
        "tables:\n  shapes: { kind: dynamic, enabled: false }\n",
    )?;
    let persist = client(&lab, stream(1))?;
    assert!(!persist.enabled(v1::TEMPLATE_ID));
    // A disabled table never runs the encoder.
    persist.record(v1::TEMPLATE_ID, v1::LEN, |_| {
        Err("encoded a message for a disabled table")
    })?;

    lab.write_config("tables:\n  shapes: { kind: dynamic, enabled: true }\n")?;
    wait_until("recording on", || Ok(persist.enabled(v1::TEMPLATE_ID)))?;

    // An invalid edit is rejected and the last good configuration stays.
    lab.write_config("tables:\n  shapes: { kind: sometimes }\n")?;
    std::thread::sleep(Duration::from_millis(2500));
    assert!(persist.enabled(v1::TEMPLATE_ID));

    lab.write_config("tables:\n  shapes: { kind: dynamic, enabled: false }\n")?;
    wait_until("recording off", || Ok(!persist.enabled(v1::TEMPLATE_ID)))?;
    Ok(())
}

#[test]
fn a_table_is_switched_per_app_until_a_time() -> TestResult {
    let lab = Lab::new(
        "aeron_per_app",
        "tables:\n  shapes: { kind: dynamic, enabled: false }\n",
    )?;
    let settings = persist_client::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id: stream(7),
        app: "binance".into(),
        subscriber_timeout: Duration::ZERO,
        ..persist_client::Settings::new(&lab.config)
    };
    let persist = Persist::connect(v1::SCHEMA, settings)?;
    assert!(!persist.enabled(v1::TEMPLATE_ID), "off for every app");

    // Another app's entry leaves this one as `enabled` says.
    lab.write_config(
        "tables:\n  shapes: { kind: dynamic, enabled: false, apps: { bybit: true } }\n",
    )?;
    std::thread::sleep(Duration::from_millis(2500));
    assert!(!persist.enabled(v1::TEMPLATE_ID));

    // On for this app until three seconds from now, then off by itself,
    // with no further edit.
    let until = jiff::Timestamp::now().checked_add(jiff::SignedDuration::from_secs(3))?;
    lab.write_config(&format!(
        "tables:\n  shapes: {{ kind: dynamic, enabled: false, apps: {{ binance: {{ until: {until} }} }} }}\n"
    ))?;
    wait_until("on for this app", || Ok(persist.enabled(v1::TEMPLATE_ID)))?;
    wait_until("off once its time has passed", || {
        Ok(!persist.enabled(v1::TEMPLATE_ID))
    })?;
    assert!(jiff::Timestamp::now() >= until, "switched off early");
    Ok(())
}

#[test]
fn metrics_reach_their_tables_every_interval() -> TestResult {
    use persist_client::clock::Clock;

    let lab = Lab::new("aeron_metrics", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(14);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let settings = persist_client::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        metrics_interval: Duration::from_secs(1),
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: "test-app".into(),
        ..persist_client::Settings::new(&lab.config)
    };
    let persist = Persist::connect(v1::SCHEMA, settings)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    let metrics = persist.metrics();
    let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
    let depth = metrics.gauge("depth", &[]);
    let latency = metrics.histogram("latency_ns", &[("stage", "decode")]);
    sent.add(5);
    depth.set(2.5);
    for v in 1..=1000 {
        latency.record(v * 1000);
    }

    // The application's loop: poll with the time it has, one message a call.
    let clock = Clock::new();
    wait_until("an interval of metrics in ClickHouse", || {
        metrics.poll(clock.now());
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        let rows = |sql: &str| lab.query(sql).ok().and_then(|n| n.parse::<u64>().ok());
        Ok(
            rows("SELECT count() FROM DB.metrics WHERE name = 'orders_sent'").unwrap_or(0) > 0
                && rows("SELECT count() FROM DB.metrics_histogram").unwrap_or(0) > 0,
        )
    })?;
    assert_eq!(
        lab.query("SELECT value, delta, labels['venue'], host, pod, app FROM DB.metrics WHERE name = 'orders_sent' ORDER BY ts LIMIT 1 FORMAT TSV")?,
        "5\t5\tbinance\ttest-host\ttest-pod\ttest-app"
    );
    assert_eq!(
        lab.query("SELECT kind, value, delta FROM DB.metrics WHERE name = 'depth' ORDER BY ts LIMIT 1 FORMAT TSV")?,
        "gauge\t2.5\t\\N"
    );
    assert_eq!(
        lab.query("SELECT DISTINCT labels['reason'] FROM DB.metrics WHERE name = 'persist_dropped' ORDER BY 1 FORMAT TSV")?,
        "back_pressure\tnot_connected\tother\ttoo_large".replace('\t', "\n"),
        "persist's own drops are counters too"
    );
    assert_eq!(
        lab.query(
            "SELECT count() FROM DB.metrics WHERE toUnixTimestamp64Nano(ts) % 1000000000 != 0"
        )?,
        "0",
        "intervals end on whole multiples of the interval"
    );
    assert_eq!(
        lab.query(
            "SELECT count, min, max, sum, labels['stage'] FROM DB.metrics_histogram FORMAT TSV"
        )?,
        "1000\t1000\t1000000\t500500000\tdecode"
    );
    // Within the buckets' 2^-5 of the true values, from the row's own
    // percentiles and from merging buckets.
    let near = |got: &str, want: f64| -> TestResult {
        let got: f64 = got.parse()?;
        if (got - want).abs() > want / 32.0 {
            return Err(format!("{got} is not within 3.1% of {want}").into());
        }
        Ok(())
    };
    near(
        &lab.query("SELECT p50 FROM DB.metrics_histogram")?,
        500_000.0,
    )?;
    near(
        &lab.query("SELECT p99 FROM DB.metrics_histogram")?,
        990_000.0,
    )?;
    near(
        &lab.query("SELECT quantileExactWeighted(0.99)(le, c) FROM DB.metrics_histogram ARRAY JOIN buckets.le AS le, buckets.count AS c")?,
        990_000.0,
    )?;
    Ok(())
}

#[test]
fn the_drivers_counters_are_sampled_with_their_streams_and_clients() -> TestResult {
    let lab = Lab::new("aeron_stats", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(15);
    let settings = persist_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        aeron_stats_interval: Duration::from_millis(100),
        ..persist_server::Settings::new(lab.ch.clone(), &lab.config, lab.dir.join("checkpoint"))
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    record(&persist, 10)?;
    // Samples until one has a delta: the second sample of a counter.
    wait_until("two samples of this stream's publication", || {
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        std::thread::sleep(Duration::from_millis(100));
        let n = lab
            .query(&format!("SELECT count() FROM DB.aeron_counters WHERE type = 'pub-pos' AND stream_id = {stream_id} AND delta IS NOT NULL"))
            .unwrap_or_default();
        Ok(n.parse::<u64>().unwrap_or(0) > 0)
    })?;
    // The application's publication, named by its client, from its key.
    assert_eq!(
        lab.query(&format!("SELECT DISTINCT client_name, channel, session_id IS NOT NULL, value >= 10 * 64 FROM DB.aeron_counters WHERE type = 'pub-pos' AND stream_id = {stream_id} FORMAT TSV"))?,
        "test-app\taeron:ipc?term-length=1m\t1\t1"
    );
    // The archive's recording of it, joined on the session.
    assert_eq!(
        lab.query(&format!("SELECT count() > 0 FROM DB.aeron_counters r JOIN DB.aeron_counters p ON r.session_id = p.session_id AND r.ts = p.ts WHERE r.type = 'rec-pos' AND r.recording_id IS NOT NULL AND p.type = 'pub-pos' AND p.stream_id = {stream_id}"))?,
        "1"
    );
    assert_eq!(
        lab.query("SELECT count() > 0 FROM DB.aeron_counters WHERE type = 'system' AND label LIKE 'Bytes%'")?,
        "1"
    );
    assert_eq!(
        lab.query("SELECT DISTINCT app FROM DB.aeron_counters")?,
        "ingester",
        "who sampled them"
    );
    Ok(())
}

#[test]
fn traces_become_spans_of_otel_traces() -> TestResult {
    use persist_client::clock::{Clock, Nanos};
    use persist_client::trace::TraceId;
    use tracing_subscriber::layer::SubscriberExt;

    let lab = Lab::new(
        "aeron_traces",
        "tables:
  shapes: { kind: dynamic }
  otel_traces:
    kind: static
    enabled: false
    apps: { test-app: true }
    traces:
      t2t: { sample: 2 }
      order: { sample: 0, slower_than: 1ms }
",
    )?;
    let stream_id = stream(16);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let settings = persist_client::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        metrics_interval: Duration::from_secs(1),
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: "test-app".into(),
        ..persist_client::Settings::new(&lab.config)
    };
    let persist = Persist::connect(v1::SCHEMA, settings)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    let t2t = persist.tracer("t2t", &["wire", "decode", "send"], &["levels"]);
    let order = persist.tracer("order", &["risk", "send"], &[]);
    let gateway = persist.tracer("order_gateway", &["send"], &[]);
    // Made after `connect` applied tables.yaml: on from the start.
    assert!(t2t.is_on() && order.is_on() && gateway.is_on());

    // Four ticks: 100, 200 and 300 ns stages. One in two is published.
    for _ in 0..4 {
        let mut t = t2t.start(Nanos(1_000), t2t.next_id());
        for at in [1_100, 1_300, 1_600] {
            t.mark(Nanos(at));
        }
        t.attr(0, 10);
        t.finish();
    }
    // Orders: none sampled, only the slow one published, and another
    // component's trace of the same order joins it.
    const ORDERS: u64 = TraceId::namespace("order");
    let mut fast = order.start(Nanos(0), TraceId::new(ORDERS, 41));
    fast.mark(Nanos(200));
    fast.mark(Nanos(500));
    fast.finish();
    let mut slow = order.start(Nanos(0), TraceId::new(ORDERS, 42));
    slow.mark(Nanos(100));
    slow.mark(Nanos(2_000_000));
    slow.finish();
    let mut sent = gateway.start(Nanos(2_000_000), TraceId::new(ORDERS, 42));
    sent.mark(Nanos(2_050_000));
    sent.finish();
    // `tracing` spans, a parent and a child.
    let subscriber = tracing_subscriber::registry().with(persist.layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info_span!("connect", venue = "binance", attempt = 2).in_scope(|| {
            tracing::info_span!("handshake").in_scope(|| {});
            // A library's internals: below INFO, never recorded.
            tracing::debug_span!("framed_read").in_scope(|| {});
        });
    });

    let metrics = persist.metrics();
    let clock = Clock::new();
    wait_until("the traces and their stage histograms", || {
        metrics.poll(clock.now());
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        let n = |sql: &str| {
            lab.query(sql)
                .ok()
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(0)
        };
        Ok(n("SELECT count() FROM DB.otel_traces") >= 15
            && n("SELECT count() FROM DB.metrics_histogram WHERE name = 'trace_ns'") >= 4)
    })?;
    // 2 ticks x (root + 3 stages), the slow order (root + 2), the gateway's
    // (root + 1), and 2 spans.
    assert_eq!(lab.query("SELECT count() FROM DB.otel_traces")?, "15");
    assert_eq!(
        lab.query("SELECT count() FROM DB.otel_traces WHERE SpanName = 'framed_read'")?,
        "0",
        "debug spans are not recorded"
    );
    assert_eq!(
        lab.query("SELECT count(), any(SpanAttributes['why']), any(SpanAttributes['levels']), any(Duration) FROM DB.otel_traces WHERE SpanName = 't2t' AND ParentSpanId = '' FORMAT TSV")?,
        "2\tsampled\t10\t600"
    );
    assert_eq!(
        lab.query("SELECT SpanName, Duration FROM DB.otel_traces s WHERE ParentSpanId IN (SELECT SpanId FROM DB.otel_traces WHERE SpanName = 't2t') GROUP BY SpanName, Duration ORDER BY Duration FORMAT TSV")?,
        "wire\t100\ndecode\t200\nsend\t300",
        "each stage a child of its trace"
    );
    assert_eq!(
        lab.query("SELECT SpanAttributes['why'], Duration FROM DB.otel_traces WHERE SpanName = 'order' AND ParentSpanId = '' FORMAT TSV")?,
        "slow\t2000000"
    );
    assert_eq!(
        lab.query("SELECT uniqExact(TraceId), count() FROM DB.otel_traces WHERE SpanName IN ('order', 'order_gateway') AND ParentSpanId = '' FORMAT TSV")?,
        "1\t2",
        "one order, one trace, across components"
    );
    assert_eq!(
        lab.query("SELECT c.TraceId = p.TraceId, c.ParentSpanId = p.SpanId, p.SpanAttributes['venue'], p.SpanAttributes['attempt'], p.SpanAttributes['why'] FROM DB.otel_traces c, DB.otel_traces p WHERE c.SpanName = 'handshake' AND p.SpanName = 'connect' FORMAT TSV")?,
        "1\t1\tbinance\t2\tspan"
    );
    assert_eq!(
        lab.query("SELECT DISTINCT ServiceName, ResourceAttributes['k8s.pod.name'], host, pod, app FROM DB.otel_traces FORMAT TSV")?,
        "test-app\ttest-pod\ttest-host\ttest-pod\ttest-app"
    );
    // Every tick counted, published or not.
    assert_eq!(
        lab.query("SELECT labels['stage'], sum(count) FROM DB.metrics_histogram WHERE name = 'trace_ns' AND labels['trace'] = 't2t' GROUP BY 1 ORDER BY 1 FORMAT TSV")?,
        "decode\t4\nsend\t4\ntotal\t4\nwire\t4"
    );
    Ok(())
}

/// The only test that installs a handle: it stays installed for the rest of
/// this test binary, and no other test here uses the free functions.
#[test]
fn an_installed_handle_records_from_anywhere() -> TestResult {
    let lab = Lab::new(
        "aeron_installed",
        "tables:\n  shapes: { kind: dynamic }\n  signal: { kind: dynamic }\n",
    )?;
    let stream_id = stream(8);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    assert!(persist.install());
    assert!(!persist.install(), "only the first install counts");
    drop(persist); // the installed handle lives on

    // Code with no handle in reach: a callback, a library.
    assert!(persist_client::enabled(v1::TEMPLATE_ID));
    for _ in 0..100 {
        persist_client::record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    }
    persist_client::record_row("signal", [("edge", persist_client::event::Value::F64(0.5))]);
    ingest(&mut ingester, &lab, "shapes", 100)?;
    ingest(&mut ingester, &lab, "signal", 1)?;
    Ok(())
}

#[test]
fn a_record_aeron_cannot_take_is_dropped_and_counted() -> TestResult {
    let lab = Lab::new("aeron_dropped", "tables:\n  shapes: { kind: dynamic }\n")?;
    // No ingester has asked the archive to record this stream.
    let persist = client(&lab, stream(2))?;
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    assert_eq!(
        persist.drops(),
        Drops {
            not_connected: 1,
            ..Drops::default()
        }
    );

    let _ingester = ingester(&lab, lab.ch.clone(), stream(2))?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    // Larger than the MTU: one `try_claim` cannot hold it.
    persist.record(v1::TEMPLATE_ID, 4096, |_| {
        Err("encoded an oversized record")
    })?;
    assert_eq!(
        persist.drops(),
        Drops {
            not_connected: 1,
            too_large: 1,
            ..Drops::default()
        }
    );
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    assert_eq!(persist.dropped(), 2);
    Ok(())
}

#[test]
fn connect_reports_when_nobody_is_recording() -> TestResult {
    let lab = Lab::new("aeron_wait", "tables:\n  shapes: { kind: dynamic }\n")?;
    let settings = persist_client::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id: stream(13),
        subscriber_timeout: Duration::from_millis(300),
        ..persist_client::Settings::new(&lab.config)
    };
    match Persist::connect(v1::SCHEMA, settings) {
        Err(err) => assert!(err.to_string().contains("no subscriber"), "{err}"),
        Ok(_) => return Err("connected with no subscriber".into()),
    }
    Ok(())
}

#[test]
fn two_threads_publish_on_the_one_stream() -> TestResult {
    let lab = Lab::new("aeron_thread", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(12);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    let other = persist.clone();
    let recorded = std::thread::spawn(move || -> Result<(), String> {
        other
            .record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
            .map_err(|e| e.to_string())
    });
    recorded
        .join()
        .map_err(|_| -> Box<dyn Error> { "the other thread panicked".into() })??;
    assert_eq!(persist.dropped(), 0);
    ingest(&mut ingester, &lab, "shapes", 2)?;
    Ok(())
}

#[test]
#[should_panic(expected = "encode wrote 164 bytes of template 1, but claimed 165 bytes of 1")]
fn a_mis_sized_encode_is_caught() {
    let Ok(lab) = Lab::new("aeron_size", "tables:\n  shapes: { kind: dynamic }\n") else {
        return;
    };
    let (Ok(_ingester), Ok(persist)) = (
        ingester(&lab, lab.ch.clone(), stream(3)),
        client(&lab, stream(3)),
    ) else {
        return;
    };
    if wait_until("the archive", || Ok(persist.is_connected())).is_err() {
        return;
    }
    // Claims one byte more than the message has.
    let _ = persist.record(v1::TEMPLATE_ID, v1::LEN + 1, v1::encode);
}
