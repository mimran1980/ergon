//! The ingester: Aeron Archive -> ClickHouse, once a second.
//!
//! `PERSIST_SCHEMAS` (`schema/`) is a directory of SBE schemas, one `.xml`
//! each: every message the applications record with them is ingested. It is
//! read at start-up, so after adding or updating a schema, restart the
//! ingester; it resumes from its checkpoint. The rest of the settings come
//! from [`Settings::from_env`].
//! An archive failure ends the process with an error: whatever restarts it
//! (Kubernetes, here) reconnects, and it resumes from its checkpoints.

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
    let mut ingester = Ingester::connect(&schemas, Settings::from_env())?;
    loop {
        let started = Instant::now();
        ingester.tick()?;
        std::thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()));
    }
}
