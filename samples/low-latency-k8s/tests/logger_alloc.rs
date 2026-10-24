//! What one event allocates on the calling thread: the count the docs
//! state (`src/logging.rs`, samples/README.md) is the one asserted here.
//!
//! The worker writes to `io::sink()`; allocations are counted, not frees
//! (the worker frees each line). The global counter would also see another
//! thread's allocations: the worker allocates nothing writing to a sink,
//! and the test runs alone (`--test-threads=1`).

use std::alloc::System;

use low_latency_k8s::logging::{LogConfig, non_blocking, subscriber};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

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
