//! The app's own placement check, run by `main` once `ll-hot` is pinned:
//! the P3–P6 rules of [`crate::probe::verdict`] over `/proc/self/task/*`.
//!
//! ```text
//! ll-selfcheck PASS thread=ll-hot tid=4242 allowed=3 ran_on=3 expected=3
//! ll-selfcheck PASS thread=aeron-client tid=4245 allowed=1 ran_on=1 expected=shared(1)
//! ll-selfcheck RESULT PASS threads=6 failed=0
//! ```
//!
//! A thread the app must have (`ll-hot`, `aeron-client`, `ll-log`,
//! `ll-stats`) that is missing is a FAIL. Any FAIL stops the app before it
//! measures (exit 2). This proves where the threads are at start;
//! `ll-stats` re-checks the helpers each second and `ll-probe` samples the
//! whole window from outside.

use crate::LlError;
use crate::cpus::CpuList;
use crate::probe::expect::{Plan, role_of};
use crate::probe::verdict::{Contract, Outcome, ThreadSeries};

/// The threads every app has once `ll-hot` is pinned.
pub const REQUIRED: [&str; 4] = ["ll-hot", "aeron-client", "ll-log", "ll-stats"];

/// One sample of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observed {
    /// Its id.
    pub tid: u32,
    /// Its name.
    pub comm: String,
    /// Its affinity.
    pub mask: CpuList,
    /// The CPU it last ran on.
    pub cpu: usize,
}

/// The check's lines and how many failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfCheck {
    /// The lines, without line ends.
    pub lines: Vec<String>,
    /// Failed checks.
    pub failed: usize,
}

/// Judges `samples` (any number per thread) against `plan`.
#[must_use]
pub fn judge(plan: &Plan, samples: &[Observed], required: &[&str]) -> SelfCheck {
    let mut series: Vec<ThreadSeries> = Vec::new();
    for s in samples {
        let i = series
            .iter()
            .position(|t| t.tid == s.tid)
            .unwrap_or_else(|| {
                let role = role_of(&s.comm);
                series.push(ThreadSeries::new(
                    "self",
                    s.tid,
                    &s.comm,
                    role,
                    plan.expect(role),
                ));
                series.len() - 1
            });
        series[i].sample(&s.mask, s.cpu);
    }
    let mut lines = Vec::new();
    let mut failed = 0;
    for t in &series {
        let a = t.affinity_check(Contract::Placement);
        let r = t.ran_on_check(Contract::Placement);
        let ok = a.outcome == Outcome::Pass && r.outcome == Outcome::Pass;
        if !ok {
            failed += 1;
        }
        let allowed: Vec<String> = t.masks.iter().map(ToString::to_string).collect();
        let expected = t
            .expected
            .as_ref()
            .map_or_else(|why| format!("none ({why})"), ToString::to_string);
        lines.push(format!(
            "ll-selfcheck {} thread={} tid={} allowed={} ran_on={} expected={expected}",
            if ok { "PASS" } else { "FAIL" },
            t.comm,
            t.tid,
            allowed.join("|"),
            t.ran_on
        ));
    }
    for name in required {
        if !series.iter().any(|t| t.comm == *name) {
            failed += 1;
            lines.push(format!("ll-selfcheck FAIL thread={name} tid=- missing"));
        }
    }
    lines.push(format!(
        "ll-selfcheck RESULT {} threads={} failed={failed}",
        if failed == 0 { "PASS" } else { "FAIL" },
        series.len()
    ));
    SelfCheck { lines, failed }
}

/// Samples every thread of this process `rounds` times, `gap` apart.
///
/// # Errors
///
/// [`LlError::Io`] when `/proc/self/task` cannot be read (not Linux).
pub fn observe(rounds: usize, gap: std::time::Duration) -> Result<Vec<Observed>, LlError> {
    let root = crate::probe::proc::Root::system();
    let mut out = Vec::new();
    for round in 0..rounds {
        if round > 0 {
            std::thread::sleep(gap);
        }
        for tid in root.numeric_entries("proc/self/task")? {
            let base = format!("proc/self/task/{tid}");
            // a thread that exited between the listing and the read is gone
            let (Ok(stat), Ok(status)) = (
                root.read(&format!("{base}/stat")),
                root.read(&format!("{base}/status")),
            ) else {
                continue;
            };
            let stat = crate::probe::proc::parse_stat(&stat)?;
            let status = crate::probe::proc::parse_status(&status)?;
            out.push(Observed {
                tid,
                comm: stat.comm,
                mask: status.cpus_allowed,
                cpu: stat.processor,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn plan() -> Result<Plan, LlError> {
        let env: BTreeMap<String, String> = [
            ("ERGON_ISOLATED_CPUS", "3"),
            ("ERGON_SHARED_CPUS", "1"),
            ("LL_ROLE", "ping"),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
        Plan::from_env(&env)
    }

    fn obs(tid: u32, comm: &str, mask: &str, cpu: usize) -> Result<Observed, LlError> {
        Ok(Observed {
            tid,
            comm: comm.to_owned(),
            mask: CpuList::parse(mask)?,
            cpu,
        })
    }

    #[test]
    fn a_good_layout_passes() -> Result<(), LlError> {
        let samples = vec![
            obs(1, "ll-app", "1", 1)?,
            obs(2, "ll-log", "1", 1)?,
            obs(3, "ll-stats", "1", 1)?,
            obs(4, "ll-hot", "3", 3)?,
            obs(5, "aeron-client", "1", 1)?,
            obs(4, "ll-hot", "3", 3)?,
        ];
        let c = judge(&plan()?, &samples, &REQUIRED);
        assert_eq!(c.failed, 0, "{:?}", c.lines);
        assert!(c.lines.contains(&"ll-selfcheck PASS thread=aeron-client tid=5 allowed=1 ran_on=1 expected=shared(1)".to_owned()));
        assert!(c.lines.contains(
            &"ll-selfcheck PASS thread=ll-hot tid=4 allowed=3 ran_on=3 expected=3".to_owned()
        ));
        assert_eq!(
            c.lines.last().map(String::as_str),
            Some("ll-selfcheck RESULT PASS threads=5 failed=0")
        );
        Ok(())
    }

    #[test]
    fn a_conductor_on_the_hot_cpu_or_a_missing_thread_fails() -> Result<(), LlError> {
        let samples = vec![
            obs(2, "ll-log", "1", 1)?,
            obs(3, "ll-stats", "1", 1)?,
            obs(4, "ll-hot", "3", 3)?,
            // created by the hot thread after it pinned: inherits the hot mask
            obs(5, "aeron-client", "3", 3)?,
        ];
        let c = judge(&plan()?, &samples, &REQUIRED);
        assert_eq!(c.failed, 1, "{:?}", c.lines);
        let mut missing = samples;
        missing.retain(|o| o.comm != "ll-stats");
        let c = judge(&plan()?, &missing, &REQUIRED);
        assert!(
            c.lines
                .iter()
                .any(|l| l == "ll-selfcheck FAIL thread=ll-stats tid=- missing"),
            "{:?}",
            c.lines
        );
        Ok(())
    }
}
