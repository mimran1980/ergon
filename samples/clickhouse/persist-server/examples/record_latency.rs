//! How long recording takes on the application's thread, while an ingester
//! replays the archive into ClickHouse beside it. Needs what `just test`
//! starts (ClickHouse on :18123, the archive driver). `just latency` runs
//! every arm:
//!
//! * `control`   an empty loop: the floor this machine can measure
//! * `sbe`       `Persist::record` of one SBE message
//! * `event`     `tracing::info!(table = "signal", …)`, table enabled
//! * `event-off` the same event for a disabled table
//! * `no-table`  a `trace!` without a `table` field: persist's filter leaves it disabled

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use persist_client::Persist;
use persist_server::{ClickHouse, Ingester};
use tracing_subscriber::layer::SubscriberExt;

#[path = "../tests/support/v1.rs"]
mod v1;

const STREAM: i32 = 9_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aeron_dir =
        std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into());
    let dir = std::env::temp_dir().join(format!("persist-latency-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let config = dir.join("tables.yaml");
    std::fs::write(
        &config,
        "tables:\n  shapes: { kind: dynamic }\n  signal: { kind: dynamic }\n  quiet: { kind: dynamic, enabled: false }\n",
    )?;
    let arm = std::env::args().nth(1).unwrap_or_else(|| "sbe".into());

    // The ingester, on its own thread as it would be in its own process.
    let stop = Arc::new(AtomicBool::new(false));
    let ingester = {
        let (stop, aeron_dir, config) = (Arc::clone(&stop), aeron_dir.clone(), config.clone());
        let ch = ClickHouse::new("http://localhost:18123", "lab", "lab", "persist_latency");
        let settings = persist_server::Settings {
            aeron_dir: Some(aeron_dir),
            stream_id: STREAM,
            ..persist_server::Settings::new(ch, config, dir.join("checkpoint"))
        };
        let (ready_tx, ready) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || -> Result<(), String> {
            let mut ingester =
                Ingester::connect(v1::SCHEMA, settings).map_err(|e| e.to_string())?;
            let _ = ready_tx.send(());
            while !stop.load(Ordering::Relaxed) {
                ingester.tick();
                std::thread::sleep(Duration::from_secs(1));
            }
            Ok(())
        });
        ready.recv_timeout(Duration::from_secs(20))?;
        thread
    };
    let persist = Persist::connect(
        v1::SCHEMA,
        persist_client::Settings {
            aeron_dir: Some(aeron_dir),
            stream_id: STREAM,
            ..persist_client::Settings::new(&config)
        },
    )?;
    while !persist.is_connected() {
        std::thread::sleep(Duration::from_millis(10));
    }
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;

    // One record every 5 µs (200k/s, far above the lab's live rate) for 8 s.
    // The first 3 s are skipped: pages are touched for the first time then,
    // which is a one-off cost.
    let mut samples = Vec::with_capacity(1_000_000);
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        let t = Instant::now();
        match arm.as_str() {
            "sbe" => persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?,
            "event" => tracing::info!(table = "signal", instrument = "BTCUSDT", edge = 0.25, n = 3),
            "no-table" => tracing::trace!(x = 1),
            "event-off" => {
                tracing::info!(table = "quiet", instrument = "BTCUSDT", edge = 0.25, n = 3)
            }
            _ => {}
        }
        let took = t.elapsed();
        if started.elapsed() > Duration::from_secs(3) {
            samples.push(took);
        }
        while t.elapsed() < Duration::from_micros(5) {}
    }
    stop.store(true, Ordering::Relaxed);
    ingester.join().map_err(|_| "ingester panicked")??;
    samples.sort();
    let at = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize];
    println!(
        "{arm:9} {} records, dropped {}: p50 {:?}  p99 {:?}  p99.9 {:?}  max {:?}",
        samples.len(),
        persist.dropped(),
        at(0.5),
        at(0.99),
        at(0.999),
        at(1.0)
    );
    Ok(())
}
