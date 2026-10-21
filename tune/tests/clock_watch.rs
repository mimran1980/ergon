//! `ClockWatch`: T11 and T12 again from housekeeping, on fixture trees and a
//! fake chronyd. Its own test binary: the counting allocator below is global,
//! and counts only the span a test marks on its own thread.
#![allow(unsafe_code)] // a global allocator is an unsafe trait
#![allow(missing_docs)]

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use common::Fixture;
use ergon_tune::{
    BoundSource, CheckId, ClockSample, ClockWatch, Finding, Host, KernelSync, Status, SyncVerdict,
    TimeSync, TimeSyncProbe,
};

type R = Result<(), Box<dyn std::error::Error>>;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

/// The system allocator, counting the allocations a marked span makes on
/// its thread.
struct Counting;

// SAFETY: every call is forwarded unchanged to the system allocator; the
// count is a thread-local `Cell` with a const initialiser, which neither
// allocates nor re-enters the allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCATIONS.with(|n| n.set(n.get() + 1));
        }
        // SAFETY: the caller's contract, passed on.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, passed on.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// `f`'s result and the allocations it made on this thread.
fn counted<T>(f: impl FnOnce() -> T) -> (T, u64) {
    ALLOCATIONS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    let r = f();
    COUNTING.with(|c| c.set(false));
    (r, ALLOCATIONS.with(Cell::get))
}

const CLOCKSOURCE: &str = "/sys/devices/system/clocksource/clocksource0/current_clocksource";
const S: u64 = 1_000_000_000;
const NOW: u64 = 1_000 * S;

/// A host tree: chronyd running, the TSC as the clock source on an x86 CPU
/// with an invariant TSC.
fn host_tree(name: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let f = Fixture::empty(name)?;
    f.file("/proc/78/comm", "chronyd\n")?
        .file(CLOCKSOURCE, "tsc\n")?
        .file(
            "/proc/cpuinfo",
            "processor\t: 0\nvendor_id\t: GenuineIntel\nflags\t\t: tsc constant_tsc nonstop_tsc\n",
        )?;
    Ok(f)
}

/// A `ClockBound` 2.0 segment, written a second before [`NOW`].
fn clockbound_segment(bound: i64, status: i32) -> Vec<u8> {
    let as_of = NOW - S;
    let mut s = Vec::with_capacity(80);
    s.extend(0x414D_5A4E_u32.to_ne_bytes());
    s.extend(0x4342_0200_u32.to_ne_bytes());
    s.extend(80_u32.to_ne_bytes());
    s.extend(2_u16.to_ne_bytes());
    s.extend(10_u16.to_ne_bytes());
    for ns in [as_of, as_of + 60 * S] {
        s.extend(i64::try_from(ns / S).unwrap_or(0).to_ne_bytes());
        s.extend(i64::try_from(ns % S).unwrap_or(0).to_ne_bytes());
    }
    s.extend(bound.to_ne_bytes());
    s.extend(7_u64.to_ne_bytes());
    s.extend(50_000_u32.to_ne_bytes());
    s.extend(status.to_ne_bytes());
    s.extend([1_u8, 0, 0, 0, 0, 0, 0, 0]);
    s
}

fn write_segment(f: &Fixture, bound: i64, status: i32) -> R {
    let path = f.root().join("var/run/clockbound/shm0");
    std::fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    std::fs::write(path, clockbound_segment(bound, status))?;
    Ok(())
}

/// What the fake chronyd answers with.
const PHC: u8 = 0;
const SERVER: u8 = 1;
const SILENT: u8 = 2;

/// A fake chronyd on the loopback, answering every request as `mode` says
/// until dropped.
struct FakeChronyd {
    addr: SocketAddr,
    mode: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FakeChronyd {
    fn start(mode: u8) -> std::io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
        socket.set_read_timeout(Some(Duration::from_millis(20)))?;
        let addr = socket.local_addr()?;
        let mode = Arc::new(AtomicU8::new(mode));
        let stop = Arc::new(AtomicBool::new(false));
        let (m, s) = (Arc::clone(&mode), Arc::clone(&stop));
        let thread = std::thread::spawn(move || {
            let mut buf = [0_u8; 1024];
            while !s.load(Ordering::Relaxed) {
                let Ok((n, from)) = socket.recv_from(&mut buf) else {
                    continue;
                };
                let reply = match m.load(Ordering::Relaxed) {
                    PHC => tracking_reply(&buf[..n], *b"PHC0", 0, [0; 4]),
                    SERVER => {
                        tracking_reply(&buf[..n], [169, 254, 169, 123], 1, [169, 254, 169, 123])
                    }
                    _ => continue,
                };
                let _ = socket.send_to(&reply, from);
            }
        });
        Ok(Self {
            addr,
            mode,
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for FakeChronyd {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// A tracking reply to `request` as chronyd 4.6.1 builds one: reference ID
/// `ref_id`, address family `family` (0: a reference clock; 1: IPv4 `addr`),
/// stratum 1, a 1 µs correction, 4 µs dispersion, 10 µs delay.
fn tracking_reply(request: &[u8], ref_id: [u8; 4], family: u16, addr: [u8; 4]) -> Vec<u8> {
    // correction, last offset, RMS offset, frequency, residual frequency,
    // skew, root delay, root dispersion, update interval
    const FLOATS: [f64; 9] = [1e-6, 0.0, 0.0, -11.5, 0.0, 0.0, 0.000_010, 0.000_004, 1.0];
    let mut p = vec![0_u8; 104];
    p[0] = 6;
    p[1] = 2;
    p[4..6].copy_from_slice(&33_u16.to_be_bytes());
    p[6..8].copy_from_slice(&5_u16.to_be_bytes());
    p[16..20].copy_from_slice(request.get(8..12).unwrap_or(&[0; 4]));
    p[28..32].copy_from_slice(&ref_id);
    p[32..36].copy_from_slice(&addr);
    p[48..50].copy_from_slice(&family.to_be_bytes());
    p[52..54].copy_from_slice(&1_u16.to_be_bytes());
    for (k, v) in FLOATS.iter().enumerate() {
        p[68 + 4 * k..72 + 4 * k].copy_from_slice(&float_bits(*v).to_be_bytes());
    }
    p
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

fn probe(chrony: Option<SocketAddr>) -> TimeSyncProbe {
    TimeSyncProbe {
        chrony,
        chrony_timeout: Duration::from_millis(50),
        clockbound_now_ns: Some(NOW),
        ..TimeSyncProbe::default()
    }
}

fn pause() {
    std::thread::sleep(Duration::from_millis(30));
}

fn ids(findings: &[Finding]) -> Vec<CheckId> {
    findings.iter().map(|f| f.id).collect()
}

/// `check` allocates nothing while nothing changes: chronyd answering the
/// same, an absent one, a silent one; the `ClockBound` segment and the clock
/// source read again each time.
#[test]
fn the_watch_allocates_nothing_while_nothing_changes() -> R {
    let f = host_tree("watch-alloc")?;
    write_segment(&f, 20_000, 1)?;
    let answering = FakeChronyd::start(PHC)?;
    let silent = FakeChronyd::start(SILENT)?;
    let closed = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?.local_addr()?;
    for (what, addr) in [
        ("answering", Some(answering.addr)),
        ("silent", Some(silent.addr)),
        ("absent", Some(closed)),
        ("not asked", None),
    ] {
        let (mut watch, start) = ClockWatch::open(&f.host(), &probe(addr));
        assert_eq!(
            ids(&start),
            [CheckId::T11, CheckId::T12],
            "{what}: {start:?}"
        );
        for pass in 0..4 {
            pause();
            let (found, allocations) = counted(|| watch.check());
            assert_eq!(found, Vec::<Finding>::new(), "{what} pass {pass}");
            assert_eq!(allocations, 0, "{what} pass {pass}");
        }
    }
    Ok(())
}

/// The watch never blocks: with chronyd silent or absent each check returns
/// at once (one non-blocking receive and one send), while the start-up probe
/// waits at most its timeout.
#[test]
fn an_absent_or_silent_chronyd_never_blocks_the_watch() -> R {
    let f = host_tree("watch-block")?;
    let silent = FakeChronyd::start(SILENT)?;
    let closed = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?.local_addr()?;
    for addr in [silent.addr, closed] {
        let started = Instant::now();
        let (mut watch, _) = ClockWatch::open(&f.host(), &probe(Some(addr)));
        assert!(started.elapsed() < Duration::from_millis(250), "{addr}");
        for _ in 0..3 {
            let started = Instant::now();
            let found = watch.check();
            let took = started.elapsed();
            assert!(took < Duration::from_millis(20), "{addr}: {took:?}");
            assert_eq!(found, Vec::<Finding>::new(), "{addr}");
        }
    }
    Ok(())
}

/// chronyd's answer to one check's request is read at the next: a change of
/// reference shows one pass after the request that sees it, as the T11
/// findings again, once.
#[test]
fn a_change_of_reference_is_reported_one_pass_later() -> R {
    let f = host_tree("watch-reference")?;
    let chronyd = FakeChronyd::start(PHC)?;
    let (mut watch, start) = ClockWatch::open(&f.host(), &probe(Some(chronyd.addr)));
    assert_eq!(watch.verdict(), SyncVerdict::PtpGrade, "{start:?}");
    pause();
    assert_eq!(watch.check(), []);
    pause(); // the request that check sent is answered as PHC0
    chronyd.mode.store(SERVER, Ordering::Relaxed);
    assert_eq!(watch.check(), [], "the answer to the earlier request");
    pause();
    let found = watch.check();
    assert_eq!(ids(&found), [CheckId::T11], "{found:?}");
    assert!(found[0].is_warning(), "{}", found[0]);
    assert!(found[0].message.contains("169.254.169.123"), "{}", found[0]);
    assert_eq!(watch.verdict(), SyncVerdict::NtpDisciplined);
    pause();
    assert_eq!(watch.check(), []);
    Ok(())
}

/// `ClockBound`'s segment is opened again at every check: a bound that
/// stops being trusted (disrupted) gives way to chrony's, and the T11
/// findings come again, once.
#[test]
fn a_clockbound_bound_going_untrusted_is_reported() -> R {
    let f = host_tree("watch-clockbound")?;
    write_segment(&f, 20_000, 1)?;
    let chronyd = FakeChronyd::start(PHC)?;
    let (mut watch, _) = ClockWatch::open(&f.host(), &probe(Some(chronyd.addr)));
    assert_eq!(
        watch.bound().map(|b| b.source),
        Some(BoundSource::ClockBound)
    );
    pause();
    assert_eq!(watch.check(), []);
    write_segment(&f, 20_000, 3)?;
    let found = watch.check();
    assert_eq!(ids(&found), [CheckId::T11], "{found:?}");
    assert_eq!(watch.bound().map(|b| b.source), Some(BoundSource::Chrony));
    pause();
    assert_eq!(watch.check(), []);
    Ok(())
}

/// The clock source is read again at every check: a switch from the TSC to
/// hpet is one T12 warning saying so, once; back to the TSC is one ok.
#[test]
fn a_clock_source_switch_warns_once() -> R {
    let f = host_tree("watch-clocksource")?;
    let (mut watch, start) = ClockWatch::open(&f.host(), &probe(None));
    let t12 = start
        .iter()
        .find(|x| x.id == CheckId::T12)
        .ok_or("no T12")?;
    assert_eq!(t12.status, Status::Ok, "{t12}");
    assert_eq!(watch.check(), []);
    f.file(CLOCKSOURCE, "hpet\n")?;
    let found = watch.check();
    assert_eq!(ids(&found), [CheckId::T12], "{found:?}");
    assert!(found[0].is_warning(), "{}", found[0]);
    assert!(
        found[0].message.contains("changed from tsc to hpet"),
        "{}",
        found[0]
    );
    assert_eq!(watch.check(), []);
    f.file(CLOCKSOURCE, "tsc\n")?;
    let found = watch.check();
    assert_eq!(ids(&found), [CheckId::T12], "{found:?}");
    assert_eq!(found[0].status, Status::Ok, "{}", found[0]);
    Ok(())
}

/// The kernel's state, injected: the watch judges every sample by the same
/// rules as the start-up check (`TimeSync::from_observed`), so a clock that
/// loses its daemon turns unsynchronised and back, a kept `maxerror` with
/// `STA_UNSYNC` stays unknown, and a pending leap second is flagged.
#[test]
fn the_kernels_state_is_judged_by_the_start_up_rules() -> R {
    const fn kernel(state: i32, status: i32, maxerror_us: i64) -> KernelSync {
        KernelSync {
            state,
            status,
            maxerror_us,
            esterror_us: 0,
            offset_ns: 0,
            freq_ppm: 0.0,
        }
    }
    let synced = kernel(0, 0x0001, 800);
    let kept = kernel(5, 0x0041, 5_000);
    let lost = kernel(5, 0x0041, 16_000_000);
    let leap = kernel(1, 0x0011, 800);
    let f = host_tree("watch-kernel")?;
    let (mut watch, _) = ClockWatch::open(&f.host(), &probe(None));
    // a tree: the kernel is not asked, so with chronyd silent it is unknown
    assert_eq!(watch.verdict(), SyncVerdict::Unknown);
    for (k, want, warns) in [
        (synced, SyncVerdict::NtpDisciplined, true),
        (synced, SyncVerdict::NtpDisciplined, false),
        (kept, SyncVerdict::Unknown, false),
        (lost, SyncVerdict::Unsynchronised, true),
        (synced, SyncVerdict::NtpDisciplined, true),
    ] {
        let sample = ClockSample {
            kernel: Some(k),
            ..watch.last()
        };
        let found = watch.update(sample);
        assert_eq!(watch.verdict(), want, "{k:?}");
        let same = TimeSync::from_observed(watch.observed().clone());
        assert_eq!(same.verdict, want, "one set of rules");
        assert_eq!(
            found.iter().any(Finding::is_warning),
            warns,
            "{k:?}: {found:?}"
        );
        assert!(!watch.leap_pending());
    }
    let found = watch.update(ClockSample {
        kernel: Some(leap),
        ..watch.last()
    });
    assert!(watch.leap_pending());
    assert!(
        found
            .iter()
            .any(|x| x.message.starts_with("a leap second is pending (adjtimex)")),
        "{found:?}"
    );
    Ok(())
}

/// The kernel's `maxerror` grows by 500 µs a second between a daemon's
/// updates, so a bound kept by ntpd or a silent chronyd crosses the limit
/// back and forth. Under a verdict that warns already (NTP-disciplined) that
/// changes no finding, so the watch reports nothing while the bound follows;
/// under one that does not (unknown), the wide-bound warning comes and goes.
#[test]
fn a_bound_crossing_the_limit_reports_only_where_it_is_its_own_warning() -> R {
    const fn kernel(maxerror_us: i64) -> KernelSync {
        KernelSync {
            state: 0,
            status: 0x0001,
            maxerror_us,
            esterror_us: 0,
            offset_ns: 0,
            freq_ppm: 0.0,
        }
    }
    let f = host_tree("watch-maxerror")?;
    let (mut watch, _) = ClockWatch::open(&f.host(), &probe(None));
    let found = watch.update(ClockSample {
        kernel: Some(kernel(800)),
        ..watch.last()
    });
    assert_eq!(watch.verdict(), SyncVerdict::NtpDisciplined);
    assert!(found.iter().any(Finding::is_warning), "{found:?}");
    for maxerror_us in [20_800_u32, 900, 32_000, 600] {
        let found = watch.update(ClockSample {
            kernel: Some(kernel(i64::from(maxerror_us))),
            ..watch.last()
        });
        assert_eq!(found, [], "maxerror {maxerror_us} µs");
        assert_eq!(
            watch.bound().map(|b| (b.source, b.ns)),
            Some((BoundSource::KernelMaxerror, u64::from(maxerror_us) * 1_000))
        );
    }
    // a PTP hardware clock present: chronyd silent is unknown, not a warning
    let f = host_tree("watch-maxerror-phc")?;
    f.file("/sys/class/ptp/ptp0/clock_name", "ena-ptp-00\n")?;
    let (mut watch, _) = ClockWatch::open(&f.host(), &probe(None));
    let mut wide = Vec::new();
    for maxerror_us in [800, 20_800, 900] {
        let found = watch.update(ClockSample {
            kernel: Some(kernel(maxerror_us)),
            ..watch.last()
        });
        assert_eq!(watch.verdict(), SyncVerdict::Unknown);
        assert!(found.first().is_some_and(Finding::is_unknown), "{found:?}");
        wide.push(found.iter().any(Finding::is_warning));
    }
    assert_eq!(wide, [false, true, false]);
    Ok(())
}

/// On the running host (`adjtimex` and the coarse clock too, on Linux;
/// nothing is read elsewhere) a check allocates only when it reports, and
/// each returns at once.
#[test]
fn on_the_running_host_a_check_allocates_only_when_it_reports() {
    let (mut watch, start) = ClockWatch::open(&Host::system(), &TimeSyncProbe::default());
    for f in &start {
        println!("{f}");
    }
    for pass in 0..4 {
        pause();
        let started = Instant::now();
        let (found, allocations) = counted(|| watch.check());
        let took = started.elapsed();
        assert!(took < Duration::from_millis(20), "pass {pass}: {took:?}");
        for f in &found {
            println!("pass {pass}: {f}");
        }
        assert!(
            allocations == 0 || !found.is_empty(),
            "pass {pass}: {allocations} allocations and nothing reported"
        );
    }
    if !cfg!(target_os = "linux") {
        assert_eq!(watch.verdict(), SyncVerdict::Unknown);
    }
}
