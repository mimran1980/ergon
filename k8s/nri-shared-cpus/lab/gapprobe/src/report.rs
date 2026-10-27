//! A run's summary row, the sampler's windows, what overlapped each max, and
//! the markdown report over every arm and rep.

use crate::stats::Summary;
use std::fmt::Write as _;

/// One spin run, as `gapprobe spin` writes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Run {
    /// `<arm>-<rep>`
    pub label: String,
    /// `plugin-on` or `plugin-off`
    pub arm: String,
    /// 1-based
    pub rep: u32,
    /// the CPU the loop ran on
    pub cpu: usize,
    /// the run's loop iterations
    pub summary: Summary,
    /// `CLOCK_REALTIME` ns at which the max ended
    pub max_end_ns: u64,
    /// gaps at or above the threshold that were logged
    pub gaps: u64,
    /// gaps over the log's capacity, counted only
    pub dropped: u64,
    /// voluntary context switches of the loop's thread during the run
    pub nvcsw: u64,
    /// involuntary context switches of the loop's thread during the run
    pub nivcsw: u64,
    /// `CLOCK_REALTIME` ns the loop started (0 in a file from before it was kept)
    pub start_ns: u64,
    /// `CLOCK_REALTIME` ns the loop ended (0 likewise)
    pub end_ns: u64,
    /// minor page faults of the loop's thread during the run (0 likewise)
    pub minflt: u64,
    /// major page faults of the loop's thread during the run (0 likewise)
    pub majflt: u64,
}

/// A run's file had a field missing or not a number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowError(pub String);

impl std::fmt::Display for RowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RowError {}

/// The summary file's header.
pub const RUN_HEADER: &str = "label\tarm\trep\tcpu\tsamples\tp50_ns\tp90_ns\tp99_ns\tp999_ns\tp9999_ns\tmax_ns\tmax_end_ns\tgaps\tdropped\tnvcsw\tnivcsw\tstart_ns\tend_ns\tminflt\tmajflt";

/// Fields of a row written before the run's start, end and faults were kept.
const LEGACY_FIELDS: usize = 16;

impl Run {
    /// The run as one tab-separated row under `RUN_HEADER`.
    #[must_use]
    pub fn to_row(&self) -> String {
        let s = &self.summary;
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.label,
            self.arm,
            self.rep,
            self.cpu,
            s.samples,
            s.p50,
            s.p90,
            s.p99,
            s.p999,
            s.p9999,
            s.max,
            self.max_end_ns,
            self.gaps,
            self.dropped,
            self.nvcsw,
            self.nivcsw,
            self.start_ns,
            self.end_ns,
            self.minflt,
            self.majflt
        )
    }

    /// Parses a row `to_row` wrote, or one of the 16 fields written before
    /// the start, end and faults were kept (those read 0).
    ///
    /// # Errors
    /// A field missing, extra, or not a number.
    pub fn from_row(row: &str) -> Result<Self, RowError> {
        let f: Vec<&str> = row.trim_end_matches(['\r', '\n']).split('\t').collect();
        let want = RUN_HEADER.split('\t').count();
        if f.len() != want && f.len() != LEGACY_FIELDS {
            return Err(RowError(format!(
                "summary row: {} fields, want {want} (or {LEGACY_FIELDS}): {row}",
                f.len()
            )));
        }
        let legacy = f.len() == LEGACY_FIELDS;
        let n = |i: usize| -> Result<u64, RowError> {
            f.get(i).and_then(|v| v.parse::<u64>().ok()).ok_or_else(|| {
                RowError(format!("summary row field {}: not a number: {row}", i + 1))
            })
        };
        Ok(Self {
            label: f.first().map(ToString::to_string).unwrap_or_default(),
            arm: f.get(1).map(ToString::to_string).unwrap_or_default(),
            rep: u32::try_from(n(2)?).map_err(|_| RowError(format!("summary row rep: {row}")))?,
            cpu: usize::try_from(n(3)?).map_err(|_| RowError(format!("summary row cpu: {row}")))?,
            summary: Summary {
                samples: n(4)?,
                p50: n(5)?,
                p90: n(6)?,
                p99: n(7)?,
                p999: n(8)?,
                p9999: n(9)?,
                max: n(10)?,
            },
            max_end_ns: n(11)?,
            gaps: n(12)?,
            dropped: n(13)?,
            nvcsw: n(14)?,
            nivcsw: n(15)?,
            start_ns: if legacy { 0 } else { n(16)? },
            end_ns: if legacy { 0 } else { n(17)? },
            minflt: if legacy { 0 } else { n(18)? },
            majflt: if legacy { 0 } else { n(19)? },
        })
    }
}

/// A sampler window: counter `name` on the probe's CPU grew by `delta`
/// between `start_ns` and `end_ns` (`CLOCK_REALTIME`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    /// the read before
    pub start_ns: u64,
    /// the read after
    pub end_ns: u64,
    /// the counter, as `irq::column` names it
    pub name: String,
    /// its growth
    pub delta: u64,
}

impl Window {
    /// `start_ns end_ns delta name`, tab-separated.
    #[must_use]
    pub fn to_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            self.start_ns, self.end_ns, self.delta, self.name
        )
    }

    /// Parses a row `to_row` wrote.
    ///
    /// # Errors
    /// A field missing or not a number.
    pub fn from_row(row: &str) -> Result<Self, RowError> {
        let mut f = row.trim_end_matches(['\r', '\n']).splitn(4, '\t');
        let mut n = || -> Result<u64, RowError> {
            f.next()
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| RowError(format!("window row: not a number: {row}")))
        };
        let (start_ns, end_ns, delta) = (n()?, n()?, n()?);
        let name = f
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| RowError(format!("window row: no name: {row}")))?;
        Ok(Self {
            start_ns,
            end_ns,
            name: name.to_string(),
            delta,
        })
    }
}

/// The counters that grew in any window intersecting `[start_ns, end_ns]`,
/// summed by name, largest first (then by name). The windows are the
/// sampler's interval wide, so what is listed overlapped the gap or sat
/// within one interval of it.
#[must_use]
pub fn overlapping(windows: &[Window], start_ns: u64, end_ns: u64) -> Vec<(String, u64)> {
    let mut sum: Vec<(String, u64)> = Vec::new();
    for w in windows
        .iter()
        .filter(|w| w.end_ns >= start_ns && w.start_ns <= end_ns)
    {
        match sum.iter_mut().find(|(n, _)| *n == w.name) {
            Some((_, d)) => *d += w.delta,
            None => sum.push((w.name.clone(), w.delta)),
        }
    }
    sum.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    sum
}

/// The median of `v` (the lower of the middle two for an even count), or 0.
#[must_use]
pub fn median(v: &[u64]) -> u64 {
    let mut s = v.to_vec();
    s.sort_unstable();
    s.get(s.len().saturating_sub(1) / 2).copied().unwrap_or(0)
}

/// ns as µs, two decimals.
fn us(ns: u64) -> String {
    format!("{}.{:02}", ns / 1000, (ns % 1000) / 10)
}

/// The report: one row per run with what overlapped its max (at most `top`
/// counters; one with no growth is a note, printed bare), then one row per arm: the median over its reps of each
/// percentile, and the max of the maxima.
#[must_use]
pub fn render(runs: &[(Run, Vec<(String, u64)>)], top: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## Per arm (median over reps; max of the maxima)\n");
    let _ = writeln!(
        out,
        "| arm | reps | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | max in rep |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|");
    let mut arms: Vec<&str> = Vec::new();
    for (r, _) in runs {
        if !arms.contains(&r.arm.as_str()) {
            arms.push(&r.arm);
        }
    }
    for arm in &arms {
        let reps: Vec<&Run> = runs
            .iter()
            .map(|(r, _)| r)
            .filter(|r| r.arm == *arm)
            .collect();
        let col = |f: fn(&Summary) -> u64| {
            median(&reps.iter().map(|r| f(&r.summary)).collect::<Vec<_>>())
        };
        let worst = reps.iter().max_by_key(|r| r.summary.max);
        let _ = writeln!(
            out,
            "| {arm} | {} | {} | {} | {} | {} | {} | {} | {} |",
            reps.len(),
            us(col(|s| s.p50)),
            us(col(|s| s.p90)),
            us(col(|s| s.p99)),
            us(col(|s| s.p999)),
            us(col(|s| s.p9999)),
            us(worst.map_or(0, |r| r.summary.max)),
            worst.map_or(0, |r| r.rep),
        );
    }
    let _ = writeln!(out, "\n## Per run\n");
    let _ = writeln!(
        out,
        "| arm | rep | cpu | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | max ended (CLOCK_REALTIME ns) | gaps logged | gaps dropped | involuntary switches | page faults | overlapping the max (counter +growth on its CPU) |"
    );
    let _ = writeln!(
        out,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for (r, over) in runs {
        let s = &r.summary;
        let what = if over.is_empty() {
            "nothing counted".to_string()
        } else {
            over.iter()
                .take(top)
                .map(|(n, d)| {
                    if *d == 0 {
                        n.clone()
                    } else {
                        format!("{n} +{d}")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {what} |",
            r.arm,
            r.rep,
            r.cpu,
            us(s.p50),
            us(s.p90),
            us(s.p99),
            us(s.p999),
            us(s.p9999),
            us(s.max),
            r.max_end_ns,
            r.gaps,
            r.dropped,
            r.nivcsw,
            if r.start_ns == 0 {
                "not kept".to_string()
            } else {
                (r.minflt + r.majflt).to_string()
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(arm: &str, rep: u32, p50: u64, max: u64) -> Run {
        Run {
            label: format!("{arm}-{rep}"),
            arm: arm.to_string(),
            rep,
            cpu: 2,
            summary: Summary {
                samples: 1000,
                p50,
                p90: p50,
                p99: p50 * 2,
                p999: p50 * 3,
                p9999: p50 * 4,
                max,
            },
            max_end_ns: 1_700_000_000_000_000_000 + u64::from(rep),
            gaps: 3,
            dropped: 0,
            nvcsw: 1,
            nivcsw: 2,
            start_ns: 1_699_999_940_000_000_000,
            end_ns: 1_700_000_000_000_000_000,
            minflt: 4,
            majflt: 0,
        }
    }

    #[test]
    fn run_row_round_trips() {
        let r = run("plugin-on", 3, 40, 12_345);
        let row = r.to_row();
        assert_eq!(row.split('\t').count(), RUN_HEADER.split('\t').count());
        assert_eq!(Run::from_row(&row), Ok(r));
    }

    #[test]
    fn run_row_refuses_a_bad_field() {
        assert!(Run::from_row("a\tb").is_err());
        let row = run("plugin-on", 1, 40, 99)
            .to_row()
            .replace("\t99\t", "\tx\t");
        assert!(Run::from_row(&row).is_err());
        let extra = format!("{}\t7", run("plugin-on", 1, 40, 99).to_row());
        assert!(Run::from_row(&extra).is_err());
    }

    #[test]
    fn run_row_of_16_fields_reads_start_end_and_faults_as_0() {
        // a summary.tsv row of run 20261024212712
        let row = "plugin-on-1\tplugin-on\t1\t2\t2498590505\t24\t24\t32\t32\t32\t17400\t1792877670347178875\t6318\t0\t0\t6";
        let r = Run::from_row(row).map_err(|e| e.0);
        assert_eq!(
            r.as_ref()
                .map(|r| (r.nivcsw, r.summary.max, r.start_ns, r.minflt)),
            Ok((6, 17_400, 0, 0))
        );
    }

    #[test]
    fn window_row_round_trips_with_spaces_in_the_name() {
        let w = Window {
            start_ns: 10,
            end_ns: 20,
            name: "IPI0 Rescheduling interrupts".into(),
            delta: 4,
        };
        assert_eq!(Window::from_row(&w.to_row()), Ok(w));
        assert!(Window::from_row("1\t2\tx\tname").is_err());
    }

    #[test]
    fn overlapping_sums_by_name_largest_first() {
        let w = |s, e, n: &str, d| Window {
            start_ns: s,
            end_ns: e,
            name: n.into(),
            delta: d,
        };
        let windows = vec![
            w(0, 10, "timer", 1),         // ends before the gap
            w(10, 20, "timer", 2),        // touches its start: counted
            w(20, 30, "IPI0 resched", 1), // inside
            w(20, 30, "timer", 1),
            w(30, 40, "softirq:RCU", 5), // touches its end
            w(41, 50, "softirq:RCU", 9), // after
        ];
        assert_eq!(
            overlapping(&windows, 20, 30),
            vec![
                ("softirq:RCU".to_string(), 5),
                ("timer".to_string(), 3),
                ("IPI0 resched".to_string(), 1)
            ]
        );
        assert_eq!(overlapping(&windows, 100, 200), Vec::<(String, u64)>::new());
    }

    #[test]
    fn median_of_odd_even_and_empty() {
        assert_eq!(median(&[5, 1, 3]), 3);
        assert_eq!(median(&[4, 1, 3, 2]), 2);
        assert_eq!(median(&[]), 0);
    }

    #[test]
    fn render_lists_every_run_and_each_arm() {
        let runs = vec![
            (
                run("plugin-on", 1, 40, 2_500),
                vec![("10 arch_timer".to_string(), 1)],
            ),
            (run("plugin-on", 2, 60, 9_000), vec![]),
            (
                run("plugin-on", 4, 60, 100),
                vec![("no sampler file".into(), 0)],
            ),
            (
                run("plugin-on", 3, 50, 3_000),
                vec![("a".into(), 3), ("b".into(), 2), ("c".into(), 1)],
            ),
            (
                run("plugin-off", 1, 45, 4_000_000),
                vec![("IPI0 Rescheduling interrupts".to_string(), 7)],
            ),
        ];
        let md = render(&runs, 2);
        // per run: the max with what overlapped it, at most two counters
        assert!(md.contains("| plugin-on | 1 | 2 | 0.04 | 0.04 | 0.08 | 0.12 | 0.16 | 2.50 | 1700000000000000001 | 3 | 0 | 2 | 4 | 10 arch_timer +1 |"), "{md}");
        assert!(md.contains("| plugin-on | 2 | 2 | 0.06 | 0.06 | 0.12 | 0.18 | 0.24 | 9.00 | 1700000000000000002 | 3 | 0 | 2 | 4 | nothing counted |"), "{md}");
        assert!(md.contains("| a +3, b +2 |"), "{md}");
        assert!(md.contains("| no sampler file |"), "{md}");
        // per arm: medians of the reps, max of the maxima
        assert!(
            md.contains("| plugin-on | 4 | 0.05 | 0.05 | 0.10 | 0.15 | 0.20 | 9.00 | 2 |"),
            "{md}"
        );
        assert!(
            md.contains("| plugin-off | 1 | 0.04 | 0.04 | 0.09 | 0.13 | 0.18 | 4000.00 |"),
            "{md}"
        );
        // arms in first-seen order
        let on = md.find("| plugin-on | 4 |").unwrap_or(usize::MAX);
        let off = md.find("| plugin-off | 1 | 0.04").unwrap_or(0);
        assert!(on < off, "{md}");
    }
}
