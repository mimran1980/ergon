//! T11: how `CLOCK_REALTIME` is kept, judged from what was read. Pure: no
//! I/O. One set of rules, [`judge`], decides the verdict for the start-up
//! check and for [`ClockWatch`](crate::ClockWatch) alike, and allocates
//! nothing; the words are rendered from its answer only when findings are
//! made.

use std::fmt;

use super::chrony::{ChronyError, ChronyReference, ChronyTracking};
use super::clockbound::{ClockBoundError, ClockBoundReading};
use super::kernel::KernelSync;
use crate::finding::{CheckId, Finding, Severity};

/// Where the T11 findings' claims come from.
const T11_EVIDENCE: &str = "Linux kernel/time/ntp.c (adjtimex: STA_UNSYNC, maxerror, leap \
     seconds); chrony 4.6.1 doc/chronyc.adoc (tracking) and doc/chrony.conf.adoc (refclock, \
     leapsecmode); systemd timesyncd-manager.c (manager_adjust_clock); clock_gettime(2); the \
     AWS EC2 User Guide and Azure documentation (chrony on the PTP hardware clock); not measured \
     here";

/// What every warning about the wall clock says UTC stamps and durations
/// rest on.
const STAMPS: &str = "UTC stamps are only as good as CLOCK_REALTIME. CLOCK_MONOTONIC is slewed \
                      with it (clock_gettime(2)), so its durations include the rate correction \
                      in force, and a counter clock calibrated against it at start-up (quanta) \
                      keeps the correction in force while it calibrated.";

/// How `CLOCK_REALTIME` is kept, as far as this process can see.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Its name: `ptp-grade`, `ntp-disciplined`, `unsynchronised` or
    /// `unknown`.
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

/// A time daemon recognised by its process name (`comm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PtpClock {
    /// The device, `ptp0`: `/dev/ptp0`.
    pub device: String,
    /// The driver's name for it: `hyperv` on Azure, `ena-ptp-*` for AWS's
    /// ENA, `mlx5_ptp` and the like for other NICs.
    pub name: String,
}

/// What chronyd's command port said. A plain value: no allocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChronyAnswer {
    /// Not asked (not Linux, or no address configured).
    NotAsked,
    /// Its tracking report.
    Tracking(ChronyTracking),
    /// No report: why.
    Failed(ChronyError),
}

/// What `ClockBound`'s segment said. A plain value: no allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockBoundAnswer {
    /// Not looked for.
    NotAsked,
    /// No segment: `ClockBound` 2.0 is not running here.
    Absent,
    /// Its bound.
    Reading(ClockBoundReading),
    /// A segment that could not be used: why.
    Unusable(ClockBoundError),
}

/// Everything read about the wall clock's discipline, before judging it:
/// what a [`TimeSyncProbe`](crate::TimeSyncProbe) observes, or a test's
/// fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct Observed {
    /// Whether this is Linux, where all of it can be read.
    pub linux: bool,
    /// The kernel's state, where `adjtimex` could be read.
    pub kernel: Option<KernelSync>,
    /// The time daemons seen running; `None` where `/proc` could not be
    /// listed.
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

    /// chronyd's report, if it gave one.
    const fn tracking(&self) -> Option<&ChronyTracking> {
        match &self.chrony {
            ChronyAnswer::Tracking(t) => Some(t),
            _ => None,
        }
    }
}

/// Where an error bound came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoundSource {
    /// `ClockBound` 2.0's segment.
    ClockBound,
    /// chronyd's tracking report: `|offset| + root dispersion + root delay / 2`.
    Chrony,
    /// The kernel's `maxerror`, as chronyd or ntpd keep it.
    KernelMaxerror,
}

impl BoundSource {
    /// Its name: `clockbound`, `chrony` or `kernel-maxerror`.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bound {
    /// The bound, ns.
    pub ns: u64,
    /// Where it came from.
    pub source: BoundSource,
}

/// What T11 warns over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimeSyncLimits {
    /// A correction still to come, a last update or an error bound over
    /// this many ns warns: 1 ms by default.
    pub warn_over_ns: u64,
}

impl Default for TimeSyncLimits {
    fn default() -> Self {
        Self {
            warn_over_ns: 1_000_000,
        }
    }
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
}

impl TimeSync {
    /// The verdict on what was observed, by the rules
    /// [`ClockWatch`](crate::ClockWatch) applies to every check too.
    #[must_use]
    pub fn from_observed(observed: Observed) -> Self {
        let why = judge(&observed);
        let verdict = why.verdict();
        let (source, reason) = explain(why, &observed);
        let bound = bound(&observed, verdict);
        Self {
            verdict,
            source,
            reason,
            bound,
            observed,
        }
    }

    /// The T11 findings:
    ///
    /// - always one for the verdict: ok when PTP-grade, a warning when
    ///   NTP-disciplined or unsynchronised, unknown (never a warning) when it
    ///   cannot be told;
    /// - unless unsynchronised, a warning when the clock is still being
    ///   corrected by more than [`TimeSyncLimits::warn_over_ns`] (or its last
    ///   update moved it by more), else when the error bound is wider than
    ///   that and the verdict's own finding does not warn already;
    /// - a warning when a leap second is pending.
    ///
    /// Allocates.
    #[must_use]
    pub fn findings(&self, limits: &TimeSyncLimits) -> Vec<Finding> {
        let o = &self.observed;
        let mut out = vec![self.verdict_finding()];
        if self.verdict != SyncVerdict::Unsynchronised {
            if let Some(u) = unsettled(o, limits.warn_over_ns) {
                out.push(u.finding());
            } else if let Some(b) = self.bound.filter(|b| b.ns > limits.warn_over_ns)
                && self.verdict != SyncVerdict::NtpDisciplined
            {
                out.push(Finding::found(
                    CheckId::T11,
                    Severity::Warn,
                    T11_EVIDENCE,
                    format!(
                        "CLOCK_REALTIME is known to UTC only within {} ({}): stamps compared \
                         with other hosts can differ by that much",
                        human_ns(b.ns),
                        b.source.name()
                    ),
                ));
            }
        }
        if let Some(source) = leap_pending(o) {
            out.push(Finding::found(
                CheckId::T11,
                Severity::Warn,
                T11_EVIDENCE,
                format!(
                    "a leap second is pending ({source}): at midnight UTC the kernel steps \
                     CLOCK_REALTIME back 1 s for an inserted second (forward for a deleted one), \
                     unless the daemon slews it instead (chrony `leapsecmode slew`), so UTC \
                     stamps around it repeat, jump or are off by up to 1 s. Measure durations \
                     with a monotonic clock, never by subtracting CLOCK_REALTIME stamps."
                ),
            ));
        }
        out
    }

    /// The verdict's finding.
    fn verdict_finding(&self) -> Finding {
        let o = &self.observed;
        let bound = self
            .bound
            .map(|b| format!("error bound {} ({})", human_ns(b.ns), b.source.name()));
        match self.verdict {
            SyncVerdict::PtpGrade => {
                let bound = bound.unwrap_or_else(|| {
                    if self.source == "phc2sys" {
                        "no error bound (phc2sys keeps no maxerror for the kernel to report)"
                            .to_owned()
                    } else {
                        "no error bound known".to_owned()
                    }
                });
                Finding::ok(
                    CheckId::T11,
                    Severity::Warn,
                    T11_EVIDENCE,
                    format!(
                        "CLOCK_REALTIME is disciplined to a PTP hardware clock: {}; {bound}",
                        self.reason
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
                let timesyncd = if o.runs(TimeDaemon::SystemdTimesyncd) {
                    " systemd-timesyncd is SNTP: it keeps no error bound (it sets the kernel's \
                     maxerror to 0 at every update) and slews CLOCK_REALTIME through the \
                     kernel's PLL."
                } else {
                    ""
                };
                Finding::found(
                    CheckId::T11,
                    Severity::Warn,
                    T11_EVIDENCE,
                    format!(
                        "CLOCK_REALTIME is not disciplined to a PTP hardware clock ({}; {}): \
                         {effect}.{timesyncd} {STAMPS} {}",
                        self.reason,
                        bound.unwrap_or_else(|| "no error bound".to_owned()),
                        fix(o)
                    ),
                )
            }
            SyncVerdict::Unsynchronised => Finding::found(
                CheckId::T11,
                Severity::Warn,
                T11_EVIDENCE,
                format!(
                    "CLOCK_REALTIME is not synchronised ({}): wall-clock stamps have no error \
                     bound and can be off by any amount, so cross-host one-way latencies are \
                     meaningless. {STAMPS} {}",
                    self.reason,
                    fix(o)
                ),
            ),
            SyncVerdict::Unknown => Finding::unknown(
                CheckId::T11,
                Severity::Warn,
                T11_EVIDENCE,
                "how CLOCK_REALTIME is kept is unknown".to_owned(),
                bound.map_or_else(|| self.reason.clone(), |b| format!("{}; {b}", self.reason)),
            ),
        }
    }
}

/// Why the verdict: one variant per rule of [`judge`], so the verdict is a
/// plain value and its words ([`explain`]) are made only when needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Why {
    /// Not Linux: nothing read.
    NotLinux,
    /// chronyd says it is synchronised but leaves `maxerror` at the limit.
    ChronydNotAdjusting,
    /// chronyd locked to a PHC reference clock.
    ChronydPhc,
    /// chronyd locked to a reference clock not named as a PHC.
    ChronydRefclock,
    /// chronyd tracking an NTP server.
    ChronydServer,
    /// phc2sys beside another client that adjusts the clock.
    Phc2sysRival,
    /// phc2sys with the kernel synchronised.
    Phc2sysSynced,
    /// phc2sys with the kernel unsynchronised.
    Phc2sysUnsynced,
    /// phc2sys, and the kernel's state unread.
    Phc2sysKernelUnread,
    /// chronyd serving its own clock.
    ChronydLocal,
    /// chronyd synchronised to nothing.
    ChronydNoSource,
    /// No report, and the kernel's state unread.
    KernelUnread,
    /// chronyd silent, the kernel synchronised, no PHC present.
    ChronydSilentNoPhc,
    /// chronyd silent, the kernel synchronised, a PHC present.
    ChronydSilentPhcPresent,
    /// chronyd silent, `STA_UNSYNC` set, `maxerror` kept.
    ChronydSilentUnsyncKept,
    /// chronyd silent, `STA_UNSYNC` set, `maxerror` at the limit.
    ChronydSilentUnsync,
    /// systemd-timesyncd with a local-time RTC (`STA_UNSYNC`, `maxerror`
    /// kept).
    TimesyncdLocalRtc,
    /// ntpd or systemd-timesyncd, the kernel unsynchronised.
    DaemonUnsynced(TimeDaemon),
    /// ntpd or systemd-timesyncd, the kernel synchronised.
    DaemonSynced(TimeDaemon),
    /// ptp4l without phc2sys.
    Ptp4lAlone,
    /// No daemon visible, the kernel synchronised.
    UnseenSynced,
    /// No daemon visible, `STA_UNSYNC` set, `maxerror` kept.
    UnseenUnsyncKept,
    /// No daemon visible, `STA_UNSYNC` set, `maxerror` at the limit.
    UnseenUnsynced,
}

impl Why {
    /// The verdict this rule gives.
    pub const fn verdict(self) -> SyncVerdict {
        match self {
            Self::ChronydPhc | Self::Phc2sysSynced => SyncVerdict::PtpGrade,
            Self::ChronydRefclock
            | Self::ChronydServer
            | Self::ChronydSilentNoPhc
            | Self::TimesyncdLocalRtc
            | Self::DaemonSynced(_) => SyncVerdict::NtpDisciplined,
            Self::ChronydNotAdjusting
            | Self::Phc2sysUnsynced
            | Self::ChronydLocal
            | Self::ChronydNoSource
            | Self::ChronydSilentUnsync
            | Self::DaemonUnsynced(_)
            | Self::UnseenUnsynced => SyncVerdict::Unsynchronised,
            Self::NotLinux
            | Self::Phc2sysRival
            | Self::Phc2sysKernelUnread
            | Self::KernelUnread
            | Self::ChronydSilentPhcPresent
            | Self::ChronydSilentUnsyncKept
            | Self::Ptp4lAlone
            | Self::UnseenSynced
            | Self::UnseenUnsyncKept => SyncVerdict::Unknown,
        }
    }
}

/// The rules. chronyd's own word first (it says what it is locked to), then
/// phc2sys (which keeps no `maxerror`, so only the kernel's `STA_UNSYNC`
/// says whether it has locked), then the kernel's state as the daemon seen
/// keeps it; what cannot be told apart is unknown, never a warning.
/// Allocates nothing.
pub fn judge(o: &Observed) -> Why {
    if !o.linux {
        return Why::NotLinux;
    }
    let tracking = o.tracking();
    if let Some(t) = tracking
        && t.synchronised()
    {
        // chronyd sets maxerror to its root distance at every update
        // (sys_timex.c `set_sync_status`): one left at the kernel's limit
        // means it is not adjusting this clock (`chronyd -x`)
        if let Some(k) = &o.kernel
            && !k.maxerror_kept()
        {
            return Why::ChronydNotAdjusting;
        }
        return match t.reference {
            ChronyReference::Refclock(_) if t.is_phc() => Why::ChronydPhc,
            ChronyReference::Refclock(_) => Why::ChronydRefclock,
            _ => Why::ChronydServer,
        };
    }
    if o.runs(TimeDaemon::Phc2sys) {
        let rival = rivals(o).next().is_some();
        return match &o.kernel {
            Some(k) if rival && (!k.unsynchronised() || k.maxerror_kept()) => Why::Phc2sysRival,
            Some(k) if !k.unsynchronised() => Why::Phc2sysSynced,
            Some(_) => Why::Phc2sysUnsynced,
            None => Why::Phc2sysKernelUnread,
        };
    }
    if let Some(t) = tracking {
        return if t.reference == ChronyReference::Local {
            Why::ChronydLocal
        } else {
            Why::ChronydNoSource
        };
    }
    let Some(k) = &o.kernel else {
        return Why::KernelUnread;
    };
    let daemon = [
        TimeDaemon::Chronyd,
        TimeDaemon::Ntpd,
        TimeDaemon::SystemdTimesyncd,
        TimeDaemon::Ptp4l,
    ]
    .into_iter()
    .find(|d| o.runs(*d));
    match daemon {
        Some(TimeDaemon::Chronyd) => {
            if !k.unsynchronised() {
                if o.ptp_clocks.is_empty() {
                    Why::ChronydSilentNoPhc
                } else {
                    Why::ChronydSilentPhcPresent
                }
            } else if k.maxerror_kept() {
                Why::ChronydSilentUnsyncKept
            } else {
                Why::ChronydSilentUnsync
            }
        }
        // it zeroes maxerror at every update, and sets STA_UNSYNC itself
        // where the RTC keeps local time (timesyncd-manager.c
        // `manager_adjust_clock`)
        Some(TimeDaemon::SystemdTimesyncd) if k.unsynchronised() && k.maxerror_kept() => {
            Why::TimesyncdLocalRtc
        }
        Some(d @ (TimeDaemon::Ntpd | TimeDaemon::SystemdTimesyncd)) => {
            if k.unsynchronised() {
                Why::DaemonUnsynced(d)
            } else {
                Why::DaemonSynced(d)
            }
        }
        Some(_) => Why::Ptp4lAlone,
        None => {
            if !k.unsynchronised() {
                Why::UnseenSynced
            } else if k.maxerror_kept() {
                Why::UnseenUnsyncKept
            } else {
                Why::UnseenUnsynced
            }
        }
    }
}

/// The clients beside phc2sys that also adjust `CLOCK_REALTIME`:
/// systemd-timesyncd, ntpd, and a chronyd that did not answer (one that
/// answered unsynchronised adjusts nothing).
fn rivals(o: &Observed) -> impl Iterator<Item = TimeDaemon> + '_ {
    let answered = o.tracking().is_some();
    [
        TimeDaemon::SystemdTimesyncd,
        TimeDaemon::Ntpd,
        TimeDaemon::Chronyd,
    ]
    .into_iter()
    .filter(move |d| o.runs(*d) && !(*d == TimeDaemon::Chronyd && answered))
}

/// What keeps the clock (no spaces) and why, in words, for a rule's
/// answer. Allocates.
#[allow(clippy::too_many_lines)] // one arm per rule of `judge`, exhaustive
fn explain(why: Why, o: &Observed) -> (String, String) {
    let tracking = o.tracking();
    let chrony = || tracking.map_or_else(|| "chronyd".to_owned(), chrony_source);
    let reference = || tracking.map_or_else(|| "-".to_owned(), ChronyTracking::reference_name);
    let stratum = tracking.map_or(0, |t| t.stratum);
    let maxerror = o
        .kernel
        .map_or_else(|| "-".to_owned(), |k| human_ns(k.maxerror_ns()));
    let silent = match &o.chrony {
        ChronyAnswer::Failed(e) => format!(" ({e})"),
        _ => String::new(),
    };
    let unseen = if o.daemons.is_none() {
        "the daemon keeping it is not visible: /proc could not be listed"
    } else {
        "the daemon keeping it is not visible to this process (it may run in another PID \
         namespace, /proc may be mounted with hidepid, or it is one this crate does not know)"
    };
    match why {
        Why::NotLinux => (
            "-".to_owned(),
            "not Linux: no adjtimex, PTP hardware clock or chronyd command port is read here"
                .to_owned(),
        ),
        Why::ChronydNotAdjusting => (
            chrony(),
            format!(
                "chronyd reports itself synchronised to {} but does not adjust this kernel's \
                 clock (its maxerror is at the 16 s limit, where chronyd sets it at every \
                 update: chronyd -x)",
                reference()
            ),
        ),
        Why::ChronydPhc => (
            chrony(),
            format!(
                "chronyd is locked to reference clock {}, a PTP hardware clock",
                reference()
            ),
        ),
        Why::ChronydRefclock => (
            chrony(),
            format!(
                "chronyd is locked to reference clock {}, not named as a PTP hardware clock \
                 (chrony names `refclock PHC` PHC0, PHC1, ...)",
                reference()
            ),
        ),
        Why::ChronydServer => (
            chrony(),
            format!(
                "chronyd tracks NTP server {} at stratum {stratum}",
                reference()
            ),
        ),
        Why::Phc2sysRival => (
            "phc2sys".to_owned(),
            format!(
                "phc2sys is running beside {}, which also disciplines CLOCK_REALTIME: which of \
                 them keeps it is unknown (phc2sys may discipline another clock, or feed it \
                 through a shared-memory reference clock), and two that both adjust it fight \
                 over it",
                rivals(o)
                    .map(TimeDaemon::name)
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
        ),
        Why::Phc2sysSynced => (
            "phc2sys".to_owned(),
            "phc2sys is running and the kernel says CLOCK_REALTIME is synchronised".to_owned(),
        ),
        Why::Phc2sysUnsynced => (
            "phc2sys".to_owned(),
            "phc2sys is running but the kernel says CLOCK_REALTIME is unsynchronised \
             (STA_UNSYNC): it has not locked, or it disciplines another clock"
                .to_owned(),
        ),
        Why::Phc2sysKernelUnread => (
            "phc2sys".to_owned(),
            "phc2sys is running but the kernel's state (adjtimex) could not be read".to_owned(),
        ),
        Why::ChronydLocal => (
            chrony(),
            format!(
                "chronyd serves its own clock (local stratum {stratum}) and is synchronised to \
                 no source"
            ),
        ),
        Why::ChronydNoSource => (chrony(), "chronyd is synchronised to no source".to_owned()),
        Why::KernelUnread => (
            "-".to_owned(),
            "the kernel's state (adjtimex) could not be read".to_owned(),
        ),
        Why::ChronydSilentNoPhc => (
            "chronyd".to_owned(),
            format!(
                "chronyd is running and the kernel says synchronised (maxerror {maxerror}); it \
                 did not answer on its command port{silent}, and no PTP hardware clock is \
                 present"
            ),
        ),
        Why::ChronydSilentPhcPresent => {
            let (device, name) = o
                .ptp_clocks
                .first()
                .map_or(("-", "-"), |p| (p.device.as_str(), p.name.as_str()));
            (
                "chronyd".to_owned(),
                format!(
                    "chronyd is running and a PTP hardware clock is present ({device}: {name}), \
                     but chronyd did not answer on its command port{silent}: whether it is \
                     locked to the PHC is unknown"
                ),
            )
        }
        Why::ChronydSilentUnsyncKept => (
            "chronyd".to_owned(),
            format!(
                "chronyd is running and keeps the kernel's maxerror ({maxerror}) but did not \
                 answer on its command port{silent}, and STA_UNSYNC is set, which chronyd leaves \
                 set without `rtcsync`"
            ),
        ),
        Why::ChronydSilentUnsync => (
            "chronyd".to_owned(),
            format!(
                "chronyd is running but the kernel says unsynchronised (STA_UNSYNC, maxerror at \
                 its 16 s limit) and chronyd did not answer on its command port{silent}"
            ),
        ),
        Why::TimesyncdLocalRtc => (
            "systemd-timesyncd".to_owned(),
            format!(
                "systemd-timesyncd is running and keeps the kernel's maxerror ({maxerror}); \
                 STA_UNSYNC is set, as it leaves it where the RTC keeps local time"
            ),
        ),
        Why::DaemonUnsynced(d) => (
            d.name().to_owned(),
            format!(
                "{} is running but the kernel says unsynchronised (STA_UNSYNC): not \
                 synchronised yet",
                d.name()
            ),
        ),
        Why::DaemonSynced(d) => (
            d.name().to_owned(),
            format!("{} is running and the kernel says synchronised", d.name()),
        ),
        Why::Ptp4lAlone => (
            "ptp4l".to_owned(),
            "ptp4l is running without phc2sys: it disciplines a NIC's PTP hardware clock, and \
             CLOCK_REALTIME only with software timestamping (-S)"
                .to_owned(),
        ),
        Why::UnseenSynced => (
            "kernel".to_owned(),
            format!(
                "the kernel says synchronised (maxerror {maxerror}) but {unseen}: whether it is \
                 disciplined to a PTP hardware clock is unknown"
            ),
        ),
        Why::UnseenUnsyncKept => (
            "kernel".to_owned(),
            format!(
                "the kernel's STA_UNSYNC is set but its maxerror ({maxerror}) is being kept, and \
                 {unseen}; chronyd leaves STA_UNSYNC set without `rtcsync`"
            ),
        ),
        Why::UnseenUnsynced => (
            "-".to_owned(),
            "no time daemon is visible and the kernel says unsynchronised (STA_UNSYNC, maxerror \
             at its 16 s limit)"
                .to_owned(),
        ),
    }
}

/// The best bound known for the verdict: `ClockBound`'s where it trusts it,
/// chronyd's where it is synchronised, else the kernel's `maxerror` where
/// the daemon seen keeps it (chronyd, ntpd; not systemd-timesyncd or
/// phc2sys, which zero it). None for an unsynchronised clock. Allocates
/// nothing.
pub fn bound(o: &Observed, verdict: SyncVerdict) -> Option<Bound> {
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
    if let Some(t) = o.tracking()
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
                ns: k.maxerror_ns(),
                source: BoundSource::KernelMaxerror,
            })
        }
        _ => None,
    }
}

/// The clock still being corrected by more than the limit, or moved by
/// more at chronyd's last update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unsettled {
    /// The correction still to come, ns, where it is over the limit.
    correction_ns: Option<i64>,
    /// The last update's offset, ns, where it is over the limit.
    last_update_ns: Option<i64>,
    /// chronyd's report, or else the kernel's PLL.
    from_chronyd: bool,
}

impl Unsettled {
    fn finding(self) -> Finding {
        let who = if self.from_chronyd {
            "chronyd"
        } else {
            "the kernel's PLL (adjtimex offset)"
        };
        let message = match (self.correction_ns, self.last_update_ns) {
            (Some(c), _) => format!(
                "CLOCK_REALTIME is not settled: {who} is still correcting it by {}, so UTC \
                 stamps are off by up to that until it is slewed out. Start once time sync has \
                 settled.",
                human_ns(c.unsigned_abs())
            ),
            (None, last) => format!(
                "CLOCK_REALTIME is not settled: {who}'s last update moved it by {}; time sync \
                 may not have settled. Start once it has.",
                human_ns(last.map_or(0, i64::unsigned_abs))
            ),
        };
        Finding::found(CheckId::T11, Severity::Warn, T11_EVIDENCE, message)
    }
}

/// The clock still being corrected by more than `limit_ns` (chronyd's
/// pending correction, else the kernel PLL's offset), or moved by more at
/// chronyd's last update. Allocates nothing.
pub fn unsettled(o: &Observed, limit_ns: u64) -> Option<Unsettled> {
    let tracking = o.tracking();
    let over = |ns: &i64| ns.unsigned_abs() > limit_ns;
    let correction = tracking.map_or_else(
        || o.kernel.map(|k| k.offset_ns),
        |t| Some(seconds_ns(t.correction_s)),
    );
    let correction_ns = correction.filter(over);
    let last_update_ns = tracking.map(|t| seconds_ns(t.last_offset_s)).filter(over);
    if correction_ns.is_none() && last_update_ns.is_none() {
        return None;
    }
    Some(Unsettled {
        correction_ns,
        last_update_ns,
        from_chronyd: tracking.is_some(),
    })
}

/// Who says a leap second is pending: chronyd's leap status (1 insert, 2
/// delete), else the kernel's `STA_INS`, `STA_DEL` or leap state. Allocates
/// nothing.
pub fn leap_pending(o: &Observed) -> Option<&'static str> {
    if o.tracking().is_some_and(ChronyTracking::leap_pending) {
        return Some("chronyd");
    }
    o.kernel
        .filter(KernelSync::leap_pending)
        .map(|_| "adjtimex")
}

/// The fix for a wall clock that is not PTP-grade: for the PHC the host
/// has (Amazon's ENA, Hyper-V, another), or, with none, both clouds'. On
/// the node: a pod shares the node's `CLOCK_REALTIME`.
fn fix(o: &Observed) -> String {
    const AWS: &str = "chrony `refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer`";
    const AZURE: &str = "chrony `refclock PHC /dev/ptp_hyperv poll 3 dpoll -2 offset 0`";
    const NODE: &str = "Fix it on the node (a pod shares the node's CLOCK_REALTIME):";
    let ena = o.ptp_clocks.iter().find(|p| p.name.starts_with("ena"));
    let hyperv = o.ptp_clocks.iter().find(|p| p.name == "hyperv");
    match (ena, hyperv, o.ptp_clocks.first()) {
        (Some(p), _, _) => format!(
            "{NODE} a PTP hardware clock is present ({}: {}); on AWS, {AWS} (keeping \
             169.254.169.123 as a backup) disciplines CLOCK_REALTIME to it.",
            p.device, p.name
        ),
        (None, Some(p), _) => format!(
            "{NODE} a PTP hardware clock is present ({}: {}); on Azure, {AZURE} disciplines \
             CLOCK_REALTIME to it.",
            p.device, p.name
        ),
        (None, None, Some(p)) => format!(
            "{NODE} a PTP hardware clock is present ({}: {}); chrony `refclock PHC /dev/{}` (or \
             linuxptp's ptp4l and phc2sys) disciplines CLOCK_REALTIME to it.",
            p.device, p.name, p.device
        ),
        (None, None, None) => format!(
            "{NODE} no PTP hardware clock is present. On AWS: an instance type with PHC support \
             (EC2 `PhcSupport`), the ENA PHC enabled (the in-tree ENA from Linux 6.17: devlink \
             `enable_phc`; Amazon's driver: `phc_enable=1`) and {AWS}; on Azure: {AZURE}."
        ),
    }
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

/// Seconds as ns, rounded and saturating.
#[allow(clippy::cast_possible_truncation)] // saturating, by `as`
fn seconds_ns(s: f64) -> i64 {
    (s * 1e9).round() as i64
}

/// The reference chronyd reports, as a source name without spaces.
fn chrony_source(t: &ChronyTracking) -> String {
    match &t.reference {
        ChronyReference::Local => "chronyd:local".to_owned(),
        ChronyReference::None => "chronyd".to_owned(),
        _ => format!("chronyd:{}", t.reference_name()),
    }
}

impl fmt::Display for SyncVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every rule's verdict is the one its words describe: the words never
    /// contradict the value.
    #[test]
    fn every_rule_has_one_verdict() {
        assert_eq!(Why::ChronydPhc.verdict(), SyncVerdict::PtpGrade);
        assert_eq!(
            Why::ChronydNotAdjusting.verdict(),
            SyncVerdict::Unsynchronised
        );
        assert_eq!(Why::UnseenSynced.verdict(), SyncVerdict::Unknown);
        assert_eq!(
            Why::DaemonSynced(TimeDaemon::Ntpd).verdict(),
            SyncVerdict::NtpDisciplined
        );
    }

    /// Durations read as a person reads them.
    #[test]
    fn durations_read_as_a_person_reads_them() {
        assert_eq!(human_ns(850), "850 ns");
        assert_eq!(human_ns(12_500), "12.5 µs");
        assert_eq!(human_ns(1_200_000), "1.20 ms");
        assert_eq!(human_ns(2_000_000_000), "2.0 s");
    }

    /// The daemons by their `comm`: `systemd-timesyncd` by its 15 bytes.
    #[test]
    fn daemons_are_known_by_their_comm() {
        assert_eq!(
            TimeDaemon::from_comm("systemd-timesyn\n"),
            Some(TimeDaemon::SystemdTimesyncd)
        );
        assert_eq!(TimeDaemon::from_comm("chronyc"), None);
        assert_eq!(TimeDaemon::SystemdTimesyncd.name(), "systemd-timesyncd");
    }
}
