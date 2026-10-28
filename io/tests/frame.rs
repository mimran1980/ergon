//! Length-prefixed frames over plain TCP. Fed without a driver (every
//! platform): random fragmentation, empty frames, oversize frames, the send
//! queue's format and bound. Through the driver over loopback on every
//! backend (Linux): `FrameDial` and `from_stream` + `add`, echo, peer
//! close, a close inside a frame, `TCP_NODELAY`, `quickack_per_read`, the
//! TCP timers and the quick-ACK route check.
#![allow(missing_docs, unsafe_code, clippy::cast_possible_truncation)]

use std::net::{TcpListener, TcpStream};

use ergon_io::{ConnId, Error, FrameConn, FrameHandler, FrameOptions, Handler, RxMeta};
use proptest::prelude::*;
use proptest::test_runner::{Config as PropConfig, RngSeed, TestRunner};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct Got {
    frames: Vec<(ConnId, Vec<u8>)>,
    metas: Vec<RxMeta>,
    closed: Vec<(ConnId, String)>,
    opened: Vec<ConnId>,
}

impl Handler for Got {
    fn on_open(&mut self, id: ConnId) {
        self.opened.push(id);
    }
    fn on_closed(&mut self, id: ConnId, error: Error) {
        self.closed.push((id, error.to_string()));
    }
}

impl FrameHandler for Got {
    fn on_frame(&mut self, conn: ConnId, payload: &[u8], meta: &RxMeta) {
        self.frames.push((conn, payload.to_vec()));
        self.metas.push(*meta);
    }
}

/// `payload` as it goes on the wire.
fn wire(payload: &[u8]) -> Vec<u8> {
    let mut out = (payload.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(payload);
    out
}

/// A connection over a loopback socket nothing is read from or written to:
/// `feed` stands in for the kernel.
fn rig(opts: FrameOptions) -> R<(FrameConn, TcpStream)> {
    let l = TcpListener::bind("127.0.0.1:0")?;
    let c = TcpStream::connect(l.local_addr()?)?;
    let (s, _) = l.accept()?;
    Ok((FrameConn::from_stream(c, opts)?, s))
}

/// Feeds `bytes` in pieces that each fit the free buffer, as reads would.
fn feed_all(c: &mut FrameConn, bytes: &[u8], h: &mut Got) -> R<usize> {
    let id = ConnId::default();
    let mut n = 0;
    let mut rest = bytes;
    while !rest.is_empty() {
        let take = rest.len().min(c.free());
        if take == 0 {
            return Err("no free buffer".into());
        }
        n += c.feed(id, &rest[..take], &RxMeta::default(), h)?;
        rest = &rest[take..];
    }
    Ok(n)
}

fn prop_config() -> PropConfig {
    PropConfig {
        cases: 128,
        rng_seed: RngSeed::Fixed(0x6672_616d_655f_7463),
        failure_persistence: None,
        ..PropConfig::default()
    }
}

/// Any cut of any sequence of frames, empty ones included, delivers exactly
/// those frames in order, and leaves nothing buffered. A buffer just one
/// frame long makes most cases compact.
#[test]
fn any_fragmentation_delivers_the_original_frames() -> R {
    let max = 4_096;
    let strategy = (
        prop::collection::vec(
            (0usize..=max, any::<u8>()).prop_map(|(n, seed)| {
                (0..n)
                    .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
                    .collect::<Vec<u8>>()
            }),
            1..24,
        ),
        prop::collection::vec(any::<prop::sample::Index>(), 0..40),
        prop::bool::weighted(0.75),
    );
    let compacted = std::cell::Cell::new(0u32);
    let mut runner = TestRunner::new(prop_config());
    runner
        .run(&strategy, |(frames, cut_at, small)| {
            let fail = |e: Box<dyn std::error::Error>| TestCaseError::fail(e.to_string());
            let opts = FrameOptions {
                max_frame: max,
                rx_buf: if small { max + 4 } else { 1 << 20 },
                ..FrameOptions::default()
            };
            let (mut c, _peer) = rig(opts).map_err(fail)?;
            let stream: Vec<u8> = frames.iter().flat_map(|f| wire(f)).collect();
            let mut cuts: Vec<usize> = cut_at.iter().map(|i| i.index(stream.len())).collect();
            cuts.push(0);
            cuts.push(stream.len());
            cuts.sort_unstable();
            let mut h = Got::default();
            let mut delivered = 0;
            for w in cuts.windows(2) {
                delivered += feed_all(&mut c, &stream[w[0]..w[1]], &mut h).map_err(fail)?;
            }
            let got: Vec<Vec<u8>> = h.frames.into_iter().map(|(_, p)| p).collect();
            prop_assert_eq!(delivered, frames.len());
            prop_assert_eq!(&got, &frames);
            prop_assert_eq!(c.buffered(), 0);
            let s = c.stats();
            prop_assert_eq!(s.frames_in, frames.len() as u64);
            prop_assert_eq!(s.bytes_in, stream.len() as u64);
            compacted.set(compacted.get() + u32::from(s.compact_bytes > 0));
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    assert!(compacted.get() > 0, "no case compacted the buffer");
    Ok(())
}

/// A 0-length frame is a frame: delivered, empty, between two others.
#[test]
fn an_empty_frame_is_delivered() -> R {
    let (mut c, _peer) = rig(FrameOptions::default())?;
    let mut h = Got::default();
    let mut bytes = wire(b"a");
    bytes.extend(wire(b""));
    bytes.extend(wire(b"b"));
    assert_eq!(feed_all(&mut c, &bytes, &mut h)?, 3);
    let got: Vec<&[u8]> = h.frames.iter().map(|(_, p)| p.as_slice()).collect();
    assert_eq!(got, [&b"a"[..], b"", b"b"]);
    Ok(())
}

/// A length over `max_frame` is refused as soon as its 4 bytes arrive,
/// before any payload; `max_frame` itself is fine.
#[test]
fn a_length_over_max_frame_is_a_frame_error() -> R {
    let opts = FrameOptions {
        max_frame: 100,
        rx_buf: 104,
        ..FrameOptions::default()
    };
    let (mut conn, _peer) = rig(opts)?;
    let mut h = Got::default();
    assert_eq!(feed_all(&mut conn, &wire(&[1u8; 100]), &mut h)?, 1);
    let fed = feed_all(&mut conn, &101u32.to_le_bytes(), &mut h);
    let err = fed.err().ok_or("101 bytes accepted")?;
    assert!(
        matches!(err.downcast_ref::<Error>(), Some(Error::Frame(_))),
        "{err}"
    );
    // A buffer that cannot hold the largest frame is refused up front.
    let bad = FrameOptions {
        rx_buf: 103,
        ..opts
    };
    assert!(
        matches!(rig(bad).map(|_| ()).map_err(|e| e.to_string()), Err(m) if m.contains("rx_buf"))
    );
    Ok(())
}

/// `send_frame` queues the little-endian length then the payload; an
/// oversize payload is a frame error and a queue past `tx_buf` is busy,
/// with nothing queued either way.
#[test]
fn send_frame_queues_length_and_payload_within_tx_buf() -> R {
    let opts = FrameOptions {
        max_frame: 1_000,
        rx_buf: 1_004,
        tx_buf: 2_000,
        ..FrameOptions::default()
    };
    let (mut c, _peer) = rig(opts)?;
    c.send_frame(b"hello")?;
    c.send_frame(b"")?;
    assert_eq!(
        c.queued_tx(),
        [&wire(b"hello")[..], &wire(b"")[..]].concat()
    );
    let big = c.send_frame(&[0u8; 1_001]);
    assert!(matches!(big, Err(Error::Frame(_))), "{big:?}");
    c.send_frame(&[1u8; 1_000])?;
    let before = c.queued();
    let full = c.send_frame(&[2u8; 1_000]);
    assert!(matches!(full, Err(Error::Busy(_))), "{full:?}");
    assert_eq!(c.queued(), before, "a refused frame queued bytes");
    c.clear_tx();
    c.send_frame(&[2u8; 1_000])?;
    assert_eq!(c.stats().frames_out, 4);
    Ok(())
}

#[cfg(target_os = "linux")]
mod driver {
    use std::io::{Read, Write};
    use std::net::Shutdown;
    use std::time::{Duration, Instant};

    use ergon_io::placement::HelperThread;
    use ergon_io::route::QuickAckCheck;
    use ergon_io::{Backend, Config, Driver, FrameDial, Target};

    use super::*;

    fn poll_until(d: &mut Driver<FrameConn>, h: &mut Got, done: impl Fn(&Got) -> bool) -> R {
        let end = Instant::now() + Duration::from_secs(10);
        while !done(h) {
            d.poll(h)?;
            if Instant::now() > end {
                return Err(format!(
                    "timed out: {} frames, closed {:?}, stats {:?}",
                    h.frames.len(),
                    h.closed,
                    d.stats()
                )
                .into());
            }
        }
        Ok(())
    }

    /// Runs `test` on every backend; a pod with one CPU refuses
    /// `uring-sqpoll` typed (no core for its kernel thread).
    fn each_backend(test: impl Fn(Backend) -> R) -> R {
        let mut ran = 0;
        for b in Backend::KERNEL {
            match test(b) {
                Ok(()) => ran += 1,
                Err(e)
                    if b == Backend::UringSqpoll
                        && matches!(
                            e.downcast_ref::<Error>(),
                            Some(Error::Cpu(c)) if matches!(
                                **c,
                                ergon_io::placement::CpuError::NoSpareCpu {
                                    helper: HelperThread::SqPoll,
                                    ..
                                }
                            )
                        ) => {}
                Err(e) => return Err(format!("{b}: {e}").into()),
            }
        }
        assert!(ran >= Backend::KERNEL.len() - 1, "only {ran} backends ran");
        Ok(())
    }

    fn nodelay(fd: std::os::fd::RawFd) -> R<bool> {
        Ok(sockopt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY)? != 0)
    }

    fn sockopt(fd: std::os::fd::RawFd, level: libc::c_int, opt: libc::c_int) -> R<libc::c_int> {
        let mut v: libc::c_int = 0;
        let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())?;
        // SAFETY: `v` and `len` are valid for the call.
        let rc = unsafe { libc::getsockopt(fd, level, opt, (&raw mut v).cast(), &raw mut len) };
        if rc != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(v)
    }

    /// A peer that echoes every byte back until it reads end of stream.
    fn echo(sock: TcpStream) -> std::thread::JoinHandle<std::io::Result<u64>> {
        std::thread::spawn(move || {
            let mut sock = sock;
            let mut b = vec![0u8; 65_536];
            let mut total = 0;
            loop {
                let n = sock.read(&mut b)?;
                if n == 0 {
                    return Ok(total);
                }
                sock.write_all(&b[..n])?;
                total += n as u64;
            }
        })
    }

    // ANCHOR: frame_echo
    /// `FrameDial` through `Driver::connect`: frames of every size out and
    /// echoed back whole, then the peer's close reported as `Closed`.
    #[test]
    fn a_dialled_connection_echoes_frames_on_every_backend() -> R {
        each_backend(|b| {
            let mut d: Driver<FrameConn> = Driver::new(Config::new(b))?;
            let l = TcpListener::bind("127.0.0.1:0")?;
            let addr = l.local_addr()?;
            let target = Target::new("localhost", addr.port(), "").with_addr(addr);
            let id = d.connect(FrameDial::new(target, FrameOptions::default())?)?;
            let (server, _) = l.accept()?;
            let peer = echo(server.try_clone()?);
            let mut h = Got::default();
            poll_until(&mut d, &mut h, |h| h.opened.contains(&id))?;
            assert!(nodelay(d.get(id).ok_or("gone")?.raw_fd())?, "TCP_NODELAY");
            let sizes = [0usize, 1, 7, 1_000, 65_536, 3];
            for &n in &sizes {
                d.get_mut(id).ok_or("gone")?.send_frame(&vec![n as u8; n])?;
            }
            poll_until(&mut d, &mut h, |h| h.frames.len() == sizes.len())?;
            let got: Vec<usize> = h.frames.iter().map(|(_, p)| p.len()).collect();
            assert_eq!(got, sizes);
            assert!(
                h.frames
                    .iter()
                    .all(|(c, p)| *c == id && p.iter().all(|&x| x == p.len() as u8))
            );
            server.shutdown(Shutdown::Both)?;
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            assert_eq!(h.closed[0].0, id);
            assert!(h.closed[0].1.contains("peer closed"), "{:?}", h.closed);
            drop(d);
            let echoed = peer.join().map_err(|_| "echo panicked")??;
            assert!(echoed > 0);
            Ok(())
        })
    }
    // ANCHOR_END: frame_echo

    /// An accepted socket (`TCP_NODELAY` cleared first) through
    /// `from_stream` and `Driver::add`: `TCP_NODELAY` is set, frames flow
    /// both ways, each read's `RxMeta` has its sequence number.
    #[test]
    fn an_accepted_socket_added_to_the_driver_carries_frames() -> R {
        each_backend(|b| {
            let mut d: Driver<FrameConn> = Driver::new(Config::new(b))?;
            let l = TcpListener::bind("127.0.0.1:0")?;
            let mut client = TcpStream::connect(l.local_addr()?)?;
            let (accepted, _) = l.accept()?;
            accepted.set_nodelay(false)?;
            let id = d.add(FrameConn::from_stream(accepted, FrameOptions::default())?)?;
            assert!(nodelay(d.get(id).ok_or("gone")?.raw_fd())?, "TCP_NODELAY");
            client.write_all(&[wire(b"one"), wire(b"two")].concat())?;
            let mut h = Got::default();
            poll_until(&mut d, &mut h, |h| h.frames.len() == 2)?;
            assert!(h.metas.iter().all(|m| m.read_seq > 0));
            d.get_mut(id).ok_or("gone")?.send_frame(b"back")?;
            d.flush()?;
            let mut b4 = [0u8; 8];
            client.set_read_timeout(Some(Duration::from_secs(5)))?;
            client.read_exact(&mut b4)?;
            assert_eq!(&b4[..], &wire(b"back")[..]);
            Ok(())
        })
    }

    /// An oversize length from the peer closes that connection with
    /// `Error::Frame`; a peer close inside a frame is `Error::Frame` too,
    /// not a clean close.
    #[test]
    fn oversize_and_truncated_frames_close_with_a_frame_error() -> R {
        each_backend(|b| {
            let mut d: Driver<FrameConn> = Driver::new(Config::new(b))?;
            let l = TcpListener::bind("127.0.0.1:0")?;
            let opts = FrameOptions {
                max_frame: 1_000,
                rx_buf: 4_096,
                ..FrameOptions::default()
            };
            let mut c1 = TcpStream::connect(l.local_addr()?)?;
            let id1 = d.add(FrameConn::from_stream(l.accept()?.0, opts)?)?;
            let mut c2 = TcpStream::connect(l.local_addr()?)?;
            let id2 = d.add(FrameConn::from_stream(l.accept()?.0, opts)?)?;
            c1.write_all(&1_001u32.to_le_bytes())?;
            let mut part = wire(b"whole");
            part.extend_from_slice(&wire(&[5u8; 50])[..20]);
            c2.write_all(&part)?;
            c2.shutdown(Shutdown::Write)?;
            let mut h = Got::default();
            poll_until(&mut d, &mut h, |h| h.closed.len() == 2)?;
            let reason = |id| {
                h.closed
                    .iter()
                    .find(|(c, _)| *c == id)
                    .map(|(_, e)| e.clone())
                    .unwrap_or_default()
            };
            assert!(reason(id1).contains("over max_frame"), "{:?}", h.closed);
            assert!(reason(id2).contains("inside a frame"), "{:?}", h.closed);
            assert_eq!(h.frames.len(), 1, "the whole frame before the cut");
            assert!(d.is_empty());
            Ok(())
        })
    }

    /// `quickack_per_read` sets `TCP_QUICKACK` once per read with data; off
    /// (the default), never.
    #[test]
    fn quickack_per_read_sets_tcp_quickack_after_each_read() -> R {
        for on in [true, false] {
            let mut d: Driver<FrameConn> = Driver::new(Config::new(Backend::EpollSpin))?;
            let l = TcpListener::bind("127.0.0.1:0")?;
            let mut client = TcpStream::connect(l.local_addr()?)?;
            let opts = FrameOptions {
                quickack_per_read: on,
                ..FrameOptions::default()
            };
            let id = d.add(FrameConn::from_stream(l.accept()?.0, opts)?)?;
            let mut h = Got::default();
            for i in 0..20u8 {
                client.write_all(&wire(&[i; 10]))?;
                poll_until(&mut d, &mut h, |h| h.frames.len() == usize::from(i) + 1)?;
                if on {
                    // Set just now, after the read: quick-ACK mode is on.
                    let fd = d.get(id).ok_or("gone")?.raw_fd();
                    assert_eq!(sockopt(fd, libc::IPPROTO_TCP, libc::TCP_QUICKACK)?, 1);
                }
            }
            let reads = d.stats().completions;
            let set = d.get(id).ok_or("gone")?.stats().quickacks;
            assert_eq!(set, if on { reads } else { 0 }, "on={on}");
            assert!(reads >= 20);
        }
        Ok(())
    }

    /// `busy_poll_us` on `epoll-spin`: `SO_BUSY_POLL` reads back on the
    /// socket, and a read with data is followed by one more read of that
    /// socket on the next poll (the kernel busy-polls only a read that finds
    /// the receive queue empty: that read makes the NAPI pass). At 0 (the
    /// default) a drained socket is not read again.
    #[test]
    fn busy_poll_us_sets_so_busy_poll_and_reads_a_socket_to_empty() -> R {
        for us in [50u32, 0] {
            let cfg = Config::parse(&format!("epoll-spin,busy_poll_us={us}"))?;
            let mut d: Driver<FrameConn> = Driver::new(cfg)?;
            let l = TcpListener::bind("127.0.0.1:0")?;
            let mut client = TcpStream::connect(l.local_addr()?)?;
            let id = d.add(FrameConn::from_stream(
                l.accept()?.0,
                FrameOptions::default(),
            )?)?;
            let fd = d.get(id).ok_or("gone")?.raw_fd();
            let bp = ergon_io::sys::busy_poll_settings(fd)?;
            assert_eq!((bp.usec, bp.prefer), (us, false), "busy_poll_us={us}");
            let mut h = Got::default();
            for i in 0..5u8 {
                let start = d.stats().recvmsg_eagain;
                client.write_all(&wire(&[i; 10]))?;
                poll_until(&mut d, &mut h, |h| h.frames.len() == usize::from(i) + 1)?;
                // The receiving poll reads once: the empty read is not in it.
                assert_eq!(d.stats().recvmsg_eagain, start, "busy_poll_us={us}");
                let before = d.stats().recvmsg_eagain;
                d.poll(&mut h)?;
                d.poll(&mut h)?;
                let empty = d.stats().recvmsg_eagain - before;
                assert_eq!(empty, u64::from(us > 0), "busy_poll_us={us}, frame {i}");
            }
        }
        Ok(())
    }

    /// The TCP keys reach a framed connection: `tcp_rto_min_us` and
    /// `tcp_delack_max_us` read back exactly on a 6.15+ kernel (refused
    /// typed at `Driver::new` before), and the quick-ACK route check runs
    /// under `require` (a loopback route is local: allowed, one lookup).
    #[test]
    fn tcp_timers_and_the_route_check_apply() -> R {
        let cfg = "epoll-spin,tcp_rto_min_us=40000,tcp_delack_max_us=20000,quickack_check=require";
        let made: Result<Driver<FrameConn>, Error> = Driver::new(Config::parse(cfg)?);
        if !kernel_at_least(6, 15)? {
            assert!(
                matches!(made, Err(Error::Unsupported(_))),
                "{:?}",
                made.err()
            );
            return Ok(());
        }
        let mut d = made?;
        assert_eq!(d.config().quickack_check, QuickAckCheck::Require);
        let l = TcpListener::bind("127.0.0.1:0")?;
        let _client = TcpStream::connect(l.local_addr()?)?;
        let id = d.add(FrameConn::from_stream(
            l.accept()?.0,
            FrameOptions::default(),
        )?)?;
        let fd = d.get(id).ok_or("gone")?.raw_fd();
        assert_eq!(
            sockopt(fd, libc::IPPROTO_TCP, 45)?,
            40_000,
            "TCP_RTO_MIN_US"
        );
        assert_eq!(
            sockopt(fd, libc::IPPROTO_TCP, 46)?,
            20_000,
            "TCP_DELACK_MAX_US"
        );
        assert_eq!(d.stats().route_lookups, 1);
        Ok(())
    }

    /// Whether the running kernel is at least `major.minor`: the oracle for
    /// which outcome is expected, never a reason to skip.
    fn kernel_at_least(major: u32, minor: u32) -> R<bool> {
        let rel = std::fs::read_to_string("/proc/sys/kernel/osrelease")?;
        let mut parts = rel.trim().split(['.', '-']);
        let maj: u32 = parts.next().ok_or("no major")?.parse()?;
        let min: u32 = parts.next().ok_or("no minor")?.parse()?;
        Ok((maj, min) >= (major, minor))
    }
}
