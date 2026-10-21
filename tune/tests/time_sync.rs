//! T11 (how `CLOCK_REALTIME` is kept) and T12 (the kernel's clock source):
//! the verdict on injected observations (`adjtimex` results, chronyd's
//! answers, daemons, PTP hardware clocks, `ClockBound`), the findings it
//! gives, the readers against fixture trees and a fake chronyd, and the real
//! readers on this host (bounded: nothing hangs).
#![allow(missing_docs)]

mod common;

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use common::Fixture;
use ergon_tune::{
    Arch, BoundSource, CheckId, ChronyAnswer, ChronyError, ChronyReference, ChronyTracking,
    ClockBoundAnswer, ClockBoundError, ClockBoundReading, ClockBoundStatus, Clocksource,
    ClocksourceKind, Finding, Host, HostCheckPolicy, KernelSync, Observed, PtpClock, ReadError,
    RefclockId, Severity, Status, SyncVerdict, TimeDaemon, TimeSync, TimeSyncLimits, TimeSyncProbe,
    t11_time_sync, t12_clocksource, time_sync,
};

type R = Result<(), Box<dyn std::error::Error>>;

/// The kernel synchronised by a daemon that keeps `maxerror` (µs).
const fn synced(maxerror_us: i64) -> KernelSync {
    KernelSync {
        state: 0,
        status: 0x0001, // STA_PLL
        maxerror_us,
        esterror_us: 10,
        offset_ns: 0,
        freq_ppm: -12.5,
    }
}

/// The kernel unsynchronised (`STA_UNSYNC`, `TIME_ERROR`), `maxerror` as
/// given (16 s: nothing keeps it).
const fn unsynced(maxerror_us: i64) -> KernelSync {
    KernelSync {
        state: 5,
        status: 0x0041,
        maxerror_us,
        esterror_us: 16_000_000,
        offset_ns: 0,
        freq_ppm: 0.0,
    }
}

const fn tracking(reference: ChronyReference, leap_status: u16) -> ChronyTracking {
    ChronyTracking {
        reference,
        ref_id: 0,
        stratum: 1,
        leap_status,
        correction_s: -0.000_002,
        last_offset_s: 0.000_001,
        rms_offset_s: 0.000_001,
        freq_ppm: -12.25,
        resid_freq_ppm: 0.0,
        skew_ppm: 0.01,
        root_delay_s: 0.000_010,
        root_dispersion_s: 0.000_003,
        update_interval_s: 1.0,
    }
}

const PHC0: u32 = u32::from_be_bytes(*b"PHC0");

const fn phc0() -> ChronyAnswer {
    ChronyAnswer::Tracking(tracking(
        ChronyReference::Refclock(RefclockId::new(PHC0)),
        0,
    ))
}

const fn ntp_server() -> ChronyAnswer {
    let mut t = tracking(
        ChronyReference::Server(Some(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 123)))),
        0,
    );
    t.ref_id = 0xA9FE_A97B;
    t.stratum = 4;
    t.root_delay_s = 0.000_400;
    t.root_dispersion_s = 0.000_950;
    ChronyAnswer::Tracking(t)
}

const fn refused() -> ChronyAnswer {
    ChronyAnswer::Failed(ChronyError::Io {
        op: "recv",
        kind: std::io::ErrorKind::ConnectionRefused,
    })
}

fn ptp(device: &str, name: &str) -> PtpClock {
    PtpClock {
        device: device.to_owned(),
        name: name.to_owned(),
    }
}

/// A Linux host: the kernel's state, the daemons seen, chronyd's answer,
/// the PHCs, no `ClockBound`.
fn linux(
    kernel: Option<KernelSync>,
    daemons: &[TimeDaemon],
    chrony: ChronyAnswer,
    ptp_clocks: Vec<PtpClock>,
) -> Observed {
    Observed {
        linux: true,
        kernel,
        daemons: Some(daemons.to_vec()),
        ptp_clocks,
        chrony,
        clockbound: ClockBoundAnswer::Absent,
    }
}

fn judge(o: Observed) -> TimeSync {
    TimeSync::from_observed(o)
}

/// The T11 findings at the default limit (1 ms).
fn findings(t: &TimeSync) -> Vec<Finding> {
    t.findings(&TimeSyncLimits::default())
}

/// The one verdict finding: the first T11 finding.
fn verdict_finding(t: &TimeSync) -> Result<Finding, Box<dyn std::error::Error>> {
    let all = findings(t);
    assert!(all.iter().all(|f| f.id == CheckId::T11), "{all:?}");
    Ok(all.into_iter().next().ok_or("no T11 finding")?)
}

// --- the verdict ------------------------------------------------------------

/// Off Linux nothing is read and nothing is judged: one unknown T11 finding,
/// its reason `not Linux`, never a warning.
#[test]
fn off_linux_the_verdict_is_unknown_with_its_reason() {
    let t = judge(Observed::not_linux());
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert_eq!(t.bound, None);
    let all = findings(&t);
    assert_eq!(all.len(), 1, "{all:?}");
    let f = &all[0];
    assert_eq!(f.id, CheckId::T11);
    assert!(
        matches!(&f.status, Status::Unknown(why) if why.starts_with("not Linux")),
        "{f}"
    );
    assert!(!f.is_warning());
}

/// chronyd locked to a PHC reference clock is PTP-grade, with chrony's
/// bound (|offset| + dispersion + delay / 2 = 2 + 3 + 5 µs): an ok T11.
#[test]
fn chronyd_locked_to_a_phc_is_ptp_grade_and_ok() {
    let t = judge(linux(
        Some(synced(450)),
        &[TimeDaemon::Chronyd],
        phc0(),
        vec![ptp("ptp0", "ena-ptp-00")],
    ));
    assert_eq!(t.verdict, SyncVerdict::PtpGrade);
    assert_eq!(t.source, "chronyd:PHC0");
    assert_eq!(
        t.bound.map(|b| (b.source, b.ns.abs_diff(10_000) <= 2)),
        Some((BoundSource::Chrony, true)),
        "{:?}",
        t.bound
    );
    let all = findings(&t);
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].status, Status::Ok, "{}", all[0]);
    assert!(all[0].message.contains("PHC0"), "{}", all[0]);
}

/// chronyd tracking an NTP server is NTP-disciplined: a warning that names
/// the server and the bound, says UTC stamps are only as good as
/// `CLOCK_REALTIME`, and gives the fix for the PHC the host has (ENA: AWS's
/// refclock line; Hyper-V: Azure's) or, with none, both clouds'.
#[test]
fn chronyd_on_an_ntp_server_warns_with_the_bound_and_the_fix() -> R {
    let t = judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(t.source, "chronyd:169.254.169.123");
    // 2 µs of offset + 950 µs + 400 / 2 µs
    assert_eq!(
        t.bound.map(|b| (b.source, b.ns.abs_diff(1_152_000) <= 2)),
        Some((BoundSource::Chrony, true)),
        "{:?}",
        t.bound
    );
    let f = verdict_finding(&t)?;
    assert!(f.is_warning(), "{f}");
    for want in [
        "not disciplined to a PTP hardware clock",
        "169.254.169.123",
        "1.15 ms",
        "cross-host",
        "UTC stamps are only as good as CLOCK_REALTIME",
        "CLOCK_MONOTONIC is slewed with it",
        "refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer",
        "/dev/ptp_hyperv",
        "on the node",
    ] {
        assert!(f.message.contains(want), "no {want:?} in {f}");
    }
    let aws = verdict_finding(&judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![ptp("ptp0", "ena-ptp-00")],
    )))?;
    assert!(aws.message.contains("ptp0: ena-ptp-00"), "{aws}");
    assert!(aws.message.contains("refclock PHC /dev/ptp_ena"), "{aws}");
    assert!(!aws.message.contains("ptp_hyperv"), "{aws}");
    let azure = verdict_finding(&judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![ptp("ptp0", "hyperv")],
    )))?;
    assert!(
        azure
            .message
            .contains("refclock PHC /dev/ptp_hyperv poll 3 dpoll -2 offset 0"),
        "{azure}"
    );
    assert!(!azure.message.contains("ptp_ena"), "{azure}");
    Ok(())
}

/// chronyd that reports itself synchronised but never adjusts this kernel's
/// clock (`chronyd -x`: the kernel's `maxerror` stays at its 16 s limit) is
/// not disciplining `CLOCK_REALTIME`: unsynchronised, never PTP-grade.
/// Without `rtcsync` chronyd leaves `STA_UNSYNC` set but keeps `maxerror`:
/// that is still its word.
#[test]
fn chronyd_that_does_not_adjust_the_clock_is_not_believed() {
    for answer in [phc0(), ntp_server()] {
        let t = judge(linux(
            Some(unsynced(16_000_000)),
            &[TimeDaemon::Chronyd],
            answer,
            vec![],
        ));
        assert_eq!(t.verdict, SyncVerdict::Unsynchronised, "{}", t.reason);
        assert!(t.reason.contains("-x"), "{}", t.reason);
        assert_eq!(t.bound, None);
    }
    let t = judge(linux(
        Some(unsynced(450)),
        &[TimeDaemon::Chronyd],
        phc0(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::PtpGrade, "{}", t.reason);
}

/// A reference clock not named as a PHC (`refid GPS`) is not PTP-grade.
#[test]
fn a_refclock_not_named_as_a_phc_is_not_ptp_grade() {
    let gps = ChronyAnswer::Tracking(tracking(
        ChronyReference::Refclock(RefclockId::new(u32::from_be_bytes(*b"GPS\0"))),
        0,
    ));
    let t = judge(linux(
        Some(synced(100)),
        &[TimeDaemon::Chronyd],
        gps,
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(t.source, "chronyd:GPS");
}

/// chronyd synchronised to nothing, or to its own clock (`local`), is
/// unsynchronised: a warning, no bound.
#[test]
fn chronyd_unsynchronised_or_local_is_unsynchronised() -> R {
    for (reference, leap, source) in [
        (ChronyReference::None, 3, "chronyd"),
        (ChronyReference::Local, 0, "chronyd:local"),
    ] {
        let answer = ChronyAnswer::Tracking(tracking(reference, leap));
        let t = judge(linux(
            Some(unsynced(16_000_000)),
            &[TimeDaemon::Chronyd],
            answer,
            vec![],
        ));
        assert_eq!(t.verdict, SyncVerdict::Unsynchronised, "{source}");
        assert_eq!(t.source, source);
        assert_eq!(
            t.reason.contains("local stratum"),
            source == "chronyd:local",
            "{}",
            t.reason
        );
        assert_eq!(t.bound, None);
        assert!(verdict_finding(&t)?.is_warning());
    }
    Ok(())
}

/// phc2sys with the kernel synchronised is PTP-grade (ok) with no bound (it
/// keeps no `maxerror`, so the kernel's would mislead); with the kernel
/// unsynchronised it has not locked (or disciplines another clock).
#[test]
fn phc2sys_is_ptp_grade_only_with_the_kernel_synchronised() -> R {
    let t = judge(linux(
        Some(synced(0)),
        &[TimeDaemon::Phc2sys, TimeDaemon::Ptp4l],
        refused(),
        vec![ptp("ptp0", "mlx5_ptp")],
    ));
    assert_eq!(t.verdict, SyncVerdict::PtpGrade);
    assert_eq!(t.source, "phc2sys");
    assert_eq!(t.bound, None);
    let f = verdict_finding(&t)?;
    assert_eq!(f.status, Status::Ok);
    assert!(f.message.contains("no error bound"), "{f}");
    let t = judge(linux(
        Some(unsynced(16_000_000)),
        &[TimeDaemon::Phc2sys],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
    assert!(verdict_finding(&t)?.is_warning());
    Ok(())
}

/// chronyd's own answer outranks a running phc2sys.
#[test]
fn chronyds_answer_outranks_phc2sys() {
    let t = judge(linux(
        Some(synced(900)),
        &[TimeDaemon::Chronyd, TimeDaemon::Phc2sys],
        ntp_server(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
}

/// systemd-timesyncd is NTP-disciplined with no bound (it sets the kernel's
/// `maxerror` to 0 at every update), and the warning says it is SNTP and
/// slews, quoting no measured rate; unsynchronised while it has not synced.
#[test]
fn systemd_timesyncd_has_no_bound_and_slews() -> R {
    let t = judge(linux(
        Some(synced(3_500)),
        &[TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(t.source, "systemd-timesyncd");
    assert_eq!(t.bound, None);
    let f = verdict_finding(&t)?;
    assert!(f.is_warning());
    for want in ["SNTP", "no error bound", "slews"] {
        assert!(f.message.contains(want), "no {want:?} in {f}");
    }
    assert!(!f.message.contains("ppm"), "a measured rate in {f}");
    let t = judge(linux(
        Some(unsynced(16_000_000)),
        &[TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
    Ok(())
}

/// ntpd keeps the kernel's `maxerror`, so it is the bound.
#[test]
fn ntpd_is_bounded_by_the_kernels_maxerror() -> R {
    let t = judge(linux(
        Some(synced(1_200)),
        &[TimeDaemon::Ntpd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(
        t.bound.map(|b| (b.source, b.ns)),
        Some((BoundSource::KernelMaxerror, 1_200_000))
    );
    let f = verdict_finding(&t)?;
    assert!(f.message.contains("1.20 ms"), "{f}");
    Ok(())
}

/// chronyd running without answering on its command port (`cmdport 0`,
/// another network namespace): with no PHC on the host it can only be NTP
/// (bounded by the kernel's `maxerror`); with a PHC present it is unknown
/// whether it is locked to it; with `STA_UNSYNC` set but `maxerror` kept,
/// unknown (chronyd leaves the flag set without `rtcsync`); with `maxerror`
/// at the kernel's limit, unsynchronised.
#[test]
fn chronyd_silent_on_its_command_port_is_judged_from_the_kernel() -> R {
    let t = judge(linux(
        Some(synced(800)),
        &[TimeDaemon::Chronyd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(
        t.bound.map(|b| (b.source, b.ns)),
        Some((BoundSource::KernelMaxerror, 800_000))
    );
    assert!(
        t.reason.contains("recv: connection refused"),
        "{}",
        t.reason
    );
    let t = judge(linux(
        Some(synced(800)),
        &[TimeDaemon::Chronyd],
        refused(),
        vec![ptp("ptp0", "hyperv")],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert!(verdict_finding(&t)?.is_unknown());
    let t = judge(linux(
        Some(unsynced(5_000)),
        &[TimeDaemon::Chronyd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert!(t.reason.contains("rtcsync"), "{}", t.reason);
    let t = judge(linux(
        Some(unsynced(16_000_000)),
        &[TimeDaemon::Chronyd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
    Ok(())
}

/// systemd-timesyncd zeroes the kernel's `maxerror` at every update, so on
/// a host where it runs beside chronyd the kernel's figure is no bound.
#[test]
fn maxerror_is_no_bound_where_systemd_timesyncd_also_runs() {
    let t = judge(linux(
        Some(synced(800)),
        &[TimeDaemon::Chronyd, TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(t.bound, None);
}

/// A PHC reference clock whose leap status is unsynchronised (3) is not a
/// synchronised source: not PTP-grade.
#[test]
fn a_phc_refclock_with_an_unsynchronised_leap_status_is_not_ptp_grade() {
    let answer = ChronyAnswer::Tracking(tracking(
        ChronyReference::Refclock(RefclockId::new(PHC0)),
        3,
    ));
    let t = judge(linux(
        Some(synced(450)),
        &[TimeDaemon::Chronyd],
        answer,
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised, "{}", t.reason);
}

/// No daemon visible: the kernel synchronised means one this process cannot
/// see (another PID namespace, `hidepid`), so unknown, never a warning;
/// `STA_UNSYNC` with `maxerror` at the limit is unsynchronised (a warning);
/// `STA_UNSYNC` with `maxerror` kept is unknown.
#[test]
fn with_no_daemon_visible_only_the_kernels_limit_is_a_warning() -> R {
    let t = judge(linux(Some(synced(900)), &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert_eq!(t.source, "kernel");
    assert_eq!(t.bound, None);
    assert!(verdict_finding(&t)?.is_unknown());
    assert!(t.reason.contains("not visible"), "{}", t.reason);
    let t = judge(linux(Some(unsynced(16_000_000)), &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
    assert!(verdict_finding(&t)?.is_warning());
    let t = judge(linux(Some(unsynced(4_000)), &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    Ok(())
}

/// What could not be read is unknown: `/proc` unlisted with the kernel
/// synchronised, or `adjtimex` unread with nothing else to go on.
#[test]
fn what_cannot_be_read_is_unknown() -> R {
    let mut o = linux(Some(synced(900)), &[], refused(), vec![]);
    o.daemons = None;
    assert_eq!(judge(o).verdict, SyncVerdict::Unknown);
    let t = judge(linux(None, &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    let f = verdict_finding(&t)?;
    assert!(f.is_unknown() && !f.is_warning(), "{f}");
    Ok(())
}

/// ptp4l without phc2sys disciplines a NIC's clock, not the wall clock
/// (unless it uses software timestamps): unknown.
#[test]
fn ptp4l_alone_is_unknown() {
    let t = judge(linux(
        Some(synced(900)),
        &[TimeDaemon::Ptp4l],
        refused(),
        vec![ptp("ptp0", "ice")],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
}

/// `ClockBound`'s bound, where it trusts it, is the bound reported; a
/// disrupted one is passed over for chrony's.
#[test]
fn clockbounds_bound_is_preferred_where_it_is_trusted() {
    let reading = |status| ClockBoundReading {
        bound_ns: 54_321,
        status,
        age_ns: 1_000_000,
        max_drift_ppb: 50_000,
    };
    let mut o = linux(
        Some(synced(450)),
        &[TimeDaemon::Chronyd, TimeDaemon::ClockBound],
        phc0(),
        vec![],
    );
    o.clockbound = ClockBoundAnswer::Reading(reading(ClockBoundStatus::Synchronized));
    let t = judge(o.clone());
    assert_eq!(
        t.bound.map(|b| (b.source, b.ns)),
        Some((BoundSource::ClockBound, 54_321))
    );
    o.clockbound = ClockBoundAnswer::Reading(reading(ClockBoundStatus::Disrupted));
    assert_eq!(judge(o).bound.map(|b| b.source), Some(BoundSource::Chrony));
}

/// phc2sys beside another client that disciplines `CLOCK_REALTIME` is not
/// shown to be PTP-grade: unknown, never PTP-grade; with `maxerror` at the
/// kernel's limit nobody keeps the clock: unsynchronised. A chronyd that
/// answered unsynchronised disciplines nothing and is no rival.
#[test]
fn phc2sys_beside_another_client_is_not_shown_ptp_grade() -> R {
    for rival in [
        TimeDaemon::SystemdTimesyncd,
        TimeDaemon::Ntpd,
        TimeDaemon::Chronyd,
    ] {
        for kernel in [synced(0), unsynced(4_000)] {
            let t = judge(linux(
                Some(kernel),
                &[TimeDaemon::Phc2sys, rival],
                refused(),
                vec![ptp("ptp0", "mlx5_ptp")],
            ));
            assert_eq!(t.verdict, SyncVerdict::Unknown, "{rival:?}: {}", t.reason);
            assert!(t.reason.contains(rival.name()), "{}", t.reason);
            assert_eq!(t.bound, None);
            assert!(verdict_finding(&t)?.is_unknown());
        }
        let t = judge(linux(
            Some(unsynced(16_000_000)),
            &[TimeDaemon::Phc2sys, rival],
            refused(),
            vec![],
        ));
        assert_eq!(t.verdict, SyncVerdict::Unsynchronised, "{rival:?}");
    }
    let idle = ChronyAnswer::Tracking(tracking(ChronyReference::None, 3));
    let t = judge(linux(
        Some(synced(0)),
        &[TimeDaemon::Chronyd, TimeDaemon::Phc2sys],
        idle,
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::PtpGrade, "{}", t.reason);
    Ok(())
}

/// systemd-timesyncd leaves `STA_UNSYNC` set when the RTC keeps local time,
/// but zeroes `maxerror` at every update: a kept `maxerror` under it is a
/// synchronised clock, NTP-disciplined, not an unsynchronised one.
#[test]
fn systemd_timesyncd_with_a_local_time_rtc_is_not_unsynchronised() -> R {
    let t = judge(linux(
        Some(unsynced(3_500)),
        &[TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined, "{}", t.reason);
    assert_eq!(t.bound, None);
    let f = verdict_finding(&t)?;
    assert!(f.is_warning());
    assert!(
        f.message
            .contains("not disciplined to a PTP hardware clock"),
        "{f}"
    );
    assert!(!f.message.contains("not synchronised ("), "{f}");
    Ok(())
}

/// The systemd-timesyncd sentence is about systemd-timesyncd: a host whose
/// chronyd tracks an NTP server is not told it is SNTP.
#[test]
fn only_a_systemd_timesyncd_host_is_told_about_its_slew() -> R {
    let f = verdict_finding(&judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![],
    )))?;
    assert!(f.is_warning());
    assert!(!f.message.contains("SNTP"), "{f}");
    Ok(())
}

// --- the other T11 findings: unsettled, a wide bound, a leap second --------

/// chronyd tracking an NTP server: `correction_s`, `last_offset_s`, root
/// dispersion, leap status.
fn chrony_ntp(correction_s: f64, last_offset_s: f64, dispersion_s: f64, leap: u16) -> Observed {
    let mut t = tracking(
        ChronyReference::Server(Some(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 123)))),
        leap,
    );
    t.ref_id = 0xA9FE_A97B;
    t.stratum = 4;
    t.correction_s = correction_s;
    t.last_offset_s = last_offset_s;
    t.root_delay_s = 0.0;
    t.root_dispersion_s = dispersion_s;
    linux(
        Some(synced(1_000)),
        &[TimeDaemon::Chronyd],
        ChronyAnswer::Tracking(t),
        vec![],
    )
}

/// chronyd locked to PHC0 with a root dispersion of `dispersion_s`.
fn chrony_phc(dispersion_s: f64) -> Observed {
    let mut t = tracking(ChronyReference::Refclock(RefclockId::new(PHC0)), 0);
    t.correction_s = 0.0;
    t.last_offset_s = 0.0;
    t.root_delay_s = 0.0;
    t.root_dispersion_s = dispersion_s;
    linux(
        Some(synced(450)),
        &[TimeDaemon::Chronyd],
        ChronyAnswer::Tracking(t),
        vec![],
    )
}

fn warnings(o: Observed, limits: TimeSyncLimits) -> Vec<String> {
    judge(o)
        .findings(&limits)
        .iter()
        .filter(|f| f.is_warning())
        .map(|f| f.message.clone())
        .collect()
}

/// The host's clock still being corrected by 120 ms: the verdict's warning
/// and one unsettled warning naming the correction; 0.9 ms only under a
/// lower limit; a large last update alone.
#[test]
fn an_unsettled_host_warns_with_the_correction() {
    let w = warnings(chrony_ntp(0.120, 0.0, 0.001, 0), TimeSyncLimits::default());
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(
        w[1].contains("120.00 ms") && w[1].contains("not settled"),
        "{w:?}"
    );
    let small = chrony_ntp(0.000_9, 0.0, 0.000_05, 0);
    assert_eq!(warnings(small.clone(), TimeSyncLimits::default()).len(), 1);
    let w = warnings(
        small,
        TimeSyncLimits {
            warn_over_ns: 500_000,
        },
    );
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(w[1].contains("900.0 µs"), "{w:?}");
    let w = warnings(chrony_ntp(0.0, 0.5, 0.000_05, 0), TimeSyncLimits::default());
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(w[1].contains("last update moved it by 500.00 ms"), "{w:?}");
}

/// Without chronyd's report, the kernel PLL's pending offset counts.
#[test]
fn without_chronyd_the_kernel_offset_counts() {
    let mut k = synced(1_000);
    k.offset_ns = -3_000_000;
    let w = warnings(
        linux(Some(k), &[TimeDaemon::Ntpd], refused(), vec![]),
        TimeSyncLimits::default(),
    );
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(
        w[1].contains("the kernel's PLL") && w[1].contains("3.00 ms"),
        "{w:?}"
    );
}

/// A wide bound where the verdict does not warn (PTP-grade, unknown) is its
/// own warning: the bound was read, so it is not an unknown. Where the
/// verdict warns (NTP-disciplined) the bound is in that warning, not twice.
#[test]
fn a_wide_bound_warns_where_the_verdict_does_not() {
    let t = judge(chrony_phc(0.012));
    let all = findings(&t);
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(all[0].status, Status::Ok);
    assert!(all[1].is_warning());
    assert_eq!(
        all[1].message,
        "CLOCK_REALTIME is known to UTC only within 12.00 ms (chrony): stamps compared with \
         other hosts can differ by that much"
    );
    assert_eq!(
        warnings(chrony_phc(0.000_5), TimeSyncLimits::default()).len(),
        0
    );
    let w = warnings(chrony_ntp(0.0, 0.0, 0.012, 0), TimeSyncLimits::default());
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].contains("12.00 ms"), "{w:?}");
    let mut unknown = linux(Some(synced(900)), &[], refused(), vec![]);
    unknown.clockbound = ClockBoundAnswer::Reading(ClockBoundReading {
        bound_ns: 5_000_000,
        status: ClockBoundStatus::FreeRunning,
        age_ns: 6_000_000_000,
        max_drift_ppb: 50_000,
    });
    let t = judge(unknown);
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    let all = findings(&t);
    assert!(all[0].is_unknown(), "{all:?}");
    assert!(
        matches!(&all[0].status, Status::Unknown(why) if why.contains("5.00 ms (clockbound)")),
        "{all:?}"
    );
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(
        all[1].is_warning() && all[1].message.contains("5.00 ms"),
        "{all:?}"
    );
}

/// A pending leap second warns, from chronyd's leap status or the kernel's
/// `STA_INS`/`STA_DEL` or its `TIME_INS`..`TIME_OOP` state.
#[test]
fn a_pending_leap_second_warns() {
    let w = warnings(chrony_ntp(0.0, 0.0, 0.000_05, 1), TimeSyncLimits::default());
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(
        w[1].starts_with("a leap second is pending (chronyd)"),
        "{w:?}"
    );
    for (status, state) in [(0x0010, 0), (0x0020, 0), (0, 1), (0, 3)] {
        let mut k = synced(1_000);
        k.status |= status;
        k.state = state;
        let w = warnings(
            linux(Some(k), &[TimeDaemon::Ntpd], refused(), vec![]),
            TimeSyncLimits::default(),
        );
        assert_eq!(w.len(), 2, "{status:#x} {state}: {w:?}");
        assert!(
            w[1].starts_with("a leap second is pending (adjtimex)"),
            "{w:?}"
        );
    }
}

/// An unsynchronised clock's warning is the only one besides a leap second:
/// its offset and bound say nothing more.
#[test]
fn unsynchronised_is_not_also_unsettled() {
    let mut k = unsynced(16_000_000);
    k.offset_ns = 50_000_000;
    let w = warnings(
        linux(Some(k), &[], refused(), vec![]),
        TimeSyncLimits::default(),
    );
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with("CLOCK_REALTIME is not synchronised"),
        "{w:?}"
    );
}

/// The texts of the unsettled, wide-bound and leap-second warnings, word for
/// word: they cite what the kernel and chrony document, no measured number.
#[test]
fn the_t11_texts_word_for_word() {
    let w = warnings(chrony_ntp(0.120, 0.0, 0.001, 1), TimeSyncLimits::default());
    assert_eq!(
        w[1..],
        [
            "CLOCK_REALTIME is not settled: chronyd is still correcting it by 120.00 ms, so \
             UTC stamps are off by up to that until it is slewed out. Start once time sync \
             has settled."
                .to_owned(),
            "a leap second is pending (chronyd): at midnight UTC the kernel steps \
             CLOCK_REALTIME back 1 s for an inserted second (forward for a deleted one), \
             unless the daemon slews it instead (chrony `leapsecmode slew`), so UTC stamps \
             around it repeat, jump or are off by up to 1 s. Measure durations with a \
             monotonic clock, never by subtracting CLOCK_REALTIME stamps."
                .to_owned(),
        ]
    );
    let w = warnings(
        chrony_ntp(0.0, -0.5, 0.000_05, 0),
        TimeSyncLimits::default(),
    );
    assert_eq!(
        w[1],
        "CLOCK_REALTIME is not settled: chronyd's last update moved it by 500.00 ms; time \
         sync may not have settled. Start once it has."
    );
}

/// Every T11 finding names its evidence: the kernel's and chrony's
/// documentation, nothing measured here.
#[test]
fn the_t11_evidence_is_documentation() -> R {
    let f = verdict_finding(&judge(chrony_ntp(0.0, 0.0, 0.000_05, 0)))?;
    for want in [
        "kernel/time/ntp.c",
        "chronyc.adoc",
        "chrony.conf.adoc",
        "not measured",
    ] {
        assert!(f.evidence.contains(want), "no {want:?} in {}", f.evidence);
    }
    assert!(!f.evidence.contains("BENCHMARKS"), "{}", f.evidence);
    Ok(())
}

// --- the readers on fixture trees and a fake chronyd ------------------------

/// A probe that asks no chronyd and judges `ClockBound` at `now`.
fn probe_at(now_ns: u64) -> TimeSyncProbe {
    TimeSyncProbe {
        chrony: None,
        clockbound_now_ns: Some(now_ns),
        ..TimeSyncProbe::default()
    }
}

/// The daemons are read from `comm` under the tree's `/proc`: each once,
/// sorted, only numeric entries, `systemd-timesyncd` by its 15-byte name, a
/// process with no `comm` passed over; no `/proc` is `None`.
#[test]
fn daemons_are_read_from_a_fake_proc() -> R {
    let f = Fixture::empty("time-proc")?;
    f.file("/proc/1/comm", "systemd\n")?
        .file("/proc/42/comm", "phc2sys\n")?
        .file("/proc/77/comm", "systemd-timesyn\n")?
        .file("/proc/78/comm", "chronyd\n")?
        .file("/proc/79/comm", "chronyd\n")?
        .file("/proc/80/comm", "chronyc\n")?
        .file("/proc/self/comm", "ntpd\n")?
        .file("/proc/sys/comm", "ptp4l\n")?
        .dir("/proc/81")?;
    let probe = probe_at(0);
    assert_eq!(
        probe.daemons(&f.host()),
        Some(vec![
            TimeDaemon::Chronyd,
            TimeDaemon::SystemdTimesyncd,
            TimeDaemon::Phc2sys
        ])
    );
    let empty = Fixture::empty("time-noproc")?;
    assert_eq!(probe.daemons(&empty.host()), None);
    Ok(())
}

/// A zombie (or dead) process is not a running daemon; one without a
/// readable `stat` still counts.
#[test]
fn a_zombie_daemon_is_not_running() -> R {
    let f = Fixture::empty("time-zombie")?;
    f.file("/proc/90/comm", "phc2sys\n")?
        .file("/proc/90/stat", "90 (phc2sys) Z 1 90 90 0 -1 4227332 0 0\n")?
        .file("/proc/91/comm", "chronyd\n")?
        .file("/proc/91/stat", "91 (chronyd) S 1 91 91 0 -1 4210944 0 0\n")?
        .file("/proc/92/comm", "ntpd\n")?
        .file("/proc/92/stat", "92 (ntpd) X 1 92 92 0 -1 0 0 0\n")?
        .file("/proc/93/comm", "ptp4l\n")?;
    assert_eq!(
        probe_at(0).daemons(&f.host()),
        Some(vec![TimeDaemon::Chronyd, TimeDaemon::Ptp4l])
    );
    Ok(())
}

/// The `/proc` scan stops at `max_processes` entries.
#[test]
fn the_proc_scan_is_capped() -> R {
    let f = Fixture::empty("time-proc-cap")?;
    f.file("/proc/10/comm", "chronyd\n")?
        .file("/proc/11/comm", "ntpd\n")?
        .file("/proc/12/comm", "phc2sys\n")?;
    let one = TimeSyncProbe {
        max_processes: 1,
        ..probe_at(0)
    };
    assert_eq!(one.daemons(&f.host()).map(|d| d.len()), Some(1));
    let none = TimeSyncProbe {
        max_processes: 0,
        ..probe_at(0)
    };
    assert_eq!(none.daemons(&f.host()), Some(Vec::new()));
    Ok(())
}

/// The PHCs are read from the tree's `/sys/class/ptp`, by device, names
/// trimmed, a missing name `-`; no directory is no PHC.
#[test]
fn ptp_clocks_are_read_from_a_fake_sysfs() -> R {
    let f = Fixture::empty("time-ptp")?;
    f.file("/sys/class/ptp/ptp1/clock_name", "hyperv\n")?
        .file("/sys/class/ptp/ptp0/clock_name", "ena-ptp-00\n")?
        .file("/sys/class/ptp/other/clock_name", "x\n")?
        .dir("/sys/class/ptp/ptp2")?;
    let probe = probe_at(0);
    assert_eq!(
        probe.ptp_clocks(&f.host()),
        vec![
            ptp("ptp0", "ena-ptp-00"),
            ptp("ptp1", "hyperv"),
            ptp("ptp2", "-")
        ]
    );
    let empty = Fixture::empty("time-noptp")?;
    assert_eq!(probe.ptp_clocks(&empty.host()), Vec::<PtpClock>::new());
    Ok(())
}

/// A `ClockBound` 2.0 segment as its daemon writes one.
fn clockbound_segment(
    generation: u16,
    as_of_ns: u64,
    bound: i64,
    drift_ppb: u32,
    status: i32,
) -> Vec<u8> {
    let mut s = Vec::with_capacity(80);
    s.extend(0x414D_5A4E_u32.to_ne_bytes());
    s.extend(0x4342_0200_u32.to_ne_bytes());
    s.extend(80_u32.to_ne_bytes());
    s.extend(2_u16.to_ne_bytes());
    s.extend(generation.to_ne_bytes());
    for ns in [as_of_ns, as_of_ns + 60_000_000_000] {
        s.extend(i64::try_from(ns / 1_000_000_000).unwrap_or(0).to_ne_bytes());
        s.extend(i64::try_from(ns % 1_000_000_000).unwrap_or(0).to_ne_bytes());
    }
    s.extend(bound.to_ne_bytes());
    s.extend(7_u64.to_ne_bytes());
    s.extend(drift_ppb.to_ne_bytes());
    s.extend(status.to_ne_bytes());
    s.extend([1_u8, 0, 0, 0, 0, 0, 0, 0]);
    s
}

const NOW: u64 = 1_000 * 1_000_000_000;

/// `ClockBound`'s segment under the tree: absent, a settled segment written
/// a second before the `now` given (its bound grown by its drift rate
/// since), a truncated one, and one mid-update, each as it is. On a tree
/// with no `now` given there is no clock to judge it against.
#[test]
fn clockbound_is_read_from_a_fake_segment() -> R {
    let f = Fixture::empty("time-clockbound")?;
    let host = f.host();
    let probe = probe_at(NOW);
    assert_eq!(probe.clockbound(&host), ClockBoundAnswer::Absent);
    let path = f.root().join("var/run/clockbound/shm0");
    std::fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    std::fs::write(
        &path,
        clockbound_segment(10, NOW - 1_000_000_000, 20_000, 50_000, 1),
    )?;
    match probe.clockbound(&host) {
        ClockBoundAnswer::Reading(r) => {
            assert_eq!(r.status, ClockBoundStatus::Synchronized);
            assert_eq!(r.max_drift_ppb, 50_000);
            // 20 µs plus 50 ppm of a second
            assert_eq!(r.bound_ns, 70_000, "{r:?}");
            assert_eq!(r.age_ns, 1_000_000_000);
        }
        other => return Err(format!("{other:?}").into()),
    }
    let no_clock = TimeSyncProbe {
        clockbound_now_ns: None,
        ..probe
    };
    assert_eq!(
        no_clock.clockbound(&host),
        ClockBoundAnswer::Unusable(ClockBoundError::NoClock)
    );
    std::fs::write(
        &path,
        &clockbound_segment(10, NOW - 1_000_000_000, 20_000, 50_000, 1)[..40],
    )?;
    assert_eq!(
        probe.clockbound(&host),
        ClockBoundAnswer::Unusable(ClockBoundError::Short(40))
    );
    assert!(ClockBoundError::Short(40).to_string().contains("shorter"));
    std::fs::write(
        &path,
        clockbound_segment(11, NOW - 1_000_000_000, 20_000, 50_000, 1),
    )?;
    assert_eq!(
        probe.clockbound(&host),
        ClockBoundAnswer::Unusable(ClockBoundError::NeverSettled)
    );
    let off = TimeSyncProbe {
        clockbound: false,
        ..probe
    };
    assert_eq!(off.clockbound(&host), ClockBoundAnswer::NotAsked);
    Ok(())
}

/// On a tree the kernel is not asked (`adjtimex` answers for the running
/// host only), and the rest is read from the tree: the verdict follows.
#[test]
fn a_tree_is_judged_without_the_kernel() -> R {
    let f = Fixture::empty("time-tree")?;
    f.file("/proc/78/comm", "chronyd\n")?;
    let host = f.host();
    assert_eq!(
        KernelSync::read(&host),
        Err(ReadError::NotLive("adjtimex(2)"))
    );
    let probe = probe_at(NOW);
    let o = probe.observe(&host);
    assert!(o.linux);
    assert_eq!(o.kernel, None);
    assert_eq!(o.daemons, Some(vec![TimeDaemon::Chronyd]));
    assert_eq!(o.chrony, ChronyAnswer::NotAsked);
    assert_eq!(o.clockbound, ClockBoundAnswer::Absent);
    let all = t11_time_sync(&host, &probe);
    assert_eq!(all.len(), 1, "{all:?}");
    assert!(all[0].is_unknown(), "{}", all[0]);
    Ok(())
}

/// A fake chronyd on the loopback: answers each request it receives with
/// `answer(request)`.
fn fake_chronyd(
    replies: usize,
    answer: impl Fn(&[u8]) -> Vec<Vec<u8>> + Send + 'static,
) -> std::io::Result<(SocketAddr, std::thread::JoinHandle<Vec<Vec<u8>>>)> {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    let addr = socket.local_addr()?;
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let mut buf = [0_u8; 1024];
        for _ in 0..replies {
            let Ok((n, from)) = socket.recv_from(&mut buf) else {
                break;
            };
            let request = buf[..n].to_vec();
            for reply in answer(&request) {
                let _ = socket.send_to(&reply, from);
            }
            seen.push(request);
        }
        seen
    });
    Ok((addr, handle))
}

/// chrony's float (`UTI_FloatHostToNetwork`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn float_bits(x: f64) -> u32 {
    let (neg, x) = if x < 0.0 { (1, -x) } else { (0, x) };
    if x < 1.0e-100 {
        return 0;
    }
    let mut exp = x.log2() as i32 + 1;
    let mut coef = x.mul_add(2_f64.powi(-exp + 25), 0.5) as i32;
    while coef > (1 << 24) - 1 + neg {
        coef >>= 1;
        exp += 1;
    }
    if neg == 1 {
        coef = -coef;
    }
    ((exp as u32 & 0x7f) << 25) | (coef as u32 & 0x1ff_ffff)
}

/// A tracking reply to `request` as chronyd 4.6.1 builds one: a PHC0
/// reference clock, stratum 1, the floats given.
fn tracking_reply(request: &[u8], floats: [f64; 9]) -> Vec<u8> {
    let mut p = vec![0_u8; 104];
    p[0] = 6;
    p[1] = 2;
    p[4..6].copy_from_slice(&33_u16.to_be_bytes());
    p[6..8].copy_from_slice(&5_u16.to_be_bytes());
    p[16..20].copy_from_slice(request.get(8..12).unwrap_or(&[0; 4]));
    p[28..32].copy_from_slice(b"PHC0");
    p[52..54].copy_from_slice(&1_u16.to_be_bytes());
    for (k, v) in floats.iter().enumerate() {
        p[68 + 4 * k..72 + 4 * k].copy_from_slice(&float_bits(*v).to_be_bytes());
    }
    p
}

const FLOATS: [f64; 9] = [
    1e-7, 2e-7, 3e-7, -11.5, 0.0, 0.002, 0.000_010, 0.000_004, 1.0,
];

fn asking(addr: SocketAddr, timeout: Duration) -> TimeSyncProbe {
    TimeSyncProbe {
        chrony: Some(addr),
        chrony_timeout: timeout,
        ..TimeSyncProbe::default()
    }
}

/// The request is chronyc's, padded to 104 bytes; a well-formed reply
/// parses, and a stray reply to another request is read past.
#[test]
fn a_fake_chronyd_is_asked_and_answers() -> R {
    let (addr, server) = fake_chronyd(1, |req| {
        let mut stray = tracking_reply(req, FLOATS);
        stray[19] ^= 0xff; // another sequence
        vec![stray, tracking_reply(req, FLOATS)]
    })?;
    let answer = asking(addr, Duration::from_millis(500)).chrony();
    let requests = server.join().map_err(|_| "the fake chronyd failed")?;
    let request = requests.first().ok_or("no request")?;
    assert_eq!(request.len(), 104);
    assert_eq!(&request[..6], &[6, 1, 0, 0, 0, 33]);
    match answer {
        ChronyAnswer::Tracking(t) => {
            assert_eq!(
                t.reference,
                ChronyReference::Refclock(RefclockId::new(PHC0))
            );
            assert_eq!(t.reference_name(), "PHC0");
            assert!(t.is_phc() && t.synchronised());
            assert!((t.freq_ppm + 11.5).abs() < 1e-5);
            // 0.1 + 4 + 5 µs
            assert!(t.bound_ns().abs_diff(9_100) <= 2, "{}", t.bound_ns());
        }
        other => return Err(format!("{other:?}").into()),
    }
    Ok(())
}

/// Malformed replies come back as malformed, with the fault: never as a
/// report.
#[test]
fn a_fake_chronyds_malformed_replies_are_refused() -> R {
    type Spoil = fn(&mut Vec<u8>);
    let cases: [(&str, Spoil); 4] = [
        ("not protocol version 6", |p| p[0] = 5),
        ("shorter than a tracking reply", |p| p.truncate(60)),
        ("leap status over 3", |p| p[55] = 9),
        ("unknown address family", |p| p[49] = 7),
    ];
    for (want, spoil) in cases {
        let (addr, server) = fake_chronyd(1, move |req| {
            let mut p = tracking_reply(req, FLOATS);
            spoil(&mut p);
            vec![p]
        })?;
        let answer = asking(addr, Duration::from_millis(500)).chrony();
        server.join().map_err(|_| "the fake chronyd failed")?;
        assert_eq!(answer, ChronyAnswer::Failed(ChronyError::Malformed(want)));
    }
    Ok(())
}

/// A chronyd that never answers costs the timeout and no more; a port
/// nobody listens on fails at once (refused) or within the timeout: the
/// probe never hangs.
#[test]
fn a_silent_or_absent_chronyd_never_hangs() -> R {
    let timeout = Duration::from_millis(50);
    let (addr, server) = fake_chronyd(1, |_| Vec::new())?;
    let started = Instant::now();
    let answer = asking(addr, timeout).chrony();
    let took = started.elapsed();
    server.join().map_err(|_| "the fake chronyd failed")?;
    assert_eq!(answer, ChronyAnswer::Failed(ChronyError::Timeout(timeout)));
    assert!(answer_text(answer).contains("no reply within 50 ms"));
    assert!(
        took >= timeout && took < timeout + Duration::from_millis(200),
        "{took:?}"
    );
    // a port just closed: nobody listens
    let closed = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?.local_addr()?;
    let started = Instant::now();
    let answer = asking(closed, timeout).chrony();
    let took = started.elapsed();
    assert!(
        matches!(
            answer,
            ChronyAnswer::Failed(ChronyError::Io { .. } | ChronyError::Timeout(_))
        ),
        "{answer:?}"
    );
    assert!(took < timeout + Duration::from_millis(200), "{took:?}");
    Ok(())
}

fn answer_text(answer: ChronyAnswer) -> String {
    match answer {
        ChronyAnswer::Failed(e) => e.to_string(),
        other => format!("{other:?}"),
    }
}

// --- the real readers -------------------------------------------------------

/// The real readers on this host: bounded, and judged by the same rules.
/// Off Linux nothing is read and T11 is unknown, `not Linux`. On Linux the
/// kernel's state reads unprivileged and `/proc` lists; chronyd's command
/// port never gives a malformed reply; with no time daemon visible the
/// verdict is the kernel's alone.
#[test]
fn the_real_readers_are_bounded_and_agree_with_the_rules() {
    let host = Host::system();
    let started = Instant::now();
    let t = time_sync(&host);
    let took = started.elapsed();
    assert!(took < Duration::from_secs(1), "{took:?}");
    let o = &t.observed;
    let all = findings(&t);
    for f in &all {
        println!("{f}");
    }
    if !cfg!(target_os = "linux") {
        assert_eq!(t.verdict, SyncVerdict::Unknown);
        assert_eq!(*o, Observed::not_linux());
        assert_eq!(all.len(), 1);
        assert!(
            matches!(&all[0].status, Status::Unknown(why) if why.starts_with("not Linux")),
            "{}",
            all[0]
        );
        return;
    }
    assert!(o.linux);
    let kernel = o.kernel.as_ref();
    assert!(kernel.is_some(), "adjtimex(modes=0) must read unprivileged");
    assert!(o.daemons.is_some(), "/proc must list");
    assert!(
        !matches!(o.chrony, ChronyAnswer::Failed(ChronyError::Malformed(_))),
        "{:?}",
        o.chrony
    );
    if o.daemons.as_ref().is_some_and(Vec::is_empty)
        && !matches!(o.chrony, ChronyAnswer::Tracking(_))
    {
        let want = match kernel {
            Some(k) if k.unsynchronised() && !k.maxerror_kept() => SyncVerdict::Unsynchronised,
            _ => SyncVerdict::Unknown,
        };
        assert_eq!(t.verdict, want, "{}", t.reason);
    }
}

/// `KernelSync::read` is `adjtimex` with `modes` 0 on the running Linux
/// host (unprivileged; a state from `TIME_OK` to `TIME_ERROR`), `not Linux`
/// elsewhere.
#[test]
fn the_kernels_state_reads_on_the_running_host() {
    match KernelSync::read(&Host::system()) {
        Ok(k) => {
            assert_eq!(std::env::consts::OS, "linux", "{k:?}");
            assert!((0..=5).contains(&k.state), "{k:?}");
            println!("{k:?}");
        }
        Err(e) => assert_eq!(e, ReadError::NotLinux("adjtimex(2)"), "{e}"),
    }
}

// --- T12 --------------------------------------------------------------------

fn x86(name: &str, invariant: Option<bool>) -> Clocksource {
    Clocksource {
        name: Ok(name.to_owned()),
        arch: Arch::X86_64,
        invariant_tsc: invariant,
    }
}

/// T12: ok for the counter itself (tsc, `arch_sys_counter`); info for a
/// hypervisor's view of it; a warning for any other source (hpet,
/// `acpi_pm`), whose reads are system calls, with the fix for the
/// architecture; unknown where the name cannot be read.
#[test]
fn the_clocksource_finding_follows_the_source() {
    for name in ["tsc", "tsc\n"] {
        let f = x86(name, Some(true)).finding();
        assert_eq!((f.id, &f.status), (CheckId::T12, &Status::Ok), "{f}");
    }
    for source in ["hpet", "acpi_pm"] {
        let f = x86(source, Some(true)).finding();
        assert!(f.is_warning(), "{f}");
        for want in [
            source,
            "system call",
            "clocksource=tsc tsc=reliable",
            "Do not force it as a fix",
            "turns off the kernel's TSC checks",
            "never on a VM",
            "on the node",
        ] {
            assert!(f.message.contains(want), "no {want:?} in {f}");
        }
    }
    let f = x86("hpet", Some(false)).finding();
    assert!(f.message.contains("not invariant"), "{f}");
    assert!(f.message.contains("forcing the TSC would be wrong"), "{f}");
    let f = x86("hpet", None).finding();
    assert!(f.message.contains("Do not force it as a fix"), "{f}");
    for source in ["kvm-clock", "hyperv_clocksource_tsc_page", "xen"] {
        let f = x86(source, Some(true)).finding();
        assert_eq!(
            (&f.status, f.severity),
            (&Status::Finding, Severity::Info),
            "{f}"
        );
        assert_eq!(ClocksourceKind::of(source), ClocksourceKind::Paravirt);
    }
    let unread = Clocksource {
        name: Err(ReadError::NotLinux("the kernel's clock source")),
        arch: Arch::Unknown,
        invariant_tsc: None,
    };
    let f = unread.finding();
    assert!(f.is_unknown() && !f.is_warning(), "{f}");
    let arm = |name: &str| Clocksource {
        name: Ok(name.to_owned()),
        arch: Arch::Aarch64,
        invariant_tsc: None,
    };
    assert_eq!(arm("arch_sys_counter").finding().status, Status::Ok);
    let f = arm("arch_mem_counter").finding();
    assert!(
        f.is_warning() && f.message.contains("clocksource=arch_sys_counter"),
        "{f}"
    );
    assert_eq!(ClocksourceKind::of("tsc"), ClocksourceKind::Counter);
    assert_eq!(ClocksourceKind::of("jiffies"), ClocksourceKind::Other);
}

/// T12 knows the counter of x86-64 and 64-bit Arm only. Elsewhere (RISC-V's
/// `riscv_clocksource`, PowerPC's `timebase`, both read through the vDSO),
/// or with no `/proc/cpuinfo` to tell, a source it does not know is unknown,
/// never a warning; the counter's own names stay ok.
#[test]
fn t12_on_an_architecture_it_does_not_know_is_unknown() {
    let other = |name: &str| Clocksource {
        name: Ok(name.to_owned()),
        arch: Arch::Unknown,
        invariant_tsc: None,
    };
    for name in ["riscv_clocksource", "timebase", "hpet"] {
        let f = other(name).finding();
        assert_eq!(f.id, CheckId::T12);
        assert!(f.is_unknown() && !f.is_warning(), "{f}");
        assert!(
            matches!(&f.status, Status::Unknown(why) if why.contains(name)),
            "{f}"
        );
    }
    assert_eq!(other("tsc").finding().status, Status::Ok);
}

/// T12 reads the tree: the clock source and, for the fix, `/proc/cpuinfo`'s
/// architecture and `constant_tsc nonstop_tsc`; no clock source file is
/// unknown.
#[test]
fn t12_reads_the_tree() -> R {
    let f = Fixture::empty("time-clocksource")?;
    f.file(
        "/sys/devices/system/clocksource/clocksource0/current_clocksource",
        "hpet\n",
    )?
    .file(
        "/proc/cpuinfo",
        "processor\t: 0\nvendor_id\t: GenuineIntel\nflags\t\t: fpu tsc constant_tsc nonstop_tsc\n",
    )?;
    let c = Clocksource::read(&f.host());
    assert_eq!(c.name, Ok("hpet".to_owned()));
    assert_eq!((c.arch, c.invariant_tsc), (Arch::X86_64, Some(true)));
    let finding = t12_clocksource(&f.host());
    assert!(finding.is_warning(), "{finding}");
    f.file(
        "/proc/cpuinfo",
        "processor\t: 0\nvendor_id\t: GenuineIntel\nflags\t\t: fpu tsc\n",
    )?;
    assert_eq!(Clocksource::read(&f.host()).invariant_tsc, Some(false));
    let empty = Fixture::empty("time-noclocksource")?;
    assert!(t12_clocksource(&empty.host()).is_unknown());
    Ok(())
}

/// The running host's clock source: a finding or ok on Linux, unknown (not
/// Linux) elsewhere.
#[test]
fn t12_answers_on_the_running_host() {
    let f = t12_clocksource(&Host::system());
    println!("{f}");
    assert_eq!(f.id, CheckId::T12);
    if cfg!(target_os = "linux") {
        assert!(!f.is_unknown(), "{f}");
    } else {
        assert_eq!(
            f.status,
            Status::Unknown(ReadError::NotLinux("the host's /sys and /proc").to_string())
        );
    }
}

// --- the switch -------------------------------------------------------------

/// Under `require` a T11 warning refuses and a T11 unknown passes (a pod
/// normally cannot see the time daemons), while a T12 unknown refuses (every
/// Linux host has the file). Under `off` the T11 warning line is dropped and
/// the unknown's info line stays.
#[test]
fn require_refuses_a_t11_warning_and_a_t12_unknown_only() {
    assert!(!CheckId::T11.unknown_stops_require());
    assert!(CheckId::T12.unknown_stops_require());
    let warn = findings(&judge(chrony_ntp(0.0, 0.0, 0.000_05, 0)));
    let unknown = findings(&judge(Observed::not_linux()));
    let t12_unknown = Clocksource {
        name: Err(ReadError::NotLinux("the host's /sys and /proc")),
        arch: Arch::Unknown,
        invariant_tsc: None,
    }
    .finding();
    let require = HostCheckPolicy::Require;
    assert!(require.refusal(&warn).is_some());
    assert_eq!(require.refusal(&unknown), None);
    assert!(
        require
            .refusal(std::slice::from_ref(&t12_unknown))
            .is_some()
    );
    let off = HostCheckPolicy::Off;
    assert_eq!(off.lines("t", &warn), Vec::<String>::new());
    let lines = off.lines("t", &unknown);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("t info: T11 unknown: "), "{lines:?}");
}
