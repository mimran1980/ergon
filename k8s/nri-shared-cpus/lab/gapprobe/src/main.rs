//! gapprobe: the ergon-shared-cpus lab's busy-spin gap probe (oslat-style).
//!
//! ```text
//! gapprobe spin --label L --arm A --rep N --seconds S --out DIR
//!               [--cpu auto|N] [--threshold-ns 1000] [--udp-rate 20000] [--gap-cap 1000000]
//!               [--delay-seconds 0]
//! gapprobe irqsample --cpu N --seconds S --out FILE [--interval-ms 10] [--pin 0]
//! gapprobe report DIR [--top 6]
//! ```
//!
//! `spin` starts its helpers first, from the main thread, so they keep the
//! affinity the container started with (the shared CPUs under the plugin):
//! a UDP socket reader on loopback and the sender feeding it `--udp-rate`
//! datagrams a second. After `--delay-seconds` (helpers at their steady
//! state, and time for the lab to start the sampler on the CPU the first
//! line names) a thread pins itself to `--cpu` (`auto`: the
//! first of `ERGON_ISOLATED_CPUS`, else the lowest CPU of its affinity) and
//! spins `--seconds` reading the CPU's counter (aarch64 `CNTVCT_EL0` at
//! `CNTFRQ_EL0`, x86 the TSC), every iteration into a histogram, every gap of
//! at least `--threshold-ns` into a log with its `CLOCK_REALTIME` end. It
//! writes `DIR/L.summary.tsv`, `DIR/L.gaps.csv` and `DIR/L.hist.tsv`.
//!
//! `irqsample` reads `/proc/interrupts` and `/proc/softirqs` every interval
//! from its own CPU (`--pin`, housekeeping) and writes, for CPU `--cpu`,
//! every counter that grew with the window it grew in.
//!
//! `report` joins each run's max with the windows of `DIR/L.irq.tsv` it
//! overlaps and prints markdown.

use ergon_gapprobe::clock::{Anchors, Rate};
use ergon_gapprobe::irq;
use ergon_gapprobe::report::{self, RUN_HEADER, Run, Window};
use ergon_gapprobe::stats::Histogram;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{BufWriter, Write};
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type Res<T> = Result<T, Box<dyn Error>>;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rc = match args.first().map(String::as_str) {
        Some("spin") => spin(&Flags::parse(&args[1..])),
        Some("irqsample") => irqsample(&Flags::parse(&args[1..])),
        Some("report") => report_cmd(&args[1..]),
        _ => Err("usage: gapprobe spin|irqsample|report (see the crate docs)".into()),
    };
    if let Err(e) = rc {
        eprintln!("gapprobe: {e}");
        std::process::exit(1);
    }
}

/// `--name value` pairs.
struct Flags(HashMap<String, String>);

impl Flags {
    fn parse(args: &[String]) -> Self {
        let mut m = HashMap::new();
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if let Some(k) = a.strip_prefix("--") {
                m.insert(k.to_string(), it.next().cloned().unwrap_or_default());
            }
        }
        Self(m)
    }

    fn str(&self, k: &str) -> Res<&str> {
        self.0
            .get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("--{k} is required").into())
    }

    fn num(&self, k: &str, default: Option<u64>) -> Res<u64> {
        match (self.0.get(k), default) {
            (Some(v), _) => v
                .parse()
                .map_err(|_| format!("--{k} {v}: not a whole number").into()),
            (None, Some(d)) => Ok(d),
            (None, None) => Err(format!("--{k} is required").into()),
        }
    }
}

fn realtime_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

// ------------------------------------------------------------------ counter

#[cfg(target_arch = "aarch64")]
#[inline]
fn ticks() -> u64 {
    let v: u64;
    // SAFETY: reads the generic timer's virtual count, readable at EL0 on
    // Linux; isb keeps the read from being taken ahead of earlier work.
    unsafe { core::arch::asm!("isb", "mrs {v}, cntvct_el0", v = out(reg) v, options(nostack)) };
    v
}

#[cfg(target_arch = "aarch64")]
fn counter_rate() -> Option<Rate> {
    let hz: u64;
    // SAFETY: reads the counter's frequency register, readable at EL0.
    unsafe { core::arch::asm!("mrs {v}, cntfrq_el0", v = out(reg) hz, options(nomem, nostack)) };
    Rate::from_hz(hz)
}

#[cfg(target_arch = "x86_64")]
#[inline]
fn ticks() -> u64 {
    // SAFETY: rdtsc has no preconditions on x86_64.
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[cfg(target_arch = "x86_64")]
fn counter_rate() -> Option<Rate> {
    // the TSC's rate, against CLOCK_MONOTONIC over 200 ms
    let a = Instant::now();
    let t0 = ticks();
    while a.elapsed() < Duration::from_millis(200) {}
    let t1 = ticks();
    let ns = a.elapsed().as_nanos() as u64;
    Rate::from_hz(((u128::from(t1 - t0) * 1_000_000_000) / u128::from(ns.max(1))) as u64)
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
fn ticks() -> u64 {
    thread_local!(static START: Instant = Instant::now());
    START.with(|s| s.elapsed().as_nanos() as u64)
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
fn counter_rate() -> Option<Rate> {
    Rate::from_hz(1_000_000_000)
}

/// (ticks, realtime) read together: the realtime read between two counter
/// reads, at their midpoint.
fn anchor() -> (u64, u64) {
    let a = ticks();
    let r = realtime_ns();
    let b = ticks();
    (a + (b - a) / 2, r)
}

// ------------------------------------------------------------------ affinity

#[cfg(target_os = "linux")]
mod affinity {
    #[repr(C)]
    struct CpuSet([u64; 16]); // cpu_set_t: 1024 bits

    unsafe extern "C" {
        fn sched_setaffinity(pid: i32, size: usize, mask: *const CpuSet) -> i32;
        fn sched_getaffinity(pid: i32, size: usize, mask: *mut CpuSet) -> i32;
        fn sched_getcpu() -> i32;
    }

    /// Pins the calling thread to `cpu`.
    pub fn pin(cpu: usize) -> std::io::Result<()> {
        if cpu >= 1024 {
            return Err(std::io::Error::other(format!("CPU {cpu} >= 1024")));
        }
        let mut set = CpuSet([0; 16]);
        set.0[cpu / 64] |= 1 << (cpu % 64);
        // SAFETY: `set` is a valid cpu_set_t for the whole call.
        if unsafe { sched_setaffinity(0, size_of::<CpuSet>(), &raw const set) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    /// The calling thread's CPUs, ascending.
    pub fn allowed() -> std::io::Result<Vec<usize>> {
        let mut set = CpuSet([0; 16]);
        // SAFETY: `set` is a valid, writable cpu_set_t for the whole call.
        if unsafe { sched_getaffinity(0, size_of::<CpuSet>(), &raw mut set) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok((0..1024)
            .filter(|c| set.0[c / 64] & (1 << (c % 64)) != 0)
            .collect())
    }

    /// The CPU the calling thread runs on.
    pub fn current() -> i64 {
        // SAFETY: no arguments, no preconditions.
        i64::from(unsafe { sched_getcpu() })
    }
}

#[cfg(not(target_os = "linux"))]
mod affinity {
    pub fn pin(_cpu: usize) -> std::io::Result<()> {
        Err(std::io::Error::other("CPU pinning is Linux only"))
    }
    pub fn allowed() -> std::io::Result<Vec<usize>> {
        Err(std::io::Error::other("CPU affinity is Linux only"))
    }
    pub fn current() -> i64 {
        -1
    }
}

/// `--cpu auto`: the first of `ERGON_ISOLATED_CPUS`, else the lowest CPU
/// this thread may run on.
fn pick_cpu(flag: &str) -> Res<usize> {
    if flag != "auto" {
        return flag
            .parse()
            .map_err(|_| format!("--cpu {flag}: auto or a CPU number").into());
    }
    if let Ok(list) = std::env::var("ERGON_ISOLATED_CPUS") {
        let first = list.split(',').next().unwrap_or("").trim();
        return first
            .parse()
            .map_err(|_| format!("ERGON_ISOLATED_CPUS={list}: no first CPU").into());
    }
    affinity::allowed()?
        .first()
        .copied()
        .ok_or_else(|| "no CPU in this thread's affinity".into())
}

/// (voluntary, involuntary) context switches of the calling thread.
fn ctxt_switches() -> (u64, u64) {
    let text = fs::read_to_string("/proc/thread-self/status").unwrap_or_default();
    let field = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(k))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0)
    };
    (
        field("voluntary_ctxt_switches:"),
        field("nonvoluntary_ctxt_switches:"),
    )
}

// ------------------------------------------------------------------ spin

struct SpinResult {
    cpu: usize,
    hist: Histogram,
    gaps: Vec<(u64, u64)>,
    dropped: u64,
    anchors: Anchors,
    switches: (u64, u64),
}

fn spin_loop(cpu: usize, seconds: u64, threshold: u64, cap: usize, rate: Rate) -> Res<SpinResult> {
    affinity::pin(cpu)?;
    let on = affinity::current();
    if on != cpu as i64 {
        return Err(format!("pinned to CPU {cpu} but running on {on}").into());
    }
    let mut hist = Histogram::new();
    let mut gaps: Vec<(u64, u64)> = Vec::with_capacity(cap);
    let mut dropped = 0_u64;
    let sw0 = ctxt_switches();
    let (t0, r0) = anchor();
    let end = t0.saturating_add(rate.hz().saturating_mul(seconds));
    let mut prev = ticks();
    while prev < end {
        let now = ticks();
        let d = now.wrapping_sub(prev);
        hist.record(d);
        if d >= threshold {
            if gaps.len() < cap {
                gaps.push((now, d));
            } else {
                dropped += 1;
            }
        }
        prev = now;
    }
    let (t1, r1) = anchor();
    let sw1 = ctxt_switches();
    Ok(SpinResult {
        cpu,
        hist,
        gaps,
        dropped,
        anchors: Anchors { t0, r0, t1, r1 },
        switches: (sw1.0.saturating_sub(sw0.0), sw1.1.saturating_sub(sw0.1)),
    })
}

/// The socket reader and its sender, started from the calling thread.
fn start_helpers(
    rate: u64,
    stop: &Arc<AtomicBool>,
    received: &Arc<AtomicU64>,
) -> Res<Vec<thread::JoinHandle<()>>> {
    if rate == 0 {
        return Ok(Vec::new());
    }
    let sink = UdpSocket::bind("127.0.0.1:0")?;
    sink.set_read_timeout(Some(Duration::from_millis(100)))?;
    let addr = sink.local_addr()?;
    let (s, r) = (Arc::clone(stop), Arc::clone(received));
    let reader = thread::Builder::new()
        .name("gp-udp-sink".into())
        .spawn(move || {
            let mut buf = [0_u8; 2048];
            while !s.load(Ordering::Relaxed) {
                if sink.recv(&mut buf).is_ok() {
                    r.fetch_add(1, Ordering::Relaxed);
                }
            }
        })?;
    let s = Arc::clone(stop);
    let src = UdpSocket::bind("127.0.0.1:0")?;
    let sender = thread::Builder::new()
        .name("gp-udp-src".into())
        .spawn(move || {
            // rate/1000 datagrams each millisecond, the rest spread over a second
            let per_ms = rate / 1000;
            let extra = rate % 1000;
            let msg = [0x5a_u8; 64];
            let mut tick = 0_u64;
            while !s.load(Ordering::Relaxed) {
                let n = per_ms + u64::from(tick % 1000 < extra);
                for _ in 0..n {
                    let _ = src.send_to(&msg, addr);
                }
                tick += 1;
                thread::sleep(Duration::from_millis(1));
            }
        })?;
    Ok(vec![reader, sender])
}

fn spin(f: &Flags) -> Res<()> {
    let label = f.str("label")?.to_string();
    let arm = f.str("arm")?.to_string();
    let rep = u32::try_from(f.num("rep", None)?)?;
    let seconds = f.num("seconds", None)?;
    let out = PathBuf::from(f.str("out")?);
    let threshold_ns = f.num("threshold-ns", Some(1000))?;
    let udp_rate = f.num("udp-rate", Some(20_000))?;
    let cap = usize::try_from(f.num("gap-cap", Some(1_000_000))?)?;
    let delay = f.num("delay-seconds", Some(0))?;
    let cpu = pick_cpu(f.0.get("cpu").map_or("auto", String::as_str))?;
    let rate = counter_rate().ok_or("no counter rate")?;
    let threshold = rate.ticks(threshold_ns).max(1);
    fs::create_dir_all(&out)?;
    println!(
        "gapprobe: label={label} cpu={cpu} counter_hz={} threshold={threshold_ns}ns seconds={seconds} udp_rate={udp_rate} ERGON_ISOLATED_CPUS={} ERGON_SHARED_CPUS={} main_affinity={:?}",
        rate.hz(),
        std::env::var("ERGON_ISOLATED_CPUS").unwrap_or_default(),
        std::env::var("ERGON_SHARED_CPUS").unwrap_or_default(),
        affinity::allowed().unwrap_or_default(),
    );
    // helpers first, from this thread: they keep its affinity
    let stop = Arc::new(AtomicBool::new(false));
    let received = Arc::new(AtomicU64::new(0));
    let helpers = start_helpers(udp_rate, &stop, &received)?;
    thread::sleep(Duration::from_secs(delay));
    let hot = thread::Builder::new()
        .name("gp-hot".into())
        .spawn(move || spin_loop(cpu, seconds, threshold, cap, rate).map_err(|e| e.to_string()))?;
    let res = hot.join().map_err(|_| "the spin thread died")?;
    stop.store(true, Ordering::Relaxed);
    for h in helpers {
        let _ = h.join();
    }
    let r = res?;
    let to_rt = |t: u64| r.anchors.realtime(t).unwrap_or(0);
    let summary = r.hist.summary().ok_or("no iterations")?.map(|t| rate.ns(t));
    let max_end = r.gaps.iter().max_by_key(|g| g.1).map_or(0, |g| to_rt(g.0));
    let run = Run {
        label: label.clone(),
        arm,
        rep,
        cpu: r.cpu,
        summary,
        max_end_ns: max_end,
        gaps: r.gaps.len() as u64,
        dropped: r.dropped,
        nvcsw: r.switches.0,
        nivcsw: r.switches.1,
    };
    fs::write(
        out.join(format!("{label}.summary.tsv")),
        format!("{RUN_HEADER}\n{}\n", run.to_row()),
    )?;
    let mut g = BufWriter::new(fs::File::create(out.join(format!("{label}.gaps.csv")))?);
    writeln!(g, "end_realtime_ns,gap_ns")?;
    for (t, d) in &r.gaps {
        writeln!(g, "{},{}", to_rt(*t), rate.ns(*d))?;
    }
    g.flush()?;
    let mut h = BufWriter::new(fs::File::create(out.join(format!("{label}.hist.tsv")))?);
    writeln!(h, "upper_ns\tcount")?;
    for (u, c) in r.hist.buckets() {
        writeln!(h, "{}\t{c}", rate.ns(u))?;
    }
    h.flush()?;
    println!(
        "gapprobe: {label} cpu={} samples={} p50={} p90={} p99={} p99.9={} p99.99={} max={} ns max_end={} gaps>={threshold_ns}ns={} dropped={} nvcsw={} nivcsw={} udp_received={}",
        run.cpu,
        summary.samples,
        summary.p50,
        summary.p90,
        summary.p99,
        summary.p999,
        summary.p9999,
        summary.max,
        run.max_end_ns,
        run.gaps,
        run.dropped,
        run.nvcsw,
        run.nivcsw,
        received.load(Ordering::Relaxed),
    );
    if r.dropped > 0 {
        return Err(format!(
            "{} gaps over the log's capacity of {cap}: raise --gap-cap",
            r.dropped
        )
        .into());
    }
    Ok(())
}

// ------------------------------------------------------------------ irqsample

fn read_columns(cpu: usize) -> Res<Vec<(String, u64)>> {
    let mut v = irq::column(&fs::read_to_string("/proc/interrupts")?, cpu, "")?;
    v.extend(irq::column(
        &fs::read_to_string("/proc/softirqs")?,
        cpu,
        "softirq:",
    )?);
    Ok(v)
}

fn irqsample(f: &Flags) -> Res<()> {
    let cpu = usize::try_from(f.num("cpu", None)?)?;
    let seconds = f.num("seconds", None)?;
    let interval = Duration::from_millis(f.num("interval-ms", Some(10))?);
    let pin = usize::try_from(f.num("pin", Some(0))?)?;
    let out = PathBuf::from(f.str("out")?);
    affinity::pin(pin)?;
    let mut w = BufWriter::new(fs::File::create(&out)?);
    writeln!(
        w,
        "# cpu {cpu} interval_ms {} start_ns end_ns delta name",
        interval.as_millis()
    )?;
    let start = Instant::now();
    let mut prev = read_columns(cpu)?;
    let mut prev_t = realtime_ns();
    while start.elapsed() < Duration::from_secs(seconds) {
        thread::sleep(interval);
        let now = read_columns(cpu)?;
        let t = realtime_ns();
        for (name, delta) in irq::deltas(&prev, &now) {
            writeln!(
                w,
                "{}",
                Window {
                    start_ns: prev_t,
                    end_ns: t,
                    name,
                    delta
                }
                .to_row()
            )?;
        }
        prev = now;
        prev_t = t;
    }
    w.flush()?;
    Ok(())
}

// ------------------------------------------------------------------ report

fn report_cmd(args: &[String]) -> Res<()> {
    let dir = Path::new(args.first().ok_or("report DIR")?);
    let top = usize::try_from(Flags::parse(args.get(1..).unwrap_or(&[])).num("top", Some(6))?)?;
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".summary.tsv"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: no *.summary.tsv", dir.display()).into());
    }
    let mut runs = Vec::new();
    for p in files {
        let text = fs::read_to_string(&p)?;
        let row = text
            .lines()
            .nth(1)
            .ok_or_else(|| format!("{}: no row", p.display()))?;
        let run = Run::from_row(row)?;
        let irq_path = dir.join(format!("{}.irq.tsv", run.label));
        let over = match fs::read_to_string(&irq_path) {
            Ok(t) => {
                let windows = t
                    .lines()
                    .filter(|l| !l.starts_with('#') && !l.is_empty())
                    .map(Window::from_row)
                    .collect::<Result<Vec<_>, _>>()?;
                let start = run.max_end_ns.saturating_sub(run.summary.max);
                report::overlapping(&windows, start, run.max_end_ns)
            }
            Err(_) => vec![("no sampler file".to_string(), 0)],
        };
        runs.push((run, over));
    }
    runs.sort_by(|a, b| a.0.arm.cmp(&b.0.arm).reverse().then(a.0.rep.cmp(&b.0.rep)));
    print!("{}", report::render(&runs, top));
    Ok(())
}
