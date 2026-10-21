//! T11's readers: what a host says about how its wall clock is kept, every
//! source optional and bounded (nothing here can hang). Linux's `adjtimex`
//! with `modes` 0 (the running host only), `/sys/class/ptp/*/clock_name`,
//! which time daemons run (`/proc/*/comm`), chronyd's tracking report over
//! its command port (UDP to 127.0.0.1:323, unprivileged, a 50 ms timeout)
//! and `ClockBound` 2.0's segment.

use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use super::chrony;
use super::clockbound::{self, ClockBoundError};
use super::kernel::KernelSync;
use super::sync::{
    ChronyAnswer, ClockBoundAnswer, Observed, PtpClock, TimeDaemon, TimeSync, TimeSyncLimits,
};
use crate::finding::Finding;
use crate::host::Host;

/// What T11 asks and where (and what it warns over): the defaults read the
/// running host; a test points them at fixtures and a fake chronyd.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeSyncProbe {
    /// chronyd's command port, an address (nothing is resolved); `None`:
    /// not asked. Asked whatever the [`Host`]: on a tree standing for a
    /// host, set it to `None` or a fake.
    pub chrony: Option<SocketAddr>,
    /// How long chronyd has to answer at start-up: 50 ms (on the loopback
    /// it answers in well under a millisecond, and a port nobody listens on
    /// is refused at once). [`ClockWatch`](crate::ClockWatch) never waits.
    pub chrony_timeout: Duration,
    /// Whether to read `ClockBound` 2.0's segment
    /// (`/var/run/clockbound/shm0` under the host).
    pub clockbound: bool,
    /// The `CLOCK_MONOTONIC_COARSE` reading, ns, to judge `ClockBound`'s
    /// segment at. `None`: the running Linux host's clock; a tree standing
    /// for a host has none, and its segment is
    /// [`ClockBoundError::NoClock`] unless this is given.
    pub clockbound_now_ns: Option<u64>,
    /// The most `/proc` entries read (a scan of a host with many processes
    /// stays bounded): 65,536.
    pub max_processes: usize,
    /// What T11 warns over.
    pub limits: TimeSyncLimits,
}

impl Default for TimeSyncProbe {
    fn default() -> Self {
        Self {
            chrony: Some(SocketAddr::from((Ipv4Addr::LOCALHOST, chrony::PORT))),
            chrony_timeout: Duration::from_millis(50),
            clockbound: true,
            clockbound_now_ns: None,
            max_processes: 65_536,
            limits: TimeSyncLimits::default(),
        }
    }
}

impl TimeSyncProbe {
    /// Reads `host` and judges it ([`TimeSync::from_observed`]). Allocates;
    /// waits at most [`chrony_timeout`](Self::chrony_timeout) for chronyd.
    /// A start-up or server call, never a busy loop's.
    #[must_use]
    pub fn run(&self, host: &Host) -> TimeSync {
        TimeSync::from_observed(self.observe(host))
    }

    /// What `host` says, unjudged. The running host off Linux:
    /// [`Observed::not_linux`], nothing read. A tree: everything but the
    /// kernel (which answers for the running host only).
    #[must_use]
    pub fn observe(&self, host: &Host) -> Observed {
        if host.is_live() && !cfg!(target_os = "linux") {
            return Observed::not_linux();
        }
        Observed {
            linux: true,
            kernel: KernelSync::read(host).ok(),
            daemons: self.daemons(host),
            ptp_clocks: self.ptp_clocks(host),
            chrony: self.chrony(),
            clockbound: self.clockbound(host),
        }
    }

    /// The time daemons running, each once, by their `comm` under the
    /// host's `/proc`: `None` where it cannot be listed. A process that
    /// exits mid-scan is passed over, and so is a zombie (its `stat` state
    /// `Z` or `X`: exited, not yet reaped), which still has a `comm`.
    #[must_use]
    pub fn daemons(&self, host: &Host) -> Option<Vec<TimeDaemon>> {
        let entries = std::fs::read_dir(host.path("/proc")).ok()?;
        let mut seen = Vec::new();
        for entry in entries.take(self.max_processes).flatten() {
            let name = entry.file_name();
            let is_pid = name
                .to_str()
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            if !is_pid {
                continue;
            }
            let mut comm = [0_u8; 32];
            let len = std::fs::File::open(entry.path().join("comm"))
                .and_then(|mut f| f.read(&mut comm))
                .unwrap_or(0);
            let daemon = std::str::from_utf8(comm.get(..len).unwrap_or_default())
                .ok()
                .and_then(TimeDaemon::from_comm);
            if let Some(d) = daemon
                && !seen.contains(&d)
                && !exited(&entry.path())
            {
                seen.push(d);
            }
        }
        seen.sort_unstable();
        Some(seen)
    }

    /// The PTP hardware clocks under the host's `/sys/class/ptp`, by device:
    /// none where there is no such directory (no PTP support, or not Linux).
    #[must_use]
    pub fn ptp_clocks(&self, host: &Host) -> Vec<PtpClock> {
        let Ok(entries) = std::fs::read_dir(host.path("/sys/class/ptp")) else {
            return Vec::new();
        };
        let mut clocks: Vec<PtpClock> = entries
            .flatten()
            .filter_map(|entry| {
                let device = entry.file_name().to_str()?.to_owned();
                if !device.starts_with("ptp") {
                    return None;
                }
                let name = std::fs::read_to_string(entry.path().join("clock_name"))
                    .map_or_else(|_| "-".to_owned(), |n| n.trim().to_owned());
                Some(PtpClock { device, name })
            })
            .collect();
        clocks.sort();
        clocks
    }

    /// chronyd's tracking report from [`chrony`](Self::chrony), within
    /// [`chrony_timeout`](Self::chrony_timeout).
    #[must_use]
    pub fn chrony(&self) -> ChronyAnswer {
        let Some(addr) = self.chrony else {
            return ChronyAnswer::NotAsked;
        };
        match chrony::query(addr, self.chrony_timeout, sequence()) {
            Ok(tracking) => ChronyAnswer::Tracking(tracking),
            Err(e) => ChronyAnswer::Failed(e),
        }
    }

    /// `ClockBound`'s bound now, from its segment under the host.
    #[must_use]
    pub fn clockbound(&self, host: &Host) -> ClockBoundAnswer {
        if !self.clockbound {
            return ClockBoundAnswer::NotAsked;
        }
        clockbound_at(&host.path(clockbound::DEFAULT_PATH), self.coarse_now(host))
    }

    /// The `CLOCK_MONOTONIC_COARSE` reading to judge `ClockBound`'s segment
    /// at: the one given, else the running Linux host's clock.
    pub(crate) fn coarse_now(&self, host: &Host) -> Option<u64> {
        self.clockbound_now_ns
            .or_else(|| host.is_live().then(coarse_now).flatten())
    }
}

/// `ClockBound`'s segment at `path`, judged at `now` (none: no clock to
/// judge it against). Allocates nothing.
pub fn clockbound_at(path: &Path, now: Option<u64>) -> ClockBoundAnswer {
    match clockbound::read(path) {
        Ok(None) => ClockBoundAnswer::Absent,
        Err(e) => ClockBoundAnswer::Unusable(e),
        Ok(Some(segment)) => now.map_or(
            ClockBoundAnswer::Unusable(ClockBoundError::NoClock),
            |now| match clockbound::parse(&segment, now) {
                Ok(reading) => ClockBoundAnswer::Reading(reading),
                Err(why) => ClockBoundAnswer::Unusable(ClockBoundError::Malformed(why)),
            },
        ),
    }
}

/// Linux's `CLOCK_MONOTONIC_COARSE`, the clock `ClockBound`'s daemon stamps
/// with (through the vDSO: no system call, no allocation); `None` elsewhere.
#[allow(clippy::missing_const_for_fn)] // const only off Linux
pub fn coarse_now() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let ts = rustix::time::clock_gettime(rustix::time::ClockId::MonotonicCoarse);
        let secs = u64::try_from(ts.tv_sec).ok()?;
        let nanos = u64::try_from(ts.tv_nsec).ok()?;
        secs.checked_mul(1_000_000_000)?.checked_add(nanos)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Whether the process at `pid_dir` has exited but is still listed: its
/// `stat`'s state (the field after the parenthesised `comm`, which may itself
/// hold a `)`) is `Z` (zombie) or `X` (dead). One read into a buffer on the
/// stack, only for a process whose `comm` names a time daemon; an unreadable
/// `stat` is no evidence either way, so the process counts.
fn exited(pid_dir: &Path) -> bool {
    let mut stat = [0_u8; 512];
    let len = std::fs::File::open(pid_dir.join("stat"))
        .and_then(|mut f| f.read(&mut stat))
        .unwrap_or(0);
    let stat = stat.get(..len).unwrap_or_default();
    stat.iter()
        .rposition(|&b| b == b')')
        .and_then(|close| stat.get(close + 1..))
        .and_then(|rest| rest.iter().find(|b| !b.is_ascii_whitespace()))
        .is_some_and(|state| matches!(state, b'Z' | b'X'))
}

/// A first request's sequence number: this process and the wall clock's
/// low bits, so a reply to another process's or an earlier request is told
/// apart. No clock of its own: std's `SystemTime`.
pub fn sequence() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    nanos ^ std::process::id().rotate_left(16)
}

/// How `CLOCK_REALTIME` is kept on `host` now: [`TimeSyncProbe`]'s defaults.
///
/// For start-up or a server's housekeeping (every minute or so: the verdict
/// changes when a daemon starts, stops, locks or loses its source). It may
/// wait 50 ms for chronyd, so never a busy loop's: that is
/// [`ClockWatch`](crate::ClockWatch)'s job.
#[must_use]
pub fn time_sync(host: &Host) -> TimeSync {
    TimeSyncProbe::default().run(host)
}

/// T11: how `CLOCK_REALTIME` is kept, as findings
/// ([`TimeSync::findings`]): the verdict always, and warnings for an
/// unsettled clock, a wide error bound or a pending leap second.
#[must_use]
pub fn t11_time_sync(host: &Host, probe: &TimeSyncProbe) -> Vec<Finding> {
    probe.run(host).findings(&probe.limits)
}
