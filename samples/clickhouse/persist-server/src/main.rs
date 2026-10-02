//! The ingester: Aeron Archive -> ClickHouse, every `PERSIST_INTERVAL`
//! (`1s`). Each table gets at most one insert per interval from each
//! ingester, and every insert is a part ClickHouse keeps in memory until it
//! is merged away: a longer interval means fewer, larger parts.
//!
//! `PERSIST_SCHEMAS` (`schema/`) is a directory of SBE schemas, one `.xml`
//! each. It is read at start-up, so after adding or updating a schema,
//! restart the ingester; it resumes from its checkpoint. Each frame is
//! matched by the schema id and template id in its header against that XML.
//! Application code decodes the compiled schemas with `AnySchemaMessage`.
//! This process does not: it builds tables from the XML. The rest of the
//! settings come from [`Settings::from_env`].
//!
//! An archive failure ends the process with an error: whatever restarts it
//! (Kubernetes, here) reconnects, and it resumes from its checkpoints.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use persist_server::{Ingester, Settings};

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
    let mut ingester = Ingester::connect(&schemas, Settings::from_env()?)?;
    // A service added to the registry is recorded from the next tick.
    let streams = std::env::var("PERSIST_STREAMS").unwrap_or_else(|_| "config/streams.yaml".into());
    if std::path::Path::new(&streams).exists() {
        ingester.follow(streams);
    }
    // SIGTERM stops it between ticks, its last inserts checkpointed. As a
    // container's first process it would otherwise ignore the signal and
    // wait out the grace period.
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop))?;
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        ingester.tick()?;
        std::thread::sleep(interval.saturating_sub(started.elapsed()));
    }
    log::info!("SIGTERM: stopped");
    Ok(())
}
