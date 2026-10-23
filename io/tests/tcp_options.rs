//! The socket options every connection gets, whichever way the driver takes
//! it, and libcurl's too: `TCP_NODELAY`, keepalive (`SO_KEEPALIVE`,
//! `TCP_KEEPIDLE`, `TCP_KEEPINTVL`, `TCP_KEEPCNT`, `TCP_USER_TIMEOUT`),
//! `SO_RCVBUF` and `SO_TIMESTAMPING`. Every one of them is read back off the
//! live socket, so a config key that silently does nothing fails here.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

use std::time::{Duration, Instant};

use ergon_io::sys::{TcpKeepalive, tcp_keepalive};
use ergon_io::{
    Backend, Config, ConnId, Driver, Error, Handler, RxMeta, WsConn, WsDial, WsHandler,
};
use support::{Lab, R};

#[derive(Default)]
struct Opened(Vec<ConnId>, Vec<String>);

impl Handler for Opened {
    fn on_open(&mut self, id: ConnId) {
        self.0.push(id);
    }
    fn on_closed(&mut self, _: ConnId, error: Error) {
        self.1.push(error.to_string());
    }
}

impl WsHandler for Opened {
    fn on_message(&mut self, _: ConnId, _: u8, _: &[u8], _: &RxMeta, _: u16) {}
}

fn tcp_nodelay(fd: std::os::fd::RawFd) -> R<bool> {
    let mut v: libc::c_int = 0;
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
    // SAFETY: `v` and `len` are valid for the call.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_NODELAY,
            (&raw mut v).cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(v != 0)
}

/// `getsockopt(SO_RCVBUF)`: what the kernel actually granted. It doubles the
/// value it was asked for and caps it at `net.core.rmem_max` (socket(7)), so
/// this is compared against `granted_rcvbuf`, never against the config key.
fn so_rcvbuf(fd: std::os::fd::RawFd) -> R<u32> {
    let mut v: libc::c_int = 0;
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
    // SAFETY: `v` and `len` are valid for the call.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVBUF,
            (&raw mut v).cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(libc::c_int::cast_unsigned(v))
}

/// `getsockopt(SO_TIMESTAMPING)`: the `SOF_TIMESTAMPING_*` bits the socket is
/// set up with. This is the wiring, not the stamps: it is what decides whether
/// `recvmsg` later carries an `SCM_TIMESTAMPING` cmsg at all. The kernel
/// stores the mask as an unsigned word.
fn so_timestamping(fd: std::os::fd::RawFd) -> R<libc::c_uint> {
    let mut v: libc::c_uint = 0;
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_uint>())?;
    // SAFETY: `v` and `len` are valid for the call.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_TIMESTAMPING,
            (&raw mut v).cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(v)
}

/// The flags `enable_rx_timestamps` asks for. Compared bit for bit with what
/// the socket reads back.
const fn rx_timestamp_flags() -> libc::c_uint {
    libc::SOF_TIMESTAMPING_RX_SOFTWARE
        | libc::SOF_TIMESTAMPING_SOFTWARE
        | libc::SOF_TIMESTAMPING_RX_HARDWARE
        | libc::SOF_TIMESTAMPING_RAW_HARDWARE
}

/// Opens one connection on `d` the way `how` names and returns its id.
fn open(lab: &Lab, d: &mut Driver<WsConn>, how: &str) -> R<ConnId> {
    let dial = || WsDial::new(lab.target("/").ok()?, lab.client_cfg.clone(), 1 << 20).ok();
    let id = match how {
        "add" => d.add(WsConn::connect(
            &lab.target("/")?,
            lab.client_cfg.clone(),
            1 << 20,
        )?)?,
        "connect" => d.connect(dial().ok_or("dial")?)?,
        _ => d.connect_background(dial().ok_or("dial")?)?,
    };
    let mut h = Opened::default();
    let end = Instant::now() + Duration::from_secs(10);
    while how != "add" && !h.0.contains(&id) {
        d.poll(&mut h)?;
        if Instant::now() > end || !h.1.is_empty() {
            return Err(format!("{how}: never opened: {:?}", h.1).into());
        }
    }
    Ok(id)
}

#[test]
fn every_way_in_gets_nodelay_and_keepalive() -> R {
    for how in ["add", "connect", "connect_background"] {
        let lab = Lab::new()?;
        let mut cfg = Config::new(Backend::EpollSpin);
        // A pod with one CPU has no core for the connect helper: opt in to
        // connecting on the loop there, so every way in is covered on any
        // pod (the refusal without this is driver.rs's one-CPU test).
        cfg.inline_connect_fallback = true;
        let mut d: Driver<WsConn> = Driver::new(cfg)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = open(&lab, &mut d, how)?;
            let fd = d.get(id).ok_or("gone")?.raw_fd();
            assert!(tcp_nodelay(fd)?, "{how}: Nagle on");
            assert_eq!(
                tcp_keepalive(fd)?,
                TcpKeepalive {
                    enabled: true,
                    idle_s: 10,
                    interval_s: 5,
                    count: 3,
                    user_timeout_ms: 25_000,
                },
                "{how}"
            );
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn keepalive_follows_the_config_and_can_be_off() -> R {
    for (cfg, want_on, want_user) in [
        (
            "recv-busy,keepalive_idle_s=30,keepalive_interval_s=2,keepalive_count=4,tcp_user_timeout_ms=9000",
            true,
            9_000,
        ),
        (
            "recv-busy,keepalive_idle_s=0,tcp_user_timeout_ms=0",
            false,
            0,
        ),
    ] {
        let lab = Lab::new()?;
        let mut d: Driver<WsConn> = Driver::new(Config::parse(cfg)?)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = open(&lab, &mut d, "add")?;
            let k = tcp_keepalive(d.get(id).ok_or("gone")?.raw_fd())?;
            assert_eq!(k.enabled, want_on, "{cfg}: {k:?}");
            assert_eq!(k.user_timeout_ms, want_user, "{cfg}: {k:?}");
            if want_on {
                assert_eq!((k.idle_s, k.interval_s, k.count), (30, 2, 4), "{cfg}");
            }
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

// `rcvbuf` is a config key that can silently do nothing and still look right
// in a parse test. Read it off the connection: the kernel doubles the value
// and caps it at `net.core.rmem_max`, so the socket must hold exactly what
// `granted_rcvbuf` says a fresh socket would hold for the same request.
#[test]
fn rcvbuf_reaches_the_socket_or_the_kernel_default_stays() -> R {
    for (cfg, want) in [
        // Four megabytes: the default. Above `rmem_max` on most hosts, so the
        // kernel caps it; the point is that the cap is the kernel's, and that
        // a `None` (rcvbuf=0) leaves the kernel's own value alone.
        ("recv-busy,rcvbuf=4194304", Some(4_194_304u32)),
        ("recv-busy,rcvbuf=65536", Some(65_536)),
        ("recv-busy,rcvbuf=0", None),
    ] {
        let lab = Lab::new()?;
        let mut d: Driver<WsConn> = Driver::new(Config::parse(cfg)?)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = open(&lab, &mut d, "add")?;
            let fd = d.get(id).ok_or("gone")?.raw_fd();
            let got = so_rcvbuf(fd)?;
            match want {
                Some(asked) => {
                    // The kernel doubles and caps; `granted_rcvbuf` is that
                    // same arithmetic on a fresh socket.
                    let expect = ergon_io::sys::granted_rcvbuf(asked)?;
                    assert_eq!(got, expect, "{cfg}: SO_RCVBUF = {got}, want {expect}");
                }
                None => {
                    // Nothing was set: the socket keeps whatever the kernel
                    // gives a fresh one. It must not be 0 (a 0 would mean the
                    // set ran with a 0), and it must not be the doubled value
                    // of an explicit request either.
                    assert_ne!(got, 0, "{cfg}: SO_RCVBUF = 0");
                }
            }
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

// `rx_timestamps` decides whether `recvmsg` can ever carry a stamp. It is a
// socket option and it reads back: on, the `SOF_TIMESTAMPING_*` bits
// `enable_rx_timestamps` sets must be there; off, the option must be clear.
// A socket with the option clear cannot produce a stamp however the receive
// is done, so this is the whole of the flag's effect from userspace.
#[test]
fn rx_timestamps_is_set_only_when_asked_for() -> R {
    for (cfg, want) in [
        ("recv-busy,rx_timestamps=true", true),
        ("recv-busy", true),
        ("recv-busy,rx_timestamps=false", false),
    ] {
        let lab = Lab::new()?;
        let mut d: Driver<WsConn> = Driver::new(Config::parse(cfg)?)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = open(&lab, &mut d, "add")?;
            let fd = d.get(id).ok_or("gone")?.raw_fd();
            let got = so_timestamping(fd)?;
            let flags = rx_timestamp_flags();
            if want {
                // Exactly what was asked for. The kernel stores the mask it
                // was given; anything else is a setsockopt that did not stick.
                assert_eq!(
                    got, flags,
                    "{cfg}: SO_TIMESTAMPING = {got:#x}, want {flags:#x}"
                );
            } else {
                assert_eq!(got, 0, "{cfg}: SO_TIMESTAMPING = {got:#x}, want 0");
            }
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

/// This process's TCP sockets connected to `peer` (libcurl's, here).
#[cfg(feature = "curl")]
fn sockets_to(peer: std::net::SocketAddr) -> R<Vec<std::os::fd::RawFd>> {
    Ok(connected_sockets()?
        .into_iter()
        .filter(|(_, p)| *p == peer)
        .map(|(fd, _)| fd)
        .collect())
}

/// Every connected `AF_INET` socket of this process as `fd,peer`. The keepalive
/// test reports all of them on a mismatch: a failure must name the socket that
/// disagreed and what else was in the process, not only the value.
#[cfg(feature = "curl")]
fn connected_sockets() -> R<Vec<(std::os::fd::RawFd, std::net::SocketAddr)>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir("/proc/self/fd")? {
        let Ok(fd) = entry?.file_name().to_string_lossy().parse() else {
            continue;
        };
        // SAFETY: zeroed storage is a valid `sockaddr_storage`.
        #[allow(unsafe_code)]
        let mut ss: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let mut len = libc::socklen_t::try_from(std::mem::size_of_val(&ss))?;
        // SAFETY: `ss` and `len` are valid for the call; a descriptor that is
        // not a connected socket (or was closed meanwhile) just fails.
        #[allow(unsafe_code)]
        let rc = unsafe { libc::getpeername(fd, (&raw mut ss).cast(), &raw mut len) };
        if rc != 0 || i32::from(ss.ss_family) != libc::AF_INET {
            continue;
        }
        // SAFETY: an `AF_INET` address fills a `sockaddr_in`.
        #[allow(unsafe_code)]
        let sin: libc::sockaddr_in = unsafe { std::ptr::read((&raw const ss).cast()) };
        let addr = std::net::SocketAddr::from((
            u32::from_be(sin.sin_addr.s_addr).to_be_bytes(),
            u16::from_be(sin.sin_port),
        ));
        found.push((fd, addr));
    }
    Ok(found)
}

#[cfg(feature = "curl")]
#[test]
fn libcurl_connections_get_keepalive_too() -> R {
    use std::sync::atomic::AtomicBool;

    use ergon_io::{ClientHandler, HttpClient, HttpClientConfig, HttpImpl, Response};
    use support::StopOnDrop;

    #[derive(Default)]
    struct Answered(bool, Vec<String>);
    impl ClientHandler for Answered {
        fn on_response(&mut self, _: u64, _: &Response<'_>) {
            self.0 = true;
        }
        fn on_error(&mut self, _: u64, _: ergon_io::Sent, error: Error) {
            self.1.push(error.to_string());
        }
    }

    for (cfg, want) in [
        (
            "recv-busy",
            TcpKeepalive {
                enabled: true,
                idle_s: 10,
                interval_s: 5,
                count: 3,
                user_timeout_ms: 25_000,
            },
        ),
        (
            "recv-busy,keepalive_idle_s=0,tcp_user_timeout_ms=0",
            TcpKeepalive {
                enabled: false,
                idle_s: 0,
                interval_s: 0,
                count: 0,
                user_timeout_ms: 0,
            },
        ),
    ] {
        let lab = Lab::new()?;
        let stop = AtomicBool::new(false);
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
            let result = (|| -> R {
                let _stop = StopOnDrop(&stop);
                let mut c = HttpClientConfig::new(HttpImpl::Curl);
                c.driver = Config::parse(cfg)?;
                c.curl_ca_file = Some(lab.ca_file()?.display().to_string());
                let mut client = HttpClient::new(&c, lab.target("")?, lab.client_cfg.clone())?;
                client.request("GET", "/k", &[], &[])?;
                let mut h = Answered::default();
                let end = Instant::now() + Duration::from_secs(10);
                while !h.0 && h.1.is_empty() {
                    client.poll(&mut h)?;
                    if Instant::now() > end {
                        return Err("no answer".into());
                    }
                }
                if !h.1.is_empty() {
                    return Err(format!("{cfg}: {:?}", h.1).into());
                }
                // libcurl keeps the connection open in its cache.
                let peer = lab.addr()?;
                let fds = sockets_to(peer)?;
                if fds.is_empty() {
                    let all = connected_sockets()?;
                    return Err(format!(
                        "{cfg}: no libcurl socket to {peer}; this process has {all:?}"
                    )
                    .into());
                }
                for fd in fds {
                    let mut got = tcp_keepalive(fd)?;
                    if !want.enabled {
                        // Only "off" matters; the probe settings are the kernel's.
                        (got.idle_s, got.interval_s, got.count) = (0, 0, 0);
                    }
                    if got != want {
                        // Name every connected socket and its keepalive: the
                        // wrong one may not be the socket libcurl opened.
                        let mut seen = Vec::new();
                        for (fd, addr) in connected_sockets()? {
                            let k = tcp_keepalive(fd).map_err(|e| format!("{fd}: {e}"))?;
                            seen.push(format!("{fd}->{addr} {k:?}"));
                        }
                        return Err(format!(
                            "{cfg}: fd {fd} is {got:?}, want {want:?}; process sockets: {}",
                            seen.join("; ")
                        )
                        .into());
                    }
                }
                Ok(())
            })();
            srv.join().map_err(|_| "server panicked")??;
            result
        })?;
    }
    // Settings the kernel would refuse are refused up front, as on the driver.
    let lab = Lab::new()?;
    let mut c = HttpClientConfig::new(HttpImpl::Curl);
    c.driver.keepalive_count = 0;
    let r = HttpClient::new(&c, lab.target("")?, lab.client_cfg.clone());
    assert!(matches!(r, Err(Error::Config(_))), "{:?}", r.err());
    Ok(())
}

/// `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US` (`include/uapi/linux/tcp.h`,
/// Linux 6.15), read straight off the socket.
fn tcp_timer_us(fd: std::os::fd::RawFd, opt: libc::c_int) -> R<libc::c_int> {
    let mut v: libc::c_int = 0;
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
    // SAFETY: `v` and `len` are valid for the call.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            opt,
            (&raw mut v).cast(),
            &raw mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(v)
}

/// Whether the running kernel is at least `major.minor` (from
/// `/proc/sys/kernel/osrelease`): the oracle for which outcome a test
/// expects, never a reason to skip one.
fn kernel_at_least(major: u32, minor: u32) -> R<bool> {
    let rel = std::fs::read_to_string("/proc/sys/kernel/osrelease")?;
    let mut parts = rel.trim().split(['.', '-']);
    let maj: u32 = parts.next().ok_or("no major")?.parse()?;
    let min: u32 = parts.next().ok_or("no minor")?.parse()?;
    Ok((maj, min) >= (major, minor))
}

/// The two timers reach every connection on a 6.15+ kernel, read back
/// exactly; an older kernel refuses at `Driver::new`, typed, naming the
/// option and the kernel it needs. Never set silently, never ignored.
#[test]
fn tcp_timers_reach_the_socket_or_are_refused_typed() -> R {
    let cfg = "epoll-spin,tcp_rto_min_us=40000,tcp_delack_max_us=20000";
    let made: Result<Driver<WsConn>, Error> = Driver::new(Config::parse(cfg)?);
    if !kernel_at_least(6, 15)? {
        let Err(Error::Unsupported(why)) = made else {
            return Err(format!("{cfg}: a pre-6.15 kernel gave {:?}", made.err()).into());
        };
        assert!(
            why.contains("TCP_RTO_MIN_US") && why.contains("6.15"),
            "{why}"
        );
        return Ok(());
    }
    let mut d = made?;
    let lab = Lab::new()?;
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let mut p = lab.accept_within(Duration::from_secs(15))?;
            assert!(p.client_closed()?);
            Ok(())
        });
        let id = open(&lab, &mut d, "add")?;
        let fd = d.get(id).ok_or("gone")?.raw_fd();
        // 40 ms and 20 ms are whole ticks, at least two, at CONFIG_HZ 100,
        // 250, 300 and 1000, so every kernel keeps them exactly (10 ms would
        // be one tick at 100 and 2.5 at 250: refused).
        assert_eq!(tcp_timer_us(fd, 45)?, 40_000, "TCP_RTO_MIN_US");
        assert_eq!(tcp_timer_us(fd, 46)?, 20_000, "TCP_DELACK_MAX_US");
        drop(d);
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })?;
    // 2.5 ms is never whole ticks of at least two: CONFIG_HZ=1000 rounds it
    // up to 3 ms, and coarser ticks make it one tick (below the kernel's
    // two). Either way a typed config error, not a silently different floor.
    let r: Result<Driver<WsConn>, Error> =
        Driver::new(Config::parse("epoll-spin,tcp_rto_min_us=2500")?);
    let Err(Error::Config(why)) = r else {
        return Err(format!("2500 us: {:?}", r.err()).into());
    };
    assert!(why.contains("tcp_rto_min_us"), "{why}");
    Ok(())
}

/// Passive `TCP_INFO`: off by default (no sample, no `getsockopt`); on, a
/// sample per connection at the configured interval, with the kernel's RTT,
/// RTO and loss counters. With `tcp_rto_min_us` (6.15+) the sampled RTO
/// shows the floor took effect.
#[test]
fn tcp_info_is_sampled_only_when_asked_for() -> R {
    let floor = kernel_at_least(6, 15)?;
    for (cfg, on) in [
        ("epoll-spin", false),
        ("epoll-spin,tcp_info_interval_ms=1", true),
        ("uring-spin,tcp_info_interval_ms=1", true),
        (
            "epoll-spin,tcp_info_interval_ms=1,tcp_rto_min_us=20000",
            true,
        ),
    ] {
        if cfg.contains("tcp_rto_min_us") && !floor {
            // Covered by `tcp_timers_reach_the_socket_or_are_refused_typed`.
            let r: Result<Driver<WsConn>, Error> = Driver::new(Config::parse(cfg)?);
            assert!(
                matches!(r, Err(Error::Unsupported(_))),
                "{cfg}: {:?}",
                r.err()
            );
            continue;
        }
        let lab = Lab::new()?;
        let mut d: Driver<WsConn> = Driver::new(Config::parse(cfg)?)?;
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                for _ in 0..5 {
                    let (op, body) = p.recv()?;
                    p.send(op, true, &body)?;
                }
                assert!(p.client_closed()?);
                Ok(())
            });
            let id = open(&lab, &mut d, "add")?;
            let mut h = Echoed::default();
            // A few round trips, so the kernel has RTT samples.
            for i in 0..5 {
                d.get_mut(id)
                    .ok_or("gone")?
                    .send_frame(ergon_io::ws::OP_BINARY, b"ping")?;
                let end = Instant::now() + Duration::from_secs(10);
                while h.0 <= i {
                    d.poll(&mut h)?;
                    if Instant::now() > end || !h.1.is_empty() {
                        return Err(format!("{cfg}: no echo: {:?}", h.1).into());
                    }
                }
            }
            // Poll for a while: many intervals when on.
            let end = Instant::now() + Duration::from_millis(200);
            let mut info = None;
            while Instant::now() < end {
                d.poll(&mut h)?;
                info = d.tcp_info(id).copied();
                if on && info.is_some_and(|i| !cfg.contains("tcp_rto_min_us") || i.rto_us < 200_000)
                {
                    break;
                }
            }
            let st = *d.stats();
            if on {
                let i = info.ok_or_else(|| format!("{cfg}: never sampled: {st:?}"))?;
                assert!(st.tcp_info_samples > 0, "{cfg}: {st:?}");
                assert!(st.tcp_info_max_ns > 0, "{cfg}: {st:?}");
                assert!(i.at_ns > 0, "{cfg}: {i:?}");
                assert!(i.rto_us > 0 && i.rto_us >= i.rtt_us, "{cfg}: {i:?}");
                assert!(i.min_rtt_us.is_some(), "{cfg}: {i:?}");
                // Loopback loses nothing.
                assert_eq!((i.total_retrans, i.lost), (0, 0), "{cfg}: {i:?}");
                if cfg.contains("tcp_rto_min_us") {
                    // The kernel's RTO is about srtt + max(rttvar, floor):
                    // 20 ms and a little on loopback, far below the 200 ms
                    // default floor.
                    assert!((20_000..200_000).contains(&i.rto_us), "{cfg}: {i:?}");
                }
                // A closed id has no sample.
                d.close(id);
                assert!(d.tcp_info(id).is_none(), "{cfg}: sample of a closed id");
            } else {
                assert_eq!(info, None, "{cfg}: sampled while off");
                assert_eq!(st.tcp_info_samples, 0, "{cfg}: {st:?}");
                assert_eq!(st.tcp_info_max_ns, 0, "{cfg}: {st:?}");
            }
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
    }
    Ok(())
}

#[derive(Default)]
struct Echoed(usize, Vec<String>);

impl Handler for Echoed {
    fn on_closed(&mut self, _: ConnId, error: Error) {
        self.1.push(error.to_string());
    }
}

impl WsHandler for Echoed {
    fn on_message(&mut self, _: ConnId, _: u8, _: &[u8], _: &RxMeta, _: u16) {
        self.0 += 1;
    }
}
