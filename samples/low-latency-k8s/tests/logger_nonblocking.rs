//! The non-blocking writer under a stdout that never drains: the lines past
//! the queue are dropped and counted, and the caller makes no voluntary
//! context switch (Linux). The cost of one event is printed p50 to max with
//! what explains the max (`low_latency_k8s::costs`). The pinned, real-fd-1
//! proof is `tests/hot_stdout.rs`. Also: lines reach an unblocked writer
//! whole and in order, with the thread's name, and failed writes are
//! counted.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use low_latency_k8s::costs::{Counters, Spread, Window, idle_gaps, idle_window_ns};
use low_latency_k8s::logging::{LogConfig, non_blocking, subscriber};
use tracing::level_filters::LevelFilter;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Exits the process (failing the test) if the test has not finished in
/// `limit`: finishing is the proof that the caller never blocked.
fn watchdog(limit: Duration) -> Arc<AtomicBool> {
    let done = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&done);
    std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < limit {
            if flag.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        eprintln!("watchdog: the logging caller blocked for {limit:?}");
        std::process::exit(101);
    });
    done
}

/// The worker's state letter from `/proc/self/task/<tid>/stat`.
#[cfg(target_os = "linux")]
fn worker_state(name: &str) -> Option<char> {
    for entry in std::fs::read_dir("/proc/self/task").ok()? {
        let dir = entry.ok()?.path();
        let comm = std::fs::read_to_string(dir.join("comm")).ok()?;
        if comm.trim_end() == name {
            let stat = std::fs::read_to_string(dir.join("stat")).ok()?;
            let after = stat.rsplit_once(") ")?.1;
            return after.chars().next();
        }
    }
    None
}

#[test]
fn a_blocked_stdout_never_stalls_the_caller() -> TestResult {
    const LIMIT: usize = 256;
    const N: usize = 10 * LIMIT;
    let done = watchdog(Duration::from_secs(30));
    // A pipe whose read end is never read: once its buffer (64 KiB on
    // Linux) is full, the worker blocks in write(2), as on a wedged stdout.
    let (reader, writer) = std::io::pipe()?;
    let cfg = LogConfig {
        level: LevelFilter::INFO,
        queue_lines: LIMIT,
        thread_name: "ll-log-blocked".to_owned(),
    };
    let a = non_blocking(writer, &cfg);
    let (nb, guard, dropped) = (a.writer, a.guard, a.dropped);
    let _default = tracing::subscriber::set_default(subscriber(nb, &cfg));
    // Fill the pipe and the queue: ~200-byte lines, well past 64 KiB.
    let pad = "x".repeat(160);
    // paced, so the worker writes until the pipe is full whatever the
    // timing: 40 x 100 lines, ~800 KiB, past the pipe and the queue
    for i in 0..4_000 {
        tracing::warn!(i, pad = %pad, "fill");
        if i % 100 == 99 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    std::thread::sleep(Duration::from_millis(300));
    #[cfg(target_os = "linux")]
    assert_eq!(
        worker_state("ll-log-blocked"),
        Some('S'),
        "the worker should be asleep in write(2)"
    );
    let before = dropped.dropped_lines();
    let cpu = low_latency_k8s::cpus::current_cpu().unwrap_or(0);
    let snap0 = low_latency_k8s::costs::snapshot(cpu).ok();
    let clock = quanta::Clock::new();
    // written once first: an untouched page would fault inside the window
    let mut costs = vec![u64::MAX; N];
    let t_start = clock.raw();
    for (k, i) in (0..N).enumerate() {
        let t0 = clock.raw();
        tracing::warn!(i, pad = %pad, "blocked");
        costs[k] = clock.delta_as_nanos(t0, clock.raw());
    }
    let wall = clock.delta_as_nanos(t_start, clock.raw());
    let snap1 = low_latency_k8s::costs::snapshot(cpu).ok();
    let lost = dropped.dropped_lines() - before;
    // At most the queue's worth gets through. macOS grows a pipe's buffer
    // (16 to 64 KiB) while a writer waits, so a few more lines can drain
    // there; on Linux the worker is asleep in write(2) (checked above).
    let slack = if cfg!(target_os = "linux") {
        LIMIT
    } else {
        2 * LIMIT
    };
    assert!(
        lost >= N - slack,
        "dropped {lost} of {N} with the worker blocked"
    );
    let spread = Spread::of(&mut costs);
    let (idle_gap_max, idle_gaps_over_10us) = idle_gaps(&clock, idle_window_ns(wall));
    let window = Window {
        counters: snap0.zip(snap1).map(|(a, b)| Counters::between(&a, &b)),
        idle_gap_max,
        idle_gaps_over_10us,
    };
    println!(
        "logger: one warn! with stdout blocked (unpinned caller): {} dropped={lost}\nlogger: window {}\nlogger: max cause: {}",
        spread.fields(),
        window.fields(),
        window.cause(&spread)
    );
    // Linux: the caller never slept (the pinned proof: tests/hot_stdout.rs)
    if let Some(c) = &window.counters {
        assert_eq!(c.voluntary, 0, "the caller slept while logging");
    }
    done.store(true, Ordering::Relaxed);
    // Unblock the worker before the guard waits on it.
    drop(reader);
    drop(guard);
    Ok(())
}

#[test]
fn lines_reach_stdout_whole_and_in_order() -> TestResult {
    let done = watchdog(Duration::from_secs(30));
    let (mut reader, writer) = std::io::pipe()?;
    let drain = std::thread::spawn(move || {
        let mut text = String::new();
        reader.read_to_string(&mut text).map(|_| text)
    });
    let cfg = LogConfig {
        thread_name: "ll-log-order".to_owned(),
        ..LogConfig::default()
    };
    let a = non_blocking(writer, &cfg);
    let (nb, guard, dropped) = (a.writer, a.guard, a.dropped);
    let named = std::thread::Builder::new()
        .name("ll-hot".to_owned())
        .spawn(move || {
            let _default = tracing::subscriber::set_default(subscriber(nb, &cfg));
            for i in 0..100 {
                tracing::warn!(i, "ordered");
            }
            tracing::debug!("filtered out");
        })?;
    named.join().map_err(|_| "the logging thread panicked")?;
    drop(guard);
    let text = drain.join().map_err(|_| "the drain panicked")??;
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 100, "{text}");
    for (i, line) in lines.iter().enumerate() {
        // fmt pads thread names to the longest seen so far
        let words: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(words.get(1..3), Some(&["WARN", "ll-hot"][..]), "{line}");
        assert!(line.ends_with(&format!("ordered i={i}")), "{line}");
        assert!(!line.contains('\x1b'), "ANSI in {line}");
    }
    assert_eq!(dropped.dropped_lines(), 0);
    done.store(true, Ordering::Relaxed);
    Ok(())
}

/// A stdout whose every write fails, as a closed pipe (EPIPE) or a dead
/// terminal (EIO) would.
struct Broken;

impl std::io::Write for Broken {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn failed_writes_are_counted_apart_from_queue_drops() {
    let done = watchdog(Duration::from_secs(30));
    let cfg = LogConfig {
        thread_name: "ll-log-broken".to_owned(),
        ..LogConfig::default()
    };
    let appender = non_blocking(Broken, &cfg);
    let (dropped, write_errors) = (appender.dropped.clone(), appender.write_errors.clone());
    {
        let _default = tracing::subscriber::set_default(subscriber(appender.writer, &cfg));
        for i in 0..50 {
            tracing::warn!(i, "lost to a broken stdout");
        }
    }
    // the guard's drop waits for the worker to drain the queue
    drop(appender.guard);
    println!(
        "logger: broken stdout: write_errors={} dropped_lines={}",
        write_errors.get(),
        dropped.dropped_lines()
    );
    // tracing-appender discards a failed write; the wrapper counts it
    assert_eq!(write_errors.get(), 50);
    assert_eq!(dropped.dropped_lines(), 0, "the queue never filled");
    done.store(true, Ordering::Relaxed);
}
