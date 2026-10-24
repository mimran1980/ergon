//! A pinned hot thread logging through the app's setup to the real fd 1
//! (`std::io::stdout()`, the writer `init_stdout` uses), with fd 1 swapped for
//! a pipe nobody reads, as a wedged `kubectl logs` reader or a full
//! container log pipe would leave it.
//!
//! Evidence, printed and asserted:
//!
//! - `ll-log`, placed on the helper CPU, is asleep in `write(2)` (state `S`);
//! - the hot thread, pinned to another CPU, makes no voluntary context
//!   switch over the timed events: it never sleeps on the wedged stdout;
//! - every timed event is dropped and counted;
//! - the cost of one event p50 to max (quanta's counter), with the window's
//!   page faults, context switches, steal and interrupts on the hot CPU and
//!   the CPU's idle stall over the same wall time, which name the max's
//!   cause or label it unexplained (`low_latency_k8s::costs`).
//!
//! Linux with at least two CPUs. One test in this binary: it swaps the
//! process's fd 1.
#![cfg(target_os = "linux")]

use std::time::Duration;

use low_latency_k8s::costs::{Counters, Spread, Window, idle_gaps, idle_window_ns, snapshot};
use low_latency_k8s::cpus::{CpuList, current_affinity, current_cpu, pin_current};
use low_latency_k8s::logging::{LogConfig, non_blocking, place_worker, subscriber};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The state letter of this process's thread named `name`.
fn state(name: &str) -> Option<char> {
    for entry in std::fs::read_dir("/proc/self/task").ok()? {
        let dir = entry.ok()?.path();
        if std::fs::read_to_string(dir.join("comm")).ok()?.trim_end() == name {
            let stat = std::fs::read_to_string(dir.join("stat")).ok()?;
            return stat.rsplit_once(") ")?.1.chars().next();
        }
    }
    None
}

/// What the hot thread saw.
struct Hot {
    worker_state: Option<char>,
    spread: Spread,
    window: Window,
    dropped: usize,
    ran_on: usize,
}

#[test]
fn a_pinned_thread_never_sleeps_on_a_wedged_stdout() -> TestResult {
    const N: usize = 20_000;
    let all = current_affinity()?;
    let (Some(helper), Some(hot_cpu)) = (all.nth(0), all.nth(1)) else {
        return Err(format!("this test needs two CPUs; this process may use {all}").into());
    };
    // fd 1 becomes a pipe nobody reads; the saved copy restores it
    let saved = rustix::io::dup(rustix::stdio::stdout())?;
    let (unread, write_end) = rustix::pipe::pipe()?;
    rustix::stdio::dup2_stdout(&write_end)?;
    drop(write_end);
    pin_current(&CpuList::one(helper))?;
    let cfg = LogConfig {
        thread_name: "ll-log-hot".to_owned(),
        ..LogConfig::default()
    };
    let appender = non_blocking(std::io::stdout(), &cfg);
    let placed = place_worker("ll-log-hot", &CpuList::one(helper))?;
    let (writer, guard, dropped) = (appender.writer, appender.guard, appender.dropped);
    let hot = std::thread::Builder::new()
        .name("ll-hot".to_owned())
        .spawn(move || -> Result<Hot, String> {
            pin_current(&CpuList::one(hot_cpu)).map_err(|e| e.to_string())?;
            let _default = tracing::subscriber::set_default(subscriber(writer, &cfg));
            // fill the pipe (64 KiB) and the queue: ~200-byte lines, paced
            // so the worker writes until the pipe is full whatever the timing
            let pad = "x".repeat(160);
            for i in 0..6_000 {
                tracing::warn!(i, pad = %pad, "fill");
                if i % 100 == 99 {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            std::thread::sleep(Duration::from_millis(300));
            let worker_state = state("ll-log-hot");
            let clock = quanta::Clock::new();
            // written once first: an untouched page would fault inside the window
            let mut costs = vec![u64::MAX; N];
            let before_dropped = dropped.dropped_lines();
            let before = snapshot(hot_cpu).map_err(|e| e.to_string())?;
            let t_start = clock.raw();
            for (k, seq) in (0..N as u64).enumerate() {
                let t0 = clock.raw();
                tracing::warn!(
                    seq = std::hint::black_box(seq),
                    waited_ns = 1_234_u64,
                    "offer held back"
                );
                let t1 = clock.raw();
                costs[k] = clock.delta_as_nanos(t0, t1);
            }
            let wall = clock.delta_as_nanos(t_start, clock.raw());
            let after = snapshot(hot_cpu).map_err(|e| e.to_string())?;
            let lost = dropped.dropped_lines() - before_dropped;
            let (idle_gap_max, idle_gaps_over_10us) = idle_gaps(&clock, idle_window_ns(wall));
            Ok(Hot {
                worker_state,
                spread: Spread::of(&mut costs),
                window: Window {
                    counters: Some(Counters::between(&before, &after)),
                    idle_gap_max,
                    idle_gaps_over_10us,
                },
                dropped: lost,
                ran_on: current_cpu().map_err(|e| e.to_string())?,
            })
        })?;
    let joined = hot.join();
    // fd 1 to /dev/null, then unblock the worker: the queued lines drain
    // there, not into the test's output; then fd 1 back
    let null = std::fs::OpenOptions::new().write(true).open("/dev/null")?;
    rustix::stdio::dup2_stdout(&null)?;
    drop(unread);
    drop(guard);
    rustix::stdio::dup2_stdout(&saved)?;
    let hot = joined.map_err(|_| "ll-hot panicked")??;
    println!(
        "hot_stdout\tworker_state={}\tworker_cpus={}\thot_cpu={hot_cpu}\tran_on={}\tdropped={}\t{}",
        hot.worker_state.unwrap_or('?'),
        placed.now,
        hot.ran_on,
        hot.dropped,
        hot.spread.fields()
    );
    println!("hot_stdout\twindow\t{}", hot.window.fields());
    println!("hot_stdout\tmax_cause\t{}", hot.window.cause(&hot.spread));
    assert_eq!(
        hot.worker_state,
        Some('S'),
        "ll-log should be asleep in write(2)"
    );
    assert_eq!(placed.now, CpuList::one(helper));
    assert_eq!(hot.ran_on, hot_cpu);
    let counters = hot.window.counters.as_ref().ok_or("no counters")?;
    assert_eq!(
        counters.voluntary, 0,
        "the hot thread slept while logging to a wedged stdout"
    );
    assert_eq!(
        hot.dropped, N,
        "every timed event should be dropped and counted"
    );
    Ok(())
}
