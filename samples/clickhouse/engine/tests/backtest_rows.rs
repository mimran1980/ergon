//! The same backtest, run twice over one window of frames recorded in
//! `ClickHouse`, records the same rows. The runs go the lab's whole way: the
//! `ClickHouse` frame source, Persist, the Archive and the ingester. Needs
//! the test `ClickHouse` and the archiving media driver, which `just test`
//! starts.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::time::Duration;

use ergon_runtime::bus::Bus;
use ergon_runtime::clickhouse_source::ClickHouseConfig;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::frames::{self, FrameRow};
use ergon_runtime::persist::Persist;
use ergon_runtime::rt::sim::SimConfig;
use ergon_runtime::source::Source;
use ergon_runtime_server::{ClickHouse, Ingester, Writer};
use lab::Streams;

const CHANNEL: &str = "aeron:ipc?term-length=1m";
/// The fixture's feeds as frames, and what the engine and its simulated
/// venue record.
const TABLES: &str = "tables:
  frame: { kind: dynamic }
  ema: { kind: dynamic }
  agg_book: { kind: dynamic }
  new_order: { kind: dynamic }
  execution_report: { kind: dynamic }
";
const OUTPUTS: [&str; 4] = ["ema", "agg_book", "new_order", "execution_report"];

fn url() -> String {
    std::env::var("CLICKHOUSE_TEST_URL").unwrap_or_else(|_| "http://localhost:18123".into())
}

fn aeron_dir() -> String {
    std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into())
}

#[test]
fn a_backtest_run_twice_records_the_same_rows() -> Result<(), Box<dyn Error>> {
    let pid = std::process::id();
    let database = "persist_test_backtest_rows";
    let ch = ClickHouse::new(&url(), "lab", "lab", database);
    ch.query(&format!("DROP DATABASE IF EXISTS {database}"))
        .map_err(|e| format!("ClickHouse at {} is required (run `just test`): {e}", url()))?;
    let dir = std::env::temp_dir().join(format!("engine-backtest-rows-{pid}"));
    std::fs::create_dir_all(&dir)?;
    let tables = dir.join("tables.yaml");
    std::fs::write(&tables, TABLES)?;
    let window = record_frames(&ch, &tables)?;

    // The ingester has the archive record the persist stream before either
    // run publishes on it. Even, as its replays use the next stream id.
    let stream_id = 2_000_000 + i32::try_from(pid % 100_000)? * 2;
    let mut ingester = Ingester::connect(
        &[schema::TRADING_SCHEMA],
        ergon_runtime_server::Settings {
            aeron_dir: Some(aeron_dir()),
            channel: CHANNEL.into(),
            stream_id,
            recheck: Duration::ZERO,
            ..ergon_runtime_server::Settings::new(ch.clone(), &tables, dir.join("checkpoint"))
        },
    )
    .map_err(|e| {
        format!(
            "an ArchivingMediaDriver on {} is required (run `just test`): {e}",
            aeron_dir()
        )
    })?;
    for run in ["a", "b"] {
        backtest(run, window, &tables, stream_id, database)?;
    }
    // Each run's recording stops with its run, and is deleted once all of it
    // is in ClickHouse.
    let clock = Clock::new();
    let deadline = clock.now().0 + 60_000_000_000;
    let mut deleted = 0;
    while deleted < 2 {
        let report = ingester.tick()?;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        deleted += report
            .purged
            .iter()
            .filter(|p| p.contains("all of it is in ClickHouse"))
            .count();
        assert!(
            clock.now().0 < deadline,
            "the runs' recordings did not reach ClickHouse"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    for table in OUTPUTS {
        let runs = rows_by_run(&ch, database, table)?;
        assert!(
            runs.get("a").is_some_and(|&(count, _)| count > 0),
            "{table}: the backtest recorded nothing: {runs:?}"
        );
        assert_eq!(
            runs.get("a"),
            runs.get("b"),
            "{table}: the two runs recorded different rows"
        );
    }
    ch.query(&format!("DROP DATABASE {database}"))?;
    Ok(())
}

/// The engine's replay fixture as `frame` rows, as the ingester writes a
/// feed's recording, and the window they span.
fn record_frames(ch: &ClickHouse, tables: &Path) -> Result<(Nanos, Nanos), Box<dyn Error>> {
    let market = engine::replay::inbound(&engine::replay::Market::FIXTURE);
    let frames::Parsed { names, records } = frames::parse(&market)?;
    let mut writer = Writer::new(&[frames::SCHEMA], ch.clone(), tables, Duration::ZERO)?;
    let source = Source::at("test-node", "test-pod", "md-test", 1, "");
    assert!(writer.push(&source.message()?, source.id));
    let (mut from, mut to) = (i64::MAX, i64::MIN);
    let mut row = Vec::new();
    for record in records {
        let name = names
            .get(usize::try_from(record.stream)?)
            .ok_or("a record of an unnamed stream")?;
        let (service, kind) = name.split_once('/').ok_or("a feed is service/kind")?;
        row.clear();
        FrameRow {
            ts: record.ts,
            recording: i64::from(record.stream) + 1,
            position: i64::try_from(record.offset)? + 1,
            session: 1,
            stream: 1,
            source: source.id,
            service,
            kind,
            message: record.frame,
        }
        .encode(&mut row)?;
        assert!(writer.push(&row, source.id));
        from = from.min(record.ts.0);
        to = to.max(record.ts.0);
    }
    let report = writer.tick();
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(writer.queued_bytes(), 0);
    Ok((Nanos(from), Nanos(to + 1)))
}

/// One backtest of `window` from `ClickHouse`, recorded as `run`, as
/// `backtest --source clickhouse --record run` does.
fn backtest(
    run: &str,
    (from, to): (Nanos, Nanos),
    tables: &Path,
    stream_id: i32,
    database: &str,
) -> Result<(), Box<dyn Error>> {
    let streams = Streams::parse(engine::replay::STREAMS)?;
    let mut config = SimConfig::new();
    config.directory = Box::new(streams.clone());
    config.region = engine::replay::REGION.into();
    config.from = Some(from);
    config.to = Some(to);
    for route in ["engine", "exch-sim"] {
        let kind = if route == "engine" { "orders" } else { "exec" };
        config
            .loopback
            .insert(format!("{route}-{}/{kind}", config.region), 100_000);
    }
    config.clickhouse = Some(ClickHouseConfig {
        client: ergon_runtime::clickhouse::ClickHouse::new(&url(), "lab", "lab", database),
        table: "frame".into(),
    });
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir()),
        channel: CHANNEL.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        host: "test-host".into(),
        pod: "test-pod".into(),
        app: format!("engine-{}", config.region),
        run: run.into(),
        sim_start: Some(from),
        ..ergon_runtime::Settings::new(tables)
    };
    let bus = Bus::connect(&settings)?;
    let persist = Persist::connect(schema::TRADING_SCHEMA, &bus, settings)?;
    let clock = Clock::new();
    let deadline = clock.now().0 + 15_000_000_000;
    while !persist.is_connected() {
        let _ = bus.poll();
        assert!(
            clock.now().0 < deadline,
            "Persist did not reach the archive"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    config.persist = Some(persist);
    config.bus = Some(bus);
    engine::backtest::execute(config, streams, Vec::new())?;
    Ok(())
}

/// Each run's row count and the sum of its rows' hashes, every column but
/// `run`: equal for two runs that recorded the same rows in any order.
fn rows_by_run(
    ch: &ClickHouse,
    database: &str,
    table: &str,
) -> Result<BTreeMap<String, (u64, u64)>, Box<dyn Error>> {
    let columns = ch.query(&format!(
        "SELECT name FROM system.columns WHERE database = '{database}' \
         AND table = '{table}' AND name != 'run' ORDER BY position FORMAT TSV"
    ))?;
    let columns: Vec<_> = columns.lines().map(|c| format!("`{c}`")).collect();
    if columns.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows = ch.query(&format!(
        "SELECT run, count(), sum(cityHash64(tuple({}))) FROM {database}.{table} \
         GROUP BY run FORMAT TSV",
        columns.join(", ")
    ))?;
    rows.lines()
        .map(|line| {
            let mut fields = line.split('\t');
            match (fields.next(), fields.next(), fields.next()) {
                (Some(run), Some(count), Some(hash)) => {
                    Ok((run.to_owned(), (count.parse()?, hash.parse()?)))
                }
                _ => Err(format!("{table}: an unexpected row {line:?}").into()),
            }
        })
        .collect()
}
