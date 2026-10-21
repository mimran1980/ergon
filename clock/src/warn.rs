//! The warnings: what makes the clock slow or its UTC stamps wrong, one
//! plain line each. The front `calibrate` prints the start-up ones; the
//! health check returns the later ones, and never prints.

use crate::calibrate::Report;
use crate::clock::Domain;
use crate::detect::{CLOCKSOURCE, Fallback};
use crate::timesync::{ChronyAnswer, Observed, SyncVerdict, TimeSync, human_ns};
use std::ffi::OsStr;
use std::fmt;
use std::io::Write;

/// The environment variable that turns the host checks' warnings off.
///
/// Takes `off` or `0`; any other value, a typo included, leaves them on.
/// `ergon-clock` reads it itself; downstream runtimes may read the same
/// name for their own start-up checks.
pub const HOST_CHECKS_ENV: &str = "ERGON_HOST_CHECKS";

/// The rate cap the front clock slews at, ppm: 0.5 ms a second.
pub const CAP_PPM: u64 = 500;

/// `adjtimex`'s `STA_INS`: a leap second is to be inserted at midnight.
const STA_INS: i32 = 0x0010;
/// `adjtimex`'s `STA_DEL`: a leap second is to be deleted at midnight.
const STA_DEL: i32 = 0x0020;

/// One thing that makes the clock slow or its UTC stamps wrong.
///
/// [`Display`](fmt::Display) gives the message; [`line`](Self::line) the
/// line the front `calibrate` prints (`ergon-clock warning: ...`), for an
/// application's own logger (it allocates: log it off the hot path).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// The clock reads the operating system's monotonic clock, not the
    /// hardware counter: slower, about what `Instant::now` costs.
    Fallback {
        /// Why not the counter.
        why: Fallback,
    },
    /// The host's clock is not synchronised to anything, so UTC stamps can
    /// be off by any amount.
    Unsynchronised {
        /// Why, as the time-sync probe found it.
        reason: String,
    },
    /// The host's clock is still being corrected by more than the
    /// threshold, or its last correction was larger: time sync has not
    /// settled, and this clock follows a correction at 0.5 ms a second.
    Unsettled {
        /// The correction still to come, ns, where it is over the
        /// threshold (positive: the host's clock is behind).
        correction_ns: Option<i64>,
        /// The last update's offset, ns, where it is over the threshold.
        last_update_ns: Option<i64>,
        /// What reported it: `chronyd` or `adjtimex`.
        source: &'static str,
    },
    /// The host's clock is known to UTC only within a bound over the
    /// threshold.
    Uncertain {
        /// The bound, ns.
        bound_ns: u64,
        /// Where it came from: `chrony`, `clockbound` or `kernel-maxerror`.
        source: &'static str,
    },
    /// A leap second is pending: the host's clock will step 1 s, and this
    /// clock will slew it.
    LeapPending {
        /// What reported it: `chronyd` or `adjtimex`.
        source: &'static str,
    },
    /// After start: the clock is this far from `CLOCK_REALTIME`.
    Gap {
        /// `CLOCK_REALTIME` less the clock, ns (positive: the clock is
        /// behind).
        gap_ns: i64,
        /// How long closing it takes at 0.5 ms a second, ns.
        catch_up_ns: u64,
    },
    /// After start: no accepted update for over 5 s.
    Stale {
        /// Since the last accepted update, ns.
        since_ns: u64,
        /// How far the clock may have drifted from `CLOCK_REALTIME`, ns.
        bound_ns: u64,
        /// Samples rejected in a row (0: `update` was not called).
        rejected_in_a_row: u64,
    },
}

impl Warning {
    /// The line the front `calibrate` prints: `ergon-clock warning: ` and
    /// the message. Allocates: log it off the hot path.
    #[must_use]
    pub fn line(&self) -> String {
        format!("ergon-clock warning: {self}")
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fallback { why } => {
                let (because, fix) = why_and_fix(why);
                write!(
                    f,
                    "the clock reads {}, not the hardware counter: slower, about what \
                     Instant::now costs; why: {because}; fix: {fix}",
                    crate::wall::MONOTONIC_NAME
                )
            }
            Self::Unsynchronised { reason } => write!(
                f,
                "UTC: the host's clock is not synchronised ({reason}); CLOCK_REALTIME, which this \
                 clock follows, can be off UTC by any amount; fix: run chrony, ntpd or PTP and \
                 start once it is synchronised"
            ),
            Self::Unsettled {
                correction_ns: Some(correction),
                source,
                ..
            } => write!(
                f,
                "UTC: the host's clock is not settled: {source} is still correcting it by {}; a \
                 disciplined clock follows at most 0.5 ms a second, so UTC stamps can be off for \
                 at least {}; start after time sync settles",
                human_ns(correction.unsigned_abs()),
                human_long(catch_up_ns(correction.unsigned_abs()))
            ),
            Self::Unsettled {
                last_update_ns,
                source,
                ..
            } => write!(
                f,
                "UTC: the host's clock is not settled: {source}'s last update moved it by {}; \
                 time sync may not have settled; start after it settles",
                human_ns(last_update_ns.map_or(0, i64::unsigned_abs))
            ),
            Self::Uncertain { bound_ns, source } => write!(
                f,
                "UTC: the host's clock is known to UTC only within {} ({source}); stamps \
                 compared with other hosts can differ by that much",
                human_ns(*bound_ns)
            ),
            Self::LeapPending { source } => write!(
                f,
                "UTC: a leap second is pending ({source}); the host's clock will step 1 s and \
                 this clock will slew it at 0.5 ms a second: off by up to 1 s for about 33 min"
            ),
            Self::Gap {
                gap_ns,
                catch_up_ns,
            } => write!(
                f,
                "the clock is {} {} CLOCK_REALTIME; it closes the gap at 0.5 ms a second \
                 (about {}); stamps are off until then",
                human_ns(gap_ns.unsigned_abs()),
                if *gap_ns >= 0 { "behind" } else { "ahead of" },
                human_long(*catch_up_ns)
            ),
            Self::Stale {
                since_ns,
                bound_ns,
                rejected_in_a_row,
            } => {
                write!(
                    f,
                    "no UTC update for {}: the clock may have drifted from CLOCK_REALTIME by up \
                     to {}; ",
                    human_long(*since_ns),
                    human_ns(*bound_ns)
                )?;
                if *rejected_in_a_row == 0 {
                    f.write_str("call Clock::update from the loop's idle time about once a second")
                } else {
                    write!(
                        f,
                        "{rejected_in_a_row} samples in a row rejected: the host interrupts the \
                         sample"
                    )
                }
            }
        }
    }
}

/// Why the fallback, in words, and its fix.
fn why_and_fix(why: &Fallback) -> (String, &'static str) {
    const CROSS_CHECK: &str = "Config { trust: ergon_clock::advanced::Trust::CrossCheck, .. } \
                               lets calibration's own checks take the counter";
    const HARDWARE: &str = "none in software (the hardware)";
    const AGAIN: &str = "calibrate again; report the host if it repeats";
    match why {
        Fallback::Paravirt(name) => (
            format!("Linux keeps time with {name}, a hypervisor's clock"),
            CROSS_CHECK,
        ),
        Fallback::NoKernelVouch => (
            "this operating system names no clock source to vouch for the counter".to_owned(),
            CROSS_CHECK,
        ),
        Fallback::ClocksourceUnreadable => (
            format!("Linux's clock source could not be read ({CLOCKSOURCE})"),
            CROSS_CHECK,
        ),
        Fallback::KernelClocksource(name) => (
            format!("Linux keeps time with {name}, so clock reads may be system calls"),
            "find why Linux left the counter (dmesg | grep -i clocksource); force it only if \
             the counter is sound",
        ),
        Fallback::TscNotInvariant => (
            "the TSC is not invariant: its rate can change".to_owned(),
            HARDWARE,
        ),
        Fallback::NoRdtscp => ("the CPU has no rdtscp".to_owned(), HARDWARE),
        Fallback::NoCounter => (
            "no hardware counter this crate reads on this CPU".to_owned(),
            HARDWARE,
        ),
        Fallback::Trapped { read_ns } => (
            format!(
                "a counter read costs {read_ns} ns, over Config::max_read_ns: it is trapped and \
                 emulated"
            ),
            HARDWARE,
        ),
        Fallback::CounterStalled => (
            "the counter did not advance during calibration".to_owned(),
            AGAIN,
        ),
        Fallback::RateOutOfRange { hz } => (
            format!("the counter's measured rate, {hz} Hz, is out of range"),
            AGAIN,
        ),
        Fallback::Unstable { ppm } => (
            format!("the counter's rate moved by {ppm} ppm between two measurements"),
            AGAIN,
        ),
        Fallback::Forced => (
            "asked for (Policy::Monotonic)".to_owned(),
            "none needed; reads cost about what Instant::now does",
        ),
    }
}

/// How long a gap of `gap_ns` takes to close at the cap, ns.
pub fn catch_up_ns(gap_ns: u64) -> u64 {
    u64::try_from(u128::from(gap_ns) * 1_000_000 / u128::from(CAP_PPM)).unwrap_or(u64::MAX)
}

/// A long duration as a person reads it: `human_ns` under a minute, then
/// `4.0 min`, `1.2 h`.
fn human_long(ns: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let s = ns as f64 / 1e9;
    if s < 60.0 {
        human_ns(ns)
    } else if s < 3_600.0 {
        format!("{:.1} min", s / 60.0)
    } else {
        format!("{:.1} h", s / 3_600.0)
    }
}

/// Seconds as ns, rounded and saturating.
#[allow(clippy::cast_possible_truncation)] // saturating, by `as`
fn seconds_ns(s: f64) -> i64 {
    (s * 1e9).round() as i64
}

impl Report {
    /// What makes this clock slow or its UTC stamps wrong, one warning per
    /// problem: exactly what the front [`calibrate`](crate::calibrate)
    /// printed. Pure: from the report's fields, read again at no cost.
    ///
    /// - The fallback, asked for or not: reads cost about what
    ///   `Instant::now` does.
    /// - In the UTC domain, from the time-sync probe (none when it was
    ///   off): the host's clock not synchronised; else still being corrected
    ///   by more than [`gap_warn_ns`](Self::gap_warn_ns), or its last
    ///   update larger; else known to UTC only within more than that; and a
    ///   leap second pending. What could not be read is never a warning.
    #[must_use]
    pub fn warnings(&self) -> Vec<Warning> {
        let mut out = Vec::new();
        if let Some(why) = &self.fallback {
            out.push(Warning::Fallback { why: why.clone() });
        }
        if self.domain != Domain::Realtime {
            return out;
        }
        let Some(sync) = &self.time_sync else {
            return out;
        };
        if sync.verdict == SyncVerdict::Unsynchronised {
            out.push(Warning::Unsynchronised {
                reason: sync.reason.clone(),
            });
        } else if let Some(w) = unsettled(&sync.observed, self.gap_warn_ns) {
            out.push(w);
        } else if let Some(w) = uncertain(sync, self.gap_warn_ns) {
            out.push(w);
        }
        if let Some(source) = leap_pending(&sync.observed) {
            out.push(Warning::LeapPending { source });
        }
        out
    }
}

/// The host's clock still being corrected by more than `threshold_ns`
/// (chronyd's pending correction, else the kernel PLL's offset), or moved
/// by more at chronyd's last update.
fn unsettled(o: &Observed, threshold_ns: u64) -> Option<Warning> {
    let tracking = match &o.chrony {
        ChronyAnswer::Tracking(t) => Some(t),
        _ => None,
    };
    let over = |ns: &i64| ns.unsigned_abs() > threshold_ns;
    let (correction, source) = tracking.map_or_else(
        || (o.kernel.map(|k| k.offset_ns), "adjtimex"),
        |t| (Some(seconds_ns(t.correction_s)), "chronyd"),
    );
    let correction_ns = correction.filter(over);
    let last_update_ns = tracking.map(|t| seconds_ns(t.last_offset_s)).filter(over);
    if correction_ns.is_none() && last_update_ns.is_none() {
        return None;
    }
    Some(Warning::Unsettled {
        correction_ns,
        last_update_ns,
        source,
    })
}

/// The host's clock known to UTC only within more than `threshold_ns`.
fn uncertain(sync: &TimeSync, threshold_ns: u64) -> Option<Warning> {
    let bound = sync.bound.filter(|b| b.ns > threshold_ns)?;
    Some(Warning::Uncertain {
        bound_ns: bound.ns,
        source: bound.source.name(),
    })
}

/// Who says a leap second is pending: chronyd's leap status (1 insert, 2
/// delete), else the kernel's `STA_INS` or `STA_DEL`.
fn leap_pending(o: &Observed) -> Option<&'static str> {
    if let ChronyAnswer::Tracking(t) = &o.chrony
        && matches!(t.leap_status, 1 | 2)
    {
        return Some("chronyd");
    }
    o.kernel
        .filter(|k| k.status & (STA_INS | STA_DEL) != 0)
        .map(|_| "adjtimex")
}

/// Whether the switch's value turns the warnings off: `off` or `0`, and
/// only those (a case variant is a typo, and a typo leaves them on).
fn silences(value: &OsStr) -> bool {
    value
        .to_str()
        .map(str::trim)
        .is_some_and(|v| v == "0" || v == "off")
}

/// Prints `report`'s warnings to `out`, one line each, unless `print` is
/// false or the switch's value (`ERGON_HOST_CHECKS`) is `off` or `0`.
/// A failed write is ignored: a warning never stops a start.
pub fn print(report: &Report, print: bool, switch: Option<&OsStr>, out: &mut impl Write) {
    if !print || switch.is_some_and(silences) {
        return;
    }
    for warning in report.warnings() {
        let _ = writeln!(out, "{}", warning.line());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::Config;
    use crate::chrony::{ChronyReference, ChronyTracking};
    use crate::detect::{Arch, HostFacts, Os};
    use crate::timesync::{ClockBoundAnswer, KernelSync, TimeDaemon};
    use std::net::{IpAddr, Ipv4Addr};

    /// A report on a Linux x86 host, in `domain`, with this time sync.
    fn report(domain: Domain, sync: Option<Observed>, gap_warn_ns: u64) -> Report {
        let mut r = Report::new(
            HostFacts::unknown(Arch::X86_64, Os::Linux),
            &Config {
                domain,
                gap_warn_ns,
                ..Config::default()
            },
        );
        r.time_sync = sync.map(TimeSync::from_observed);
        r
    }

    fn kernel(status: i32, offset_ns: i64) -> KernelSync {
        KernelSync {
            state: 0,
            status,
            maxerror_us: 1_000,
            esterror_us: 0,
            offset_ns,
            freq_ppm: 0.0,
        }
    }

    /// chronyd tracking an NTP server: `correction_s`, `last_offset_s`,
    /// root dispersion, leap status.
    fn chrony(correction_s: f64, last_offset_s: f64, dispersion_s: f64, leap: u16) -> Observed {
        Observed {
            linux: true,
            kernel: Some(kernel(0, 0)),
            daemons: Some(vec![TimeDaemon::Chronyd]),
            ptp_clocks: Vec::new(),
            chrony: ChronyAnswer::Tracking(ChronyTracking {
                reference: ChronyReference::Server(Some(IpAddr::V4(Ipv4Addr::new(
                    169, 254, 169, 123,
                )))),
                ref_id: 0xA9FE_A97B,
                stratum: 4,
                leap_status: leap,
                correction_s,
                last_offset_s,
                rms_offset_s: 0.0,
                freq_ppm: 0.0,
                resid_freq_ppm: 0.0,
                skew_ppm: 0.0,
                root_delay_s: 0.0,
                root_dispersion_s: dispersion_s,
                update_interval_s: 16.0,
            }),
            clockbound: ClockBoundAnswer::NotAsked,
        }
    }

    /// No daemon visible, the kernel unsynchronised with `maxerror` at its
    /// limit: no time sync at all.
    fn unsynchronised() -> Observed {
        let mut k = kernel(crate::timesync::STA_UNSYNC, 0);
        k.maxerror_us = crate::timesync::MAXERROR_LIMIT_US;
        Observed {
            linux: true,
            kernel: Some(k),
            daemons: Some(Vec::new()),
            ptp_clocks: Vec::new(),
            chrony: ChronyAnswer::NotAsked,
            clockbound: ClockBoundAnswer::NotAsked,
        }
    }

    fn lines(r: &Report) -> Vec<String> {
        r.warnings().iter().map(Warning::line).collect()
    }

    /// (a) The fallback: one line naming the hypervisor's clock, slower,
    /// and the fix; a requested fallback (`Policy::Monotonic`) says so and
    /// still warns, because its reads are slow.
    #[test]
    fn a_fallback_is_one_line_unless_forced() {
        let mut r = report(Domain::Realtime, None, 1_000_000);
        r.fallback = Some(Fallback::Paravirt("hyperv_clocksource_tsc_page".to_owned()));
        let l = lines(&r);
        assert_eq!(l.len(), 1, "{l:?}");
        for want in ["hyperv_clocksource_tsc_page", "slower", "Trust::CrossCheck"] {
            assert!(l[0].contains(want), "no {want:?} in {}", l[0]);
        }
        r.fallback = Some(Fallback::Forced);
        let l = lines(&r);
        assert_eq!(l.len(), 1, "{l:?}");
        for want in ["Policy::Monotonic", "Instant::now"] {
            assert!(l[0].contains(want), "no {want:?} in {}", l[0]);
        }
    }

    /// (b) An unsynchronised host clock warns in UTC only; what cannot be
    /// read and an NTP clock within the bound do not.
    #[test]
    fn an_unsynchronised_host_warns_in_utc_only() {
        let r = report(Domain::Realtime, Some(unsynchronised()), 1_000_000);
        assert_eq!(
            r.time_sync.as_ref().map(|t| t.verdict),
            Some(SyncVerdict::Unsynchronised)
        );
        assert!(matches!(r.warnings()[..], [Warning::Unsynchronised { .. }]));
        let r = report(Domain::Monotonic, Some(unsynchronised()), 1_000_000);
        assert_eq!(r.warnings(), []);
        let r = report(Domain::Realtime, Some(Observed::not_linux()), 1_000_000);
        assert_eq!(
            r.time_sync.as_ref().map(|t| t.verdict),
            Some(SyncVerdict::Unknown)
        );
        assert_eq!(r.warnings(), []);
        let r = report(
            Domain::Realtime,
            Some(chrony(0.0, 0.0, 0.000_2, 0)),
            1_000_000,
        );
        assert_eq!(
            r.time_sync.as_ref().map(|t| t.verdict),
            Some(SyncVerdict::NtpDisciplined)
        );
        assert_eq!(r.warnings(), []);
    }

    /// (c1) The host's clock still being corrected by 120 ms: one line
    /// (not also c2) with the gap and the catch-up time at the cap; 0.9 ms
    /// only under a lower threshold; a large last update alone, with no
    /// catch-up time.
    #[test]
    fn an_unsettled_host_warns_with_the_gap_and_the_catch_up() {
        let r = report(
            Domain::Realtime,
            Some(chrony(0.120, 0.0, 0.001, 0)),
            1_000_000,
        );
        assert_eq!(
            r.time_sync
                .as_ref()
                .and_then(|t| t.bound)
                .map(|b| b.ns.abs_diff(121_000_000) <= 1),
            Some(true)
        );
        let l = lines(&r);
        assert_eq!(l.len(), 1, "{l:?}");
        assert!(
            l[0].contains("120.00 ms") && l[0].contains("at least 4.0 min"),
            "{}",
            l[0]
        );
        let small = chrony(0.000_9, 0.0, 0.000_05, 0);
        assert_eq!(
            report(Domain::Realtime, Some(small.clone()), 1_000_000).warnings(),
            []
        );
        let l = lines(&report(Domain::Realtime, Some(small), 500_000));
        assert_eq!(l.len(), 1, "{l:?}");
        assert!(l[0].contains("900.0 µs"), "{}", l[0]);
        let l = lines(&report(
            Domain::Realtime,
            Some(chrony(0.0, 0.5, 0.000_05, 0)),
            1_000_000,
        ));
        assert_eq!(l.len(), 1, "{l:?}");
        assert!(
            l[0].contains("500.00 ms") && !l[0].contains("at least"),
            "{}",
            l[0]
        );
    }

    /// (c1) Without chronyd's report, the kernel PLL's pending offset.
    #[test]
    fn without_chronyd_the_kernel_offset_counts() {
        let mut o = unsynchronised();
        o.kernel = Some(kernel(0, -3_000_000));
        o.daemons = Some(vec![TimeDaemon::Ntpd]);
        let w = report(Domain::Realtime, Some(o), 1_000_000).warnings();
        assert_eq!(
            w,
            [Warning::Unsettled {
                correction_ns: Some(-3_000_000),
                last_update_ns: None,
                source: "adjtimex"
            }]
        );
    }

    /// (c2) Known to UTC only within 12 ms: one line, no catch-up time.
    #[test]
    fn a_wide_bound_warns_without_a_catch_up() {
        let l = lines(&report(
            Domain::Realtime,
            Some(chrony(0.0, 0.0, 0.012, 0)),
            1_000_000,
        ));
        assert_eq!(
            l,
            [
                "ergon-clock warning: UTC: the host's clock is known to UTC only within 12.00 ms \
              (chrony); stamps compared with other hosts can differ by that much"
            ]
        );
    }

    /// (d) A leap second pending, from chronyd or the kernel.
    #[test]
    fn a_pending_leap_second_warns() {
        let w = report(
            Domain::Realtime,
            Some(chrony(0.0, 0.0, 0.000_05, 1)),
            1_000_000,
        )
        .warnings();
        assert_eq!(w, [Warning::LeapPending { source: "chronyd" }]);
        let mut o = unsynchronised();
        o.kernel = Some(kernel(STA_INS, 0));
        o.daemons = Some(vec![TimeDaemon::Ntpd]);
        let w = report(Domain::Realtime, Some(o), 1_000_000).warnings();
        assert_eq!(w, [Warning::LeapPending { source: "adjtimex" }]);
    }

    /// Every variant's line, word for word.
    #[test]
    fn every_line_word_for_word() {
        let cases = [
            (
                Warning::Fallback {
                    why: Fallback::Paravirt("hyperv_clocksource_tsc_page".to_owned()),
                },
                format!(
                    "the clock reads {}, not the hardware counter: slower, about what \
                     Instant::now costs; why: Linux keeps time with hyperv_clocksource_tsc_page, \
                     a hypervisor's clock; fix: Config {{ trust: \
                     ergon_clock::advanced::Trust::CrossCheck, .. }} lets calibration's own \
                     checks take the counter",
                    crate::wall::MONOTONIC_NAME
                ),
            ),
            (
                Warning::Unsynchronised {
                    reason: "no time daemon".to_owned(),
                },
                "UTC: the host's clock is not synchronised (no time daemon); CLOCK_REALTIME, \
                 which this clock follows, can be off UTC by any amount; fix: run chrony, ntpd \
                 or PTP and start once it is synchronised"
                    .to_owned(),
            ),
            (
                Warning::Unsettled {
                    correction_ns: Some(120_000_000),
                    last_update_ns: None,
                    source: "chronyd",
                },
                "UTC: the host's clock is not settled: chronyd is still correcting it by 120.00 \
                 ms; a disciplined clock follows at most 0.5 ms a second, so UTC stamps can be \
                 off for at least 4.0 min; start after time sync settles"
                    .to_owned(),
            ),
            (
                Warning::Unsettled {
                    correction_ns: None,
                    last_update_ns: Some(-500_000_000),
                    source: "chronyd",
                },
                "UTC: the host's clock is not settled: chronyd's last update moved it by 500.00 \
                 ms; time sync may not have settled; start after it settles"
                    .to_owned(),
            ),
            (
                Warning::LeapPending { source: "chronyd" },
                "UTC: a leap second is pending (chronyd); the host's clock will step 1 s and \
                 this clock will slew it at 0.5 ms a second: off by up to 1 s for about 33 min"
                    .to_owned(),
            ),
            (
                Warning::Gap {
                    gap_ns: 120_000_000,
                    catch_up_ns: 240_000_000_000,
                },
                "the clock is 120.00 ms behind CLOCK_REALTIME; it closes the gap at 0.5 ms a \
                 second (about 4.0 min); stamps are off until then"
                    .to_owned(),
            ),
            (
                Warning::Gap {
                    gap_ns: -1_000_000_000,
                    catch_up_ns: 2_000_000_000_000,
                },
                "the clock is 1.0 s ahead of CLOCK_REALTIME; it closes the gap at 0.5 ms a \
                 second (about 33.3 min); stamps are off until then"
                    .to_owned(),
            ),
            (
                Warning::Stale {
                    since_ns: 6_000_000_000,
                    bound_ns: 150_000,
                    rejected_in_a_row: 0,
                },
                "no UTC update for 6.0 s: the clock may have drifted from CLOCK_REALTIME by up \
                 to 150.0 µs; call Clock::update from the loop's idle time about once a second"
                    .to_owned(),
            ),
            (
                Warning::Stale {
                    since_ns: 6_000_000_000,
                    bound_ns: 150_000,
                    rejected_in_a_row: 3,
                },
                "no UTC update for 6.0 s: the clock may have drifted from CLOCK_REALTIME by up \
                 to 150.0 µs; 3 samples in a row rejected: the host interrupts the sample"
                    .to_owned(),
            ),
        ];
        for (warning, message) in cases {
            assert_eq!(warning.line(), format!("ergon-clock warning: {message}"));
        }
        assert_eq!(
            catch_up_ns(1_000_000_000),
            2_000_000_000_000,
            "1 s: about 33 min"
        );
    }

    /// The time-sync finding says UTC stamps are only as good as
    /// `CLOCK_REALTIME`, and that the default clock follows it.
    #[test]
    fn the_time_sync_finding_speaks_of_utc_stamps() {
        let t = TimeSync::from_observed(chrony(0.0, 0.0, 0.000_2, 0));
        let m = t.finding().message;
        assert!(
            m.contains("UTC stamps are only as good as CLOCK_REALTIME"),
            "{m}"
        );
        assert!(
            m.contains("durations include the host's rate corrections"),
            "{m}"
        );
    }

    /// Printing: one line per warning; nothing with the switch `off` or
    /// `0` or the configuration's off; the report keeps them either way.
    /// Only `off` and `0` silence: a case variant is a typo, like `of`.
    #[test]
    fn the_switch_and_the_config_silence_the_print() {
        let mut r = report(Domain::Realtime, Some(unsynchronised()), 1_000_000);
        r.fallback = Some(Fallback::NoKernelVouch);
        assert_eq!(r.warnings().len(), 2);
        let printed = |print_on: bool, switch: Option<&str>| {
            let mut out = Vec::new();
            super::print(&r, print_on, switch.map(OsStr::new), &mut out);
            String::from_utf8_lossy(&out).lines().count()
        };
        assert_eq!(printed(true, None), 2);
        assert_eq!(printed(true, Some("on")), 2);
        assert_eq!(printed(true, Some("of")), 2, "a typo never silences");
        assert_eq!(
            printed(true, Some("OFF")),
            2,
            "a case variant never silences"
        );
        assert_eq!(printed(true, Some("off")), 0);
        assert_eq!(printed(true, Some("0")), 0);
        assert_eq!(printed(false, None), 0);
        assert_eq!(r.warnings().len(), 2);
    }
}
