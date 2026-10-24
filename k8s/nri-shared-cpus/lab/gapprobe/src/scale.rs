//! The shared-CPU scaling report: for each case the lab's `vm.sh scale`
//! ran, how busy each shared and exclusive CPU was, how much of the shared
//! CPUs each pod's helpers took, its cgroup's weight, quota and throttling,
//! where its threads ran, and its spin thread's worst gap.
//!
//! A case is a directory the lab writes:
//!
//! ```text
//! meta.txt                  case= title= shared= exclusive= window_ns= expect=
//! proc.stat.before/.after   /proc/stat on the VM around the window
//! <pod>.meta                cpu= millicpus= threads= duty=
//! <pod>.cpu.stat.before/.after, <pod>.cpu.weight, <pod>.cpu.max
//!                           the container's cgroup; <pod>.pod.cpu.* the pod's
//! <pod>.hot.schedstat.before/.after   the spin thread's schedstat
//! <pod>.psr.tsv             "t tid psr allowed comm" every second
//! <pod>.summary.tsv         the spin's summary (gapprobe spin)
//! ```

use crate::procfs::{self, CpuTimes, ParseError};
use crate::report::Run;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// `key=value` lines.
fn kv(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix('=').map(str::to_string))
}

/// The CPUs of a cpulist (`1-2`, `3,5-7`).
#[must_use]
pub fn cpus(list: &str) -> Vec<usize> {
    let mut v = Vec::new();
    for part in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (a, b) = part.split_once('-').unwrap_or((part, part));
        if let (Ok(a), Ok(b)) = (a.parse::<usize>(), b.parse::<usize>()) {
            v.extend(a..=b);
        }
    }
    v
}

/// One pod of a case, as read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pod {
    /// its name
    pub name: String,
    /// exclusive CPUs asked
    pub cpu: String,
    /// ergon.io/shared-millicpus asked
    pub millicpus: String,
    /// helper threads and their duty, as run
    pub threads: String,
    /// percent of each period a helper is busy
    pub duty: String,
    /// the `ERGON_SHARED_CPUS` its probe saw (the shared CPUs it was given)
    pub shared_cpus: String,
    /// container and pod cgroup `cpu.weight`
    pub weight: (String, String),
    /// container and pod cgroup `cpu.max`
    pub max: (String, String),
    /// container cgroup usage over the window, ns
    pub usage_ns: u64,
    /// the spin thread's CPU time over the window, ns
    pub hot_ns: u64,
    /// `nr_periods`, `nr_throttled`, `throttled_usec` grown over the window
    pub throttle: (u64, u64, u64),
    /// the spin's summary
    pub run: Option<Run>,
    /// CPUs each helper thread was seen on: (cpu, samples)
    pub helper_cpus: Vec<(usize, u64)>,
    /// CPUs the spin thread was seen on
    pub hot_cpus: Vec<(usize, u64)>,
    /// samples taken
    pub samples: u64,
}

impl Pod {
    /// The helpers' CPU time over the window: the container's less the spin
    /// thread's, ns.
    #[must_use]
    pub fn helpers_ns(&self) -> u64 {
        self.usage_ns.saturating_sub(self.hot_ns)
    }
}

/// One case, as read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Case {
    /// directory name
    pub case: String,
    /// what it runs
    pub title: String,
    /// shared CPUs
    pub shared: Vec<usize>,
    /// exclusive CPUs
    pub exclusive: Vec<usize>,
    /// the measured window, ns
    pub window_ns: u64,
    /// what the case is expected to show
    pub expect: String,
    /// busy fraction of each CPU over the window, in millicores
    pub busy_milli: Vec<(usize, u64)>,
    /// the pods
    pub pods: Vec<Pod>,
}

fn read(dir: &Path, name: &str) -> Result<String, ParseError> {
    fs::read_to_string(dir.join(name))
        .map_err(|e| ParseError(format!("{}: {e}", dir.join(name).display())))
}

/// A one-line file's text, or `?` when missing or empty.
fn opt(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "?".into())
}

/// Busy millicores of each CPU between two `/proc/stat` reads.
///
/// # Errors
/// Either read does not parse.
pub fn busy_milli(before: &str, after: &str) -> Result<Vec<(usize, u64)>, ParseError> {
    let b = procfs::proc_stat(before)?;
    let a = procfs::proc_stat(after)?;
    Ok(a.iter()
        .filter_map(|(cpu, t1)| {
            let t0: CpuTimes = b.iter().find(|(c, _)| c == cpu)?.1;
            let total = t1.total.checked_sub(t0.total)?;
            let busy = t1.busy.checked_sub(t0.busy)?;
            (total > 0).then(|| (*cpu, busy * 1000 / total))
        })
        .collect())
}

/// `(cpu, samples)` pairs, ascending by CPU.
pub type Seen = Vec<(usize, u64)>;

/// The CPUs seen in `psr.tsv` lines, split into the spin thread's
/// (`gp-hot`) and every other thread's, and the number of samples.
#[must_use]
pub fn psr(text: &str) -> (Seen, Seen, u64) {
    let mut hot: Vec<(usize, u64)> = Vec::new();
    let mut helpers: Vec<(usize, u64)> = Vec::new();
    let mut ts: Vec<&str> = Vec::new();
    for l in text.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        let (Some(t), Some(cpu), Some(comm)) = (
            f.first(),
            f.get(2).and_then(|c| c.parse::<usize>().ok()),
            f.get(4),
        ) else {
            continue;
        };
        if !ts.contains(t) {
            ts.push(t);
        }
        let v = if *comm == "gp-hot" {
            &mut hot
        } else {
            &mut helpers
        };
        match v.iter_mut().find(|x| x.0 == cpu) {
            Some(x) => x.1 += 1,
            None => v.push((cpu, 1)),
        }
    }
    hot.sort_unstable();
    helpers.sort_unstable();
    (hot, helpers, ts.len() as u64)
}

/// Reads one pod's files.
///
/// # Errors
/// A cgroup or schedstat file missing or malformed.
pub fn read_pod(dir: &Path, name: &str) -> Result<Pod, ParseError> {
    let meta = read(dir, &format!("{name}.meta"))?;
    let s0 = read(dir, &format!("{name}.cpu.stat.before"))?;
    let s1 = read(dir, &format!("{name}.cpu.stat.after"))?;
    let grown = |k: &str| -> Result<u64, ParseError> {
        match (procfs::keyed(&s0, k), procfs::keyed(&s1, k)) {
            (Some(a), Some(b)) => Ok(b.saturating_sub(a)),
            _ => Err(ParseError(format!("{name}: cpu.stat has no {k}"))),
        }
    };
    let h0 = procfs::schedstat_ns(&read(dir, &format!("{name}.hot.schedstat.before"))?)?;
    let h1 = procfs::schedstat_ns(&read(dir, &format!("{name}.hot.schedstat.after"))?)?;
    let run = fs::read_to_string(dir.join(format!("{name}.summary.tsv")))
        .ok()
        .and_then(|t| t.lines().nth(1).and_then(|r| Run::from_row(r).ok()));
    let (hot_cpus, helper_cpus, samples) =
        psr(&fs::read_to_string(dir.join(format!("{name}.psr.tsv"))).unwrap_or_default());
    Ok(Pod {
        name: name.to_string(),
        cpu: kv(&meta, "cpu").unwrap_or_default(),
        millicpus: kv(&meta, "millicpus").unwrap_or_default(),
        threads: kv(&meta, "threads").unwrap_or_default(),
        duty: kv(&meta, "duty").unwrap_or_default(),
        shared_cpus: kv(&meta, "shared_cpus")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "?".into()),
        weight: (
            opt(dir, &format!("{name}.cpu.weight")),
            opt(dir, &format!("{name}.pod.cpu.weight")),
        ),
        max: (
            opt(dir, &format!("{name}.cpu.max")),
            opt(dir, &format!("{name}.pod.cpu.max")),
        ),
        usage_ns: grown("usage_usec")? * 1000,
        hot_ns: h1.saturating_sub(h0),
        throttle: (
            grown("nr_periods")?,
            grown("nr_throttled")?,
            grown("throttled_usec")?,
        ),
        run,
        helper_cpus,
        hot_cpus,
        samples,
    })
}

/// Reads one case directory.
///
/// # Errors
/// `meta.txt`, a `/proc/stat` read or a pod's files missing or malformed.
pub fn read_case(dir: &Path) -> Result<Case, ParseError> {
    let meta = read(dir, "meta.txt")?;
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| ParseError(format!("{}: {e}", dir.display())))?
        .filter_map(Result::ok)
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_suffix(".meta")
                .map(str::to_string)
        })
        .collect();
    names.sort();
    let pods = names
        .iter()
        .map(|n| read_pod(dir, n))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Case {
        case: kv(&meta, "case").unwrap_or_default(),
        title: kv(&meta, "title").unwrap_or_default(),
        shared: cpus(&kv(&meta, "shared").unwrap_or_default()),
        exclusive: cpus(&kv(&meta, "exclusive").unwrap_or_default()),
        window_ns: kv(&meta, "window_ns")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        expect: kv(&meta, "expect").unwrap_or_default(),
        busy_milli: busy_milli(
            &read(dir, "proc.stat.before")?,
            &read(dir, "proc.stat.after")?,
        )?,
        pods,
    })
}

/// Millicores as cores, two decimals.
fn cores(milli: u64) -> String {
    format!("{}.{:02}", milli / 1000, (milli % 1000) / 10)
}

fn seen(v: &[(usize, u64)]) -> String {
    if v.is_empty() {
        return "not sampled".into();
    }
    v.iter()
        .map(|(c, n)| format!("{c} ({n})"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The markdown for one case.
#[must_use]
pub fn render_case(c: &Case) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## {}: {}\n", c.case, c.title);
    let _ = writeln!(out, "Expected: {}\n", c.expect);
    let _ = writeln!(
        out,
        "Window {} s. Busy CPU from /proc/stat on the VM (every task, not only the pods').\n",
        c.window_ns / 1_000_000_000
    );
    let _ = writeln!(out, "| CPU | role | busy, cores |\n|---|---|---|");
    let mut shared_milli = 0;
    for (cpu, m) in &c.busy_milli {
        let role = if c.shared.contains(cpu) {
            shared_milli += m;
            "shared"
        } else if c.exclusive.contains(cpu) {
            "exclusive"
        } else {
            continue;
        };
        let _ = writeln!(out, "| {cpu} | {role} | {} |", cores(*m));
    }
    let _ = writeln!(
        out,
        "\nShared CPUs busy: {} of {} cores.\n",
        cores(shared_milli),
        c.shared.len()
    );
    let _ = writeln!(
        out,
        "| pod | exclusive cpu | shared-millicpus | its shared CPUs | helper threads x duty % | cpu.weight container / pod | cpu.max container / pod | helpers' CPU, cores | share of the shared CPUs' busy time | nr_periods / nr_throttled / throttled_usec grown | spin max µs | spin involuntary switches | spin thread's CPUs (samples) | its other threads' CPUs (samples) |"
    );
    let _ = writeln!(
        out,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for p in &c.pods {
        let helpers_milli = (p.helpers_ns() * 1000)
            .checked_div(c.window_ns)
            .unwrap_or(0);
        let share = (helpers_milli * 100)
            .checked_div(shared_milli)
            .map_or_else(|| "-".to_string(), |s| format!("{s} %"));
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} x {} | {} / {} | {} / {} | {} | {share} | {} / {} / {} | {} | {} | {} | {} |",
            p.name,
            p.cpu,
            p.millicpus,
            p.shared_cpus,
            p.threads,
            p.duty,
            p.weight.0,
            p.weight.1,
            p.max.0,
            p.max.1,
            cores(helpers_milli),
            p.throttle.0,
            p.throttle.1,
            p.throttle.2,
            p.run.as_ref().map_or_else(
                || "no summary".into(),
                |r| format!(
                    "{}.{:02}",
                    r.summary.max / 1000,
                    (r.summary.max % 1000) / 10
                )
            ),
            p.run
                .as_ref()
                .map_or_else(|| "-".into(), |r| r.nivcsw.to_string()),
            seen(&p.hot_cpus),
            seen(&p.helper_cpus),
        );
    }
    out
}

/// The whole report over every case directory under `dir` (sorted), then
/// `dir/accounting.txt` verbatim.
///
/// # Errors
/// A case that does not read.
pub fn render(dir: &Path) -> Result<String, ParseError> {
    let mut cases: Vec<_> = fs::read_dir(dir)
        .map_err(|e| ParseError(format!("{}: {e}", dir.display())))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("meta.txt").is_file())
        .collect();
    cases.sort();
    let mut out = String::from("# Shared CPUs: how much a pod gets\n\n");
    for c in &cases {
        out.push_str(&render_case(&read_case(c)?));
        out.push('\n');
    }
    if let Ok(acc) = fs::read_to_string(dir.join("accounting.txt")) {
        let _ = writeln!(
            out,
            "## accounting: the scheduler counts ergon.io/shared-millicpus\n"
        );
        for l in acc.lines().filter(|l| !l.trim().is_empty()) {
            let _ = writeln!(out, "- {l}");
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) {
        let _ = fs::write(dir.join(name), text);
    }

    #[test]
    fn cpulists() {
        assert_eq!(cpus("1-2"), vec![1, 2]);
        assert_eq!(cpus("3,5-7"), vec![3, 5, 6, 7]);
        assert_eq!(cpus(""), Vec::<usize>::new());
    }

    #[test]
    fn busy_millicores_between_reads() {
        let b = "cpu1 100 0 0 900 0 0 0 0 0 0\ncpu2 0 0 0 1000 0 0 0 0 0 0\n";
        let a = "cpu1 1000 0 50 950 0 0 0 0 0 0\ncpu2 0 0 10 1990 0 0 0 0 0 0\n";
        assert_eq!(busy_milli(b, a), Ok(vec![(1, 950), (2, 10)]));
    }

    #[test]
    fn psr_samples_split_hot_and_helpers() {
        let t = "1 10 3 3 gp-hot\n1 11 1 1-2 gp-burn\n1 12 2 1-2 gp-burn\n2 10 3 3 gp-hot\n2 11 2 1-2 gp-burn\n";
        assert_eq!(psr(t), (vec![(3, 2)], vec![(1, 1), (2, 2)], 2));
    }

    #[test]
    fn a_case_reads_and_renders() {
        let d = std::env::temp_dir().join(format!("gapprobe-scale-{}", std::process::id()));
        let c = d.join("a-burst");
        let _ = fs::create_dir_all(&c);
        write(
            &c,
            "meta.txt",
            "case=a-burst\ntitle=one pod, 4 busy helpers\nshared=1-2\nexclusive=3-7\nwindow_ns=30000000000\nexpect=near 2 cores\n",
        );
        write(
            &c,
            "proc.stat.before",
            "cpu0 0 0 0 100 0 0 0 0 0 0\ncpu1 0 0 0 100 0 0 0 0 0 0\ncpu2 0 0 0 100 0 0 0 0 0 0\ncpu3 0 0 0 100 0 0 0 0 0 0\n",
        );
        write(
            &c,
            "proc.stat.after",
            "cpu0 1 0 0 199 0 0 0 0 0 0\ncpu1 2900 0 60 140 0 0 0 0 0 0\ncpu2 2950 0 20 130 0 0 0 0 0 0\ncpu3 3000 0 0 100 0 0 0 0 0 0\n",
        );
        write(
            &c,
            "scale-a.meta",
            "cpu=1\nmillicpus=100\nthreads=4\nduty=100\nshared_cpus=1\n",
        );
        write(
            &c,
            "scale-a.cpu.stat.before",
            "usage_usec 1000000\nnr_periods 0\nnr_throttled 0\nthrottled_usec 0\n",
        );
        write(
            &c,
            "scale-a.cpu.stat.after",
            "usage_usec 90000000\nnr_periods 0\nnr_throttled 0\nthrottled_usec 0\n",
        );
        write(&c, "scale-a.cpu.weight", "39\n");
        write(&c, "scale-a.pod.cpu.weight", "39\n");
        write(&c, "scale-a.cpu.max", "max 100000\n");
        write(&c, "scale-a.hot.schedstat.before", "1000000000 0 0\n");
        write(&c, "scale-a.hot.schedstat.after", "31000000000 0 0\n");
        write(
            &c,
            "scale-a.psr.tsv",
            "1 10 3 3 gp-hot\n1 11 1 1-2 gp-burn\n1 12 2 1-2 gp-burn\n",
        );
        write(
            &c,
            "scale-a.summary.tsv",
            &format!(
                "{}\nscale-a\ta-burst\t1\t3\t100\t24\t24\t32\t32\t32\t5120\t1\t3\t0\t0\t0\t1\t2\t0\t0\n",
                crate::report::RUN_HEADER
            ),
        );
        write(
            &d,
            "accounting.txt",
            "ok:   allocatable ergon.io/shared-millicpus 2000\n",
        );
        let md = render(&d).map_err(|e| e.0);
        let _ = fs::remove_dir_all(&d);
        let md = md.unwrap_or_else(|e| e);
        assert!(md.contains("## a-burst: one pod, 4 busy helpers"), "{md}");
        assert!(md.contains("| 1 | shared | 0.98 |"), "{md}");
        assert!(md.contains("| 2 | shared | 0.99 |"), "{md}");
        assert!(md.contains("| 3 | exclusive | 1.00 |"), "{md}");
        assert!(!md.contains("\n| 0 | "), "{md}");
        assert!(md.contains("Shared CPUs busy: 1.97 of 2 cores."), "{md}");
        // 89 s of usage less 30 s of spin over 30 s: 1.96 cores, 99 % of 1.97
        assert!(md.contains("| scale-a | 1 | 100 | 1 | 4 x 100 | 39 / 39 | max 100000 / ? | 1.96 | 99 % | 0 / 0 / 0 | 5.12 | 0 | 3 (1) | 1 (1), 2 (1) |"), "{md}");
        assert!(
            md.contains("- ok:   allocatable ergon.io/shared-millicpus 2000"),
            "{md}"
        );
    }

    #[test]
    fn a_missing_cgroup_file_is_an_error() {
        let d = std::env::temp_dir().join(format!("gapprobe-scale-bad-{}", std::process::id()));
        let _ = fs::create_dir_all(&d);
        write(&d, "x.meta", "cpu=1\n");
        let r = read_pod(&d, "x");
        let _ = fs::remove_dir_all(&d);
        assert!(r.is_err());
    }
}
