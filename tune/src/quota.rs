//! T9 (a CPU quota on the process's cgroup) and [`ThrottleWatch`], its
//! throttling counters read again while the loop runs.
//!
//! A CPU quota (Linux's CFS bandwidth control: `cpu.max` on cgroup v2,
//! `cpu.cfs_quota_us` on v1) lets a cgroup's threads run for at most the
//! quota in each period. When they have used it, the kernel stops all of
//! them until the period ends. A busy-spinning thread uses its CPU all the
//! time, so it runs at that cap.

use std::fmt::Write as _;
use std::fs::File;

use crate::finding::{CheckId, Finding, Severity};
use crate::host::{Host, ReadError};

/// The cgroup hierarchy the CPU controller is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CgroupVersion {
    /// cgroup v1: `cpu.cfs_quota_us` and `cpu.cfs_period_us`.
    V1,
    /// cgroup v2: `cpu.max`.
    V2,
}

/// One cgroup's CPU quota (its CFS bandwidth limit).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuQuota {
    /// The cgroup's directory on the host.
    pub dir: String,
    /// The CPU time its threads may use in each period, in µs. `None`: no
    /// limit (`max` on v2, `-1` on v1).
    pub quota_us: Option<u64>,
    /// The period, in µs.
    pub period_us: u64,
}

/// This process's CPU cgroup, as far as it can be seen from here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuCgroup {
    /// v1 or v2.
    pub version: CgroupVersion,
    /// The cgroup's directory on the host. In a container with its own
    /// cgroup namespace this is the mount point itself (`/sys/fs/cgroup`):
    /// the container's cgroup, its pod's above it out of sight.
    pub dir: String,
    /// The quota of the cgroup and of each parent visible here, nearest
    /// first: those where the cpu controller is on (the file is there).
    pub limits: Vec<CpuQuota>,
}

const PROC_CGROUP: &str = "/proc/self/cgroup";
const V2_MOUNT: &str = "/sys/fs/cgroup";
const V1_MOUNTS: [&str; 3] = [
    "/sys/fs/cgroup/cpu",
    "/sys/fs/cgroup/cpu,cpuacct",
    "/sys/fs/cgroup/cpuacct,cpu",
];

/// The CPU controller's line of `/proc/self/cgroup`: a v1 hierarchy that
/// has `cpu`, else the unified (v2) one, `0::<path>`.
fn own_line(text: &str) -> Option<(CgroupVersion, &str)> {
    let mut unified = None;
    for line in text.lines() {
        let mut parts = line.splitn(3, ':');
        let (Some(_), Some(controllers), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if controllers.split(',').any(|c| c == "cpu") {
            return Some((CgroupVersion::V1, path));
        }
        if controllers.is_empty() {
            unified = Some((CgroupVersion::V2, path));
        }
    }
    unified
}

/// The process's CPU cgroup directory and each parent visible here, nearest
/// first, ending at the mount point.
///
/// The path is taken under the mount point when that directory exists.
/// Otherwise (a container: runc mounts the container's own directory there,
/// or a cgroup namespace shows a path from outside it) the mount point alone.
fn locate(host: &Host) -> Result<(CgroupVersion, Vec<String>), ReadError> {
    let text = host.read(PROC_CGROUP)?;
    let (version, path) = own_line(&text).ok_or_else(|| ReadError::Malformed {
        path: PROC_CGROUP.to_owned(),
        detail: "no cpu or unified (0::) cgroup line",
    })?;
    let mount = match version {
        CgroupVersion::V2 => Some(V2_MOUNT),
        CgroupVersion::V1 => V1_MOUNTS.into_iter().find(|m| host.is_dir(m)),
    }
    .filter(|m| host.is_dir(m))
    .ok_or_else(|| ReadError::Io {
        path: match version {
            CgroupVersion::V2 => V2_MOUNT.to_owned(),
            CgroupVersion::V1 => V1_MOUNTS[0].to_owned(),
        },
        kind: std::io::ErrorKind::NotFound,
    })?;
    let parts: Vec<&str> = path
        .trim_end_matches(" (deleted)")
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let under = |n: usize| {
        let mut d = mount.to_owned();
        for p in &parts[..n] {
            d.push('/');
            d.push_str(p);
        }
        d
    };
    let visible = !parts.contains(&"..") && host.is_dir(&under(parts.len()));
    let dirs = if visible {
        (0..=parts.len()).rev().map(under).collect()
    } else {
        vec![mount.to_owned()]
    };
    Ok((version, dirs))
}

/// `dir`'s quota, or `None` when the cpu controller is not on there.
fn quota_in(host: &Host, version: CgroupVersion, dir: &str) -> Result<Option<CpuQuota>, ReadError> {
    let malformed = |path: String, detail| ReadError::Malformed { path, detail };
    match version {
        CgroupVersion::V2 => {
            let path = format!("{dir}/cpu.max");
            let Some(text) = host.read_optional(&path)? else {
                return Ok(None);
            };
            let mut words = text.split_whitespace();
            let quota_us = match words.next() {
                Some("max") => None,
                Some(q) => Some(
                    q.parse()
                        .map_err(|_| malformed(path.clone(), "a quota neither max nor a number"))?,
                ),
                None => return Err(malformed(path, "empty")),
            };
            let period_us = words
                .next()
                .and_then(|p| p.parse().ok())
                .filter(|p| *p > 0)
                .ok_or_else(|| malformed(path.clone(), "no period"))?;
            Ok(Some(CpuQuota {
                dir: dir.to_owned(),
                quota_us,
                period_us,
            }))
        }
        CgroupVersion::V1 => {
            let path = format!("{dir}/cpu.cfs_quota_us");
            let Some(text) = host.read_optional(&path)? else {
                return Ok(None);
            };
            let quota: i64 = crate::host::first_line(&text)
                .parse()
                .map_err(|_| malformed(path, "not a number"))?;
            let period_path = format!("{dir}/cpu.cfs_period_us");
            let period_us = u64::try_from(host.read_i64(&period_path)?)
                .ok()
                .filter(|p| *p > 0)
                .ok_or_else(|| malformed(period_path, "not a positive number"))?;
            Ok(Some(CpuQuota {
                dir: dir.to_owned(),
                quota_us: u64::try_from(quota).ok(),
                period_us,
            }))
        }
    }
}

/// This process's CPU cgroup and the quotas on it and its parents visible
/// here.
///
/// # Errors
/// `/proc/self/cgroup` or a quota file cannot be read or does not parse, or
/// the cgroup filesystem is not mounted.
pub fn cpu_cgroup(host: &Host) -> Result<CpuCgroup, ReadError> {
    let (version, dirs) = locate(host)?;
    let mut limits = Vec::new();
    for d in &dirs {
        if let Some(q) = quota_in(host, version, d)? {
            limits.push(q);
        }
    }
    let dir = dirs.into_iter().next().unwrap_or_default();
    Ok(CpuCgroup {
        version,
        dir,
        limits,
    })
}

const T9_EVIDENCE: &str = "Linux Documentation/scheduler/sched-bwc.rst and \
     admin-guide/cgroup-v2.rst (cpu.max, cpu.stat); Kubernetes PR #127525 \
     (DisableCPUQuotaWithExclusiveCPUs, kubelet 1.33 and later, on by default); not measured here";

/// The short fix, for a finding while the loop runs. Never the node-wide
/// `cpuCFSQuota: false`: that lifts every pod's CPU limit on the node.
const FIX_SHORT: &str = "Fix: remove the quota: on Kubernetes 1.33 or later, the static CPU \
     manager with a Guaranteed pod and a whole-number CPU request, so the kubelet drops the \
     quota itself. Never set kubelet `cpuCFSQuota: false`: it removes every pod's CPU limit \
     on the node";

/// `us` µs, as `12.5 ms`, `4 ms` or `800 µs`.
fn ms(us: u64) -> String {
    if us < 1000 {
        format!("{us} µs")
    } else if us % 1000 < 100 {
        format!("{} ms", us / 1000)
    } else {
        format!("{}.{} ms", us / 1000, us % 1000 / 100)
    }
}

/// `quota / period` CPUs: `0.5 CPU`, `1.5 CPUs`, `2 CPUs`.
fn cpus(quota_us: u64, period_us: u64) -> String {
    let hundredths = quota_us.saturating_mul(100) / period_us.max(1);
    let (whole, frac) = (hundredths / 100, hundredths % 100);
    let n = if frac == 0 {
        whole.to_string()
    } else if frac % 10 == 0 {
        format!("{whole}.{}", frac / 10)
    } else {
        format!("{whole}.{frac:02}")
    };
    format!("{n} CPU{}", if hundredths > 100 { "s" } else { "" })
}

/// What the quota file holds, as `<dir>/cpu.max is <its text>` (or v1's
/// two files).
fn quota_text(version: CgroupVersion, q: &CpuQuota) -> String {
    let quota = q
        .quota_us
        .map_or_else(|| "max".to_owned(), |v| v.to_string());
    match version {
        CgroupVersion::V2 => format!("{}/cpu.max is `{quota} {}`", q.dir, q.period_us),
        CgroupVersion::V1 => format!(
            "{}/cpu.cfs_quota_us is `{}` per {} µs (cpu.cfs_period_us)",
            q.dir,
            q.quota_us
                .map_or_else(|| "-1".to_owned(), |v| v.to_string()),
            q.period_us
        ),
    }
}

/// ` (nr_throttled 12 of nr_periods 5000 so far)`, from `dir`'s `cpu.stat`.
fn so_far(host: &Host, version: CgroupVersion, dir: &str) -> String {
    let stat = host
        .read(&format!("{dir}/cpu.stat"))
        .ok()
        .and_then(|t| parse_stat(&t, version).ok());
    match stat {
        Some(Throttling {
            periods: Some(p),
            throttled: Some(t),
            ..
        }) => format!(" (nr_throttled {t} of nr_periods {p} so far)"),
        _ => String::new(),
    }
}

/// T9: no CPU quota on this process's cgroup, nor on a parent visible here.
///
/// A busy-spinning thread uses its CPU all the time, so it runs at the
/// quota, and once the cgroup's threads have used it the kernel stops them
/// all until the period ends. The finding names the quota, what it costs,
/// and the fix: on Kubernetes, the kubelet settings that remove it. A pod's
/// cgroup above a container's own cgroup namespace cannot be seen:
/// [`ThrottleWatch`] reads its effect in `cpu.stat.local`.
#[must_use]
pub fn t9_cpu_quota(host: &Host) -> Finding {
    let id = CheckId::T9;
    let sev = Severity::Warn;
    let cg = match cpu_cgroup(host) {
        Ok(cg) => cg,
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                T9_EVIDENCE,
                "cannot tell whether this process's cgroup has a CPU quota".to_owned(),
                e,
            );
        }
    };
    let mut quotas = cg.limits.iter().filter_map(|q| q.quota_us.map(|v| (q, v)));
    let Some((q, quota)) = quotas.next() else {
        if cg.limits.is_empty() {
            return Finding::ok(
                id,
                sev,
                T9_EVIDENCE,
                format!(
                    "no CPU quota: the cpu controller is not on for this process's cgroup ({}) \
                     or a parent visible here",
                    cg.dir
                ),
            );
        }
        let files: Vec<String> = cg
            .limits
            .iter()
            .map(|q| quota_text(cg.version, q))
            .collect();
        let hidden = if cg.version == CgroupVersion::V2 && cg.dir == V2_MOUNT {
            "; a parent cgroup (a pod's) cannot be seen from here: its throttling shows in \
             cpu.stat.local"
        } else {
            ""
        };
        return Finding::ok(
            id,
            sev,
            T9_EVIDENCE,
            format!(
                "no CPU quota on this process's cgroup: {}{hidden}",
                files.join(", ")
            ),
        );
    };
    let own = q.dir == cg.dir;
    let who = if own {
        "this process's cgroup"
    } else {
        "a parent cgroup"
    };
    let period = ms(q.period_us);
    let effect = if quota < q.period_us {
        format!(
            "A busy-spinning thread alone uses it up {} into every period, and the kernel stops \
             the whole cgroup for the other {}",
            ms(quota),
            ms(q.period_us - quota)
        )
    } else {
        format!(
            "A busy-spinning thread uses one CPU of it all the time, so when the {} threads (a \
             logger, a second spinner, a child process) bring the total to the quota, the kernel \
             stops all of them, the spinner too, until the period ends: up to {period}",
            if own {
                "cgroup's other"
            } else {
                "parent's other"
            }
        )
    };
    let mut message = format!(
        "{who}: {}: a quota of {} per {period} period. {effect}{}.",
        quota_text(cg.version, q),
        cpus(quota, q.period_us),
        so_far(host, cg.version, &q.dir)
    );
    for (other, _) in quotas {
        let _ = write!(message, " Also {}.", quota_text(cg.version, other));
    }
    let plain = match cg.version {
        CgroupVersion::V2 => format!("echo max > {}/cpu.max", q.dir),
        CgroupVersion::V1 => format!("echo -1 > {}/cpu.cfs_quota_us", q.dir),
    };
    let _ = write!(
        message,
        " Fix: on Kubernetes, use kubelet 1.33 or later with \
         `cpuManagerPolicy: static`, make the pod Guaranteed and this container's CPU request a \
         whole number: the kubelet then sets no quota on the container or its pod, and every \
         other pod keeps its CPU limit. Never set kubelet `cpuCFSQuota: false`: it removes the \
         CPU limit of every pod on the node. CRI-O also takes the pod \
         annotation `cpu-quota.crio.io: \"disable\"` with a high-performance runtime class \
         and whole CPUs; containerd ignores it. Elsewhere, run without a CPU limit (no `docker run --cpus`, an \
         empty systemd `CPUQuota=`), or as root `{plain}`"
    );
    Finding::found(id, sev, T9_EVIDENCE, message)
}

/// A cgroup's throttling counters, from its `cpu.stat` and
/// `cpu.stat.local`. `None`: not given here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Throttling {
    /// Periods that have passed with a thread runnable (`nr_periods`).
    pub periods: Option<u64>,
    /// Periods in which the cgroup's own quota stopped it
    /// (`nr_throttled`).
    pub throttled: Option<u64>,
    /// How long it was stopped by its own quota, in µs (`throttled_usec`; v1
    /// `throttled_time`, in ns, divided by 1000).
    pub throttled_us: Option<u64>,
    /// How long it was stopped by its own quota or a parent's, in µs
    /// (`throttled_usec` of `cpu.stat.local`, Linux 6.6 and later): the
    /// only counter that shows a pod's quota from inside a container.
    pub local_throttled_us: Option<u64>,
}

impl Throttling {
    /// How much each counter grew from `earlier` to `self`; `None` where
    /// either is `None`.
    #[must_use]
    pub fn since(&self, earlier: &Self) -> Self {
        let d = |now: Option<u64>, then: Option<u64>| Some(now?.saturating_sub(then?));
        Self {
            periods: d(self.periods, earlier.periods),
            throttled: d(self.throttled, earlier.throttled),
            throttled_us: d(self.throttled_us, earlier.throttled_us),
            local_throttled_us: d(self.local_throttled_us, earlier.local_throttled_us),
        }
    }

    /// Whether any throttling counter is above zero: on a [`Self::since`]
    /// difference, whether the cgroup was stopped in between.
    #[must_use]
    pub fn any(&self) -> bool {
        [self.throttled, self.throttled_us, self.local_throttled_us]
            .iter()
            .any(|c| c.is_some_and(|n| n > 0))
    }
}

/// The counters of a `cpu.stat` (`nr_periods`, `nr_throttled`, and
/// `throttled_usec`, or v1's `throttled_time` in ns).
fn parse_stat(text: &str, version: CgroupVersion) -> Result<Throttling, &'static str> {
    let mut t = Throttling::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        let (slot, divide) = match (key, version) {
            ("nr_periods", _) => (&mut t.periods, 1),
            ("nr_throttled", _) => (&mut t.throttled, 1),
            ("throttled_usec", CgroupVersion::V2) => (&mut t.throttled_us, 1),
            ("throttled_time", CgroupVersion::V1) => (&mut t.throttled_us, 1000),
            _ => continue,
        };
        let n: u64 = value
            .trim()
            .parse()
            .map_err(|_| "a throttling counter that is not a number")?;
        *slot = Some(n / divide);
    }
    Ok(t)
}

/// `throttled_usec` of a `cpu.stat.local`.
fn parse_local(text: &str) -> Result<Option<u64>, &'static str> {
    text.lines()
        .find_map(|l| l.strip_prefix("throttled_usec "))
        .map(|v| v.trim().parse().map_err(|_| "throttled_usec not a number"))
        .transpose()
}

/// Reads `file` from its start into `buf` (`pread` at offset 0: no seek,
/// no allocation); the bytes read.
#[cfg(unix)]
pub fn read_from_start(file: &File, buf: &mut [u8]) -> std::io::Result<usize> {
    use std::os::unix::fs::FileExt;
    let mut n = 0;
    while n < buf.len() {
        let at = u64::try_from(n).map_err(std::io::Error::other)?;
        match file.read_at(&mut buf[n..], at) {
            Ok(0) => break,
            Ok(got) => n += got,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

#[cfg(not(unix))]
pub fn read_from_start(file: &File, buf: &mut [u8]) -> std::io::Result<usize> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = file;
    file.seek(SeekFrom::Start(0))?;
    let mut n = 0;
    while n < buf.len() {
        match file.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(got) => n += got,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// An open counter file and its path on the host.
#[derive(Debug)]
struct Counter {
    file: File,
    path: String,
}

impl Counter {
    /// The file's text, read into `buf`.
    fn read<'b>(&self, buf: &'b mut [u8]) -> Result<&'b str, ReadError> {
        let n = read_from_start(&self.file, buf).map_err(|e| ReadError::io(&self.path, &e))?;
        std::str::from_utf8(&buf[..n]).map_err(|_| ReadError::Malformed {
            path: self.path.clone(),
            detail: "not text",
        })
    }
}

/// How many bytes a counter file is read into: a `cpu.stat` is about 200.
const BUF: usize = 4096;

/// T9 while the loop runs: this process's cgroup's throttling counters,
/// read again to see whether the kernel stopped it since the last read.
///
/// [`ThrottleWatch::open`] opens `cpu.stat` (and `cpu.stat.local`, Linux 6.6
/// and later, which also counts a parent's quota: the pod's, from inside a
/// container) once, before the loop. [`ThrottleWatch::check`] reads
/// `cpu.stat.local` again from its start (`pread`), and `cpu.stat` only after
/// that grew ([`ThrottleWatch::sample`]): no open, no allocation unless it
/// reports. Call it from housekeeping or idle-deferred work, never inside the
/// loop's hot path: once every few seconds is enough.
#[derive(Debug)]
pub struct ThrottleWatch {
    version: CgroupVersion,
    dir: String,
    stat: Counter,
    local: Option<Counter>,
    buf: Vec<u8>,
    last: Throttling,
    failed: bool,
}

impl ThrottleWatch {
    /// Opens this process's cgroup's counter files and reads them once.
    ///
    /// # Errors
    /// The cgroup cannot be found, or its `cpu.stat` cannot be opened or
    /// read (`cpu.stat.local` missing is not an error: [`Self::note`]).
    pub fn open(host: &Host) -> Result<Self, ReadError> {
        let (version, dirs) = locate(host)?;
        let dir = dirs.into_iter().next().unwrap_or_default();
        let path = format!("{dir}/cpu.stat");
        let stat = Counter {
            file: host.open(&path)?,
            path,
        };
        let local = match version {
            CgroupVersion::V2 => {
                let path = format!("{dir}/cpu.stat.local");
                host.open(&path).ok().map(|file| Counter { file, path })
            }
            CgroupVersion::V1 => None,
        };
        let mut watch = Self {
            version,
            dir,
            stat,
            local,
            buf: vec![0; BUF],
            last: Throttling::default(),
            failed: false,
        };
        watch.last = watch.sample()?;
        Ok(watch)
    }

    /// The cgroup directory watched, on the host.
    #[must_use]
    pub fn dir(&self) -> &str {
        &self.dir
    }

    /// The counters at the last read.
    #[must_use]
    pub const fn last(&self) -> Throttling {
        self.last
    }

    /// What this watch cannot see, for one line at start-up: no
    /// `cpu.stat.local` (Linux before 6.6), or no `nr_throttled` (the cpu
    /// controller is not on for this cgroup). `None` when it sees both.
    #[must_use]
    pub fn note(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.last.throttled.is_none() {
            parts.push(format!(
                "{} has no nr_throttled: the cpu controller is not on for this cgroup, so only a \
                 parent's quota can stop it",
                self.stat.path
            ));
        }
        if self.version == CgroupVersion::V2 && self.local.is_none() {
            parts.push(format!(
                "{}/cpu.stat.local is not here (Linux before 6.6): a parent cgroup's quota (a \
                 pod's) stopping this one is not seen",
                self.dir
            ));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }

    /// The counters now, from open files, with no allocation:
    /// `cpu.stat.local`, then `cpu.stat` only when that grew since
    /// [`Self::last`] or is not there. Printing `cpu.stat` first flushes
    /// the cgroup's CPU statistics under a lock every cgroup shares (Linux
    /// 6.12). `cpu.stat.local` counts every stop, by the cgroup's own quota
    /// too, so while it holds still `cpu.stat`'s counters stay as last read.
    ///
    /// # Errors
    /// A read fails, or a counter does not parse.
    pub fn sample(&mut self) -> Result<Throttling, ReadError> {
        let mut t = self.last;
        if let Some(local) = &self.local {
            let text = local.read(&mut self.buf)?;
            t.local_throttled_us = parse_local(text).map_err(|detail| ReadError::Malformed {
                path: local.path.clone(),
                detail,
            })?;
        }
        if t.local_throttled_us.is_some() && t.local_throttled_us == self.last.local_throttled_us {
            return Ok(t);
        }
        let text = self.stat.read(&mut self.buf)?;
        let stat = parse_stat(text, self.version).map_err(|detail| ReadError::Malformed {
            path: self.stat.path.clone(),
            detail,
        })?;
        t.periods = stat.periods;
        t.throttled = stat.throttled;
        t.throttled_us = stat.throttled_us;
        Ok(t)
    }

    /// T9 again: a warning when the kernel stopped this cgroup since the
    /// last call (or [`Self::open`]), saying how often and for how long;
    /// `None` when it did not. A read that fails gives one unknown finding,
    /// the first time only.
    pub fn check(&mut self) -> Option<Finding> {
        let now = match self.sample() {
            Ok(t) => t,
            Err(e) => {
                if self.failed {
                    return None;
                }
                self.failed = true;
                return Some(Finding::unknown(
                    CheckId::T9,
                    Severity::Warn,
                    T9_EVIDENCE,
                    "cannot read this process's throttling counters".to_owned(),
                    e,
                ));
            }
        };
        self.failed = false;
        let (before, d) = (self.last, now.since(&self.last));
        self.last = now;
        if !d.any() {
            return None;
        }
        let own = d.throttled.unwrap_or(0) > 0 || d.throttled_us.unwrap_or(0) > 0;
        let message = if own {
            let times = match (d.throttled, d.periods) {
                (Some(t), Some(p)) => format!("in {t} of the last {p} periods"),
                (Some(t), None) => format!("{t} times"),
                _ => "again".to_owned(),
            };
            let total = d
                .throttled_us
                .map_or_else(String::new, |us| format!(", {} in all", ms(us)));
            let counter = match (before.throttled, now.throttled) {
                (Some(a), Some(b)) => format!(" (nr_throttled {a} → {b})"),
                _ => String::new(),
            };
            format!(
                "this process's cgroup ({}) was throttled {times}{total}{counter}: each time the \
                 kernel stopped every thread in it, busy-spinning ones too, until the period \
                 ended. {FIX_SHORT}",
                self.dir
            )
        } else {
            format!(
                "a parent cgroup (a pod's) throttled this process's cgroup ({}): cpu.stat.local \
                 throttled_usec grew by {} while its own nr_throttled did not; each time every \
                 thread here, busy-spinning ones too, stopped until the period ended. {FIX_SHORT}",
                self.dir,
                ms(d.local_throttled_us.unwrap_or(0))
            )
        };
        Some(Finding::found(
            CheckId::T9,
            Severity::Warn,
            T9_EVIDENCE,
            message,
        ))
    }
}
