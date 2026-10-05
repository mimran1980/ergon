//! Run an engine and its simulated venue against file, Archive or ClickHouse history.
use std::collections::BTreeMap;

use ergon_runtime::clock::Nanos;
use ergon_runtime::rt::ArchiveConfig;
use ergon_runtime::rt::sim::SimConfig;
use lab::Streams;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!(
            "backtest [--source file|archive|clickhouse|journal] [--input PATH] [--output PATH] [--streams PATH] [--region REGION] [--from EPOCH_NS] [--to EPOCH_NS] [--latency-ns NS] [--route-delay SERVICE/KIND=NS|measured] [--journal RUN_ID] [--record RUN_ID]\nWith no input, file mode uses the synthetic fixture. Recording needs the lab's Aeron ingester. ClickHouse mode requires --features clickhouse."
        );
        return Ok(());
    }
    let mut options = BTreeMap::new();
    let mut routes = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2 {
            return Err(format!("missing value for {}", pair[0]).into());
        }
        if pair[0] == "--route-delay" {
            let (name, delay) = pair[1]
                .split_once('=')
                .ok_or("route delay needs SERVICE/KIND=NS")?;
            routes.insert(name.to_owned(), delay.to_owned());
        } else {
            if ![
                "--source",
                "--input",
                "--output",
                "--streams",
                "--region",
                "--from",
                "--to",
                "--latency-ns",
                "--record",
                "--journal",
            ]
            .contains(&pair[0].as_str())
            {
                return Err(format!("unknown option {}", pair[0]).into());
            }
            options.insert(pair[0].as_str(), pair[1].as_str());
        }
    }
    let source = options.get("--source").copied().unwrap_or("file");
    let streams = match options.get("--streams") {
        Some(path) => Streams::load(path)?,
        None if source == "file" => Streams::parse(engine::replay::STREAMS)?,
        None => Streams::load(lab::streams_path())?,
    };
    let mut config = SimConfig::new();
    // An archive replay finds a feed's recordings by its stream id.
    config.directory = Box::new(streams.clone());
    config.region = options.get("--region").map_or_else(
        || {
            if source == "file" {
                engine::replay::REGION.into()
            } else {
                std::env::var("REGION").unwrap_or_else(|_| ergon_runtime::UNKNOWN_REGION.into())
            }
        },
        |region| (*region).into(),
    );
    config.from = options
        .get("--from")
        .map(|n| n.parse::<i64>().map(Nanos))
        .transpose()?;
    config.to = options
        .get("--to")
        .map(|n| n.parse::<i64>().map(Nanos))
        .transpose()?;
    for (name, delay) in routes {
        let nanos = if delay == "measured" {
            measured(&streams, &config, &name)?
        } else {
            delay.parse::<i64>()?
        };
        config.route_delays.insert(name, nanos);
    }
    let file_logs = if source == "file" {
        let bytes = match options.get("--input") {
            Some(path) => std::fs::read(path)?,
            None => engine::replay::inbound(&engine::replay::Market::FIXTURE),
        };
        if config.from.is_none() {
            config.from = ergon_runtime::frames::parse(&bytes)?
                .records
                .map(|r| r.ts)
                .min();
        }
        vec![bytes]
    } else {
        Vec::new()
    };
    if source == "journal" {
        journal(
            &mut config,
            options
                .get("--journal")
                .copied()
                .ok_or("journal source needs --journal RUN_ID")?,
        )?;
    }

    let latency = options
        .get("--latency-ns")
        .copied()
        .unwrap_or("0")
        .parse::<i64>()?;
    config
        .loopback
        .insert(format!("engine-{}/orders", config.region), latency);
    config
        .loopback
        .insert(format!("exch-sim-{}/exec", config.region), latency);
    if source == "archive" || options.contains_key("--record") {
        let mut settings = ergon_runtime::Settings::from_env();
        if settings.app.is_empty() {
            settings.app = format!("engine-{}", config.region);
        }
        if let Some(run) = options.get("--record") {
            settings.run = (*run).into();
        }
        settings.sim_start = config.from;
        // The simulation owns it and runs its conductor.
        let bus = ergon_runtime::bus::Bus::connect(&settings)?;
        if options.contains_key("--record") {
            config.persist = Some(ergon_runtime::persist::Persist::connect(
                schema::TRADING_SCHEMA,
                &bus,
                settings,
            )?);
        }
        config.bus = Some(bus);
    }
    let logs = match source {
        "file" => file_logs,
        "journal" => {
            config.loopback.clear();
            Vec::new()
        }
        "archive" => {
            config.archive = Some(ArchiveConfig {
                control: std::env::var("ARCHIVE_CONTROL").unwrap_or_else(|_| "aeron:ipc".into()),
                response: std::env::var("ARCHIVE_RESPONSE").unwrap_or_else(|_| "aeron:ipc".into()),
                replay_stream: 1001,
            });
            Vec::new()
        }
        "clickhouse" => {
            clickhouse(&mut config)?;
            Vec::new()
        }
        _ => return Err(format!("unknown source {source}").into()),
    };
    let output = if source == "journal" {
        let mut sim = ergon_runtime::rt::sim::Sim::new(config, logs)?;
        let mut engine = engine::agent::Engine::new(sim.ctx(), streams)?;
        sim.run(&mut engine)?;
        sim.ctx().captured().to_bytes()
    } else {
        engine::backtest::execute(config, streams, logs)?
    };
    if let Some(path) = options.get("--output") {
        std::fs::write(path, &output)?;
    }
    let frames = ergon_runtime::frames::parse(&output)?;
    println!("backtest: {} output frames", frames.records.count());
    Ok(())
}

#[cfg(feature = "clickhouse")]
fn clickhouse(config: &mut SimConfig) -> Result<(), Box<dyn std::error::Error>> {
    config.clickhouse = Some(ergon_runtime::clickhouse_source::ClickHouseConfig {
        client: client(),
        table: std::env::var("CLICKHOUSE_FRAMES_TABLE").unwrap_or_else(|_| "frame".into()),
    });
    Ok(())
}

#[cfg(not(feature = "clickhouse"))]
fn clickhouse(_config: &mut SimConfig) -> Result<(), Box<dyn std::error::Error>> {
    Err("ClickHouse source needs --features clickhouse".into())
}

/// The ingester's database (`CLICKHOUSE_DATABASE`, its default `md`), where
/// `frame` and `input` land, as the ingester's user (`lab` by default).
#[cfg(feature = "clickhouse")]
fn client() -> ergon_runtime::clickhouse::ClickHouse {
    client_in(&std::env::var("CLICKHOUSE_DATABASE").unwrap_or_else(|_| "md".into()))
}

#[cfg(feature = "clickhouse")]
fn client_in(database: &str) -> ergon_runtime::clickhouse::ClickHouse {
    ergon_runtime::clickhouse::ClickHouse::new(
        &std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".into()),
        &std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "lab".into()),
        &std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| "lab".into()),
        database,
    )
}

#[cfg(feature = "clickhouse")]
fn measured(
    streams: &Streams,
    config: &SimConfig,
    name: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let (service, _) = name.split_once('/').ok_or("route needs SERVICE/KIND")?;
    let region = &streams
        .services
        .get(service)
        .ok_or("unknown route service")?
        .region;
    let venue = service.trim_start_matches("md-").to_uppercase();
    // Metrics have a database of their own (`tables.yaml`: `metrics`).
    let metrics = client_in(
        &std::env::var("CLICKHOUSE_METRICS_DATABASE").unwrap_or_else(|_| "metrics".into()),
    );
    Ok(lab::route_delay(&metrics, &venue, region, &config.region)?.0)
}

#[cfg(not(feature = "clickhouse"))]
fn measured(
    _streams: &Streams,
    _config: &SimConfig,
    _name: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    Err("measured route needs --features clickhouse".into())
}

#[cfg(feature = "clickhouse")]
fn journal(config: &mut SimConfig, run: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = client();
    let journal = ergon_runtime::journal::clickhouse::load(
        &client,
        &std::env::var("CLICKHOUSE_INPUT_TABLE").unwrap_or_else(|_| "input".into()),
        run,
    )?;
    config.from = journal.inputs.first().map(|input| input.ts);
    config.journal = Some(journal);
    clickhouse(config)
}

#[cfg(not(feature = "clickhouse"))]
fn journal(_config: &mut SimConfig, _run: &str) -> Result<(), Box<dyn std::error::Error>> {
    Err("journal source needs --features clickhouse".into())
}
