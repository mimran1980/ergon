//! Where each thread must run, from its name and its process's environment.
//!
//! | thread | expected |
//! |---|---|
//! | `ll-hot` | exactly the app's hot CPU: `LL_HOT_CPU` (`isolated:0` by default) resolved against `ERGON_ISOLATED_CPUS` |
//! | `sender` or `aeron-md-snd` | exactly `AERON_SENDER_CPU_AFFINITY` (a CPU number once `ergon-cpus` resolved it) |
//! | `receiver` or `aeron-md-rcv` | exactly `AERON_RECEIVER_CPU_AFFINITY` |
//! | anything else | within the helper CPUs: `ERGON_HELPER_CPUS`, else `ERGON_SHARED_CPUS` |
//!
//! "Anything else" is every helper by name (`aeron-client`, `ll-log`,
//! `ll-stats`, `conductor`, `aeron-md-cnd`, `aeron-md-nra`, `dpdk-*`,
//! `md-host-checks`, `eal-intr-thread`, `rte_mp_handle`) and every thread
//! whose name the table does not know: an unknown thread must be on the
//! shared CPUs too. Aeron names the driver's threads by
//! `AERON_THREAD_NAMING`: the classic names or the `aeron-md-*` ones; both
//! map to the same roles.

use std::collections::BTreeMap;
use std::fmt;

use crate::LlError;
use crate::cpus::{CpuList, ErgonEnv, Spec};

/// What a thread is, from its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThreadRole {
    /// The app's busy-spinning thread, `ll-hot`.
    Hot,
    /// The driver's sender agent.
    Sender,
    /// The driver's receiver agent.
    Receiver,
    /// Any other thread: a helper.
    Other,
}

impl ThreadRole {
    /// Whether it busy-spins on an exclusive CPU.
    #[must_use]
    pub const fn is_hot(self) -> bool {
        !matches!(self, Self::Other)
    }
}

/// The role of a thread named `comm`.
#[must_use]
pub fn role_of(comm: &str) -> ThreadRole {
    match comm {
        "ll-hot" => ThreadRole::Hot,
        "sender" | "aeron-md-snd" => ThreadRole::Sender,
        "receiver" | "aeron-md-rcv" => ThreadRole::Receiver,
        _ => ThreadRole::Other,
    }
}

/// Where a thread may be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expected {
    /// Exactly this CPU.
    Exactly(usize),
    /// Any CPU of the shared set.
    Within(CpuList),
}

impl Expected {
    /// Whether an affinity mask is what this asks: exactly the one CPU, or
    /// a non-empty subset of the shared set.
    #[must_use]
    pub fn allows_mask(&self, mask: &CpuList) -> bool {
        match self {
            Self::Exactly(c) => *mask == CpuList::one(*c),
            Self::Within(set) => !mask.is_empty() && mask.is_subset(set),
        }
    }

    /// Whether running on `cpu` is allowed.
    #[must_use]
    pub fn allows_cpu(&self, cpu: usize) -> bool {
        match self {
            Self::Exactly(c) => *c == cpu,
            Self::Within(set) => set.contains(cpu),
        }
    }
}

impl fmt::Display for Expected {
    /// `3` or `shared(1)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exactly(c) => write!(f, "{c}"),
            Self::Within(set) => write!(f, "shared({set})"),
        }
    }
}

/// One process's placement plan, from its environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The plugin's lists.
    pub env: ErgonEnv,
    /// The app's hot CPU, when the process is an app (`LL_ROLE` set).
    pub hot: Option<usize>,
    /// The driver's sender CPU, when `AERON_SENDER_CPU_AFFINITY` is set.
    pub sender: Option<usize>,
    /// The driver's receiver CPU.
    pub receiver: Option<usize>,
}

impl Plan {
    /// From a process's environment (`/proc/<pid>/environ`).
    ///
    /// # Errors
    ///
    /// As [`ErgonEnv::from_lookup`]; [`LlError::BadValue`] when an affinity
    /// variable is not one CPU number or `LL_HOT_CPU` is not a spec.
    pub fn from_env(env: &BTreeMap<String, String>) -> Result<Self, LlError> {
        let ergon = ErgonEnv::from_lookup(|k| env.get(k).cloned())?;
        let cpu = |name: &str| -> Result<Option<usize>, LlError> {
            env.get(name)
                .filter(|v| !v.is_empty())
                .map(|v| {
                    v.parse().map_err(|_| {
                        LlError::bad(
                            name,
                            v.as_str(),
                            "not one CPU number: did ergon-cpus resolve it?",
                        )
                    })
                })
                .transpose()
        };
        let hot = if env.contains_key("LL_ROLE") {
            let spec = match env.get("LL_HOT_CPU").filter(|v| !v.is_empty()) {
                Some(v) => Spec::parse(v)?,
                None => DEFAULT_HOT_SPEC,
            };
            spec.resolve(&ergon)?.nth(0)
        } else {
            None
        };
        Ok(Self {
            env: ergon,
            hot,
            sender: cpu("AERON_SENDER_CPU_AFFINITY")?,
            receiver: cpu("AERON_RECEIVER_CPU_AFFINITY")?,
        })
    }

    /// Where a thread in `role` may be.
    ///
    /// # Errors
    ///
    /// A message when the process plans no CPU for that role (an `ll-hot`
    /// thread in the driver, a `sender` in an app).
    pub fn expect(&self, role: ThreadRole) -> Result<Expected, String> {
        let exactly = |cpu: Option<usize>, what: &str| {
            cpu.map(Expected::Exactly)
                .ok_or_else(|| format!("this process plans no {what} CPU"))
        };
        match role {
            ThreadRole::Hot => exactly(self.hot, "hot (LL_ROLE unset)"),
            ThreadRole::Sender => exactly(self.sender, "sender (AERON_SENDER_CPU_AFFINITY unset)"),
            ThreadRole::Receiver => exactly(
                self.receiver,
                "receiver (AERON_RECEIVER_CPU_AFFINITY unset)",
            ),
            ThreadRole::Other => Ok(Expected::Within(self.env.helpers.clone())),
        }
    }

    /// Every exclusive CPU this process plans a hot thread on.
    #[must_use]
    pub fn hot_cpus(&self) -> CpuList {
        self.hot
            .into_iter()
            .chain(self.sender)
            .chain(self.receiver)
            .collect()
    }
}

/// `LL_HOT_CPU`'s default.
pub const DEFAULT_HOT_SPEC: Spec = Spec::Isolated(0);

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> BTreeMap<String, String> {
        vars.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn names_map_to_roles_under_both_naming_schemes() {
        assert_eq!(role_of("ll-hot"), ThreadRole::Hot);
        assert_eq!(role_of("sender"), ThreadRole::Sender);
        assert_eq!(role_of("aeron-md-snd"), ThreadRole::Sender);
        assert_eq!(role_of("receiver"), ThreadRole::Receiver);
        assert_eq!(role_of("aeron-md-rcv"), ThreadRole::Receiver);
        for other in [
            "conductor",
            "aeron-md-cnd",
            "aeron-md-nra",
            "aeron-client",
            "ll-log",
            "dpdk-housekeep",
            "media_driver",
            "?",
        ] {
            assert_eq!(role_of(other), ThreadRole::Other, "{other}");
        }
    }

    #[test]
    fn an_app_plans_its_hot_cpu() -> Result<(), LlError> {
        let plan = Plan::from_env(&env(&[
            ("ERGON_ISOLATED_CPUS", "4"),
            ("ERGON_SHARED_CPUS", "1"),
            ("LL_ROLE", "ping"),
        ]))?;
        assert_eq!(plan.expect(ThreadRole::Hot), Ok(Expected::Exactly(4)));
        assert_eq!(
            plan.expect(ThreadRole::Other),
            Ok(Expected::Within(CpuList::one(1)))
        );
        assert!(plan.expect(ThreadRole::Sender).is_err());
        assert_eq!(plan.hot_cpus(), CpuList::one(4));
        Ok(())
    }

    #[test]
    fn a_driver_plans_sender_and_receiver() -> Result<(), LlError> {
        let plan = Plan::from_env(&env(&[
            ("ERGON_ISOLATED_CPUS", "2,3"),
            ("ERGON_SHARED_CPUS", "1"),
            ("AERON_SENDER_CPU_AFFINITY", "2"),
            ("AERON_RECEIVER_CPU_AFFINITY", "3"),
        ]))?;
        assert_eq!(plan.expect(ThreadRole::Sender), Ok(Expected::Exactly(2)));
        assert_eq!(plan.expect(ThreadRole::Receiver), Ok(Expected::Exactly(3)));
        assert!(plan.expect(ThreadRole::Hot).is_err());
        assert_eq!(plan.hot_cpus().to_string(), "2-3");
        // ergon-cpus did not run: the spec is still unresolved
        assert!(
            Plan::from_env(&env(&[
                ("ERGON_ISOLATED_CPUS", "2,3"),
                ("ERGON_SHARED_CPUS", "1"),
                ("AERON_SENDER_CPU_AFFINITY", "isolated:0"),
            ]))
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn masks_and_cpus() {
        let hot = Expected::Exactly(3);
        assert!(hot.allows_mask(&CpuList::one(3)));
        assert!(!hot.allows_mask(&[1, 3].into_iter().collect()));
        assert!(hot.allows_cpu(3) && !hot.allows_cpu(1));
        let shared = Expected::Within([0, 1].into_iter().collect());
        assert!(shared.allows_mask(&CpuList::one(1)));
        assert!(shared.allows_mask(&[0, 1].into_iter().collect()));
        assert!(!shared.allows_mask(&[1, 3].into_iter().collect()));
        assert!(!shared.allows_mask(&CpuList::new()));
        assert_eq!(shared.to_string(), "shared(0-1)");
    }
}
