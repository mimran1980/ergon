//! Application -> Aeron Archive -> ingester -> ClickHouse, for real.
//!
//! Needs the test ClickHouse (`CLICKHOUSE_TEST_URL`) and an Aeron
//! `ArchivingMediaDriver` on `AERON_TEST_DIR` (default
//! `/tmp/persist-test-aeron`); `just test` starts both. Every test fails,
//! never skips, when they are missing. Each test records its own stream.

use std::error::Error;
use std::time::{Duration, Instant};

use persist_client::Persist;
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
    Ok(Ingester::connect(v1::SCHEMA, settings).map_err(|e| {
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
        "DERIBIT\t41.5\t0\nHYPERLIQUID\t0\t7"
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
        // A new field is a new column; a missing one reads its default.
        tracing::info!(table = "signal", instrument = %"ETHUSDT", edge = 0.5, flag = true);
        tracing::info!(table = "fixed", a = 1);
        tracing::info!(table = "quiet", x = 1); // disabled
        tracing::info!(no_table = 1); // not a record
    });
    ingest(&mut ingester, &lab, "signal", 2)?;
    assert_eq!(
        lab.query("SELECT instrument, edge, n, flag FROM DB.signal ORDER BY ts FORMAT TSV")?,
        "BTCUSDT\t0.25\t3\tfalse\nETHUSDT\t0.5\t0\ttrue"
    );
    assert_eq!(
        lab.query("SELECT name, type FROM system.columns WHERE database = 'DB' AND table = 'signal' ORDER BY position FORMAT TSV")?,
        "ts\tDateTime64(9, \\'UTC\\')\ninstrument\tString\nedge\tFloat64\nn\tInt64\nflag\tBool\ninserted_at\tDateTime64(3, \\'UTC\\')"
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
            "fixed: static table is missing column b; not writing it. Fix: ALTER TABLE `persist_test_aeron_events`.`fixed` ADD COLUMN IF NOT EXISTS `b` Int64"
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
            "signal: 1 value(s) did not match their column's type (set by the first value seen); wrote the default"
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
fn an_override_file_switches_tables_for_one_application() -> TestResult {
    let lab = Lab::new("aeron_override", "tables:\n  shapes: { kind: dynamic }\n")?;
    let overrides = lab.dir.join("app.yaml");
    let settings = persist_client::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id: stream(7),
        overrides_path: Some(overrides.clone()),
        ..persist_client::Settings::new(&lab.config)
    };
    let persist = Persist::connect(v1::SCHEMA, settings)?;
    assert!(
        persist.enabled(v1::TEMPLATE_ID),
        "no override file: tables.yaml decides"
    );

    std::fs::write(&overrides, "tables:\n  shapes: { enabled: false }\n")?;
    wait_until("the override to switch it off", || {
        Ok(!persist.enabled(v1::TEMPLATE_ID))
    })?;
    // An override naming a table tables.yaml lacks is rejected whole.
    std::fs::write(
        &overrides,
        "tables:\n  shapes: { enabled: true }\n  nope: { enabled: true }\n",
    )?;
    std::thread::sleep(Duration::from_millis(2500));
    assert!(!persist.enabled(v1::TEMPLATE_ID));

    std::fs::remove_file(&overrides)?;
    wait_until("tables.yaml to decide again", || {
        Ok(persist.enabled(v1::TEMPLATE_ID))
    })?;
    Ok(())
}

#[test]
fn a_record_aeron_cannot_take_is_dropped_and_counted() -> TestResult {
    let lab = Lab::new("aeron_dropped", "tables:\n  shapes: { kind: dynamic }\n")?;
    // No ingester has asked the archive to record this stream.
    let persist = client(&lab, stream(2))?;
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    assert_eq!(persist.dropped(), 1);

    let _ingester = ingester(&lab, lab.ch.clone(), stream(2))?;
    wait_until("the archive to record the stream", || {
        Ok(persist.is_connected())
    })?;
    // Larger than the MTU: one `try_claim` cannot hold it.
    persist.record(v1::TEMPLATE_ID, 4096, |_| {
        Err("encoded an oversized record")
    })?;
    assert_eq!(persist.dropped(), 2);
    persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
    assert_eq!(persist.dropped(), 2);
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
