//! How long `Persist::record` takes on the application's thread, while the
//! writer thread is inserting into ClickHouse (`just test` starts one on
//! :18123). Run `just latency`, which also runs the timing-only control.

use std::time::{Duration, Instant};

use persist::{ClickHouse, Persist, Settings};

#[path = "../tests/support/v1.rs"]
mod v1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join("persist-latency");
    std::fs::create_dir_all(&dir)?;
    let config = dir.join("tables.yaml");
    std::fs::write(&config, "tables:\n  shapes: { kind: dynamic }\n")?;
    let ch = ClickHouse::new("http://localhost:18123", "lab", "lab", "persist_latency");
    let (persist, writer) = Persist::start(v1::SCHEMA, Settings::new(ch, config))?;
    // `control` times an empty loop: the floor this machine can measure.
    let control = std::env::args().nth(1).as_deref() == Some("control");

    // One record every 5 µs (200k/s, far above the lab's live rate) for 8 s.
    // The first 3 s are skipped: the buffers' pages are touched for the
    // first time then, which is a one-off cost.
    let mut samples = Vec::with_capacity(1_000_000);
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        let t = Instant::now();
        if !control {
            persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
        }
        let took = t.elapsed();
        if started.elapsed() > Duration::from_secs(3) {
            samples.push(took);
        }
        while t.elapsed() < Duration::from_micros(5) {}
    }
    writer.stop();
    samples.sort();
    let at = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize];
    println!(
        "{} {} records, dropped {}: p50 {:?}  p99 {:?}  p99.9 {:?}  max {:?}",
        if control { "control" } else { "record " },
        samples.len(),
        persist.dropped(),
        at(0.5),
        at(0.99),
        at(0.999),
        at(1.0)
    );
    Ok(())
}
