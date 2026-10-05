//! The lab's ingester: Aeron Archive -> `ClickHouse`, every
//! `PERSIST_INTERVAL` (`1s`). Each table gets at most one insert per
//! interval from each ingester, and every insert is a part `ClickHouse`
//! keeps in memory until it is merged away: a longer interval means fewer,
//! larger parts.
//!
//! `PERSIST_SCHEMAS` (`schema/`) is a directory of SBE schemas, one `.xml`
//! each. It is read at start-up, so after adding or updating a schema,
//! restart the ingester; it resumes from its checkpoint. Each frame is
//! matched by the schema id and template id in its header against that XML.
//!
//! The archive records the persist stream and, through a spy, every
//! archived feed `PERSIST_STREAMS` (`config/streams.yaml`) has published on
//! this node (`HOST_IP`). A feed added to the registry is recorded from the
//! next tick, with no restart.
//!
//! An archive failure ends the process with an error: whatever restarts it
//! (Kubernetes, here) reconnects, and it resumes from its checkpoints.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ergon_runtime_server::{Ingester, RecordedFeed, Settings};
use lab::Streams;

/// Every archived feed of `streams`, as recorded on the node at `host_ip`.
fn recorded(streams: &Streams, host_ip: &str) -> Result<Vec<RecordedFeed>, lab::streams::Error> {
    streams
        .archived()
        .map(|(service, kind, stream_id)| {
            Ok(RecordedFeed {
                service: service.into(),
                kind: kind.into(),
                stream_id,
                spy: streams.spy(service, host_ip)?,
            })
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let dir = std::env::var("PERSIST_SCHEMAS").unwrap_or_else(|_| "schema".into());
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|x| x == "xml"))
        .collect();
    paths.sort();
    let schemas = paths
        .iter()
        .map(|p| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display())))
        .collect::<Result<Vec<_>, _>>()?;
    for path in &paths {
        log::info!("schema: {}", path.display());
    }
    let schemas: Vec<&str> = schemas.iter().map(String::as_str).collect();
    let interval = std::env::var("PERSIST_INTERVAL").map_or(Ok(Duration::from_secs(1)), |v| {
        v.parse::<jiff::SignedDuration>()
            .ok()
            .and_then(|d| Duration::try_from(d).ok())
            .ok_or_else(|| format!("PERSIST_INTERVAL={v}: expected a duration such as 2s"))
    })?;
    let host_ip = std::env::var("HOST_IP").unwrap_or_else(|_| "127.0.0.1".into());
    let streams_path = lab::streams_path();
    let (feeds, mut watch) = if std::path::Path::new(&streams_path).exists() {
        let (streams, watch) = lab::Watch::start(&streams_path)?;
        (recorded(&streams, &host_ip)?, Some(watch))
    } else {
        (Vec::new(), None)
    };
    let mut ingester = Ingester::connect(
        &schemas,
        Settings {
            feeds,
            ..Settings::from_env()
        },
    )?;
    // SIGTERM stops it between ticks, its last inserts checkpointed. As a
    // container's first process it would otherwise ignore the signal and
    // wait out the grace period.
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop))?;
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        if let Some(streams) = watch.as_mut().and_then(lab::Watch::changed) {
            ingester.record_feeds(&recorded(&streams, &host_ip)?)?;
        }
        ingester.tick()?;
        std::thread::sleep(interval.saturating_sub(started.elapsed()));
    }
    log::info!("SIGTERM: stopped");
    Ok(())
}
