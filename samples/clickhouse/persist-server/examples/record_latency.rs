//! How long recording takes on the application's thread, while an ingester
//! replays the archive into ClickHouse beside it. Needs what `just test`
//! starts (ClickHouse on :18123, the archive driver). `just latency` runs
//! every arm:
//!
//! * `control`   an empty loop: the floor this machine can measure
//! * `sbe`       `Persist::record` of one SBE message
//! * `installed` the same through `persist_client::record`, the installed handle
//! * `uninstalled` `persist_client::record` with no handle installed: a no-op
//! * `event`     `tracing::info!(table = "signal", …)`, table enabled
//! * `value`     `Persist::record_value` of a struct with the event's three fields
//! * `value-nested` `record_value` of a struct with a nested struct and five levels
//! * `event-off` the same event for a disabled table
//! * `no-table`  a `trace!` without a `table` field: persist's filter leaves it disabled
//! * `counter`, `gauge`, `histogram`  one update of a metric
//!
//! The metric and clock arms are below the timer's resolution, so each
//! sample times 100 of them (`x100` in the output).
//! * `clock-now` `Clock::now`; `clock-cached` `Clock::cached`; `system-time` `SystemTime::now`
//! * `poll-idle` `Metrics::poll` between intervals; `poll-due` with a 1 ms interval,
//!   so every interval publishes, one message a call
//! * `trace-off`, `trace-unsampled`, `trace-sampled`  a 4-stage checkpoint trace
//!   (start, 4 marks, an attribute, finish) with `otel_traces` off, on but not
//!   sampled, and every one published
//! * `span-on`, `span-off`  a `tracing` span entered and closed

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use persist_client::Persist;
use persist_client::clock::{Clock, Nanos};
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
    let arm = std::env::args().nth(1).unwrap_or_else(|| "sbe".into());
    // Traces and spans are on for this app except in the `-off` arms.
    let traces = match arm.as_str() {
        "trace-off" | "span-off" => "enabled: false",
        "trace-unsampled" => "enabled: true, traces: { t2t: { sample: 0 } }",
        _ => "enabled: true",
    };
    std::fs::write(
        &config,
        format!(
            "tables:\n  shapes: {{ kind: dynamic }}\n  signal: {{ kind: dynamic }}\n  book: {{ kind: dynamic }}\n  quiet: {{ kind: dynamic, enabled: false }}\n  otel_traces: {{ kind: static, {traces} }}\n"
        ),
    )?;

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
                Ingester::connect(&[v1::SCHEMA], settings).map_err(|e| e.to_string())?;
            let _ = ready_tx.send(());
            while !stop.load(Ordering::Relaxed) {
                ingester.tick().map_err(|e| e.to_string())?;
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
            app: "latency".into(),
            metrics_interval: if arm == "poll-due" {
                Duration::from_millis(1)
            } else {
                Duration::from_secs(5)
            },
            ..persist_client::Settings::new(&config)
        },
    )?;
    while !persist.is_connected() {
        std::thread::sleep(Duration::from_millis(10));
    }
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()))?;
    if arm == "installed" {
        persist.install();
    }
    let metrics = persist.metrics();
    let counter = metrics.counter("latency_counter", &[("arm", "counter")]);
    let gauge = metrics.gauge("latency_gauge", &[]);
    let histogram = metrics.histogram("latency_histogram", &[]);
    // Series for `poll-due` to publish: ten counters and a histogram.
    let others: Vec<_> = (0..10)
        .map(|i| metrics.counter("latency_other", &[("i", &i.to_string())]))
        .collect();
    let clock = Clock::new();
    let tracer = persist.tracer("t2t", &["wire", "decode", "decide", "send"], &["levels"]);
    while (arm.starts_with("trace-") || arm.starts_with("span-"))
        && tracer.is_on() != !arm.ends_with("-off")
    {
        std::thread::sleep(Duration::from_millis(10));
    }

    let signal = Signal {
        instrument: "BTCUSDT",
        edge: 0.25,
        n: 3,
    };
    let book = Book {
        instrument: "BTCUSDT",
        spread: Spread { bps: 1.5, ticks: 2 },
        bids: [100.0, 99.5, 99.0, 98.5, 98.0]
            .map(|price| Level { price, size: 1.25 })
            .to_vec(),
    };

    // One record every 5 µs (200k/s, far above the lab's live rate) for 8 s.
    // The first 3 s are skipped: pages are touched for the first time then,
    // which is a one-off cost.
    let reps = match arm.as_str() {
        "counter" | "gauge" | "histogram" | "clock-now" | "clock-cached" | "system-time" => 100,
        _ => 1,
    };
    let mut samples = Vec::with_capacity(1_000_000);
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        clock.now(); // the loop's one clock read, as a duty cycle would
        let t = Instant::now();
        for i in 0..reps {
            match arm.as_str() {
                "sbe" => persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?,
                "installed" | "uninstalled" => {
                    persist_client::record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?
                }
                "event" => {
                    tracing::info!(table = "signal", instrument = "BTCUSDT", edge = 0.25, n = 3)
                }
                "value" => persist.record_value("signal", &signal),
                "value-nested" => persist.record_value("book", &book),
                "no-table" => tracing::trace!(x = 1),
                "event-off" => {
                    tracing::info!(table = "quiet", instrument = "BTCUSDT", edge = 0.25, n = 3)
                }
                "counter" => counter.inc(),
                "gauge" => gauge.set(i as f64),
                "histogram" => histogram.record(850 + (i as u64 & 63) * 64),
                "clock-now" => {
                    let _ = std::hint::black_box(clock.now());
                }
                "clock-cached" => {
                    let _ = std::hint::black_box(clock.cached());
                }
                "system-time" => {
                    let _ = std::hint::black_box(std::time::SystemTime::now());
                }
                "poll-idle" | "poll-due" => {
                    others[0].inc();
                    histogram.record(850);
                    metrics.poll(clock.cached());
                }
                "trace-off" | "trace-unsampled" | "trace-sampled" => {
                    let mut t = tracer.start(Nanos(1_000), tracer.next_id());
                    for at in [1_100, 1_300, 1_600, 2_000] {
                        t.mark(Nanos(at));
                    }
                    t.attr(0, 10);
                    t.finish();
                }
                "span-on" | "span-off" => tracing::info_span!("work", n = 3).in_scope(|| {}),
                _ => {}
            }
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
        "{arm:11} {} records{}, dropped {}: p50 {:?}  p99 {:?}  p99.9 {:?}  max {:?}",
        samples.len(),
        if reps > 1 {
            format!(" x{reps}")
        } else {
            String::new()
        },
        persist.dropped(),
        at(0.5),
        at(0.99),
        at(0.999),
        at(1.0)
    );
    Ok(())
}

#[derive(serde::Serialize)]
struct Signal {
    instrument: &'static str,
    edge: f64,
    n: i64,
}

#[derive(serde::Serialize)]
struct Book {
    instrument: &'static str,
    spread: Spread,
    bids: Vec<Level>,
}

#[derive(serde::Serialize)]
struct Spread {
    bps: f64,
    ticks: u32,
}

#[derive(Clone, serde::Serialize)]
struct Level {
    price: f64,
    size: f64,
}
