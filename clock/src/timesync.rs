//! Whether the wall clock is disciplined, to what and how well, and what
//! the report finds about the host's clocks.
//!
//! UTC stamps are only as good as `CLOCK_REALTIME`, which kernel and NIC
//! timestamps and a [`WallOffset`](crate::advanced::WallOffset) are on too.
//! A stamp compared with another host's (a one-way latency, a venue's event
//! time) is off by both hosts' errors, and an undisciplined or
//! NTP-disciplined wall clock's error is tens of microseconds to
//! milliseconds, against about a microsecond for one disciplined to a PTP
//! hardware clock (PHC). The default clock follows `CLOCK_REALTIME`'s rate
//! within ±500 ppm, so its durations include the host's rate corrections;
//! raw ticks (the counter on its own) are never affected.
//!
//! [`TimeSyncProbe`] reads what the host says, every source optional and
//! bounded (nothing here can hang): Linux's `adjtimex` with `modes` 0
//! (`STA_UNSYNC`, `maxerror`), `/sys/class/ptp/*/clock_name`, which time
//! daemons run (`/proc/*/comm`), chronyd's tracking report over its command
//! port (UDP to 127.0.0.1:323, unprivileged, a 50 ms timeout) and `ClockBound`
//! 2.0's shared memory segment. [`TimeSync::from_observed`] turns that into
//! a [`SyncVerdict`], an error bound where one is known, and a
//! [`Finding`]: a warning when the wall clock is not PTP-grade, never a
//! failure unless the caller asks for one ([`TimeSyncPolicy::RequirePtp`]).
//! Anything that cannot be read is reported as unknown, never as a warning.

use crate::chrony::{self, ChronyError, ChronyReference, ChronyTracking};
use crate::clockbound::{self, ClockBoundReading};
use crate::error::Error;
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How much a [`Finding`] matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Worth a log line at info: nothing to fix, or nothing could be read.
    Info,
    /// Worth a log line at warn: something to fix for latency or accuracy.
    Warn,
}

impl Severity {
    /// The name a log line prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
        }
    }
}

/// Which check a [`Finding`] is from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// The kernel's clock source (Linux's `current_clocksource`).
    Clocksource,
    /// How `CLOCK_REALTIME` is disciplined ([`TimeSync`]).
    TimeSync,
}

impl Check {
    /// The name a log line prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clocksource => "clocksource",
            Self::TimeSync => "time-sync",
        }
    }
}

/// One thing found about the host's clocks: a log line's worth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The check it is from.
    pub check: Check,
    /// How much it matters.
    pub severity: Severity,
    /// What was found, its effect and its fix: one line.
    pub message: String,
}

impl Finding {
    pub(crate) const fn new(check: Check, severity: Severity, message: String) -> Self {
        Self {
            check,
            severity,
            message,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.check.name(), self.message)
    }
}

/// Whether calibration also judges how the wall clock is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeSyncPolicy {
    /// Not probed: no time-sync section in the report.
    Off,
    /// Probed and reported, a warning when not PTP-grade, never a failure.
    /// The default.
    #[default]
    Warn,
    /// Probed, and calibration fails unless the verdict is
    /// [`SyncVerdict::PtpGrade`]: for a process whose wall-clock stamps must
    /// be comparable across hosts to about a microsecond. Unknown (another
    /// PID or network namespace, macOS) is not PTP-grade.
    RequirePtp,
}

/// How `CLOCK_REALTIME` is kept, as far as this process can see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncVerdict {
    /// Disciplined to a PTP hardware clock: chronyd locked to a PHC
    /// reference clock, or phc2sys running with the kernel synchronised and
    /// no other client seen adjusting `CLOCK_REALTIME`.
    PtpGrade,
    /// Synchronised, but not to a PTP hardware clock: chronyd or ntpd to an
    /// NTP server, systemd-timesyncd, or a reference clock not named as a
    /// PHC. Its error is the bound reported, where one is known.
    NtpDisciplined,
    /// Not synchronised to anything: the wall clock's error has no bound.
    Unsynchronised,
    /// Not known: not Linux, or what decides it could not be read (the
    /// daemon is in another PID or network namespace, or `/proc` is mounted
    /// with `hidepid`), or phc2sys runs beside another client and which of
    /// them keeps the clock cannot be told.
    Unknown,
}

impl SyncVerdict {
    /// The name the report prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PtpGrade => "ptp-grade",
            Self::NtpDisciplined => "ntp-disciplined",
            Self::Unsynchronised => "unsynchronised",
            Self::Unknown => "unknown",
        }
    }
}

/// A time daemon this crate recognises by its process name (`comm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeDaemon {
    /// chronyd.
    Chronyd,
    /// ntpd (the reference implementation, `NTPsec`, `OpenNTPD`).
    Ntpd,
    /// systemd-timesyncd, an SNTP client.
    SystemdTimesyncd,
    /// linuxptp's phc2sys, which disciplines a clock to another (a PHC).
    Phc2sys,
    /// linuxptp's ptp4l, which disciplines a NIC's PHC over PTP.
    Ptp4l,
    /// AWS `ClockBound`'s daemon.
    ClockBound,
}

impl TimeDaemon {
    /// Every daemon recognised.
    pub const ALL: [Self; 6] = [
        Self::Chronyd,
        Self::Ntpd,
        Self::SystemdTimesyncd,
        Self::Phc2sys,
        Self::Ptp4l,
        Self::ClockBound,
    ];

    /// Its process name, as `/proc/<pid>/comm` holds it (at most 15
    /// bytes: `systemd-timesyncd` is cut to `systemd-timesyn`).
    #[must_use]
    pub const fn comm(self) -> &'static str {
        match self {
            Self::Chronyd => "chronyd",
            Self::Ntpd => "ntpd",
            Self::SystemdTimesyncd => "systemd-timesyn",
            Self::Phc2sys => "phc2sys",
            Self::Ptp4l => "ptp4l",
            Self::ClockBound => "clockbound",
        }
    }

    /// Its name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SystemdTimesyncd => "systemd-timesyncd",
            other => other.comm(),
        }
    }

    /// The daemon a process name is, if any.
    #[must_use]
    pub fn from_comm(comm: &str) -> Option<Self> {
        let comm = comm.trim();
        Self::ALL.into_iter().find(|d| d.comm() == comm)
    }
}

/// A PTP hardware clock: `/sys/class/ptp/<device>/clock_name`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PtpClock {
    /// The device, `ptp0`: `/dev/ptp0`.
    pub device: String,
    /// The driver's name for it: `hyperv` on Azure, `ena-ptp-*` for AWS's
    /// ENA, `mlx5_ptp` and the like for other NICs.
    pub name: String,
}

/// `adjtimex`'s `STA_UNSYNC`: the clock is unsynchronised.
pub const STA_UNSYNC: i32 = 0x0040;
/// `adjtimex`'s `STA_NANO`: `offset` is in ns, not µs.
pub const STA_NANO: i32 = 0x2000;
/// The kernel's `maxerror` limit, µs (`NTP_PHASE_LIMIT`, 16 s).
///
/// The kernel adds 500 µs to `maxerror` every second (`second_overflow`,
/// Linux `kernel/time/ntp.c`) and sets `STA_UNSYNC` once it reaches this,
/// so a `maxerror` below it is being kept by a daemon.
pub const MAXERROR_LIMIT_US: i64 = 16_000_000;

/// The kernel's view of the wall clock: `adjtimex` with `modes` 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KernelSync {
    /// The clock state: 0 `TIME_OK` ... 5 `TIME_ERROR` (unsynchronised).
    pub state: i32,
    /// The status bits (`STA_*`).
    pub status: i32,
    /// The maximum error, µs, as the daemon last set it plus 500 µs a second
    /// since. chronyd sets it to its root distance at each update; ntpd
    /// keeps it too; systemd-timesyncd and phc2sys set it to 0, so under
    /// them it is not a bound.
    pub maxerror_us: i64,
    /// The estimated error, µs, as the daemon set it.
    pub esterror_us: i64,
    /// The offset the kernel's PLL is still slewing out, ns.
    pub offset_ns: i64,
    /// The frequency correction, ppm.
    pub freq_ppm: f64,
}

impl KernelSync {
    /// The kernel's view now: Linux's `adjtimex` with `modes` 0 (a read,
    /// unprivileged, under a microsecond), with nothing allocated, so a
    /// loop's housekeeping can call it every few seconds.
    ///
    /// # Errors
    /// The error number: the call failed (`EPERM` under a seccomp filter
    /// that denies it, say), or `ENOSYS` off Linux.
    #[allow(clippy::useless_conversion)] // c_long: i64 on 64-bit Linux, i32 on 32-bit
    #[allow(clippy::missing_const_for_fn)] // const only off Linux
    pub fn read() -> Result<Self, i32> {
        #[cfg(target_os = "linux")]
        {
            let (state, tx) = crate::arch::adjtimex_read()?;
            let status = tx.status;
            let offset = i64::from(tx.offset);
            #[allow(clippy::cast_precision_loss)] // ppm, scaled by 2^16: far under 2^52
            let freq_ppm = i64::from(tx.freq) as f64 / 65_536.0;
            Ok(Self {
                state,
                status,
                maxerror_us: i64::from(tx.maxerror),
                esterror_us: i64::from(tx.esterror),
                offset_ns: if status & STA_NANO != 0 {
                    offset
                } else {
                    offset.saturating_mul(1_000)
                },
                freq_ppm,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(libc::ENOSYS)
        }
    }

    /// Whether the kernel says the clock is unsynchronised (`STA_UNSYNC`).
    #[must_use]
    pub const fn unsynchronised(&self) -> bool {
        self.status & STA_UNSYNC != 0
    }

    /// Whether something keeps `maxerror` below the kernel's limit: a
    /// daemon has updated it in the last nine hours or so.
    #[must_use]
    pub const fn maxerror_kept(&self) -> bool {
        self.maxerror_us < MAXERROR_LIMIT_US
    }
}

/// What chronyd's command port said.
#[derive(Clone, Debug, PartialEq)]
pub enum ChronyAnswer {
    /// Not asked (not Linux, or no address configured).
    NotAsked,
    /// Its tracking report.
    Tracking(ChronyTracking),
    /// No report: why.
    Failed(ChronyError),
}

/// What `ClockBound`'s segment said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClockBoundAnswer {
    /// Not looked for.
    NotAsked,
    /// No segment: `ClockBound` 2.0 is not running here.
    Absent,
    /// Its bound.
    Reading(ClockBoundReading),
    /// A segment that could not be used: why.
    Unusable(String),
}

/// Everything read about the wall clock's discipline, before judging it:
/// what a [`TimeSyncProbe`] observes, or a test's fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct Observed {
    /// Whether this is Linux, where all of it can be read.
    pub linux: bool,
    /// The kernel's state, where `adjtimex` could be read.
    pub kernel: Option<KernelSync>,
    /// The time daemons seen running; `None` where `/proc` could not be
    /// read.
    pub daemons: Option<Vec<TimeDaemon>>,
    /// The PTP hardware clocks present.
    pub ptp_clocks: Vec<PtpClock>,
    /// chronyd's tracking report, or why there is none.
    pub chrony: ChronyAnswer,
    /// `ClockBound`'s bound, or why there is none.
    pub clockbound: ClockBoundAnswer,
}

impl Observed {
    /// A host about which nothing was read: not Linux.
    #[must_use]
    pub const fn not_linux() -> Self {
        Self {
            linux: false,
            kernel: None,
            daemons: None,
            ptp_clocks: Vec::new(),
            chrony: ChronyAnswer::NotAsked,
            clockbound: ClockBoundAnswer::NotAsked,
        }
    }

    /// Whether `daemon` was seen running.
    #[must_use]
    pub fn runs(&self, daemon: TimeDaemon) -> bool {
        self.daemons.as_ref().is_some_and(|d| d.contains(&daemon))
    }
}

/// Where an error bound came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundSource {
    /// `ClockBound` 2.0's segment.
    ClockBound,
    /// chronyd's tracking report: `|offset| + root dispersion + root delay / 2`.
    Chrony,
    /// The kernel's `maxerror`, as chronyd or ntpd keep it.
    KernelMaxerror,
}

impl BoundSource {
    /// The name the report prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ClockBound => "clockbound",
            Self::Chrony => "chrony",
            Self::KernelMaxerror => "kernel-maxerror",
        }
    }
}

/// A bound on `CLOCK_REALTIME`'s error against UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bound {
    /// The bound, ns.
    pub ns: u64,
    /// Where it came from.
    pub source: BoundSource,
}

/// How `CLOCK_REALTIME` is kept: the verdict, what keeps it, its error
/// bound where known, and what was read.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeSync {
    /// The verdict.
    pub verdict: SyncVerdict,
    /// What keeps it, without spaces: `chronyd:PHC0`,
    /// `chronyd:169.254.169.123`, `systemd-timesyncd`, `phc2sys`, `kernel`
    /// (a daemon not visible here), or `-`.
    pub source: String,
    /// Why the verdict, in words.
    pub reason: String,
    /// The error bound, where one is known.
    pub bound: Option<Bound>,
    /// What was read.
    pub observed: Observed,
    /// How long reading it took.
    pub took: Duration,
}

impl TimeSync {
    /// The verdict on what was observed.
    #[must_use]
    pub fn from_observed(observed: Observed) -> Self {
        let (verdict, source, reason) = assess(&observed);
        let bound = bound(&observed, verdict);
        Self {
            verdict,
            source,
            reason,
            bound,
            observed,
            took: Duration::ZERO,
        }
    }

    /// The finding: info when PTP-grade or unknown, else a warning with
    /// its effect and fix.
    #[must_use]
    pub fn finding(&self) -> Finding {
        finding(self)
    }

    /// The time-sync section as `(key, value)`s, values without spaces or
    /// commas: the report's fields.
    #[must_use]
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        let o = &self.observed;
        let clean = |s: &str| s.replace([' ', ','], "_");
        vec![
            ("time_sync", self.verdict.name().to_owned()),
            ("time_sync_source", clean(&self.source)),
            (
                "time_sync_bound_ns",
                self.bound
                    .map_or_else(|| "-".to_owned(), |b| b.ns.to_string()),
            ),
            (
                "time_sync_bound_from",
                self.bound.map_or("-", |b| b.source.name()).to_owned(),
            ),
            (
                "time_daemons",
                o.daemons.as_ref().map_or_else(
                    || if o.linux { "unreadable" } else { "-" }.to_owned(),
                    |d| {
                        if d.is_empty() {
                            "-".to_owned()
                        } else {
                            d.iter().map(|d| d.name()).collect::<Vec<_>>().join(";")
                        }
                    },
                ),
            ),
            (
                "ptp_clocks",
                if o.ptp_clocks.is_empty() {
                    "-".to_owned()
                } else {
                    o.ptp_clocks
                        .iter()
                        .map(|p| clean(&format!("{}:{}", p.device, p.name)))
                        .collect::<Vec<_>>()
                        .join(";")
                },
            ),
            (
                "kernel_sync",
                o.kernel
                    .map_or("-", |k| {
                        if k.unsynchronised() {
                            "unsynchronised"
                        } else {
                            "synchronised"
                        }
                    })
                    .to_owned(),
            ),
            (
                "kernel_maxerror_us",
                o.kernel
                    .map_or_else(|| "-".to_owned(), |k| k.maxerror_us.to_string()),
            ),
            (
                "time_sync_ms",
                format!("{:.3}", self.took.as_secs_f64() * 1e3),
            ),
        ]
    }
}

/// The verdict, what keeps the clock (no spaces), and why. chronyd's own
/// word first (it says what it is locked to), then phc2sys (which keeps no
/// `maxerror`, so only the kernel's `STA_UNSYNC` says whether it has
/// locked), then the kernel's state as the daemon seen keeps it; what
/// cannot be told apart is unknown, never a warning.
fn assess(o: &Observed) -> Judged {
    use SyncVerdict::{NtpDisciplined, PtpGrade, Unknown, Unsynchronised};
    let v = |verdict, source: &str, reason: String| (verdict, source.to_owned(), reason);
    if !o.linux {
        return v(
            Unknown,
            "-",
            "not Linux: no adjtimex, PTP hardware clock or chronyd command port is read here"
                .to_owned(),
        );
    }
    let tracking = match &o.chrony {
        ChronyAnswer::Tracking(t) => Some(t),
        _ => None,
    };
    if let Some(t) = tracking
        && t.synchronised()
    {
        let source = chrony_source(t);
        // chronyd sets maxerror to its root distance at every update
        // (sys_timex.c `set_sync_status`): one left at the kernel's limit
        // means it is not adjusting this clock (`chronyd -x`)
        if let Some(k) = &o.kernel
            && !k.maxerror_kept()
        {
            return (
                Unsynchronised,
                source,
                format!(
                    "chronyd reports itself synchronised to {} but does not adjust this kernel's \
                     clock (its maxerror is at the 16 s limit, where chronyd sets it at every \
                     update: chronyd -x)",
                    t.reference_name()
                ),
            );
        }
        return match &t.reference {
            ChronyReference::Refclock(name) if t.is_phc() => (
                PtpGrade,
                source,
                format!("chronyd is locked to reference clock {name}, a PTP hardware clock"),
            ),
            ChronyReference::Refclock(name) => (
                NtpDisciplined,
                source,
                format!(
                    "chronyd is locked to reference clock {name}, not named as a PTP hardware \
                     clock (chrony names `refclock PHC` PHC0, PHC1, ...)"
                ),
            ),
            _ => (
                NtpDisciplined,
                source,
                format!(
                    "chronyd tracks NTP server {} at stratum {}",
                    t.reference_name(),
                    t.stratum
                ),
            ),
        };
    }
    let kernel = o.kernel.as_ref();
    if o.runs(TimeDaemon::Phc2sys) {
        return from_phc2sys(o, tracking.is_some(), kernel);
    }
    if let Some(t) = tracking {
        let reason = if t.reference == ChronyReference::Local {
            format!(
                "chronyd serves its own clock (local stratum {}) and is synchronised to no source",
                t.stratum
            )
        } else {
            "chronyd is synchronised to no source".to_owned()
        };
        return (Unsynchronised, chrony_source(t), reason);
    }
    let Some(k) = kernel else {
        return v(
            Unknown,
            "-",
            "the kernel's state (adjtimex) could not be read".to_owned(),
        );
    };
    from_kernel(o, k)
}

/// A verdict, what keeps the clock and why.
type Judged = (SyncVerdict, String, String);

/// phc2sys running, chronyd not synchronised: it keeps no `maxerror`, so
/// only the kernel's `STA_UNSYNC` says whether it has locked, and only where
/// no other client adjusts `CLOCK_REALTIME` (systemd-timesyncd, ntpd, a
/// chronyd that did not answer; one that answered unsynchronised adjusts
/// nothing) is it the one keeping it.
fn from_phc2sys(o: &Observed, chronyd_answered: bool, kernel: Option<&KernelSync>) -> Judged {
    let rivals: Vec<&str> = [
        TimeDaemon::SystemdTimesyncd,
        TimeDaemon::Ntpd,
        TimeDaemon::Chronyd,
    ]
    .into_iter()
    .filter(|d| o.runs(*d) && !(*d == TimeDaemon::Chronyd && chronyd_answered))
    .map(TimeDaemon::name)
    .collect();
    let (verdict, reason) = match kernel {
        Some(k) if !rivals.is_empty() && (!k.unsynchronised() || k.maxerror_kept()) => (
            SyncVerdict::Unknown,
            format!(
                "phc2sys is running beside {}, which also disciplines CLOCK_REALTIME: which of \
                 them keeps it is unknown (phc2sys may discipline another clock, or feed it \
                 through a shared-memory reference clock), and two that both adjust it fight \
                 over it",
                rivals.join(" and ")
            ),
        ),
        Some(k) if !k.unsynchronised() => (
            SyncVerdict::PtpGrade,
            "phc2sys is running and the kernel says CLOCK_REALTIME is synchronised".to_owned(),
        ),
        Some(_) => (
            SyncVerdict::Unsynchronised,
            "phc2sys is running but the kernel says CLOCK_REALTIME is unsynchronised \
             (STA_UNSYNC): it has not locked, or it disciplines another clock"
                .to_owned(),
        ),
        None => (
            SyncVerdict::Unknown,
            "phc2sys is running but the kernel's state (adjtimex) could not be read".to_owned(),
        ),
    };
    (verdict, "phc2sys".to_owned(), reason)
}

/// The verdict from the kernel's state, as the daemon seen running keeps
/// it, where chronyd gave no report and phc2sys is not running.
fn from_kernel(o: &Observed, k: &KernelSync) -> Judged {
    let maxerror = human_ns(
        u64::try_from(k.maxerror_us)
            .unwrap_or(0)
            .saturating_mul(1_000),
    );
    let daemon = [
        TimeDaemon::Chronyd,
        TimeDaemon::Ntpd,
        TimeDaemon::SystemdTimesyncd,
        TimeDaemon::Ptp4l,
    ]
    .into_iter()
    .find(|d| o.runs(*d));
    match daemon {
        Some(TimeDaemon::Chronyd) => chronyd_silent(o, k, &maxerror),
        Some(TimeDaemon::SystemdTimesyncd) if k.unsynchronised() && k.maxerror_kept() => (
            // it zeroes maxerror at every update, and sets STA_UNSYNC
            // itself where the RTC keeps local time (timesyncd-manager.c
            // `manager_adjust_clock`)
            SyncVerdict::NtpDisciplined,
            "systemd-timesyncd".to_owned(),
            format!(
                "systemd-timesyncd is running and keeps the kernel's maxerror ({maxerror}); \
                 STA_UNSYNC is set, as it leaves it where the RTC keeps local time"
            ),
        ),
        Some(d @ (TimeDaemon::Ntpd | TimeDaemon::SystemdTimesyncd)) => {
            let name = d.name();
            if k.unsynchronised() {
                (
                    SyncVerdict::Unsynchronised,
                    name.to_owned(),
                    format!(
                        "{name} is running but the kernel says unsynchronised (STA_UNSYNC): not \
                         synchronised yet"
                    ),
                )
            } else {
                (
                    SyncVerdict::NtpDisciplined,
                    name.to_owned(),
                    format!("{name} is running and the kernel says synchronised"),
                )
            }
        }
        Some(_) => (
            SyncVerdict::Unknown,
            "ptp4l".to_owned(),
            "ptp4l is running without phc2sys: it disciplines a NIC's PTP hardware clock, and \
             CLOCK_REALTIME only with software timestamping (-S)"
                .to_owned(),
        ),
        None => unseen_daemon(o, k, &maxerror),
    }
}

/// chronyd running but silent on its command port (`cmdport 0`, another
/// network namespace): its kernel state, and whether a PHC is there to be
/// locked to.
fn chronyd_silent(o: &Observed, k: &KernelSync, maxerror: &str) -> Judged {
    let silent = match &o.chrony {
        ChronyAnswer::Failed(e) => format!(" ({e})"),
        _ => String::new(),
    };
    let (verdict, reason) = if !k.unsynchronised() {
        o.ptp_clocks.first().map_or_else(
            || {
                (
                    SyncVerdict::NtpDisciplined,
                    format!(
                        "chronyd is running and the kernel says synchronised (maxerror \
                         {maxerror}); it did not answer on its command port{silent}, and no PTP \
                         hardware clock is present"
                    ),
                )
            },
            |p| {
                (
                    SyncVerdict::Unknown,
                    format!(
                        "chronyd is running and a PTP hardware clock is present ({}: {}), but \
                         chronyd did not answer on its command port{silent}: whether it is \
                         locked to the PHC is unknown",
                        p.device, p.name
                    ),
                )
            },
        )
    } else if k.maxerror_kept() {
        (
            SyncVerdict::Unknown,
            format!(
                "chronyd is running and keeps the kernel's maxerror ({maxerror}) but did not \
                 answer on its command port{silent}, and STA_UNSYNC is set, which chronyd leaves \
                 set without `rtcsync`"
            ),
        )
    } else {
        (
            SyncVerdict::Unsynchronised,
            format!(
                "chronyd is running but the kernel says unsynchronised (STA_UNSYNC, maxerror at \
                 its 16 s limit) and chronyd did not answer on its command port{silent}"
            ),
        )
    };
    (verdict, "chronyd".to_owned(), reason)
}

/// No time daemon visible: the kernel's state alone, which says whether
/// one is keeping the clock but not which.
fn unseen_daemon(o: &Observed, k: &KernelSync, maxerror: &str) -> Judged {
    let unseen = if o.daemons.is_none() {
        "the daemon keeping it is not visible: /proc could not be listed"
    } else {
        "the daemon keeping it is not visible to this process (it may run in another PID \
         namespace, /proc may be mounted with hidepid, or it is one this crate does not know)"
    };
    if !k.unsynchronised() {
        (
            SyncVerdict::Unknown,
            "kernel".to_owned(),
            format!(
                "the kernel says synchronised (maxerror {maxerror}) but {unseen}: whether it is \
                 disciplined to a PTP hardware clock is unknown"
            ),
        )
    } else if k.maxerror_kept() {
        (
            SyncVerdict::Unknown,
            "kernel".to_owned(),
            format!(
                "the kernel's STA_UNSYNC is set but its maxerror ({maxerror}) is being kept, and \
                 {unseen}; chronyd leaves STA_UNSYNC set without `rtcsync`"
            ),
        )
    } else {
        (
            SyncVerdict::Unsynchronised,
            "-".to_owned(),
            "no time daemon is visible and the kernel says unsynchronised (STA_UNSYNC, maxerror \
             at its 16 s limit)"
                .to_owned(),
        )
    }
}

/// The best bound known for the verdict: `ClockBound`'s where it trusts it,
/// chronyd's where it is synchronised, else the kernel's `maxerror` where
/// the daemon seen keeps it (chronyd, ntpd; not systemd-timesyncd or
/// phc2sys, which zero it). None for an unsynchronised clock.
fn bound(o: &Observed, verdict: SyncVerdict) -> Option<Bound> {
    if verdict == SyncVerdict::Unsynchronised {
        return None;
    }
    if let ClockBoundAnswer::Reading(r) = &o.clockbound
        && r.status.trusted()
    {
        return Some(Bound {
            ns: r.bound_ns,
            source: BoundSource::ClockBound,
        });
    }
    if let ChronyAnswer::Tracking(t) = &o.chrony
        && t.synchronised()
    {
        return Some(Bound {
            ns: t.bound_ns(),
            source: BoundSource::Chrony,
        });
    }
    let keeps_maxerror = o.runs(TimeDaemon::Chronyd) || o.runs(TimeDaemon::Ntpd);
    let zeroes_maxerror = o.runs(TimeDaemon::SystemdTimesyncd) || o.runs(TimeDaemon::Phc2sys);
    match o.kernel {
        Some(k)
            if keeps_maxerror && !zeroes_maxerror && !k.unsynchronised() && k.maxerror_kept() =>
        {
            Some(Bound {
                ns: u64::try_from(k.maxerror_us)
                    .unwrap_or(0)
                    .saturating_mul(1_000),
                source: BoundSource::KernelMaxerror,
            })
        }
        _ => None,
    }
}

/// The fix for a wall clock that is not PTP-grade: for the PHC the host
/// has (Amazon's ENA, Hyper-V, another), or, with none, both clouds'.
fn fix(o: &Observed) -> String {
    const AWS: &str = "chrony `refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer`";
    const AZURE: &str = "chrony `refclock PHC /dev/ptp_hyperv poll 3 dpoll -2 offset 0`";
    let ena = o.ptp_clocks.iter().find(|p| p.name.starts_with("ena"));
    let hyperv = o.ptp_clocks.iter().find(|p| p.name == "hyperv");
    match (ena, hyperv, o.ptp_clocks.first()) {
        (Some(p), _, _) => format!(
            "A PTP hardware clock is present ({}: {}): on AWS, {AWS} (keeping 169.254.169.123 as \
             a backup) disciplines CLOCK_REALTIME to it.",
            p.device, p.name
        ),
        (None, Some(p), _) => format!(
            "A PTP hardware clock is present ({}: {}): on Azure, {AZURE} disciplines \
             CLOCK_REALTIME to it.",
            p.device, p.name
        ),
        (None, None, Some(p)) => format!(
            "A PTP hardware clock is present ({}: {}): chrony `refclock PHC /dev/{}` (or \
             linuxptp's ptp4l and phc2sys) disciplines CLOCK_REALTIME to it.",
            p.device, p.name, p.device
        ),
        (None, None, None) => format!(
            "No PTP hardware clock is present. On AWS: an instance type with PHC support (EC2 \
             `PhcSupport`), the ENA PHC enabled (the in-tree ENA from Linux 6.17: devlink \
             `enable_phc`; Amazon's driver: `phc_enable=1`) and {AWS}; on Azure: {AZURE}."
        ),
    }
}

/// The finding for a verdict: info when PTP-grade or unknown (what cannot
/// be read is never a warning), else a warning with its effect and fix.
fn finding(t: &TimeSync) -> Finding {
    const COUNTER: &str = "UTC stamps are only as good as CLOCK_REALTIME. The default clock \
                           follows it, so its durations include the host's rate corrections; \
                           raw ticks are not affected.";
    let bound = t
        .bound
        .map(|b| format!("error bound {} ({})", human_ns(b.ns), b.source.name()));
    let (severity, message) = match t.verdict {
        SyncVerdict::PtpGrade => {
            let bound = bound.unwrap_or_else(|| {
                if t.source == "phc2sys" {
                    "no error bound (phc2sys keeps no maxerror for the kernel to report)".to_owned()
                } else {
                    "no error bound known".to_owned()
                }
            });
            (
                Severity::Info,
                format!(
                    "CLOCK_REALTIME is disciplined to a PTP hardware clock: {}; {bound}",
                    t.reason
                ),
            )
        }
        SyncVerdict::NtpDisciplined => {
            let effect = if bound.is_some() {
                "wall-clock stamps can be off by up to that bound and drift between \
                 corrections, so cross-host one-way latencies are not reliable below it"
            } else {
                "wall-clock stamps can be off by an unknown amount and drift between \
                 corrections, so cross-host one-way latencies are not reliable"
            };
            let timesyncd = if t.observed.runs(TimeDaemon::SystemdTimesyncd) {
                " systemd-timesyncd is SNTP: it keeps no error bound (it zeroes the kernel's \
                 maxerror at every update) and slews CLOCK_REALTIME through the kernel's PLL, by \
                 hundreds of ppm after a boot (CLOCK_MONOTONIC moved up to 533 ppm from \
                 CLOCK_MONOTONIC_RAW over 120 s on AWS: ergon's clock/BENCHMARKS.md)."
            } else {
                ""
            };
            (
                Severity::Warn,
                format!(
                    "CLOCK_REALTIME is not disciplined to a PTP hardware clock ({}; {}): \
                     {effect}.{timesyncd} {COUNTER} {}",
                    t.reason,
                    bound.unwrap_or_else(|| "no error bound".to_owned()),
                    fix(&t.observed)
                ),
            )
        }
        SyncVerdict::Unsynchronised => (
            Severity::Warn,
            format!(
                "CLOCK_REALTIME is not synchronised ({}): wall-clock stamps have no error bound \
                 and can be off by any amount, so cross-host one-way latencies are meaningless. \
                 {COUNTER} {}",
                t.reason,
                fix(&t.observed)
            ),
        ),
        SyncVerdict::Unknown => (
            Severity::Info,
            format!(
                "time sync unknown: {}{}",
                t.reason,
                bound.map(|b| format!("; {b}")).unwrap_or_default()
            ),
        ),
    };
    Finding::new(Check::TimeSync, severity, message)
}

/// Where and how long to look ([`TimeSyncProbe::run`]): the defaults read
/// this host; a test points them at fixtures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeSyncProbe {
    /// procfs: each `<pid>/comm` names a process.
    pub proc_dir: PathBuf,
    /// The PTP hardware clocks' class directory.
    pub ptp_dir: PathBuf,
    /// chronyd's command port; `None`: not asked.
    pub chrony: Option<SocketAddr>,
    /// How long chronyd has to answer: 50 ms (on the loopback it answers
    /// in well under a millisecond, and a port nobody listens on is refused
    /// at once).
    pub chrony_timeout: Duration,
    /// `ClockBound` 2.0's segment; `None`: not looked for.
    pub clockbound: Option<PathBuf>,
    /// The most `/proc` entries read (a scan of a host with many processes
    /// stays bounded): 65,536.
    pub max_processes: usize,
}

impl Default for TimeSyncProbe {
    fn default() -> Self {
        Self {
            proc_dir: PathBuf::from("/proc"),
            ptp_dir: PathBuf::from("/sys/class/ptp"),
            chrony: Some(SocketAddr::from((Ipv4Addr::LOCALHOST, chrony::PORT))),
            chrony_timeout: Duration::from_millis(50),
            clockbound: Some(PathBuf::from(clockbound::DEFAULT_PATH)),
            max_processes: 65_536,
        }
    }
}

impl TimeSyncProbe {
    /// Reads the host and judges it: file reads, one `adjtimex`, one UDP
    /// exchange of at most [`chrony_timeout`](Self::chrony_timeout).
    /// Allocates. A start-up or housekeeping call, never a hot path.
    #[must_use]
    pub fn run(&self) -> TimeSync {
        let started = Instant::now();
        let mut sync = TimeSync::from_observed(self.observe());
        sync.took = started.elapsed();
        sync
    }

    /// What the host says, unjudged. Off Linux nothing is read.
    #[must_use]
    pub fn observe(&self) -> Observed {
        if !cfg!(target_os = "linux") {
            return Observed::not_linux();
        }
        Observed {
            linux: true,
            kernel: kernel_sync().ok(),
            daemons: self.daemons(),
            ptp_clocks: self.ptp_clocks(),
            chrony: self.chrony(),
            clockbound: self.clockbound(),
        }
    }

    /// The time daemons running, each once, by their `comm` in
    /// `proc_dir`: `None` where it cannot be listed. A process that exits
    /// mid-scan is passed over, and so is a zombie (its `stat` state `Z` or
    /// `X`: exited, not yet reaped), which still has a `comm`.
    #[must_use]
    pub fn daemons(&self) -> Option<Vec<TimeDaemon>> {
        use std::io::Read;
        let entries = std::fs::read_dir(&self.proc_dir).ok()?;
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

    /// The PTP hardware clocks in `ptp_dir`, by device: none where there is
    /// no such directory (no PTP support, or not Linux).
    #[must_use]
    pub fn ptp_clocks(&self) -> Vec<PtpClock> {
        let Ok(entries) = std::fs::read_dir(&self.ptp_dir) else {
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

    /// `ClockBound`'s bound now, from [`clockbound`](Self::clockbound).
    #[must_use]
    pub fn clockbound(&self) -> ClockBoundAnswer {
        let Some(path) = &self.clockbound else {
            return ClockBoundAnswer::NotAsked;
        };
        read_clockbound(path)
    }
}

/// Whether the process at `pid_dir` has exited but is still listed: its
/// `stat`'s state (the field after the parenthesised `comm`, which may itself
/// hold a `)`) is `Z` (zombie) or `X` (dead). One read into a buffer on the
/// stack, only for a process whose `comm` names a time daemon; an unreadable
/// `stat` is no evidence either way, so the process counts.
fn exited(pid_dir: &Path) -> bool {
    use std::io::Read;
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

/// `ClockBound`'s segment at `path`, read and parsed against
/// `CLOCK_MONOTONIC_COARSE` (the clock its daemon stamps with; elsewhere
/// the monotonic clock).
fn read_clockbound(path: &Path) -> ClockBoundAnswer {
    match clockbound::read(path) {
        Ok(None) => ClockBoundAnswer::Absent,
        Err(e) => ClockBoundAnswer::Unusable(e),
        Ok(Some(segment)) => {
            #[cfg(target_os = "linux")]
            let now = crate::arch::clock_ns(libc::CLOCK_MONOTONIC_COARSE);
            #[cfg(not(target_os = "linux"))]
            let now = crate::arch::clock_ns(crate::wall::MONOTONIC);
            match now {
                Err(errno) => ClockBoundAnswer::Unusable(format!(
                    "CLOCK_MONOTONIC_COARSE could not be read (errno {errno})"
                )),
                Ok(now) => match clockbound::parse(&segment, now) {
                    Ok(reading) => ClockBoundAnswer::Reading(reading),
                    Err(why) => ClockBoundAnswer::Unusable(why.to_owned()),
                },
            }
        }
    }
}

/// A request's sequence number: this process and the raw clock's low bits,
/// so a reply to another process's or an earlier request is told apart.
fn sequence() -> u32 {
    let raw = crate::wall::monotonic_raw_ns();
    #[allow(clippy::cast_possible_truncation)] // the low bits, on purpose
    let low = raw as u32;
    low ^ std::process::id().rotate_left(16)
}

/// The kernel's view of the wall clock: Linux's `adjtimex` with `modes` 0
/// (a read, unprivileged; under a microsecond).
///
/// # Errors
/// [`Error::ClockRead`] for `adjtimex`: the call failed (its error number),
/// or not Linux (`ENOSYS`).
pub fn kernel_sync() -> Result<KernelSync, Error> {
    KernelSync::read().map_err(|errno| Error::ClockRead {
        clock: "adjtimex",
        errno,
    })
}

/// How `CLOCK_REALTIME` is kept on this host now: [`TimeSyncProbe`]'s defaults.
///
/// For a long-running process's housekeeping (every minute or so: the
/// verdict changes when a daemon starts, stops, locks or loses its source),
/// never a hot path.
#[must_use]
pub fn time_sync() -> TimeSync {
    TimeSyncProbe::default().run()
}

/// A duration in ns as a person reads it: `850 ns`, `12.5 µs`, `1.20 ms`,
/// `2.0 s`.
#[must_use]
pub fn human_ns(ns: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let f = ns as f64;
    if ns < 1_000 {
        format!("{ns} ns")
    } else if ns < 1_000_000 {
        format!("{:.1} µs", f / 1e3)
    } else if ns < 1_000_000_000 {
        format!("{:.2} ms", f / 1e6)
    } else {
        format!("{:.1} s", f / 1e9)
    }
}

/// The reference chronyd reports, as a source name without spaces.
fn chrony_source(t: &ChronyTracking) -> String {
    match &t.reference {
        ChronyReference::Local => "chronyd:local".to_owned(),
        ChronyReference::None => "chronyd".to_owned(),
        _ => format!("chronyd:{}", t.reference_name()),
    }
}
