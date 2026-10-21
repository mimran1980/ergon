//! Where the driver runs its checks, over loopback: the quick-ACK route
//! lookup on ergon-io's warning thread under `warn` (the connection goes
//! ahead and its `RouteCheck` arrives later) and on the polling thread under
//! `require`, counted in `DriverStats`; `TCP_NODELAY` and `TCP_CORK` on
//! `Driver::add`; and what `Driver::new` reports about the host and the
//! receive buffer. Its own test binary: lines are reported once per process.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use ergon_io::checks::{self, LoopCpu};
use ergon_io::placement::{CpuError, HelperThread};
use ergon_io::route::{self, QuickAck, QuickAckCheck, RouteCheck};
use ergon_io::{
    Backend, Config, ConnId, Driver, Error, Handler, HttpConn, HttpDial, HttpHandler, HttpOptions,
    Response, RxMeta, warnings,
};
use support::{Lab, R, one_cpu};

/// Held by every test here. The route-lookup counters, the warning thread and
/// the host report are per process, and tests in one binary may run in
/// parallel (`--test-threads`): a test that compares a counter before and
/// after must not see another test's lookups.
static PROCESS_STATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn process_state() -> std::sync::MutexGuard<'static, ()> {
    PROCESS_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

const WAIT: Duration = Duration::from_secs(10);

#[derive(Default)]
struct Opened {
    opened: Vec<ConnId>,
    closed: Vec<String>,
}

impl Handler for Opened {
    fn on_open(&mut self, id: ConnId) {
        self.opened.push(id);
    }
    fn on_closed(&mut self, _: ConnId, error: Error) {
        self.closed.push(error.to_string());
    }
}

impl HttpHandler for Opened {
    fn on_response(&mut self, _: ConnId, _: u64, _: &Response<'_>, _: &RxMeta) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum How {
    Connect,
    Background,
    Add,
}

/// Opens one connection to a loopback server `how`, on the calling thread
/// (the polling thread), and hands the driver and its id to `then` while
/// the server side is still open.
fn with_conn(cfg: Config, how: How, then: impl FnOnce(&mut Driver<HttpConn>, ConnId) -> R) -> R {
    let lab = Lab::new()?;
    let mut d: Driver<HttpConn> = Driver::new(cfg)?;
    std::thread::scope(|s| -> R {
        // Bounded: with no CPU for the helper (a pod with one CPU)
        // `connect_background` refuses and no client arrives.
        let server = s.spawn(|| {
            lab.accept_tls_within(Duration::from_secs(10))
                .map_err(|e| e.to_string())
        });
        let dial = HttpDial::new(
            lab.target("/")?,
            lab.client_cfg.clone(),
            HttpOptions::default(),
        )?;
        let id = match how {
            How::Connect => d.connect(dial)?,
            How::Background => d.connect_background(dial)?,
            How::Add => d.add(dial.connect_blocking()?)?,
        };
        let mut h = Opened::default();
        let end = Instant::now() + WAIT;
        while how != How::Add && h.opened.is_empty() && h.closed.is_empty() {
            d.poll(&mut h)?;
            if Instant::now() > end {
                return Err("connect timed out".into());
            }
        }
        let _peer = server.join().map_err(|_| "server panicked")??;
        if let Some(e) = h.closed.first() {
            return Err(format!("{how:?} closed: {e}").into());
        }
        then(&mut d, id)
    })
}

const fn cfg(policy: QuickAckCheck) -> Config {
    let mut c = Config::new(Backend::EpollSpin);
    c.quickack_check = policy;
    c
}

/// On a pod with one CPU `connect_background` has no CPU for its helper and
/// refuses typed, at once, before any lookup or connect (no server runs).
/// The helper-side half of a check cannot be exercised there; this asserts
/// the refusal instead (driver.rs's `background_helper_leaves_the_polling_cpu`
/// covers the helper on a pod with two CPUs).
fn background_refuses() -> R {
    let lab = Lab::new()?;
    let mut d: Driver<HttpConn> = Driver::new(cfg(QuickAckCheck::Require))?;
    let r = d.connect_background(HttpDial::new(
        lab.target("/")?,
        lab.client_cfg.clone(),
        HttpOptions::default(),
    )?);
    match r {
        Err(Error::Cpu(e)) => assert!(
            matches!(
                *e,
                CpuError::NoSpareCpu {
                    helper: HelperThread::Dial,
                    ..
                }
            ),
            "{e}"
        ),
        other => return Err(format!("expected Error::Cpu, got {other:?}").into()),
    }
    Ok(())
}

fn route_of(d: &Driver<HttpConn>, id: ConnId) -> R<RouteCheck> {
    Ok(d.get(id).ok_or("not open")?.route().clone())
}

#[test]
fn warn_looks_the_route_up_on_the_warning_thread() -> R {
    let _state = process_state();
    for how in [How::Connect, How::Add] {
        let here = route::lookups_here();
        with_conn(cfg(QuickAckCheck::Warn), how, |d, id| {
            assert_eq!(
                route::lookups_here(),
                here,
                "{how:?}: the polling thread looked the route up"
            );
            let s = *d.stats();
            assert_eq!(s.route_lookups, 0, "{how:?}: {s:?}");
            // Reading the addresses and queuing the lookup is counted.
            assert!(s.route_check_ns > 0, "{how:?}: {s:?}");
            assert!(s.route_check_max_ns <= s.route_check_ns, "{how:?}: {s:?}");
            assert!(warnings::flush(WAIT), "the warning thread did not catch up");
            let check = route_of(d, id)?;
            let r = check.route().ok_or_else(|| format!("{how:?}: {check:?}"))?;
            assert!(r.is_local(), "{how:?}: {r}");
            assert_eq!(r.dev.as_deref(), Some("lo"));
            assert_eq!(check.quickack(), QuickAck::Off);
            Ok(())
        })?;
        assert_eq!(route::lookups_here(), here);
    }
    Ok(())
}

#[test]
fn a_pending_check_reads_as_pending() {
    let _state = process_state();
    let p = RouteCheck::Pending;
    assert_eq!(p.quickack(), QuickAck::Unknown);
    assert_eq!(p.route(), None);
    assert_eq!(p.rto_min_ms(), None);
}

#[test]
fn require_decides_where_the_connection_is_set_up() -> R {
    let _state = process_state();
    for (how, on_loop) in [(How::Connect, 1), (How::Add, 1), (How::Background, 0)] {
        if how == How::Background && one_cpu()? {
            // No core for the helper: it refuses typed instead of looking
            // the route up off the loop (the `on_loop` 0 half is the
            // helper's, which needs a second CPU).
            background_refuses()?;
            continue;
        }
        let here = route::lookups_here();
        with_conn(cfg(QuickAckCheck::Require), how, |d, id| {
            assert_eq!(route::lookups_here(), here + on_loop, "{how:?}");
            let s = *d.stats();
            assert_eq!(s.route_lookups, on_loop, "{how:?}: {s:?}");
            if on_loop > 0 {
                assert!(s.route_check_ns > 0, "{how:?}: {s:?}");
                assert!(s.route_check_max_ns > 0, "{how:?}: {s:?}");
            } else {
                assert_eq!(s.route_check_ns, 0, "{how:?}: {s:?}");
            }
            // Decided before the connection was handed over: no wait.
            let check = route_of(d, id)?;
            let r = check.route().ok_or_else(|| format!("{how:?}: {check:?}"))?;
            assert!(r.is_local(), "{how:?}: {r}");
            Ok(())
        })?;
    }
    Ok(())
}

/// A TCP server that accepts and never answers TLS: a connection to it
/// gets as far as TCP and its `ClientHello`, never further.
fn silent_server() -> R<(std::net::TcpListener, ergon_io::Target)> {
    let l = std::net::TcpListener::bind("127.0.0.1:0")?;
    let a = l.local_addr()?;
    Ok((
        l,
        ergon_io::Target::new("localhost", a.port(), "/").with_addr(a),
    ))
}

#[test]
fn require_checks_the_route_before_the_handshake() -> R {
    let _state = process_state();
    let lab = Lab::new()?;
    let (listener, target) = silent_server()?;
    // Driver::connect: on the polling thread, at TCP connect, and the
    // ClientHello goes out only after it.
    let mut d: Driver<HttpConn> = Driver::new(cfg(QuickAckCheck::Require))?;
    let here = route::lookups_here();
    d.connect(HttpDial::new(
        target.clone(),
        lab.client_cfg.clone(),
        HttpOptions::default(),
    )?)?;
    let (server, _) = listener.accept()?;
    server.set_nonblocking(true)?;
    let mut h = Opened::default();
    let end = Instant::now() + WAIT;
    while route::lookups_here() == here {
        d.poll(&mut h)?;
        assert!(
            Instant::now() < end,
            "the route was not checked: {:?}",
            d.stats()
        );
    }
    let mut b = [0u8; 1];
    assert!(
        matches!(server.peek(&mut b), Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock),
        "TLS bytes before the route check"
    );
    assert_eq!(d.stats().route_lookups, 1);
    while server.peek(&mut b).is_err() {
        d.poll(&mut h)?;
        assert!(Instant::now() < end, "no ClientHello after the check");
    }
    assert_eq!(b[0], 0x16, "not a TLS handshake record");
    assert!(h.opened.is_empty() && h.closed.is_empty());
    // Driver::connect_background: on the helper thread, at TCP connect,
    // although the handshake never completes. A pod with one CPU has no
    // core for the helper: it refuses typed instead (asserted; the
    // connect half above ran either way).
    if one_cpu()? {
        return background_refuses();
    }
    let all = route::lookups();
    d.connect_background(HttpDial::new(
        target,
        lab.client_cfg,
        HttpOptions::default(),
    )?)?;
    let (_server2, _) = listener.accept()?;
    while route::lookups() == all {
        d.poll(&mut h)?;
        assert!(Instant::now() < end, "the helper did not check the route");
    }
    assert_eq!(
        route::lookups_here(),
        here + 1,
        "the polling thread looked up"
    );
    assert!(h.opened.is_empty(), "opened without a handshake");
    Ok(())
}

#[test]
fn off_looks_nothing_up_anywhere() -> R {
    let _state = process_state();
    for how in [How::Connect, How::Background, How::Add] {
        if how == How::Background && one_cpu()? {
            // No core for the helper: the typed refusal looks nothing up
            // either (asserted), and the other two ways run below.
            background_refuses()?;
            continue;
        }
        let all = route::lookups();
        with_conn(cfg(QuickAckCheck::Off), how, |d, id| {
            assert!(warnings::flush(WAIT));
            assert_eq!(route::lookups(), all, "{how:?}");
            assert_eq!(route_of(d, id)?, RouteCheck::NotChecked, "{how:?}");
            let s = *d.stats();
            assert_eq!((s.route_lookups, s.route_check_ns), (0, 0), "{how:?}");
            Ok(())
        })?;
    }
    Ok(())
}

fn tcp_opt(fd: i32, name: libc::c_int) -> R<i32> {
    let mut v: libc::c_int = 0;
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
    // SAFETY: `v` and `len` are valid for the call.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            name,
            (&raw mut v).cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(v)
}

fn set_tcp_opt(fd: i32, name: libc::c_int, v: libc::c_int) -> R {
    let len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
    // SAFETY: `v` outlives the call and its size is passed.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::setsockopt(fd, libc::IPPROTO_TCP, name, (&raw const v).cast(), len) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[test]
fn add_sets_tcp_nodelay_and_reports_tcp_cork() -> R {
    let _state = process_state();
    warnings::keep_lines();
    let lab = Lab::new()?;
    let mut d: Driver<HttpConn> = Driver::new(cfg(QuickAckCheck::Off))?;
    std::thread::scope(|s| -> R {
        let server = s.spawn(|| -> Result<_, String> {
            let a = lab.accept_tls().map_err(|e| e.to_string())?;
            let b = lab.accept_tls().map_err(|e| e.to_string())?;
            Ok((a, b))
        });
        let dial = || -> R<HttpDial> {
            Ok(HttpDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                HttpOptions::default(),
            )?)
        };
        // Both connections first, so a failed check below never leaves the
        // server waiting for one (the scope would wait for it).
        let corked = dial()?.connect_blocking()?;
        let plain = dial()?.connect_blocking()?;
        let _peers = server.join().map_err(|_| "server panicked")??;
        let fd = corked.raw_fd();
        set_tcp_opt(fd, libc::TCP_NODELAY, 0)?;
        set_tcp_opt(fd, libc::TCP_CORK, 1)?;
        assert_eq!(tcp_opt(fd, libc::TCP_NODELAY)?, 0);
        let id = d.add(corked)?;
        assert_ne!(tcp_opt(fd, libc::TCP_NODELAY)?, 0, "add left Nagle on");
        let plain_id = d.add(plain)?;
        assert!(warnings::flush(WAIT));
        let lines = warnings::take_lines();
        let cork: Vec<&String> = lines.iter().filter(|l| l.contains("TCP_CORK")).collect();
        assert_eq!(cork.len(), 1, "{lines:#?}");
        assert_eq!(*cork[0], checks::cork_line(id));
        assert_ne!(checks::cork_line(id), checks::cork_line(plain_id));
        Ok(())
    })
}

#[test]
fn driver_new_reports_the_host_once_per_process() -> R {
    let _state = process_state();
    warnings::keep_lines();
    // Earlier tests in this binary already reported their host lines (once
    // per process). Forget those, so this driver's report is printed and its
    // lines can be counted here.
    warnings::forget_seen();
    let mut c = cfg(QuickAckCheck::Off);
    // A CPU the process may use: `loop_cpu` outside the cpuset is refused
    // (`CpuError::Outside`), so a made-up number is not a label here.
    let label = u32::try_from(*support::cpuset()?.last().ok_or("no CPU")?)?;
    c.loop_cpu = Some(label);
    let d: Driver<HttpConn> = Driver::new(c.clone())?;
    let report = d.host_report().clone();
    assert_eq!(report.loop_cpu, LoopCpu::Configured(label));
    let mine: Vec<String> = report
        .lines()
        .into_iter()
        .filter(|l| l.contains(&format!("CPU {label}")))
        .collect();
    assert!(!mine.is_empty(), "{report:#?}");
    assert!(warnings::flush(WAIT));
    let first = warnings::take_lines();
    for l in &mine {
        assert_eq!(
            first.iter().filter(|x| *x == l).count(),
            1,
            "{l}\n{first:#?}"
        );
    }
    drop(Driver::<HttpConn>::new(c)?);
    assert!(warnings::flush(WAIT));
    let second = warnings::take_lines();
    for l in &mine {
        assert!(!second.contains(l), "reported twice: {l}");
    }
    Ok(())
}

#[test]
fn the_receive_buffer_the_kernel_grants_is_checked() -> R {
    let _state = process_state();
    warnings::keep_lines();
    // A size no other test asks for.
    let asked: u32 = (64 << 20) + 4096;
    let probe = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
    probe.set_recv_buffer_size(usize::try_from(asked)?)?;
    let granted = u32::try_from(probe.recv_buffer_size()?)?;
    let _ = probe.as_raw_fd();
    let expected = checks::rcvbuf_line(asked, granted);
    let mut c = cfg(QuickAckCheck::Off);
    c.rcvbuf = Some(asked);
    drop(Driver::<HttpConn>::new(c)?);
    assert!(warnings::flush(WAIT));
    let lines = warnings::take_lines();
    let mine: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains(&format!("SO_RCVBUF asked {asked} bytes")))
        .collect();
    match expected {
        Some(e) => assert_eq!(mine, vec![&e], "{lines:#?}"),
        None => assert!(mine.is_empty(), "{lines:#?}"),
    }
    Ok(())
}
