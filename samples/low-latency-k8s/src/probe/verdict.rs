//! The checks: each turns what the probe sampled into PASS, FAIL,
//! `NOT-IN-CONTRACT` or INFO. A check with no sample fails: the probe
//! never passes on missing evidence.
//!
//! | id | contract | PASS when |
//! |---|---|---|
//! | P1 | placement | `ERGON_*` set and disjoint; the container's `cpuset.cpus.effective` is isolated ∪ shared |
//! | P2 | placement | the hot threads exist: `ll-hot` in an app, `sender` and `receiver` in the driver |
//! | P3 | placement | a hot thread's affinity is exactly its CPU; the driver's sender and receiver are on different cores |
//! | P4 | placement | every sampled CPU of a hot thread is its CPU |
//! | P5 | placement | every other thread's affinity is within the shared CPUs |
//! | P6 | placement | every other thread was only ever sampled on a shared CPU |
//! | F0 | full | `/sys/devices/system/cpu/isolated` and `nohz_full` hold every exclusive CPU |
//! | F1 | full | no task but the occupant (kernel threads aside) was sampled on an exclusive CPU |
//! | F2 | full | no device interrupt on an exclusive CPU over the window (the timer row aside) |
//! | F3 | full | the per-CPU timer fires at most `LL_TICK_LIMIT` times a second on an exclusive CPU |
//! | F4 | full | a hot thread made no voluntary and no involuntary context switch over the window |
//! | F5 | full | the container has no CPU quota (`cpu.max` is `max`) and was never throttled |

use std::fmt;

use crate::cpus::CpuList;
use crate::probe::expect::{Expected, ThreadRole};

/// Which checks a run must pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contract {
    /// P1–P6: the threads are where they belong.
    Placement,
    /// P1–P6 and F0–F5: and nothing else runs or interrupts there.
    Full,
}

impl Contract {
    /// `placement` or `full`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "placement" => Some(Self::Placement),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    /// Whether check `id` (`P3`, `F2`) is in this contract.
    #[must_use]
    pub fn covers(self, id: &str) -> bool {
        id.starts_with('P') || (self == Self::Full && id.starts_with('F'))
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Placement => "placement",
            Self::Full => "full",
        })
    }
}

/// A check's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Held.
    Pass,
    /// Did not hold, or had no evidence.
    Fail,
    /// Not in the run's contract: printed with its raw values, not counted.
    NotInContract,
    /// Information only (kernel threads, IPIs, softirqs).
    Info,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::NotInContract => "NOT-IN-CONTRACT",
            Self::Info => "INFO",
        })
    }
}

/// One line of the verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// `pod=trading/ll-ping-x` or `node=<n>`.
    pub scope: String,
    /// `P3.affinity`.
    pub id: String,
    /// `ll-hot/4242`, `cpu=3`, or `-`.
    pub subject: String,
    /// The result.
    pub outcome: Outcome,
    /// The values behind it: `allowed=3 expected=3`.
    pub detail: String,
}

impl Check {
    /// A check whose outcome follows `held` under `contract`: PASS/FAIL when
    /// `contract` covers it, else `NOT-IN-CONTRACT`.
    #[must_use]
    pub fn judged(
        contract: Contract,
        scope: &str,
        id: &str,
        subject: &str,
        held: bool,
        detail: String,
    ) -> Self {
        let outcome = if !contract.covers(id) {
            Outcome::NotInContract
        } else if held {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        Self {
            scope: scope.to_owned(),
            id: id.to_owned(),
            subject: subject.to_owned(),
            outcome,
            detail,
        }
    }
}

/// What the probe saw of one thread over the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadSeries {
    /// The thread's pod (`trading/ll-ping-x`) or process label.
    pub scope: String,
    /// Its id.
    pub tid: u32,
    /// Its name.
    pub comm: String,
    /// What its name makes it.
    pub role: ThreadRole,
    /// Where it may be, or why the process plans no CPU for it.
    pub expected: Result<Expected, String>,
    /// Every distinct affinity mask seen.
    pub masks: Vec<CpuList>,
    /// Every distinct CPU it was sampled on.
    pub ran_on: CpuList,
    /// Samples taken.
    pub samples: u64,
    /// Samples on a CPU it may not use.
    pub bad_samples: u64,
}

impl ThreadSeries {
    /// A series with no samples yet.
    #[must_use]
    pub fn new(
        scope: &str,
        tid: u32,
        comm: &str,
        role: ThreadRole,
        expected: Result<Expected, String>,
    ) -> Self {
        Self {
            scope: scope.to_owned(),
            tid,
            comm: comm.to_owned(),
            role,
            expected,
            masks: Vec::new(),
            ran_on: CpuList::new(),
            samples: 0,
            bad_samples: 0,
        }
    }

    /// Adds one sample: its mask and the CPU it last ran on.
    pub fn sample(&mut self, mask: &CpuList, cpu: usize) {
        self.samples += 1;
        if !self.masks.contains(mask) {
            self.masks.push(mask.clone());
        }
        self.ran_on.insert(cpu);
        if !self.expected.as_ref().is_ok_and(|e| e.allows_cpu(cpu)) {
            self.bad_samples += 1;
        }
    }

    /// `ll-hot/4242`.
    #[must_use]
    pub fn subject(&self) -> String {
        format!("{}/{}", self.comm, self.tid)
    }

    /// P3 (a hot thread) or P5 (any other): its affinity.
    #[must_use]
    pub fn affinity_check(&self, contract: Contract) -> Check {
        let id = if self.role.is_hot() {
            "P3.affinity"
        } else {
            "P5.affinity"
        };
        let allowed: Vec<String> = self.masks.iter().map(ToString::to_string).collect();
        let allowed = if allowed.is_empty() {
            "-".to_owned()
        } else {
            allowed.join("|")
        };
        let (held, detail) = match &self.expected {
            Err(why) => (false, format!("allowed={allowed} expected=none ({why})")),
            Ok(e) => (
                self.samples > 0 && self.masks.iter().all(|m| e.allows_mask(m)),
                format!("allowed={allowed} expected={e}"),
            ),
        };
        Check::judged(contract, &self.scope, id, &self.subject(), held, detail)
    }

    /// P4 (a hot thread) or P6 (any other): where it ran.
    #[must_use]
    pub fn ran_on_check(&self, contract: Contract) -> Check {
        let id = if self.role.is_hot() {
            "P4.ran_on"
        } else {
            "P6.ran_on"
        };
        let ran_on = if self.ran_on.is_empty() {
            "-".to_owned()
        } else {
            self.ran_on.to_string()
        };
        let expected = self
            .expected
            .as_ref()
            .map_or_else(|why| format!("none ({why})"), ToString::to_string);
        let held = self.expected.is_ok() && self.samples > 0 && self.bad_samples == 0;
        let detail = format!(
            "ran_on={ran_on} samples={} bad={} expected={expected}",
            self.samples, self.bad_samples
        );
        Check::judged(contract, &self.scope, id, &self.subject(), held, detail)
    }
}

/// The overall verdict: FAIL when any counted check failed or nothing was
/// checked; the number of counted checks and of failures.
#[must_use]
pub fn result(checks: &[Check]) -> (Outcome, usize, usize) {
    let counted = checks
        .iter()
        .filter(|c| matches!(c.outcome, Outcome::Pass | Outcome::Fail))
        .count();
    let failed = checks.iter().filter(|c| c.outcome == Outcome::Fail).count();
    let outcome = if counted == 0 || failed > 0 {
        Outcome::Fail
    } else {
        Outcome::Pass
    };
    (outcome, counted, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hot() -> ThreadSeries {
        ThreadSeries::new(
            "pod=trading/ll-ping",
            42,
            "ll-hot",
            ThreadRole::Hot,
            Ok(Expected::Exactly(3)),
        )
    }

    fn helper() -> ThreadSeries {
        ThreadSeries::new(
            "pod=trading/ll-ping",
            43,
            "aeron-client",
            ThreadRole::Other,
            Ok(Expected::Within(CpuList::one(1))),
        )
    }

    #[test]
    fn a_pinned_hot_thread_passes() {
        let mut t = hot();
        for _ in 0..5 {
            t.sample(&CpuList::one(3), 3);
        }
        let a = t.affinity_check(Contract::Placement);
        assert_eq!((a.id.as_str(), a.outcome), ("P3.affinity", Outcome::Pass));
        assert_eq!(a.detail, "allowed=3 expected=3");
        assert_eq!(a.subject, "ll-hot/42");
        let r = t.ran_on_check(Contract::Placement);
        assert_eq!((r.id.as_str(), r.outcome), ("P4.ran_on", Outcome::Pass));
        assert_eq!(r.detail, "ran_on=3 samples=5 bad=0 expected=3");
    }

    #[test]
    fn one_sample_off_its_cpu_fails() {
        let mut t = hot();
        t.sample(&CpuList::one(3), 3);
        t.sample(&CpuList::one(3), 1);
        assert_eq!(t.ran_on_check(Contract::Placement).outcome, Outcome::Fail);
        let mut t = hot();
        t.sample(&[1, 3].into_iter().collect(), 3);
        assert_eq!(t.affinity_check(Contract::Placement).outcome, Outcome::Fail);
    }

    #[test]
    fn a_helper_must_stay_on_the_shared_cpus() {
        let mut t = helper();
        t.sample(&CpuList::one(1), 1);
        let a = t.affinity_check(Contract::Placement);
        assert_eq!((a.id.as_str(), a.outcome), ("P5.affinity", Outcome::Pass));
        assert_eq!(a.detail, "allowed=1 expected=shared(1)");
        assert_eq!(t.ran_on_check(Contract::Placement).id, "P6.ran_on");
        t.sample(&CpuList::one(1), 3);
        assert_eq!(t.ran_on_check(Contract::Placement).outcome, Outcome::Fail);
    }

    #[test]
    fn no_samples_or_no_plan_fails() {
        let t = hot();
        assert_eq!(t.affinity_check(Contract::Placement).outcome, Outcome::Fail);
        assert_eq!(t.ran_on_check(Contract::Placement).outcome, Outcome::Fail);
        let mut t = ThreadSeries::new(
            "pod=x",
            1,
            "ll-hot",
            ThreadRole::Hot,
            Err("no hot CPU planned".into()),
        );
        t.sample(&CpuList::one(3), 3);
        let a = t.affinity_check(Contract::Placement);
        assert_eq!(a.outcome, Outcome::Fail);
        assert!(a.detail.contains("no hot CPU planned"), "{}", a.detail);
    }

    #[test]
    fn the_result_counts_only_the_contract() {
        let pass = Check::judged(
            Contract::Placement,
            "node=n",
            "P1.env",
            "-",
            true,
            String::new(),
        );
        let out = Check::judged(
            Contract::Placement,
            "node=n",
            "F2.device_irqs",
            "cpu=3",
            false,
            String::new(),
        );
        assert_eq!(out.outcome, Outcome::NotInContract);
        assert_eq!(result(&[pass.clone(), out.clone()]), (Outcome::Pass, 1, 0));
        let fail = Check::judged(
            Contract::Full,
            "node=n",
            "F2.device_irqs",
            "cpu=3",
            false,
            String::new(),
        );
        assert_eq!(result(&[pass, out, fail]), (Outcome::Fail, 2, 1));
        assert_eq!(result(&[]), (Outcome::Fail, 0, 0));
    }

    proptest::proptest! {
        #[test]
        fn any_sample_off_the_set_fails(cpus in proptest::collection::vec(0_usize..8, 1..50)) {
            let mut t = helper();
            for &c in &cpus {
                t.sample(&CpuList::one(1), c);
            }
            let held = cpus.iter().all(|&c| c == 1);
            proptest::prop_assert_eq!(t.ran_on_check(Contract::Placement).outcome == Outcome::Pass, held);
        }
    }
}
