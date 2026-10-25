//! UDP through the driver over loopback: whole datagrams both ways, empty
//! datagrams, truncation, `ECONNREFUSED`, the send ring, receive-only
//! sockets, the per-socket read burst, the `epoll_wait` batch, the socket
//! options a datagram socket gets, and the backends that refuse it.
#![cfg(target_os = "linux")]
#![allow(missing_docs, unsafe_code)]

use std::net::{SocketAddr, UdpSocket};
use std::os::fd::RawFd;
use std::time::{Duration, Instant};

use ergon_io::route::QuickAckCheck;
use ergon_io::{
    Backend, Config, ConnId, Driver, Error, Handler, RxMeta, UdpConn, UdpHandler, UdpOptions,
};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct Got {
    dgrams: Vec<(ConnId, Vec<u8>, RxMeta)>,
    closed: Vec<(ConnId, String)>,
}

impl Handler for Got {
    fn on_closed(&mut self, id: ConnId, error: Error) {
        self.closed.push((id, error.to_string()));
    }
}

impl UdpHandler for Got {
    fn on_datagram(&mut self, conn: ConnId, payload: &[u8], meta: &RxMeta) {
        self.dgrams.push((conn, payload.to_vec(), *meta));
    }
}

fn any_port() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 0))
}

fn driver(cfg: &str) -> R<Driver<UdpConn>> {
    Ok(Driver::new(Config::parse(cfg)?)?)
}

/// Two sockets connected to each other.
fn pair(opts: &UdpOptions) -> R<(UdpConn, UdpConn)> {
    let a = UdpSocket::bind(any_port())?;
    let b = UdpSocket::bind(any_port())?;
    let (pa, pb) = (a.local_addr()?, b.local_addr()?);
    drop((a, b));
    Ok((
        UdpConn::connect(pa, pb, opts)?,
        UdpConn::connect(pb, pa, opts)?,
    ))
}

fn poll_until(d: &mut Driver<UdpConn>, h: &mut Got, done: impl Fn(&Got) -> bool) -> R {
    let end = Instant::now() + Duration::from_secs(10);
    while !done(h) {
        d.poll(h)?;
        if Instant::now() > end {
            return Err(format!(
                "timed out: {} datagrams, closed {:?}, stats {:?}",
                h.dgrams.len(),
                h.closed,
                d.stats()
            )
            .into());
        }
    }
    Ok(())
}

/// Waits until `fd` has a datagram (or an error) to read.
fn readable(fd: RawFd) -> R {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one valid pollfd for the duration of the call.
    let n = unsafe { libc::poll(&raw mut p, 1, 5_000) };
    if n == 1 {
        Ok(())
    } else {
        Err(format!("socket never became readable: {n}").into())
    }
}

fn sockopt(fd: RawFd, level: libc::c_int, opt: libc::c_int) -> R<libc::c_int> {
    let mut v: libc::c_int = 0;
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
    // SAFETY: `v` and `len` are valid for the call.
    let rc = unsafe { libc::getsockopt(fd, level, opt, (&raw mut v).cast(), &raw mut len) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(v)
}

/// UDP runs on `epoll-spin` only: the other two backends refuse a UDP
/// driver at `new`, typed, before any socket is taken.
#[test]
fn udp_refuses_every_backend_but_epoll_spin() -> R {
    for b in [Backend::RecvBusy, Backend::UringSqpoll] {
        let made: Result<Driver<UdpConn>, Error> = Driver::new(Config::new(b));
        let Err(Error::Unsupported(why)) = made else {
            return Err(format!("{b}: {:?}", made.err()).into());
        };
        assert!(why.contains("epoll-spin"), "{b}: {why}");
    }
    let _ok: Driver<UdpConn> = Driver::new(Config::new(Backend::EpollSpin))?;
    Ok(())
}

// ANCHOR: udp_round_trip
/// Datagrams both ways, each whole and in order, an empty one included: a
/// 0-byte datagram is delivered, never taken for a close.
#[test]
fn a_connected_pair_round_trips_whole_datagrams() -> R {
    let mut d = driver("epoll-spin")?;
    let (a, b) = pair(&UdpOptions::default())?;
    let (a, b) = (d.add(a)?, d.add(b)?);
    let sizes = [0usize, 1, 100, 2_048, 0];
    for &n in &sizes {
        let payload = vec![u8::try_from(n % 251)?; n];
        d.get_mut(a).ok_or("a gone")?.send(&payload)?;
    }
    let mut h = Got::default();
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == sizes.len())?;
    let got: Vec<usize> = h.dgrams.iter().map(|(_, p, _)| p.len()).collect();
    assert_eq!(got, sizes);
    assert!(h.dgrams.iter().all(|(id, _, _)| *id == b));
    // The other way.
    d.get_mut(b).ok_or("b gone")?.send(b"pong")?;
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == sizes.len() + 1)?;
    assert_eq!(
        h.dgrams.last().map(|(id, p, _)| (*id, p.as_slice())),
        Some((a, &b"pong"[..]))
    );
    assert!(h.closed.is_empty(), "{:?}", h.closed);
    assert_eq!(d.len(), 2);
    let (sa, sb) = (d.get(a).ok_or("a")?.stats(), d.get(b).ok_or("b")?.stats());
    assert_eq!((sa.tx, sa.rx, sb.tx, sb.rx), (5, 1, 1, 5));
    Ok(())
}
// ANCHOR_END: udp_round_trip

/// Kernel receive timestamps on each datagram under `rx_timestamps` (the
/// default), none without; each read has its own sequence number.
#[test]
fn rx_timestamps_ride_on_each_datagram_only_when_asked() -> R {
    for (cfg, stamped) in [
        ("epoll-spin", true),
        ("epoll-spin,rx_timestamps=false", false),
    ] {
        let mut d = driver(cfg)?;
        let (a, b) = pair(&UdpOptions::default())?;
        let (a, _b) = (d.add(a)?, d.add(b)?);
        for _ in 0..3 {
            d.get_mut(a).ok_or("a gone")?.send(b"x")?;
        }
        let mut h = Got::default();
        poll_until(&mut d, &mut h, |h| h.dgrams.len() == 3)?;
        for (_, _, m) in &h.dgrams {
            assert_eq!(m.rx_sw_ns != 0, stamped, "{cfg}: {m:?}");
        }
        let seqs: Vec<u64> = h.dgrams.iter().map(|(_, _, m)| m.read_seq).collect();
        assert!(seqs.windows(2).all(|w| w[1] > w[0]), "{cfg}: {seqs:?}");
    }
    Ok(())
}

/// A datagram longer than `max_datagram` is cut by the kernel
/// (`MSG_TRUNC`): it is counted and dropped, never delivered short, and the
/// next one arrives.
#[test]
fn a_datagram_over_max_datagram_is_counted_and_dropped() -> R {
    let mut d = driver("epoll-spin")?;
    let peer = UdpSocket::bind(any_port())?;
    let opts = UdpOptions {
        max_datagram: 64,
        ..UdpOptions::default()
    };
    let conn = UdpConn::connect(any_port(), peer.local_addr()?, &opts)?;
    peer.connect(conn.local_addr()?)?;
    let id = d.add(conn)?;
    peer.send(&[7u8; 100])?;
    peer.send(&[9u8; 64])?;
    let mut h = Got::default();
    poll_until(&mut d, &mut h, |h| !h.dgrams.is_empty())?;
    assert_eq!(h.dgrams.len(), 1);
    assert_eq!(h.dgrams[0].1, vec![9u8; 64]);
    assert_eq!(d.get(id).ok_or("gone")?.stats().truncated, 1);
    assert!(h.closed.is_empty(), "{:?}", h.closed);
    Ok(())
}

/// Sending to a port nobody listens on: the kernel reports `ECONNREFUSED`
/// (ICMP port unreachable) on a later read or send. It is counted; the
/// socket stays open and keeps sending.
#[test]
fn connection_refused_is_counted_not_a_close() -> R {
    let mut d = driver("epoll-spin")?;
    let gone = UdpSocket::bind(any_port())?.local_addr()?;
    let id = d.add(UdpConn::connect(any_port(), gone, &UdpOptions::default())?)?;
    let mut h = Got::default();
    let end = Instant::now() + Duration::from_secs(10);
    let mut sent = 0;
    while d.get(id).ok_or("closed")?.stats().refused < 2 {
        if d.get(id).ok_or("closed")?.queued() == 0 {
            d.get_mut(id).ok_or("closed")?.send(b"anyone?")?;
            sent += 1;
        }
        d.poll(&mut h)?;
        assert!(h.closed.is_empty(), "closed: {:?}", h.closed);
        if Instant::now() > end {
            return Err(format!("no refusal after {sent} sends: {:?}", d.stats()).into());
        }
    }
    assert_eq!(d.len(), 1);
    assert!(d.get(id).ok_or("closed")?.stats().tx >= 2);
    Ok(())
}

/// The send ring holds `tx_slots` datagrams: one more is `Busy` and
/// counted; once the driver has sent them there is room again.
#[test]
fn a_full_send_ring_is_busy_until_the_driver_sends() -> R {
    let mut d = driver("epoll-spin")?;
    let opts = UdpOptions {
        tx_slots: 2,
        ..UdpOptions::default()
    };
    let (a, b) = pair(&opts)?;
    let (a, _b) = (d.add(a)?, d.add(b)?);
    {
        let mut c = d.get_mut(a).ok_or("a gone")?;
        c.send(b"1")?;
        c.send(b"2")?;
        let full = c.send(b"3");
        assert!(matches!(full, Err(Error::Busy(_))), "{full:?}");
        assert_eq!((c.queued(), c.stats().tx_full), (2, 1));
        let big = c.send(&[0u8; 2_049]);
        assert!(matches!(big, Err(Error::Config(_))), "{big:?}");
    }
    d.flush()?;
    assert_eq!(d.get(a).ok_or("a gone")?.queued(), 0);
    d.get_mut(a).ok_or("a gone")?.send(b"3")?;
    let mut h = Got::default();
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == 3)?;
    let got: Vec<&[u8]> = h.dgrams.iter().map(|(_, p, _)| p.as_slice()).collect();
    assert_eq!(got, [&b"1"[..], b"2", b"3"]);
    Ok(())
}

/// A receive-only socket takes datagrams from any sender and has no peer
/// to send to.
#[test]
fn a_bound_socket_receives_from_anyone_and_refuses_to_send() -> R {
    let mut d = driver("epoll-spin")?;
    let conn = UdpConn::bind(any_port(), &UdpOptions::default())?;
    let at = conn.local_addr()?;
    assert_eq!(conn.peer_addr(), None);
    let id = d.add(conn)?;
    let (s1, s2) = (UdpSocket::bind(any_port())?, UdpSocket::bind(any_port())?);
    s1.send_to(b"one", at)?;
    s2.send_to(b"two", at)?;
    let mut h = Got::default();
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == 2)?;
    let refused = d.get_mut(id).ok_or("gone")?.send(b"x");
    assert!(matches!(refused, Err(Error::Config(_))), "{refused:?}");
    Ok(())
}

/// Sends queued on a socket before the driver takes it go out on the next
/// poll.
#[test]
fn datagrams_queued_before_add_are_sent() -> R {
    let mut d = driver("epoll-spin")?;
    let (mut a, b) = pair(&UdpOptions::default())?;
    a.send(b"early")?;
    let (_a, _b) = (d.add(a)?, d.add(b)?);
    let mut h = Got::default();
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == 1)?;
    assert_eq!(h.dgrams[0].1, b"early");
    Ok(())
}

/// One poll reads at most `datagram_burst` datagrams from a ready socket;
/// the rest wait for the next polls (level-triggered).
#[test]
fn a_poll_reads_at_most_datagram_burst_from_one_socket() -> R {
    let mut d = driver("epoll-spin,datagram_burst=4")?;
    let conn = UdpConn::bind(any_port(), &UdpOptions::default())?;
    let at = conn.local_addr()?;
    let fd = conn.raw_fd();
    d.add(conn)?;
    let s = UdpSocket::bind(any_port())?;
    for i in 0..10u8 {
        s.send_to(&[i], at)?;
    }
    readable(fd)?;
    let mut h = Got::default();
    let mut per_poll = Vec::new();
    while h.dgrams.len() < 10 && per_poll.len() < 100 {
        per_poll.push(d.poll(&mut h)?);
    }
    per_poll.retain(|&n| n > 0);
    assert_eq!(per_poll, [4, 4, 2]);
    let got: Vec<u8> = h.dgrams.iter().map(|(_, p, _)| p[0]).collect();
    assert_eq!(got, (0..10).collect::<Vec<u8>>());
    Ok(())
}

/// `epoll_events` caps the ready sockets one poll learns of: with 2 and
/// five sockets holding a datagram each, the first poll delivers 2.
#[test]
fn epoll_events_caps_the_sockets_one_poll_reads() -> R {
    let mut d = driver("epoll-spin,epoll_events=2")?;
    let s = UdpSocket::bind(any_port())?;
    let mut fds = Vec::new();
    for _ in 0..5 {
        let conn = UdpConn::bind(any_port(), &UdpOptions::default())?;
        s.send_to(b"x", conn.local_addr()?)?;
        fds.push(conn.raw_fd());
        d.add(conn)?;
    }
    for fd in fds {
        readable(fd)?;
    }
    let mut h = Got::default();
    assert_eq!(d.poll(&mut h)?, 2);
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == 5)?;
    Ok(())
}

/// A datagram socket gets no TCP option (keepalive and `TCP_USER_TIMEOUT`
/// are on by default: setting them on UDP would fail the add) and no
/// quick-ACK route check, even under `require`; `SO_RCVBUF`,
/// `SO_TIMESTAMPING` and `SO_BUSY_POLL` (0 on `epoll-spin`) are set.
#[test]
fn a_datagram_socket_gets_the_socket_options_but_no_tcp_ones() -> R {
    let cfg = Config::parse("epoll-spin,quickack_check=require,rcvbuf=1048576")?;
    assert!(cfg.keepalive_idle_s > 0 && cfg.tcp_user_timeout_ms > 0);
    assert_eq!(cfg.quickack_check, QuickAckCheck::Require);
    let mut d: Driver<UdpConn> = Driver::new(cfg)?;
    let (a, _b) = pair(&UdpOptions::default())?;
    let fd = a.raw_fd();
    let before = sockopt(fd, libc::SOL_SOCKET, libc::SO_RCVBUF)?;
    let id = d.add(a)?;
    assert_eq!(d.stats().route_lookups, 0);
    let granted = ergon_io::sys::granted_rcvbuf(1 << 20)?;
    let rcvbuf = libc::c_int::cast_unsigned(sockopt(fd, libc::SOL_SOCKET, libc::SO_RCVBUF)?);
    assert_eq!(rcvbuf, granted, "SO_RCVBUF (was {before})");
    assert_ne!(
        sockopt(fd, libc::SOL_SOCKET, libc::SO_TIMESTAMPING)?,
        0,
        "SO_TIMESTAMPING"
    );
    assert_eq!(sockopt(fd, libc::SOL_SOCKET, libc::SO_BUSY_POLL)?, 0);
    assert_eq!(sockopt(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE)?, 0);
    assert!(d.get(id).is_some());
    Ok(())
}
