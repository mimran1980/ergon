//! Per-event cost on the calling thread and what explains its maximum.
//!
//! A timed loop (`tests/hot_stdout.rs`, `benches/log_event.rs`) records each
//! event's cost with quanta's counter ([`Spread`]) and, around the window,
//! what can stretch one event ([`Window`]):
//!
//! | source | read from |
//! |---|---|
//! | page faults of the thread (minor, major) | `/proc/thread-self/stat` fields 10 and 12 |
//! | context switches of the thread | `/proc/thread-self/status` |
//! | steal time of its CPU, 10 ms ticks | `/proc/stat` `cpu<N>` field 8 |
//! | interrupts on its CPU, per row | `/proc/interrupts` (Docker hides it: `irqs=-`) |
//! | softirqs on its CPU, per row (`TIMER` counts the tick) | `/proc/softirqs` |
//! | the CPU's own stalls with no event | [`idle_gaps`]: the longest gap between two counter reads, spinning for [`idle_window_ns`] |
//!
//! [`Window::cause`] calls the max explained when the CPU stalls about as
//! long with no event at all (the environment: a hypervisor or the host
//! taking the vCPU), lists the sources that moved in the window otherwise
//! (counted per window, not per event, so they are candidates), and says
//! "unexplained" when nothing moved. Off Linux the `/proc` columns are
//! absent and only the idle gap is measured.

use crate::LlError;
use crate::probe::interrupts::{Table, parse_interrupts, parse_softirqs};

/// p50 to max of a set of costs, ns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spread {
    /// How many.
    pub n: usize,
    /// Median.
    pub p50: u64,
    /// 90th.
    pub p90: u64,
    /// 99th.
    pub p99: u64,
    /// 99.9th.
    pub p999: u64,
    /// 99.99th.
    pub p9999: u64,
    /// The largest.
    pub max: u64,
}

impl Spread {
    /// Sorts `costs` and takes its percentiles (nearest rank).
    #[must_use]
    pub fn of(costs: &mut [u64]) -> Self {
        costs.sort_unstable();
        let at = |q: f64| -> u64 {
            if costs.is_empty() {
                return 0;
            }
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let i = ((costs.len() as f64 - 1.0) * q).round() as usize;
            costs[i.min(costs.len() - 1)]
        };
        Self {
            n: costs.len(),
            p50: at(0.50),
            p90: at(0.90),
            p99: at(0.99),
            p999: at(0.999),
            p9999: at(0.9999),
            max: costs.last().copied().unwrap_or(0),
        }
    }

    /// `n=… p50=… p90=… p99=… p99.9=… p99.99=… max=…`, tab-separated.
    #[must_use]
    pub fn fields(&self) -> String {
        format!(
            "n={}\tp50={}\tp90={}\tp99={}\tp99.9={}\tp99.99={}\tmax={}",
            self.n, self.p50, self.p90, self.p99, self.p999, self.p9999, self.max
        )
    }
}

/// The calling thread's counters and its CPU's at one moment (Linux).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// The CPU the interrupt and steal columns are for.
    pub cpu: usize,
    /// Minor page faults of the thread.
    pub minflt: u64,
    /// Major page faults of the thread.
    pub majflt: u64,
    /// Voluntary context switches of the thread.
    pub voluntary: u64,
    /// Involuntary context switches of the thread.
    pub nonvoluntary: u64,
    /// Steal ticks of the CPU (`None`: not in `/proc/stat`).
    pub steal_ticks: Option<u64>,
    /// `/proc/interrupts` (`None`: unreadable).
    pub irqs: Option<Table>,
    /// `/proc/softirqs` (`None`: unreadable).
    pub softirqs: Option<Table>,
}

/// Minor and major faults from a `stat` line (fields 10 and 12).
///
/// # Errors
///
/// [`LlError::BadValue`] when the line is too short or a field is not a
/// number.
pub fn parse_faults(stat: &str) -> Result<(u64, u64), LlError> {
    let bad = |why: &str| LlError::bad("stat", stat.trim_end(), why);
    let close = stat.rfind(") ").ok_or_else(|| bad("no ) after comm"))?;
    let rest: Vec<&str> = stat[close + 2..].split_whitespace().collect();
    let field = |n: usize| -> Result<u64, LlError> {
        rest.get(n - 3)
            .ok_or_else(|| bad("too few fields"))?
            .parse()
            .map_err(|_| bad("a fault count is not a number"))
    };
    Ok((field(10)?, field(12)?))
}

/// `cpu<N>`'s steal ticks from `/proc/stat` (the 8th number).
#[must_use]
pub fn parse_steal(stat: &str, cpu: usize) -> Option<u64> {
    let name = format!("cpu{cpu}");
    stat.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next()? == name).then_some(())?;
        words.nth(7)?.parse().ok()
    })
}

/// Reads the calling thread's counters and `cpu`'s.
///
/// # Errors
///
/// [`LlError::Io`] or [`LlError::BadValue`] when the thread's own `stat` or
/// `status` cannot be read (off Linux: always). The CPU's columns are
/// `None` when unreadable.
pub fn snapshot(cpu: usize) -> Result<Snapshot, LlError> {
    let read = |path: &str| std::fs::read_to_string(path).map_err(|e| LlError::io(path, e));
    let (minflt, majflt) = parse_faults(&read("/proc/thread-self/stat")?)?;
    let status = crate::probe::proc::parse_status(&read("/proc/thread-self/status")?)?;
    Ok(Snapshot {
        cpu,
        minflt,
        majflt,
        voluntary: status.voluntary,
        nonvoluntary: status.nonvoluntary,
        steal_ticks: read("/proc/stat").ok().and_then(|s| parse_steal(&s, cpu)),
        irqs: read("/proc/interrupts")
            .ok()
            .and_then(|s| parse_interrupts(&s).ok()),
        softirqs: read("/proc/softirqs")
            .ok()
            .and_then(|s| parse_softirqs(&s).ok()),
    })
}

/// How long [`idle_gaps`] spins after a window of `wall_ns`: three times
/// the window, at least 500 ms, so a stall as rare as the window's worst
/// has a fair chance to show.
#[must_use]
pub const fn idle_window_ns(wall_ns: u64) -> u64 {
    let three = wall_ns.saturating_mul(3);
    if three > 500_000_000 {
        three
    } else {
        500_000_000
    }
}

/// The longest gap between two consecutive counter reads while spinning
/// for `over_ns`, and how many gaps passed 10 µs: what the CPU does to a
/// thread that runs no event at all.
#[must_use]
pub fn idle_gaps(clock: &quanta::Clock, over_ns: u64) -> (u64, u64) {
    let start = clock.raw();
    let (mut last, mut max, mut over) = (start, 0_u64, 0_u64);
    loop {
        let now = clock.raw();
        let gap = clock.delta_as_nanos(last, now);
        max = max.max(gap);
        over += u64::from(gap > 10_000);
        last = now;
        if clock.delta_as_nanos(start, now) >= over_ns {
            return (max, over);
        }
    }
}

/// What moved over a timed window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Window {
    /// From `/proc`; `None` off Linux.
    pub counters: Option<Counters>,
    /// The idle gap max over [`idle_window_ns`], ns.
    pub idle_gap_max: u64,
    /// Idle gaps over 10 µs.
    pub idle_gaps_over_10us: u64,
}

/// The `/proc` deltas of a [`Window`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// The CPU.
    pub cpu: usize,
    /// Minor faults.
    pub minflt: u64,
    /// Major faults.
    pub majflt: u64,
    /// Voluntary switches.
    pub voluntary: u64,
    /// Involuntary switches.
    pub nonvoluntary: u64,
    /// Steal ticks (`None`: not readable).
    pub steal_ticks: Option<u64>,
    /// Interrupt rows that moved on the CPU, `(name, count)` (`None`:
    /// `/proc/interrupts` unreadable).
    pub irqs: Option<Vec<(String, u64)>>,
    /// Softirq rows that moved on the CPU (`None`: unreadable).
    pub softirqs: Option<Vec<(String, u64)>>,
}

/// The rows of `after` that moved on `cpu` since `before`.
fn moved(before: Option<&Table>, after: Option<&Table>, cpu: usize) -> Option<Vec<(String, u64)>> {
    let (b, a) = (before?, after?);
    Some(
        a.deltas(b, cpu)
            .into_iter()
            .filter(|(_, _, d)| *d > 0)
            .map(|(name, _, d)| (name, d))
            .collect(),
    )
}

/// `name:count,…`, `none` when empty, `-` when unreadable.
fn rows(r: Option<&Vec<(String, u64)>>) -> String {
    match r {
        None => "-".to_owned(),
        Some(v) if v.is_empty() => "none".to_owned(),
        Some(v) => v
            .iter()
            .map(|(n, d)| format!("{n}:{d}"))
            .collect::<Vec<_>>()
            .join(","),
    }
}

impl Counters {
    /// `after - before`.
    #[must_use]
    pub fn between(before: &Snapshot, after: &Snapshot) -> Self {
        Self {
            cpu: after.cpu,
            minflt: after.minflt.saturating_sub(before.minflt),
            majflt: after.majflt.saturating_sub(before.majflt),
            voluntary: after.voluntary.saturating_sub(before.voluntary),
            nonvoluntary: after.nonvoluntary.saturating_sub(before.nonvoluntary),
            steal_ticks: after
                .steal_ticks
                .zip(before.steal_ticks)
                .map(|(a, b)| a.saturating_sub(b)),
            irqs: moved(before.irqs.as_ref(), after.irqs.as_ref(), after.cpu),
            softirqs: moved(before.softirqs.as_ref(), after.softirqs.as_ref(), after.cpu),
        }
    }
}

impl Window {
    /// The window's fields, tab-separated.
    #[must_use]
    pub fn fields(&self) -> String {
        let proc_fields = self.counters.as_ref().map_or_else(String::new, |c| {
            format!(
                "cpu={}\tminflt={}\tmajflt={}\tvol_ctxt={}\tinvol_ctxt={}\tsteal_ticks={}\tirqs={}\tsoftirqs={}\t",
                c.cpu,
                c.minflt,
                c.majflt,
                c.voluntary,
                c.nonvoluntary,
                c.steal_ticks.map_or_else(|| "-".to_owned(), |s| s.to_string()),
                rows(c.irqs.as_ref()),
                rows(c.softirqs.as_ref()),
            )
        });
        format!(
            "{proc_fields}idle_gap_max={}\tidle_gaps_over_10us={}",
            self.idle_gap_max, self.idle_gaps_over_10us
        )
    }

    /// What explains `spread.max`, the candidates, or "unexplained".
    #[must_use]
    pub fn cause(&self, spread: &Spread) -> String {
        // a stall the CPU also shows with no event explains a max of about
        // the same size (half or more of it)
        if self.idle_gap_max.saturating_mul(2) >= spread.max.saturating_sub(spread.p50) {
            return format!(
                "explained: the CPU stalls {} ns with no event at all (the environment, not the event)",
                self.idle_gap_max
            );
        }
        let mut seen = Vec::new();
        if let Some(c) = &self.counters {
            if c.majflt + c.minflt > 0 {
                seen.push(format!(
                    "page faults (minor {}, major {})",
                    c.minflt, c.majflt
                ));
            }
            if c.voluntary + c.nonvoluntary > 0 {
                seen.push(format!(
                    "context switches (voluntary {}, involuntary {})",
                    c.voluntary, c.nonvoluntary
                ));
            }
            if let Some(steal) = c.steal_ticks.filter(|s| *s > 0) {
                seen.push(format!("steal ({steal} ticks)"));
            }
            for (what, r) in [("interrupts", &c.irqs), ("softirqs", &c.softirqs)] {
                if r.as_ref().is_some_and(|v| !v.is_empty()) {
                    seen.push(format!("{what} on cpu {} ({})", c.cpu, rows(r.as_ref())));
                }
            }
        }
        if seen.is_empty() {
            format!(
                "unexplained: {}, and the CPU's idle stalls reach only {} ns",
                if self.counters.is_some() {
                    "no fault, switch, steal or interrupt in the window"
                } else {
                    "no /proc here"
                },
                self.idle_gap_max
            )
        } else {
            format!(
                "candidates (counted over the window, not per event): {}; idle stalls reach only {} ns",
                seen.join("; "),
                self.idle_gap_max
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spread_takes_nearest_ranks() {
        let mut v: Vec<u64> = (1..=10_000).rev().collect();
        let s = Spread::of(&mut v);
        assert_eq!(
            (s.n, s.p50, s.p99, s.p9999, s.max),
            (10_000, 5_001, 9_900, 9_999, 10_000)
        );
        assert_eq!(Spread::of(&mut []), Spread::default());
    }

    #[test]
    fn faults_and_steal_parse() -> Result<(), LlError> {
        let stat = "42 (ll hot) R 1 42 42 0 -1 4194560 17 0 3 0 5 6 0 0 20 0 1 0 1 0 0";
        assert_eq!(parse_faults(stat)?, (17, 3));
        assert!(parse_faults("42 (x) R 1").is_err());
        let proc_stat =
            "cpu  1 2 3 4 5 6 7 8 0 0\ncpu0 1 2 3 4 5 6 7 80 0 0\ncpu1 1 2 3 4 5 6 7 9 0 0\n";
        assert_eq!(parse_steal(proc_stat, 0), Some(80));
        assert_eq!(parse_steal(proc_stat, 1), Some(9));
        assert_eq!(parse_steal(proc_stat, 2), None);
        Ok(())
    }

    #[test]
    fn the_cause_names_what_moved_or_says_unexplained() {
        let spread = Spread {
            n: 100,
            p50: 1_000,
            max: 500_000,
            ..Spread::default()
        };
        let quiet = Window {
            counters: Some(Counters::default()),
            idle_gap_max: 2_000,
            idle_gaps_over_10us: 0,
        };
        assert!(
            quiet.cause(&spread).starts_with("unexplained"),
            "{}",
            quiet.cause(&spread)
        );
        let stalls = Window {
            idle_gap_max: 300_000,
            ..quiet.clone()
        };
        assert!(stalls.cause(&spread).starts_with("explained"));
        let busy = Window {
            counters: Some(Counters {
                minflt: 2,
                irqs: Some(vec![("LOC".to_owned(), 3)]),
                softirqs: Some(vec![]),
                ..Counters::default()
            }),
            ..quiet
        };
        let c = busy.cause(&spread);
        assert!(
            c.starts_with("candidates")
                && c.contains("page faults (minor 2")
                && c.contains("LOC:3"),
            "{c}"
        );
        assert!(!c.contains("softirqs"), "{c}");
        assert!(
            busy.fields().contains("irqs=LOC:3\tsoftirqs=none\t"),
            "{}",
            busy.fields()
        );
        assert!(Window::default().fields().starts_with("idle_gap_max=0"));
    }
}
