//! The send set (`Conns::tx_dirty`) against a full scan of the slots, on
//! every backend, and the sends it must not lose: one queued on an idle
//! connection among many, a reply queued by a receive, a queue larger than
//! the socket takes in one pass, and the sends queued with no `get_mut` (an
//! endpoint added with bytes queued, a reply to input buffered before it was
//! added, a timer's Ping).

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

use super::Driver;
use crate::backend::{Backend, Config};
use crate::endpoint::sealed::{Deliver, Io, RouteSlot};
use crate::endpoint::{ConnId, Handler};
use crate::error::{Error, Result};
use crate::{RxMeta, route::QuickAckCheck};

type R<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A connection over a loopback TCP socket: every `r` it receives queues a
/// `pong` reply (a send queued by a receive, as a WebSocket Pong is).
struct Stub {
    sock: TcpStream,
    rx: Box<[u8]>,
    tx: crate::stream::TxQueue,
    route: RouteSlot,
    /// Input held from before it was added: decoding it queues a `pong`.
    buffered: bool,
    /// A timer due: the next `expire` queues a `ping` (a WebSocket Ping).
    ping_due: bool,
}

impl Io for Stub {
    const DEADLINES: bool = true;
    fn expire(&mut self, _: i64) -> Result<()> {
        if std::mem::take(&mut self.ping_due) {
            self.tx.extend_from_slice(b"ping");
        }
        Ok(())
    }
    fn fd(&self) -> RawFd {
        self.sock.as_raw_fd()
    }
    fn rx_space(&mut self) -> &mut [u8] {
        // Each receive is consumed whole, so the buffer is reused from the
        // front and never moves (io_uring writes into it later).
        &mut self.rx
    }
    fn tx_queue(&mut self) -> &mut crate::stream::TxQueue {
        &mut self.tx
    }
    fn has_buffered_input(&self) -> bool {
        self.buffered
    }
    fn set_route(&mut self, route: RouteSlot) {
        self.route = route;
    }
    fn route_state(&self) -> &RouteSlot {
        &self.route
    }
}

impl<H: ?Sized> Deliver<H> for Stub {
    fn on_received(&mut self, _: ConnId, n: usize, _: &RxMeta, _: &mut H) -> Result<usize> {
        if n == 0 && std::mem::take(&mut self.buffered) {
            self.tx.extend_from_slice(b"pong");
        }
        let replies = self.rx.get(..n).unwrap_or_default().iter();
        for _ in replies.filter(|&&b| b == b'r') {
            self.tx.extend_from_slice(b"pong");
        }
        Ok(n)
    }
}

#[derive(Default)]
struct Closed(Vec<(ConnId, String)>);

impl Handler for Closed {
    fn on_closed(&mut self, id: ConnId, error: Error) {
        self.0.push((id, error.to_string()));
    }
}

/// The server side of one connection and what it has read.
struct Peer {
    sock: TcpStream,
    got: Vec<u8>,
}

impl Peer {
    /// Reads whatever has arrived.
    fn drain(&mut self) -> R {
        let mut b = [0u8; 1 << 14];
        loop {
            match self.sock.read(&mut b) {
                Ok(0) => return Ok(()),
                Ok(n) => self.got.extend_from_slice(b.get(..n).unwrap_or_default()),
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
    }
}

fn pair(l: &TcpListener) -> R<(Stub, Peer)> {
    let c = TcpStream::connect(l.local_addr()?)?;
    let (s, _) = l.accept()?;
    c.set_nonblocking(true)?;
    s.set_nonblocking(true)?;
    let stub = Stub {
        sock: c,
        rx: vec![0; 4096].into_boxed_slice(),
        tx: crate::stream::TxQueue::default(),
        route: RouteSlot::default(),
        buffered: false,
        ping_due: false,
    };
    Ok((
        stub,
        Peer {
            sock: s,
            got: Vec::new(),
        },
    ))
}

/// A driver for `b`, or `None` where this host cannot run it: the `uring`
/// feature is off, or a pod with one CPU has none for `uring-sqpoll`'s
/// kernel thread (refused typed, see `placement_tests`).
fn driver(b: Backend) -> R<Option<Driver<Stub>>> {
    let mut cfg = Config::new(b);
    // Loopback: no route to look up.
    cfg.quickack_check = QuickAckCheck::Off;
    match Driver::new(cfg) {
        Ok(d) => Ok(Some(d)),
        Err(Error::Unsupported(_)) if !cfg!(feature = "uring") && b.name().starts_with("uring") => {
            Ok(None)
        }
        Err(Error::Cpu(_)) if b == Backend::UringSqpoll => Ok(None),
        Err(e) => Err(format!("{b}: {e}").into()),
    }
}

/// Backends each test must run on: all but `uring-sqpoll` (refused on a
/// one-CPU host); without the `uring` feature, the epoll ones.
fn backends_needed() -> usize {
    if cfg!(feature = "uring") {
        Backend::KERNEL.len() - 1
    } else {
        Backend::KERNEL
            .iter()
            .filter(|b| !b.name().starts_with("uring"))
            .count()
    }
}

fn queue(d: &mut Driver<Stub>, id: ConnId, bytes: &[u8]) -> R {
    let mut c = d.get_mut(id).ok_or("connection gone")?;
    c.inner().tx.extend_from_slice(bytes);
    Ok(())
}

/// Polls (draining every peer) until `done` holds, for at most 10 s.
fn poll_until(
    d: &mut Driver<Stub>,
    peers: &mut [Peer],
    h: &mut Closed,
    done: impl Fn(&[Peer]) -> bool,
) -> R {
    let end = Instant::now() + Duration::from_secs(10);
    while !done(peers) {
        d.poll(h)?;
        d.conns.tx_dirty_check(true)?;
        for p in peers.iter_mut() {
            p.drain()?;
        }
        if Instant::now() > end {
            let got: Vec<usize> = peers.iter().map(|p| p.got.len()).collect();
            return Err(format!("timed out: peers got {got:?}, closed {:?}", h.0).into());
        }
    }
    Ok(())
}

/// A deterministic pseudo-random sequence (an LCG) for the operations.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or(0) % n.max(1)
    }
}

/// Queues, bare `get_mut`s, receives that queue replies, polls, flushes, a
/// queue larger than the socket, and closes whose slot a new connection
/// takes: after each, the send set holds every open slot with bytes queued
/// (a full scan), and after each send pass exactly those. Then every byte
/// queued reaches its peer.
#[test]
fn the_send_set_matches_a_full_scan() -> R {
    let mut ran = 0;
    for b in Backend::KERNEL {
        let Some(mut d) = driver(b)? else {
            eprintln!("backend {b}: not on this host");
            continue;
        };
        eprintln!("backend {b}");
        ran += 1;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let mut ids = Vec::new();
        let mut peers = Vec::new();
        // What each connection's peer must end up with: the bytes queued
        // through `get_mut`, in order (all below 100), and the replies
        // (`pong`, letters above 100), which interleave with them.
        let mut want: Vec<Vec<u8>> = Vec::new();
        let mut pongs: Vec<usize> = Vec::new();
        for _ in 0..8 {
            let (stub, peer) = pair(&listener)?;
            ids.push(d.add(stub)?);
            peers.push(peer);
            want.push(Vec::new());
            pongs.push(0);
        }
        d.conns
            .tx_dirty_check(true)
            .map_err(|e| format!("{b}: {e}"))?;
        let mut closed = Closed::default();
        // More than the loopback socket takes at once, its peer not read
        // until the end: connection 0 keeps bytes queued (or a send in
        // flight) through every pass.
        let big: Vec<u8> = (0..8u32 << 20)
            .map(|i| (i % 100).to_le_bytes()[0])
            .collect();
        queue(&mut d, ids[0], &big)?;
        want[0].extend_from_slice(&big);
        let mut rng = Lcg(7 + u64::try_from(b.name().len()).unwrap_or(0));
        for step in 0..600 {
            let k = rng.below(ids.len());
            let exact = match rng.below(8) {
                0 | 1 => {
                    let bytes: Vec<u8> = (0..=rng.below(64))
                        .map(|i| ((step + i) % 100).to_le_bytes()[0])
                        .collect();
                    queue(&mut d, ids[k], &bytes)?;
                    want[k].extend_from_slice(&bytes);
                    false
                }
                2 => {
                    // Listed, nothing queued: the next pass drops it.
                    d.get_mut(ids[k]).ok_or("connection gone")?;
                    false
                }
                3 if k != 0 => {
                    let replies = 1 + rng.below(3);
                    peers[k].sock.write_all(&b"rrr"[..replies])?;
                    pongs[k] += replies;
                    false
                }
                4 => {
                    d.flush_with(&mut closed)?;
                    true
                }
                5 if k != 0 => {
                    // The slot is reused at once by a new connection.
                    d.close(ids[k]);
                    d.conns
                        .tx_dirty_check(false)
                        .map_err(|e| format!("{b}: {e}"))?;
                    let (stub, peer) = pair(&listener)?;
                    ids[k] = d.add(stub)?;
                    peers[k] = peer;
                    want[k].clear();
                    pongs[k] = 0;
                    false
                }
                _ => {
                    d.poll(&mut closed)?;
                    for p in peers.iter_mut().skip(1) {
                        p.drain()?;
                    }
                    true
                }
            };
            d.conns
                .tx_dirty_check(exact)
                .map_err(|e| format!("{b}, step {step}: {e}"))?;
        }
        assert!(closed.0.is_empty(), "{b}: {:?}", closed.0);
        poll_until(&mut d, &mut peers, &mut closed, |p| {
            p.iter()
                .zip(want.iter().zip(&pongs))
                .all(|(p, (w, &m))| p.got.len() >= w.len() + 4 * m)
        })
        .map_err(|e| format!("{b}: {e}"))?;
        for (k, p) in peers.iter().enumerate() {
            let (replies, queued): (Vec<u8>, Vec<u8>) = p.got.iter().partition(|&&x| x >= 100);
            assert!(queued == want[k], "{b}: connection {k}: wrong bytes");
            assert!(
                replies == b"pong".repeat(pongs[k]),
                "{b}: connection {k}: wrong replies"
            );
        }
    }
    assert!(ran >= backends_needed(), "only {ran} backends ran");
    Ok(())
}

/// Among many idle connections, a send queued on one (the last slot, then
/// one in the middle) goes out on the next polls, a reply queued by a
/// receive on another goes out without any `get_mut`, and a queue larger
/// than the socket keeps going out over later polls with no further call.
#[test]
fn a_send_queued_on_an_idle_slot_still_goes_out() -> R {
    let mut ran = 0;
    for b in Backend::KERNEL {
        let Some(mut d) = driver(b)? else {
            eprintln!("backend {b}: not on this host");
            continue;
        };
        eprintln!("backend {b}");
        ran += 1;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let mut ids = Vec::new();
        let mut peers = Vec::new();
        for _ in 0..64 {
            let (stub, peer) = pair(&listener)?;
            ids.push(d.add(stub)?);
            peers.push(peer);
        }
        let mut closed = Closed::default();
        for _ in 0..100 {
            d.poll(&mut closed)?;
        }
        for k in [63, 31] {
            queue(&mut d, ids[k], b"hello")?;
            poll_until(&mut d, &mut peers, &mut closed, |p| p[k].got.len() >= 5)
                .map_err(|e| format!("{b}: slot {k}: {e}"))?;
            assert_eq!(peers[k].got, b"hello", "{b}");
            peers[k].got.clear();
        }
        peers[17].sock.write_all(b"r")?;
        poll_until(&mut d, &mut peers, &mut closed, |p| p[17].got.len() >= 4)
            .map_err(|e| format!("{b}: reply: {e}"))?;
        assert_eq!(peers[17].got, b"pong", "{b}");
        let big = vec![7u8; 8 << 20];
        queue(&mut d, ids[5], &big)?;
        poll_until(&mut d, &mut peers, &mut closed, |p| {
            p[5].got.len() >= big.len()
        })
        .map_err(|e| format!("{b}: large queue: {e}"))?;
        assert!(peers[5].got == big, "{b}: large queue corrupted");
        let others: usize = (0..64)
            .filter(|&k| ![5, 17].contains(&k))
            .map(|k| peers[k].got.len())
            .sum();
        assert_eq!(
            others, 0,
            "{b}: bytes reached a connection nothing was queued on"
        );
        assert!(closed.0.is_empty(), "{b}: {:?}", closed.0);
    }
    assert!(ran >= backends_needed(), "only {ran} backends ran");
    Ok(())
}

/// A case of `sends_queued_without_get_mut_go_out`: its name, what it does to
/// the endpoint before `add`, and what the peer must receive.
type Case = (&'static str, fn(&mut Stub), &'static [u8]);

/// Sends queued with no `get_mut` among idle connections: by an endpoint
/// added with bytes already queued, by decoding input it held from before it
/// was added, and by a timer (`expire`). Each goes out on the next polls, and
/// the send set lists it at every step (a full scan).
#[test]
fn sends_queued_without_get_mut_go_out() -> R {
    let mut ran = 0;
    for b in Backend::KERNEL {
        let Some(mut d) = driver(b)? else {
            eprintln!("backend {b}: not on this host");
            continue;
        };
        eprintln!("backend {b}");
        ran += 1;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let mut peers = Vec::new();
        for _ in 0..8 {
            let (stub, peer) = pair(&listener)?;
            d.add(stub)?;
            peers.push(peer);
        }
        let mut closed = Closed::default();
        for _ in 0..100 {
            d.poll(&mut closed)?;
        }
        let cases: [Case; 3] = [
            (
                "added with bytes queued",
                |s| s.tx.extend_from_slice(b"early"),
                b"early",
            ),
            ("buffered input's reply", |s| s.buffered = true, b"pong"),
            ("timer", |s| s.ping_due = true, b"ping"),
        ];
        for (what, set, want) in cases {
            let (mut stub, peer) = pair(&listener)?;
            set(&mut stub);
            d.add(stub)?;
            d.conns
                .tx_dirty_check(false)
                .map_err(|e| format!("{b}: {what}: {e}"))?;
            peers.push(peer);
            let k = peers.len() - 1;
            poll_until(&mut d, &mut peers, &mut closed, |p| {
                p[k].got.len() >= want.len()
            })
            .map_err(|e| format!("{b}: {what}: {e}"))?;
            assert_eq!(peers[k].got, want, "{b}: {what}");
        }
        let others: usize = peers.iter().take(8).map(|p| p.got.len()).sum();
        assert_eq!(others, 0, "{b}: bytes reached an idle connection");
        assert!(closed.0.is_empty(), "{b}: {:?}", closed.0);
    }
    assert!(ran >= backends_needed(), "only {ran} backends ran");
    Ok(())
}

/// A queue the socket takes only part of: the epoll send path moves the
/// queue's head and copies nothing (the unsent tail stays where it is);
/// every backend still delivers every byte in order.
#[test]
fn a_partial_send_leaves_the_unsent_tail_in_place() -> R {
    let mut ran = 0;
    for b in Backend::KERNEL {
        let Some(mut d) = driver(b)? else {
            eprintln!("backend {b}: not on this host");
            continue;
        };
        ran += 1;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let (stub, peer) = pair(&listener)?;
        let id = d.add(stub)?;
        let mut peers = vec![peer];
        let mut closed = Closed::default();
        let big: Vec<u8> = (0..8 << 20).map(|i: u32| i.to_le_bytes()[1]).collect();
        queue(&mut d, id, &big)?;
        // One pass without the peer reading: the socket takes a part.
        d.poll(&mut closed)?;
        {
            let mut c = d.get_mut(id).ok_or("connection gone")?;
            let q = &c.inner().tx;
            if !b.name().starts_with("uring") {
                assert!(
                    q.head() > 0 && q.len() < big.len(),
                    "{b}: expected a partial send, head {} len {}",
                    q.head(),
                    q.len()
                );
            }
            assert_eq!(q.moved(), 0, "{b}: the unsent tail was moved");
        }
        poll_until(&mut d, &mut peers, &mut closed, |p| {
            p[0].got.len() >= big.len()
        })
        .map_err(|e| format!("{b}: {e}"))?;
        assert!(peers[0].got == big, "{b}: bytes out of order or lost");
        let mut c = d.get_mut(id).ok_or("connection gone")?;
        let q = &c.inner().tx;
        assert_eq!((q.head(), q.len(), q.moved()), (0, 0, 0), "{b}");
        assert!(closed.0.is_empty(), "{b}: {:?}", closed.0);
    }
    assert!(ran >= backends_needed(), "only {ran} backends ran");
    Ok(())
}
