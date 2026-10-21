//! clockbench: every clock read on this host, compared: ergon-clock's
//! (unordered and ordered ticks, nanoseconds, the enum, the fallback, the
//! conversion alone), the bare instructions (`rdtsc`, `rdtscp`,
//! `lfence; rdtsc`; `mrs cntvct_el0`, `isb` first, `CNTVCTSS_EL0`), quanta
//! (held `Clock::raw`, `Clock::now`, `delta_as_nanos`; the global
//! `quanta::Instant::now`), minstant, std, and `clock_gettime` for each
//! kernel clock.
//!
//! ```text
//! clockbench [--cpu N] [--cpu2 M] [--label S] [--build S] [--quick]
//!            [--drift-secs S] [--spawns N] [--batches N] [--batch-reads N]
//!            [--delta-reads N] [--skew-rounds N] [--only SUBSTRING]
//!            [--csv FILE] [--enforce-gates]
//! clockbench --discipline SECS [--verifier-hz N] [--cpu N --cpu2 M] [--label S] [--csv FILE]
//! ```
//!
//! `--discipline` runs only the discipline of the counter to
//! `CLOCK_REALTIME` for `SECS` seconds, an update a second and a verifier
//! thread reading through a follower, both busy-waiting, the owner on
//! `--cpu` and the verifier on `--cpu2` (`ergon_clock_bench::discipline`:
//! its records and summaries).
//!
//! Each clock is first checked: its rate against `CLOCK_MONOTONIC_RAW`
//! within 1% (plus its resolution's share of the window), and an ordered or
//! kernel-monotonic read never lower than the one before. A clock that
//! fails is reported as failed and not timed, and clockbench exits 1. Then,
//! per clock: the cost of a read (the mean over `--batch-reads` back-to-back
//! reads, for `--batches` batches: median, p99, max and min of the batch
//! means); the gaps between consecutive reads over `--delta-reads` reads
//! (the smallest non-zero gap is the resolution; p50, p99, p99.99 and max;
//! reads equal to and lower than the one before; each gap of 10 us or more
//! with its `CLOCK_REALTIME`, to attribute it; the thread's context switches
//! and page faults over the pass, the buffer having been filled once
//! untimed); and, for the counters and a few others, backwards steps
//! across two threads on `--cpu` and `--cpu2` (which go together, and
//! differ) handing off `--skew-rounds` times each way. Then the conversions alone, the calibration of
//! ergon-clock and quanta in `--spawns` fresh processes (time taken, spread
//! of the rate chosen), the drift of each clock against
//! `CLOCK_MONOTONIC_RAW` and against the NTP-steered `CLOCK_MONOTONIC` at
//! checkpoints up to `--drift-secs`, and the gates
//! (ergon's unordered ns read and unordered ticks within 1 ns of the bare
//! read, its ordered ticks within 1 ns of the bare read of the instruction
//! its ordered read takes: `rdtscp`, `CNTVCTSS_EL0` or `isb` first, all on
//! a counter clock re-anchored 10,000 times as a disciplined one is; a
//! follower's read within 1.5 ns of the owner's; the fallback's read within
//! 1.5 ns of `clock_gettime`; enforced with `--enforce-gates` on Linux,
//! never on macOS, where performance is not a target).
//!
//! The CSV (stdout, or `--csv`), one record a line, its kind first: `meta`,
//! `check`, `cost`, `delta`, `gap`, `skew`, `convert`, `calib`, `drift`,
//! `gate`, `cmp` (fields in BENCHMARKS.md). A run starts with
//! `meta,tool,clockbench`. The readable table goes to stderr.
// Benchmark arithmetic: counts and nanoseconds far under 2^52 move between
// u64 and f64 throughout.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use ergon_clock::advanced::share::{ClockFollower, ClockPublisher, SharedClock};
use ergon_clock::advanced::{
    Clock, Kind, TimeSyncPolicy, Trust, calibrate, monotonic_raw_ns, realtime_ns,
};
use ergon_clock::{Config, Domain, Report};
use ergon_clock_bench::discipline;
use ergon_clock_bench::measure::{self, Cost, Deltas, Skew};
use ergon_clock_bench::rows::{self, Clocks, Spec, Visitor};
use ergon_clock_bench::sys;
use std::error::Error;
use std::fmt::Write as _;
use std::io::Write;
use std::time::{Duration, Instant};

/// The command line.
#[derive(Debug)]
struct Args {
    cpu: Option<usize>,
    cpu2: Option<usize>,
    label: String,
    build: String,
    batches: usize,
    batch_reads: usize,
    delta_reads: usize,
    skew_rounds: usize,
    drift_secs: u64,
    spawns: usize,
    only: Option<String>,
    csv: Option<String>,
    enforce: bool,
    child: bool,
    /// `--discipline`: seconds of the discipline alone.
    discipline: Option<u64>,
    verifier_hz: u64,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        cpu: None,
        cpu2: None,
        label: "-".to_owned(),
        build: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_owned(),
        batches: 200,
        batch_reads: 10_000,
        delta_reads: 1_000_000,
        skew_rounds: 100_000,
        drift_secs: 60,
        spawns: 20,
        only: None,
        csv: None,
        enforce: false,
        child: false,
        discipline: None,
        verifier_hz: 100,
    };
    let mut it = args.into_iter();
    let num = |v: Option<String>, what: &str| -> Result<u64, String> {
        v.ok_or_else(|| format!("{what}: a value"))?
            .parse::<u64>()
            .map_err(|e| format!("{what}: {e}"))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--cpu" => a.cpu = Some(num(it.next(), "--cpu")? as usize),
            "--cpu2" => a.cpu2 = Some(num(it.next(), "--cpu2")? as usize),
            "--label" => a.label = it.next().ok_or("--label: a value")?.replace(',', ";"),
            "--build" => a.build = it.next().ok_or("--build: a value")?.replace(',', ";"),
            "--batches" => a.batches = num(it.next(), "--batches")?.max(1) as usize,
            "--batch-reads" => a.batch_reads = num(it.next(), "--batch-reads")?.max(1) as usize,
            "--delta-reads" => a.delta_reads = num(it.next(), "--delta-reads")?.max(2) as usize,
            "--skew-rounds" => a.skew_rounds = num(it.next(), "--skew-rounds")? as usize,
            "--drift-secs" => a.drift_secs = num(it.next(), "--drift-secs")?,
            "--spawns" => a.spawns = num(it.next(), "--spawns")? as usize,
            "--only" => a.only = Some(it.next().ok_or("--only: a value")?),
            "--csv" => a.csv = Some(it.next().ok_or("--csv: a path")?),
            "--enforce-gates" => a.enforce = true,
            "--child-calibrate" => a.child = true,
            "--discipline" => a.discipline = Some(num(it.next(), "--discipline")?.max(1)),
            "--verifier-hz" => a.verifier_hz = num(it.next(), "--verifier-hz")?.clamp(1, 10_000),
            "--quick" => {
                a.batches = 50;
                a.delta_reads = 200_000;
                a.skew_rounds = 20_000;
                a.drift_secs = 0;
                a.spawns = 5;
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    // the cross-core threads each pin to one of the two; with --cpu alone
    // both would inherit the main thread's one CPU, and every handoff would
    // wait for the scheduler to switch between them. The discipline's owner
    // and verifier both busy-wait: on one CPU each would wait out the
    // other's time slices
    if a.skew_rounds > 0 || a.discipline.is_some() {
        match (a.cpu, a.cpu2) {
            (Some(_), None) => {
                return Err("--cpu needs --cpu2: two threads need a CPU each".into());
            }
            (None, Some(_)) => return Err("--cpu2 needs --cpu".into()),
            (Some(x), Some(y)) if x == y => return Err("--cpu and --cpu2 must differ".into()),
            _ => {}
        }
    }
    Ok(a)
}

/// The CSV, to stdout or a file.
struct Csv(Box<dyn Write>);

impl Csv {
    fn line(&mut self, fields: &[String]) {
        let _ = writeln!(self.0, "{}", fields.join(","));
    }
}

/// Nanoseconds, two decimals.
fn ns(v: f64) -> String {
    format!("{v:.2}")
}

/// What was learnt of one read row.
struct Row {
    spec: Spec,
    ok: bool,
    cost: Option<Cost>,
    deltas: Option<Deltas>,
    skew: Option<Skew>,
}

/// A clock kept for the drift measurement.
type Drifting<'c> = (Spec, Box<dyn Fn() -> u64 + Sync + 'c>, f64);

/// Runs every row: checks, cost, deltas, skew; keeps the drift rows.
struct Runner<'c, 'o> {
    args: &'o Args,
    csv: &'o mut Csv,
    /// The conversions' inputs: raw ticks captured once.
    inputs: &'c [u64],
    rows: Vec<Row>,
    converts: Vec<(Spec, Cost)>,
    drift: Vec<Drifting<'c>>,
    failed: usize,
}

impl Runner<'_, '_> {
    fn wanted(&self, name: &str) -> bool {
        self.args.only.as_deref().is_none_or(|o| name.contains(o))
    }
}

/// The step a clock advances by: the least non-zero difference between
/// consecutive reads over up to 50 ms, ns.
fn step_ns<F: Fn() -> u64>(read: &F, ns_per_unit: f64) -> f64 {
    let start = monotonic_raw_ns();
    let mut last = read();
    let mut smallest = f64::INFINITY;
    let mut changes = 0;
    while changes < 64 && monotonic_raw_ns() - start < 50_000_000 {
        let now = read();
        if now > last {
            smallest = smallest.min((now - last) as f64 * ns_per_unit);
            changes += 1;
        }
        last = now;
    }
    smallest
}

impl<'c> Visitor<'c> for Runner<'c, '_> {
    #[allow(clippy::too_many_lines)] // check, time, record: in order
    fn read<F: Fn() -> u64 + Sync + 'c>(&mut self, spec: Spec, read: F) {
        if !self.wanted(spec.name) {
            return;
        }
        let mut row = Row {
            spec,
            ok: true,
            cost: None,
            deltas: None,
            skew: None,
        };
        // the checks: a sane rate, and no step back where none may be
        let mut detail = String::new();
        let mut rate = String::from("-");
        let mut back = String::from("-");
        if let Some(scale) = spec.unit.ns_per_unit() {
            let step = step_ns(&read, scale);
            // a coarse clock is measured over enough of its steps
            let window = if step.is_finite() {
                (step * 100.0).clamp(20e6, 1e9)
            } else {
                20e6
            };
            let ppm = measure::rate_ppm(&read, scale, window as u64);
            let allowed = 10_000.0
                + if step.is_finite() {
                    2.0 * step / window * 1e6
                } else {
                    0.0
                };
            rate = format!("{ppm:.3}");
            if !ppm.is_finite() || ppm.abs() > allowed {
                row.ok = false;
                let _ = write!(
                    detail,
                    "rate {ppm:.1} ppm against CLOCK_MONOTONIC_RAW (allowed {allowed:.0});"
                );
            }
        }
        if spec.monotonic {
            let n = measure::backwards(&read, 200_000);
            back = n.to_string();
            if n > 0 {
                row.ok = false;
                let _ = write!(detail, "{n} reads lower than the one before;");
            }
        }
        self.csv.line(&[
            "check".into(),
            spec.name.into(),
            rate,
            back,
            if row.ok { "ok" } else { "fail" }.into(),
            detail.replace(',', ";"),
        ]);
        if !row.ok {
            eprintln!(
                "clockbench: {} FAILED its check: {detail} (not timed)",
                spec.name
            );
            self.failed += 1;
            self.rows.push(row);
            return;
        }
        let c = measure::cost(&read, self.args.batches, self.args.batch_reads);
        self.csv.line(&[
            "cost".into(),
            spec.name.into(),
            spec.unit.name().into(),
            c.batches.to_string(),
            c.reads.to_string(),
            ns(c.median_ns),
            ns(c.p99_ns),
            ns(c.max_ns),
            ns(c.min_ns),
        ]);
        row.cost = Some(c);
        if let Some(scale) = spec.unit.ns_per_unit() {
            let d = measure::deltas(&read, scale, self.args.delta_reads);
            self.csv.line(&[
                "delta".into(),
                spec.name.into(),
                d.reads.to_string(),
                ns(d.min_nonzero_ns),
                ns(d.p50_ns),
                ns(d.p99_ns),
                ns(d.p9999_ns),
                ns(d.max_ns),
                d.zeros.to_string(),
                d.backwards.to_string(),
                ns(d.max_backwards_ns),
                d.gaps.len().to_string(),
                d.switches.0.to_string(),
                d.switches.1.to_string(),
                d.faults.to_string(),
            ]);
            for (at, gap) in &d.gaps {
                self.csv
                    .line(&["gap".into(), spec.name.into(), at.to_string(), ns(*gap)]);
            }
            row.deltas = Some(d);
            if spec.skew && self.args.skew_rounds > 0 {
                let s = measure::skew(
                    &read,
                    scale,
                    self.args.skew_rounds,
                    self.args.cpu,
                    self.args.cpu2,
                );
                self.csv.line(&[
                    "skew".into(),
                    spec.name.into(),
                    s.rounds.to_string(),
                    ns(s.min_ab_ns),
                    ns(s.min_ba_ns),
                    ns(s.offset_ns),
                    s.inversions_ab.to_string(),
                    s.inversions_ba.to_string(),
                    ns(s.max_inversion_ns),
                ]);
                row.skew = Some(s);
            }
            if spec.drift {
                self.drift.push((spec, Box::new(read), scale));
            }
        }
        self.rows.push(row);
    }

    fn convert<F: Fn(u64) -> u64 + 'c>(&mut self, spec: Spec, convert: F) {
        if !self.wanted(spec.name) {
            return;
        }
        let c = measure::cost_convert(
            &convert,
            self.inputs,
            self.args.batches,
            self.args.batch_reads,
        );
        self.csv.line(&[
            "convert".into(),
            spec.name.into(),
            c.batches.to_string(),
            c.reads.to_string(),
            ns(c.median_ns),
            ns(c.p99_ns),
            ns(c.max_ns),
            ns(c.min_ns),
        ]);
        self.converts.push((spec, c));
    }
}

/// Child mode: one calibration of each, timed, in a fresh process. The
/// time-sync probe is off: the counter's calibration alone is timed, as
/// quanta's is (the parent's report carries the probe).
fn child() -> Result<(), Box<dyn Error>> {
    let t = Instant::now();
    let c = calibrate(&Config {
        trust: Trust::CrossCheck,
        domain: Domain::Monotonic,
        time_sync: TimeSyncPolicy::Off,
        ..Config::default()
    })?;
    let ms = t.elapsed().as_secs_f64() * 1e3;
    println!("{}", ergon_child_line(ms, &c.report));
    let t = Instant::now();
    let q = quanta::Clock::new();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    let ns = q.delta_as_nanos(0, 1 << 40);
    let hz = (1_u64 << 40) as f64 * 1e9 / ns.max(1) as f64;
    println!("child,quanta,{ms:.3},{hz:.3},-");
    Ok(())
}

/// ergon-clock's line from a child, `child,ergon,<ms>,<Hz>,<source>`: the
/// counter's rate and where it came from, or, for the monotonic fallback,
/// which has no counter rate, `-` and `fallback:<why>`.
fn ergon_child_line(ms: f64, report: &Report) -> String {
    match (report.kind, &report.fallback) {
        (Kind::Counter, _) => {
            let hz = 1e9 * (1_u64 << 32) as f64 / report.mult as f64;
            format!("child,ergon,{ms:.3},{hz:.3},{}", report.rate_source.name())
        }
        (Kind::Monotonic, why) => format!(
            "child,ergon,{ms:.3},-,fallback:{}",
            why.as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string)
                .replace(',', ";")
        ),
    }
}

/// One calibration in a fresh process, as the child reported it.
#[derive(Clone, Debug, PartialEq)]
struct ChildCalibration {
    /// `ergon` or `quanta`.
    name: String,
    /// The time it took, ms.
    ms: f64,
    /// The counter's rate it chose, Hz; none for ergon-clock's fallback.
    hz: Option<f64>,
    /// Where the rate came from, or `fallback:<why>`; `-` for quanta's.
    source: String,
}

/// A child's line as a calibration: `None` for a line that is not one.
///
/// # Errors
/// A child's line whose time or rate is not a number.
fn parse_child(line: &str) -> Result<Option<ChildCalibration>, String> {
    let f: Vec<&str> = line.split(',').collect();
    if f.len() < 5 || f[0] != "child" {
        return Ok(None);
    }
    let number = |v: &str| v.parse::<f64>().map_err(|e| format!("{line}: {e}"));
    Ok(Some(ChildCalibration {
        name: f[1].to_owned(),
        ms: number(f[2])?,
        hz: if f[3] == "-" {
            None
        } else {
            Some(number(f[3])?)
        },
        source: f[4].to_owned(),
    }))
}

/// One implementation's calibrations in fresh processes, summarised.
#[derive(Clone, Debug, PartialEq)]
struct Calibrations {
    name: String,
    /// The times, ms, sorted.
    ms: Vec<f64>,
    /// The counter rates chosen, Hz, sorted (the fallback's have none).
    hz: Vec<f64>,
    /// The rate sources and fallbacks seen, sorted, each once.
    sources: Vec<String>,
}

impl Calibrations {
    fn new(name: &str, all: &[ChildCalibration]) -> Self {
        let mine = || all.iter().filter(|c| c.name == name);
        let mut ms: Vec<f64> = mine().map(|c| c.ms).collect();
        let mut hz: Vec<f64> = mine().filter_map(|c| c.hz).collect();
        ms.sort_by(f64::total_cmp);
        hz.sort_by(f64::total_cmp);
        let mut sources: Vec<String> = mine()
            .map(|c| c.source.clone())
            .filter(|s| !s.is_empty() && s != "-")
            .collect();
        sources.sort_unstable();
        sources.dedup();
        Self {
            name: name.to_owned(),
            ms,
            hz,
            sources,
        }
    }

    /// [min, median, max, spread ppm] of the counter rates, where any.
    fn rates(&self) -> Option<[f64; 4]> {
        let (min, max) = (*self.hz.first()?, *self.hz.last()?);
        let med = measure::quantile(&self.hz, 0.5);
        Some([min, med, max, (max - min) / med * 1e6])
    }

    fn sources(&self) -> String {
        if self.sources.is_empty() {
            "-".to_owned()
        } else {
            self.sources.join(";")
        }
    }

    /// The `calib` record: name, processes, min, median and max ms, min,
    /// median and max rate (Hz), spread (ppm), sources; `-` for the rate
    /// and spread where no process chose the counter.
    fn record(&self) -> Vec<String> {
        let ms = |v: f64| format!("{v:.3}");
        let mut r = vec![
            "calib".to_owned(),
            self.name.clone(),
            self.ms.len().to_string(),
            ms(self.ms.first().copied().unwrap_or(f64::NAN)),
            ms(measure::quantile(&self.ms, 0.5)),
            ms(self.ms.last().copied().unwrap_or(f64::NAN)),
        ];
        match self.rates() {
            Some(rates) => r.extend(rates.map(ms)),
            None => r.extend(["-", "-", "-", "-"].map(str::to_owned)),
        }
        r.push(self.sources());
        r
    }

    /// The readable table's line.
    fn line(&self) -> String {
        let times = format!(
            "{:<10} {:>9.2} {:>9.2} {:>9.2} ms",
            self.name,
            self.ms.first().copied().unwrap_or(f64::NAN),
            measure::quantile(&self.ms, 0.5),
            self.ms.last().copied().unwrap_or(f64::NAN)
        );
        match self.rates() {
            Some([_, med, _, spread]) => {
                format!(
                    "{times}; {med:.3} Hz median, spread {spread:.3} ppm ({})",
                    self.sources()
                )
            }
            None => format!("{times}; no counter rate ({})", self.sources()),
        }
    }
}

/// Calibration in `spawns` fresh processes, per implementation.
fn calibrations(spawns: usize) -> Result<Vec<Calibrations>, Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    let mut all = Vec::new();
    for _ in 0..spawns {
        let out = std::process::Command::new(&exe)
            .arg("--child-calibrate")
            .output()?;
        if !out.status.success() {
            return Err(format!(
                "a calibration child failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )
            .into());
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            all.extend(parse_child(line)?);
        }
    }
    let mut names: Vec<&str> = Vec::new();
    for c in &all {
        if !names.contains(&c.name.as_str()) {
            names.push(&c.name);
        }
    }
    Ok(names.iter().map(|n| Calibrations::new(n, &all)).collect())
}

/// The CPU's model: Linux's /proc/cpuinfo (x86 `model name`; Arm's part
/// number), macOS's brand string.
fn cpu_model() -> String {
    if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
        let field = |key: &str| {
            info.lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_owned())
        };
        if let Some(model) = field("model name") {
            return model;
        }
        if let (Some(implementer), Some(part)) = (field("CPU implementer"), field("CPU part")) {
            let name = match part.as_str() {
                "0xd0c" => " (Neoverse N1, Graviton2)",
                "0xd40" => " (Neoverse V1, Graviton3)",
                "0xd4f" => " (Neoverse V2, Graviton4)",
                "0xd84" => " (Neoverse V3, Graviton5)",
                _ => "",
            };
            return format!("implementer {implementer} part {part}{name}");
        }
    }
    std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "-".to_owned())
}

/// A reference drift is measured against: (the CSV's key, the clock, its
/// name).
type DriftReference = (&'static str, fn() -> u64, &'static str);

/// `CLOCK_MONOTONIC`, NTP-steered on Linux and macOS alike (ergon-clock's
/// `monotonic_ns` is macOS's raw `CLOCK_UPTIME_RAW`).
fn ntp_monotonic_ns() -> u64 {
    sys::clock_ns(libc::CLOCK_MONOTONIC)
}

/// The references drift is measured against.
///
/// - `CLOCK_MONOTONIC_RAW`: the kernel's unadjusted scaling of its clock
///   source. Where that source is the counter, it is the counter itself, so
///   a counter clock's drift from it is how its conversion differs from
///   the kernel's (what ergon-clock calibrates against), not its accuracy.
/// - `CLOCK_MONOTONIC`: steered by whichever NTP client the host runs
///   (systemd-timesyncd on the lab's Debian images, chrony where
///   installed) to its time sources' rate, and slewed while it removes an
///   offset: a counter clock's drift from it is the oscillator's frequency
///   error as the client sees it plus that slew, which after a boot is most
///   of it (systemd-timesyncd's PLL slews by hundreds of ppm).
fn drift_references() -> [DriftReference; 2] {
    [
        ("raw", monotonic_raw_ns, "CLOCK_MONOTONIC_RAW"),
        ("monotonic", ntp_monotonic_ns, "CLOCK_MONOTONIC"),
    ]
}

/// A CPU option as its `meta` value: the number, or `-`.
fn cpu_meta(cpu: Option<usize>) -> String {
    cpu.map_or_else(|| "-".to_owned(), |c| c.to_string())
}

fn meta(csv: &mut Csv, key: &str, value: &str) {
    csv.line(&["meta".into(), key.into(), value.replace(',', ";")]);
}

fn report_meta(csv: &mut Csv, prefix: &str, report: &Report) {
    for (key, value) in report.fields() {
        meta(csv, &format!("{prefix}_{key}"), &value);
    }
}

/// The second thread's CPU (`--cpu2`), checked from a thread of its own
/// before this one pins: the thread pinned there later cannot report a
/// refusal mid-run (a handoff, or a discipline's verifier).
fn check_cpu2(args: &Args) -> Result<(), Box<dyn Error>> {
    if let Some(cpu2) = args.cpu2 {
        std::thread::spawn(move || sys::pin(cpu2))
            .join()
            .map_err(|_| "the pinning check's thread failed")?
            .map_err(|e| format!("cannot pin to CPU {cpu2} (--cpu2): {e}"))?;
    }
    Ok(())
}

/// A thread's context switches as `<voluntary>;<involuntary>`.
fn switches((voluntary, involuntary): (u64, u64)) -> String {
    format!("{voluntary};{involuntary}")
}

/// `--discipline`: the discipline alone, its records and a summary.
fn discipline_only(args: &Args, secs: u64) -> Result<(), Box<dyn Error>> {
    check_cpu2(args)?;
    let pinned = match args.cpu {
        Some(cpu) => sys::pin(cpu).map_err(|e| format!("cannot pin to CPU {cpu}: {e}"))?,
        None => false,
    };
    let mut csv = Csv(match &args.csv {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout()),
    });
    meta(&mut csv, "tool", "clockbench");
    meta(&mut csv, "mode", "discipline");
    meta(&mut csv, "version", env!("CARGO_PKG_VERSION"));
    meta(&mut csv, "label", &args.label);
    meta(&mut csv, "build", &args.build);
    meta(&mut csv, "arch", std::env::consts::ARCH);
    meta(&mut csv, "os", std::env::consts::OS);
    meta(&mut csv, "kernel", &sys::kernel());
    meta(&mut csv, "cpu_model", &cpu_model());
    meta(&mut csv, "cpu", &cpu_meta(args.cpu));
    meta(&mut csv, "cpu2", &cpu_meta(args.cpu2));
    meta(&mut csv, "pinned", if pinned { "yes" } else { "no" });
    meta(&mut csv, "start_realtime_ns", &realtime_ns().to_string());
    meta(
        &mut csv,
        "settings",
        &format!("discipline_secs={secs} verifier_hz={}", args.verifier_hz),
    );
    let run = discipline::run(secs, args.verifier_hz, args.cpu2)?;
    meta(
        &mut csv,
        "owner_context_switches",
        &switches(run.owner_switches),
    );
    meta(
        &mut csv,
        "verifier_context_switches",
        &switches(run.verifier_switches),
    );
    let cpus: Vec<String> = run.verifier_cpus.iter().map(ToString::to_string).collect();
    let cpus = if cpus.is_empty() {
        "-".to_owned()
    } else {
        cpus.join(";")
    };
    meta(&mut csv, "verifier_cpus", &cpus);
    for u in &run.updates {
        csv.line(&discipline::discipline_record(u));
    }
    for t in &run.truths {
        csv.line(&discipline::truth_record(t));
    }
    let summaries = run.summaries();
    for (series, spread) in &summaries {
        csv.line(&spread.record(series));
    }
    let _ = csv.0.flush();
    let mut t = format!(
        "\nclockbench --discipline {secs} {} ({}, {} {}): {} updates, {} verifier reads\n",
        args.label,
        args.build,
        std::env::consts::ARCH,
        std::env::consts::OS,
        run.updates.len(),
        run.truths.len()
    );
    for (series, s) in &summaries {
        let _ = writeln!(
            t,
            "{series:<24} n {:>7}  p50 {:>9}  p99 {:>9}  p99.9 {:>9}  max {:>9}",
            s.n, s.p50, s.p99, s.p999, s.max
        );
    }
    let _ = writeln!(
        t,
        "context switches, voluntary;involuntary: owner {}, verifier {} (on CPUs {cpus})",
        switches(run.owner_switches),
        switches(run.verifier_switches)
    );
    eprint!("{t}");
    Ok(())
}

#[allow(clippy::too_many_lines)] // one run, in order
fn main() -> Result<(), Box<dyn Error>> {
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            eprintln!(
                "usage: clockbench [--cpu N] [--cpu2 M] [--label S] [--build S] [--quick] [--drift-secs S] \
                 [--spawns N] [--batches N] [--batch-reads N] [--delta-reads N] [--skew-rounds N] [--only S] \
                 [--csv FILE] [--enforce-gates]\n       clockbench --discipline SECS [--verifier-hz N] [--cpu N --cpu2 M] [--label S] [--csv FILE]"
            );
            std::process::exit(2);
        }
    };
    if args.child {
        return child();
    }
    if let Some(secs) = args.discipline {
        return discipline_only(&args, secs);
    }
    check_cpu2(&args)?;
    let pinned = match args.cpu {
        Some(cpu) => sys::pin(cpu).map_err(|e| format!("cannot pin to CPU {cpu}: {e}"))?,
        None => false,
    };
    let mut csv = Csv(match &args.csv {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout()),
    });
    let os = if cfg!(target_os = "macos") {
        "macos"
    } else {
        std::env::consts::OS
    };
    meta(&mut csv, "tool", "clockbench");
    meta(&mut csv, "version", env!("CARGO_PKG_VERSION"));
    meta(&mut csv, "label", &args.label);
    meta(&mut csv, "build", &args.build);
    meta(&mut csv, "arch", std::env::consts::ARCH);
    meta(&mut csv, "os", os);
    meta(&mut csv, "kernel", &sys::kernel());
    meta(&mut csv, "cpu_model", &cpu_model());
    meta(&mut csv, "cpu", &cpu_meta(args.cpu));
    meta(&mut csv, "cpu2", &cpu_meta(args.cpu2));
    meta(&mut csv, "pinned", if pinned { "yes" } else { "no" });
    meta(&mut csv, "start_realtime_ns", &realtime_ns().to_string());
    meta(
        &mut csv,
        "settings",
        &format!(
            "batches={} batch_reads={} delta_reads={} skew_rounds={} drift_secs={} spawns={}",
            args.batches,
            args.batch_reads,
            args.delta_reads,
            args.skew_rounds,
            args.drift_secs,
            args.spawns
        ),
    );
    meta(
        &mut csv,
        "minstant_tsc",
        if minstant::is_tsc_available() {
            "yes"
        } else {
            "no"
        },
    );

    let clocks = Clocks::new()?;
    meta(
        &mut csv,
        "quanta_ns_per_tick",
        &format!("{:.6}", clocks.quanta_ns_per_tick),
    );
    report_meta(&mut csv, "ergon", &clocks.ergon_report);
    report_meta(&mut csv, "default", &clocks.default_report);

    let mut runner = Runner {
        args: &args,
        csv: &mut csv,
        inputs: &clocks.samples,
        rows: Vec::new(),
        converts: Vec::new(),
        drift: Vec::new(),
        failed: 0,
    };
    rows::visit(&clocks, &mut runner);
    let Runner {
        mut rows,
        converts,
        drift,
        failed,
        ..
    } = runner;
    // a follower of the counter clock, timed apart: per thread, `&mut`
    if let Some(e) = clocks.ergon
        && args
            .only
            .as_deref()
            .is_none_or(|o| rows::FOLLOWER.name.contains(o))
    {
        let shared = SharedClock::new();
        let owner = Clock::Counter(e);
        ClockPublisher::new(&shared)
            .ok_or("the shared clock is claimed")?
            .publish(&owner);
        let mut follower = ClockFollower::new(&shared, owner);
        let c = measure::cost(|| follower.clock().now_ns(), args.batches, args.batch_reads);
        csv.line(&[
            "cost".into(),
            rows::FOLLOWER.name.into(),
            rows::FOLLOWER.unit.name().into(),
            c.batches.to_string(),
            c.reads.to_string(),
            ns(c.median_ns),
            ns(c.p99_ns),
            ns(c.max_ns),
            ns(c.min_ns),
        ]);
        rows.push(Row {
            spec: rows::FOLLOWER,
            ok: true,
            cost: Some(c),
            deltas: None,
            skew: None,
        });
    }

    // the front API's follower, of a counter clock calibrated as the
    // ergon rows' (cross-checked), timed apart as the follower above
    if clocks.ergon.is_some()
        && args
            .only
            .as_deref()
            .is_none_or(|o| rows::FRONT_FOLLOWER.name.contains(o))
    {
        let mut owner = ergon_clock::calibrate(&Config {
            trust: Trust::CrossCheck,
            print_warnings: false,
            ..Config::default()
        })?;
        if owner.report().fallback.is_none() {
            let follower = owner.follower();
            let c = measure::cost(|| follower.now_ns(), args.batches, args.batch_reads);
            csv.line(&[
                "cost".into(),
                rows::FRONT_FOLLOWER.name.into(),
                rows::FRONT_FOLLOWER.unit.name().into(),
                c.batches.to_string(),
                c.reads.to_string(),
                ns(c.median_ns),
                ns(c.p99_ns),
                ns(c.max_ns),
                ns(c.min_ns),
            ]);
            rows.push(Row {
                spec: rows::FRONT_FOLLOWER,
                ok: true,
                cost: Some(c),
                deltas: None,
                skew: None,
            });
        }
    }

    // calibration in fresh processes
    let calib = if args.spawns > 0 {
        calibrations(args.spawns)?
    } else {
        Vec::new()
    };
    for c in &calib {
        csv.line(&c.record());
    }

    // drift against each reference
    let references = drift_references();
    let mut drift_rows = Vec::new();
    if args.drift_secs > 0 && !drift.is_empty() {
        let starts: Vec<Vec<(u64, u64)>> = drift
            .iter()
            .map(|(_, read, _)| {
                references
                    .iter()
                    .map(|(_, reference, _)| measure::against(reference, read.as_ref()))
                    .collect()
            })
            .collect();
        let begun = Instant::now();
        let mut points: Vec<u64> = [1, 2, 5, 10, 20, 30, 60, 120, 300, 600]
            .into_iter()
            .filter(|&s| s < args.drift_secs)
            .collect();
        points.push(args.drift_secs);
        for secs in points {
            while begun.elapsed() < Duration::from_secs(secs) {
                std::thread::sleep(Duration::from_millis(20));
            }
            for ((spec, read, scale), starts) in drift.iter().zip(&starts) {
                for ((key, reference, _), start) in references.iter().zip(starts) {
                    let end = measure::against(reference, read.as_ref());
                    let (error, ppm) = measure::drift(*start, end, *scale);
                    csv.line(&[
                        "drift".into(),
                        spec.name.into(),
                        secs.to_string(),
                        end.0.wrapping_sub(start.0).to_string(),
                        format!("{error:.0}"),
                        format!("{ppm:.3}"),
                        (*key).into(),
                    ]);
                    drift_rows.push((spec.name, *key, secs, error, ppm));
                }
            }
        }
    }

    // the gates, and ergon against quanta
    let median = |name: &str| {
        rows.iter()
            .find(|r| r.spec.name == name)
            .and_then(|r| r.cost.map(|c| c.median_ns))
    };
    let bare = if cfg!(target_arch = "aarch64") {
        "raw_cntvct"
    } else {
        "raw_rdtsc"
    };
    // the bare row of the instruction ergon-clock's ordered read takes
    // (`raw_cntvctss` where it chose FEAT_ECV's), so the gate compares like
    // with like
    let ordered = clocks
        .ergon
        .and_then(|e| rows::bare_ordered_row(e.ordered_read()))
        .unwrap_or("-");
    let enforced = cfg!(target_os = "linux");
    let mut gate_failed = false;
    let mut gates = Vec::new();
    // A bracket takes ordered ticks and converts their difference after: an
    // ordered read in nanoseconds back to back also waits, at the next
    // read's barrier, for the last one's multiply (its latency), which a
    // bracket never pays (cmp rows below).
    for (gate, ours, theirs, limit) in [
        ("unordered_ns_over_bare_read", "ergon_now_ns", bare, 1.0),
        ("unordered_ticks_over_bare_read", "ergon_ticks", bare, 1.0),
        (
            "ordered_ticks_over_bare_ordered_read",
            "ergon_ticks_ordered",
            ordered,
            1.0,
        ),
        (
            "follower_ns_over_owner_ns",
            rows::FOLLOWER.name,
            "ergon_now_ns",
            1.5,
        ),
        (
            "front_follower_ns_over_owner_ns",
            rows::FRONT_FOLLOWER.name,
            "ergon_now_ns",
            1.5,
        ),
        (
            "fallback_ns_over_clock_gettime",
            "ergon_monotonic_now_ns",
            rows::FALLBACK_KERNEL_ROW,
            1.5,
        ),
    ] {
        if let (Some(a), Some(b)) = (median(ours), median(theirs)) {
            let over = a - b;
            let verdict = if !enforced {
                "not-enforced"
            } else if over <= limit {
                "pass"
            } else {
                gate_failed = true;
                "fail"
            };
            csv.line(&[
                "gate".into(),
                gate.into(),
                ns(over),
                ns(limit),
                verdict.into(),
                ours.into(),
                theirs.into(),
            ]);
            gates.push((gate, over, limit, verdict, theirs));
        }
    }
    let mut cmps = Vec::new();
    for (name, ours, theirs) in [
        (
            "ergon_now_ns_ordered_minus_bare_ordered_read",
            "ergon_now_ns_ordered",
            ordered,
        ),
        // where the ordered read is CNTVCTSS_EL0, what it saves over isb
        (
            "ergon_ticks_ordered_minus_raw_isb_cntvct",
            "ergon_ticks_ordered",
            "raw_isb_cntvct",
        ),
        (
            "ergon_now_ns_minus_quanta_now_ns",
            "ergon_now_ns",
            "quanta_now_ns",
        ),
        ("ergon_ticks_minus_quanta_raw", "ergon_ticks", "quanta_raw"),
        (
            "ergon_now_ns_minus_minstant_unix_ns",
            "ergon_now_ns",
            "minstant_unix_ns",
        ),
        (
            "ergon_now_ns_minus_std_instant_now",
            "ergon_now_ns",
            "std_instant_now",
        ),
        (
            "ergon_clock_now_ns_minus_ergon_now_ns_loaded",
            "ergon_clock_now_ns",
            "ergon_now_ns_loaded",
        ),
    ] {
        if let (Some(a), Some(b)) = (median(ours), median(theirs)) {
            csv.line(&["cmp".into(), name.into(), ns(a - b)]);
            cmps.push((name, a - b));
        }
    }
    let _ = csv.0.flush();

    // the readable table
    let mut t = String::new();
    let _ = writeln!(
        t,
        "\nclockbench {} ({}, {} {}, {}){}",
        args.label,
        args.build,
        std::env::consts::ARCH,
        os,
        cpu_model(),
        if pinned {
            format!(", pinned to CPU {}", args.cpu.unwrap_or(0))
        } else {
            String::new()
        }
    );
    let _ = writeln!(
        t,
        "ergon-clock (Trust::CrossCheck): {}",
        clocks.ergon_report
    );
    let _ = writeln!(
        t,
        "ergon-clock's ordered read (ergon_ticks_ordered, ergon_now_ns_ordered): {}",
        clocks.ergon_report.ordered_read
    );
    let d = &clocks.default_report;
    let _ = writeln!(
        t,
        "ergon-clock default (Trust::Kernel): kind={} fallback={}",
        d.kind.name(),
        d.fallback
            .as_ref()
            .map_or_else(|| "none".to_owned(), ToString::to_string)
    );
    for finding in d.findings() {
        let _ = writeln!(t, "{} {finding}", finding.severity.name());
    }
    let _ = writeln!(
        t,
        "\n{:<32} {:>9} {:>8} {:>9} | {:>8} {:>8} {:>9} {:>11} {:>5} {:>5} | {:>6} {:>8}",
        "read (ns, batch mean)",
        "median",
        "p99",
        "max batch",
        "resol",
        "p50",
        "p99.99",
        "max gap",
        "back",
        "csw",
        "x-inv",
        "x-offset"
    );
    for r in &rows {
        let Some(c) = r.cost else {
            let _ = writeln!(t, "{:<32} FAILED its check, not timed", r.spec.name);
            continue;
        };
        let _ = write!(
            t,
            "{:<32} {:>9.2} {:>8.2} {:>9.2} |",
            r.spec.name, c.median_ns, c.p99_ns, c.max_ns
        );
        match &r.deltas {
            Some(d) => {
                let _ = write!(
                    t,
                    " {:>8.1} {:>8.1} {:>9.1} {:>11.1} {:>5} {:>5} |",
                    d.min_nonzero_ns,
                    d.p50_ns,
                    d.p9999_ns,
                    d.max_ns,
                    d.backwards,
                    d.switches.0 + d.switches.1
                );
            }
            None => {
                let _ = write!(
                    t,
                    " {:>8} {:>8} {:>9} {:>11} {:>5} {:>5} |",
                    "-", "-", "-", "-", "-", "-"
                );
            }
        }
        match r.skew {
            Some(s) => {
                let _ = writeln!(
                    t,
                    " {:>6} {:>8.1}",
                    s.inversions_ab + s.inversions_ba,
                    s.offset_ns
                );
            }
            None => {
                let _ = writeln!(t, " {:>6} {:>8}", "-", "-");
            }
        }
    }
    let _ = writeln!(
        t,
        "\n{:<32} {:>9} {:>8} {:>9}",
        "conversion (ns, batch mean)", "median", "p99", "max batch"
    );
    for (spec, c) in &converts {
        let _ = writeln!(
            t,
            "{:<32} {:>9.2} {:>8.2} {:>9.2}",
            spec.name, c.median_ns, c.p99_ns, c.max_ns
        );
    }
    if !calib.is_empty() {
        let _ = writeln!(
            t,
            "\ncalibration in {} fresh processes: min / median / max ms; rate spread ppm",
            args.spawns
        );
        for c in &calib {
            let _ = writeln!(t, "{}", c.line());
        }
    }
    if !drift_rows.is_empty() {
        let _ = write!(t, "\ndrift at {} s (error ns, ppm)", args.drift_secs);
        for (_, _, name) in &references {
            let _ = write!(t, " | against {name}");
        }
        let _ = writeln!(t);
        for name in drift.iter().map(|(spec, _, _)| spec.name) {
            let _ = write!(t, "{name:<32}");
            for (key, _, _) in &references {
                if let Some((_, _, _, error, ppm)) = drift_rows
                    .iter()
                    .find(|r| r.0 == name && r.1 == *key && r.2 == args.drift_secs)
                {
                    let _ = write!(t, " | {error:>12.0} {ppm:>10.3}");
                }
            }
            let _ = writeln!(t);
        }
    }
    let _ = writeln!(t);
    for (gate, over, limit, verdict, theirs) in &gates {
        let _ = writeln!(
            t,
            "gate {gate} (against {theirs}): {over:+.2} ns (limit {limit:+.2}): {verdict}"
        );
    }
    for (name, diff) in &cmps {
        let _ = writeln!(t, "cmp {name}: {diff:+.2} ns");
    }
    eprint!("{t}");
    let _ = std::io::stderr().flush();

    if failed > 0 {
        eprintln!("clockbench: {failed} clock(s) failed their checks");
        std::process::exit(1);
    }
    if args.enforce && gate_failed {
        eprintln!("clockbench: a gate failed (--enforce-gates)");
        std::process::exit(3);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Calibrations, ChildCalibration, ergon_child_line, parse, parse_child};
    use ergon_clock::Config;
    use ergon_clock::advanced::{Policy, Trust, calibrate};

    fn args(list: &[&str]) -> Result<super::Args, String> {
        parse(list.iter().map(|s| (*s).to_owned()))
    }

    /// The cross-core measurement needs two CPUs: with `--cpu` alone its
    /// threads inherit the main thread's one-CPU affinity and every handoff
    /// waits for the scheduler to switch between them (milliseconds each,
    /// hours for a run), so `--cpu` and `--cpu2` come together, and differ.
    #[test]
    fn the_cross_core_cpus_come_together_and_differ() {
        assert!(args(&["--cpu", "2"]).is_err_and(|e| e.contains("--cpu2")));
        assert!(args(&["--cpu2", "3"]).is_err_and(|e| e.contains("--cpu")));
        assert!(args(&["--cpu", "2", "--cpu2", "2"]).is_err_and(|e| e.contains("differ")));
        assert!(
            args(&["--cpu", "2", "--cpu2", "3"])
                .is_ok_and(|a| (a.cpu, a.cpu2) == (Some(2), Some(3)))
        );
        // no cross-core measurement: one CPU will do
        assert!(args(&["--cpu", "2", "--skew-rounds", "0"]).is_ok());
        assert!(args(&[]).is_ok_and(|a| (a.cpu, a.cpu2) == (None, None)));
    }

    /// `--discipline` runs the discipline alone (its seconds at least one,
    /// the verifier at 100 Hz unless told). Both its threads busy-wait, so
    /// pinned they need a CPU each: `--cpu` the owner's, `--cpu2` the
    /// verifier's, together and different.
    #[test]
    fn the_discipline_runs_alone_on_two_cpus() {
        assert!(
            args(&["--discipline", "30", "--cpu", "2", "--cpu2", "3"]).is_ok_and(|a| (
                a.discipline,
                a.verifier_hz,
                a.cpu,
                a.cpu2
            ) == (
                Some(30),
                100,
                Some(2),
                Some(3)
            ))
        );
        assert!(args(&["--discipline", "30", "--cpu", "2"]).is_err_and(|e| e.contains("--cpu2")));
        assert!(args(&["--discipline", "30", "--cpu2", "3"]).is_err_and(|e| e.contains("--cpu")));
        assert!(
            args(&["--discipline", "30", "--cpu", "2", "--cpu2", "2"])
                .is_err_and(|e| e.contains("differ"))
        );
        assert!(
            args(&["--discipline", "0", "--verifier-hz", "0"])
                .is_ok_and(|a| (a.discipline, a.verifier_hz) == (Some(1), 1))
        );
        assert!(args(&["--discipline"]).is_err());
        assert!(args(&[]).is_ok_and(|a| a.discipline.is_none()));
    }

    fn children(lines: &[&str]) -> Result<Vec<ChildCalibration>, String> {
        let mut all = Vec::new();
        for line in lines {
            all.extend(parse_child(line)?);
        }
        Ok(all)
    }

    /// A child's line says what ergon-clock chose: the counter's rate and
    /// its source, or, for the monotonic fallback, no rate and why.
    #[test]
    fn a_childs_line_names_the_fallback_or_the_rate() -> Result<(), Box<dyn std::error::Error>> {
        let fallback = calibrate(&Config {
            policy: Policy::Monotonic,
            ..Config::default()
        })?;
        assert_eq!(
            ergon_child_line(40.5, &fallback.report),
            "child,ergon,40.500,-,fallback:forced"
        );
        let chosen = calibrate(&Config {
            trust: Trust::CrossCheck,
            ..Config::default()
        })?;
        let line = ergon_child_line(40.5, &chosen.report);
        let parsed = parse_child(&line)?.ok_or("not a child's line")?;
        if chosen.clock.is_counter() {
            let hz = parsed.hz.ok_or("a counter with no rate")?;
            assert!((hz - chosen.clock.hz() as f64).abs() < 1.0, "{line}");
            assert_eq!(parsed.source, chosen.report.rate_source.name());
        } else {
            assert_eq!(parsed.hz, None, "{line}");
            assert!(parsed.source.starts_with("fallback:"), "{line}");
        }
        Ok(())
    }

    /// Calibrations that fell back to the monotonic clock have no counter
    /// rate: their record says why, with no rate and no spread, never a
    /// 1 GHz "rate" agreeing with itself to 0 ppm. Beside counter
    /// calibrations, the rate and spread are the counter's alone and both
    /// are named.
    #[test]
    fn a_fallback_calibration_reports_why_not_a_rate() -> Result<(), String> {
        let fallback = children(&[
            "child,ergon,40.100,-,fallback:kernel-clocksource:hpet",
            "child,ergon,40.300,-,fallback:kernel-clocksource:hpet",
        ])?;
        let c = Calibrations::new("ergon", &fallback);
        assert_eq!(
            c.record(),
            [
                "calib",
                "ergon",
                "2",
                "40.100",
                "40.300",
                "40.300",
                "-",
                "-",
                "-",
                "-",
                "fallback:kernel-clocksource:hpet"
            ]
        );
        assert!(
            c.line()
                .ends_with("ms; no counter rate (fallback:kernel-clocksource:hpet)"),
            "{}",
            c.line()
        );
        let mixed = children(&[
            "child,ergon,40.100,1000000000.000,register",
            "child,ergon,40.200,1000001000.000,register",
            "child,ergon,40.300,-,fallback:unstable:12ppm",
            "child,quanta,0.590,999990000.000,-",
        ])?;
        let r = Calibrations::new("ergon", &mixed).record();
        assert_eq!(r[2], "3");
        assert_eq!(
            r[6..],
            [
                "1000000000.000",
                "1000001000.000",
                "1000001000.000",
                "1.000",
                "fallback:unstable:12ppm;register"
            ]
        );
        let q = Calibrations::new("quanta", &mixed).record();
        assert_eq!(q[2..4], ["1", "0.590"]);
        assert_eq!(q[9..], ["0.000", "-"]);
        // a line that is not a child's is none; a malformed one, an error
        assert_eq!(parse_child("clockbench: noise"), Ok(None));
        assert!(parse_child("child,ergon,soon,1,register").is_err());
        Ok(())
    }
}
