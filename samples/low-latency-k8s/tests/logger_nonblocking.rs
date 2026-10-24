//! The non-blocking writer under a stdout that never drains: the caller
//! never stalls, the lines past the queue are dropped and counted, and the
//! cost of one event stays bounded. Also: lines reach an unblocked writer
//! whole and in order, with the thread's name.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

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

fn percentile(sorted: &[u64], q: f64) -> u64 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let i = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[i.min(sorted.len() - 1)]
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
    let (nb, guard, dropped) = non_blocking(writer, &cfg);
    let _default = tracing::subscriber::set_default(subscriber(nb, &cfg));
    // Fill the pipe and the queue: ~200-byte lines, well past 64 KiB.
    let pad = "x".repeat(160);
    for i in 0..2_000 {
        tracing::warn!(i, pad = %pad, "fill");
    }
    std::thread::sleep(Duration::from_millis(300));
    #[cfg(target_os = "linux")]
    assert_eq!(
        worker_state("ll-log-blocked"),
        Some('S'),
        "the worker should be asleep in write(2)"
    );
    let before = dropped.dropped_lines();
    let mut costs = Vec::with_capacity(N);
    for i in 0..N {
        let t = Instant::now();
        tracing::warn!(i, pad = %pad, "blocked");
        costs.push(u64::try_from(t.elapsed().as_nanos())?);
    }
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
    costs.sort_unstable();
    let max = costs[costs.len() - 1];
    println!(
        "logger: one warn! with stdout blocked: p50={} p99={} p99.9={} max={} ns (n={N}, dropped={lost})",
        percentile(&costs, 0.50),
        percentile(&costs, 0.99),
        percentile(&costs, 0.999),
        max
    );
    // A stall guard, not a performance claim.
    assert!(max < 10_000_000, "one event took {max} ns");
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
    let (nb, guard, dropped) = non_blocking(writer, &cfg);
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
