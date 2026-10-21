//! How the wall clock is kept: the verdict on injected observations
//! (`adjtimex` results, chronyd's answers, daemons, PTP hardware clocks,
//! `ClockBound`), the readers against fixture trees and a fake chronyd, and
//! the real readers on this host (bounded: nothing hangs).

use ergon_clock::advanced::{
    Arch, BoundSource, Check, ChronyAnswer, ChronyError, ChronyReference, ChronyTracking,
    ClockBoundAnswer, ClockBoundReading, ClockBoundStatus, HostFacts, KernelSync, Observed, Os,
    PtpClock, STA_UNSYNC, Severity, SyncVerdict, TimeDaemon, TimeSync, TimeSyncPolicy,
    TimeSyncProbe, calibrate, time_sync,
};
use ergon_clock::{Config, Error};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

fn phc0() -> ChronyAnswer {
    ChronyAnswer::Tracking(tracking(ChronyReference::Refclock("PHC0".to_owned()), 0))
}

const fn ntp_server() -> ChronyAnswer {
    let mut t = tracking(
        ChronyReference::Server(Some(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 123)))),
        0,
    );
    t.stratum = 4;
    t.root_delay_s = 0.000_400;
    t.root_dispersion_s = 0.000_950;
    ChronyAnswer::Tracking(t)
}

fn refused() -> ChronyAnswer {
    ChronyAnswer::Failed(ChronyError::Io("recv: Connection refused".to_owned()))
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

/// Off Linux nothing is read and nothing is judged: unknown, with the
/// reason, at info (never a warning).
#[test]
fn off_linux_the_verdict_is_unknown_with_its_reason() {
    let t = judge(Observed::not_linux());
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert_eq!(t.bound, None);
    let f = t.finding();
    assert_eq!((f.check, f.severity), (Check::TimeSync, Severity::Info));
    assert!(f.message.contains("not Linux"), "{f}");
}

/// chronyd locked to a PHC reference clock is PTP-grade, with chrony's
/// bound (|offset| + dispersion + delay / 2 = 2 + 3 + 5 µs), at info.
#[test]
fn chronyd_locked_to_a_phc_is_ptp_grade() {
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
    let f = t.finding();
    assert_eq!(f.severity, Severity::Info, "{f}");
    assert!(f.message.contains("PHC0"), "{f}");
}

/// chronyd tracking an NTP server is NTP-disciplined: a warning that names
/// the server and the bound, says UTC stamps are only as good as
/// `CLOCK_REALTIME`, and gives
/// the fix for the PHC the host has (ENA: AWS's refclock line; Hyper-V:
/// Azure's) or, with none, both clouds'.
#[test]
fn chronyd_on_an_ntp_server_warns_with_the_bound_and_the_fix() {
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
    let f = t.finding();
    assert_eq!(f.severity, Severity::Warn);
    for want in [
        "not disciplined to a PTP hardware clock",
        "169.254.169.123",
        "1.15 ms",
        "cross-host",
        "UTC stamps are only as good as CLOCK_REALTIME",
        "refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer",
        "/dev/ptp_hyperv",
    ] {
        assert!(f.message.contains(want), "no {want:?} in {f}");
    }
    let aws = judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![ptp("ptp0", "ena-ptp-00")],
    ))
    .finding();
    assert!(aws.message.contains("ptp0: ena-ptp-00"), "{aws}");
    assert!(aws.message.contains("refclock PHC /dev/ptp_ena"), "{aws}");
    assert!(!aws.message.contains("ptp_hyperv"), "{aws}");
    let azure = judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![ptp("ptp0", "hyperv")],
    ))
    .finding();
    assert!(
        azure
            .message
            .contains("refclock PHC /dev/ptp_hyperv poll 3 dpoll -2 offset 0"),
        "{azure}"
    );
    assert!(!azure.message.contains("ptp_ena"), "{azure}");
}

/// chronyd that reports itself synchronised but never adjusts this
/// kernel's clock (`chronyd -x`: the kernel's `maxerror` stays at its 16 s
/// limit, where chronyd sets it at every update otherwise) is not
/// disciplining `CLOCK_REALTIME`, whatever it is locked to: unsynchronised,
/// never PTP-grade. Seen with chrony 4.6.1 in a container. Without
/// `rtcsync` chronyd leaves `STA_UNSYNC` set but keeps `maxerror`: that is
/// still its word.
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
    let gps = ChronyAnswer::Tracking(tracking(ChronyReference::Refclock("GPS".to_owned()), 0));
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
fn chronyd_unsynchronised_or_local_is_unsynchronised() {
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
        assert_eq!(t.finding().severity, Severity::Warn);
    }
}

/// phc2sys with the kernel synchronised is PTP-grade at info, with no
/// bound (it keeps no `maxerror`, so the kernel's would mislead); with the
/// kernel unsynchronised it has not locked (or disciplines another clock).
#[test]
fn phc2sys_is_ptp_grade_only_with_the_kernel_synchronised() {
    let t = judge(linux(
        Some(synced(0)),
        &[TimeDaemon::Phc2sys, TimeDaemon::Ptp4l],
        refused(),
        vec![ptp("ptp0", "mlx5_ptp")],
    ));
    assert_eq!(t.verdict, SyncVerdict::PtpGrade);
    assert_eq!(t.source, "phc2sys");
    assert_eq!(t.bound, None);
    let f = t.finding();
    assert_eq!(f.severity, Severity::Info);
    assert!(f.message.contains("no error bound"), "{f}");
    let t = judge(linux(
        Some(unsynced(16_000_000)),
        &[TimeDaemon::Phc2sys],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
    assert_eq!(t.finding().severity, Severity::Warn);
}

/// chronyd's own answer outranks a running phc2sys: chronyd on an NTP
/// server disciplines the wall clock, whatever phc2sys does with a PHC.
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

/// systemd-timesyncd is NTP-disciplined with no bound (it zeroes the
/// kernel's `maxerror` at every update), and the warning says it slews by
/// hundreds of ppm after a boot; unsynchronised while it has not synced.
#[test]
fn systemd_timesyncd_has_no_bound_and_slews() {
    let t = judge(linux(
        Some(synced(3_500)),
        &[TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined);
    assert_eq!(t.source, "systemd-timesyncd");
    assert_eq!(t.bound, None);
    let f = t.finding();
    assert_eq!(f.severity, Severity::Warn);
    for want in ["SNTP", "no error bound", "slews", "533 ppm"] {
        assert!(f.message.contains(want), "no {want:?} in {f}");
    }
    let t = judge(linux(
        Some(unsynced(16_000_000)),
        &[TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
}

/// ntpd keeps the kernel's `maxerror`, so it is the bound.
#[test]
fn ntpd_is_bounded_by_the_kernels_maxerror() {
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
    assert!(t.finding().message.contains("1.20 ms"), "{}", t.finding());
}

/// chronyd running without answering on its command port (`cmdport 0`):
/// with no PHC on the host it can only be NTP (bounded by the kernel's
/// `maxerror`, which chronyd keeps); with a PHC present it is unknown
/// whether it is locked to it; with `STA_UNSYNC` set but `maxerror` kept,
/// unknown (chronyd leaves the flag set without `rtcsync`); with `maxerror`
/// at the kernel's limit, unsynchronised.
#[test]
fn chronyd_silent_on_its_command_port_is_judged_from_the_kernel() {
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
    let t = judge(linux(
        Some(synced(800)),
        &[TimeDaemon::Chronyd],
        refused(),
        vec![ptp("ptp0", "hyperv")],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert_eq!(t.finding().severity, Severity::Info);
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
}

/// systemd-timesyncd zeroes the kernel's `maxerror` at every update, so on
/// a host where it runs beside chronyd (two clients fighting over one clock)
/// the kernel's figure is no bound.
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
    let answer = ChronyAnswer::Tracking(tracking(ChronyReference::Refclock("PHC0".to_owned()), 3));
    let t = judge(linux(
        Some(synced(450)),
        &[TimeDaemon::Chronyd],
        answer,
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised, "{}", t.reason);
}

/// No daemon visible: the kernel synchronised means one this process
/// cannot see (another PID namespace, `hidepid`), so unknown at info, never
/// a warning; `STA_UNSYNC` with `maxerror` at the limit is unsynchronised
/// (a warning); `STA_UNSYNC` with `maxerror` kept is unknown.
#[test]
fn with_no_daemon_visible_only_the_kernels_limit_is_a_warning() {
    let t = judge(linux(Some(synced(900)), &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert_eq!(t.source, "kernel");
    // the kernel's maxerror is no bound when what keeps it cannot be seen
    // (systemd-timesyncd and phc2sys zero it)
    assert_eq!(t.bound, None);
    assert_eq!(t.finding().severity, Severity::Info);
    assert!(t.reason.contains("not visible"), "{}", t.reason);
    let t = judge(linux(Some(unsynced(16_000_000)), &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unsynchronised);
    assert_eq!(t.finding().severity, Severity::Warn);
    let t = judge(linux(Some(unsynced(4_000)), &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
}

/// What could not be read is unknown: `/proc` unlisted with the kernel
/// synchronised, or `adjtimex` unread with nothing else to go on.
#[test]
fn what_cannot_be_read_is_unknown() {
    let mut o = linux(Some(synced(900)), &[], refused(), vec![]);
    o.daemons = None;
    assert_eq!(judge(o).verdict, SyncVerdict::Unknown);
    let t = judge(linux(None, &[], refused(), vec![]));
    assert_eq!(t.verdict, SyncVerdict::Unknown);
    assert_eq!(t.finding().severity, Severity::Info);
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
/// disrupted or unknown one is passed over for chrony's.
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

/// The report's fields carry the section without spaces or commas.
#[test]
fn the_sections_fields_have_no_spaces_or_commas() {
    let t = judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd, TimeDaemon::SystemdTimesyncd],
        ntp_server(),
        vec![ptp("ptp0", "odd name, really")],
    ));
    let fields = t.fields();
    for (key, value) in &fields {
        assert!(!value.contains([' ', ',']), "{key}={value}");
    }
    let get = |k: &str| {
        fields
            .iter()
            .find(|(key, _)| *key == k)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(get("time_sync").as_deref(), Some("ntp-disciplined"));
    assert_eq!(
        get("time_sync_source").as_deref(),
        Some("chronyd:169.254.169.123")
    );
    assert_eq!(
        get("time_daemons").as_deref(),
        Some("chronyd;systemd-timesyncd")
    );
    assert_eq!(get("time_sync_bound_from").as_deref(), Some("chrony"));
}

/// K1: the clock source the gate classifies. A warning for a source the
/// kernel's clocks are not read through the vDSO with (hpet, `acpi_pm`), with
/// the fix for the architecture; info for a hypervisor's view of the
/// counter and for an unreadable name; nothing for the counter itself, off
/// Linux, or without a counter.
#[test]
fn the_clocksource_finding_follows_the_gate() {
    let x86 = HostFacts {
        invariant_tsc: Some(true),
        rdtscp: Some(true),
        ..HostFacts::unknown(Arch::X86_64, Os::Linux)
    };
    assert_eq!(x86.clocksource_finding(Some("tsc")), None);
    assert_eq!(x86.clocksource_finding(Some("tsc\n")), None);
    for source in ["hpet", "acpi_pm"] {
        let f = x86.clocksource_finding(Some(source));
        let f = f
            .as_ref()
            .map(|f| (f.check, f.severity, f.message.as_str()));
        assert!(
            matches!(f, Some((Check::Clocksource, Severity::Warn, m)) if m.contains(source)
                && m.contains("system call") && m.contains("clocksource=tsc tsc=reliable")
                && m.contains("Do not force it as a fix") && m.contains("never on a VM")),
            "{f:?}"
        );
    }
    let not_invariant = HostFacts {
        invariant_tsc: Some(false),
        ..x86.clone()
    };
    let f = not_invariant.clocksource_finding(Some("hpet"));
    assert!(f.is_some_and(|f| f.message.contains("not invariant")));
    for source in ["kvm-clock", "hyperv_clocksource_tsc_page", "xen"] {
        let f = x86.clocksource_finding(Some(source));
        assert_eq!(f.map(|f| f.severity), Some(Severity::Info), "{source}");
    }
    assert_eq!(
        x86.clocksource_finding(None).map(|f| f.severity),
        Some(Severity::Info)
    );
    let arm = HostFacts::unknown(Arch::Aarch64, Os::Linux);
    assert_eq!(arm.clocksource_finding(Some("arch_sys_counter")), None);
    let f = arm.clocksource_finding(Some("arch_mem_counter"));
    assert!(f.is_some_and(
        |f| f.severity == Severity::Warn && f.message.contains("clocksource=arch_sys_counter")
    ));
    assert_eq!(
        HostFacts::unknown(Arch::X86_64, Os::MacOs).clocksource_finding(Some("hpet")),
        None
    );
    assert_eq!(
        HostFacts::unknown(Arch::Other, Os::Linux).clocksource_finding(Some("hpet")),
        None
    );
}

/// A scratch directory, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join(format!(
            "ergon-clock-timesync-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    fn file(&self, rel: &str, contents: &[u8]) -> std::io::Result<PathBuf> {
        let path = self.0.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        Ok(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn probe_at(dir: &Path) -> TimeSyncProbe {
    TimeSyncProbe {
        proc_dir: dir.join("proc"),
        ptp_dir: dir.join("ptp"),
        chrony: None,
        clockbound: Some(dir.join("clockbound/shm0")),
        ..TimeSyncProbe::default()
    }
}

/// The daemons are read from `comm` in a fake `/proc`: each once, sorted,
/// only numeric entries, `systemd-timesyncd` by its 15-byte name, a
/// process with no `comm` passed over; no `/proc` is `None`.
#[test]
fn daemons_are_read_from_a_fake_proc() -> R {
    let s = Scratch::new("proc")?;
    s.file("proc/1/comm", b"systemd\n")?;
    s.file("proc/42/comm", b"phc2sys\n")?;
    s.file("proc/77/comm", b"systemd-timesyn\n")?;
    s.file("proc/78/comm", b"chronyd\n")?;
    s.file("proc/79/comm", b"chronyd\n")?;
    s.file("proc/80/comm", b"chronyc\n")?;
    s.file("proc/self/comm", b"ntpd\n")?;
    s.file("proc/sys/comm", b"ptp4l\n")?;
    std::fs::create_dir_all(s.0.join("proc/81"))?;
    let probe = probe_at(&s.0);
    assert_eq!(
        probe.daemons(),
        Some(vec![
            TimeDaemon::Chronyd,
            TimeDaemon::SystemdTimesyncd,
            TimeDaemon::Phc2sys
        ])
    );
    let missing = TimeSyncProbe {
        proc_dir: s.0.join("no-such"),
        ..probe
    };
    assert_eq!(missing.daemons(), None);
    Ok(())
}

/// The PHCs are read from a fake `/sys/class/ptp`, by device, names
/// trimmed, a missing name `-`; no directory is no PHC.
#[test]
fn ptp_clocks_are_read_from_a_fake_sysfs() -> R {
    let s = Scratch::new("ptp")?;
    s.file("ptp/ptp1/clock_name", b"hyperv\n")?;
    s.file("ptp/ptp0/clock_name", b"ena-ptp-00\n")?;
    s.file("ptp/other/clock_name", b"x\n")?;
    std::fs::create_dir_all(s.0.join("ptp/ptp2"))?;
    let probe = probe_at(&s.0);
    assert_eq!(
        probe.ptp_clocks(),
        vec![
            ptp("ptp0", "ena-ptp-00"),
            ptp("ptp1", "hyperv"),
            ptp("ptp2", "-")
        ]
    );
    let missing = TimeSyncProbe {
        ptp_dir: s.0.join("no-such"),
        ..probe
    };
    assert_eq!(missing.ptp_clocks(), Vec::<PtpClock>::new());
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

/// `ClockBound`'s segment, read from a file: absent, a settled segment
/// written a second ago (its bound grown by its drift rate since), a
/// truncated one, and one mid-update, each as it is.
#[test]
fn clockbound_is_read_from_a_fake_segment() -> R {
    let s = Scratch::new("clockbound")?;
    let probe = probe_at(&s.0);
    assert_eq!(probe.clockbound(), ClockBoundAnswer::Absent);
    // a second ago on the clock the reader compares with (the coarse clock
    // lags the fine one by a tick at most)
    let as_of = ergon_clock::advanced::monotonic_ns().saturating_sub(1_000_000_000);
    s.file(
        "clockbound/shm0",
        &clockbound_segment(10, as_of, 20_000, 50_000, 1),
    )?;
    match probe.clockbound() {
        ClockBoundAnswer::Reading(r) => {
            assert_eq!(r.status, ClockBoundStatus::Synchronized);
            assert_eq!(r.max_drift_ppb, 50_000);
            // 20 µs plus 50 ppm of about a second
            assert!((69_000..80_000).contains(&r.bound_ns), "{r:?}");
        }
        other => return Err(format!("{other:?}").into()),
    }
    s.file(
        "clockbound/shm0",
        &clockbound_segment(10, as_of, 20_000, 50_000, 1)[..40],
    )?;
    assert!(
        matches!(probe.clockbound(), ClockBoundAnswer::Unusable(why) if why.contains("shorter"))
    );
    s.file(
        "clockbound/shm0",
        &clockbound_segment(11, as_of, 20_000, 50_000, 1),
    )?;
    assert!(matches!(probe.clockbound(), ClockBoundAnswer::Unusable(_)));
    Ok(())
}

/// A fake chronyd on the loopback: answers each request it receives with
/// `answer(request)`, or with nothing where that is `None`.
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
            assert_eq!(t.reference, ChronyReference::Refclock("PHC0".to_owned()));
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
    assert!(
        matches!(&answer, ChronyAnswer::Failed(ChronyError::Io(why)) if why.contains("no reply")),
        "{answer:?}"
    );
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
        matches!(answer, ChronyAnswer::Failed(ChronyError::Io(_))),
        "{answer:?}"
    );
    assert!(took < timeout + Duration::from_millis(200), "{took:?}");
    Ok(())
}

/// The real readers on this host: bounded, and judged by the same rules.
/// Off Linux nothing is read and the verdict is unknown. On Linux the
/// kernel's state reads unprivileged and `/proc` lists; where no chronyd
/// runs its command port does not answer (never a malformed reply); and
/// with no time daemon visible the verdict is the kernel's alone.
#[test]
fn the_real_readers_are_bounded_and_agree_with_the_rules() {
    let started = Instant::now();
    let t = time_sync();
    let took = started.elapsed();
    assert!(took < Duration::from_secs(1), "{took:?}");
    assert!(t.took <= took);
    let o = &t.observed;
    if !cfg!(target_os = "linux") {
        assert_eq!(t.verdict, SyncVerdict::Unknown);
        assert_eq!(*o, Observed::not_linux());
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
    if !o.runs(TimeDaemon::Chronyd) && !matches!(o.chrony, ChronyAnswer::Tracking(_)) {
        assert!(
            matches!(o.chrony, ChronyAnswer::Failed(ChronyError::Io(_))),
            "{:?}",
            o.chrony
        );
    }
    if o.daemons.as_ref().is_some_and(Vec::is_empty)
        && !matches!(o.chrony, ChronyAnswer::Tracking(_))
    {
        let want = match kernel {
            Some(k) if k.unsynchronised() && !k.maxerror_kept() => SyncVerdict::Unsynchronised,
            _ => SyncVerdict::Unknown,
        };
        assert_eq!(t.verdict, want, "{}", t.finding());
    }
    eprintln!(
        "this host: {} ({})",
        t.finding(),
        t.fields()
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}

/// The calibration report carries the section by default, `off` without
/// it, and `RequirePtp` refuses exactly where the verdict is not
/// PTP-grade; the findings include it.
#[test]
fn the_report_carries_the_time_sync_section() -> R {
    let report = calibrate(&Config::default())?.report;
    let sync = report.time_sync.as_ref().ok_or("no time-sync section")?;
    assert!(
        report
            .to_string()
            .contains(&format!("time_sync={}", sync.verdict.name()))
    );
    assert!(report.findings().iter().any(|f| f.check == Check::TimeSync));
    assert!(
        report
            .recheck_findings()
            .iter()
            .any(|f| f.check == Check::TimeSync)
    );
    let off = calibrate(&Config {
        time_sync: TimeSyncPolicy::Off,
        ..Config::default()
    })?
    .report;
    assert!(off.time_sync.is_none());
    assert!(off.to_string().contains("time_sync=off"));
    assert!(off.findings().iter().all(|f| f.check != Check::TimeSync));
    let required = calibrate(&Config {
        time_sync: TimeSyncPolicy::RequirePtp,
        ..Config::default()
    });
    match (time_sync().verdict, required) {
        (SyncVerdict::PtpGrade, Ok(c)) => {
            assert_eq!(
                c.report.time_sync.map(|t| t.verdict),
                Some(SyncVerdict::PtpGrade)
            );
        }
        (
            verdict,
            Err(Error::NotPtpGrade {
                verdict: refused,
                why,
            }),
        ) => {
            assert_eq!(verdict, refused);
            assert_ne!(why, String::new());
        }
        (verdict, other) => return Err(format!("{verdict:?}: {other:?}").into()),
    }
    Ok(())
}

/// phc2sys beside another client that disciplines `CLOCK_REALTIME`
/// (systemd-timesyncd, ntpd, or a chronyd that does not answer on its
/// command port) is not shown to be PTP-grade: phc2sys may discipline another
/// clock, or feed that client through a shared-memory reference clock, and
/// two clients that both adjust the clock fight over it. Unknown at info
/// (so `RequirePtp` refuses), never PTP-grade; with `maxerror` at the
/// kernel's limit nobody keeps the clock: unsynchronised. A chronyd that
/// answered unsynchronised disciplines nothing and is no rival.
#[test]
fn phc2sys_beside_another_client_is_not_shown_ptp_grade() {
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
            assert_eq!(t.finding().severity, Severity::Info);
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
}

/// systemd-timesyncd leaves `STA_UNSYNC` set when the RTC keeps local time
/// (`timesyncd-manager.c` `manager_adjust_clock`), but zeroes `maxerror` at
/// every update: a kept `maxerror` under it is a synchronised clock, NTP-
/// disciplined, not an unsynchronised one.
#[test]
fn systemd_timesyncd_with_a_local_time_rtc_is_not_unsynchronised() {
    let t = judge(linux(
        Some(unsynced(3_500)),
        &[TimeDaemon::SystemdTimesyncd],
        refused(),
        vec![],
    ));
    assert_eq!(t.verdict, SyncVerdict::NtpDisciplined, "{}", t.reason);
    assert_eq!(t.bound, None);
    let f = t.finding();
    assert_eq!(f.severity, Severity::Warn);
    assert!(
        f.message
            .contains("not disciplined to a PTP hardware clock"),
        "{f}"
    );
    assert!(!f.message.contains("not synchronised ("), "{f}");
}

/// The systemd-timesyncd sentence is about systemd-timesyncd: a host whose
/// chronyd tracks an NTP server is not told it is SNTP.
#[test]
fn only_a_systemd_timesyncd_host_is_told_about_its_slew() {
    let f = judge(linux(
        Some(synced(2_000)),
        &[TimeDaemon::Chronyd],
        ntp_server(),
        vec![],
    ))
    .finding();
    assert_eq!(f.severity, Severity::Warn);
    assert!(!f.message.contains("SNTP"), "{f}");
    assert!(!f.message.contains("533 ppm"), "{f}");
}

/// The kernel's state is read with the crate's typed error: off Linux an
/// `adjtimex` that cannot be read, never a bare string.
#[test]
fn the_kernels_state_reads_with_a_typed_error() -> R {
    let linux = std::env::consts::OS == "linux";
    match ergon_clock::advanced::kernel_sync() {
        Ok(k) => assert!(linux, "{k:?}"),
        Err(Error::ClockRead { clock, .. }) => {
            assert!(!linux);
            assert_eq!(clock, "adjtimex");
        }
        Err(other) => return Err(other.into()),
    }
    Ok(())
}

/// `KernelSync::read` is the allocation-free read a loop's housekeeping
/// takes every few seconds: the kernel's state on Linux, `ENOSYS` off it,
/// and `kernel_sync` is it with the crate's error.
#[test]
fn the_kernels_state_reads_without_allocating_an_error() -> R {
    match KernelSync::read() {
        Ok(k) => {
            assert_eq!(std::env::consts::OS, "linux", "{k:?}");
            assert!((0..=5).contains(&k.state), "{k:?}");
            let again = ergon_clock::advanced::kernel_sync()?;
            assert_eq!(again.status & STA_UNSYNC, k.status & STA_UNSYNC);
        }
        Err(errno) => {
            if std::env::consts::OS == "linux" {
                // a seccomp filter that denies adjtimex
                assert_eq!(errno, libc::EPERM, "errno {errno}");
            } else {
                assert_eq!(errno, libc::ENOSYS, "errno {errno}");
            }
            assert_eq!(
                ergon_clock::advanced::kernel_sync(),
                Err(Error::ClockRead {
                    clock: "adjtimex",
                    errno
                })
            );
        }
    }
    Ok(())
}

/// The report's findings carry K1, the clock source the gate classifies:
/// at calibration (`facts.clocksource`) and read again from housekeeping.
#[test]
fn the_reports_findings_carry_the_clocksource() -> R {
    let mut report = calibrate(&Config {
        time_sync: TimeSyncPolicy::Off,
        ..Config::default()
    })?
    .report;
    report.facts = HostFacts {
        clocksource: Some("hpet".to_owned()),
        invariant_tsc: Some(true),
        rdtscp: Some(true),
        ..HostFacts::unknown(Arch::X86_64, Os::Linux)
    };
    let findings = report.findings();
    assert!(
        findings
            .iter()
            .any(|f| f.check == Check::Clocksource && f.severity == Severity::Warn),
        "{findings:?}"
    );
    // read again from housekeeping: whatever Linux names now (nothing off
    // Linux), judged as the gate judges it; one of the two architectures'
    // counter sources is not this host's clock source, so at least one
    // finding is expected, whatever the kernel and the binary's architecture
    let now = std::fs::read_to_string(ergon_clock::advanced::CLOCKSOURCE).ok();
    let mut expected_any = false;
    for arch in [Arch::X86_64, Arch::Aarch64] {
        // what calibration saw, which the fresh read must not reuse
        report.facts = HostFacts {
            clocksource: Some("calibration-time-source".to_owned()),
            invariant_tsc: Some(true),
            rdtscp: Some(true),
            ..HostFacts::unknown(arch, Os::Linux)
        };
        let expected = report.facts.clocksource_finding(now.as_deref());
        expected_any |= expected.is_some();
        let fresh: Vec<_> = report
            .recheck_findings()
            .into_iter()
            .filter(|f| f.check == Check::Clocksource)
            .collect();
        assert_eq!(fresh, expected.into_iter().collect::<Vec<_>>(), "{arch:?}");
    }
    assert!(expected_any, "{now:?}");
    Ok(())
}

/// Forcing the TSC is a hardware decision, never the fix for a source the
/// kernel rejected. The message names the boot option so an operator knows
/// what is meant, and says what it costs: `tsc=reliable` turns off the
/// kernel's TSC checks. With invariance unknown it is the same caution, not
/// "boot with it".
#[test]
fn a_rejected_tsc_is_never_advised_as_a_boot_fix() {
    for invariant in [None, Some(true), Some(false)] {
        let x86 = HostFacts {
            invariant_tsc: invariant,
            ..HostFacts::unknown(Arch::X86_64, Os::Linux)
        };
        let f = x86.clocksource_finding(Some("hpet"));
        let f = f.as_ref().map(|f| f.message.as_str());
        match invariant {
            Some(false) => assert!(
                f.is_some_and(|m| m.contains("forcing the TSC would be wrong")),
                "{invariant:?} {f:?}"
            ),
            _ => assert!(
                f.is_some_and(|m| {
                    m.contains("Do not force it as a fix")
                        && m.contains("tsc=reliable")
                        && m.contains("turns off the kernel's TSC checks")
                        && m.contains("never on a VM")
                }),
                "{invariant:?} {f:?}"
            ),
        }
    }
}

/// The `/proc` scan stops at `max_processes` entries, whatever the order
/// the directory lists them in.
#[test]
fn the_proc_scan_is_capped() -> R {
    let s = Scratch::new("proc-cap")?;
    s.file("proc/10/comm", b"chronyd\n")?;
    s.file("proc/11/comm", b"ntpd\n")?;
    s.file("proc/12/comm", b"phc2sys\n")?;
    let probe = TimeSyncProbe {
        max_processes: 1,
        ..probe_at(&s.0)
    };
    assert_eq!(probe.daemons().map(|d| d.len()), Some(1));
    let none = TimeSyncProbe {
        max_processes: 0,
        ..probe_at(&s.0)
    };
    assert_eq!(none.daemons(), Some(Vec::new()));
    Ok(())
}

/// A `/proc` that could not be listed is `unreadable` in the report on
/// Linux, `-` off it.
#[test]
fn unlisted_daemons_are_unreadable_in_the_fields() {
    let mut o = linux(Some(synced(900)), &[], refused(), vec![]);
    o.daemons = None;
    let fields = judge(o).fields();
    assert!(
        fields
            .iter()
            .any(|(k, v)| *k == "time_daemons" && v == "unreadable"),
        "{fields:?}"
    );
    let fields = judge(Observed::not_linux()).fields();
    assert!(
        fields.iter().any(|(k, v)| *k == "time_daemons" && v == "-"),
        "{fields:?}"
    );
}

/// A zombie (or dead) process is not a running daemon: a daemon that has
/// exited but not been reaped (a container whose PID 1 reaps nothing) still
/// has a `comm`, and its `stat` says `Z`. A process without a readable
/// `stat` is still counted, as its `comm` was read.
#[test]
fn a_zombie_daemon_is_not_running() -> R {
    let s = Scratch::new("zombie")?;
    s.file("proc/90/comm", b"phc2sys\n")?;
    s.file("proc/90/stat", b"90 (phc2sys) Z 1 90 90 0 -1 4227332 0 0\n")?;
    s.file("proc/91/comm", b"chronyd\n")?;
    s.file("proc/91/stat", b"91 (chronyd) S 1 91 91 0 -1 4210944 0 0\n")?;
    s.file("proc/92/comm", b"ntpd\n")?;
    s.file("proc/92/stat", b"92 (ntpd) X 1 92 92 0 -1 0 0 0\n")?;
    s.file("proc/93/comm", b"ptp4l\n")?;
    assert_eq!(
        probe_at(&s.0).daemons(),
        Some(vec![TimeDaemon::Chronyd, TimeDaemon::Ptp4l])
    );
    Ok(())
}
