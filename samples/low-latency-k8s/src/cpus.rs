//! CPU lists, the plugin's environment, and pinning.
//!
//! The `ergon-shared-cpus` plugin puts every thread of a new container on
//! the shared CPUs and names the sets in the environment:
//!
//! | variable | holds |
//! |---|---|
//! | `ERGON_ISOLATED_CPUS` | the pod's exclusive CPUs, sorted: `3` |
//! | `ERGON_SHARED_CPUS` | this container's shared CPUs: `1` |
//! | `ERGON_HELPER_CPUS` | the same as `ERGON_SHARED_CPUS` under the plugin |
//!
//! A thread inherits its creator's mask, so the app creates every helper
//! before the hot thread pins itself, and pins by index ([`Spec`]):
//! `isolated:0` is the first exclusive CPU. Helpers read
//! `ERGON_HELPER_CPUS` first and fall back to `ERGON_SHARED_CPUS`, as
//! `ergon-io` does (`io/src/placement.rs`).

use std::collections::BTreeSet;
use std::fmt;

use crate::LlError;

/// A set of CPUs in the kernel's list format (`0-3,7`), ordered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CpuList(BTreeSet<usize>);

impl CpuList {
    /// No CPUs.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    /// Parses `0-3,7` (spaces and a trailing newline allowed; empty is the
    /// empty set).
    ///
    /// # Errors
    ///
    /// [`LlError::CpuList`] for anything else, a reversed range included.
    pub fn parse(text: &str) -> Result<Self, LlError> {
        let bad = |why| LlError::CpuList {
            text: text.to_owned(),
            why,
        };
        let number = |s: &str| -> Result<usize, LlError> {
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad("not a CPU number"));
            }
            s.parse().map_err(|_| bad("CPU number too large"))
        };
        let trimmed = text.trim();
        let mut set = BTreeSet::new();
        if trimmed.is_empty() {
            return Ok(Self(set));
        }
        for part in trimmed.split(',') {
            let part = part.trim();
            match part.split_once('-') {
                None => {
                    set.insert(number(part)?);
                }
                Some((lo, hi)) => {
                    let (lo, hi) = (number(lo)?, number(hi)?);
                    if lo > hi {
                        return Err(bad("a range runs backwards"));
                    }
                    set.extend(lo..=hi);
                }
            }
        }
        Ok(Self(set))
    }

    /// The set holding `cpu` alone.
    #[must_use]
    pub fn one(cpu: usize) -> Self {
        Self(BTreeSet::from([cpu]))
    }

    /// Whether `cpu` is in the set.
    #[must_use]
    pub fn contains(&self, cpu: usize) -> bool {
        self.0.contains(&cpu)
    }

    /// Whether every CPU of this set is in `other`.
    #[must_use]
    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    /// Whether the two share no CPU.
    #[must_use]
    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.0.is_disjoint(&other.0)
    }

    /// Both sets' CPUs.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0).copied().collect())
    }

    /// The `i`-th CPU in ascending order.
    #[must_use]
    pub fn nth(&self, i: usize) -> Option<usize> {
        self.0.iter().nth(i).copied()
    }

    /// How many CPUs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The CPUs, ascending.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().copied()
    }

    /// Adds `cpu`.
    pub fn insert(&mut self, cpu: usize) {
        self.0.insert(cpu);
    }
}

impl FromIterator<usize> for CpuList {
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl fmt::Display for CpuList {
    /// The kernel's compact form: `0-3,7`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut iter = self.0.iter().copied().peekable();
        while let Some(lo) = iter.next() {
            let mut hi = lo;
            while iter.peek() == Some(&(hi + 1)) {
                hi += 1;
                iter.next();
            }
            if !first {
                f.write_str(",")?;
            }
            first = false;
            if lo == hi {
                write!(f, "{lo}")?;
            } else {
                write!(f, "{lo}-{hi}")?;
            }
        }
        Ok(())
    }
}

/// An `ergon-cpus` spec: which plugin CPU a thread takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spec {
    /// `isolated:<i>`: the i-th CPU of `ERGON_ISOLATED_CPUS`.
    Isolated(usize),
    /// `shared:<i>`: the i-th CPU of `ERGON_SHARED_CPUS`.
    Shared(usize),
    /// `isolated`: the whole exclusive list.
    AllIsolated,
    /// `shared`: the whole shared list.
    AllShared,
}

impl Spec {
    /// Parses `isolated:<i>`, `shared:<i>`, `isolated` or `shared`.
    ///
    /// # Errors
    ///
    /// [`LlError::BadValue`] for anything else.
    pub fn parse(text: &str) -> Result<Self, LlError> {
        let bad = || LlError::bad("spec", text, "isolated:<i>, shared:<i>, isolated or shared");
        let index = |s: &str| -> Result<usize, LlError> {
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            s.parse().map_err(|_| bad())
        };
        match text.split_once(':') {
            None if text == "isolated" => Ok(Self::AllIsolated),
            None if text == "shared" => Ok(Self::AllShared),
            Some(("isolated", i)) => Ok(Self::Isolated(index(i)?)),
            Some(("shared", i)) => Ok(Self::Shared(index(i)?)),
            _ => Err(bad()),
        }
    }

    /// The CPUs this spec names under `env`.
    ///
    /// # Errors
    ///
    /// [`LlError::NoSuchIndex`] when the index is past the list's end.
    pub fn resolve(self, env: &ErgonEnv) -> Result<CpuList, LlError> {
        let nth = |list: &CpuList, i: usize| {
            list.nth(i)
                .map(CpuList::one)
                .ok_or_else(|| LlError::NoSuchIndex {
                    spec: self.to_string(),
                    len: list.len(),
                })
        };
        match self {
            Self::Isolated(i) => nth(&env.isolated, i),
            Self::Shared(i) => nth(&env.shared, i),
            Self::AllIsolated => Ok(env.isolated.clone()),
            Self::AllShared => Ok(env.shared.clone()),
        }
    }
}

impl fmt::Display for Spec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Isolated(i) => write!(f, "isolated:{i}"),
            Self::Shared(i) => write!(f, "shared:{i}"),
            Self::AllIsolated => f.write_str("isolated"),
            Self::AllShared => f.write_str("shared"),
        }
    }
}

/// The plugin's environment, read once at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErgonEnv {
    /// `ERGON_ISOLATED_CPUS`.
    pub isolated: CpuList,
    /// `ERGON_SHARED_CPUS`.
    pub shared: CpuList,
    /// `ERGON_HELPER_CPUS`, else `ERGON_SHARED_CPUS`: where helpers go.
    pub helpers: CpuList,
}

impl ErgonEnv {
    /// Reads the variables through `lookup` (`std::env::var(..).ok()` in the
    /// app, a map in tests).
    ///
    /// # Errors
    ///
    /// [`LlError::MissingEnv`] when `ERGON_ISOLATED_CPUS` or
    /// `ERGON_SHARED_CPUS` is unset or empty, [`LlError::CpuList`] when one
    /// does not parse, and [`LlError::BadValue`] when the exclusive and the
    /// helper CPUs overlap.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, LlError> {
        let required = |name: &'static str| -> Result<CpuList, LlError> {
            let list = CpuList::parse(&lookup(name).ok_or(LlError::MissingEnv(name))?)?;
            if list.is_empty() {
                return Err(LlError::MissingEnv(name));
            }
            Ok(list)
        };
        let isolated = required("ERGON_ISOLATED_CPUS")?;
        let shared = required("ERGON_SHARED_CPUS")?;
        let helpers = match lookup("ERGON_HELPER_CPUS").filter(|v| !v.trim().is_empty()) {
            Some(v) => CpuList::parse(&v)?,
            None => shared.clone(),
        };
        if !isolated.is_disjoint(&helpers) || !isolated.is_disjoint(&shared) {
            return Err(LlError::bad(
                "ERGON_ISOLATED_CPUS",
                isolated.to_string(),
                format!("overlaps the shared CPUs {shared} (helpers {helpers})"),
            ));
        }
        Ok(Self {
            isolated,
            shared,
            helpers,
        })
    }

    /// From the process environment.
    ///
    /// # Errors
    ///
    /// As [`ErgonEnv::from_lookup`].
    pub fn from_process() -> Result<Self, LlError> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

/// Pins the calling thread to `cpus`.
///
/// # Errors
///
/// [`LlError::Io`] when the kernel refuses (a CPU outside the cpuset), and
/// [`LlError::Unsupported`] off Linux.
pub fn pin_current(cpus: &CpuList) -> Result<(), LlError> {
    imp::set_affinity(None, cpus)
}

/// Pins thread `tid` of this process to `cpus`.
///
/// # Errors
///
/// As [`pin_current`].
pub fn pin_thread(tid: u32, cpus: &CpuList) -> Result<(), LlError> {
    imp::set_affinity(Some(tid), cpus)
}

/// The calling thread's affinity.
///
/// # Errors
///
/// As [`pin_current`].
pub fn current_affinity() -> Result<CpuList, LlError> {
    imp::get_affinity(None)
}

/// The affinity of thread `tid`.
///
/// # Errors
///
/// As [`pin_current`].
pub fn thread_affinity(tid: u32) -> Result<CpuList, LlError> {
    imp::get_affinity(Some(tid))
}

/// The CPU the calling thread runs on now (`sched_getcpu`).
///
/// # Errors
///
/// [`LlError::Unsupported`] off Linux.
pub fn current_cpu() -> Result<usize, LlError> {
    imp::current_cpu()
}

/// The calling thread's kernel thread id.
///
/// # Errors
///
/// [`LlError::Unsupported`] off Linux.
pub fn current_tid() -> Result<u32, LlError> {
    imp::current_tid()
}

/// Pins the calling thread to `cpus` and checks with `sched_getcpu()` that
/// it now runs there: the CPU it runs on.
///
/// # Errors
///
/// As [`pin_current`], and [`LlError::NotOnCpu`] when the thread runs
/// elsewhere after the pin.
pub fn pin_and_check(cpus: &CpuList) -> Result<usize, LlError> {
    pin_current(cpus)?;
    let cpu = current_cpu()?;
    if cpus.contains(cpu) {
        Ok(cpu)
    } else {
        Err(LlError::NotOnCpu {
            expected: cpus.nth(0).unwrap_or(usize::MAX),
            got: cpu,
        })
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::CpuList;
    use crate::LlError;
    use rustix::process::Pid;
    use rustix::thread::{CpuSet, sched_getaffinity, sched_getcpu, sched_setaffinity};

    fn pid(tid: Option<u32>) -> Result<Option<Pid>, LlError> {
        match tid {
            None => Ok(None),
            Some(t) => {
                let raw = i32::try_from(t)
                    .map_err(|_| LlError::bad("tid", t.to_string(), "out of range"))?;
                Pid::from_raw(raw)
                    .map(Some)
                    .ok_or_else(|| LlError::bad("tid", t.to_string(), "not a thread id"))
            }
        }
    }

    pub fn set_affinity(tid: Option<u32>, cpus: &CpuList) -> Result<(), LlError> {
        let mut set = CpuSet::new();
        for cpu in cpus.iter() {
            if cpu >= CpuSet::MAX_CPU {
                return Err(LlError::bad("cpu", cpu.to_string(), "past CPU_SETSIZE"));
            }
            set.set(cpu);
        }
        sched_setaffinity(pid(tid)?, &set)
            .map_err(|e| LlError::io(format!("sched_setaffinity({cpus})"), e.into()))
    }

    pub fn get_affinity(tid: Option<u32>) -> Result<CpuList, LlError> {
        let set =
            sched_getaffinity(pid(tid)?).map_err(|e| LlError::io("sched_getaffinity", e.into()))?;
        Ok((0..CpuSet::MAX_CPU).filter(|&c| set.is_set(c)).collect())
    }

    #[allow(clippy::unnecessary_wraps)] // the same signature as the other systems'
    pub fn current_cpu() -> Result<usize, LlError> {
        Ok(sched_getcpu())
    }

    pub fn current_tid() -> Result<u32, LlError> {
        let raw = rustix::thread::gettid().as_raw_nonzero().get();
        u32::try_from(raw).map_err(|_| LlError::bad("tid", raw.to_string(), "negative"))
    }
}

#[cfg(not(target_os = "linux"))]
#[allow(clippy::missing_const_for_fn)] // the Linux versions make system calls
mod imp {
    use super::CpuList;
    use crate::LlError;

    pub fn set_affinity(_tid: Option<u32>, _cpus: &CpuList) -> Result<(), LlError> {
        Err(LlError::Unsupported("sched_setaffinity"))
    }

    pub fn get_affinity(_tid: Option<u32>) -> Result<CpuList, LlError> {
        Err(LlError::Unsupported("sched_getaffinity"))
    }

    pub fn current_cpu() -> Result<usize, LlError> {
        Err(LlError::Unsupported("sched_getcpu"))
    }

    pub fn current_tid() -> Result<u32, LlError> {
        Err(LlError::Unsupported("gettid"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn list(text: &str) -> CpuList {
        CpuList::parse(text).unwrap_or_default()
    }

    #[test]
    fn parses_the_kernel_format() -> Result<(), LlError> {
        assert_eq!(
            CpuList::parse("0-3,7")?,
            [0, 1, 2, 3, 7].into_iter().collect()
        );
        assert_eq!(CpuList::parse(" 5 \n")?, CpuList::one(5));
        assert_eq!(CpuList::parse("")?, CpuList::new());
        assert_eq!(CpuList::parse("2,1,2")?, [1, 2].into_iter().collect());
        Ok(())
    }

    #[test]
    fn refuses_what_is_not_a_list() {
        for bad in ["a", "3-1", "1,,2", "1-", "-1", "1-2-3", "0x1"] {
            assert!(CpuList::parse(bad).is_err(), "{bad:?} parsed");
        }
    }

    #[test]
    fn formats_compact_ranges() {
        assert_eq!(list("0,1,2,3,7").to_string(), "0-3,7");
        assert_eq!(list("4").to_string(), "4");
        assert_eq!(list("1,3,5-6").to_string(), "1,3,5-6");
        assert_eq!(CpuList::new().to_string(), "");
    }

    #[test]
    fn parses_specs() -> Result<(), LlError> {
        assert_eq!(Spec::parse("isolated:0")?, Spec::Isolated(0));
        assert_eq!(Spec::parse("shared:2")?, Spec::Shared(2));
        assert_eq!(Spec::parse("isolated")?, Spec::AllIsolated);
        assert_eq!(Spec::parse("shared")?, Spec::AllShared);
        for bad in ["isolated:", "isolated:x", "core:1", "", "shared:-1"] {
            assert!(Spec::parse(bad).is_err(), "{bad:?} parsed");
        }
        Ok(())
    }

    fn env(vars: &[(&str, &str)]) -> Result<ErgonEnv, LlError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        ErgonEnv::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn reads_the_plugin_environment() -> Result<(), LlError> {
        let e = env(&[("ERGON_ISOLATED_CPUS", "3,5"), ("ERGON_SHARED_CPUS", "1")])?;
        assert_eq!(e.isolated, list("3,5"));
        assert_eq!(e.helpers, list("1"));
        let e = env(&[
            ("ERGON_ISOLATED_CPUS", "3"),
            ("ERGON_SHARED_CPUS", "1"),
            ("ERGON_HELPER_CPUS", "0-1"),
        ])?;
        assert_eq!(e.helpers, list("0-1"));
        assert_eq!(
            Spec::Isolated(1).resolve(&env(&[
                ("ERGON_ISOLATED_CPUS", "3,5"),
                ("ERGON_SHARED_CPUS", "1")
            ])?)?,
            list("5")
        );
        assert_eq!(Spec::AllShared.resolve(&e)?, list("1"));
        assert!(matches!(
            Spec::Isolated(1).resolve(&e),
            Err(LlError::NoSuchIndex { len: 1, .. })
        ));
        Ok(())
    }

    #[test]
    fn refuses_a_missing_or_overlapping_environment() {
        assert!(matches!(
            env(&[("ERGON_SHARED_CPUS", "1")]),
            Err(LlError::MissingEnv("ERGON_ISOLATED_CPUS"))
        ));
        assert!(matches!(
            env(&[("ERGON_ISOLATED_CPUS", "3")]),
            Err(LlError::MissingEnv("ERGON_SHARED_CPUS"))
        ));
        assert!(matches!(
            env(&[("ERGON_ISOLATED_CPUS", ""), ("ERGON_SHARED_CPUS", "1")]),
            Err(LlError::MissingEnv("ERGON_ISOLATED_CPUS"))
        ));
        assert!(matches!(
            env(&[("ERGON_ISOLATED_CPUS", "1-3"), ("ERGON_SHARED_CPUS", "1")]),
            Err(LlError::BadValue { .. })
        ));
    }

    proptest::proptest! {
        #[test]
        fn cpu_lists_round_trip(cpus in proptest::collection::btree_set(0_usize..1024, 0..64)) {
            let set = CpuList(cpus);
            proptest::prop_assert_eq!(CpuList::parse(&set.to_string()).ok(), Some(set));
        }
    }
}
