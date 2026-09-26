//! The ingester: Aeron Archive -> ClickHouse, once a second.
//!
//! `PERSIST_SCHEMA` (`schema/market.xml`) is the SBE schema the applications
//! record with; the rest of the settings come from [`Settings::from_env`].
//! An archive failure ends the process with an error: whatever restarts it
//! (Kubernetes, here) reconnects, and it resumes from its checkpoints.

use std::time::{Duration, Instant};

use persist_server::{Ingester, Settings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let schema_path =
        std::env::var("PERSIST_SCHEMA").unwrap_or_else(|_| "schema/market.xml".into());
    let schema =
        std::fs::read_to_string(&schema_path).map_err(|e| format!("{schema_path}: {e}"))?;
    let mut ingester = Ingester::connect(&schema, Settings::from_env())?;
    loop {
        let started = Instant::now();
        ingester.tick()?;
        std::thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()));
    }
}
