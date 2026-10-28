//! The `smoltcp-spin` backend end to end: a DPDK `net_af_packet` port on one
//! end of a veth pair, the kernel's TCP/IP stack on the other, and
//! `smoltcp_peer` (another process) serving framed TCP, WebSocket over TLS
//! and UDP echo there.
//!
//! Run by `scripts/test-io-smoltcp-vdev.sh`, which builds the network
//! namespace, the veth pair and the peer, and sets:
//! - `ERGON_SMOLTCP_STACK`: the `StackConfig` string (port, address, EAL
//!   arguments);
//! - `ERGON_SMOLTCP_PEER`: the peer's address;
//! - `ERGON_SMOLTCP_PEER_BIN`: the peer binary;
//! - `ERGON_SMOLTCP_NETEM`: the two veth ends, for the lossy-link test.
//!
//! Without them every test fails: nothing here is skipped.
#![cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
#![allow(
    missing_docs,
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ergon_io::smoltcp_dpdk::{SmolTcp, SmolUdp, Stack, StackConfig};
use ergon_io::{
    Config, ConnId, Driver, Error, FrameConn, FrameDial, FrameHandler, FrameOptions, Handler,
    RxMeta, Target, UdpConn, UdpHandler, UdpOptions, WsConn, WsDial, WsHandler,
};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

// ---- a counting allocator: the steady-state test counts this thread's ----

struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn allocs() -> u64 {
    ALLOCS.with(Cell::get)
}

// SAFETY: forwards to the system allocator; only adds a thread-local counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// ---- the environment -----------------------------------------------------

fn env(k: &str) -> R<String> {
    std::env::var(k).map_err(|_| {
        format!("{k} is not set: run this test through scripts/test-io-smoltcp-vdev.sh").into()
    })
}

fn stack_cfg() -> R<StackConfig> {
    Ok(StackConfig::parse(&env("ERGON_SMOLTCP_STACK")?)?)
}

fn peer_ip() -> R<Ipv4Addr> {
    Ok(env("ERGON_SMOLTCP_PEER")?.parse()?)
}

/// The kernel-side echo process; killed when dropped.
struct PeerProc {
    child: Child,
    cert: std::path::PathBuf,
    out: Arc<std::sync::Mutex<Vec<String>>>,
}

impl PeerProc {
    fn start() -> R<Self> {
        let cert =
            std::env::temp_dir().join(format!("ergon-smoltcp-peer-{}.pem", std::process::id()));
        let mut child = Command::new(env("ERGON_SMOLTCP_PEER_BIN")?)
            .arg(peer_ip()?.to_string())
            .arg(&cert)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().ok_or("no peer stdout")?;
        let mut lines = BufReader::new(stdout).lines();
        match lines.next() {
            Some(Ok(l)) if l == "READY" => {}
            other => return Err(format!("peer did not start: {other:?}").into()),
        }
        let out = Arc::new(std::sync::Mutex::new(Vec::new()));
        let o = Arc::clone(&out);
        std::thread::spawn(move || {
            for l in lines.map_while(Result::ok) {
                if let Ok(mut v) = o.lock() {
                    v.push(l);
                }
            }
        });
        Ok(Self { child, cert, out })
    }

    fn lines(&self) -> Vec<String> {
        self.out.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Waits for the peer's `TCPINFO` line about client port `port`.
    fn tcp_info(&self, port: u16) -> R<String> {
        let end = Instant::now() + Duration::from_secs(5);
        let want = format!("TCPINFO port={port} ");
        loop {
            if let Some(l) = self.lines().into_iter().find(|l| l.starts_with(&want)) {
                return Ok(l);
            }
            if Instant::now() > end {
                return Err(format!("no {want} line from the peer: {:?}", self.lines()).into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for PeerProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.cert);
    }
}

fn driver<E: ergon_io::Endpoint>() -> R<Driver<E>> {
    Ok(Driver::new(Config::parse("smoltcp-spin")?)?)
}

/// Polls `d` until `done` or `secs` pass.
fn pump<E, H>(
    d: &mut Driver<E>,
    h: &mut H,
    secs: u64,
    mut done: impl FnMut(&H, &Driver<E>) -> bool,
) -> R
where
    E: ergon_io::Receive<H>,
    H: Handler,
{
    let end = Instant::now() + Duration::from_secs(secs);
    while !done(h, d) {
        d.poll(h)?;
        if Instant::now() > end {
            return Err(format!("timed out after {secs} s: {:?}", d.stats()).into());
        }
    }
    Ok(())
}

/// p50, p99, p99.9 and max of `v`, µs.
fn summary(v: &mut [u64]) -> String {
    v.sort_unstable();
    let at = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)] as f64 / 1e3;
    format!(
        "n={} p50={:.1}us p99={:.1}us p99.9={:.1}us max={:.1}us",
        v.len(),
        at(0.5),
        at(0.99),
        at(0.999),
        v.last().copied().unwrap_or(0) as f64 / 1e3
    )
}

// ---- framed TCP -----------------------------------------------------------

#[derive(Default)]
struct Frames {
    open: Vec<ConnId>,
    closed: Vec<(ConnId, String)>,
    got: Vec<(ConnId, Vec<u8>)>,
}

impl Handler for Frames {
    fn on_open(&mut self, id: ConnId) {
        self.open.push(id);
    }
    fn on_closed(&mut self, id: ConnId, e: Error) {
        self.closed.push((id, e.to_string()));
    }
}

impl FrameHandler for Frames {
    fn on_frame(&mut self, id: ConnId, payload: &[u8], _: &RxMeta) {
        self.got.push((id, payload.to_vec()));
    }
}

fn frame_dial(port: u16) -> R<FrameDial<SmolTcp>> {
    let addr = SocketAddr::new(peer_ip()?.into(), port);
    Ok(FrameDial::on_stack(
        Target::new("peer", port, "/").with_addr(addr),
        FrameOptions::default(),
    )?)
}

/// Connects `n` framed connections.
fn connect_frames(d: &mut Driver<FrameConn<SmolTcp>>, h: &mut Frames, n: usize) -> R<Vec<ConnId>> {
    let mut ids = Vec::with_capacity(n);
    for _ in 0..n {
        ids.push(d.connect(frame_dial(7001)?)?);
    }
    pump(d, h, 10, |h, _| h.open.len() + h.closed.len() >= n)?;
    if !h.closed.is_empty() {
        return Err(format!("connects failed: {:?}", h.closed).into());
    }
    Ok(ids)
}

/// Sends `payload` on `id` and waits for its echo; the round trip, ns.
fn echo(d: &mut Driver<FrameConn<SmolTcp>>, h: &mut Frames, id: ConnId, payload: &[u8]) -> R<u64> {
    h.got.clear();
    let t0 = Instant::now();
    d.get_mut(id)
        .ok_or("connection gone")?
        .send_frame(payload)?;
    pump(d, h, 10, |h, _| !h.got.is_empty() || !h.closed.is_empty())?;
    let ns = t0.elapsed().as_nanos() as u64;
    if let Some(c) = h.closed.first() {
        return Err(format!("closed during echo: {c:?}").into());
    }
    let (from, got) = h.got.remove(0);
    if from != id || got != payload {
        return Err(format!(
            "echo mismatch: {} bytes back, {} sent",
            got.len(),
            payload.len()
        )
        .into());
    }
    Ok(ns)
}

#[test]
fn framed_echo_over_one_connection() -> R {
    let _peer = PeerProc::start()?;
    let stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let t0 = Instant::now();
    let id = connect_frames(&mut d, &mut h, 1)?[0];
    println!("connect: {:.1} us", t0.elapsed().as_nanos() as f64 / 1e3);
    // Sizes across one MSS, several, and the frame limit (64 KiB).
    for n in [0, 1, 100, 1_400, 1_460, 1_500, 4_000, 16_384, 65_536] {
        let p: Vec<u8> = (0..n).map(|i: usize| (i * 7 % 251) as u8).collect();
        echo(&mut d, &mut h, id, &p)?;
    }
    let mut rtt = Vec::new();
    for i in 0..5_000u32 {
        rtt.push(echo(&mut d, &mut h, id, &i.to_le_bytes().repeat(16))?);
    }
    println!("framed 64 B echo over smoltcp-spin: {}", summary(&mut rtt));
    let s = stack.stats();
    println!("stack: {s:?}");
    assert_eq!(s.tcp_retransmits, 0, "retransmissions on a clean link");
    assert_eq!((s.rx_chained, s.tx_no_mbuf, s.tx_ring_full), (0, 0, 0));
    let fd = d.get(id).ok_or("gone")?.raw_fd();
    let t = stack.tcp_stats(fd).ok_or("no tcp stats")?;
    assert_eq!(t.state, "ESTABLISHED");
    Ok(())
}

#[test]
fn framed_echo_over_eight_connections() -> R {
    let _peer = PeerProc::start()?;
    let _stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let ids = connect_frames(&mut d, &mut h, 8)?;
    // All eight in flight at once, each with its own payload.
    for round in 0..500u32 {
        h.got.clear();
        for (k, &id) in ids.iter().enumerate() {
            let p = [round.to_le_bytes(), (k as u32).to_le_bytes()].concat();
            d.get_mut(id).ok_or("gone")?.send_frame(&p)?;
        }
        pump(&mut d, &mut h, 10, |h, _| {
            h.got.len() == ids.len() || !h.closed.is_empty()
        })?;
        for (from, p) in &h.got {
            let k = ids.iter().position(|i| i == from).ok_or("unknown id")?;
            assert_eq!(p, &[round.to_le_bytes(), (k as u32).to_le_bytes()].concat());
        }
    }
    assert!(h.closed.is_empty(), "{:?}", h.closed);
    Ok(())
}

// ---- WebSocket over TLS ---------------------------------------------------

#[derive(Default)]
struct Ws {
    open: Vec<ConnId>,
    closed: Vec<(ConnId, String)>,
    got: Vec<(u8, Vec<u8>)>,
}

impl Handler for Ws {
    fn on_open(&mut self, id: ConnId) {
        self.open.push(id);
    }
    fn on_closed(&mut self, id: ConnId, e: Error) {
        self.closed.push((id, e.to_string()));
    }
}

impl WsHandler for Ws {
    fn on_message(&mut self, _: ConnId, opcode: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.got.push((opcode, payload.to_vec()));
    }
}

fn tls_config(cert: &std::path::Path) -> R<Arc<rustls::ClientConfig>> {
    let pem = std::fs::read(cert)?;
    let mut roots = rustls::RootCertStore::empty();
    for c in rustls_pki_pem(&pem)? {
        roots.add(c)?;
    }
    Ok(ergon_io::tls::client_config(roots)?)
}

/// The certificates of a PEM file (DER).
fn rustls_pki_pem(pem: &[u8]) -> R<Vec<rustls::pki_types::CertificateDer<'static>>> {
    use rustls::pki_types::pem::PemObject;
    Ok(rustls::pki_types::CertificateDer::pem_slice_iter(pem).collect::<Result<Vec<_>, _>>()?)
}

#[test]
fn websocket_over_tls_echo() -> R {
    let peer = PeerProc::start()?;
    let stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<WsConn<SmolTcp>> = driver()?;
    let mut h = Ws::default();
    let addr = SocketAddr::new(peer_ip()?.into(), 7443);
    let dial = WsDial::<SmolTcp>::on_stack(
        Target::new("localhost", 7443, "/ws").with_addr(addr),
        tls_config(&peer.cert)?,
        ergon_io::WsOptions {
            buf_size: 1 << 20,
            ..ergon_io::WsOptions::default()
        },
    )?;
    let t0 = Instant::now();
    let id = d.connect(dial)?;
    pump(&mut d, &mut h, 10, |h, _| {
        !h.open.is_empty() || !h.closed.is_empty()
    })?;
    assert!(h.closed.is_empty(), "{:?}", h.closed);
    println!(
        "TCP + TLS 1.3 + upgrade: {:.1} us (longest step {} ns)",
        t0.elapsed().as_nanos() as f64 / 1e3,
        d.stats().connect_step_max_ns
    );
    let mut rtt = Vec::new();
    for (i, n) in [1usize, 64, 125, 126, 1_000, 4_000, 16_000, 65_000, 200_000]
        .into_iter()
        .cycle()
        .take(900)
        .enumerate()
    {
        let p: Vec<u8> = (0..n).map(|k| b'a' + ((k + i) % 26) as u8).collect();
        h.got.clear();
        let t = Instant::now();
        if i % 2 == 0 {
            d.get_mut(id)
                .ok_or("gone")?
                .send_text(std::str::from_utf8(&p)?)?;
        } else {
            d.get_mut(id).ok_or("gone")?.send_binary(&p)?;
        }
        pump(&mut d, &mut h, 10, |h, _| {
            !h.got.is_empty() || !h.closed.is_empty()
        })?;
        if n <= 125 {
            rtt.push(t.elapsed().as_nanos() as u64);
        }
        assert!(h.closed.is_empty(), "{:?}", h.closed);
        let (op, back) = h.got.remove(0);
        assert_eq!(
            op,
            if i % 2 == 0 {
                ergon_io::ws::OP_TEXT
            } else {
                ergon_io::ws::OP_BINARY
            }
        );
        assert_eq!(back, p, "message {i} of {n} bytes");
    }
    println!(
        "WebSocket/TLS <= 125 B echo over smoltcp-spin: {}",
        summary(&mut rtt)
    );
    // The close handshake.
    d.get_mut(id).ok_or("gone")?.close(1000, "done")?;
    pump(&mut d, &mut h, 10, |h, _| !h.closed.is_empty())?;
    println!("closed: {:?}", h.closed);
    println!("stack: {:?}", stack.stats());
    Ok(())
}

// ---- UDP -------------------------------------------------------------------

#[derive(Default)]
struct Dgrams {
    got: Vec<Vec<u8>>,
    closed: Vec<String>,
}

impl Handler for Dgrams {
    fn on_closed(&mut self, _: ConnId, e: Error) {
        self.closed.push(e.to_string());
    }
}

impl UdpHandler for Dgrams {
    fn on_datagram(&mut self, _: ConnId, payload: &[u8], _: &RxMeta) {
        self.got.push(payload.to_vec());
    }
}

#[test]
fn udp_echo() -> R {
    let _peer = PeerProc::start()?;
    let stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<UdpConn<SmolUdp>> = driver()?;
    let mut h = Dgrams::default();
    let peer = SocketAddrV4::new(peer_ip()?, 7002);
    let conn = stack.udp_connect(0, peer, &UdpOptions::default())?;
    let local = conn.local_addr()?;
    assert_eq!(conn.peer_addr(), Some(SocketAddr::V4(peer)));
    let id = d.add(conn)?;
    let mut rtt = Vec::new();
    for i in 0..2_000u32 {
        let p = i.to_le_bytes().repeat(1 + (i as usize % 300));
        h.got.clear();
        let t = Instant::now();
        d.get_mut(id).ok_or("gone")?.send(&p)?;
        pump(&mut d, &mut h, 5, |h, _| {
            !h.got.is_empty() || !h.closed.is_empty()
        })?;
        rtt.push(t.elapsed().as_nanos() as u64);
        assert_eq!(h.got[0], p);
    }
    println!(
        "UDP echo from {local} over smoltcp-spin: {}",
        summary(&mut rtt)
    );
    let s = d.get(id).ok_or("gone")?.stats();
    assert_eq!((s.tx, s.rx), (2_000, 2_000));
    Ok(())
}

// ---- lifecycle ---------------------------------------------------------------

#[test]
fn close_reconnect_and_peer_close() -> R {
    let _peer = PeerProc::start()?;
    let stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    for round in 0..30 {
        h.open.clear();
        let id = connect_frames(&mut d, &mut h, 1)?[0];
        echo(&mut d, &mut h, id, format!("round {round}").as_bytes())?;
        d.close(id);
        assert!(d.get(id).is_none());
    }
    // The closed connections finish (FIN exchange, TIME-WAIT) and free
    // their sockets and ports.
    let end = Instant::now() + Duration::from_secs(5);
    while stack.stats().sockets_closing > 0 || stack.stats().sockets_open > 0 {
        d.poll(&mut h)?;
        assert!(Instant::now() < end, "sockets left: {:?}", stack.stats());
    }
    // The peer closes: on_closed with Error::Closed.
    h.open.clear();
    let id = connect_frames(&mut d, &mut h, 1)?[0];
    d.get_mut(id).ok_or("gone")?.send_frame(b"!close")?;
    pump(&mut d, &mut h, 5, |h, _| !h.closed.is_empty())?;
    assert_eq!(h.closed[0].0, id);
    assert!(h.closed[0].1.contains("closed"), "{:?}", h.closed);
    println!("stack after 31 connections: {:?}", stack.stats());
    Ok(())
}

#[test]
fn a_refused_connect_fails_at_once() -> R {
    let _peer = PeerProc::start()?;
    let _stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let t0 = Instant::now();
    d.connect(frame_dial(7999)?)?;
    pump(&mut d, &mut h, 5, |h, _| {
        !h.closed.is_empty() || !h.open.is_empty()
    })?;
    let took = t0.elapsed();
    assert_eq!(h.open, [] as [ConnId; 0]);
    let why = &h.closed[0].1;
    println!(
        "refused after {:.1} us: {why}",
        took.as_nanos() as f64 / 1e3
    );
    assert!(why.contains("refused"), "{why}");
    assert!(
        took < Duration::from_millis(500),
        "{took:?}: a refusal is an RST, not a timeout"
    );
    assert_eq!(d.stats().connect_failures, 1);
    Ok(())
}

#[test]
fn a_driver_refuses_the_other_stacks_endpoints() -> R {
    let _stack = Stack::init(&stack_cfg()?)?;
    // Endpoint types carry their stack: a kernel driver of smoltcp sockets,
    // or a smoltcp driver of kernel ones, does not start.
    let e = Driver::<UdpConn<SmolUdp>>::new(Config::parse("epoll-spin")?)
        .err()
        .ok_or("a kernel driver of smoltcp sockets")?;
    assert!(e.to_string().contains("smoltcp's"), "{e}");
    let e = Driver::<FrameConn>::new(Config::parse("smoltcp-spin")?)
        .err()
        .ok_or("a smoltcp driver of kernel sockets")?;
    assert!(e.to_string().contains("the kernel's"), "{e}");
    Ok(())
}

#[test]
fn a_pinned_gateway_mac_needs_no_arp() -> R {
    let _peer = PeerProc::start()?;
    // The peer as gateway, its MAC pinned: smoltcp learns it from the
    // stack's own answer, and no ARP request leaves for it.
    let mac = env("ERGON_SMOLTCP_PEER_MAC")?;
    let cfg = StackConfig::parse(&format!(
        "gw={},gw_mac={mac},{}",
        peer_ip()?,
        env("ERGON_SMOLTCP_STACK")?
    ))?;
    let stack = Stack::init(&cfg)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let id = connect_frames(&mut d, &mut h, 1)?[0];
    // Long enough for several refreshes (every 200 ms at the default floor).
    let end = Instant::now() + Duration::from_millis(700);
    while Instant::now() < end {
        echo(&mut d, &mut h, id, b"pinned")?;
    }
    let s = stack.stats();
    println!("pinned gateway: {s:?}");
    assert_eq!(s.arp_refreshes, 0, "an ARP request for a pinned neighbour");
    assert!(s.arp_pinned >= 3, "{s:?}");
    Ok(())
}

#[test]
fn a_learnt_neighbour_is_refreshed_before_it_expires() -> R {
    let _peer = PeerProc::start()?;
    let stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let id = connect_frames(&mut d, &mut h, 1)?[0];
    // Past two neighbour lifetimes (600 ms each at the default floor): the
    // entry never lapses, so no round trip waits for ARP.
    let mut rtt = Vec::new();
    let end = Instant::now() + Duration::from_millis(1_500);
    while Instant::now() < end {
        rtt.push(echo(&mut d, &mut h, id, b"arp")?);
    }
    let s = stack.stats();
    println!("learnt neighbour: {}; {s:?}", summary(&mut rtt));
    assert!(s.arp_refreshes >= 4, "{s:?}");
    Ok(())
}

// ---- loss ------------------------------------------------------------------

/// `tc qdisc` on both veth ends (`ERGON_SMOLTCP_NETEM`): `args` (`netem
/// loss 1%`), or none when empty.
fn netem(args: &str) -> R {
    for dev in env("ERGON_SMOLTCP_NETEM")?.split_whitespace() {
        // Nothing to delete is not an error here.
        let _ = Command::new("tc")
            .args(["qdisc", "del", "dev", dev, "root"])
            .stderr(Stdio::null())
            .status();
        if !args.is_empty() {
            let mut c = Command::new("tc");
            c.args(["qdisc", "add", "dev", dev, "root"])
                .args(args.split_whitespace());
            let st = c.status()?;
            if !st.success() {
                return Err(format!("tc qdisc add dev {dev} root {args}: {st}").into());
            }
        }
    }
    Ok(())
}

struct NetemOff;
impl Drop for NetemOff {
    fn drop(&mut self) {
        let _ = netem("");
    }
}

/// `n` 100-byte echoes over one connection on a stack with `cfg`: the
/// latencies, the stack's retransmissions and the peer's `TCP_INFO` line.
fn lossy_run(cfg: &StackConfig, peer: &PeerProc, n: u32) -> R<(Vec<u64>, u64, String)> {
    let stack = Stack::init(cfg)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let id = connect_frames(&mut d, &mut h, 1)?[0];
    let fd = d.get(id).ok_or("gone")?.raw_fd();
    let port = stack.tcp_stats(fd).ok_or("no stats")?.local_port;
    let mut rtt = Vec::new();
    for i in 0..n {
        rtt.push(echo(&mut d, &mut h, id, &i.to_le_bytes().repeat(25))?);
    }
    let retx = stack.tcp_stats(fd).ok_or("no stats")?.retransmits;
    d.get_mut(id).ok_or("gone")?.send_frame(b"!close")?;
    pump(&mut d, &mut h, 30, |h, _| !h.closed.is_empty())?;
    let info = peer.tcp_info(port)?;
    Ok((rtt, retx, info))
}

#[test]
fn a_lossy_link_recovers_within_the_rto_floor() -> R {
    let peer = PeerProc::start()?;
    let _off = NetemOff;
    netem("netem loss 2%")?;
    let cfg = stack_cfg()?;
    let (mut rtt, retx, info) = lossy_run(&cfg, &peer, 3_000)?;
    let max_ms = rtt.iter().max().copied().unwrap_or(0) as f64 / 1e6;
    println!(
        "2% loss each way, rto_min_us={}: {}; client retransmissions {retx}; peer {info}",
        cfg.rto_min_us,
        summary(&mut rtt)
    );
    let slow = rtt.iter().filter(|&&ns| ns > 5_000_000).count();
    println!(
        "  round trips over 5 ms: {slow} ({:.2}%)",
        slow as f64 * 100.0 / rtt.len() as f64
    );
    assert!(retx > 0, "2% loss and no client retransmission");
    // Every echo completed; the worst stall is a few RTO floors (a loss on
    // a retransmission doubles it), far under smoltcp's own 1 s.
    assert!(max_ms < 400.0, "worst round trip {max_ms:.1} ms");

    // smoltcp's own floor (1 s), for comparison: a lost segment with no
    // later one to trigger a fast retransmit waits a whole second.
    let native = StackConfig {
        rto_min_us: 1_000_000,
        ..cfg
    };
    let (mut rtt, retx, info) = lossy_run(&native, &peer, 500)?;
    let max_ms = rtt.iter().max().copied().unwrap_or(0) as f64 / 1e6;
    println!(
        "2% loss each way, rto_min_us=1000000 (smoltcp's own): {}; client retransmissions {retx}; peer {info}",
        summary(&mut rtt)
    );
    assert!(
        max_ms >= 900.0 || retx == 0,
        "a client retransmission waited only {max_ms:.1} ms at a 1 s floor"
    );
    Ok(())
}

// ---- steady state allocates nothing -----------------------------------------

#[test]
fn the_steady_state_allocates_nothing() -> R {
    let _peer = PeerProc::start()?;
    let stack = Stack::init(&stack_cfg()?)?;
    let mut d: Driver<FrameConn<SmolTcp>> = driver()?;
    let mut h = Frames::default();
    let id = connect_frames(&mut d, &mut h, 1)?[0];
    let payload = [7u8; 200];
    let mut got = 0u64;
    let mut on = |d: &mut Driver<FrameConn<SmolTcp>>, n: u32| -> R {
        for _ in 0..n {
            d.get_mut(id).ok_or("gone")?.send_frame(&payload)?;
            let before = got;
            let end = Instant::now() + Duration::from_secs(5);
            while got == before {
                d.poll(&mut Counter(&mut got))?;
                if Instant::now() > end {
                    return Err("no echo".into());
                }
            }
        }
        Ok(())
    };
    // Warm: buffers grown, ARP learnt, every path taken once.
    on(&mut d, 500)?;
    let a0 = allocs();
    on(&mut d, 5_000)?;
    let used = allocs() - a0;
    println!(
        "allocations over 5000 round trips: {used}; stack {:?}",
        stack.stats()
    );
    assert_eq!(used, 0, "the polling thread allocated in steady state");
    drop(h);
    Ok(())
}

/// Counts frames without allocating.
struct Counter<'a>(&'a mut u64);
impl Handler for Counter<'_> {}
impl FrameHandler for Counter<'_> {
    fn on_frame(&mut self, _: ConnId, _: &[u8], _: &RxMeta) {
        *self.0 += 1;
    }
}
