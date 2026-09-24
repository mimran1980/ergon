//! How long `Persist::record` takes on the application's thread, while the
//! writer thread is inserting into ClickHouse (`just test` starts one on
//! :18123). Run `just latency`, which also runs the timing-only control.

use std::time::{Duration, Instant};

use persist::{ClickHouse, Persist, Settings};

#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
mod v1 {
    include!(concat!(env!("OUT_DIR"), "/shapes_v1.rs"));
}

fn message(buf: &mut [u8]) -> Result<usize, v1::sbe_rt::EncodeError> {
    Ok(v1::ShapesEncoder::wrap_and_apply_header(buf, 0)
        .fixed(&v1::ShapesFixedFields {
            ts: 1_700_000_000_000_000_000,
            i8: 1,
            i16: 2,
            i32: 3,
            i64: 4,
            u8: 5,
            u16: 6,
            u32: 7,
            u64: 8,
            f32: 1.5,
            f64: 2.5,
            opt_i32: None,
            opt_u64: Some(9),
            opt_f64: None,
            colour: v1::Colour::Green,
            code: *b"ABCDEF",
        })
        .entries(2, |e| {
            e.add_struct(&v1::EntriesEntry {
                qty: 1,
                side: v1::Colour::Red,
                maybe: 0.5,
            })?;
            e.add_struct(&v1::EntriesEntry {
                qty: 2,
                side: v1::Colour::Green,
                maybe: 1.5,
            })?;
            Ok(())
        })?
        .note(b"BTCUSDT")?
        .encoded_length_with_header())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join("persist-latency");
    std::fs::create_dir_all(&dir)?;
    let config = dir.join("tables.yaml");
    std::fs::write(&config, "tables:\n  shapes: { kind: dynamic }\n")?;
    let settings = Settings {
        clickhouse: ClickHouse::new("http://localhost:18123", "lab", "lab", "persist_latency"),
        config_path: config,
        max_buffered_bytes: 256 << 20,
        flush_interval: Duration::from_secs(1),
        recheck: Duration::from_secs(30),
    };
    let (persist, writer) =
        Persist::start(include_str!("../tests/schemas/shapes_v1.xml"), settings)?;
    let id = v1::ShapesEncoder::TEMPLATE_ID;
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
            persist.record(id, message)?;
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
