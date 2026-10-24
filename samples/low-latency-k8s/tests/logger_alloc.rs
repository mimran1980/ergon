//! What one event allocates on the calling thread: the count the docs
//! state (`src/logging.rs`, samples/README.md) is the one asserted here.
//!
//! The worker writes to `io::sink()`; allocations are counted, not frees
//! (the worker frees each line). The global counter also sees the worker's
//! allocations: writing to a sink it makes none, but the first time it
//! blocks in the channel's `recv` it makes two (crossbeam's 48-byte
//! `Context` and its 96-byte waker list; seen once, mid-run, when a loaded
//! machine had not yet scheduled the worker). So the test waits until the
//! worker sleeps before it measures, and runs alone (`--test-threads=1`).

use std::alloc::System;
use std::time::{Duration, Instant};

use low_latency_k8s::logging::{LogConfig, non_blocking, subscriber};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Waits until this process's thread `name` sleeps (Linux: `S` in its
/// `stat`, up to 5 s); elsewhere waits 200 ms.
fn wait_asleep(name: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while cfg!(target_os = "linux") && Instant::now() < deadline {
        let asleep = std::fs::read_dir("/proc/self/task")
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| {
                let dir = e.path();
                std::fs::read_to_string(dir.join("comm")).is_ok_and(|c| c.trim_end() == name)
                    && std::fs::read_to_string(dir.join("stat"))
                        .is_ok_and(|s| s.rsplit_once(") ").is_some_and(|(_, r)| r.starts_with('S')))
            });
        if asleep {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(200));
}

/// Allocations made by `f`.
fn allocations(f: impl FnOnce()) -> usize {
    let region = Region::new(GLOBAL);
    f();
    region.change().allocations
}

#[test]
fn one_warm_event_allocates_once_and_a_disabled_one_never() -> Result<(), Box<dyn std::error::Error>>
{
    let cfg = LogConfig {
        thread_name: "ll-log-alloc".to_owned(),
        ..LogConfig::default()
    };
    let a = non_blocking(std::io::sink(), &cfg);
    let (nb, guard, dropped) = (a.writer, a.guard, a.dropped);
    let hot = std::thread::Builder::new()
        .name("ll-hot".to_owned())
        .spawn(move || {
            let _default = tracing::subscriber::set_default(subscriber(nb, &cfg));
            let cold = allocations(|| tracing::warn!(seq = 1_u64, "first event on this thread"));
            // the worker has written that line and blocked in recv once
            wait_asleep("ll-log-alloc");
            // the thread-local String is warm now
            let mut warm = Vec::new();
            for seq in 0..1_000_u64 {
                warm.push(allocations(|| {
                    tracing::warn!(seq, lost = 3_u64, "echo timeout");
                }));
            }
            let disabled = allocations(|| tracing::debug!(seq = 1_u64, "below the level"));
            (cold, warm, disabled)
        })?;
    let (cold, warm, disabled) = hot.join().map_err(|_| "the logging thread panicked")?;
    println!(
        "logger: allocations per event: first on a thread={cold}, warm={}, disabled={disabled}",
        warm[0]
    );
    assert!(
        warm.iter().all(|&n| n == 1),
        "warm events allocated {warm:?}"
    );
    assert_eq!(disabled, 0);
    assert!(cold >= 1);
    drop(guard);
    assert_eq!(dropped.dropped_lines(), 0);
    Ok(())
}
