//! How long recording takes on the application's thread, while an ingester
//! replays the archive into `ClickHouse` beside it. Needs what `just test`
//! starts (`ClickHouse` on :18123, the archive driver). `just latency` runs
//! every arm:
//!
//! * `control`   an empty loop: the floor this machine can measure
//! * `control-x100` the amplified loop's floor, for the x100 arms
//! * `sbe`       `Persist::record` of one SBE message
//! * `event`     `tracing::info!(table = "signal", …)` through the bridge, table enabled
//! * `value`     `Persist::record_value` of a struct with the event's three fields
//! * `value-nested` `record_value` of a struct with a nested struct and five levels
//! * `event-off` the same event for a disabled table: the bridge switches
//!   nothing, so it costs what `event` does, and the ingester leaves it out
//! * `no-table`  a `trace!` without a `table` field: the bridge's filter leaves it disabled
//! * `counter`, `gauge`, `histogram`  one update of a metric handle
//!
//! Metric and clock samples time 100 operations (`x100` in the output).
//! * `clock-now` `Clock::now`; `clock-cached` `Clock::cached`; `system-time` `SystemTime::now`
//! * `poll-idle` `Persist::poll` between the 5 s interval. A histogram makes
//!   poll due every 1 ms. `poll-due` uses a 1 ms metrics interval, so every
//!   deadline also includes counters. Both arms measure a mixed duty cycle:
//!   most polls are idle, while due polls publish at most one metrics
//!   message, and up to 8 dictionary messages while the 5 s heartbeat round
//!   is under way.
//! * `trace-off`, `trace-unsampled`, `trace-sampled`  a 4-stage checkpoint trace
//!   (start, 4 marks, an attribute, finish) with `otel_traces` off, on but not
//!   sampled, and every one published
//! * `span-on`, `span-off`  a `tracing` span entered and closed, through the
//!   bridge, with `otel_traces` on and off: the bridge publishes both, and the
//!   ingester keeps only the first

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ergon_runtime::bus::Bus;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::persist::Persist;
use ergon_runtime_server::{ClickHouse, Ingester};
use tracing_subscriber::layer::SubscriberExt;

#[path = "../tests/support/v1.rs"]
mod v1;

const STREAM: i32 = 9_000;

#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aeron_dir =
        std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| env!("AERON_TEST_DIR").into());
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
        let settings = ergon_runtime_server::Settings {
            aeron_dir: Some(aeron_dir),
            stream_id: STREAM,
            ..ergon_runtime_server::Settings::new(ch, config, dir.join("checkpoint"))
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
    let settings = ergon_runtime::Settings {
        aeron_dir: Some(aeron_dir),
        stream_id: STREAM,
        app: "latency".into(),
        metrics_interval: if arm == "poll-due" {
            Duration::from_millis(1)
        } else {
            Duration::from_secs(5)
        },
        ..ergon_runtime::Settings::new(&config)
    };
    // The application's client: its conductor runs in this loop, outside
    // every timed operation.
    let bus = Bus::connect(&settings)?;
    let persist = Persist::connect(v1::SCHEMA, &bus, settings)?;
    while !persist.is_connected() {
        let _ = bus.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(persist.layer()?))?;
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
        && tracer.is_on() == arm.ends_with("-off")
    {
        let _ = bus.poll();
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

    // Dispatch once, outside the measured loop. Each closure is monomorphized:
    // string comparisons and a dynamic call are not part of operation timing.
    macro_rules! sample {
        ($reps:literal, |$i:ident| $operation:block) => {
            measure::<$reps>(&clock, &bus, &persist, |$i| {
                $operation;
                Ok(())
            })
            .map(|measurement| (measurement, $reps))
        };
    }
    let measured = match arm.as_str() {
        "control" => sample!(1, |i| {
            black_box(i);
        }),
        "control-x100" => sample!(100, |i| {
            black_box(i);
        }),
        "sbe" => sample!(1, |_i| {
            persist.record(v1::TEMPLATE_ID, v1::LEN, v1::encode)?;
        }),
        "event" => sample!(1, |_i| {
            tracing::info!(table = "signal", instrument = "BTCUSDT", edge = 0.25, n = 3);
        }),
        "value" => sample!(1, |_i| {
            persist.record_value("signal", black_box(&signal));
        }),
        "value-nested" => sample!(1, |_i| {
            persist.record_value("book", black_box(&book));
        }),
        "no-table" => sample!(1, |_i| {
            tracing::trace!(x = 1);
        }),
        "event-off" => sample!(1, |_i| {
            tracing::info!(table = "quiet", instrument = "BTCUSDT", edge = 0.25, n = 3);
        }),
        "counter" => sample!(100, |_i| {
            black_box(&counter).inc();
        }),
        "gauge" => sample!(100, |i| {
            black_box(&gauge).set(black_box(i as f64));
        }),
        "histogram" => sample!(100, |i| {
            black_box(&histogram).record(black_box(850 + (i as u64 & 63) * 64));
        }),
        "clock-now" => sample!(100, |_i| {
            black_box(black_box(&clock).now());
        }),
        "clock-cached" => sample!(100, |_i| {
            black_box(black_box(&clock).cached());
        }),
        "system-time" => sample!(100, |_i| {
            black_box(std::time::SystemTime::now());
        }),
        "poll-idle" | "poll-due" => sample!(1, |_i| {
            others[0].inc();
            histogram.record(850);
            persist.poll(clock.cached());
        }),
        "trace-off" | "trace-unsampled" | "trace-sampled" => sample!(1, |_i| {
            let mut t = tracer.start(Nanos(1_000), tracer.next_id());
            for at in [1_100, 1_300, 1_600, 2_000] {
                t.mark(Nanos(black_box(at)));
            }
            t.attr(0, 10);
            t.finish();
        }),
        "span-on" | "span-off" => sample!(1, |_i| {
            tracing::info_span!("work", n = 3).in_scope(|| {});
        }),
        _ => Err(format!("unknown latency arm: {arm}").into()),
    };
    stop.store(true, Ordering::Relaxed);
    ingester.join().map_err(|_| "ingester panicked")??;
    let ((mut samples, dropped), reps) = measured?;
    samples.sort();
    let at = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize];
    println!(
        "{arm:16} {} samples{}, dropped during measurement {dropped} (total {} including warm-up): batch p50 {:?}  p99 {:?}  p99.9 {:?}  max {:?}; batch p50/op {:.2} ns  p99/op {:.2} ns",
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
        at(1.0),
        at(0.5).as_secs_f64() * 1e9 / f64::from(reps),
        at(0.99).as_secs_f64() * 1e9 / f64::from(reps)
    );
    Ok(())
}

/// One sample every 5 µs (at most 200k/s). Amplified arms do REPS operations
/// per sample. Keep the ingester running, skip 3 s warm-up, measure for 5 s.
/// `bus`'s conductor runs between samples, untimed.
fn measure<const REPS: usize>(
    clock: &Clock,
    bus: &Bus,
    persist: &Persist,
    mut operation: impl FnMut(usize) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(Vec<Duration>, u64), Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(1_000_000);
    let started = Instant::now();
    let mut drops_at_measurement = None;
    while started.elapsed() < Duration::from_secs(8) {
        clock.now();
        let measuring = started.elapsed() > Duration::from_secs(3);
        if measuring && drops_at_measurement.is_none() {
            drops_at_measurement = Some(persist.dropped());
        }
        let t = Instant::now();
        for i in 0..REPS {
            operation(i)?;
        }
        let took = t.elapsed();
        if measuring {
            samples.push(took);
        }
        let _ = bus.poll();
        while t.elapsed() < Duration::from_micros(5) {}
    }
    let dropped = persist
        .dropped()
        .saturating_sub(drops_at_measurement.unwrap_or_else(|| persist.dropped()));
    Ok((samples, dropped))
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
