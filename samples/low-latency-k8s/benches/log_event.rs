//! The cost of one `warn!` on the calling thread, through the sample's
//! logging (lossy non-blocking writer): to a pipe that is drained (the
//! worker keeps up), to `/dev/null`, and to a pipe nobody reads (the worker
//! is blocked in `write(2)` and every line is dropped). Prints p50, p90,
//! p99, p99.9, p99.99 and max in ns (quanta's counter), then the window's
//! page faults, context switches, steal and interrupts on the bench CPU and
//! the CPU's idle stall over the same wall time, and the max's cause from
//! them (`low_latency_k8s::costs`), or "unexplained".
//!
//! It checks first that every event was either written or counted as
//! dropped, and passes the fields through `black_box`. `--smoke` runs 1,000
//! events instead of 200,000. A figure here is the formatting and enqueue
//! cost on this machine, not a latency of the sample.

use std::hint::black_box;
use std::io::Write;
use std::time::Duration;

use low_latency_k8s::costs::{Counters, Spread, Window, idle_gaps, idle_window_ns, snapshot};
use low_latency_k8s::logging::{LogConfig, non_blocking, subscriber};
use tracing_appender::non_blocking::WorkerGuard;

type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

struct Run {
    spread: Spread,
    window: Window,
    dropped: usize,
    guard: WorkerGuard,
}

/// `prefill` events, then `n` timed ones, into `sink` from a thread named
/// `ll-hot`.
fn run<W: Write + Send + 'static>(
    sink: W,
    n: usize,
    queue_lines: usize,
    prefill: usize,
) -> BenchResult<Run> {
    let cfg = LogConfig {
        queue_lines,
        thread_name: "ll-log-bench".to_owned(),
        ..LogConfig::default()
    };
    let a = non_blocking(sink, &cfg);
    let (nb, guard, counter) = (a.writer, a.guard, a.dropped);
    let probe = counter;
    let hot = std::thread::Builder::new()
        .name("ll-hot".to_owned())
        .spawn(move || {
            let _default = tracing::subscriber::set_default(subscriber(nb, &cfg));
            let pad = "x".repeat(160);
            // paced, so the worker writes until the pipe is full whatever
            // the timing
            for i in 0..prefill {
                tracing::warn!(i, pad = %pad, "fill");
                if i % 100 == 99 {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            if prefill > 0 {
                // the worker blocks in write(2) once the pipe is full
                std::thread::sleep(Duration::from_millis(200));
            }
            let before = probe.dropped_lines();
            let cpu = low_latency_k8s::cpus::current_cpu().unwrap_or(0);
            let snap0 = snapshot(cpu).ok();
            let clock = quanta::Clock::new();
            // written once first: an untouched page would fault inside the window
            let mut costs = vec![u64::MAX; n];
            let t_start = clock.raw();
            for (k, seq) in (0..n as u64).enumerate() {
                let t0 = clock.raw();
                tracing::warn!(
                    seq = black_box(seq),
                    waited_ns = black_box(1_234_u64),
                    "offer held back"
                );
                costs[k] = clock.delta_as_nanos(t0, clock.raw());
            }
            let wall = clock.delta_as_nanos(t_start, clock.raw());
            let snap1 = snapshot(cpu).ok();
            let dropped = probe.dropped_lines() - before;
            let (idle_gap_max, idle_gaps_over_10us) = idle_gaps(&clock, idle_window_ns(wall));
            let window = Window {
                counters: snap0.zip(snap1).map(|(a, b)| Counters::between(&a, &b)),
                idle_gap_max,
                idle_gaps_over_10us,
            };
            (Spread::of(&mut costs), window, dropped)
        })?;
    let (spread, window, dropped) = hot.join().map_err(|_| "the bench thread panicked")?;
    Ok(Run {
        spread,
        window,
        dropped,
        guard,
    })
}

fn report(label: &str, r: &Run) {
    println!(
        "log_event\t{label}\tdropped={}\t{}",
        r.dropped,
        r.spread.fields()
    );
    println!("log_event\t{label}\twindow\t{}", r.window.fields());
    println!(
        "log_event\t{label}\tmax_cause\t{}",
        r.window.cause(&r.spread)
    );
}

fn main() -> BenchResult<()> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    let n = if smoke { 1_000 } else { 200_000 };

    // A drained pipe: every event must arrive or be counted.
    let (mut reader, writer) = std::io::pipe()?;
    let drain = std::thread::spawn(move || {
        let mut buf = vec![0_u8; 65_536];
        let mut lines = 0;
        while let Ok(k) = std::io::Read::read(&mut reader, &mut buf) {
            if k == 0 {
                break;
            }
            #[allow(clippy::naive_bytecount)] // a bench helper, not the measured path
            let n = buf[..k].iter().filter(|&&b| b == b'\n').count();
            lines += n;
        }
        lines
    });
    let drained = run(writer, n, 4096, 0)?;
    report("drained_pipe", &drained);
    let dropped = drained.dropped;
    drop(drained.guard);
    let written = drain.join().map_err(|_| "the drain panicked")?;
    if written + dropped != n {
        return Err(format!("{written} written + {dropped} dropped != {n} events").into());
    }

    let dev_null = std::fs::OpenOptions::new().write(true).open("/dev/null")?;
    let null = run(dev_null, n, 4096, 0)?;
    report("dev_null", &null);
    drop(null.guard);

    // A pipe nobody reads: the queue is full, every timed event is dropped.
    let (reader, writer) = std::io::pipe()?;
    let blocked = run(writer, n, 256, 4_000)?;
    report("blocked_pipe", &blocked);
    if blocked.dropped != n {
        return Err(format!("{} of {n} dropped with the worker blocked", blocked.dropped).into());
    }
    drop(reader); // unblock the worker before the guard waits on it
    drop(blocked.guard);
    Ok(())
}
