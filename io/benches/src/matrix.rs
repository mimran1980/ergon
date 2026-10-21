//! The settings matrix (`analyse --matrix DIR`): one table per measurement
//! across every condition, in run order, each value next to its change against
//! the same round's baselines. Every latency column includes the maximum, and
//! every table names the cause of its worst case.
//!
//! Layout: `DIR/<tag>/<run>`, where a tag is `r<round>-base-<k>`,
//! `r<round>-<condition>` or `boot-<profile>`, and a run is `noise`,
//! `pp-<backend>-<rate>`, `http-recv-busy-1000` or `md-recv-busy`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::record::read_records;
use crate::stalls::{self, Attribution, StallSummary};
use crate::stats::{
    SEND_SCHEDULE_HEADER, SendSchedule, Summary, WORK_HEADER, Work, send_schedule_row, summarize,
    us, work_row,
};

/// One run's results.
#[derive(Debug, Clone, Default)]
pub struct Run {
    /// Round trip (ping-pong, HTTP) or kernel arrival → decoded (burst);
    /// `None` for the noise probe.
    pub latency: Option<Summary>,
    /// The loop's iteration counters.
    pub loop_stats: StallSummary,
    /// Run length in seconds.
    pub seconds: f64,
    /// Stall time by cause, nanoseconds.
    pub causes: Vec<(String, u64)>,
    /// What made the worst case (the slowest round trip, or the longest stall).
    pub worst: String,
    /// Samples above 100 µs, 500 µs, 1 ms and 10 ms: how often the tail
    /// reaches each size (steadier than the maximum, which stays reported).
    pub over: [u64; 4],
    /// The five slowest samples (or, for the noise probe, longest gaps), each
    /// with when it happened and why.
    pub worst_events: Vec<WorstEvent>,
    /// Time the kept samples span, seconds (for rates).
    pub span_s: f64,
    /// Paced round trips measured from when each was due rather than when it
    /// was sent: a stall delays every message due during it, and this counts
    /// that wait (no coordinated omission). `None` for unpaced runs and runs
    /// recorded before the due time was.
    pub due: Option<Summary>,
    /// As [`Run::over`], for [`Run::due`].
    pub due_over: [u64; 4],
    /// Market data: each message from its send time on the sender (lab) or
    /// the exchange's event time (live) to decoded. Two hosts' clocks, so it
    /// includes their offset, which runs made close together share. `None`
    /// for round trips.
    pub one_way: Option<Summary>,
    /// As [`Run::over`], for [`Run::one_way`].
    pub one_way_over: [u64; 4],
    /// The bound on the two hosts' clock offset during the run, ns
    /// ([`stalls::clock_bound_ns`]): one-way values are relative within
    /// it, and the server's stalls are matched within it.
    pub clock_bound_ns: Option<i64>,
    /// Interrupts per second during the run ([`IRQ_KINDS`], from
    /// `/proc/interrupts` before and after) on the loop CPU and, when another
    /// CPU received the packets (the SQPOLL thread's, or a softirq's), on that
    /// one: `(cpu, rates)`, loop CPU first. Empty without snapshots.
    pub irqs: Vec<(usize, [f64; 5])>,
    /// What the run set out to do and what became of it, beside the percentiles
    /// (which are over [`Work::completed`] alone). `None` for a run recorded
    /// before the counts were.
    pub work: Option<Work>,
    /// The sender's batch schedule beside that: whether the traffic was the
    /// planned traffic. A different population from [`Work::late`] (see
    /// [`SendSchedule`]). `None` for a run with no sender account.
    pub send: Option<SendSchedule>,
}

/// The interrupts that show tick and IPI work on a CPU: local timer,
/// irq_work (a `nohz_full` CPU's tick restarted), rescheduling and
/// function-call IPIs, and on bare metal system-management interrupts
/// (firmware pauses; `vm-setup.sh irqs` adds the row).
pub const IRQ_KINDS: [&str; 5] = ["LOC", "IWI", "RES", "CAL", "SMI"];

/// Which of [`IRQ_KINDS`] a `/proc/interrupts` row (as keyed by
/// [`stalls::interrupt_deltas`]: name, then description) counts as. x86 names
/// its rows `LOC`, `IWI`, `RES`, `CAL` (and `vm-setup.sh` adds `SMI`); arm64
/// numbers its IPIs (`IPI0`, ...) with the same descriptions, and its tick is
/// the numbered `arch_timer` interrupt.
#[must_use]
pub fn irq_kind(key: &str) -> Option<usize> {
    let name = key.split_whitespace().next().unwrap_or_default();
    if let Some(i) = IRQ_KINDS.iter().position(|&c| c == name) {
        return Some(i);
    }
    let desc = key.get(name.len()..).unwrap_or_default().trim();
    if name.chars().all(|c| c.is_ascii_digit()) && desc.ends_with(" arch_timer") {
        return Some(0);
    }
    if !name.starts_with("IPI") {
        return None;
    }
    match desc {
        "IRQ work interrupts" => Some(1),
        "Rescheduling interrupts" => Some(2),
        "Function call interrupts" => Some(3),
        _ => None,
    }
}

/// [`IRQ_KINDS`] per second on `cpu` over `seconds`, or `None` without
/// both snapshots.
fn irq_rates(dir: &Path, cpu: usize, seconds: f64) -> Option<[f64; 5]> {
    let before = std::fs::read_to_string(dir.join("irq-before.txt")).ok()?;
    let after = std::fs::read_to_string(dir.join("irq-after.txt")).ok()?;
    let mut v = [0.0; 5];
    for (k, n) in stalls::interrupt_deltas(&before, &after, cpu) {
        if let Some(i) = irq_kind(&k) {
            #[allow(clippy::cast_precision_loss)]
            let rate = n as f64 / seconds.max(1.0);
            v[i] += rate;
        }
    }
    Some(v)
}

/// The loop CPU and, if different, the CPU that last received each
/// connection's packets (`rx_cpu:` in `meta.txt`), with their interrupt rates.
fn run_irqs(
    dir: &Path,
    m: &HashMap<String, String>,
    loop_cpu: i64,
    seconds: f64,
) -> Vec<(usize, [f64; 5])> {
    let mut cpus: Vec<usize> = usize::try_from(loop_cpu).into_iter().collect();
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    for k in keys {
        let rx = m[k]
            .split_whitespace()
            .find_map(|t| t.strip_prefix("rx_cpu:"))
            .and_then(|c| c.parse::<usize>().ok());
        if let Some(c) = rx
            && !cpus.contains(&c)
        {
            cpus.push(c);
        }
    }
    cpus.into_iter()
        .filter_map(|c| irq_rates(dir, c, seconds).map(|r| (c, r)))
        .collect()
}

/// One of a run's worst samples.
#[derive(Debug, Clone, PartialEq)]
pub struct WorstEvent {
    /// The sample (round trip, arrival → decoded, or gap), nanoseconds.
    pub ns: i64,
    /// Seconds into the run.
    pub at_s: f64,
    /// Why (see [`stalls::round_trip_cause`]).
    pub cause: String,
}

/// Thresholds for [`Run::over`], nanoseconds.
pub const OVER_NS: [i64; 4] = [100_000, 500_000, 1_000_000, 10_000_000];

/// Counts of `v` above each of [`OVER_NS`], and the seconds from the first
/// to the last of `times`.
fn excursions(v: &[i64], times: impl Iterator<Item = i64> + Clone) -> ([u64; 4], f64) {
    let mut over = [0u64; 4];
    for &x in v {
        for (c, &t) in over.iter_mut().zip(&OVER_NS) {
            *c += u64::from(x > t);
        }
    }
    let lo = times.clone().min().unwrap_or(0);
    let hi = times.max().unwrap_or(0);
    #[allow(clippy::cast_precision_loss)]
    let span = (hi - lo) as f64 / 1e9;
    (over, span)
}

/// One condition's runs.
#[derive(Debug, Clone)]
pub struct Tag {
    /// Directory name.
    pub name: String,
    /// Runs by directory name.
    pub runs: HashMap<String, Run>,
}

impl Tag {
    /// The round of a stage-1 tag (`r<round>-…`); 0 for anything else.
    #[must_use]
    pub fn round(&self) -> u8 {
        match self.name.as_bytes() {
            [b'r', d, b'-', ..] if d.is_ascii_digit() => d - b'0',
            _ => 0,
        }
    }

    /// Whether this is a stage-2 tag (`c<repeat>-…`).
    #[must_use]
    pub fn is_confirm(&self) -> bool {
        matches!(self.name.as_bytes(), [b'c', d, b'-', ..] if d.is_ascii_digit())
    }

    /// Whether this is a baseline measurement (`r<n>-base-<k>`, `c<n>-base`).
    #[must_use]
    pub fn is_base(&self) -> bool {
        self.name.contains("-base-") || (self.is_confirm() && self.name.ends_with("-base"))
    }
}

fn meta(dir: &Path) -> Result<HashMap<String, String>, String> {
    Ok(std::fs::read_to_string(dir.join("meta.txt"))
        .map_err(|e| format!("meta.txt: {e}"))?
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect())
}

/// The five longest stalls of a loop with their causes.
fn worst_stalls(client: &[Attribution]) -> Vec<WorstEvent> {
    let t0 = client
        .iter()
        .map(|a| a.stall.end_real_ns - i64::try_from(a.stall.ns).unwrap_or(0))
        .min()
        .unwrap_or(0);
    let mut v: Vec<&Attribution> = client.iter().collect();
    v.sort_by_key(|a| std::cmp::Reverse(a.stall.ns));
    v.iter()
        .take(5)
        .map(|a| WorstEvent {
            ns: i64::try_from(a.stall.ns).unwrap_or(i64::MAX),
            #[allow(clippy::cast_precision_loss)]
            at_s: (a.stall.end_real_ns - t0) as f64 / 1e9,
            cause: a.cause.clone(),
        })
        .collect()
}

/// Ping-pong and HTTP runs drop their first second (as `analyse` does).
pub const ROUND_TRIP_WARMUP_S: i64 = 1;

/// Loads one run directory. Stream runs (the burst) drop their first
/// `warmup_s` seconds, as `analyse --warmup-s` does.
///
/// # Errors
/// A file the run needs is missing or malformed.
pub fn load_run(dir: &Path, warmup_s: i64) -> Result<Run, String> {
    let m = meta(dir)?;
    let mode = m.get("mode").ok_or("meta.txt has no mode")?.clone();
    let seconds: f64 = m
        .get("duration_s")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let (loop_stats, client) = stalls::load_loop(dir, &dir.join("stalls-0.txt"))?;
    let irqs = run_irqs(dir, &m, loop_stats.cpu, seconds);
    let clock_bound = stalls::clock_bound_ns(dir);
    let mut causes: HashMap<String, u64> = HashMap::new();
    for a in &client {
        let key = if a.cause.starts_with("loop work") {
            "loop work".to_owned()
        } else {
            a.cause.clone()
        };
        *causes.entry(key).or_default() += a.stall.ns;
    }
    let mut causes: Vec<(String, u64)> = causes.into_iter().collect();
    causes.sort_by_key(|c| std::cmp::Reverse(c.1));
    let longest_stall = client.iter().max_by_key(|a| a.stall.ns).map(|a| {
        format!(
            "{} µs stall: {}",
            us(i64::try_from(a.stall.ns).unwrap_or(0)),
            a.cause
        )
    });
    if mode == "noise" {
        return Ok(Run {
            latency: None,
            loop_stats,
            seconds,
            causes,
            worst: longest_stall.unwrap_or_default(),
            over: [0; 4],
            span_s: seconds,
            worst_events: worst_stalls(&client),
            irqs,
            ..Run::default()
        });
    }
    let records =
        read_records(&dir.join("records.bin")).map_err(|e| format!("records.bin: {e}"))?;
    // The work counts are the run's own account of what it did, checked
    // against the records it wrote before any warm-up is dropped. A run that
    // claims counts that cannot hold, or more completions than it recorded,
    // is named here rather than reported beside the percentiles.
    let work = Work::load(&m, records.len())?;
    // Beside the work counts: the sender's batch schedule, which says whether
    // the traffic was the planned traffic. `Work::late` cannot say that (see
    // [`SendSchedule`]), and a run whose sender fell behind measures a load it
    // was not offered on purpose.
    let send = SendSchedule::from_meta(&m)?;
    let t0 = records.iter().map(|r| r.user_ns).min().unwrap_or(0);
    let mut due = None;
    let mut due_over = [0; 4];
    let mut one_way = None;
    let mut one_way_over = [0; 4];
    let (latency, worst, over, span_s, worst_events) = if mode == "pingpong" {
        let kept: Vec<_> = records
            .iter()
            .filter(|r| r.user_ns >= t0 + ROUND_TRIP_WARMUP_S * 1_000_000_000)
            .collect();
        let mut from_due: Vec<i64> = kept
            .iter()
            .filter(|r| r.src2_ns > 0)
            .map(|r| r.user_ns - r.src2_ns)
            .collect();
        due_over = excursions(&from_due, std::iter::empty()).0;
        due = summarize(&mut from_due);
        let mut rtt: Vec<i64> = kept.iter().map(|r| r.user_ns - r.src_ns).collect();
        let (over, span_s) = excursions(&rtt, kept.iter().map(|r| r.user_ns));
        let s = summarize(&mut rtt);
        let mut server: Vec<Attribution> = Vec::new();
        for f in stalls::server_stall_files(dir) {
            server.extend(stalls::load_attributed(&f, &dir.join("server-trace.txt"))?.1);
        }
        let p50 = s.as_ref().map_or(0, |s| s.p50);
        let slack = stalls::server_slack_ns(clock_bound);
        let mut slow: Vec<_> = kept.clone();
        slow.sort_by_key(|r| std::cmp::Reverse(r.user_ns - r.src_ns));
        let worst_events: Vec<WorstEvent> = slow
            .iter()
            .take(5)
            .map(|r| WorstEvent {
                ns: r.user_ns - r.src_ns,
                #[allow(clippy::cast_precision_loss)]
                at_s: (r.user_ns - t0) as f64 / 1e9,
                cause: stalls::round_trip_cause(&client, &server, r.src_ns, r.user_ns, p50, slack)
                    .verdict,
            })
            .collect();
        let worst = worst_events
            .first()
            .map_or_else(String::new, |e| e.cause.clone());
        (s, worst, over, span_s, worst_events)
    } else {
        // Every message: kernel arrival of its read's newest segment → decoded.
        let kept: Vec<_> = records
            .iter()
            .filter(|r| r.user_ns >= t0 + warmup_s * 1_000_000_000 && r.rx_sw_ns > 0)
            .collect();
        // Every message with a send time, kernel timestamp or not (a plain
        // `read`, as mio_bench does, has none).
        let mut sent: Vec<i64> = records
            .iter()
            .filter(|r| r.user_ns >= t0 + warmup_s * 1_000_000_000 && r.src_ns > 0)
            .map(|r| r.user_ns - r.src_ns)
            .collect();
        one_way_over = excursions(&sent, std::iter::empty()).0;
        one_way = summarize(&mut sent);
        let mut lat: Vec<i64> = kept.iter().map(|r| r.user_ns - r.rx_sw_ns).collect();
        let (over, span_s) = excursions(&lat, kept.iter().map(|r| r.user_ns));
        let s = summarize(&mut lat);
        let p50 = s.as_ref().map_or(0, |s| s.p50);
        let mut slow: Vec<_> = kept.clone();
        slow.sort_by_key(|r| std::cmp::Reverse(r.user_ns - r.rx_sw_ns));
        // A slow decode is the receiving loop stalling while the message
        // waited in the socket, or something no trace saw.
        let worst_events: Vec<WorstEvent> = slow
            .iter()
            .take(5)
            .map(|r| WorstEvent {
                ns: r.user_ns - r.rx_sw_ns,
                #[allow(clippy::cast_precision_loss)]
                at_s: (r.user_ns - t0) as f64 / 1e9,
                cause: stalls::round_trip_cause(
                    &client,
                    &[],
                    r.rx_sw_ns,
                    r.user_ns,
                    p50,
                    stalls::SERVER_SLACK_NS,
                )
                .verdict,
            })
            .collect();
        (
            s,
            longest_stall.unwrap_or_default(),
            over,
            span_s,
            worst_events,
        )
    };
    Ok(Run {
        latency,
        loop_stats,
        seconds,
        causes,
        worst,
        over,
        span_s,
        worst_events,
        due,
        due_over,
        one_way,
        one_way_over,
        irqs,
        clock_bound_ns: clock_bound,
        work,
        send,
    })
}

/// Tags in run order, and the runs that could not be loaded (`tag/run`, why).
pub type Loaded = (Vec<Tag>, Vec<(String, String)>);

/// Every tag under `root` in the order they ran, and the runs that could not
/// be loaded (`tag/run`, why).
///
/// # Errors
/// `root` cannot be read.
pub fn load(root: &Path, warmup_s: i64) -> std::io::Result<Loaded> {
    let mut tags: Vec<(std::time::SystemTime, Tag)> = Vec::new();
    let mut skipped = Vec::new();
    for e in std::fs::read_dir(root)?.filter_map(Result::ok) {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let tag = e.file_name().to_string_lossy().into_owned();
        // Diagnosis runs are slowed by their tracing, and the gap maps are
        // spinners on every CPU: both reported apart (`diag`, `periodic`).
        if tag == "diag" || tag.starts_with("cpumap-") {
            continue;
        }
        let mut runs = HashMap::new();
        let mut first = None;
        for r in std::fs::read_dir(&p)?.filter_map(Result::ok) {
            let rp = r.path();
            if !rp.is_dir() {
                continue;
            }
            let name = r.file_name().to_string_lossy().into_owned();
            if let Ok(t) = std::fs::metadata(rp.join("meta.txt")).and_then(|m| m.modified()) {
                first = Some(first.map_or(t, |f: std::time::SystemTime| f.min(t)));
            }
            // A point the lab failed (its settings drifted, or the run did
            // not finish): left out, with why.
            if let Ok(why) = std::fs::read_to_string(rp.join("failed.txt")) {
                skipped.push((format!("{tag}/{name}"), format!("failed: {}", why.trim())));
                continue;
            }
            match load_run(&rp, warmup_s) {
                Ok(run) => {
                    runs.insert(name, run);
                }
                Err(why) => skipped.push((format!("{tag}/{name}"), why)),
            }
        }
        if let (Some(t), false) = (first, runs.is_empty()) {
            tags.push((t, Tag { name: tag, runs }));
        }
    }
    tags.sort_by_key(|t| t.0);
    skipped.sort();
    Ok((tags.into_iter().map(|(_, t)| t).collect(), skipped))
}

fn median(mut v: Vec<i64>) -> Option<i64> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    Some(v[v.len() / 2])
}

/// The baseline value of `f` for `run` as seen by `tag`: the median over the
/// same round's baselines (boot profiles use round 1; stage-2 tags use every
/// stage-2 baseline, measured interleaved with them).
fn baseline(tags: &[Tag], tag: &Tag, run: &str, f: impl Fn(&Summary) -> i64) -> Option<i64> {
    baseline_of(tags, tag, run, |r| r.latency.as_ref(), f)
}

/// As [`baseline`], for the summary `pick` chooses.
fn baseline_of(
    tags: &[Tag],
    tag: &Tag,
    run: &str,
    pick: impl Fn(&Run) -> Option<&Summary>,
    f: impl Fn(&Summary) -> i64,
) -> Option<i64> {
    let confirm = tag.is_confirm();
    let round = tag.round().max(1);
    median(
        tags.iter()
            .filter(|t| t.is_base())
            .filter(|t| {
                if confirm {
                    t.is_confirm()
                } else {
                    t.round() == round
                }
            })
            .filter_map(|t| pick(t.runs.get(run)?).map(&f))
            .collect(),
    )
}

fn with_delta(v: i64, base: Option<i64>, is_base: bool) -> String {
    match base {
        Some(b) if !is_base => {
            let d = v - b;
            format!(
                "{} ({}{})",
                us(v),
                if d >= 0 { "+" } else { "−" },
                us(d.abs())
            )
        }
        _ => us(v),
    }
}

#[allow(clippy::cast_precision_loss)]
fn per_min(n: u64, seconds: f64) -> String {
    if seconds > 0.0 {
        format!("{:.1}", n as f64 * 60.0 / seconds)
    } else {
        n.to_string()
    }
}

/// A second view of `kind`'s samples ([`Run::due`] or [`Run::one_way`]):
/// how to pick it and its excursion counts.
struct View {
    heading: &'static str,
    intro: &'static str,
    pick: fn(&Run) -> Option<&Summary>,
    over: fn(&Run) -> [u64; 4],
    /// Two hosts' clocks: say how far apart they may be.
    two_clocks: bool,
}

const VIEWS: [View; 2] = [
    View {
        heading: "from when each was due",
        intro: "The same round trips timed from their slot on the paced schedule, which never slips: \
                a stall delays every message due during it, and this includes that wait.",
        pick: |r| r.due.as_ref(),
        over: |r| r.due_over,
        two_clocks: false,
    },
    View {
        heading: "sender → decoded (one way)",
        intro: "Every message from its send time on the sender (or the exchange's event time) to \
                decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values \
                are relative only: each includes the offset between the clocks, which drifts, and \
                which chrony bounds by the sum of the hosts' |System time| + ½ root delay + root \
                dispersion (chronyc tracking). Compare runs only where they differ by much more than that.",
        pick: |r| r.one_way.as_ref(),
        over: |r| r.one_way_over,
        two_clocks: true,
    },
];

/// The table of `view` for `kind`, if any run has it.
fn view_table(out: &mut String, tags: &[Tag], kind: &str, title: &str, view: &View) {
    let pick = view.pick;
    if !tags
        .iter()
        .any(|t| t.runs.get(kind).and_then(pick).is_some())
    {
        return;
    }
    let bounds: Vec<i64> = tags
        .iter()
        .filter_map(|t| t.runs.get(kind))
        .filter(|r| pick(r).is_some())
        .filter_map(|r| r.clock_bound_ns)
        .collect();
    let clocks = match (view.two_clocks, bounds.iter().min(), bounds.iter().max()) {
        (false, _, _) => String::new(),
        (true, Some(lo), Some(hi)) => format!(
            " Here the bound was ±{}–{} µs (chrony's own estimate, at its default maxclockerror of \
             1 ppm: optimistic).",
            us(*lo),
            us(*hi)
        ),
        (true, _, _) => " These runs kept no chrony log, so the bound is not known.".to_owned(),
    };
    let _ = writeln!(
        out,
        "### {title}, {}\n\n{}{clocks}\n\n\
         | condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |\n\
         |---|---|---|---|---|---|---|---|---|---|---|",
        view.heading, view.intro
    );
    for t in tags {
        let Some(r) = t.runs.get(kind) else { continue };
        let Some(s) = pick(r) else { continue };
        let base = t.is_base();
        let b = |f: fn(&Summary) -> i64| baseline_of(tags, t, kind, pick, f);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            t.name,
            s.n,
            with_delta(s.p50, b(|x| x.p50), base),
            with_delta(s.p99, b(|x| x.p99), base),
            with_delta(s.p999, b(|x| x.p999), base),
            with_delta(s.p9999, b(|x| x.p9999), base),
            with_delta(s.max, b(|x| x.max), base),
            per_min((view.over)(r)[0], r.span_s),
            per_min((view.over)(r)[1], r.span_s),
            per_min((view.over)(r)[2], r.span_s),
            per_min((view.over)(r)[3], r.span_s),
        );
    }
    let _ = writeln!(out);
}

/// The work behind `kind`'s percentiles: what the run planned and what became
/// of it.
fn work_table(out: &mut String, tags: &[Tag], kind: &str) {
    if !tags
        .iter()
        .any(|t| t.runs.get(kind).and_then(|r| r.work).is_some())
    {
        return;
    }
    let _ = writeln!(
        out,
        "### Work behind the percentiles\n\n\
         Every percentile above is over `completed` alone, and the tables drop the warm-up, so `n` is \
         smaller still. `censored` is the issued work they leave out: no reply came back before the run \
         ended, or the reply was discarded because the record vector was full. `scheduled − issued` is \
         work the schedule called for that never went out, and `late` is the part issued when the next \
         slot was already due, so the run owed more than one. A WebSocket or HTTP client's send queue \
         grows instead of refusing, and a send \
         failure fails the run, so `rejected` is 0 on every run that reported. `not yet measured` is a \
         count the path does not produce, never a zero. On a receive-only run the send-side counts are \
         the sender's.\n\n\
         {WORK_HEADER}"
    );
    for t in tags {
        let Some(r) = t.runs.get(kind) else { continue };
        let Some(w) = r.work else { continue };
        let _ = writeln!(out, "{}", work_row(&t.name, &w));
    }
    let _ = writeln!(out);
}

/// The sender's schedule beside `kind`'s percentiles: whether the offered
/// traffic was the planned traffic. Reported separately from the work counts
/// because it counts a different population (see [`SendSchedule`]).
fn send_schedule_table(out: &mut String, tags: &[Tag], kind: &str) {
    if !tags
        .iter()
        .any(|t| t.runs.get(kind).and_then(|r| r.send).is_some())
    {
        return;
    }
    let _ = writeln!(
        out,
        "### Sender schedule behind those percentiles\n\n\
         `batches` are the due times the sender had traffic for — the order connection and the feeds \
         together, the first message of each — and `late >100 µs` is the part that began over 100 µs \
         after its due time. This is not `late` above: that counts the paced order slots filled when \
         the next slot was already due, and a run can show 2 of those and tens of thousands of late \
         batches at once. Traffic sent late is not the traffic planned (a saturated sender also slows \
         down with a slow receiver), so a row that is over 1% is measuring a load the run was not \
         offered on purpose.\n\n\
         {SEND_SCHEDULE_HEADER}"
    );
    for t in tags {
        let Some(r) = t.runs.get(kind) else { continue };
        let Some(s) = r.send else { continue };
        let _ = writeln!(out, "{}", send_schedule_row(&t.name, &s));
    }
    let _ = writeln!(out);
}

/// Interrupts per second on the loop CPU and the receiving CPU, every run.
fn irq_table(out: &mut String, tags: &[Tag]) {
    let mut kinds: Vec<&String> = tags
        .iter()
        .flat_map(|t| t.runs.iter())
        .filter(|(_, r)| !r.irqs.is_empty())
        .map(|(k, _)| k)
        .collect();
    kinds.sort();
    kinds.dedup();
    if kinds.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "## Interrupts per second on the loop CPU and the receiving CPU\n\n\
         From `/proc/interrupts` around each run (so including its start-up): local timer (LOC; on arm64 the \
         `arch_timer` interrupt), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), \
         rescheduling (RES) and function-call (CAL) IPIs (on arm64 the `IPIn` rows of those names), and \
         system-management interrupts (SMI, x86 bare metal only: firmware pauses). The receiving CPU is the one \
         that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a \
         softirq's.\n\n\
         | run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let fmt = |v: &[f64; 5]| {
        v.iter()
            .map(|x| format!("{x:.0}"))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    for kind in kinds {
        for t in tags {
            let Some(r) = t.runs.get(kind) else { continue };
            let Some((c, l)) = r.irqs.first() else {
                continue;
            };
            let rx = r.irqs.get(1).map_or_else(
                // The receiving CPU and its rates, empty.
                || [""; 6].join(" | "),
                |(c, v)| format!("{c} | {}", fmt(v)),
            );
            let _ = writeln!(out, "| {kind} | {} | {c} | {} | {rx} |", t.name, fmt(l));
        }
    }
    let _ = writeln!(out);
}

/// The matrix report; `skipped` lists runs that could not be loaded.
#[must_use]
pub fn report(tags: &[Tag], skipped: &[(String, String)]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Settings matrix\n\nConditions in run order; baselines (`rN-base-k`) bracket every two conditions and \
         each value shows its change against the median of the same round's baselines (boot profiles against \
         round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). \
         All values in µs. Every table keeps the maximum and names the cause of the worst case, and the \
         worst-events section lists the five slowest samples of every run with their causes. A maximum is \
         still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, \
         1 ms and 10 ms as well. Each kind's percentiles end with the work behind them — scheduled, \
         issued, completed, late, rejected and censored — so a shorter sample is never read as a \
         complete one.\n"
    );
    if !skipped.is_empty() {
        let _ = writeln!(out, "**Runs not included** (they could not be loaded):\n");
        for (run, why) in skipped {
            let _ = writeln!(out, "- `{run}`: {why}");
        }
        let _ = writeln!(out);
    }
    // Platform noise.
    let _ = writeln!(
        out,
        "## Platform noise on the loop CPU (a loop that only reads the counter)\n\n\
         | condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |\n|---|---|---|---|---|---|"
    );
    for t in tags {
        if let Some(r) = t.runs.get("noise") {
            let s = &r.loop_stats;
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                t.name,
                us(i64::try_from(s.max_ns).unwrap_or(0)),
                per_min(s.over[0], r.seconds),
                per_min(s.over[1], r.seconds),
                per_min(s.over[2], r.seconds),
                r.worst
            );
        }
    }
    let _ = writeln!(out);
    // Latency tables, one per run kind.
    let mut kinds: Vec<String> = tags
        .iter()
        .flat_map(|t| t.runs.keys().cloned())
        .filter(|k| k != "noise")
        .collect();
    kinds.sort();
    kinds.dedup();
    for kind in kinds {
        let title = if kind.starts_with("pp-") {
            format!("WebSocket round trip, {kind}")
        } else if kind.starts_with("rt-") {
            format!("WebSocket order round trip with feed connections on the loop, {kind}")
        } else if kind.starts_with("http-") {
            format!("HTTP round trip, {kind}")
        } else {
            format!("Market-data burst, kernel arrival → decoded (every message), {kind}")
        };
        let _ = writeln!(
            out,
            "## {title}\n\n| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
        );
        for t in tags {
            let Some(r) = t.runs.get(&kind) else { continue };
            let Some(s) = r.latency.as_ref() else {
                continue;
            };
            let base = t.is_base();
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                t.name,
                s.n,
                with_delta(s.p50, baseline(tags, t, &kind, |x| x.p50), base),
                with_delta(s.p99, baseline(tags, t, &kind, |x| x.p99), base),
                with_delta(s.p999, baseline(tags, t, &kind, |x| x.p999), base),
                with_delta(s.p9999, baseline(tags, t, &kind, |x| x.p9999), base),
                with_delta(s.max, baseline(tags, t, &kind, |x| x.max), base),
                per_min(r.over[0], r.span_s),
                per_min(r.over[1], r.span_s),
                per_min(r.over[2], r.span_s),
                per_min(r.over[3], r.span_s),
                us(i64::try_from(r.loop_stats.max_ns).unwrap_or(0)),
                per_min(r.loop_stats.over[1], r.seconds),
                r.worst
            );
        }
        let _ = writeln!(out);
        for view in &VIEWS {
            view_table(&mut out, tags, &kind, &title, view);
        }
        work_table(&mut out, tags, &kind);
        send_schedule_table(&mut out, tags, &kind);
    }
    irq_table(&mut out, tags);
    // The worst samples themselves, each with when and why.
    let mut kinds: Vec<String> = tags.iter().flat_map(|t| t.runs.keys().cloned()).collect();
    kinds.sort();
    kinds.dedup();
    let _ = writeln!(
        out,
        "## Worst events\n\nThe five slowest samples of every run (noise: longest gaps): value in µs, seconds into \
         the run, and cause.\n"
    );
    for kind in kinds {
        let _ = writeln!(out, "### {kind}\n\n| condition | worst events |\n|---|---|");
        for t in tags {
            let Some(r) = t.runs.get(&kind) else { continue };
            let ev = r
                .worst_events
                .iter()
                .map(|e| format!("{} at {:.1} s: {}", us(e.ns), e.at_s, e.cause))
                .collect::<Vec<_>>()
                .join("; ");
            let _ = writeln!(out, "| {} | {ev} |", t.name);
        }
        let _ = writeln!(out);
    }
    // Where stall time went.
    let _ = writeln!(
        out,
        "## Stall time by cause (all runs of a condition, loop iterations over 20 µs)\n\n| condition | causes (total µs) |\n|---|---|"
    );
    for t in tags {
        let mut total: HashMap<&str, u64> = HashMap::new();
        for r in t.runs.values() {
            for (c, ns) in &r.causes {
                *total.entry(c.as_str()).or_default() += ns;
            }
        }
        let mut v: Vec<_> = total.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        let causes = v
            .iter()
            .take(5)
            .map(|(c, ns)| format!("{c} {}", us(i64::try_from(*ns).unwrap_or(0))))
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(out, "| {} | {causes} |", t.name);
    }
    out.push('\n');
    out
}
