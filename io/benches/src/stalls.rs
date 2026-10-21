//! Loop stalls: every iteration longer than a threshold, recorded on the loop
//! thread for one counter read per iteration, then attributed after the run to
//! kernel events on the loop's CPU (an ftrace capture on the `mono` clock) or
//! to the other host's stalls (wall clock).
//!
//! A stall with no kernel event inside it, no work and no syscall (the noise
//! probe) is time the CPU was taken away below the kernel: the hypervisor (or
//! firmware). In a loop that makes syscalls it may also be time spent inside
//! one; the noise probe on the same CPU tells the two apart statistically.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use crate::clock::{monotonic_ns, rdtsc, realtime_ns, ticks_per_us};

/// One stalled iteration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stall {
    /// Length in nanoseconds.
    pub ns: u64,
    /// `CLOCK_MONOTONIC` at the end of the stall (ftrace's `mono` clock).
    pub end_mono_ns: i64,
    /// `CLOCK_REALTIME` at the end (to match the other host's stalls).
    pub end_real_ns: i64,
    /// Work done in the stalled iteration (messages delivered, requests done).
    pub work: u32,
}

impl Stall {
    /// Start, monotonic nanoseconds.
    #[must_use]
    pub const fn start_mono_ns(&self) -> i64 {
        self.end_mono_ns.saturating_sub(self.ns as i64)
    }
}

/// Iteration counters plus the stalls themselves, preallocated.
pub struct StallLog {
    tpu: f64,
    record_ticks: u64,
    thresholds: [u64; 3],
    last: u64,
    last_work: u64,
    /// Iterations timed.
    pub iterations: u64,
    /// Longest iteration in counter ticks.
    pub max_ticks: u64,
    /// Iterations longer than 5, 20 and 100 µs.
    pub over: [u64; 3],
    entries: Vec<Stall>,
    /// Stalls not kept because the log was full.
    pub overflow: u64,
}

impl StallLog {
    /// Records every iteration longer than `record_over_us`, up to `capacity`.
    #[must_use]
    pub fn new(record_over_us: f64, capacity: usize) -> Self {
        let tpu = ticks_per_us();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ticks = |us: f64| (us * tpu) as u64;
        Self {
            tpu,
            record_ticks: ticks(record_over_us),
            thresholds: [ticks(5.0), ticks(20.0), ticks(100.0)],
            last: rdtsc(),
            last_work: 0,
            iterations: 0,
            max_ticks: 0,
            over: [0; 3],
            entries: Vec::with_capacity(capacity),
            overflow: 0,
        }
    }

    /// Restarts the iteration clock (call right before the timed loop).
    pub fn start(&mut self, work_total: u64) {
        self.last = rdtsc();
        self.last_work = work_total;
    }

    /// Ends the previous iteration and starts the next; `work_total` is a
    /// running count of work done (messages delivered so far).
    #[inline]
    pub fn tick(&mut self, work_total: u64) {
        let now = rdtsc();
        let gap = now.wrapping_sub(self.last);
        self.last = now;
        self.iterations += 1;
        self.max_ticks = self.max_ticks.max(gap);
        if gap > self.thresholds[0] {
            self.slow(gap, work_total);
        }
        self.last_work = work_total;
    }

    #[cold]
    #[inline(never)]
    fn slow(&mut self, gap: u64, work_total: u64) {
        for (c, &t) in self.over.iter_mut().zip(&self.thresholds) {
            *c += u64::from(gap > t);
        }
        if gap > self.record_ticks {
            if self.entries.len() < self.entries.capacity() {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss
                )]
                let ns = (gap as f64 / self.tpu * 1000.0) as u64;
                self.entries.push(Stall {
                    ns,
                    end_mono_ns: monotonic_ns(),
                    end_real_ns: realtime_ns(),
                    work: u32::try_from(work_total.saturating_sub(self.last_work))
                        .unwrap_or(u32::MAX),
                });
            } else {
                self.overflow += 1;
            }
            // The clock reads above are not the next iteration's.
            self.last = rdtsc();
        }
    }

    /// The recorded stalls.
    #[must_use]
    pub fn stalls(&self) -> &[Stall] {
        &self.entries
    }

    /// Longest iteration in microseconds.
    #[must_use]
    pub fn max_us(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let t = self.max_ticks as f64;
        t / self.tpu
    }

    /// The summary line of a stall file.
    #[must_use]
    pub fn summary(&self) -> StallSummary {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        StallSummary {
            iterations: self.iterations,
            max_ns: (self.max_us() * 1000.0) as u64,
            over: self.over,
            overflow: self.overflow,
            tid: current_tid(),
            cpu: current_cpu(),
        }
    }

    /// Writes the stalls: a summary line, then `ns end_mono_ns end_real_ns work`
    /// per stall.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        std::fs::write(path, format_stalls(&self.summary(), &self.entries))
    }
}

/// Totals for one stall log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StallSummary {
    /// Iterations timed.
    pub iterations: u64,
    /// Longest iteration in nanoseconds.
    pub max_ns: u64,
    /// Iterations longer than 5, 20 and 100 µs.
    pub over: [u64; 3],
    /// Stalls not kept.
    pub overflow: u64,
    /// The loop thread's id (to recognise it in `sched_switch` events).
    pub tid: i64,
    /// The CPU the loop ran on (it is pinned); -1 when unknown.
    pub cpu: i64,
}

/// The loop thread's kernel id (0 where unknown).
#[must_use]
pub fn current_tid() -> i64 {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: gettid has no preconditions.
        i64::from(unsafe { libc::gettid() })
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

/// The calling thread's CPU (-1 where unknown).
#[must_use]
pub fn current_cpu() -> i64 {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: sched_getcpu has no preconditions.
        i64::from(unsafe { libc::sched_getcpu() })
    }
    #[cfg(not(target_os = "linux"))]
    {
        -1
    }
}

/// The text form of a stall file.
#[must_use]
pub fn format_stalls(s: &StallSummary, stalls: &[Stall]) -> String {
    let mut out = String::with_capacity(64 + stalls.len() * 48);
    let _ = writeln!(
        out,
        "# iterations={} max_ns={} over5={} over20={} over100={} overflow={} tid={} cpu={}",
        s.iterations, s.max_ns, s.over[0], s.over[1], s.over[2], s.overflow, s.tid, s.cpu
    );
    for st in stalls {
        let _ = writeln!(
            out,
            "{} {} {} {}",
            st.ns, st.end_mono_ns, st.end_real_ns, st.work
        );
    }
    out
}

/// Parses a stall file (see [`format_stalls`]).
///
/// # Errors
/// A malformed line.
pub fn parse_stalls(text: &str) -> Result<(StallSummary, Vec<Stall>), String> {
    let mut summary = StallSummary::default();
    let mut stalls = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if let Some(h) = line.strip_prefix('#') {
            for kv in h.split_whitespace() {
                let Some((k, v)) = kv.split_once('=') else {
                    continue;
                };
                let num = || v.parse::<i64>().map_err(|e| format!("line {n}: {k}: {e}"));
                #[allow(clippy::cast_sign_loss)]
                match k {
                    "iterations" => summary.iterations = num()? as u64,
                    "max_ns" => summary.max_ns = num()? as u64,
                    "over5" => summary.over[0] = num()? as u64,
                    "over20" => summary.over[1] = num()? as u64,
                    "over100" => summary.over[2] = num()? as u64,
                    "overflow" => summary.overflow = num()? as u64,
                    "tid" => summary.tid = num()?,
                    "cpu" => summary.cpu = num()?,
                    _ => {}
                }
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        let [ns, mono, real, work] = f[..] else {
            return Err(format!("line {n}: expected 4 fields: {line:?}"));
        };
        let bad = |e: std::num::ParseIntError| format!("line {n}: {e}");
        stalls.push(Stall {
            ns: ns.parse().map_err(bad)?,
            end_mono_ns: mono.parse().map_err(bad)?,
            end_real_ns: real.parse().map_err(bad)?,
            work: work.parse().map_err(bad)?,
        });
    }
    Ok((summary, stalls))
}

/// One kernel trace event on a CPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// CPU.
    pub cpu: u32,
    /// Monotonic nanoseconds (microsecond resolution in the text trace).
    pub ns: i64,
    /// Event name (`irq_handler_entry`, `softirq_exit`, …).
    pub name: String,
    /// The rest of the line.
    pub detail: String,
}

/// Parses ftrace text output (`trace` file); lines that are not events are
/// skipped.
#[must_use]
pub fn parse_trace(text: &str) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        let toks: Vec<&str> = line.split_whitespace().collect();
        let Some(ci) = toks
            .iter()
            .position(|t| t.len() > 2 && t.starts_with('[') && t.ends_with(']'))
        else {
            continue;
        };
        let Ok(cpu) = toks[ci][1..toks[ci].len() - 1].parse::<u32>() else {
            continue;
        };
        // After the CPU: optional flags, then `seconds.micros:` and `event:`.
        let Some(ti) = (ci + 1..toks.len()).find(|&i| {
            toks[i]
                .strip_suffix(':')
                .is_some_and(|t| t.contains('.') && t.parse::<f64>().is_ok())
        }) else {
            continue;
        };
        let Some(name) = toks.get(ti + 1).and_then(|t| t.strip_suffix(':')) else {
            continue;
        };
        let ts = toks[ti].trim_end_matches(':');
        let (secs, frac) = ts.split_once('.').unwrap_or((ts, "0"));
        let (Ok(s), Ok(f)) = (secs.parse::<i64>(), frac.parse::<i64>()) else {
            continue;
        };
        // The fraction has as many digits as the trace prints (6 for µs).
        let scale = 10i64.pow(9u32.saturating_sub(u32::try_from(frac.len()).unwrap_or(9)));
        out.push(TraceEvent {
            cpu,
            ns: s * 1_000_000_000 + f * scale,
            name: name.to_owned(),
            detail: toks[ti + 2..].join(" "),
        });
    }
    out
}

/// A kernel activity interval on the loop's CPU.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    start: i64,
    end: i64,
    label: String,
}

fn field<'a>(detail: &'a str, key: &str) -> Option<&'a str> {
    detail
        .split_whitespace()
        .find_map(|t| t.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

/// Pairs entry/exit events into labelled intervals, and collects point events.
fn spans(events: &[TraceEvent], cpu: u32, tid: i64) -> (Vec<Span>, Vec<(i64, String)>) {
    let mut open: HashMap<String, (i64, String)> = HashMap::new();
    let mut spans = Vec::new();
    let mut points = Vec::new();
    let mut off_cpu: Option<(i64, String)> = None;
    for e in events.iter().filter(|e| e.cpu == cpu) {
        let n = e.name.as_str();
        if let Some(base) = n
            .strip_suffix("_entry")
            .or_else(|| n.strip_suffix("_start"))
        {
            let label = match base {
                "irq_handler" => format!("irq {}", field(&e.detail, "name").unwrap_or("?")),
                "softirq" => format!(
                    "softirq {}",
                    e.detail
                        .split("action=")
                        .nth(1)
                        .map_or("?", |a| a.trim_end_matches(']'))
                ),
                "hrtimer_expire" => {
                    format!("timer {}", field(&e.detail, "function").unwrap_or("?"))
                }
                "workqueue_execute" => {
                    format!("workqueue {}", e.detail.rsplit(' ').next().unwrap_or("?"))
                }
                "local_timer" => "local timer tick".to_owned(),
                "ipi" => format!("ipi {}", e.detail.trim_matches(|c| c == '(' || c == ')')),
                other => format!("{} (IPI/vector)", other.replace('_', " ")),
            };
            open.insert(base.to_owned(), (e.ns, label));
        } else if let Some(base) = n.strip_suffix("_exit").or_else(|| n.strip_suffix("_end")) {
            if let Some((start, label)) = open.remove(base) {
                spans.push(Span {
                    start,
                    end: e.ns,
                    label,
                });
            }
        } else if n == "sched_switch" {
            let prev = field(&e.detail, "prev_pid").and_then(|p| p.parse::<i64>().ok());
            let next = field(&e.detail, "next_pid").and_then(|p| p.parse::<i64>().ok());
            if prev == Some(tid) {
                let by = field(&e.detail, "next_comm").unwrap_or("?").to_owned();
                off_cpu = Some((e.ns, format!("preempted by {by}")));
            } else if next == Some(tid)
                && let Some((start, label)) = off_cpu.take()
            {
                spans.push(Span {
                    start,
                    end: e.ns,
                    label,
                });
            }
        } else if n.starts_with("page_fault") {
            points.push((e.ns, "page fault".to_owned()));
        } else {
            points.push((e.ns, n.to_owned()));
        }
    }
    (spans, points)
}

/// Why one stall happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribution {
    /// The stall.
    pub stall: Stall,
    /// The main cause.
    pub cause: String,
    /// Kernel time inside the stall, by label, in nanoseconds.
    pub kernel: Vec<(String, i64)>,
}

/// Most time one item of work (a message, a request) can plausibly take; a
/// longer stall is not explained by its work.
pub const MAX_WORK_NS_PER_ITEM: u64 = 20_000;

/// Attributes each stall on `cpu` (loop thread `tid`) to the kernel events
/// overlapping it; `events` is `None` when no trace was captured. Trace
/// timestamps have microsecond resolution, so windows are widened by 2 µs.
#[must_use]
pub fn attribute(
    stalls: &[Stall],
    events: Option<&[TraceEvent]>,
    cpu: u32,
    tid: i64,
) -> Vec<Attribution> {
    const SLACK: i64 = 2_000;
    let traced = events.is_some();
    let (spans, points) = spans(events.unwrap_or_default(), cpu, tid);
    stalls
        .iter()
        .map(|st| {
            let (s, e) = (st.start_mono_ns() - SLACK, st.end_mono_ns + SLACK);
            let mut kernel: HashMap<String, i64> = HashMap::new();
            for sp in spans.iter().filter(|sp| sp.end >= s && sp.start <= e) {
                let overlap = sp.end.min(e) - sp.start.max(s);
                *kernel.entry(sp.label.clone()).or_default() += overlap.max(1_000);
            }
            for (_, label) in points.iter().filter(|(t, _)| *t >= s && *t <= e) {
                *kernel.entry(label.clone()).or_default() += 0;
            }
            let mut kernel: Vec<(String, i64)> = kernel.into_iter().collect();
            kernel.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let half = i64::try_from(st.ns / 2).unwrap_or(i64::MAX);
            let work_fits = st.work > 0 && st.ns <= u64::from(st.work) * MAX_WORK_NS_PER_ITEM;
            let cause = match kernel.first() {
                Some((label, ns)) if *ns >= half => label.clone(),
                _ if kernel.iter().any(|(l, _)| l == "page fault") => "page faults".to_owned(),
                _ if work_fits => format!("loop work ({} items)", st.work),
                _ if !kernel.is_empty() => format!("mostly unexplained (+{})", kernel[0].0),
                _ if !traced => "not traced".to_owned(),
                _ => "unexplained: no kernel event (hypervisor or in-kernel time)".to_owned(),
            };
            Attribution {
                stall: *st,
                cause,
                kernel,
            }
        })
        .collect()
}

/// Markdown tables: stalls by cause, then the longest ones.
#[must_use]
pub fn report(title: &str, summary: &StallSummary, attributions: &[Attribution]) -> String {
    let mut out = String::new();
    #[allow(clippy::cast_precision_loss)]
    let us = |ns: u64| ns as f64 / 1000.0;
    let _ = writeln!(
        out,
        "#### Stalls: {title}\n\n{} iterations, longest {:.1} µs; over 5 / 20 / 100 µs: {} / {} / {}{}\n",
        summary.iterations,
        us(summary.max_ns),
        summary.over[0],
        summary.over[1],
        summary.over[2],
        if summary.overflow > 0 {
            format!(" ({} stalls not kept)", summary.overflow)
        } else {
            String::new()
        }
    );
    if attributions.is_empty() {
        return out;
    }
    let mut by: HashMap<String, (u64, u64, u64)> = HashMap::new();
    for a in attributions {
        let key = if a.cause.starts_with("loop work") {
            "loop work".to_owned()
        } else {
            a.cause.clone()
        };
        let v = by.entry(key).or_default();
        v.0 += 1;
        v.1 += a.stall.ns;
        v.2 = v.2.max(a.stall.ns);
    }
    let mut rows: Vec<_> = by.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.1));
    let _ = writeln!(
        out,
        "| cause | stalls | total µs | max µs |\n|---|---|---|---|"
    );
    for (cause, (n, total, max)) in rows {
        let _ = writeln!(out, "| {cause} | {n} | {:.1} | {:.1} |", us(total), us(max));
    }
    let mut top: Vec<&Attribution> = attributions.iter().collect();
    top.sort_by_key(|a| std::cmp::Reverse(a.stall.ns));
    let _ = writeln!(
        out,
        "\n| longest | µs | work | cause | kernel time inside (µs) |\n|---|---|---|---|---|"
    );
    for (i, a) in top.iter().take(10).enumerate() {
        let kernel = a
            .kernel
            .iter()
            .take(4)
            .map(|(l, ns)| {
                #[allow(clippy::cast_precision_loss)]
                let v = *ns as f64 / 1000.0;
                format!("{l} {v:.0}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "| {} | {:.1} | {} | {} | {} |",
            i + 1,
            us(a.stall.ns),
            a.stall.work,
            a.cause,
            kernel
        );
    }
    out.push('\n');
    out
}

/// Per-CPU interrupt counts from two `/proc/interrupts` snapshots: the rows
/// that changed on `cpu`, largest first.
#[must_use]
pub fn interrupt_deltas(before: &str, after: &str, cpu: usize) -> Vec<(String, u64)> {
    let parse = |text: &str| -> HashMap<String, u64> {
        let mut lines = text.lines();
        let Some(header) = lines.next() else {
            return HashMap::new();
        };
        let col = header
            .split_whitespace()
            .position(|c| c == format!("CPU{cpu}"));
        let mut m = HashMap::new();
        for l in lines {
            let mut t = l.split_whitespace();
            let Some(name) = t.next() else { continue };
            let counts: Vec<&str> = t.collect();
            let Some(v) = col
                .and_then(|c| counts.get(c))
                .and_then(|v| v.parse::<u64>().ok())
            else {
                continue;
            };
            // The description follows the per-CPU counts.
            let ncpu = header.split_whitespace().count();
            let desc = counts.get(ncpu..).map(|d| d.join(" ")).unwrap_or_default();
            m.insert(
                format!("{} {}", name.trim_end_matches(':'), desc)
                    .trim()
                    .to_owned(),
                v,
            );
        }
        m
    };
    let (b, a) = (parse(before), parse(after));
    let mut out: Vec<(String, u64)> = a
        .iter()
        .filter_map(|(k, &v)| {
            let d = v.saturating_sub(b.get(k).copied().unwrap_or(0));
            (d > 0).then(|| (k.clone(), d))
        })
        .collect();
    out.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    out
}

/// Loads a stall file and attributes it with the trace in `trace` (a path that
/// may not exist: then "not traced"). A loop whose CPU is unknown is "not
/// traced" too.
///
/// # Errors
/// The stall file is missing or malformed.
pub fn load_attributed(
    stall_file: &Path,
    trace: &Path,
) -> Result<(StallSummary, Vec<Attribution>), String> {
    let text = std::fs::read_to_string(stall_file)
        .map_err(|e| format!("{}: {e}", stall_file.display()))?;
    let (sum, st) = parse_stalls(&text).map_err(|e| format!("{}: {e}", stall_file.display()))?;
    let events = crate::record::read_text(trace)
        .ok()
        .map(|t| parse_trace(&t));
    let attrs = match (u32::try_from(sum.cpu), &events) {
        (Ok(cpu), Some(ev)) => attribute(&st, Some(ev), cpu, sum.tid),
        _ => attribute(&st, None, u32::MAX, sum.tid),
    };
    Ok((sum, attrs))
}

/// The server's stall files in a run directory (`server-<mode>-<port>.txt`).
#[must_use]
pub fn server_stall_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                (n.starts_with("server-echo-") || n.starts_with("server-http-"))
                    && n.ends_with(".txt")
            })
        })
        .collect();
    v.sort();
    v
}

/// The least slack for matching the server's stalls on the wall clock.
pub const SERVER_SLACK_NS: i64 = 50_000;

/// The slack when the run kept no chrony log: the two hosts' clocks are then
/// only known to be within the AWS runs' typical bounds of UTC (99–228 µs
/// each, from their env files' `chronyc tracking`), so twice the largest,
/// rounded up.
pub const UNKNOWN_CLOCK_SLACK_NS: i64 = 500_000;

/// The bound chrony gives on its host's clock against UTC in one
/// `chronyc tracking` output: |System time| + ½ Root delay + Root
/// dispersion (chronyc(1)), in ns; the largest when `text` holds several.
/// `None` when no complete capture is there (no daemon, a cut capture).
///
/// It is chrony's own estimate, at its default `maxclockerror` of 1 ppm,
/// so optimistic.
#[must_use]
pub fn chrony_bound_ns(text: &str) -> Option<i64> {
    let secs = |line: &str| -> Option<f64> {
        line.split_once(':')?
            .1
            .split_whitespace()
            .next()?
            .parse::<f64>()
            .ok()
    };
    let mut best: Option<i64> = None;
    let (mut system, mut delay) = (None, None);
    for line in text.lines() {
        let key = line.split(':').next().unwrap_or_default().trim();
        match key {
            "System time" => {
                system = secs(line).map(f64::abs);
                delay = None;
            }
            "Root delay" => delay = secs(line),
            "Root dispersion" => {
                if let (Some(sys), Some(d), Some(disp)) = (system, delay, secs(line)) {
                    #[allow(clippy::cast_possible_truncation)]
                    let ns = ((sys + d / 2.0 + disp) * 1e9).round() as i64;
                    best = Some(best.map_or(ns, |b| b.max(ns)));
                }
                (system, delay) = (None, None);
            }
            _ => {}
        }
    }
    best
}

/// The bound on the offset between the two hosts' clocks during the run in
/// `dir`: the sum of each host's [`chrony_bound_ns`], the larger of its
/// captures. From the run's own chrony logs (`clock-before.txt`,
/// `clock-after.txt` from the receiver; `clock-before-sender.txt`,
/// `clock-after-sender.txt` from the sender, written by
/// `scripts/lib-matrix.sh`), else from the `env-rcv*.txt` / `env-snd*.txt`
/// of the results directory above it (one capture per host, at set-up).
/// `None` when either host's is missing.
#[must_use]
pub fn clock_bound_ns(dir: &Path) -> Option<i64> {
    let largest = |files: &[std::path::PathBuf]| {
        files
            .iter()
            .filter_map(|f| std::fs::read_to_string(f).ok())
            .filter_map(|t| chrony_bound_ns(&t))
            .max()
    };
    let own = |names: [&str; 2]| largest(&names.map(|n| dir.join(n)));
    if let (Some(r), Some(s)) = (
        own(["clock-before.txt", "clock-after.txt"]),
        own(["clock-before-sender.txt", "clock-after-sender.txt"]),
    ) {
        return Some(r + s);
    }
    dir.ancestors().skip(1).take(4).find_map(|up| {
        let env = |prefix: &str| -> Vec<std::path::PathBuf> {
            std::fs::read_dir(up)
                .map(|d| {
                    d.filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(".txt"))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        Some(largest(&env("env-rcv"))? + largest(&env("env-snd"))?)
    })
}

/// The slack for matching the server's stalls on the wall clock: at least
/// the two hosts' clock bound ([`clock_bound_ns`]), never below
/// [`SERVER_SLACK_NS`]; [`UNKNOWN_CLOCK_SLACK_NS`] when it is not known.
#[must_use]
pub fn server_slack_ns(clock_bound: Option<i64>) -> i64 {
    clock_bound.map_or(UNKNOWN_CLOCK_SLACK_NS, |b| b.max(SERVER_SLACK_NS))
}

/// Why one round trip took long: the time each loop stalled inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripCause {
    /// Client stall time inside the round trip, ns.
    pub client_ns: i64,
    /// Cause of the longest client stall involved.
    pub client_cause: String,
    /// Server stall time inside it (window widened by the slack, see
    /// [`server_slack_ns`]), ns.
    pub server_ns: i64,
    /// Cause of the longest server stall involved.
    pub server_cause: String,
    /// Time over the median neither loop accounts for, ns.
    pub unexplained_ns: i64,
    /// One-line verdict.
    pub verdict: String,
}

fn stall_overlap(a: &[Attribution], from: i64, to: i64) -> (i64, String) {
    let mut total = 0;
    let mut longest: Option<&Attribution> = None;
    for x in a {
        let e = x.stall.end_real_ns;
        let b = e - i64::try_from(x.stall.ns).unwrap_or(i64::MAX);
        let ov = e.min(to) - b.max(from);
        if ov > 0 {
            total += ov;
            if longest.is_none_or(|l| l.stall.ns < x.stall.ns) {
                longest = Some(x);
            }
        }
    }
    (total, longest.map_or_else(String::new, |l| l.cause.clone()))
}

/// Attributes one round trip (`from`..`to`, wall clock) over a median of
/// `median_ns`: each loop's stall time inside it, summed, and the rest. The
/// verdict names whichever side explains at least half of the excess, else
/// says neither did (the network, the hypervisor, or something no trace saw).
/// The server's stalls are on its own clock, so its window is widened by
/// `slack_ns` ([`server_slack_ns`]).
#[must_use]
pub fn round_trip_cause(
    client: &[Attribution],
    server: &[Attribution],
    from: i64,
    to: i64,
    median_ns: i64,
    slack_ns: i64,
) -> RoundTripCause {
    let (client_ns, client_cause) = stall_overlap(client, from, to);
    let (server_ns, server_cause) = stall_overlap(server, from - slack_ns, to + slack_ns);
    let excess = (to - from - median_ns).max(0);
    let unexplained_ns = (excess - client_ns - server_ns).max(0);
    let verdict = if client_ns * 2 >= excess && client_ns >= server_ns && client_ns > 0 {
        format!("client: {client_cause}")
    } else if server_ns * 2 >= excess && server_ns > 0 {
        format!("server: {server_cause}")
    } else {
        "neither loop stalled long enough: network, hypervisor or untraced".to_owned()
    };
    RoundTripCause {
        client_ns,
        client_cause,
        server_ns,
        server_cause,
        unexplained_ns,
        verdict,
    }
}

/// The cause given to a loop stall that the spare core shared.
pub const VM_PAUSE: &str = "VM paused: the idle spare core stalled too (hypervisor)";

/// Relabels the loop stalls no kernel event explains as VM pauses when the
/// spare core (spinning on nothing, traced) had an equally unexplained stall
/// covering at least half of the same time: something below the kernel
/// stopped both cores.
pub fn mark_vm_pauses(attrs: &mut [Attribution], spare: &[Attribution]) {
    let below_kernel = |c: &str| c.starts_with("unexplained") || c == "not traced";
    let spare: Vec<&Attribution> = spare.iter().filter(|x| below_kernel(&x.cause)).collect();
    if spare.is_empty() {
        return;
    }
    for a in attrs
        .iter_mut()
        .filter(|a| below_kernel(&a.cause) || a.cause.starts_with("mostly unexplained"))
    {
        let (s, e) = (a.stall.start_mono_ns(), a.stall.end_mono_ns);
        let shared: i64 = spare
            .iter()
            .map(|x| (x.stall.end_mono_ns.min(e) - x.stall.start_mono_ns().max(s)).max(0))
            .sum();
        if shared * 2 >= i64::try_from(a.stall.ns).unwrap_or(i64::MAX) {
            a.cause = VM_PAUSE.to_owned();
        }
    }
}

/// A run's loop stalls (`stall_file`), attributed with the run's trace and,
/// when the run had a spare-core probe (`<dir>/spare`), with VM pauses marked.
///
/// # Errors
/// As [`load_attributed`].
pub fn load_loop(
    dir: &Path,
    stall_file: &Path,
) -> Result<(StallSummary, Vec<Attribution>), String> {
    let (sum, mut attrs) = load_attributed(stall_file, &dir.join("trace.txt"))?;
    let spare = dir.join("spare").join("stalls-0.txt");
    if spare.exists() {
        let (_, spare) = load_attributed(&spare, &dir.join("trace.txt"))?;
        mark_vm_pauses(&mut attrs, &spare);
    }
    Ok((sum, attrs))
}
