//! The hot-thread sequence against an embedded media driver over IPC:
//!
//! - `aeron-client`, started by `ll-hot` before it pins, keeps the creator's
//!   mask; `ll-hot` ends on its own CPU. (The plugin's README lists Aeron as
//!   "not run" for that inheritance; this is the evidence.)
//! - The ping/pong loops allocate nothing per round trip: a run of 10 times
//!   the round trips makes as many allocations as the short one, plus
//!   exactly one per log line written meanwhile. The app's subscriber is
//!   installed (non-blocking writer to a capture), so a warning in the loop
//!   really formats and enqueues, as in the app; its one allocation is the
//!   line's copy into the queue (`src/logging.rs`). Lines from other
//!   threads (the embedded driver's wrapper) count the same way, and the
//!   test prints every line.
//!
//! Linux with at least two CPUs (three keep the ping and the pong apart).
#![cfg(target_os = "linux")]
#![allow(clippy::similar_names)] // ping and pong are the names of the two sides

use std::alloc::System;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use low_latency_k8s::cpus::{CpuList, current_affinity, pin_current};
use low_latency_k8s::logging::{Appender, LogConfig, non_blocking, subscriber};
use low_latency_k8s::pingpong::{AppConfig, HotHandle, HotOutcome, Pinned, Shared, spawn_hot};
use low_latency_k8s::selfcheck::observe;
use rusteron_media_driver::testing::EmbeddedDriver;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Every line the subscriber wrote, from any thread (the embedded driver's
/// wrapper logs too). Reserved up front: the worker's writes never
/// allocate.
#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut v = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("poisoned"))?;
        if v.len() + buf.len() > v.capacity() {
            return Err(std::io::Error::other("capture full"));
        }
        v.extend_from_slice(buf);
        drop(v);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Capture {
    /// The lines written so far.
    fn text(&self) -> String {
        self.0
            .lock()
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .unwrap_or_default()
    }
}

/// The app's logging, process-wide (the hot threads are spawned, so a
/// thread-local default would not reach them): `fmt` through the lossy
/// non-blocking writer into a [`Capture`]. The guard lives as long as the
/// process.
fn install_logging() -> Result<Capture, String> {
    type Installed = Result<(tracing_appender::non_blocking::WorkerGuard, Capture), String>;
    static INSTALLED: OnceLock<Installed> = OnceLock::new();
    INSTALLED
        .get_or_init(|| {
            let cfg = LogConfig::default();
            let capture = Capture(Arc::new(Mutex::new(Vec::with_capacity(16 << 20))));
            let Appender { writer, guard, .. } = non_blocking(capture.clone(), &cfg);
            tracing::subscriber::set_global_default(subscriber(writer, &cfg))
                .map_err(|e| e.to_string())?;
            Ok((guard, capture))
        })
        .as_ref()
        .map(|(_, c)| c.clone())
        .map_err(Clone::clone)
}

fn config(
    dir: &str,
    role: &str,
    warmup: u64,
    samples: u64,
    streams: i32,
) -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Each pair gets its own streams: a closed IPC publication lingers, and
    // a new subscription on its stream would see its image first.
    let vars = [
        ("LL_PING_STREAM", streams.to_string()),
        ("LL_PONG_STREAM", (streams + 1).to_string()),
        ("LL_ROLE", role.to_owned()),
        ("AERON_DIR", dir.to_owned()),
        ("LL_PUB_CHANNEL", "aeron:ipc?term-length=65536".to_owned()),
        ("LL_SUB_CHANNEL", "aeron:ipc".to_owned()),
        ("LL_WARMUP", warmup.to_string()),
        ("LL_SAMPLES", samples.to_string()),
        ("LL_DRIVER_WAIT_S", "10".to_owned()),
        ("LL_CONNECT_WAIT_S", "10".to_owned()),
    ];
    Ok(AppConfig::from_lookup(|k| {
        vars.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone())
    })?)
}

/// The CPUs: the creators' (helper) CPU, the ping's and the pong's.
fn cpus() -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    let all = current_affinity()?;
    let pick = |i: usize| all.nth(i.min(all.len() - 1));
    match (pick(0), pick(1), pick(2)) {
        (Some(a), Some(b), Some(c)) if all.len() >= 2 => Ok((a, b, c)),
        _ => Err(format!("these tests need two CPUs; this process may use {all}").into()),
    }
}

/// Spawns `ll-hot` for `cfg` from a creator thread pinned to `helper`.
fn spawn_from(
    helper: usize,
    cfg: AppConfig,
    hot: usize,
    shared: Arc<Shared>,
) -> Result<HotHandle, Box<dyn std::error::Error>> {
    let creator = std::thread::spawn(move || -> Result<HotHandle, String> {
        pin_current(&CpuList::one(helper)).map_err(|e| e.to_string())?;
        spawn_hot(cfg, CpuList::one(hot), shared).map_err(|e| e.to_string())
    });
    Ok(creator.join().map_err(|_| "creator panicked")??)
}

fn pinned(h: &HotHandle) -> Result<Pinned, Box<dyn std::error::Error>> {
    Ok(h.pinned.recv_timeout(Duration::from_secs(30))??)
}

struct Pair {
    ping: HotHandle,
    pong: HotHandle,
    pong_shared: Arc<Shared>,
}

fn start_pair(
    dir: &str,
    warmup: u64,
    samples: u64,
    streams: i32,
) -> Result<Pair, Box<dyn std::error::Error>> {
    let (helper, ping_cpu, pong_cpu) = cpus()?;
    let pong_shared = Arc::new(Shared::default());
    let pong = spawn_from(
        helper,
        config(dir, "pong", 0, 0, streams)?,
        pong_cpu,
        Arc::clone(&pong_shared),
    )?;
    let ping = spawn_from(
        helper,
        config(dir, "ping", warmup, samples, streams)?,
        ping_cpu,
        Arc::new(Shared::default()),
    )?;
    Ok(Pair {
        ping,
        pong,
        pong_shared,
    })
}

/// What one pair's run made: both outcomes, the allocations from every
/// thread while the loops ran, and the log lines written in that time.
struct Ran {
    ping: HotOutcome,
    pong: HotOutcome,
    allocations: usize,
    lines: Vec<String>,
}

/// Runs both loops.
fn run_pair(pair: Pair, capture: &Capture) -> Result<Ran, Box<dyn std::error::Error>> {
    pinned(&pair.pong)?;
    pinned(&pair.ping)?;
    // the worker writes what was queued before; then the region opens
    std::thread::sleep(Duration::from_millis(100));
    let from = capture.text().len();
    let region = Region::new(GLOBAL);
    pair.pong.go.send(true)?;
    pair.ping.go.send(true)?;
    let ping = pair.ping.join.join().map_err(|_| "ping panicked")??;
    pair.pong_shared.stop.store(true, Ordering::Relaxed);
    let pong = pair.pong.join.join().map_err(|_| "pong panicked")??;
    let change = region.change();
    // the lines queued inside the region reach the capture
    std::thread::sleep(Duration::from_millis(100));
    let text = capture.text();
    let lines: Vec<String> = text
        .get(from..)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    println!(
        "region: allocations={} reallocations={} bytes_allocated={} log_lines={} (ping {} round trips, ended {})",
        change.allocations,
        change.reallocations,
        change.bytes_allocated,
        lines.len(),
        ping.messages,
        ping.ended
    );
    for line in &lines {
        println!("region: logged: {line}");
    }
    assert!(
        pong.messages >= ping.messages,
        "pong echoed {} of {}",
        pong.messages,
        ping.messages
    );
    Ok(Ran {
        ping,
        pong,
        allocations: change.allocations,
        lines,
    })
}

#[test]
fn the_client_conductor_keeps_the_creators_cpu() -> TestResult {
    let _capture = install_logging()?;
    let driver = EmbeddedDriver::launch()?;
    let (helper, ping_cpu, pong_cpu) = cpus()?;
    let pair = start_pair(driver.dir(), 100, 1_000, 2001)?;
    let ping_pin = pinned(&pair.ping)?;
    let pong_pin = pinned(&pair.pong)?;
    assert_eq!((ping_pin.cpu, pong_pin.cpu), (ping_cpu, pong_cpu));
    let threads = observe(1, Duration::ZERO)?;
    let clients: Vec<_> = threads
        .iter()
        .filter(|t| t.comm == "aeron-client")
        .collect();
    let hots: Vec<_> = threads.iter().filter(|t| t.comm == "ll-hot").collect();
    println!(
        "placement: aeron-client masks {:?}, ll-hot masks {:?} (helper {helper})",
        clients
            .iter()
            .map(|t| t.mask.to_string())
            .collect::<Vec<_>>(),
        hots.iter().map(|t| t.mask.to_string()).collect::<Vec<_>>()
    );
    assert_eq!(clients.len(), 2, "{threads:?}");
    assert!(
        clients.iter().all(|t| t.mask == CpuList::one(helper)),
        "{clients:?}"
    );
    assert_eq!(hots.len(), 2);
    assert!(hots.iter().any(|t| t.mask == CpuList::one(ping_cpu)));
    assert!(hots.iter().any(|t| t.mask == CpuList::one(pong_cpu)));
    pair.pong.go.send(true)?;
    pair.ping.go.send(true)?;
    let ping = pair.ping.join.join().map_err(|_| "ping panicked")??;
    pair.pong_shared.stop.store(true, Ordering::Relaxed);
    let _ = pair.pong.join.join().map_err(|_| "pong panicked")??;
    assert_eq!((ping.messages, ping.lost), (1_000, 0));
    assert_eq!(ping.ran_on, ping_cpu);
    Ok(())
}

#[test]
fn the_loops_allocate_nothing_per_round_trip() -> TestResult {
    let capture = install_logging()?;
    let driver = EmbeddedDriver::launch()?;
    let short = run_pair(start_pair(driver.dir(), 1_000, 10_000, 3001)?, &capture)?;
    let long = run_pair(start_pair(driver.dir(), 1_000, 100_000, 3003)?, &capture)?;
    let warns = |r: &Ran| r.ping.warn_events + r.pong.warn_events;
    println!(
        "allocations: {} for {} round trips ({} warnings, {} log lines), {} for {} ({} warnings, {} log lines); ping p50={} max={} ns",
        short.allocations,
        short.ping.messages,
        warns(&short),
        short.lines.len(),
        long.allocations,
        long.ping.messages,
        warns(&long),
        long.lines.len(),
        long.ping.rtt.map_or(0, |p| p.p50),
        long.ping.rtt.map_or(0, |p| p.max)
    );
    assert_eq!(
        (short.ping.messages, long.ping.messages),
        (10_000, 100_000),
        "ended: {} / {}",
        short.ping.ended,
        long.ping.ended
    );
    assert_eq!((short.ping.lost, long.ping.lost), (0, 0));
    // Each hot-loop warning is a log line; a line from any thread (the
    // embedded driver's wrapper logs too) costs its caller one allocation.
    for r in [&short, &long] {
        let hot_lines = r.lines.iter().filter(|l| l.contains(" ll-hot ")).count();
        assert_eq!(
            u64::try_from(hot_lines)?,
            warns(r),
            "hot warnings against hot log lines: {:?}",
            r.lines
        );
    }
    // allocations not explained by a log line: the same for both runs
    let unexplained = |r: &Ran| {
        i128::try_from(r.allocations).unwrap_or(i128::MAX)
            - i128::try_from(r.lines.len()).unwrap_or(0)
    };
    assert_eq!(
        unexplained(&long),
        unexplained(&short),
        "the longer run allocated {} with {} log lines against {} with {}: the loop allocates",
        long.allocations,
        long.lines.len(),
        short.allocations,
        short.lines.len()
    );
    Ok(())
}
