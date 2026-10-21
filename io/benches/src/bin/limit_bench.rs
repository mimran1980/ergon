//! Cost of `limit::Budget::try_charge`, the check an application runs before
//! each request: one charge across 1, 4 and 8 rolling buckets, a decaying
//! bucket, the first charge after a long idle (every ring catches up), and a
//! refusal (8 parts, the last one full). Pure code: no I/O, 0 connections.
//!
//! Each sample times `--batch` calls with the raw counter and reports the mean
//! per call, so the counter's tick (41.7 ns on Apple M) and read cost do not
//! swamp a call of a few ns. A slow call shows up in its batch's mean, divided
//! by the batch size: `--batch 1` times single calls (x86 TSC). The `empty`
//! row is the same loop without the call; subtract it.
//!
//! Every case checks its answers before and during timing. Per case it also
//! prints `max batch µs` (the slowest batch), `stalls` (batches over 10 × the
//! median), `allocs` (allocations in the timed loop; must be 0) and `host
//! gap µs`: the longest gap seen by a loop that only reads the counter, run
//! for as long as the case right after it. A max batch near the host gap is
//! an interrupt or preemption, not the code. Pin to an isolated CPU on Linux
//! (`--cpu`) for a quiet max.
//!
//! ```text
//! limit_bench --samples 200000 --batch 64 [--cpu 2]
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

use ergon_io::clock::{rdtsc, ticks_per_us};
use ergon_io::limit::{BucketSpec, Budget, Charge, Class, Rate, Why};
use io_bench::args::Args;

/// Counts allocations, to show the timed loops make none.
struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);

// SAFETY: forwards to the system allocator; only adds a relaxed counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("limit-bench: {e}");
            ExitCode::FAILURE
        }
    }
}

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// One measured case: a budget, the charge, how far the clock moves per
/// call, and whether every call must pass.
struct Case {
    name: &'static str,
    budget: Budget,
    parts: Vec<Charge>,
    step_ns: i64,
    pass: bool,
    call: bool,
}

/// `n` rolling buckets that never fill at one call per µs: 10 s windows in
/// 100 sub-windows (a ring of 102), limit far above 10 M per window.
fn rolling(n: usize) -> R<(Budget, Vec<Charge>)> {
    let specs = (0..n)
        .map(|i| BucketSpec::rolling(format!("r{i}"), 4_000_000_000, 10_000, 100).with_reserve(10))
        .collect();
    let b = Budget::new(specs)?;
    let parts = (0..n)
        .map(|i| Ok(Charge::new(b.id(&format!("r{i}")).ok_or("id")?, 1)))
        .collect::<R<_>>()?;
    Ok((b, parts))
}

fn cases() -> R<Vec<Case>> {
    let mut v = Vec::new();
    let (budget, parts) = rolling(1)?;
    v.push(Case {
        name: "empty (loop only)",
        budget,
        parts,
        step_ns: 1_000,
        pass: true,
        call: false,
    });
    for (name, n) in [("rolling", 1), ("rolling", 4), ("rolling", 8)] {
        let (budget, parts) = rolling(n)?;
        v.push(Case {
            name,
            budget,
            parts,
            step_ns: 1_000,
            pass: true,
            call: true,
        });
    }
    let drain = Rate {
        units: 1_000_000,
        per_ms: 1,
    };
    let budget = Budget::new(vec![BucketSpec::decay("d", 1_000_000, drain)])?;
    let parts = vec![Charge::new(budget.id("d").ok_or("id")?, 1)];
    v.push(Case {
        name: "decay",
        budget,
        parts,
        step_ns: 1_000,
        pass: true,
        call: true,
    });
    let (budget, parts) = rolling(8)?;
    v.push(Case {
        name: "rolling, after 20 s idle",
        budget,
        parts,
        step_ns: 20_000_000_000,
        pass: true,
        call: true,
    });
    // Worst case for the ring catch-up: a long ring stepped just under one
    // full turn, so `advance` walks nearly every slot instead of taking the
    // `fill(0)` path a longer idle would. `Budget::tick` during idle keeps
    // this off the send path. The step is sized so `now` cannot overflow
    // over a full run (12.8 M calls): step_ns * samples * batch < i64::MAX.
    // window_ms must be a multiple of slots.
    for (name, window_ms, slots, step_ns) in [
        // 1440 slots of 100 ms: the ring is 1442 long, 1300 steps walked.
        ("ring 1442, catch-up walk", 144_000, 1_440, 130_000_000_000),
        // 6000 slots of 1 ms: the ring is 6002 long, 5500 steps walked.
        ("ring 6002, catch-up walk", 6_000, 6_000, 5_500_000_000),
    ] {
        let b = Budget::new(vec![
            BucketSpec::rolling("w", 1_000_000, window_ms, slots).with_reserve(10),
        ])?;
        let parts = vec![Charge::new(b.id("w").ok_or("id")?, 1)];
        v.push(Case {
            name,
            budget: b,
            parts,
            step_ns,
            pass: true,
            call: true,
        });
    }
    let (mut budget, mut parts) = rolling(7)?;
    let full = budget.add(BucketSpec::rolling("full", 1, 3_600_000, 60))?;
    budget
        .try_charge(&[Charge::new(full, 1)], Class::Normal, 0)
        .map_err(|l| format!("filling the last bucket: {l}"))?;
    parts.push(Charge::new(full, 1));
    v.push(Case {
        name: "refused (last of 8 full)",
        budget,
        parts,
        step_ns: 1_000,
        pass: false,
        call: true,
    });
    Ok(v)
}

/// Nearest-rank percentile of a sorted slice.
fn pct(sorted: &[f64], q: f64) -> f64 {
    let rank = (q * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn cpu_model() -> String {
    if let Ok(s) = std::fs::read_to_string("/proc/cpuinfo")
        && let Some(l) = s.lines().find(|l| l.starts_with("model name"))
        && let Some((_, m)) = l.split_once(':')
    {
        return m.trim().to_owned();
    }
    std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn run() -> R {
    let a = Args::from_env();
    let samples: usize = a.num_or("samples", 200_000)?;
    let batch: u32 = a.num_or("batch", 64)?;
    if samples == 0 || batch == 0 {
        return Err("--samples and --batch must be at least 1".into());
    }
    if let Some(c) = a.list("cpu", "").first() {
        io_bench::clock::pin_current_thread(c.parse()?)?;
    }
    let tpu = ticks_per_us();
    println!(
        "host: {} ({} {}), counter {} at {:.1} MHz; connections: 0 (pure code)",
        cpu_model(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        io_bench::clock::counter_name(),
        tpu
    );
    println!("{samples} samples × {batch} calls; ns per call, the mean of each batch\n");
    println!(
        "| case | buckets | p50 ns | p99 ns | p99.9 ns | max ns | max batch µs | stalls | allocs | host gap µs |\n\
         |---|---|---|---|---|---|---|---|---|---|"
    );
    for mut c in cases()? {
        let mut now: i64 = 1_000_000_000;
        // Correctness before timing: the warm-up must give the expected answer.
        for _ in 0..10_000 {
            now += c.step_ns;
            match c.budget.try_charge(&c.parts, Class::Normal, now) {
                Ok(_) if c.pass => {}
                Err(l) if !c.pass && l.why == Why::Full => {}
                other => {
                    return Err(format!("{}: unexpected warm-up result {other:?}", c.name).into());
                }
            }
        }
        let mut per_call = Vec::with_capacity(samples);
        let mut wrong = 0u64;
        let allocs = ALLOCS.load(Ordering::Relaxed);
        let start = rdtsc();
        for _ in 0..samples {
            let t0 = rdtsc();
            for _ in 0..batch {
                now += c.step_ns;
                let ok = if c.call {
                    black_box(&mut c.budget)
                        .try_charge(black_box(&c.parts), Class::Normal, black_box(now))
                        .is_ok()
                } else {
                    black_box(&c.parts).len() == black_box(c.parts.len())
                };
                wrong += u64::from(ok != c.pass);
            }
            let t1 = rdtsc();
            per_call.push(t1.wrapping_sub(t0) as f64 / tpu * 1_000.0 / f64::from(batch));
        }
        let run_s = rdtsc().wrapping_sub(start) as f64 / tpu / 1e6;
        let allocs = ALLOCS.load(Ordering::Relaxed) - allocs;
        let gap_us = longest_gap_us(run_s, tpu);
        if wrong > 0 {
            return Err(format!("{}: {wrong} calls gave the wrong answer", c.name).into());
        }
        per_call.sort_unstable_by(f64::total_cmp);
        let (p50, max) = (pct(&per_call, 0.50), per_call[per_call.len() - 1]);
        // A batch at 10x the median: the thread was interrupted, or the call slowed.
        let stalls = per_call.iter().filter(|&&v| v > 10.0 * p50).count();
        println!(
            "| {} | {} | {p50:.2} | {:.2} | {:.2} | {max:.2} | {:.1} | {stalls} | {allocs} | {gap_us:.1} |",
            c.name,
            c.parts.len(),
            pct(&per_call, 0.99),
            pct(&per_call, 0.999),
            max * f64::from(batch) / 1_000.0,
        );
    }
    Ok(())
}

/// Where a max comes from: spins reading only the counter for `seconds` (as
/// long as the case ran, right after it) and returns the longest gap, µs: an
/// interrupt or preemption. A case whose max batch is about this long shows
/// the host, not the code.
fn longest_gap_us(seconds: f64, tpu: f64) -> f64 {
    let end = rdtsc() + (seconds * tpu * 1e6) as u64;
    let mut longest = 0u64;
    let mut last = rdtsc();
    while last < end {
        let t = rdtsc();
        longest = longest.max(t.wrapping_sub(last));
        last = t;
    }
    longest as f64 / tpu
}
