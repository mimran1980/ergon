//! T11 and T12 while a process runs: [`ClockWatch`].

use std::fs::File;
use std::io;
use std::path::PathBuf;

use super::chrony::ChronyPoll;
use super::clockbound;
use super::clocksource::{CLOCKSOURCE, Clocksource, ClocksourceName};
use super::kernel::KernelSync;
use super::probe::{TimeSyncProbe, clockbound_at, coarse_now, sequence};
use super::sync::{
    Bound, BoundSource, ChronyAnswer, ClockBoundAnswer, Observed, SyncVerdict, TimeSync, Why,
    bound, judge, leap_pending, unsettled,
};
use crate::finding::Finding;
use crate::host::{Host, ReadError};
use crate::platform::Arch;
use crate::quota::read_from_start;

/// One check's fresh reads: what [`ClockWatch::check`] reads, or what a
/// test gives [`ClockWatch::update`]. A plain value: no allocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockSample {
    /// The kernel's state (`adjtimex`), where it was read: the running
    /// Linux host only.
    pub kernel: Option<KernelSync>,
    /// chronyd's answer to the previous check's request.
    pub chrony: ChronyAnswer,
    /// `ClockBound`'s segment, read again.
    pub clockbound: ClockBoundAnswer,
    /// The kernel's clock source, read again, or why it could not be.
    pub clocksource: Result<ClocksourceName, io::ErrorKind>,
}

/// What a change is judged by: the verdict's rule and every input to a
/// T11 or T12 finding, as plain values (numbers that only drift, like a
/// bound's size, count only as over or under the limit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    why: Why,
    chrony_ref: Option<u32>,
    bound: Option<(BoundSource, bool)>,
    unsettled: bool,
    leap: Option<&'static str>,
}

/// T11 and T12 checked again from housekeeping, modelled on
/// [`ThrottleWatch`](crate::ThrottleWatch).
///
/// [`ClockWatch::open`] runs the full T11 probe and T12 once, before the
/// loop: the `/proc` scan for time daemons, the PTP clocks, and one blocking
/// chronyd query of at most [`TimeSyncProbe::chrony_timeout`]. It keeps a
/// non-blocking socket to chronyd, the daemon and PTP lists (they are not
/// read again) and the clock source file open.
///
/// [`ClockWatch::check`], every few seconds from idle-deferred housekeeping
/// (never inside the loop's hot path), never waits on anything:
///
/// - one `adjtimex` (the running Linux host only);
/// - one non-blocking receive of chronyd's reply to the previous check's
///   request, then one send of the next request: chronyd's answer is used
///   one check late;
/// - `ClockBound`'s segment opened (so a daemon started or restarted since
///   is seen) and read from its start twice;
/// - the clock source read from its start.
///
/// Every sample is judged by the start-up check's own rules
/// ([`TimeSync::from_observed`]). While nothing that decides a finding
/// changes, `check` returns an empty `Vec` and allocates nothing (for paths
/// under a few hundred bytes, which std converts on the stack). When
/// something does, it renders the findings again: every T11 finding when
/// the T11 judgement changed, the T12 finding when the clock source did.
#[derive(Debug)]
pub struct ClockWatch {
    probe: TimeSyncProbe,
    /// The running Linux host: `adjtimex` and its coarse clock are read.
    live_linux: bool,
    observed: Observed,
    last: ClockSample,
    key: Key,
    verdict: SyncVerdict,
    bound: Option<Bound>,
    chrony: Option<ChronyPoll>,
    clockbound: Option<PathBuf>,
    clocksource: Result<File, io::ErrorKind>,
    arch: Arch,
    invariant_tsc: Option<bool>,
}

impl ClockWatch {
    /// T11 and T12 on `host`, and the watch that checks them again. Start-up
    /// only: allocates, and waits at most `probe.chrony_timeout` for
    /// chronyd.
    #[must_use]
    pub fn open(host: &Host, probe: &TimeSyncProbe) -> (Self, Vec<Finding>) {
        let observed = probe.observe(host);
        let mut findings = TimeSync::from_observed(observed.clone()).findings(&probe.limits);
        let source = Clocksource::read(host);
        findings.push(source.finding());
        let live_linux = host.is_live() && observed.linux;
        let chrony = if observed.linux {
            probe
                .chrony
                .and_then(|addr| ChronyPoll::open(addr, sequence()).ok())
        } else {
            None
        };
        let clockbound =
            (probe.clockbound && observed.linux).then(|| host.path(clockbound::DEFAULT_PATH));
        let clocksource = host.open(CLOCKSOURCE).map_err(|e| match e {
            ReadError::Io { kind, .. } => kind,
            _ => io::ErrorKind::Unsupported,
        });
        let mut watch = Self {
            probe: probe.clone(),
            live_linux,
            last: ClockSample {
                kernel: observed.kernel,
                chrony: observed.chrony,
                clockbound: observed.clockbound,
                clocksource: Err(io::ErrorKind::Unsupported),
            },
            observed,
            key: Key {
                why: Why::NotLinux,
                chrony_ref: None,
                bound: None,
                unsettled: false,
                leap: None,
            },
            verdict: SyncVerdict::Unknown,
            bound: None,
            chrony,
            clockbound,
            clocksource,
            arch: source.arch,
            invariant_tsc: source.invariant_tsc,
        };
        watch.last.clocksource = watch.read_clocksource();
        watch.key = watch.judge();
        (watch, findings)
    }

    /// The reads of one check, none of which waits; allocates nothing.
    pub fn sample(&mut self) -> ClockSample {
        let kernel = if self.live_linux {
            KernelSync::read_live().ok()
        } else {
            None
        };
        let chrony = match &mut self.chrony {
            Some(poll) => poll.poll(self.last.chrony),
            None => self.last.chrony,
        };
        let clockbound = match &self.clockbound {
            Some(path) => {
                let now = self
                    .probe
                    .clockbound_now_ns
                    .or_else(|| self.live_linux.then(coarse_now).flatten());
                clockbound_at(path, now)
            }
            None => self.last.clockbound,
        };
        ClockSample {
            kernel,
            chrony,
            clockbound,
            clocksource: self.read_clocksource(),
        }
    }

    /// Judges `sample` by the start-up rules and returns what changed: every
    /// T11 finding when the T11 judgement changed, the T12 finding when the
    /// clock source did, nothing (and no allocation) otherwise. The pure
    /// step of [`Self::check`]: a test gives it samples a host cannot.
    pub fn update(&mut self, sample: ClockSample) -> Vec<Finding> {
        let before = self.last.clocksource;
        self.observed.kernel = sample.kernel;
        self.observed.chrony = sample.chrony;
        self.observed.clockbound = sample.clockbound;
        self.last = sample;
        let key = self.judge();
        let t11 = key != self.key;
        let t12 = sample.clocksource != before;
        self.key = key;
        if !t11 && !t12 {
            return Vec::new();
        }
        let mut out = Vec::new();
        if t11 {
            out.extend(TimeSync::from_observed(self.observed.clone()).findings(&self.probe.limits));
        }
        if t12 {
            out.push(self.clocksource_finding(before, sample.clocksource));
        }
        out
    }

    /// [`Self::sample`], then [`Self::update`]: the findings that changed
    /// since the last check, empty while nothing did.
    pub fn check(&mut self) -> Vec<Finding> {
        let sample = self.sample();
        self.update(sample)
    }

    /// The verdict at the last check.
    #[must_use]
    pub const fn verdict(&self) -> SyncVerdict {
        self.verdict
    }

    /// The error bound at the last check, where one is known.
    #[must_use]
    pub const fn bound(&self) -> Option<Bound> {
        self.bound
    }

    /// Whether a leap second was pending at the last check (chronyd's leap
    /// status, else the kernel's).
    #[must_use]
    pub const fn leap_pending(&self) -> bool {
        self.key.leap.is_some()
    }

    /// What was read, as of the last check (the daemons and PTP clocks as
    /// of [`Self::open`]).
    #[must_use]
    pub const fn observed(&self) -> &Observed {
        &self.observed
    }

    /// The last sample judged.
    #[must_use]
    pub const fn last(&self) -> ClockSample {
        self.last
    }

    /// The verdict, bound and key of what was observed. Allocates nothing.
    fn judge(&mut self) -> Key {
        let o = &self.observed;
        let why = judge(o);
        self.verdict = why.verdict();
        self.bound = bound(o, self.verdict);
        let limit = self.probe.limits.warn_over_ns;
        Key {
            why,
            chrony_ref: match &o.chrony {
                ChronyAnswer::Tracking(t) => Some(t.ref_id),
                _ => None,
            },
            bound: self.bound.map(|b| (b.source, b.ns > limit)),
            unsettled: unsettled(o, limit).is_some(),
            leap: leap_pending(o),
        }
    }

    /// The clock source now, read from the start of the open file into a
    /// buffer on the stack.
    fn read_clocksource(&self) -> Result<ClocksourceName, io::ErrorKind> {
        let file = self.clocksource.as_ref().map_err(|kind| *kind)?;
        let mut buf = [0_u8; 64];
        let n = read_from_start(file, &mut buf).map_err(|e| e.kind())?;
        let text = std::str::from_utf8(buf.get(..n).unwrap_or_default())
            .map_err(|_| io::ErrorKind::InvalidData)?;
        ClocksourceName::new(text).ok_or(io::ErrorKind::InvalidData)
    }

    /// T12 for the clock source now, saying what it changed from.
    fn clocksource_finding(
        &self,
        before: Result<ClocksourceName, io::ErrorKind>,
        now: Result<ClocksourceName, io::ErrorKind>,
    ) -> Finding {
        let source = Clocksource {
            name: now
                .map(|n| n.as_str().to_owned())
                .map_err(|kind| ReadError::Io {
                    path: CLOCKSOURCE.to_owned(),
                    kind,
                }),
            arch: self.arch,
            invariant_tsc: self.invariant_tsc,
        };
        let mut f = source.finding();
        if let (Ok(before), Ok(now)) = (before, now) {
            f.message = format!("changed from {before} to {now}: {}", f.message);
        }
        f
    }
}
