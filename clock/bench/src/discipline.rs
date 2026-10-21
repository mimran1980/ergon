//! `clockbench --discipline <secs>`: ergon-clock's discipline of the
//! calibrated counter to `CLOCK_REALTIME`, live, with a verifier.
//!
//! The owner (this thread) calibrates in the realtime domain, then once a
//! second takes a sample through the kernel's clocks and re-anchors the
//! clock ([`Discipline::sample_and_apply`]) and publishes it; the first
//! update is the start update. A verifier thread, at `--verifier-hz` (100),
//! reads the published clock through a [`ClockFollower`] and brackets
//! `CLOCK_REALTIME` between two of its ordered reads: the residual between
//! the owner's samples, which is what a stamp taken there is off by. Each
//! update's cost (the sample, the controller and the publish) is timed.
//!
//! Both threads busy-wait between their work, as threads of a
//! busy-spinning runtime do: a sleep lets the core idle, and the update or
//! bracket after it pays the idle state's exit and cold caches. Pinned, each
//! needs a CPU of its own (`--cpu` the owner's, `--cpu2` the verifier's).
//!
//! `meta` records after the run's settings: `owner_context_switches` and
//! `verifier_context_switches` (`<voluntary>;<involuntary>` over the run,
//! Linux; a voluntary one is a sleep) and `verifier_cpus` (where it could
//! run, `;`-separated, Linux; `-` elsewhere).
//!
//! Records (CSV, after the `meta` records):
//!
//! - `discipline,<s>,<action>,<residual_ns>,<wall_offset_ns>,<error_ns>,
//!   <rate_ppb>,<freq_ppb>,<wall_step_ns>,<flags>,<cost_ns>,<adjtimex>`:
//!   one an update; `<adjtimex>` is `status;maxerror_us;freq_ppm` (Linux)
//!   or `-`.
//! - `truth,<s>,<residual_ns>,<width_ns>`: one a verifier read.
//! - `discipline_summary,<series>,<n>,<p50>,<p99>,<p99.9>,<max>`: the
//!   absolute residual at the samples and from the verifier, ns (a verifier
//!   bracket over ten times the median width was interrupted, and its
//!   residual is off by up to half of it: those are summarised apart, by
//!   width), and the update's cost, ns.
//!
//! Seconds count from the start update.

use crate::sys;
use ergon_clock::advanced::discipline::{Action, Adjustment, Discipline, DisciplineConfig, Phase};
use ergon_clock::advanced::share::{ClockFollower, ClockPublisher, SharedClock};
use ergon_clock::advanced::{
    KernelSource, KernelSync, Sampler, TimeSyncPolicy, Trust, calibrate, monotonic_raw_ns,
    realtime_ns, realtime_res_ns,
};
use ergon_clock::{Config, Domain};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};

/// One update, as recorded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Update {
    /// ns since the start update (`CLOCK_MONOTONIC_RAW`).
    pub at_ns: u64,
    /// What the discipline did.
    pub adjustment: Adjustment,
    /// The update's cost: sample, controller and publish, ns.
    pub cost_ns: u64,
    /// The kernel's view then (Linux).
    pub kernel: Option<KernelSync>,
}

/// One verifier read: the wall clock bracketed by the follower's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Truth {
    /// ns since the start update.
    pub at_ns: u64,
    /// `CLOCK_REALTIME` less the follower's clock at the bracket's middle.
    pub residual_ns: i64,
    /// The bracket's width, ns.
    pub width_ns: u64,
}

/// The `discipline` record of `u`.
#[must_use]
pub fn discipline_record(u: &Update) -> Vec<String> {
    let a = &u.adjustment;
    let action = match a.action {
        Action::Slewed => "slewed".to_owned(),
        Action::Stepped { ns } => format!("stepped:{ns}"),
        Action::Held => "held".to_owned(),
    };
    let kernel = u.kernel.map_or_else(
        || "-".to_owned(),
        |k| format!("{};{};{:.3}", k.status, k.maxerror_us, k.freq_ppm),
    );
    vec![
        "discipline".into(),
        seconds(u.at_ns),
        action,
        a.residual_ns.to_string(),
        a.wall_offset_ns.to_string(),
        a.error_ns.to_string(),
        (a.rate_ppt / 1_000).to_string(),
        (a.freq_ppt / 1_000).to_string(),
        a.wall_step_ns.to_string(),
        a.flags.to_string(),
        u.cost_ns.to_string(),
        kernel,
    ]
}

/// The `truth` record of `t`.
#[must_use]
pub fn truth_record(t: &Truth) -> Vec<String> {
    vec![
        "truth".into(),
        seconds(t.at_ns),
        t.residual_ns.to_string(),
        t.width_ns.to_string(),
    ]
}

/// ns as seconds, to the microsecond.
fn seconds(ns: u64) -> String {
    format!("{}.{:06}", ns / 1_000_000_000, ns % 1_000_000_000 / 1_000)
}

/// A series' order statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spread {
    /// Values.
    pub n: usize,
    /// Median.
    pub p50: u64,
    /// 99th percentile.
    pub p99: u64,
    /// 99.9th percentile.
    pub p999: u64,
    /// Largest.
    pub max: u64,
}

impl Spread {
    /// Of `values` (sorted here); `None` when empty.
    #[must_use]
    pub fn of(values: &mut [u64]) -> Option<Self> {
        values.sort_unstable();
        let last = values.len().checked_sub(1)?;
        let at = |q: usize| values[last * q / 1_000];
        Some(Self {
            n: values.len(),
            p50: at(500),
            p99: at(990),
            p999: at(999),
            max: values[last],
        })
    }

    /// The `discipline_summary` record of `series`.
    #[must_use]
    pub fn record(&self, series: &str) -> Vec<String> {
        vec![
            "discipline_summary".into(),
            series.into(),
            self.n.to_string(),
            self.p50.to_string(),
            self.p99.to_string(),
            self.p999.to_string(),
            self.max.to_string(),
        ]
    }
}

/// What a run measured.
#[derive(Clone, Debug, Default)]
pub struct Run {
    /// Every update.
    pub updates: Vec<Update>,
    /// Every verifier read.
    pub truths: Vec<Truth>,
    /// The owner's voluntary and involuntary context switches over the
    /// updates (Linux; zeros elsewhere): voluntary ones are sleeps.
    pub owner_switches: (u64, u64),
    /// The verifier's, over its reads.
    pub verifier_switches: (u64, u64),
    /// The CPUs the verifier could run on, pinned (Linux; empty elsewhere).
    pub verifier_cpus: Vec<usize>,
}

impl Run {
    /// The summaries: residual at the samples, residual from the verifier
    /// (its uninterrupted brackets), the interrupted brackets' widths (where
    /// there were any), update cost.
    #[must_use]
    pub fn summaries(&self) -> Vec<(&'static str, Spread)> {
        let mut at_samples: Vec<u64> = self
            .updates
            .iter()
            .filter(|u| u.adjustment.action != Action::Held)
            .map(|u| u.adjustment.residual_ns.unsigned_abs())
            .collect();
        // a verifier bracket ten times wider than the median was interrupted
        // (its residual is off by up to half its width): summarised apart,
        // by width
        let mut widths: Vec<u64> = self.truths.iter().map(|t| t.width_ns).collect();
        let wide = Spread::of(&mut widths).map_or(u64::MAX, |w| w.p50.saturating_mul(10).max(1));
        let mut verifier: Vec<u64> = self
            .truths
            .iter()
            .filter(|t| t.width_ns <= wide)
            .map(|t| t.residual_ns.unsigned_abs())
            .collect();
        let mut interrupted: Vec<u64> = self
            .truths
            .iter()
            .filter(|t| t.width_ns > wide)
            .map(|t| t.width_ns)
            .collect();
        let mut cost: Vec<u64> = self.updates.iter().map(|u| u.cost_ns).collect();
        [
            ("residual_at_samples_ns", &mut at_samples),
            ("residual_verifier_ns", &mut verifier),
            ("verifier_interrupted_width_ns", &mut interrupted),
            ("update_cost_ns", &mut cost),
        ]
        .into_iter()
        .filter_map(|(name, v)| Spread::of(v).map(|s| (name, s)))
        .collect()
    }
}

/// Busy-waits until `CLOCK_MONOTONIC_RAW` reads `due_ns`, as a thread of a
/// busy-spinning runtime waits: never a sleep, so the core never idles and
/// what runs next starts warm (sleeping, c7i's cores reached C6, which
/// empties their caches: clock/BENCHMARKS.md).
fn spin_until(due_ns: u64) {
    while monotonic_raw_ns() < due_ns {
        std::hint::spin_loop();
    }
}

/// Calibrates in the realtime domain and disciplines for `secs` seconds.
///
/// The verifier reads at `verifier_hz`, pinned to `verifier_cpu` (`None`:
/// wherever the caller's thread may run). Both threads busy-wait.
///
/// # Errors
/// Calibration failed, the discipline refused the clock, or the verifier
/// could not be pinned.
pub fn run(
    secs: u64,
    verifier_hz: u64,
    verifier_cpu: Option<usize>,
) -> Result<Run, Box<dyn Error>> {
    let mut clock = calibrate(&Config {
        domain: Domain::Realtime,
        trust: Trust::CrossCheck,
        time_sync: TimeSyncPolicy::Off,
        ..Config::default()
    })?
    .clock;
    let mut discipline = Discipline::new(&clock, DisciplineConfig::default())?;
    let mut sampler = Sampler::for_clock(&clock, realtime_res_ns()?);
    let shared = SharedClock::new();
    let mut publisher = ClockPublisher::new(&shared).ok_or("the shared clock is claimed")?;
    publisher.publish(&clock);
    let done = AtomicBool::new(false);
    let start = monotonic_raw_ns();
    let period_ns = 1_000_000_000 / verifier_hz.max(1);
    let initial = clock;
    std::thread::scope(|scope| {
        let verifier = scope.spawn(|| {
            if let Some(cpu) = verifier_cpu {
                sys::pin(cpu).map_err(|e| format!("cannot pin the verifier to CPU {cpu}: {e}"))?;
            }
            let cpus = sys::allowed_cpus();
            let s0 = sys::context_switches();
            let mut follower = ClockFollower::new(&shared, initial);
            let mut truths = Vec::with_capacity(usize::try_from(secs * verifier_hz).unwrap_or(0));
            while !done.load(Ordering::Acquire) {
                let c = *follower.clock();
                let a = c.now_ns_ordered();
                let wall = realtime_ns();
                let b = c.now_ns_ordered();
                let width = b.saturating_sub(a);
                truths.push(Truth {
                    at_ns: monotonic_raw_ns().saturating_sub(start),
                    residual_ns: wall.wrapping_sub((a + width / 2).cast_signed()),
                    width_ns: width,
                });
                spin_until(monotonic_raw_ns() + period_ns);
            }
            let s1 = sys::context_switches();
            Ok::<_, String>((truths, (s1.0 - s0.0, s1.1 - s0.1), cpus))
        });
        let mut updates = Vec::with_capacity(usize::try_from(secs + 1).unwrap_or(0));
        let o0 = sys::context_switches();
        for k in 0..=secs {
            // the next whole second from the start update
            spin_until(start + k * 1_000_000_000);
            let phase = if k == 0 { Phase::Start } else { Phase::Running };
            let reader = clock;
            let began = monotonic_raw_ns();
            let adjustment = discipline.sample_and_apply(
                &mut clock,
                &mut sampler,
                &mut KernelSource(&reader),
                phase,
            );
            publisher.publish(&clock);
            let ended = monotonic_raw_ns();
            updates.push(Update {
                at_ns: began.saturating_sub(start),
                adjustment,
                cost_ns: ended.saturating_sub(began),
                kernel: KernelSync::read().ok(),
            });
        }
        let o1 = sys::context_switches();
        done.store(true, Ordering::Release);
        let (truths, verifier_switches, verifier_cpus) = verifier
            .join()
            .map_err(|_| "the verifier thread panicked")??;
        Ok(Run {
            updates,
            truths,
            owner_switches: (o1.0 - o0.0, o1.1 - o0.1),
            verifier_switches,
            verifier_cpus,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The order statistics of a series, and none of an empty one.
    #[test]
    fn a_spread_is_its_order_statistics() {
        let mut v: Vec<u64> = (1..=1_000).rev().collect();
        let s = Spread::of(&mut v);
        assert_eq!(
            s,
            Some(Spread {
                n: 1_000,
                p50: 500,
                p99: 990,
                p999: 999,
                max: 1_000
            })
        );
        assert_eq!(Spread::of(&mut []), None);
        let one = Spread::of(&mut [7]);
        assert_eq!(one.map(|s| (s.p50, s.p99, s.max)), Some((7, 7, 7)));
    }

    /// A verifier bracket over ten times the median width was interrupted:
    /// its residual (up to half its width) is summarised by width, apart.
    #[test]
    fn interrupted_brackets_are_summarised_apart() {
        let mut truths: Vec<Truth> = (0..100)
            .map(|i| Truth {
                at_ns: i,
                residual_ns: 50 - i64::try_from(i).unwrap_or(0),
                width_ns: 160 + i % 10,
            })
            .collect();
        truths.push(Truth {
            at_ns: 100,
            residual_ns: 2_849_874,
            width_ns: 5_701_560,
        });
        let run = Run {
            truths,
            ..Run::default()
        };
        let s = run.summaries();
        let find = |name: &str| s.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        assert_eq!(
            find("residual_verifier_ns").map(|v| (v.n, v.max)),
            Some((100, 50))
        );
        assert_eq!(
            find("verifier_interrupted_width_ns").map(|v| (v.n, v.max)),
            Some((1, 5_701_560))
        );
    }

    /// The owner busy-waits between updates and the verifier between reads,
    /// as threads of a busy-spinning runtime do: neither sleeps (no
    /// voluntary context switch; Linux counts them, elsewhere they read 0),
    /// so no update or bracket starts on a core back from idle. The
    /// verifier runs on the CPU it was given (Linux; nothing pins
    /// elsewhere).
    #[test]
    fn the_owner_and_the_verifier_spin_the_verifier_on_its_own_cpu() -> Result<(), Box<dyn Error>> {
        let _alone = crate::faults_alone();
        let cpu = crate::sys::allowed_cpus().last().copied();
        let run = run(2, 100, cpu)?;
        assert_eq!(run.owner_switches.0, 0, "the owner slept");
        assert_eq!(run.verifier_switches.0, 0, "the verifier slept");
        if cfg!(target_os = "linux") {
            assert_eq!(run.verifier_cpus, cpu.into_iter().collect::<Vec<_>>());
        } else {
            assert_eq!(run.verifier_cpus, Vec::<usize>::new());
        }
        Ok(())
    }

    /// A short live run on this host: an update a second, the first the
    /// start update, every update's clock never behind the last, and the
    /// verifier reading between them. The residual's size is the host's
    /// (macOS: NTP slews its wall clock, and nothing here is a verdict).
    #[test]
    fn a_short_run_disciplines_and_verifies() -> Result<(), Box<dyn Error>> {
        // its threads and buffers fault pages: never beside the gap pass's
        // count of them (the whole process's, off Linux)
        let _alone = crate::faults_alone();
        let run = run(2, 100, None)?;
        assert_eq!(run.updates.len(), 3);
        assert_ne!(
            run.updates[0].adjustment.flags & ergon_clock::advanced::discipline::flags::START,
            0
        );
        assert!(
            run.truths.len() >= 10,
            "{} verifier reads",
            run.truths.len()
        );
        for w in run.updates.windows(2) {
            let (a, b) = (&w[0].adjustment, &w[1].adjustment);
            let at = b.clock.params().base_ticks;
            assert!(
                b.clock.to_ns(at) >= a.clock.to_ns(at),
                "a re-anchor went back"
            );
        }
        let records: Vec<Vec<String>> = run.updates.iter().map(discipline_record).collect();
        assert!(
            records
                .iter()
                .all(|r| r.len() == 12 && r[0] == "discipline")
        );
        assert!(run.summaries().len() >= 3);
        Ok(())
    }
}
