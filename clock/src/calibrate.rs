//! Calibration: an explicit start-up call that measures the counter and
//! returns a `Copy` clock and a report of what it chose and why.

use crate::arch;
use crate::clock::{Clock, CounterClock, Domain, MonotonicClock};
use crate::detect::{self, Arch, Fallback, HostFacts, OrderedRead, RateSource, Trust};
use crate::error::Error;
use crate::scale::Scale;
use crate::timesync::{self, Finding, SyncVerdict, TimeSync, TimeSyncPolicy};
use crate::wall;
use std::fmt;
use std::hint::black_box;
use std::time::Duration;

/// Whether the counter is wanted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Policy {
    /// The counter where it is usable, else the monotonic fallback, with
    /// the reason in the report. The default.
    #[default]
    Auto,
    /// The counter or an error: for a process that must not run on the
    /// slower clock unnoticed.
    RequireCounter,
    /// The monotonic fallback (to compare the two).
    Monotonic,
}

/// How to calibrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Whether the counter is wanted.
    pub policy: Policy,
    /// How much evidence the counter needs ([`HostFacts::gate`]).
    pub trust: Trust,
    /// What the nanoseconds count from: UTC ([`Domain::Realtime`], the
    /// default) or boot ([`Domain::Monotonic`]).
    pub domain: Domain,
    /// The measurement of the counter's rate against
    /// `CLOCK_MONOTONIC_RAW` where no frequency register gives it (x86):
    /// 50 ms, a rate within about 1 ppm of the raw clock's.
    pub window: Duration,
    /// The check of a frequency register (Arm), and the verification of the
    /// rate chosen: 20 ms each.
    pub check_window: Duration,
    /// A counter read costing more than this, ns, is trapped and emulated
    /// (an erratum workaround, a hypervisor): refused. 200 ns.
    pub max_read_ns: u64,
    /// Whether the report also judges how `CLOCK_REALTIME` is kept
    /// ([`TimeSync`]), and whether calibration fails unless it is
    /// disciplined to a PTP hardware clock: probed and warned about by
    /// default, never a failure unless asked for.
    pub time_sync: TimeSyncPolicy,
    /// A gap to UTC over this, ns, is a warning: at start-up, the host's
    /// own clock still being corrected by more, or known to UTC only within
    /// more ([`Report::warnings`]); after start, the clock that far from
    /// `CLOCK_REALTIME` ([`Health::warning`](crate::Health::warning)).
    /// 1 ms.
    pub gap_warn_ns: u64,
    /// Whether the front [`calibrate`](crate::calibrate) prints its
    /// start-up warnings to stderr, one line each (`true`; the environment
    /// variable `ERGON_HOST_CHECKS=off` or `0` also turns them off).
    /// [`advanced::calibrate`](crate::advanced::calibrate) never prints. The
    /// report keeps every warning either way.
    pub print_warnings: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            policy: Policy::Auto,
            trust: Trust::Kernel,
            domain: Domain::Realtime,
            window: Duration::from_millis(50),
            check_window: Duration::from_millis(20),
            max_read_ns: 200,
            time_sync: TimeSyncPolicy::Warn,
            gap_warn_ns: 1_000_000,
            print_warnings: true,
        }
    }
}

/// Which clock calibration chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The hardware counter.
    Counter,
    /// The monotonic fallback.
    Monotonic,
}

impl Kind {
    /// The name the report prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Monotonic => "monotonic",
        }
    }
}

/// A calibration's result: the clock, and what was chosen and why.
#[derive(Clone, Debug)]
pub struct Calibrated {
    /// The clock: `Copy` it to every thread that reads the time.
    pub clock: Clock,
    /// What was chosen and why: log it, and put it in every benchmark's
    /// metadata.
    pub report: Report,
}

/// What calibration chose and why, and what it measured.
///
/// `#[non_exhaustive]`: a field may be added without a breaking change. Build it by calibrating, and read the fields.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Report {
    /// The clock chosen.
    pub kind: Kind,
    /// Why not the counter, when the fallback was chosen.
    pub fallback: Option<Fallback>,
    /// The counter's unordered read: `rdtsc`, `cntvct_el0`, or `none`.
    pub counter: &'static str,
    /// The counter's ordered read ([`OrderedRead::name`]): `rdtscp`;
    /// `mrs cntvctss_el0` where the CPU has `FEAT_ECV`, else
    /// `isb; mrs cntvct_el0`; or `none`.
    pub ordered_read: &'static str,
    /// The clock calibration measures against.
    pub reference: &'static str,
    /// The fallback's clock.
    pub monotonic: &'static str,
    /// What the nanoseconds count from.
    pub domain: Domain,
    /// Ticks a second (10^9 for the fallback, whose ticks are ns).
    pub hz: u64,
    /// ns per tick, 32.32.
    pub mult: u64,
    /// Where the rate came from.
    pub rate_source: RateSource,
    /// The counter's rate measured against the reference, Hz.
    pub measured_hz: Option<f64>,
    /// The measurement's error bound, ppm (both ends' brackets and both
    /// clocks' resolutions over the window).
    pub uncertainty_ppm: Option<f64>,
    /// The rate chosen against the rate measured, ppm.
    pub disagreement_ppm: Option<f64>,
    /// The verification's measurement against the rate chosen, ppm.
    pub verify_ppm: Option<f64>,
    /// The unordered read's cost, ns (the mean of 1,024, the least of 5).
    pub read_ns: Option<f64>,
    /// The ordered read's cost, ns.
    pub ordered_read_ns: Option<f64>,
    /// How long calibration took.
    pub calibration: Duration,
    /// What the host said.
    pub facts: HostFacts,
    /// The trust calibration was given.
    pub trust: Trust,
    /// How `CLOCK_REALTIME` is kept (`None`: not probed,
    /// [`TimeSyncPolicy::Off`]): what wall-clock stamps, and a
    /// [`WallOffset`](crate::advanced::WallOffset), are worth across hosts. The
    /// counter's own rate does not depend on it.
    pub time_sync: Option<TimeSync>,
    /// The gap to UTC, ns, over which [`warnings`](Self::warnings) warns
    /// ([`Config::gap_warn_ns`]).
    pub gap_warn_ns: u64,
}

impl Report {
    pub(crate) const fn new(facts: HostFacts, config: &Config) -> Self {
        let counter = match facts.arch {
            Arch::X86_64 => "rdtsc",
            Arch::Aarch64 => "cntvct_el0",
            Arch::Other => "none",
        };
        let ordered_read = facts.ordered_read().name();
        Self {
            kind: Kind::Monotonic,
            fallback: None,
            counter,
            ordered_read,
            reference: wall::RAW_NAME,
            monotonic: wall::MONOTONIC_NAME,
            domain: config.domain,
            hz: 1_000_000_000,
            mult: 1 << 32,
            rate_source: RateSource::None,
            measured_hz: None,
            uncertainty_ppm: None,
            disagreement_ppm: None,
            verify_ppm: None,
            read_ns: None,
            ordered_read_ns: None,
            calibration: Duration::ZERO,
            facts,
            trust: config.trust,
            time_sync: None,
            gap_warn_ns: config.gap_warn_ns,
        }
    }

    /// What the report finds about the host's clocks, a log line each: the
    /// kernel's clock source (a warning where it is not read through the
    /// vDSO, info for a hypervisor's) and how `CLOCK_REALTIME` is kept (a
    /// warning where it is not disciplined to a PTP hardware clock), as they
    /// were at calibration.
    #[must_use]
    pub fn findings(&self) -> Vec<Finding> {
        let mut findings: Vec<Finding> = self
            .facts
            .clocksource_finding(self.facts.clocksource.as_deref())
            .into_iter()
            .collect();
        findings.extend(self.time_sync.as_ref().map(TimeSync::finding));
        findings
    }

    /// [`findings`](Self::findings) as they are now: the clock source read
    /// again and the wall clock's discipline probed again
    /// ([`time_sync`](crate::advanced::time_sync)), for a long-running process's
    /// housekeeping (every minute or so). File reads, one `adjtimex` and
    /// one UDP exchange of at most 50 ms, and it allocates: never a hot
    /// path. The time-sync finding only where calibration probed it.
    #[must_use]
    pub fn recheck_findings(&self) -> Vec<Finding> {
        let mut findings: Vec<Finding> =
            detect::with_clocksource(|clocksource| self.facts.clocksource_finding(clocksource))
                .into_iter()
                .collect();
        if self.time_sync.is_some() {
            findings.push(timesync::time_sync().finding());
        }
        findings
    }

    /// Whether the clock calibration chose may still be used: the gate
    /// ([`HostFacts::gate`]) re-applied, at the trust calibration was
    /// given, to the clock source Linux keeps time with now.
    ///
    /// Calibration trusted the kernel's choice of clock source when it was
    /// made; the kernel can change its mind later (its clocksource watchdog
    /// judging the TSC unstable and moving to `hpet`, an administrator
    /// writing `current_clocksource`), and the counter clock cannot know. A
    /// long-running process calls this from its housekeeping, every few
    /// seconds, and on an error calibrates again (the fallback, with the
    /// reason). Make the new clock's followers again: followers of a
    /// replaced clock keep its last segment and drift
    /// ([`Clock::follower`](crate::Clock::follower)). A file read into a
    /// buffer on the stack: no allocation while the answer is yes, but a
    /// cold path, never a hot one. Always yes for the monotonic fallback,
    /// which needs no vouching.
    ///
    /// # Errors
    /// The reason the counter may no longer be the clock.
    pub fn recheck(&self) -> Result<(), Fallback> {
        detect::with_clocksource(|clocksource| self.recheck_against(clocksource))
    }

    /// [`recheck`](Self::recheck) with the clock source given, as Linux
    /// names it (`None`: unreadable), for a caller that reads it itself.
    ///
    /// # Errors
    /// The reason the counter may no longer be the clock.
    pub fn recheck_against(&self, clocksource: Option<&str>) -> Result<(), Fallback> {
        if self.kind != Kind::Counter {
            return Ok(());
        }
        self.facts.gate_with(clocksource, self.trust)
    }

    /// Every field as `(key, value)`, values without spaces or commas: the
    /// benchmarks' metadata rows, and [`Display`](fmt::Display)'s line.
    #[must_use]
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        let opt = |v: Option<f64>, digits: usize| {
            v.map_or_else(|| "-".to_owned(), |v| format!("{v:.digits$}"))
        };
        let flag = |v: Option<bool>| v.map_or("-", |b| if b { "yes" } else { "no" }).to_owned();
        let facts = &self.facts;
        let mut fields = vec![
            ("kind", self.kind.name().to_owned()),
            (
                "fallback",
                self.fallback
                    .as_ref()
                    .map_or_else(|| "none".to_owned(), ToString::to_string),
            ),
            ("trust", self.trust.name().to_owned()),
            ("counter", self.counter.to_owned()),
            ("ordered_read", self.ordered_read.replace(' ', "_")),
            ("reference", self.reference.to_owned()),
            ("monotonic", self.monotonic.to_owned()),
            ("domain", self.domain.name().to_owned()),
            ("hz", self.hz.to_string()),
            ("mult", self.mult.to_string()),
            ("rate_source", self.rate_source.name().to_owned()),
            ("measured_hz", opt(self.measured_hz, 1)),
            ("uncertainty_ppm", opt(self.uncertainty_ppm, 3)),
            ("disagreement_ppm", opt(self.disagreement_ppm, 3)),
            ("verify_ppm", opt(self.verify_ppm, 3)),
            ("read_ns", opt(self.read_ns, 2)),
            ("ordered_read_ns", opt(self.ordered_read_ns, 2)),
            (
                "calibration_ms",
                format!("{:.3}", self.calibration.as_secs_f64() * 1e3),
            ),
            (
                "clocksource",
                facts.clocksource.clone().unwrap_or_else(|| "-".to_owned()),
            ),
            ("invariant_tsc", flag(facts.invariant_tsc)),
            ("rdtscp", flag(facts.rdtscp)),
            ("hypervisor", flag(facts.hypervisor)),
            ("ecv", flag(facts.ecv)),
            (
                "register_hz",
                facts
                    .register_hz
                    .map_or_else(|| "-".to_owned(), |hz| hz.to_string()),
            ),
            (
                "nominal",
                if facts.nominal.is_empty() {
                    "-".to_owned()
                } else {
                    facts
                        .nominal
                        .iter()
                        .map(|(source, hz)| format!("{}:{hz}", source.name()))
                        .collect::<Vec<_>>()
                        .join(";")
                },
            ),
        ];
        match &self.time_sync {
            Some(sync) => fields.extend(sync.fields()),
            None => fields.push(("time_sync", "off".to_owned())),
        }
        fields
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (key, value)) in self.fields().iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{key}={value}")?;
        }
        Ok(())
    }
}

/// Calibrates a fixed segment, once, at start-up: the `Copy` clock and its
/// report, with nothing kept and nothing printed.
///
/// The front [`calibrate`](crate::calibrate) runs this, then keeps the
/// clock on UTC ([`Clock::update`](crate::Clock::update)) and prints the
/// report's [`warnings`](Report::warnings). A realtime-domain segment from
/// here stays on UTC only under a
/// [`Discipline`](crate::advanced::discipline::Discipline): left alone it
/// keeps the counter's rate and drifts from `CLOCK_REALTIME` by the
/// hardware's rate error (up to 73 ms an hour measured). Set
/// [`Config::domain`] to [`Domain::Monotonic`] for nanoseconds since boot.
///
/// An explicit call that blocks for tens of milliseconds (with the default
/// windows: a 20 ms check of a frequency register, or a 50 ms measurement,
/// and a 20 ms verification) and returns a `Copy` clock to give every thread, so nothing is
/// measured, initialised or locked lazily on a hot path.
///
/// The counter is the clock when the host's facts pass
/// [`HostFacts::gate`] at `config.trust`, a read costs at most
/// `config.max_read_ns`, its rate (a frequency register or a nominal rate
/// the measurement cannot tell apart from what it measured, else the
/// measurement) is verified by a second measurement, and the policy allows
/// it; otherwise the monotonic fallback, with the reason in the report.
///
/// First, unless `config.time_sync` is [`TimeSyncPolicy::Off`], it probes
/// how `CLOCK_REALTIME` is kept ([`time_sync`](crate::advanced::time_sync): file
/// reads, one `adjtimex`, chronyd's command port within 50 ms; on Linux
/// only) for the report's time-sync section; that is not counted in
/// [`Report::calibration`].
///
/// # Errors
/// [`Error::CounterUnavailable`] under [`Policy::RequireCounter`] where the
/// counter cannot be used; [`Error::ClockRead`] when a kernel clock cannot
/// be read; [`Error::NotPtpGrade`] under [`TimeSyncPolicy::RequirePtp`]
/// where the wall clock is not shown to be disciplined to a PTP hardware
/// clock.
pub fn calibrate(config: &Config) -> Result<Calibrated, Error> {
    let time_sync = match config.time_sync {
        TimeSyncPolicy::Off => None,
        TimeSyncPolicy::Warn | TimeSyncPolicy::RequirePtp => Some(timesync::time_sync()),
    };
    if config.time_sync == TimeSyncPolicy::RequirePtp
        && let Some(sync) = &time_sync
        && sync.verdict != SyncVerdict::PtpGrade
    {
        return Err(Error::NotPtpGrade {
            verdict: sync.verdict,
            why: sync.finding().message,
        });
    }
    let started = wall::raw()?;
    let facts = HostFacts::detect();
    let gate = match config.policy {
        Policy::Monotonic => Err(Fallback::Forced),
        Policy::Auto | Policy::RequireCounter => facts.gate(config.trust),
    };
    let mut report = Report::new(facts, config);
    let chosen = match gate {
        Ok(()) => counter(config, &mut report)?,
        Err(why) => Err(why),
    };
    let clock = match chosen {
        Ok(counter) => {
            report.kind = Kind::Counter;
            report.hz = counter.hz();
            report.mult = counter.scale().mult();
            Clock::Counter(counter)
        }
        Err(why) => {
            if config.policy == Policy::RequireCounter {
                return Err(Error::CounterUnavailable(why));
            }
            report.fallback = Some(why);
            report.rate_source = RateSource::None;
            Clock::Monotonic(MonotonicClock::new(config.domain)?)
        }
    };
    report.calibration = Duration::from_nanos(wall::raw()?.saturating_sub(started).max(1));
    report.time_sync = time_sync;
    Ok(Calibrated { clock, report })
}

/// The counter clock, or why not: its read cost, its rate, its anchor in
/// the domain, then the verification.
fn counter(config: &Config, report: &mut Report) -> Result<Result<CounterClock, Fallback>, Error> {
    // the ordered read the clock will take is the one measured, timed and
    // checked for a trap here
    let ordered = report.facts.ordered_read();
    let (read_ns, ordered_ns) = (read_cost(arch::ticks)?, read_cost(|| ordered.read())?);
    report.read_ns = Some(read_ns);
    report.ordered_read_ns = Some(ordered_ns);
    let worst = read_ns.max(ordered_ns);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // ns, far under 2^52
    if worst > config.max_read_ns as f64 {
        return Ok(Err(Fallback::Trapped {
            read_ns: worst.round() as u64,
        }));
    }

    let register = report.facts.register_hz.filter(|&hz| hz >= 1_000);
    let candidates: Vec<(RateSource, u64)> = register
        .map(|hz| (RateSource::Register, hz))
        .into_iter()
        .chain(report.facts.nominal.iter().copied())
        .collect();
    // a register is the kernel's own rate, so a short check settles it; a
    // nominal rate may not be, so it is held to the full window
    let first = if register.is_some() {
        config.check_window
    } else {
        config.window
    };
    let mut m = Measurement::take(first, ordered)?;
    if m.ticks == 0 {
        return Ok(Err(Fallback::CounterStalled));
    }
    let mut chosen = choose(&m, &candidates);
    if chosen.is_none() && first < config.window {
        m = Measurement::take(config.window, ordered)?;
        if m.ticks == 0 {
            return Ok(Err(Fallback::CounterStalled));
        }
        chosen = choose(&m, &candidates);
    }
    report.measured_hz = Some(m.hz());
    report.uncertainty_ppm = Some(m.uncertainty_ppm());
    let (source, rate) = match chosen {
        Some((source, hz)) => (source, Scale::from_hz(hz, 0, 0)),
        None => (RateSource::Measured, Scale::from_mult(m.mult(), 0, 0)),
    };
    let Some(rate) = rate else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return Ok(Err(Fallback::RateOutOfRange {
            hz: m.hz().round() as u64,
        }));
    };
    report.rate_source = source;
    #[allow(clippy::cast_precision_loss)]
    {
        report.disagreement_ppm = Some((rate.hz() as f64 - m.hz()) / m.hz() * 1e6);
    }

    // the anchor: the domain's clock between two ordered counter reads
    let domain_read = || match config.domain {
        Domain::Monotonic => wall::monotonic(),
        Domain::Realtime => wall::read(wall::REALTIME, "CLOCK_REALTIME"),
    };
    let (base_ticks, base_ns, _) = wall::bracket(domain_read, || Ok(ordered.read()))?;
    let scale = rate.rebased(base_ticks, base_ns);

    // the verification: a second window against the rate chosen
    let v = Measurement::take(config.check_window, ordered)?;
    let (verify_ppm, steady) = verify(&scale, &m, &v);
    report.verify_ppm = Some(verify_ppm);
    if !steady {
        #[allow(clippy::cast_possible_truncation)]
        return Ok(Err(Fallback::Unstable {
            ppm: verify_ppm.round() as i64,
        }));
    }
    Ok(Ok(CounterClock::new(scale, config.domain, ordered)))
}

/// A second measurement `v` against the rate chosen from the first, `m`:
/// (its disagreement, ppm; whether it is within both measurements' error
/// bounds, the first's scaled to the second's window, twice, and 10 ppm).
fn verify(scale: &Scale, m: &Measurement, v: &Measurement) -> (f64, bool) {
    let predicted = scale.ticks_to_ns(v.ticks);
    let off = i128::from(predicted) - i128::from(v.ns);
    #[allow(clippy::cast_precision_loss)]
    let ppm = off as f64 / v.ns.max(1) as f64 * 1e6;
    let allowed = 2
        * (u128::from(v.err_ns)
            + u128::from(m.err_ns) * u128::from(v.ns) / u128::from(m.ns.max(1)))
        + u128::from(v.ns) / 100_000;
    (ppm, v.ticks != 0 && off.unsigned_abs() <= allowed)
}

/// The mean cost of one `read`, ns: 1,024 back to back against the raw
/// clock, the least of 5 tries (a try a preemption lands in is not the
/// read's cost).
fn read_cost(read: impl Fn() -> u64) -> Result<f64, Error> {
    let mut best = u64::MAX;
    for _ in 0..5 {
        let start = wall::raw()?;
        for _ in 0..1_024 {
            black_box(read());
        }
        best = best.min(wall::raw()?.saturating_sub(start));
    }
    #[allow(clippy::cast_precision_loss)]
    Ok(best as f64 / 1_024.0)
}

/// The counter against `CLOCK_MONOTONIC_RAW` over a window: ticks and ns
/// between the tightest of 16 brackets at each end, and the error bound on
/// those ns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Measurement {
    /// Counter ticks between the ends (0: it did not advance).
    ticks: u64,
    /// Raw-clock ns between the ends.
    ns: u64,
    /// How far `ns` may be from the true time between the counter reads:
    /// half of each end's bracket, and both clocks' resolutions.
    err_ns: u64,
}

impl Measurement {
    fn take(window: Duration, ordered: OrderedRead) -> Result<Self, Error> {
        let window_ns = u64::try_from(window.as_nanos()).unwrap_or(u64::MAX).max(1);
        let raw_res = wall::resolution_ns(wall::RAW, wall::RAW_NAME)?;
        let sample = || wall::bracket(wall::raw, || Ok(ordered.read()));
        sample()?; // warm-up: the code, the vDSO's page
        let (t0, r0, w0) = sample()?;
        let mut now = wall::raw()?;
        while now.saturating_sub(r0) < window_ns {
            std::hint::spin_loop();
            now = wall::raw()?;
        }
        let (t1, r1, w1) = sample()?;
        Ok(Self::from_ends((t0, r0, w0), (t1, r1, w1), raw_res))
    }

    /// From the two ends: `(counter at the bracket's middle, raw ns, the
    /// bracket's width in ticks)` each.
    fn from_ends(
        (t0, r0, w0): (u64, u64, u64),
        (t1, r1, w1): (u64, u64, u64),
        raw_res: u64,
    ) -> Self {
        let ns = r1.saturating_sub(r0);
        if t1 <= t0 || ns == 0 {
            return Self {
                ticks: 0,
                ns,
                err_ns: 0,
            };
        }
        let ticks = t1 - t0;
        let to_ns = |t: u64| {
            u64::try_from(u128::from(t) * u128::from(ns) / u128::from(ticks)).unwrap_or(u64::MAX)
        };
        let tick_ns = to_ns(1).max(1);
        let err_ns = to_ns(w0.saturating_add(w1)).div_ceil(2) + raw_res + tick_ns;
        Self { ticks, ns, err_ns }
    }

    /// ns per tick, 32.32, rounded.
    fn mult(&self) -> u64 {
        let ticks = u128::from(self.ticks.max(1));
        u64::try_from(((u128::from(self.ns) << 32) + ticks / 2) / ticks).unwrap_or(u64::MAX)
    }

    #[allow(clippy::cast_precision_loss)]
    fn hz(&self) -> f64 {
        self.ticks as f64 * 1e9 / self.ns.max(1) as f64
    }

    #[allow(clippy::cast_precision_loss)]
    fn uncertainty_ppm(&self) -> f64 {
        self.err_ns as f64 / self.ns.max(1) as f64 * 1e6
    }

    /// Whether a counter at `hz` would have counted what was counted, to
    /// within twice the error bound (and a part per billion), and within
    /// [`MAX_CANDIDATE_PPM`]: the measurement cannot tell it apart from the
    /// true rate, and is good enough to say so.
    fn agrees(&self, hz: u64) -> bool {
        let expected = u128::from(hz) * u128::from(self.ns);
        let counted = u128::from(self.ticks) * 1_000_000_000;
        let diff = expected.abs_diff(counted);
        // diff / counted <= 2 err / ns + 1e-9
        let within_bound = diff * u128::from(self.ns)
            <= (2 * u128::from(self.err_ns) * 1_000_000_000 + u128::from(self.ns))
                * u128::from(self.ticks);
        // diff / counted <= MAX_CANDIDATE_PPM / 10^6
        within_bound && diff * 1_000_000 <= MAX_CANDIDATE_PPM * counted
    }
}

/// The furthest, ppm, a candidate rate may be from the measured one and
/// still be taken, however loose the measurement's bound: twice what an
/// honest bound needs (a 24 MHz counter over 20 ms: about 8 ppm). A
/// measurement too poor to tell a candidate apart from 50 ppm off confirms
/// none, and its own rate is used.
const MAX_CANDIDATE_PPM: u128 = 50;

/// The first candidate the measurement cannot tell apart from its rate.
fn choose(m: &Measurement, candidates: &[(RateSource, u64)]) -> Option<(RateSource, u64)> {
    candidates.iter().copied().find(|&(_, hz)| m.agrees(hz))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 50 ms of a 2.4 GHz counter with 40 ns brackets: an error bound of
    /// about 1 ppm.
    fn measured(hz: u64, ns: u64, bracket_ns: u64) -> Measurement {
        let ticks = u64::try_from(u128::from(hz) * u128::from(ns) / 1_000_000_000).unwrap_or(0);
        let w = bracket_ns * hz / 1_000_000_000;
        Measurement::from_ends((1_000, 0, w), (1_000 + ticks, ns, w), 1)
    }

    #[test]
    fn the_error_bound_is_the_brackets_and_resolutions_over_the_window() {
        let m = measured(2_400_000_000, 50_000_000, 40);
        // 40 ns per end, halved and summed: 40; raw resolution 1; a tick 1
        assert_eq!(m.err_ns, 42, "{m:?}");
        assert!(
            (m.uncertainty_ppm() - 0.84).abs() < 1e-9,
            "{}",
            m.uncertainty_ppm()
        );
        assert!((m.hz() - 2.4e9).abs() < 1.0);
        assert_eq!(
            m.mult(),
            Scale::from_hz(2_400_000_000, 0, 0).map_or(0, |s| s.mult())
        );
    }

    /// A candidate is taken only when the measurement cannot tell it apart:
    /// within twice the error bound, not a part per million beyond.
    #[test]
    fn a_candidate_within_the_error_bound_is_taken_and_one_beyond_is_not() {
        let m = measured(2_400_000_000, 50_000_000, 40);
        let u = 2.0 * 42.0 / 50e6; // twice the bound, relative
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let off = |frac: f64| (2.4e9 * (1.0 + frac)).round() as u64;
        assert!(m.agrees(2_400_000_000));
        assert!(m.agrees(off(u * 0.9)));
        assert!(m.agrees(off(-u * 0.9)));
        assert!(!m.agrees(off(u * 1.1)));
        assert!(!m.agrees(off(-u * 1.1)));
        assert!(!m.agrees(2_400_100_000), "42 ppm off");
        let candidates = [
            (RateSource::KernelTscKhz, 2_400_100_000),
            (RateSource::Hypervisor, 2_400_000_000),
            (RateSource::CpuidBase, 2_400_000_000),
        ];
        assert_eq!(
            choose(&m, &candidates),
            Some((RateSource::Hypervisor, 2_400_000_000))
        );
        assert_eq!(choose(&m, &candidates[..1]), None);
    }

    /// The verification passes a counter that kept the rate chosen and
    /// refuses one whose rate moved (a TSC that is not invariant after all,
    /// a hypervisor rescaling it): 100 ppm off over 20 ms is refused, the
    /// bounds' worth is not.
    #[test]
    fn the_verification_refuses_a_counter_whose_rate_moved() {
        let m = measured(2_400_000_000, 50_000_000, 40);
        let Some(scale) = Scale::from_hz(2_400_000_000, 0, 0) else {
            return;
        };
        let (ppm, steady) = verify(&scale, &m, &measured(2_400_000_000, 20_000_000, 40));
        assert!(steady && ppm.abs() < 0.01, "{ppm}");
        // within the bounds: 2 x (42 + 42 x 20/50) ns + 10 ppm of 20 ms
        let (ppm, steady) = verify(&scale, &m, &measured(2_400_024_000, 20_000_000, 40));
        assert!(steady, "10 ppm off: {ppm}");
        let (ppm, steady) = verify(&scale, &m, &measured(2_400_240_000, 20_000_000, 40));
        assert!(!steady && (ppm - 100.0).abs() < 1.0, "100 ppm off: {ppm}");
        let (_, steady) = verify(
            &scale,
            &m,
            &Measurement::from_ends((5, 0, 1), (5, 20_000_000, 1), 1),
        );
        assert!(!steady, "a stalled counter");
    }

    /// A counter that did not advance measures nothing.
    #[test]
    fn a_stalled_counter_measures_nothing() {
        let m = Measurement::from_ends((5_000, 0, 2), (5_000, 20_000_000, 2), 1);
        assert_eq!(m.ticks, 0);
        let m = Measurement::from_ends((5_000, 0, 2), (4_000, 20_000_000, 2), 1);
        assert_eq!(m.ticks, 0);
    }

    /// However loose the measurement's bound (a raw clock's resolution
    /// overstated: Linux's `clock_getres` is a jiffy on a kernel without
    /// high-resolution timers, a 5% bound over 20 ms; a host so noisy every
    /// bracket was wide), a candidate more than 50 ppm from what was
    /// measured is never taken: the measured rate is, and the report says
    /// how far it is from the candidates.
    #[test]
    fn a_candidate_far_from_the_measurement_is_never_taken() {
        // 20 ms of a 2.4 GHz counter with a millisecond of raw resolution
        let m = Measurement::from_ends(
            (1_000, 0, 0),
            (1_000 + 48_000_000, 20_000_000, 0),
            1_000_000,
        );
        assert!(m.uncertainty_ppm() > 40_000.0, "{}", m.uncertainty_ppm());
        assert!(m.agrees(2_400_000_000));
        assert!(m.agrees(2_400_024_000), "10 ppm off: within the cap");
        assert!(!m.agrees(2_400_240_000), "100 ppm off");
        assert!(!m.agrees(2_424_000_000), "1% off");
        assert!(!m.agrees(2_376_000_000), "1% off, below");
        let candidates = [(RateSource::CpuidBase, 2_424_000_000)];
        assert_eq!(choose(&m, &candidates), None);
    }

    /// The recheck re-applies the gate, at the trust calibration was given,
    /// to the clock source the kernel names now: a counter the kernel no
    /// longer keeps time with (its watchdog judged the TSC unstable and
    /// moved to `hpet`) is reported with the reason; the fallback needs no
    /// vouch.
    #[test]
    fn the_recheck_sees_the_kernel_drop_the_counter() {
        let facts = HostFacts {
            clocksource: Some("tsc".to_owned()),
            invariant_tsc: Some(true),
            rdtscp: Some(true),
            ..HostFacts::unknown(Arch::X86_64, crate::detect::Os::Linux)
        };
        let mut report = Report::new(facts, &Config::default());
        report.kind = Kind::Counter;
        assert_eq!(report.trust, Trust::Kernel);
        assert_eq!(report.recheck_against(Some("tsc")), Ok(()));
        assert_eq!(report.recheck_against(Some("tsc\n")), Ok(()));
        let hpet = Err(Fallback::KernelClocksource("hpet".to_owned()));
        assert_eq!(report.recheck_against(Some("hpet")), hpet);
        assert_eq!(
            report.recheck_against(None),
            Err(Fallback::ClocksourceUnreadable)
        );
        assert_eq!(
            report.recheck_against(Some("kvm-clock")),
            Err(Fallback::Paravirt("kvm-clock".to_owned()))
        );
        report.trust = Trust::CrossCheck;
        assert_eq!(report.recheck_against(Some("kvm-clock")), Ok(()));
        assert_eq!(report.recheck_against(None), Ok(()));
        assert_eq!(report.recheck_against(Some("hpet")), hpet);
        report.kind = Kind::Monotonic;
        assert_eq!(report.recheck_against(Some("hpet")), Ok(()));
    }

    /// A coarse counter (Apple silicon's 24 MHz, 41.7 ns a tick) and a
    /// coarse raw clock widen the bound by their resolutions: its
    /// frequency register is still told apart from 0.1% off.
    #[test]
    fn a_coarse_counter_widens_the_bound() {
        let m = measured(24_000_000, 20_000_000, 0);
        assert!(m.err_ns >= 42, "{m:?}");
        assert!(m.agrees(24_000_000));
        assert!(!m.agrees(24_024_000));
    }
}
