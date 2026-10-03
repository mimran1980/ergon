//! Application -> Aeron Archive -> ingester -> `ClickHouse`, for real.
//!
//! Needs the test `ClickHouse` (`CLICKHOUSE_TEST_URL`) and an Aeron
//! `ArchivingMediaDriver` on `AERON_TEST_DIR` (default
//! `/tmp/persist-test-aeron`); `just test` starts both. Every test fails,
//! never skips, when they are missing. Each test records its own stream.

use std::error::Error;
use std::time::{Duration, Instant};

use ergon_runtime::bus::Bus;
use ergon_runtime::bus::Drops;
use ergon_runtime::persist::Persist;
use ergon_runtime_server::{ClickHouse, Ingester, Report};

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
    10_000 + (std::process::id() % 100_000).cast_signed() * 16 + n * 2
}

/// A `Persist` on its own bus: the pair every application starts with.
fn connect(settings: ergon_runtime::Settings) -> Result<Persist, Box<dyn Error>> {
    let bus = Bus::connect(&settings)?;
    Ok(Persist::connect(v1::SCHEMA, &bus, settings)?)
}

fn client(lab: &Lab, stream_id: i32) -> Result<Persist, Box<dyn Error>> {
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        // Several tests connect before anything records the stream, and one
        // checks that those records are dropped.
        subscriber_timeout: Duration::ZERO,
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: "test-app".into(),
        ..ergon_runtime::Settings::new(&lab.config)
    };
    connect(settings)
}

fn ingester(lab: &Lab, ch: ClickHouse, stream_id: i32) -> Result<Ingester, Box<dyn Error>> {
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        ..ergon_runtime_server::Settings::new(ch, &lab.config, lab.dir.join("checkpoint"))
    };
    Ok(Ingester::connect(&[v1::SCHEMA], settings).map_err(|e| {
        format!(
            "an ArchivingMediaDriver on {} is required (run `just test`): {e}",
            aeron_dir()
        )
    })?)
}

/// Is `template_id` recorded, once `persist` has applied `tables.yaml`
/// again? Polling it is what applies edits.
fn enabled(persist: &Persist) -> bool {
    persist.poll(ergon_runtime::clock::Clock::new().now());
    persist.enabled(v1::TEMPLATE_ID)
}

/// Poll `persist` for `wait`: an edit that should change nothing has had
/// its chance.
fn settle(persist: &Persist, wait: Duration) {
    let until = Instant::now() + wait;
    while Instant::now() < until {
        enabled(persist);
        std::thread::sleep(Duration::from_millis(20));
    }
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
/// burst larger than half a term outruns the archive and is dropped. On a
/// loaded machine the archive can fall behind even this, so a record
/// refused for back pressure is retried, as an application that must not
/// lose it would; any other drop fails the test.
fn record(persist: &Persist, n: usize) -> TestResult {
    for i in 0..n {
        taken(persist, || {
            persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
        if i % 100 == 99 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(dropped_but_back_pressure(persist), 0);
    Ok(())
}

/// Make one record with `record`, again while back pressure refuses it.
fn taken<E: Into<Box<dyn Error>>>(
    persist: &Persist,
    mut record: impl FnMut() -> Result<(), E>,
) -> TestResult {
    wait_until("a record to be taken", || {
        let before = persist.bus().drops().back_pressure;
        record().map_err(Into::into)?;
        Ok(persist.bus().drops().back_pressure == before)
    })
}

/// Records dropped for anything but back pressure (which [`taken`] retries).
fn dropped_but_back_pressure(persist: &Persist) -> u64 {
    let drops = persist.bus().drops();
    drops.total() - drops.back_pressure
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
    wait_until("the stopped recording to be deleted", || {
        let report = ingester_c.tick()?;
        Ok(report
            .purged
            .iter()
            .any(|p| p.contains("all of it is in ClickHouse")))
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

#[test]
fn a_replayed_batch_is_inserted_once() -> TestResult {
    let lab = Lab::new("aeron_dedup", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(5);
    let persist = client(&lab, stream_id)?;
    {
        let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
        wait_until("the archive to record the stream", || {
            Ok(persist.is_connected())
        })?;
        record(&persist, 100)?;
        ingest(&mut ingester, &lab, "shapes", 100)?;
    }
    // The insert landed and the checkpoint was saved, which deletes the
    // pending file. Put that position back as pending and drop the
    // checkpoint: the next ingester replays the batch with the same token.
    let checkpoint = lab.dir.join("checkpoint");
    assert!(
        !lab.dir.join("checkpoint.pending").exists(),
        "a committed batch keeps no pending file"
    );
    std::fs::rename(&checkpoint, lab.dir.join("checkpoint.pending"))?;
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    record(&persist, 50)?;
    ingest(&mut ingester, &lab, "shapes", 150)?;
    Ok(())
}

#[test]
fn a_large_pending_batch_is_replayed_completely_before_inserting() -> TestResult {
    use rusteron_archive::{
        Aeron, AeronArchiveAsyncConnect, AeronArchiveContext, AeronContext, IntoCString,
    };

    let lab = Lab::new(
        "aeron_pending_prefix",
        "tables:\n  shapes: { kind: dynamic }\n",
    )?;
    let stream_id = stream(28);
    let recorder = ingester(&lab, lab.ch.clone(), stream_id)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    record(&persist, 300_000)?;
    drop(persist);
    drop(recorder);

    let ctx = AeronContext::new()?;
    ctx.set_dir(&aeron_dir().into_c_string())?;
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    let archive_ctx = AeronArchiveContext::new()?;
    archive_ctx.set_aeron(&aeron)?;
    archive_ctx.set_control_request_channel(c"aeron:ipc?term-length=64k")?;
    archive_ctx.set_control_response_channel(c"aeron:ipc?term-length=64k")?;
    let archive = AeronArchiveAsyncConnect::new_with_aeron(&archive_ctx, &aeron)?
        .poll_blocking(Duration::from_secs(10))?;
    let mut pending = None;
    wait_until("the complete recording to stop", || {
        archive.list_recordings_fn(&mut 0, 0, i32::MAX, |d| {
            if d.stream_id() == stream_id && d.stop_position() > 0 {
                pending = Some(format!("{} {}\n", d.recording_id(), d.stop_position()));
            }
        })?;
        Ok(pending.is_some())
    })?;
    // Crash after saving the full batch identity, before sending its insert.
    let pending_path = lab.dir.join("checkpoint.pending");
    std::fs::write(&pending_path, pending.ok_or("missing recording endpoint")?)?;
    let mut recovery = ingester(&lab, lab.ch.clone(), stream_id)?;
    wait_until("the complete pending batch to be committed", || {
        let report = recovery.tick()?;
        lab::clean(&report)?;
        let count = lab
            .query("SELECT count() FROM DB.shapes")
            .unwrap_or_default();
        assert!(
            count.is_empty() || count == "0" || count == "300000",
            "a partial insert consumes the full batch's deduplication token: {count} rows"
        );
        Ok(!pending_path.exists())
    })?;
    assert_eq!(lab.query("SELECT count() FROM DB.shapes")?, "300000");
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
    assert_eq!(persist.bus().dropped(), 1);

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
    assert_eq!(persist.bus().dropped(), 0);
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
    use ergon_runtime::event::Value;

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
    wait_until("recording on", || Ok(enabled(&persist)))?;

    // An invalid edit is rejected and the last good configuration stays.
    lab.write_config("tables:\n  shapes: { kind: sometimes }\n")?;
    settle(&persist, Duration::from_millis(2500));
    assert!(enabled(&persist));

    lab.write_config("tables:\n  shapes: { kind: dynamic, enabled: false }\n")?;
    wait_until("recording off", || Ok(!enabled(&persist)))?;
    Ok(())
}

#[test]
fn a_table_is_switched_per_app_until_a_time() -> TestResult {
    let lab = Lab::new(
        "aeron_per_app",
        "tables:\n  shapes: { kind: dynamic, enabled: false }\n",
    )?;
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id: stream(7),
        app: "binance".into(),
        subscriber_timeout: Duration::ZERO,
        ..ergon_runtime::Settings::new(&lab.config)
    };
    let persist = connect(settings)?;
    assert!(!persist.enabled(v1::TEMPLATE_ID), "off for every app");

    // Another app's entry leaves this one as `enabled` says.
    lab.write_config(
        "tables:\n  shapes: { kind: dynamic, enabled: false, apps: { bybit: true } }\n",
    )?;
    settle(&persist, Duration::from_millis(2500));
    assert!(!enabled(&persist));

    // On for this app until three seconds from now, then off by itself,
    // with no further edit.
    let until = jiff::Timestamp::now().checked_add(jiff::SignedDuration::from_secs(3))?;
    lab.write_config(&format!(
        "tables:\n  shapes: {{ kind: dynamic, enabled: false, apps: {{ binance: {{ until: {until} }} }} }}\n"
    ))?;
    wait_until("on for this app", || Ok(enabled(&persist)))?;
    wait_until("off once its time has passed", || Ok(!enabled(&persist)))?;
    assert!(jiff::Timestamp::now() >= until, "switched off early");
    Ok(())
}

#[test]
#[allow(clippy::float_cmp)] // the stored mean and the top percentile are exact
fn metrics_reach_their_tables_every_interval() -> TestResult {
    use ergon_runtime::clock::Clock;
    use tracing_subscriber::layer::SubscriberExt;

    let lab = Lab::new("aeron_metrics", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(14);
    let mut ingester = ingester(&lab, lab.ch.clone(), stream_id)?;
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        metrics_interval: Duration::from_secs(1),
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: "test-app".into(),
        ..ergon_runtime::Settings::new(&lab.config)
    };
    let persist = connect(settings)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    let metrics = persist.metrics();
    let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
    let depth = metrics.gauge("depth", &[]);
    let latency = metrics.histogram("latency_ns", &[("stage", "decode")]);
    sent.add(3);
    depth.set(2.5);
    for v in 1..=500 {
        latency.record(v * 1000);
    }
    // Instance ownership: no Persist::install or global metric handle.
    // Macro updates and ordinary handles share the same published registry.
    tracing::subscriber::with_default(tracing_subscriber::registry().with(persist.layer()), || {
        tracing::info!(counter = "orders_sent", venue = "binance", value = 2u64);
        tracing::info!(gauge = "depth", value = 2.5);
        for v in 501..=1000u64 {
            tracing::info!(histogram = "latency_ns", stage = "decode", value = v * 1000);
        }
    });

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
    let avg: f64 = lab.query("SELECT avg FROM DB.metrics_histogram")?.parse()?;
    assert_eq!(avg, 500_500.0);
    let p50: f64 = lab.query("SELECT p50 FROM DB.metrics_histogram")?.parse()?;
    let p99: f64 = lab.query("SELECT p99 FROM DB.metrics_histogram")?.parse()?;
    assert_eq!(p50, p99);
    let scale = 500_500.0;
    if (p50 - scale).abs() > scale * 0.001 {
        return Err(format!("{p50} is not within 0.1% of {scale}").into());
    }
    let p9999: f64 = lab
        .query("SELECT p9999 FROM DB.metrics_histogram")?
        .parse()?;
    assert_eq!(p9999, 1_000_000.0);
    Ok(())
}

#[test]
fn the_drivers_counters_are_sampled_with_their_streams_and_clients() -> TestResult {
    let lab = Lab::new("aeron_stats", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(15);
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        aeron_stats_interval: Duration::from_millis(100),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    let persist = client(&lab, stream_id)?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    // Publication counters exist before replay delivers the application's
    // Source metadata. Keep that initial sample to exercise late naming.
    wait_until("the publication's initial counter sample", || {
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        let n = lab.query(&format!(
            "SELECT count() FROM DB.aeron_counters WHERE type = 'pub-pos' AND stream_id = {stream_id}"
        ))?;
        Ok(n.parse::<u64>()? > 0)
    })?;
    assert_eq!(
        lab.query(&format!("SELECT DISTINCT client_name = '', value = 0 FROM DB.aeron_counters WHERE type = 'pub-pos' AND stream_id = {stream_id} FORMAT TSV"))?,
        "1\t1",
        "the first record has not sent Source or data yet"
    );
    record(&persist, 10)?;
    // Samples until one has a delta: the second sample of a counter.
    wait_until("two samples of this stream's publication", || {
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        std::thread::sleep(Duration::from_millis(100));
        let n = lab
            .query(&format!("SELECT count() FROM DB.aeron_counters WHERE type = 'pub-pos' AND stream_id = {stream_id} AND delta IS NOT NULL AND client_name = 'test-app'"))
            .unwrap_or_default();
        Ok(n.parse::<u64>().unwrap_or(0) > 0)
    })?;
    // The latest sample of every publication must have the name learned
    // from Source and the recorded bytes. Earlier samples legitimately
    // precede both; keep them rather than filtering unnamed rows away.
    assert_eq!(
        lab.query(&format!("SELECT tupleElement(sample, 1), tupleElement(sample, 2), tupleElement(sample, 3), tupleElement(sample, 4) FROM (SELECT argMax(tuple(client_name, channel, session_id IS NOT NULL, value >= 10 * 64), ts) AS sample FROM DB.aeron_counters WHERE type = 'pub-pos' AND stream_id = {stream_id} GROUP BY counter_id, registration_id) FORMAT TSV"))?,
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

    // Given a database in tables.yaml, the next samples land there.
    let metrics = format!("{}_metrics", lab.ch.database);
    lab.ch
        .query(&format!("DROP DATABASE IF EXISTS {metrics}"))?;
    lab.write_config(&format!(
        "tables:\n  shapes: {{ kind: dynamic }}\n  aeron_counters: {{ kind: static, database: {metrics} }}\n"
    ))?;
    wait_until("a sample in the database tables.yaml names", || {
        let report = ingester.tick()?;
        if !report.errors.is_empty() {
            return Err(format!("unexpected errors: {:?}", report.errors).into());
        }
        std::thread::sleep(Duration::from_millis(100));
        let n = lab
            .ch
            .query(&format!("SELECT count() FROM {metrics}.aeron_counters"))
            .unwrap_or_default();
        Ok(n.trim().parse::<u64>().unwrap_or(0) > 0)
    })?;
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn traces_become_spans_of_otel_traces() -> TestResult {
    use ergon_runtime::clock::{Clock, Nanos};
    use ergon_runtime::trace::TraceId;
    use tracing_subscriber::layer::SubscriberExt;

    const ORDERS: u64 = TraceId::namespace("order");

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
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        metrics_interval: Duration::from_secs(1),
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: "test-app".into(),
        ..ergon_runtime::Settings::new(&lab.config)
    };
    let persist = connect(settings)?;
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
    let mut fast = order.start(Nanos(0), TraceId::new(ORDERS, 41));
    fast.mark(Nanos(200));
    fast.mark(Nanos(500));
    fast.finish();
    let mut slow = order.start(Nanos(0), TraceId::new(ORDERS, 42));
    slow.mark(Nanos(100));
    slow.mark(Nanos(2_000_000));
    slow.finish();
    // Kept: published although fast and unsampled, under the id it learnt
    // part way.
    let mut kept = order.start(Nanos(0), order.next_id());
    kept.mark(Nanos(200));
    kept.set_id(TraceId::new(ORDERS, 43));
    kept.keep();
    kept.mark(Nanos(500));
    kept.finish();
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
        Ok(n("SELECT count() FROM DB.otel_traces") >= 18
            && n("SELECT count() FROM DB.metrics_histogram WHERE name = 'trace_ns'") >= 4)
    })?;
    // 2 ticks x (root + 3 stages), the slow and kept orders (root + 2
    // each), the gateway's (root + 1), and 2 spans.
    assert_eq!(lab.query("SELECT count() FROM DB.otel_traces")?, "18");
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
        lab.query("SELECT SpanAttributes['why'], Duration, TraceId FROM DB.otel_traces WHERE SpanName = 'order' AND ParentSpanId = '' ORDER BY Duration FORMAT TSV")?,
        format!("kept\t500\t{ORDERS:016x}{:016x}\nslow\t2000000\t{ORDERS:016x}{:016x}", 43, 42)
    );
    assert_eq!(
        lab.query("SELECT uniqExact(TraceId), count() FROM DB.otel_traces WHERE SpanName IN ('order', 'order_gateway') AND ParentSpanId = '' AND SpanAttributes['why'] != 'kept' FORMAT TSV")?,
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
    assert!(ergon_runtime::persist::enabled(v1::TEMPLATE_ID));
    for _ in 0..100 {
        ergon_runtime::persist::record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    }
    ergon_runtime::persist::record_row("signal", [("edge", ergon_runtime::event::Value::F64(0.5))]);
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
        persist.bus().drops(),
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
        persist.bus().drops(),
        Drops {
            not_connected: 1,
            too_large: 1,
            ..Drops::default()
        }
    );
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    assert_eq!(persist.bus().dropped(), 2);
    Ok(())
}

#[test]
fn connect_reports_when_nobody_is_recording() -> TestResult {
    let lab = Lab::new("aeron_wait", "tables:\n  shapes: { kind: dynamic }\n")?;
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id: stream(13),
        subscriber_timeout: Duration::from_millis(300),
        ..ergon_runtime::Settings::new(&lab.config)
    };
    match connect(settings) {
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
    assert_eq!(persist.bus().dropped(), 0);
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

/// The Aeron behaviour the multi-node lab rests on, on one driver: an MDC
/// feed publication with `ssc=true` publishes with no subscriber, the
/// node's archive records it through a spy (no network hop), and
/// subscribers reach it by name, reliably and best-effort.
#[test]
fn an_mdc_feed_is_spy_recorded_and_reachable_by_name() -> TestResult {
    use rusteron_archive::{
        Aeron, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveReplayParams,
        AeronContext, Handlers, IntoCString, SOURCE_LOCATION_LOCAL,
    };

    let stream_id = stream(17);
    let port = 41_000 + (std::process::id() % 1000).cast_signed() * 2;
    let ctx = AeronContext::new()?;
    ctx.set_dir(&aeron_dir().into_c_string())?;
    let aeron = Aeron::new(&ctx)?;
    aeron.start()?;
    let archive_ctx = AeronArchiveContext::new()?;
    archive_ctx.set_aeron(&aeron)?;
    archive_ctx.set_control_request_channel(c"aeron:ipc?term-length=64k")?;
    archive_ctx.set_control_response_channel(c"aeron:ipc?term-length=64k")?;
    let archive = AeronArchiveAsyncConnect::new_with_aeron(&archive_ctx, &aeron)?
        .poll_blocking(Duration::from_secs(10))?;
    // The node's archive listens for the feed before it exists.
    let spy = format!("aeron-spy:aeron:udp?control=127.0.0.1:{port}|control-mode=dynamic");
    archive.start_recording(
        &spy.as_str().into_c_string(),
        stream_id,
        SOURCE_LOCATION_LOCAL,
        false,
    )?;

    let feed = format!(
        "aeron:udp?control=127.0.0.1:{port}|control-mode=dynamic|fc=min|ssc=true|term-length=64k"
    );
    let publication = aeron
        .async_add_publication(&feed.as_str().into_c_string(), stream_id)?
        .poll_blocking(Duration::from_secs(10))?;
    wait_until("the spy to connect the publication (ssc)", || {
        Ok(publication.is_connected())
    })?;
    let offer = |i: u8| -> TestResult {
        wait_until("an offer to succeed", || {
            Ok(publication.offer(&[i; 32]).is_ok())
        })
    };
    // No subscriber at all: still published, and archived.
    for i in 0..10 {
        offer(i)?;
    }

    let subscribe = |extra: &str| {
        let channel = format!(
            "aeron:udp?endpoint=127.0.0.1:0|control=localhost:{port}|control-mode=dynamic{extra}"
        );
        aeron
            .async_add_subscription(
                &channel.as_str().into_c_string(),
                stream_id,
                Handlers::NONE,
                Handlers::NONE,
            )?
            .poll_blocking(Duration::from_secs(10))
    };
    let reliable = subscribe("")?;
    let best_effort = subscribe("|reliable=false|tether=false|group=false")?;
    wait_until("both subscribers to join by name", || {
        Ok(reliable.is_connected() && best_effort.is_connected())
    })?;
    for i in 10..20 {
        offer(i)?;
    }
    // A subscriber joins at the sender's position, which can still be
    // behind the publisher's (the first ten went to no network receiver):
    // it gets a run of messages ending at the last, with the ten sent after
    // it joined, none lost and in order.
    for (name, sub) in [("reliable", &reliable), ("best-effort", &best_effort)] {
        let mut got = Vec::new();
        wait_until(&format!("the {name} subscriber's ten"), || {
            sub.poll_fn(|m, _| got.push(m[0]), 100)?;
            Ok(got.last() == Some(&19))
        })?;
        let first = *got.first().ok_or("nothing received")?;
        assert!(first <= 10, "{name}: {got:?}");
        assert_eq!(got, (first..20).collect::<Vec<u8>>(), "{name}");
    }

    // The archive has all twenty, from before and after the subscribers.
    let mut ids = Vec::new();
    let mut count = 0;
    archive.list_recordings_for_uri_fn(&mut count, 0, 1000, c"aeron:udp", stream_id, |d| {
        ids.push(d.recording_id());
    })?;
    let recording = *ids.last().ok_or("the spy made no recording")?;
    let params = AeronArchiveReplayParams::new(-1, -1, 0, -1, -1, -1)?;
    let session = archive.start_replay(recording, c"aeron:ipc", stream_id + 1, &params)?;
    let session_id = ergon_runtime::subscription::replay_image_session(session);
    let replay = aeron
        .async_add_subscription(
            &format!("aeron:ipc?session-id={session_id}").into_c_string(),
            stream_id + 1,
            Handlers::NONE,
            Handlers::NONE,
        )?
        .poll_blocking(Duration::from_secs(10))?;
    let mut replayed = Vec::new();
    wait_until("the recording to replay", || {
        replay.poll_fn(|m, _| replayed.push(m[0]), 100)?;
        Ok(replayed.len() >= 20)
    })?;
    assert_eq!(replayed, (0..20).collect::<Vec<u8>>());
    let _ = archive.stop_replay(session);
    let _ = archive.stop_recording_channel_and_stream(&spy.as_str().into_c_string(), stream_id);
    Ok(())
}

/// With a `frame` table in `tables.yaml`, every feed frame is also kept as
/// is: its publish stamp, recording and position, the feed's names, and the
/// message bytes, sorted by feed and time (not by the bytes).
#[test]
fn feed_frames_are_kept_raw_in_the_frame_table() -> TestResult {
    let lab = Lab::new(
        "aeron_frames",
        "tables:\n  shapes: { kind: dynamic }\n  frame: { kind: static }\n",
    )?;
    let stream_id = stream(31);
    let feed_stream = stream(32);
    let port = 47_000 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    let streams = ergon_runtime::streams::Streams::parse(&format!(
        "services:\n  md-frames: {{ port: {port}, region: an1, streams: {{ md: {feed_stream} }} }}\nkinds:\n  md: {{ reliable: true }}\n"
    ))?;
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        streams: Some(streams.clone()),
        host_ip: "127.0.0.1".into(),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    let persist = client(&lab, stream_id)?;
    let feed = persist
        .bus()
        .publication(&streams.publication("md-frames", "127.0.0.1")?, feed_stream)?;
    wait_until("the persist stream and the feed to be recorded", || {
        Ok(persist.is_connected() && feed.is_connected())
    })?;
    let before = ergon_runtime::clock::epoch_now().0;
    for _ in 0..20 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    let after = ergon_runtime::clock::epoch_now().0;
    ingest(&mut ingester, &lab, "shapes", 20)?;
    wait_until("the frames", || {
        ingester.tick()?;
        let n = lab.query(&format!(
            "SELECT count() FROM DB.frame WHERE template_id = {}",
            v1::TEMPLATE_ID
        ))?;
        Ok(n.parse::<u64>()? >= 20)
    })?;
    assert_eq!(
        lab.query(&format!(
            "SELECT count(), uniqExact(position), any(service), any(kind), any(app), \
             countIf(length(message) = {}), countIf(toUnixTimestamp64Nano(ts) BETWEEN {before} AND {after}) \
             FROM DB.frame WHERE template_id = {} FORMAT TSV",
            v1::LEN,
            v1::TEMPLATE_ID
        ))?,
        "20\t20\tmd-frames\tmd\ttest-app\t20\t20"
    );
    assert_eq!(
        lab.query(
            "SELECT sorting_key FROM system.tables WHERE database = 'DB' AND name = 'frame'"
        )?,
        "service, kind, ts"
    );
    Ok(())
}

/// A feed frame's reserved value is its publish time, so its rows take the
/// source the recording's `Source` message named. An ingester that resumes
/// past that message (it is sent once, then every 5 s) still knows it, from
/// the sources saved beside its checkpoint.
#[test]
fn a_feed_resumed_mid_recording_keeps_its_source() -> TestResult {
    let lab = Lab::new(
        "aeron_feed_resume",
        "tables:\n  shapes: { kind: dynamic }\n",
    )?;
    let stream_id = stream(29);
    let feed_stream = stream(30);
    let port = 46_000 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    let streams = ergon_runtime::streams::Streams::parse(&format!(
        "services:\n  md-resume: {{ port: {port}, region: an1, streams: {{ md: {feed_stream} }} }}\nkinds:\n  md: {{ reliable: true }}\n"
    ))?;
    let settings = || ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        streams: Some(streams.clone()),
        host_ip: "127.0.0.1".into(),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings())?;
    let persist = client(&lab, stream_id)?;
    let feed = persist
        .bus()
        .publication(&streams.publication("md-resume", "127.0.0.1")?, feed_stream)?;
    wait_until("the persist stream and the feed to be recorded", || {
        Ok(persist.is_connected() && feed.is_connected())
    })?;
    for _ in 0..20 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    ingest(&mut ingester, &lab, "shapes", 20)?;
    assert!(
        lab.dir.join("checkpoint.sources").is_file(),
        "the feed recording's source is saved"
    );
    drop(ingester);
    // No `Source` message between these and the checkpoint.
    for _ in 0..20 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    let mut resumed = Ingester::connect(&[v1::SCHEMA], settings())?;
    ingest(&mut resumed, &lab, "shapes", 40)?;
    assert_eq!(
        lab.query("SELECT app, count() FROM DB.shapes GROUP BY app FORMAT TSV")?,
        "test-app\t40",
        "rows after the checkpoint take the saved source"
    );
    Ok(())
}

/// A feed (a UDP publication in the registry) is recorded by the node's
/// archive through the ingester's spy, and ingested beside the persist
/// stream: two live recordings at once. Its tables are switched when
/// inserted, since the feed itself is published regardless.
#[test]
fn a_feed_is_recorded_by_spy_and_its_table_switched_at_insert() -> TestResult {
    let lab = Lab::new("aeron_feed", "tables:\n  shapes: { kind: dynamic }\n")?;
    let stream_id = stream(18);
    let feed_stream = stream(19);
    let port = 42_000 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    let streams = ergon_runtime::streams::Streams::parse(&format!(
        "services:\n  md-test: {{ port: {port}, region: an1, streams: {{ md: {feed_stream} }} }}\nkinds:\n  md: {{ reliable: true }}\n"
    ))?;
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        streams: Some(streams.clone()),
        host_ip: "127.0.0.1".into(),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    let persist = client(&lab, stream_id)?;
    let feed = persist
        .bus()
        .publication(&streams.publication("md-test", "127.0.0.1")?, feed_stream)?;
    wait_until("the persist stream and the feed to be recorded", || {
        Ok(persist.is_connected() && feed.is_connected())
    })?;
    assert!(
        feed.max_payload() < 1500,
        "one UDP frame: {}",
        feed.max_payload()
    );
    for _ in 0..50 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    record(&persist, 30)?;
    ingest(&mut ingester, &lab, "shapes", 80)?;
    assert_eq!(
        lab.query("SELECT app, count() FROM DB.shapes GROUP BY app FORMAT TSV")?,
        "test-app\t80",
        "the feed's rows are attributed through the Source message on the feed itself"
    );

    // Off for this app: the feed still publishes (a subscriber needs it) and
    // the archive still records it, but the ingester keeps it out.
    lab.write_config("tables:\n  shapes: { kind: dynamic, apps: { test-app: false } }\n")?;
    std::thread::sleep(Duration::from_millis(1500));
    for _ in 0..20 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    for _ in 0..10 {
        let report = ingester.tick()?;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(dropped_but_back_pressure(&persist), 0, "published");
    assert_eq!(
        lab.query("SELECT count() FROM DB.shapes")?,
        "80",
        "not inserted"
    );

    // SIGTERM: every publication closes at once, so subscribers move on to
    // the next publisher without waiting out this client's timeout. A
    // record after it is quietly not published.
    let before = persist.bus().dropped();
    persist.bus().shutdown();
    feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    assert_eq!(persist.bus().dropped(), before, "not counted as drops");
    Ok(())
}

#[test]
fn a_subscriber_follows_a_restarted_publisher_and_is_told_so() -> TestResult {
    use ergon_runtime::streams::subscription_channel;

    let lab = Lab::new("aeron_subscriber", "tables:\n  shapes: { kind: dynamic }\n")?;
    let (stream_id, feed_stream) = (stream(20), stream(21));
    let port = 42_001 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    let publication = format!(
        "aeron:udp?control=127.0.0.1:{port}|control-mode=dynamic|fc=max|ssc=true|term-length=64k"
    );
    let reader = client(&lab, stream_id)?;
    let mut sub = reader.bus().subscription(
        &subscription_channel("localhost", port, "127.0.0.1", true),
        feed_stream,
    );
    // A name that never resolves: retried quietly, never a panic.
    let mut nowhere = reader.bus().subscription(
        &subscription_channel("nowhere.invalid", port, "127.0.0.1", true),
        feed_stream,
    );
    // v1 messages, and messages that started a session.
    let (rows, sessions) = (std::cell::Cell::new(0), std::cell::Cell::new(0));
    let poll = |sub: &mut ergon_runtime::subscription::Subscription| {
        sub.poll(
            |m, delivery| {
                assert!(delivery.is_live(), "a plain subscription replays nothing");
                if m.get(4..6) == Some(&v1::SCHEMA_ID.to_le_bytes()[..]) {
                    rows.set(rows.get() + 1);
                }
                sessions.set(sessions.get() + usize::from(delivery.first));
            },
            100,
        )
    };
    for n in 1..=2 {
        // A publisher, then its restart: a new client, so a new session.
        let publisher = client(&lab, stream_id)?;
        let feed = publisher.bus().publication(&publication, feed_stream)?;
        // Both ends: the subscriber's image appears one status message
        // before the publisher counts it (no spy here for `ssc`).
        wait_until("the subscriber and the publisher to connect", || {
            poll(&mut sub);
            Ok(sub.is_connected() && feed.is_connected())
        })?;
        // Connected a moment before the driver raises the publication's
        // limit: early records are back pressured. Each is retried until
        // taken.
        for _ in 0..5 {
            taken(&publisher, || {
                feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
            })?;
        }
        wait_until("the publisher's rows", || {
            poll(&mut sub);
            Ok(rows.get() == 5 * n)
        })?;
        // The next publisher starts at once. A publication still open on
        // the driver would be shared, in the same session, and the
        // subscriber would never see the restart: `shutdown` returns only
        // once the driver has the close.
        publisher.bus().shutdown();
    }
    assert_eq!(sessions.get(), 2, "the first session, then the restart");
    assert_eq!(poll(&mut nowhere), 0);
    assert!(!nowhere.is_connected());
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn a_persistent_subscription_loses_nothing_across_a_publisher_restart() -> TestResult {
    let lab = Lab::new("aeron_persistent", "tables:\n  shapes: { kind: dynamic }\n")?;
    let (stream_id, feed_stream) = (stream(22), stream(23));
    let port = 43_000 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    // `md-test.localhost` is this machine, and so is its archive.
    let streams = ergon_runtime::streams::Streams::parse(&format!(
        "domain: localhost\narchive_port: 18010\nservices:\n  md-test: {{ port: {port}, region: an1, streams: {{ md: {feed_stream} }} }}\nkinds:\n  md: {{ reliable: true }}\n"
    ))?;
    // The node's ingester has its archive record the feed.
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        streams: Some(streams.clone()),
        host_ip: "127.0.0.1".into(),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let _ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    let publication = streams.publication("md-test", "127.0.0.1")?;
    let reader = client(&lab, stream_id)?;
    let mut sub = reader.bus().subscribe(&streams, "md-test", "md")?;
    // v1 messages, those replayed, and messages that started a subscription.
    let (rows, replayed, fresh) = (
        std::cell::Cell::new(0),
        std::cell::Cell::new(0),
        std::cell::Cell::new(0),
    );
    let poll = |sub: &mut ergon_runtime::subscription::PersistentSubscription| {
        sub.poll(
            |m, delivery| {
                if m.get(4..6) == Some(&v1::SCHEMA_ID.to_le_bytes()[..]) {
                    rows.set(rows.get() + 1);
                    replayed.set(replayed.get() + usize::from(!delivery.is_live()));
                }
                fresh.set(fresh.get() + usize::from(delivery.first));
            },
            100,
        )
    };

    let first = client(&lab, stream_id)?;
    let feed = first.bus().publication(&publication, feed_stream)?;
    wait_until("the feed to be recorded", || Ok(feed.is_connected()))?;
    // It starts from live: what was published before is not its business.
    wait_until("the subscriber to go live", || {
        taken(&first, || feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode))?;
        poll(&mut sub);
        Ok(sub.is_live())
    })?;
    // Take everything in flight: quiet for 200 ms.
    let drain = |sub: &mut ergon_runtime::subscription::PersistentSubscription| {
        let mut quiet = Instant::now();
        while quiet.elapsed() < Duration::from_millis(200) {
            if poll(sub) > 0 {
                quiet = Instant::now();
            }
        }
    };
    drain(&mut sub);
    rows.set(0);
    for _ in 0..5 {
        taken(&first, || feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode))?;
    }
    wait_until("the first publisher's five", || {
        poll(&mut sub);
        Ok(rows.get() >= 5)
    })?;
    assert_eq!(rows.get(), 5);

    // A restart: a new session, which publishes before the subscriber polls
    // again, past the old session's last position. Back on the live stream,
    // Aeron's persistent subscription would take the new image as if it
    // continued the old recording, having missed its start. Every one of its
    // messages arrives, once, as a new session.
    first.bus().shutdown();
    let second = client(&lab, stream_id)?;
    let feed = second.bus().publication(&publication, feed_stream)?;
    for _ in 0..100 {
        taken(&second, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    wait_until("the second publisher's hundred", || {
        poll(&mut sub);
        Ok(rows.get() >= 105)
    })?;
    drain(&mut sub);
    assert_eq!(rows.get(), 105, "every message, once");
    assert!(
        replayed.get() > 0,
        "the new session's start comes from the recording, as a replay"
    );
    // Caught up: what is published now arrives live.
    wait_until("the subscriber to be live again", || {
        poll(&mut sub);
        Ok(sub.is_live())
    })?;
    let before = replayed.get();
    taken(&second, || {
        feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
    })?;
    wait_until("one more, live", || {
        poll(&mut sub);
        Ok(rows.get() == 106)
    })?;
    assert_eq!(replayed.get(), before, "a live message is not a replay");
    assert_eq!(
        fresh.get(),
        2,
        "the first subscription, then the new session's"
    );
    Ok(())
}

#[test]
fn a_feed_added_to_the_registry_is_recorded_without_a_restart() -> TestResult {
    use ergon_runtime::streams::Streams;

    let lab = Lab::new("aeron_follow", "tables:\n  shapes: { kind: dynamic }\n")?;
    let (stream_id, feed_stream) = (stream(24), stream(25));
    let port = 44_000 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    let path = lab.dir.join("streams.yaml");
    std::fs::write(&path, "services: {}\nkinds:\n  md: { reliable: true }\n")?;
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        streams: Some(Streams::load(&path)?),
        host_ip: "127.0.0.1".into(),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    ingester.follow(&path);

    // A new service in the registry, as a ConfigMap update delivers it.
    let registry = format!(
        "services:\n  md-new: {{ port: {port}, region: an1, streams: {{ md: {feed_stream} }} }}\nkinds:\n  md: {{ reliable: true }}\n"
    );
    std::fs::write(&path, &registry)?;
    let streams = Streams::parse(&registry)?;
    let persist = client(&lab, stream_id)?;
    let feed = persist
        .bus()
        .publication(&streams.publication("md-new", "127.0.0.1")?, feed_stream)?;
    // Connected only once the archive's spy records it (`ssc`).
    wait_until("the ingester to record the new feed", || {
        ingester.tick()?;
        Ok(feed.is_connected())
    })?;
    for _ in 0..20 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    ingest(&mut ingester, &lab, "shapes", 20)?;
    Ok(())
}

#[test]
fn a_recording_with_nothing_in_it_yet_waits_for_data() -> TestResult {
    let lab = Lab::new("aeron_empty", "tables:\n  shapes: { kind: dynamic }\n")?;
    let (stream_id, feed_stream) = (stream(26), stream(27));
    let port = 45_000 + u16::try_from(std::process::id() % 1000).unwrap_or(0) * 2;
    let streams = ergon_runtime::streams::Streams::parse(&format!(
        "services:\n  md-idle: {{ port: {port}, region: an1, streams: {{ md: {feed_stream} }} }}\nkinds:\n  md: {{ reliable: true }}\n"
    ))?;
    let settings = ergon_runtime_server::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        recheck: Duration::ZERO,
        streams: Some(streams.clone()),
        host_ip: "127.0.0.1".into(),
        ..ergon_runtime_server::Settings::new(
            lab.ch.clone(),
            &lab.config,
            lab.dir.join("checkpoint"),
        )
    };
    let mut ingester = Ingester::connect(&[v1::SCHEMA], settings)?;
    // A feed handler opens its feed at start and publishes nothing until its
    // exchange connects: an active recording with nothing past its start,
    // which the archive refuses to replay.
    let persist = client(&lab, stream_id)?;
    let feed = persist
        .bus()
        .publication(&streams.publication("md-idle", "127.0.0.1")?, feed_stream)?;
    wait_until("the feed to be recorded", || Ok(feed.is_connected()))?;
    for _ in 0..3 {
        let report = ingester.tick()?;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }
    for _ in 0..5 {
        taken(&persist, || {
            feed.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)
        })?;
    }
    ingest(&mut ingester, &lab, "shapes", 5)?;
    Ok(())
}
