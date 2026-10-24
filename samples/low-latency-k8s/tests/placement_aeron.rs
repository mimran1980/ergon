//! The hot-thread sequence against an embedded media driver over IPC:
//!
//! - `aeron-client`, started by `ll-hot` before it pins, keeps the creator's
//!   mask; `ll-hot` ends on its own CPU. (The plugin's README lists Aeron as
//!   "not run" for that inheritance; this is the evidence.)
//! - The ping/pong loops allocate nothing per round trip: a run of 10 times
//!   the round trips makes as many allocations as the short one, plus at
//!   most one per extra warning.
//!
//! Linux with at least two CPUs (three keep the ping and the pong apart).
#![cfg(target_os = "linux")]
#![allow(clippy::similar_names)] // ping and pong are the names of the two sides

use std::alloc::System;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use low_latency_k8s::cpus::{CpuList, current_affinity, pin_current};
use low_latency_k8s::pingpong::{AppConfig, HotHandle, HotOutcome, Pinned, Shared, spawn_hot};
use low_latency_k8s::selfcheck::observe;
use rusteron_media_driver::testing::EmbeddedDriver;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

/// Runs both loops; returns the ping's outcome and the allocations made
/// while they ran.
fn run_pair(pair: Pair) -> Result<(HotOutcome, HotOutcome, usize), Box<dyn std::error::Error>> {
    pinned(&pair.pong)?;
    pinned(&pair.ping)?;
    let region = Region::new(GLOBAL);
    pair.pong.go.send(true)?;
    pair.ping.go.send(true)?;
    let ping = pair.ping.join.join().map_err(|_| "ping panicked")??;
    pair.pong_shared.stop.store(true, Ordering::Relaxed);
    let pong = pair.pong.join.join().map_err(|_| "pong panicked")??;
    let allocations = region.change().allocations;
    assert!(
        pong.messages >= ping.messages,
        "pong echoed {} of {}",
        pong.messages,
        ping.messages
    );
    Ok((ping, pong, allocations))
}

#[test]
fn the_client_conductor_keeps_the_creators_cpu() -> TestResult {
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
    let driver = EmbeddedDriver::launch()?;
    let (short, short_pong, short_allocs) =
        run_pair(start_pair(driver.dir(), 1_000, 10_000, 3001)?)?;
    let (long, long_pong, long_allocs) = run_pair(start_pair(driver.dir(), 1_000, 100_000, 3003)?)?;
    // A warning is an abnormal pass (back-pressure or a lost echo, which an
    // emulated amd64 run hits) and may allocate (one amd64 run made one
    // allocation more with one warning; its cause was not pinned down).
    // Extra allocations beyond the extra warnings fail.
    let short_warns = short.warn_events + short_pong.warn_events;
    let long_warns = long.warn_events + long_pong.warn_events;
    println!(
        "allocations: {short_allocs} for {} round trips ({short_warns} warnings), {long_allocs} for {} \
         ({long_warns} warnings); ping p50={} max={} ns",
        short.messages,
        long.messages,
        long.rtt.map_or(0, |p| p.p50),
        long.rtt.map_or(0, |p| p.max)
    );
    assert_eq!(
        (short.messages, long.messages),
        (10_000, 100_000),
        "ended: {} / {}",
        short.ended,
        long.ended
    );
    assert_eq!((short.lost, long.lost), (0, 0));
    let extra = long_allocs.saturating_sub(short_allocs);
    assert!(
        long_allocs >= short_allocs
            && u64::try_from(extra)? <= long_warns.saturating_sub(short_warns),
        "the longer run allocated {extra} more with {long_warns} warnings against {short_warns}: the loop allocates"
    );
    Ok(())
}
