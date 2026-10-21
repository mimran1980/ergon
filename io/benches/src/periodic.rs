//! Once-a-minute stalls: whether the long stalls and slow samples of a run
//! bunch at one second of the wall-clock minute (a guest agent or a host task
//! on a one-minute timer) instead of falling at random.
//!
//! Each source is reported per run kind over every condition, and each
//! diagnosis run (`diag/<name>`) on its own row, so the guest-agent A/B shows
//! whether stopping the agents removes the peak. The gap maps
//! (`cpumap-<host>/cpu<N>`, a counter-only spinner on every isolated CPU at
//! once before the runs) get a row per CPU: which CPUs the stall reaches.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::record::read_records;
use crate::stalls::{parse_stalls, server_stall_files};
use crate::stats::us;

/// Events this long or longer are counted, nanoseconds.
pub const PERIODIC_NS: i64 = 300_000;

const MINUTE_NS: i64 = 60_000_000_000;
const SECOND_NS: i64 = 1_000_000_000;

/// One source's events over one or more runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Series {
    /// Each event: its end (wall clock, ns) and its length (ns).
    pub events: Vec<(i64, i64)>,
    /// The wall-clock spans (ns) the runs covered.
    pub spans: Vec<(i64, i64)>,
}

/// The busiest three seconds of the minute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    /// Its middle second (0–59).
    pub second: u8,
    /// Events inside it.
    pub count: usize,
    /// Their share of all events.
    pub share: f64,
    /// Distinct minutes with an event inside it.
    pub minutes_hit: usize,
    /// Minutes whose three seconds the runs reached.
    pub minutes_covered: i64,
    /// The longest event inside it, ns.
    pub longest_ns: i64,
}

impl Peak {
    /// At least five events, and at least a quarter of them in these three
    /// seconds (chance is one in twenty).
    #[must_use]
    pub fn is_periodic(&self) -> bool {
        self.count >= 5 && self.share >= 0.25
    }
}

fn second_of_minute(t: i64) -> i64 {
    t.rem_euclid(MINUTE_NS) / SECOND_NS
}

/// Minutes `m` whose three seconds from `start` (`m * 60 s + start`, for
/// 3 s) overlap a span.
fn covered(spans: &[(i64, i64)], start: i64) -> i64 {
    let off = start * SECOND_NS;
    spans
        .iter()
        .map(|&(s, e)| {
            let first = (s - 3 * SECOND_NS - off).div_euclid(MINUTE_NS) + 1;
            let last = (e - off).div_euclid(MINUTE_NS);
            (last - first + 1).max(0)
        })
        .sum()
}

/// The busiest three-second window of the minute, or `None` without events.
#[must_use]
pub fn peak(s: &Series) -> Option<Peak> {
    if s.events.is_empty() {
        return None;
    }
    let mut hist = [0usize; 60];
    for &(t, _) in &s.events {
        // In range: `second_of_minute` is 0..60.
        hist[usize::try_from(second_of_minute(t)).unwrap_or(0)] += 1;
    }
    let window = |c: usize| hist[(c + 59) % 60] + hist[c] + hist[(c + 1) % 60];
    // Of equal windows, the one whose middle second holds most, then the
    // first.
    let mut best = 0;
    for c in 1..60 {
        if (window(c), hist[c]) > (window(best), hist[best]) {
            best = c;
        }
    }
    let centre = i64::try_from(best).unwrap_or(0);
    // Minutes counted from the window's first second, so a window across
    // :00 stays in one minute.
    let start = (centre + 59) % 60;
    let inside: Vec<&(i64, i64)> = s
        .events
        .iter()
        .filter(|(t, _)| (second_of_minute(*t) - start).rem_euclid(60) < 3)
        .collect();
    let minutes_hit = inside
        .iter()
        .map(|(t, _)| (t - start * SECOND_NS).div_euclid(MINUTE_NS))
        .collect::<BTreeSet<_>>()
        .len();
    #[allow(clippy::cast_precision_loss)]
    let share = inside.len() as f64 / s.events.len() as f64;
    Some(Peak {
        second: u8::try_from(best).unwrap_or(0),
        count: inside.len(),
        share,
        minutes_hit,
        minutes_covered: covered(&s.spans, start),
        longest_ns: inside.iter().map(|e| e.1).max().unwrap_or(0),
    })
}

/// The run directories under `root`: `(kind, dir)`, where the kind is the run
/// name (`pp-recv-busy-10000`, …), or `diag/<name>` and `cpumap-<host>/cpu<N>`
/// for diagnosis runs and gap maps.
fn run_dirs(root: &Path) -> Vec<(String, PathBuf)> {
    let subdirs = |p: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(p)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        v.sort();
        v
    };
    let name = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for tag in subdirs(root) {
        let t = name(&tag);
        let apart = t == "diag" || t.starts_with("cpumap-");
        for run in subdirs(&tag) {
            if run.join("meta.txt").exists() {
                let kind = if apart {
                    format!("{t}/{}", name(&run))
                } else {
                    name(&run)
                };
                out.push((kind, run));
            } else if apart {
                // A diagnosis step that ran a usual measurement
                // (`diag/agents-on-1/pp-recv-busy-10000`): one row per step.
                for inner in subdirs(&run) {
                    if inner.join("meta.txt").exists() {
                        out.push((format!("{t}/{}", name(&run)), inner));
                    }
                }
            }
        }
    }
    out
}

/// Events (end, length) and the span (start, end) they cover, wall clock,
/// ns; no span without stalls.
type Events = (Vec<(i64, i64)>, Option<(i64, i64)>);

/// A stall file's stalls of [`PERIODIC_NS`] or more, and the span all its
/// stalls cover (for runs without records); `None` if there is no file.
fn stall_events(path: &Path) -> Option<Events> {
    let (_, st) = parse_stalls(&std::fs::read_to_string(path).ok()?).ok()?;
    let len = |ns: u64| i64::try_from(ns).unwrap_or(i64::MAX);
    let lo = st.iter().map(|s| s.end_real_ns - len(s.ns)).min();
    let hi = st.iter().map(|s| s.end_real_ns).max();
    let ev = st
        .iter()
        .filter(|s| len(s.ns) >= PERIODIC_NS)
        .map(|s| (s.end_real_ns, len(s.ns)))
        .collect();
    Some((ev, lo.zip(hi)))
}

/// Every source's series under `root`, keyed by `(kind, source)`.
#[must_use]
pub fn collect(root: &Path) -> BTreeMap<(String, &'static str), Series> {
    let mut map: BTreeMap<(String, &'static str), Series> = BTreeMap::new();
    for (kind, dir) in run_dirs(root) {
        let mode = std::fs::read_to_string(dir.join("meta.txt"))
            .unwrap_or_default()
            .lines()
            .find_map(|l| l.strip_prefix("mode=").map(str::to_owned))
            .unwrap_or_default();
        // The samples, minus the first second (connection warm-up).
        let records = read_records(&dir.join("records.bin")).unwrap_or_default();
        let t0 = records.iter().map(|r| r.user_ns).min();
        let t1 = records.iter().map(|r| r.user_ns).max();
        let record_span = t0.zip(t1).map(|(a, b)| (a + SECOND_NS, b));
        let keep = |t: i64| record_span.is_none_or(|(a, _)| t >= a);
        if let Some(span) = record_span {
            let slow: Vec<(i64, i64)> = records
                .iter()
                .filter(|r| keep(r.user_ns))
                .filter_map(|r| {
                    let lat = if mode == "pingpong" {
                        r.user_ns - r.src_ns
                    } else if r.rx_sw_ns > 0 {
                        r.user_ns - r.rx_sw_ns
                    } else {
                        return None;
                    };
                    (lat >= PERIODIC_NS).then_some((r.user_ns, lat))
                })
                .collect();
            let e = map.entry((kind.clone(), "slow samples")).or_default();
            e.events.extend(slow);
            e.spans.push(span);
        }
        let loop_source = if kind.starts_with("cpumap-") {
            "counter-only spinner"
        } else if mode == "noise" {
            "noise probe (loop CPU)"
        } else {
            "receiver loop"
        };
        let mut files = vec![
            (dir.join("stalls-0.txt"), loop_source),
            (dir.join("spare").join("stalls-0.txt"), "idle spare core"),
        ];
        files.extend(
            server_stall_files(&dir)
                .into_iter()
                .map(|f| (f, "sender loop")),
        );
        for (f, source) in files {
            let Some((ev, own_span)) = stall_events(&f) else {
                continue;
            };
            let e = map.entry((kind.clone(), source)).or_default();
            e.events.extend(ev.into_iter().filter(|(t, _)| keep(*t)));
            // The sender may have several stall files for one run: one span.
            if let Some(span) = record_span.or(own_span)
                && !e.spans.contains(&span)
            {
                e.spans.push(span);
            }
        }
    }
    map
}

/// The report section for `root`.
#[must_use]
pub fn report(root: &Path) -> String {
    let map = collect(root);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "## Once-a-minute stalls\n\nWhether each source's events of {} µs or more (loop stalls, or samples that slow) \
         bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; \
         three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events \
         is marked periodic. Each host has its own second (its agents' timers start at boot).\n\n\
         | run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |\n\
         |---|---|---|---|---|---|---|---|",
        PERIODIC_NS / 1000
    );
    for ((kind, source), s) in &map {
        let Some(p) = peak(s) else {
            let _ = writeln!(out, "| {kind} | {source} | 0 | | | | | |");
            continue;
        };
        let _ = writeln!(
            out,
            "| {kind} | {source} | {} | :{:02} | {} ({:.0}%) | {} of {} | {} | {} |",
            s.events.len(),
            p.second,
            p.count,
            p.share * 100.0,
            p.minutes_hit,
            p.minutes_covered,
            us(p.longest_ns),
            if p.is_periodic() { "**yes**" } else { "no" }
        );
    }
    out.push('\n');
    out
}
