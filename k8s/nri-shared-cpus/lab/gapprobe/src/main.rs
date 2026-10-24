//! gapprobe: the ergon-shared-cpus lab's busy-spin gap probe (oslat-style).
//!
//! ```text
//! gapprobe spin --label L --arm A --rep N --seconds S --out DIR
//!               [--cpu auto|N] [--threshold-ns 1000] [--udp-rate 20000] [--gap-cap 1000000]
//!               [--delay-seconds 0] [--helpers process|thread]
//! gapprobe udp-helpers --udp-rate R      (spin's helper process: until stdin closes)
//! gapprobe burn --threads H --seconds S [--duty 100] [--period-ms 10]
//! gapprobe irqsample --cpu N --seconds S --out FILE [--interval-ms 10] [--pin 0]
//! gapprobe clocks
//! gapprobe report DIR [--top 6]
//! gapprobe trace-report --trace FILE --cpu N --offset-ns X --out FILE [--tid T] [--offset-end-ns Y]
//!               [--summary FILE] [--gaps FILE] [--events FILE] [--missing FILE]
//!               [--min-gap-ns 3000] [--slack-ns 10000]
//! gapprobe scale-report DIR
//! ```
//!
//! `spin` starts its helpers first, from the main thread, so they keep the
//! affinity the container started with (the shared CPUs under the plugin):
//! a UDP socket reader on loopback and the sender feeding it `--udp-rate`
//! datagrams a second. `--helpers process` (the default) runs them in a
//! child process (`udp-helpers`), so nothing they do to their address space
//! reaches the spin thread's; `thread` runs them as threads of this process.
//! The gap log and the histogram are written in full before the spin thread
//! pins itself, so its loop takes no page fault (the summary counts its
//! faults). After `--delay-seconds` (helpers at their steady
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
//! `burn` runs `--threads` helper threads (`gp-burn`), each busy `--duty`
//! percent of every `--period-ms`, for `--seconds`: the scale step's load
//! on the shared CPUs.
//!
//! `clocks` prints `CLOCK_REALTIME` and `CLOCK_MONOTONIC` ns read together:
//! their difference turns the kernel trace's `mono` clock into realtime.
//!
//! `report` joins each run's max with the windows of `DIR/L.irq.tsv` it
//! overlaps and prints markdown. `trace-report` names what ran on, or was
//! sent to, the spin's CPU in a tracefs `trace` file, and ties it to the
//! gap log. `scale-report` renders the scale step's cases.

use ergon_gapprobe::clock::{Anchors, Rate};
use ergon_gapprobe::gaplog::GapLog;
use ergon_gapprobe::report::{self, RUN_HEADER, Run, Window};
use ergon_gapprobe::stats::Histogram;
use ergon_gapprobe::{irq, procfs, scale, trace};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufWriter, Read, Write};
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
        Some("udp-helpers") => udp_helpers(&Flags::parse(&args[1..])),
        Some("burn") => burn(&Flags::parse(&args[1..])),
        Some("irqsample") => irqsample(&Flags::parse(&args[1..])),
        Some("clocks") => clocks(),
        Some("report") => report_cmd(&args[1..]),
        Some("trace-report") => trace_report(&Flags::parse(&args[1..])),
        Some("scale-report") => scale_report(&args[1..]),
        _ => Err(
            "usage: gapprobe spin|udp-helpers|burn|irqsample|clocks|report|trace-report|scale-report (see the crate docs)"
                .into(),
        ),
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
fn ctxt_switches(buf: &mut [u8]) -> (u64, u64) {
    let text = read_small("/proc/thread-self/status", buf);
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

/// A small procfs file into `buf` (written before the spin thread pins
/// itself): no allocation and no fresh page, so reading it next to the spin
/// loop takes no page fault there ("" when unreadable).
fn read_small<'a>(path: &str, buf: &'a mut [u8]) -> &'a str {
    let n = fs::File::open(path)
        .and_then(|mut f| f.read(buf))
        .unwrap_or(0);
    buf.get(..n)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("")
}

/// (minor, major) page faults of the calling thread; (0, 0) when unreadable.
fn thread_faults(buf: &mut [u8]) -> (u64, u64) {
    procfs::faults(read_small("/proc/thread-self/stat", buf)).unwrap_or((0, 0))
}

// ------------------------------------------------------------------ spin

struct SpinResult {
    cpu: usize,
    hist: Histogram,
    log: GapLog,
    anchors: Anchors,
    switches: (u64, u64),
    faults: (u64, u64),
}

fn spin_loop(cpu: usize, seconds: u64, threshold: u64, cap: usize, rate: Rate) -> Res<SpinResult> {
    // written in full here, on the start CPU, before the pin: the loop
    // takes no page fault on the hot CPU
    let mut hist = Histogram::new();
    let mut log = GapLog::new(cap);
    // and the buffer the counters are read into, the reads once
    let mut buf = std::hint::black_box(vec![1_u8; 8192]);
    let _ = std::hint::black_box((ctxt_switches(&mut buf), thread_faults(&mut buf), anchor()));
    affinity::pin(cpu)?;
    let on = affinity::current();
    if on != cpu as i64 {
        return Err(format!("pinned to CPU {cpu} but running on {on}").into());
    }
    // a first read on the new CPU can count a fault of its own (seen in a
    // Linux VM on Apple silicon): read once more, so the counts below are
    // the loop's
    let _ = std::hint::black_box((ctxt_switches(&mut buf), thread_faults(&mut buf)));
    let sw0 = ctxt_switches(&mut buf);
    let f0 = thread_faults(&mut buf);
    let (t0, r0) = anchor();
    let end = t0.saturating_add(rate.hz().saturating_mul(seconds));
    let mut prev = ticks();
    while prev < end {
        let now = ticks();
        let d = now.wrapping_sub(prev);
        hist.record(d);
        if d >= threshold {
            log.push(now, d);
        }
        prev = now;
    }
    let f1 = thread_faults(&mut buf);
    let (t1, r1) = anchor();
    let sw1 = ctxt_switches(&mut buf);
    Ok(SpinResult {
        cpu,
        hist,
        log,
        anchors: Anchors { t0, r0, t1, r1 },
        switches: (sw1.0.saturating_sub(sw0.0), sw1.1.saturating_sub(sw0.1)),
        faults: (f1.0.saturating_sub(f0.0), f1.1.saturating_sub(f0.1)),
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

/// The helpers in a child process (`gapprobe udp-helpers`): a separate
/// address space, so its mappings never touch the spin thread's.
struct HelperProcess {
    child: std::process::Child,
}

impl HelperProcess {
    fn start(rate: u64) -> Res<Self> {
        let child = std::process::Command::new(std::env::current_exe()?)
            .args(["udp-helpers", "--udp-rate", &rate.to_string()])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        Ok(Self { child })
    }

    /// Closes its stdin (it stops) and reads the datagrams it received.
    fn stop(mut self) -> Res<u64> {
        drop(self.child.stdin.take());
        let mut text = String::new();
        if let Some(mut o) = self.child.stdout.take() {
            o.read_to_string(&mut text)?;
        }
        let st = self.child.wait()?;
        if !st.success() {
            return Err(format!("the helper process exited {st}").into());
        }
        text.trim()
            .strip_prefix("received=")
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| format!("the helper process printed {text:?}").into())
    }
}

/// `udp-helpers`: the socket reader and sender until stdin closes, then
/// `received=N` on stdout.
fn udp_helpers(f: &Flags) -> Res<()> {
    let rate = f.num("udp-rate", Some(20_000))?;
    let stop = Arc::new(AtomicBool::new(false));
    let received = Arc::new(AtomicU64::new(0));
    let helpers = start_helpers(rate, &stop, &received)?;
    // blocks until the parent closes the pipe (or exits)
    let mut sink = Vec::new();
    let _ = std::io::stdin().lock().read_until(0, &mut sink);
    stop.store(true, Ordering::Relaxed);
    for h in helpers {
        let _ = h.join();
    }
    println!("received={}", received.load(Ordering::Relaxed));
    Ok(())
}

/// `burn`: `--threads` threads, each busy `--duty` % of every `--period-ms`.
fn burn(f: &Flags) -> Res<()> {
    let threads = f.num("threads", None)?;
    let seconds = f.num("seconds", None)?;
    let duty = f.num("duty", Some(100))?.min(100);
    let period = Duration::from_millis(f.num("period-ms", Some(10))?.max(1));
    let busy = period * u32::try_from(duty)? / 100;
    let end = Instant::now() + Duration::from_secs(seconds);
    let mut hs = Vec::new();
    for _ in 0..threads {
        hs.push(
            thread::Builder::new()
                .name("gp-burn".into())
                .spawn(move || {
                    let mut spins = 0_u64;
                    while Instant::now() < end {
                        let start = Instant::now();
                        while start.elapsed() < busy {
                            spins = std::hint::black_box(spins + 1);
                        }
                        let rest = period.saturating_sub(busy);
                        if !rest.is_zero() {
                            thread::sleep(rest);
                        }
                    }
                    spins
                })?,
        );
    }
    let mut total = 0_u64;
    for h in hs {
        total += h.join().map_err(|_| "a burn thread died")?;
    }
    println!(
        "gapprobe: burn threads={threads} duty={duty}% period_ms={} seconds={seconds} loops={total}",
        period.as_millis()
    );
    Ok(())
}

/// The spin's helpers: threads of this process, a child process, or none.
enum Helpers {
    Threads(Vec<thread::JoinHandle<()>>, Arc<AtomicBool>, Arc<AtomicU64>),
    Process(HelperProcess),
    None,
}

impl Helpers {
    fn start(mode: &str, rate: u64) -> Res<Self> {
        if rate == 0 {
            return Ok(Self::None);
        }
        if mode == "thread" {
            let stop = Arc::new(AtomicBool::new(false));
            let received = Arc::new(AtomicU64::new(0));
            let hs = start_helpers(rate, &stop, &received)?;
            return Ok(Self::Threads(hs, stop, received));
        }
        Ok(Self::Process(HelperProcess::start(rate)?))
    }

    /// Stops them; the datagrams received.
    fn stop(self) -> Res<u64> {
        match self {
            Self::Threads(hs, stop, received) => {
                stop.store(true, Ordering::Relaxed);
                for h in hs {
                    let _ = h.join();
                }
                Ok(received.load(Ordering::Relaxed))
            }
            Self::Process(p) => p.stop(),
            Self::None => Ok(0),
        }
    }
}

/// `L.gaps.csv` (every logged gap, its realtime end) and `L.hist.tsv`.
fn write_logs(out: &Path, label: &str, r: &SpinResult, rate: Rate) -> Res<()> {
    let to_rt = |t: u64| r.anchors.realtime(t).unwrap_or(0);
    let mut g = BufWriter::new(fs::File::create(out.join(format!("{label}.gaps.csv")))?);
    writeln!(g, "end_realtime_ns,gap_ns")?;
    for (t, d) in r.log.gaps() {
        writeln!(g, "{},{}", to_rt(*t), rate.ns(*d))?;
    }
    g.flush()?;
    let mut h = BufWriter::new(fs::File::create(out.join(format!("{label}.hist.tsv")))?);
    writeln!(h, "upper_ns\tcount")?;
    for (u, c) in r.hist.buckets() {
        writeln!(h, "{}\t{c}", rate.ns(u))?;
    }
    h.flush()?;
    Ok(())
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
    let mode =
        f.0.get("helpers")
            .map_or("process", String::as_str)
            .to_string();
    if !matches!(mode.as_str(), "process" | "thread") {
        return Err(format!("--helpers {mode}: process or thread").into());
    }
    let cpu = pick_cpu(f.0.get("cpu").map_or("auto", String::as_str))?;
    let rate = counter_rate().ok_or("no counter rate")?;
    let threshold = rate.ticks(threshold_ns).max(1);
    fs::create_dir_all(&out)?;
    println!(
        "gapprobe: label={label} cpu={cpu} counter_hz={} threshold={threshold_ns}ns seconds={seconds} udp_rate={udp_rate} helpers={mode} ERGON_ISOLATED_CPUS={} ERGON_SHARED_CPUS={} main_affinity={:?}",
        rate.hz(),
        std::env::var("ERGON_ISOLATED_CPUS").unwrap_or_default(),
        std::env::var("ERGON_SHARED_CPUS").unwrap_or_default(),
        affinity::allowed().unwrap_or_default(),
    );
    // helpers first, from this thread: they keep its affinity
    let helpers = Helpers::start(&mode, udp_rate)?;
    thread::sleep(Duration::from_secs(delay));
    let hot = thread::Builder::new()
        .name("gp-hot".into())
        .spawn(move || spin_loop(cpu, seconds, threshold, cap, rate).map_err(|e| e.to_string()))?;
    let res = hot.join().map_err(|_| "the spin thread died")?;
    let received = helpers.stop()?;
    let r = res?;
    let to_rt = |t: u64| r.anchors.realtime(t).unwrap_or(0);
    let summary = r.hist.summary().ok_or("no iterations")?.map(|t| rate.ns(t));
    let max_end = r
        .log
        .gaps()
        .iter()
        .max_by_key(|g| g.1)
        .map_or(0, |g| to_rt(g.0));
    let run = Run {
        label: label.clone(),
        arm,
        rep,
        cpu: r.cpu,
        summary,
        max_end_ns: max_end,
        gaps: r.log.gaps().len() as u64,
        dropped: r.log.dropped(),
        nvcsw: r.switches.0,
        nivcsw: r.switches.1,
        start_ns: r.anchors.r0,
        end_ns: r.anchors.r1,
        minflt: r.faults.0,
        majflt: r.faults.1,
    };
    fs::write(
        out.join(format!("{label}.summary.tsv")),
        format!("{RUN_HEADER}\n{}\n", run.to_row()),
    )?;
    write_logs(&out, &label, &r, rate)?;
    println!(
        "gapprobe: {label} cpu={} samples={} p50={} p90={} p99={} p99.9={} p99.99={} max={} ns max_end={} gaps>={threshold_ns}ns={} dropped={} nvcsw={} nivcsw={} minflt={} majflt={} udp_received={}",
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
        run.minflt,
        run.majflt,
        received,
    );
    if run.dropped > 0 {
        return Err(format!(
            "{} gaps over the log's capacity of {cap}: raise --gap-cap",
            run.dropped
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

// ------------------------------------------------------------------ clocks

#[cfg(target_os = "linux")]
fn monotonic_ns() -> Res<u64> {
    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(id: i32, ts: *mut Timespec) -> i32;
    }
    let mut ts = Timespec { sec: 0, nsec: 0 };
    // SAFETY: `ts` is a valid, writable timespec for the call; 1 is
    // CLOCK_MONOTONIC on Linux.
    if unsafe { clock_gettime(1, &raw mut ts) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(u64::try_from(ts.sec)? * 1_000_000_000 + u64::try_from(ts.nsec)?)
}

#[cfg(not(target_os = "linux"))]
fn monotonic_ns() -> Res<u64> {
    Err("CLOCK_MONOTONIC as the trace's mono clock: Linux only".into())
}

/// `clocks`: `realtime_ns monotonic_ns`, the realtime read between two
/// monotonic reads (their midpoint), the closest of 100 tries.
fn clocks() -> Res<()> {
    let mut best: Option<(u64, u64, u64)> = None;
    for _ in 0..100 {
        let a = monotonic_ns()?;
        let r = realtime_ns();
        let b = monotonic_ns()?;
        let w = b.saturating_sub(a);
        if best.is_none_or(|x| w < x.2) {
            best = Some((r, a + w / 2, w));
        }
    }
    let (r, m, _) = best.ok_or("no clock read")?;
    println!("{r} {m}");
    Ok(())
}

// ------------------------------------------------------------------ trace-report

fn lines_of(f: &Flags, k: &str) -> Vec<String> {
    f.0.get(k)
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|t| {
            t.lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn trace_report(f: &Flags) -> Res<()> {
    let text = fs::read_to_string(f.str("trace")?)?;
    let run = match f.0.get("summary") {
        Some(p) => Some(Run::from_row(
            fs::read_to_string(p)?
                .lines()
                .nth(1)
                .ok_or_else(|| format!("{p}: no row"))?,
        )?),
        None => None,
    };
    let gaps = match f.0.get("gaps") {
        Some(p) => trace::parse_gaps(&fs::read_to_string(p)?)?,
        None => Vec::new(),
    };
    let offset: i128 = f
        .str("offset-ns")?
        .parse()
        .map_err(|_| "--offset-ns: a whole number of ns")?;
    let input = trace::Input {
        trace: trace::parse(&text)?,
        cpu: usize::try_from(f.num("cpu", None)?)?,
        hot_tid: f.0.get("tid").and_then(|t| t.parse().ok()),
        offset_ns: offset,
        offset_end_ns: f.0.get("offset-end-ns").and_then(|v| v.parse().ok()),
        window: run.as_ref().map_or((0, 0), |r| (r.start_ns, r.end_ns)),
        gaps,
        nivcsw: run.as_ref().map(|r| r.nivcsw),
        min_gap_ns: f.num("min-gap-ns", Some(3000))?,
        slack_ns: f.num("slack-ns", Some(10_000))?,
        enabled: lines_of(f, "events"),
        missing: lines_of(f, "missing"),
    };
    fs::write(f.str("out")?, trace::render(&input))?;
    Ok(())
}

fn scale_report(args: &[String]) -> Res<()> {
    let dir = Path::new(args.first().ok_or("scale-report DIR")?);
    print!("{}", scale::render(dir)?);
    Ok(())
}
