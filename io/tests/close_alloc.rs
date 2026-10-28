//! Closing connections allocates nothing on the polling thread: a peer that
//! closes (TCP FIN, a WebSocket Close) and `Driver::close`, for 1 and for
//! 128 connections at once, after steady polling, on every backend. Only
//! allocations inside `Driver::poll` and `Driver::close` count: the test's
//! own peers allocate on the same thread, outside them. Its own test binary,
//! for the counting allocator. Linux only (the driver).
#![cfg(target_os = "linux")]
#![allow(missing_docs, unsafe_code, clippy::cast_possible_truncation)]

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

use ergon_io::placement::HelperThread;
use ergon_io::{
    Backend, Config, ConnId, Driver, Endpoint, FrameConn, FrameDial, FrameHandler, FrameOptions,
    Handler, Receive, RxMeta, Target, UdpConn, UdpHandler, UdpOptions, WsConn, WsHandler,
    WsOptions, ws,
};
use support::{Lab, Peer, R};

struct Counting;

thread_local! {
    /// This thread's allocations: ergon-io's helper threads do not count.
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
}

fn allocs() -> u64 {
    ALLOCS.with(Cell::get)
}

// SAFETY: forwards to the system allocator; only adds a thread-local counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count();
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const COUNTS: [usize; 2] = [1, 128];
const WAIT: Duration = Duration::from_secs(20);

/// Counts what it sees without allocating (the error is dropped here).
#[derive(Default)]
struct Sum {
    msgs: u64,
    opened: u64,
    closed: u64,
}

impl Handler for Sum {
    fn on_open(&mut self, _: ConnId) {
        self.opened += 1;
    }
    fn on_closed(&mut self, _: ConnId, _: ergon_io::Error) {
        self.closed += 1;
    }
}

impl FrameHandler for Sum {
    fn on_frame(&mut self, _: ConnId, _: &[u8], _: &RxMeta) {
        self.msgs += 1;
    }
}

impl UdpHandler for Sum {
    fn on_datagram(&mut self, _: ConnId, _: &[u8], _: &RxMeta) {
        self.msgs += 1;
    }
}

impl WsHandler for Sum {
    fn on_message(&mut self, _: ConnId, _: u8, _: &[u8], _: &RxMeta, _: u16) {
        self.msgs += 1;
    }
}

/// The driver and what its own calls allocated.
struct Rig<E> {
    d: Driver<E>,
    h: Sum,
    allocs: u64,
}

impl<E: Endpoint + Receive<Sum>> Rig<E> {
    fn new(b: Backend) -> R<Self> {
        Ok(Self {
            d: Driver::new(Config::new(b))?,
            h: Sum::default(),
            allocs: 0,
        })
    }

    fn poll(&mut self) -> R {
        let a = allocs();
        let r = self.d.poll(&mut self.h);
        self.allocs += allocs() - a;
        r?;
        Ok(())
    }

    fn poll_until(&mut self, what: &str, done: impl Fn(&Sum) -> bool) -> R {
        let end = Instant::now() + WAIT;
        while !done(&self.h) {
            self.poll()?;
            if Instant::now() > end {
                return Err(format!("timed out waiting for {what}").into());
            }
        }
        Ok(())
    }

    fn close(&mut self, id: ConnId) {
        let a = allocs();
        self.d.close(id);
        self.allocs += allocs() - a;
    }

    /// Polls for a while with nothing to do: steady polling, and any
    /// first-poll set-up out of the measured part.
    fn idle(&mut self) -> R {
        for _ in 0..2_000 {
            self.poll()?;
        }
        Ok(())
    }
}

/// Runs `test` on every backend; `uring-sqpoll` may refuse typed on a
/// process with one CPU (driver.rs covers that refusal).
fn each_backend(test: impl Fn(Backend) -> R) -> R {
    for b in Backend::KERNEL {
        match test(b) {
            Ok(()) => {}
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {}
            Err(e) => return Err(format!("{b}: {e}").into()),
        }
    }
    Ok(())
}

/// `n` framed connections on `rig`, each with its peer's socket; one frame
/// each way per connection, polled through.
fn frame_conns(rig: &mut Rig<FrameConn>, n: usize) -> R<(Vec<ConnId>, Vec<TcpStream>)> {
    let l = TcpListener::bind("127.0.0.1:0")?;
    let opts = FrameOptions {
        rx_buf: 4_096,
        tx_buf: 4_096,
        max_frame: 1_024,
        ..FrameOptions::default()
    };
    let (mut ids, mut peers) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        let c = TcpStream::connect(l.local_addr()?)?;
        let (p, _) = l.accept()?;
        ids.push(rig.d.add(FrameConn::from_stream(c, opts)?)?);
        peers.push(p);
    }
    warm_frames(rig, &ids, &mut peers)?;
    Ok((ids, peers))
}

fn warm_frames(rig: &mut Rig<FrameConn>, ids: &[ConnId], peers: &mut [TcpStream]) -> R {
    use std::io::{Read, Write};
    let frame = [4u8, 0, 0, 0, 1, 2, 3, 4];
    for round in 0..3u64 {
        for p in peers.iter_mut() {
            p.write_all(&frame)?;
        }
        let want = (round + 1) * ids.len() as u64;
        rig.poll_until("frames", |h| h.msgs >= want)?;
        for &id in ids {
            rig.d
                .get_mut(id)
                .ok_or("closed early")?
                .send_frame(&frame[4..])?;
        }
        rig.poll()?;
        for p in peers.iter_mut() {
            let mut got = [0u8; 8];
            p.read_exact(&mut got)?;
        }
    }
    rig.idle()
}

#[test]
fn a_framed_peer_close_allocates_nothing() -> R {
    each_backend(|b| {
        for n in COUNTS {
            let mut rig = Rig::<FrameConn>::new(b)?;
            let (_ids, peers) = frame_conns(&mut rig, n)?;
            rig.allocs = 0;
            drop(peers);
            rig.poll_until("the closes", |h| h.closed == n as u64)?;
            rig.idle()?;
            assert!(rig.d.is_empty());
            assert_eq!(rig.allocs, 0, "{n} peer closes: {} allocations", rig.allocs);
        }
        Ok(())
    })
}

#[test]
fn closing_framed_connections_allocates_nothing() -> R {
    each_backend(|b| {
        for n in COUNTS {
            let mut rig = Rig::<FrameConn>::new(b)?;
            let (ids, _peers) = frame_conns(&mut rig, n)?;
            rig.allocs = 0;
            for id in ids {
                rig.close(id);
                rig.poll()?;
            }
            rig.idle()?;
            assert!(rig.d.is_empty());
            assert_eq!(rig.allocs, 0, "{n} closes: {} allocations", rig.allocs);
        }
        Ok(())
    })
}

/// With the background connect helper running, closed connections are
/// freed on it: handing them over allocates nothing here either.
#[test]
fn a_peer_close_with_the_connect_helper_running_allocates_nothing() -> R {
    if support::one_cpu()? {
        // No CPU for the helper: `connect_background` refuses (driver.rs).
        return Ok(());
    }
    for n in COUNTS {
        let mut rig = Rig::<FrameConn>::new(Backend::EpollSpin)?;
        let l = TcpListener::bind("127.0.0.1:0")?;
        let addr = l.local_addr()?;
        let target = Target::new("localhost", addr.port(), "").with_addr(addr);
        let bg = rig
            .d
            .connect_background(FrameDial::new(target, FrameOptions::default())?)?;
        let (bg_peer, _) = l.accept()?;
        rig.poll_until("the background connect", |h| h.opened == 1)?;
        let (_ids, peers) = frame_conns(&mut rig, n)?;
        rig.allocs = 0;
        drop(peers);
        rig.poll_until("the closes", |h| h.closed == n as u64)?;
        rig.idle()?;
        assert!(rig.d.get(bg).is_some());
        assert_eq!(rig.allocs, 0, "{n} peer closes: {} allocations", rig.allocs);
        drop(bg_peer);
    }
    Ok(())
}

/// `n` connected UDP pairs on `rig`, one datagram each way, polled through.
fn udp_conns(rig: &mut Rig<UdpConn>, n: usize) -> R<(Vec<ConnId>, Vec<UdpSocket>)> {
    let opts = UdpOptions::default();
    let (mut ids, mut peers) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        let p = UdpSocket::bind("127.0.0.1:0")?;
        let local: SocketAddr = {
            let s = UdpSocket::bind("127.0.0.1:0")?;
            s.local_addr()?
        };
        p.connect(local)?;
        ids.push(
            rig.d
                .add(UdpConn::connect(local, p.local_addr()?, &opts)?)?,
        );
        p.set_read_timeout(Some(WAIT))?;
        peers.push(p);
    }
    for round in 0..3u64 {
        for p in &peers {
            p.send(b"warm")?;
        }
        let want = (round + 1) * n as u64;
        rig.poll_until("datagrams", |h| h.msgs >= want)?;
        for &id in &ids {
            rig.d.get_mut(id).ok_or("closed early")?.send(b"back")?;
        }
        rig.poll()?;
        for p in &peers {
            let mut got = [0u8; 16];
            p.recv(&mut got)?;
        }
    }
    rig.idle()?;
    Ok((ids, peers))
}

#[test]
fn closing_udp_connections_allocates_nothing() -> R {
    // Datagram endpoints run on `epoll-spin` only.
    for n in COUNTS {
        let mut rig = Rig::<UdpConn>::new(Backend::EpollSpin)?;
        let (ids, _peers) = udp_conns(&mut rig, n)?;
        rig.allocs = 0;
        for id in ids {
            rig.close(id);
            rig.poll()?;
        }
        rig.idle()?;
        assert!(rig.d.is_empty());
        assert_eq!(rig.allocs, 0, "{n} closes: {} allocations", rig.allocs);
    }
    Ok(())
}

/// `n` WebSocket connections to the lab, with their server ends; one
/// message each way per connection, polled through.
fn ws_conns(lab: &Lab, rig: &mut Rig<WsConn>, n: usize) -> R<(Vec<ConnId>, Vec<Peer>)> {
    let opts = WsOptions {
        ping_interval_ms: 0,
        buf_size: 256 * 1024,
        ..WsOptions::default()
    };
    let target = lab.target("/")?;
    let mut ids = Vec::with_capacity(n);
    let mut peers = std::thread::scope(|s| -> R<Vec<Peer>> {
        let server = s.spawn(|| -> Result<Vec<Peer>, String> {
            (0..n)
                .map(|_| lab.accept_within(WAIT).map_err(|e| e.to_string()))
                .collect()
        });
        for _ in 0..n {
            ids.push(rig.d.add(WsConn::connect_with(
                &target,
                lab.client_cfg.clone(),
                opts.clone(),
            )?)?);
        }
        Ok(server.join().map_err(|_| "server panicked")??)
    })?;
    for round in 0..3u64 {
        for p in &mut peers {
            p.send(ws::OP_BINARY, true, b"warm")?;
        }
        let want = (round + 1) * n as u64;
        rig.poll_until("messages", |h| h.msgs >= want)?;
        for &id in &ids {
            rig.d
                .get_mut(id)
                .ok_or("closed early")?
                .send_binary(b"back")?;
        }
        rig.poll()?;
        for p in &mut peers {
            p.recv()?;
        }
    }
    rig.idle()?;
    Ok((ids, peers))
}

/// The server's TCP close, with no WebSocket Close first.
#[test]
fn a_websocket_peer_drop_allocates_nothing() -> R {
    each_backend(|b| {
        for n in COUNTS {
            let lab = Lab::new()?;
            let mut rig = Rig::<WsConn>::new(b)?;
            let (_ids, peers) = ws_conns(&lab, &mut rig, n)?;
            rig.allocs = 0;
            drop(peers);
            rig.poll_until("the closes", |h| h.closed == n as u64)?;
            rig.idle()?;
            assert!(rig.d.is_empty());
            assert_eq!(rig.allocs, 0, "{n} peer drops: {} allocations", rig.allocs);
        }
        Ok(())
    })
}

/// The server's Close handshake: Close with a code and a reason, the
/// client's echo, then the server's TCP close; reported as `WsClose`.
#[test]
fn a_websocket_close_handshake_allocates_nothing() -> R {
    each_backend(|b| {
        for n in COUNTS {
            let lab = Lab::new()?;
            let mut rig = Rig::<WsConn>::new(b)?;
            let (ids, mut peers) = ws_conns(&lab, &mut rig, n)?;
            rig.allocs = 0;
            for p in &mut peers {
                p.send_close(Some(1001), "going away for maintenance")?;
            }
            // Each client echoes the Close; the servers read the echoes.
            let end = Instant::now() + WAIT;
            while !ids
                .iter()
                .all(|&id| rig.d.get(id).is_some_and(WsConn::is_closing))
            {
                rig.poll()?;
                if Instant::now() > end {
                    return Err("a Close was never received".into());
                }
            }
            rig.idle()?;
            for p in &mut peers {
                let (op, _) = p.recv()?;
                assert_eq!(op, ws::OP_CLOSE);
            }
            drop(peers);
            rig.poll_until("the closes", |h| h.closed == n as u64)?;
            rig.idle()?;
            assert!(rig.d.is_empty());
            assert_eq!(
                rig.allocs, 0,
                "{n} close handshakes: {} allocations",
                rig.allocs
            );
        }
        Ok(())
    })
}

#[test]
fn closing_websocket_connections_allocates_nothing() -> R {
    each_backend(|b| {
        for n in COUNTS {
            let lab = Lab::new()?;
            let mut rig = Rig::<WsConn>::new(b)?;
            let (ids, _peers) = ws_conns(&lab, &mut rig, n)?;
            rig.allocs = 0;
            for id in ids {
                rig.close(id);
                rig.poll()?;
            }
            rig.idle()?;
            assert!(rig.d.is_empty());
            assert_eq!(rig.allocs, 0, "{n} closes: {} allocations", rig.allocs);
        }
        Ok(())
    })
}
