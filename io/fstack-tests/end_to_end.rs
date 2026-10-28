//! `fstack-spin` end to end: two processes, each with its own F-Stack on one
//! end of a veth pair (DPDK's `net_af_packet` PMD, no hugepages), as
//! `io/fstack-tests/docker-test.sh` runs them. One is the peer (an echo
//! server on F-Stack's own sockets: framed TCP, WebSocket over TLS, HTTP
//! over TLS), the other drives ergon-io's endpoints over F-Stack:
//!
//! - framed TCP: `Driver::connect` and `connect_blocking` + `Driver::add`,
//!   echo, the peer's close, `Driver::close` and a reconnect, a refused
//!   connect, `TCP_INFO`, no allocation once warm, and an echo-latency
//!   smoke (meaningless under emulation: it shows the path works);
//! - WebSocket over TLS: connect, echo, a ping, the close handshake, no
//!   allocation once warm, the latency smoke;
//! - HTTP over TLS: requests and their responses.
//!
//! The role comes from `FSTACK_ROLE` (`server` or `client`); without it,
//! in a build with F-Stack linked, the test fails: run the script.
//! Only with F-Stack linked (`FSTACK_PREFIX` at build time).
#![cfg(all(target_os = "linux", feature = "fstack", ergon_fstack_linked))]
#![allow(
    missing_docs,
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ergon_io::fstack::{FstackListener, FstackTcp};
use ergon_io::http::{HttpConn, HttpDial, HttpHandler, HttpOptions, Response};
use ergon_io::{
    Config, ConnId, Driver, Error, FrameConn, FrameDial, FrameHandler, FrameOptions, Handler,
    RxMeta, Target, WsConn, WsDial, WsHandler, WsOptions, tls, ws,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{RootCertStore, ServerConfig, ServerConnection};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Counting;

thread_local! {
    /// This thread's allocations (F-Stack's own are C's, not counted).
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

const SERVER: Ipv4Addr = Ipv4Addr::new(10, 9, 0, 2);
const ECHO: u16 = 7000;
const WS: u16 = 7001;
const HTTP: u16 = 7002;
const CTL: u16 = 7009;
/// Nothing listens here: a connect is refused (RST; `blackhole` is off).
const CLOSED: u16 = 7999;
/// A frame with this payload makes the echo peer close after echoing it.
const BYE: &[u8] = b"bye";
/// Round trips per latency smoke, after as many to warm up.
const TRIPS: usize = 2_000;

fn env(k: &str) -> R<String> {
    std::env::var(k).map_err(|_| {
        format!("{k} unset: io/fstack-tests/docker-test.sh runs this test (two F-Stack processes)")
            .into()
    })
}

/// This process's F-Stack: `FSTACK_ADDR` on `FSTACK_IFACE`, lcore
/// `FSTACK_LCORE`.
fn config() -> R<Config> {
    let s = format!(
        "fstack-spin,loop_cpu={},fstack_addr={},fstack_prefix_len=24,fstack_gateway=10.9.0.254,\
         fstack_vdev=net_af_packet0;iface={},fstack_no_huge=true,fstack_memory_mb=512,\
         fstack_file_prefix={},fstack_sysctl=net.inet.tcp.blackhole=0,tcp_info_interval_ms=10",
        env("FSTACK_LCORE")?,
        env("FSTACK_ADDR")?,
        env("FSTACK_IFACE")?,
        env("FSTACK_ROLE")?,
    );
    Ok(Config::parse(&s)?)
}

fn cert_path() -> R<std::path::PathBuf> {
    Ok(std::path::PathBuf::from(env("FSTACK_SHARE")?).join("cert.der"))
}

#[test]
fn fstack_end_to_end() -> R {
    match env("FSTACK_ROLE")?.as_str() {
        "server" => server(),
        "client" => client(),
        r => Err(format!("FSTACK_ROLE {r}: server or client").into()),
    }
}

// ----- the peer: echo servers on F-Stack's own sockets -----

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Echo,
    Ws,
    Http,
    Ctl,
}

/// One accepted connection.
struct Peer {
    kind: Kind,
    sock: FstackTcp,
    tls: Option<ServerConnection>,
    /// Plaintext received, not yet handled.
    plain: Vec<u8>,
    /// Plain-TCP bytes to write back (the echo).
    out: Vec<u8>,
    upgraded: bool,
    /// Close once `out` and TLS are flushed.
    closing: bool,
}

impl Peer {
    /// Reads, answers and writes what it can; `Ok(false)` once it is done.
    fn step(&mut self, stop: &mut bool) -> R<bool> {
        let eof = match &mut self.tls {
            None => read_all(&mut self.sock, &mut self.plain)?,
            Some(t) => {
                let mut eof = false;
                loop {
                    match t.read_tls(&mut self.sock) {
                        Ok(0) => {
                            eof = true;
                            break;
                        }
                        Ok(_) => {
                            t.process_new_packets()?;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                            eof = true;
                            break;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                let mut b = [0u8; 16_384];
                loop {
                    match t.reader().read(&mut b) {
                        Ok(0) => break,
                        Ok(n) => self.plain.extend_from_slice(&b[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        // A client that closes its socket without a
                        // close_notify (`Driver::close`).
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            eof = true;
                            break;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                eof
            }
        };
        self.answer(stop)?;
        self.flush()?;
        let flushed = self.out.is_empty() && !self.tls.as_ref().is_some_and(|t| t.wants_write());
        Ok(!(eof || (self.closing && flushed)))
    }

    fn answer(&mut self, stop: &mut bool) -> R {
        match self.kind {
            Kind::Ctl => {
                if !self.plain.is_empty() {
                    *stop = true;
                }
            }
            Kind::Echo => {
                // Echo whole frames; one carrying BYE closes after it.
                while self.plain.len() >= 4 {
                    let len = u32::from_le_bytes([
                        self.plain[0],
                        self.plain[1],
                        self.plain[2],
                        self.plain[3],
                    ]) as usize;
                    if self.plain.len() < 4 + len {
                        break;
                    }
                    if &self.plain[4..4 + len] == BYE {
                        self.closing = true;
                    }
                    self.out.extend_from_slice(&self.plain[..4 + len]);
                    self.plain.drain(..4 + len);
                }
            }
            Kind::Ws => self.answer_ws()?,
            Kind::Http => self.answer_http()?,
        }
        Ok(())
    }

    fn tls_write(&mut self, data: &[u8]) -> R {
        let t = self.tls.as_mut().ok_or("no TLS")?;
        t.writer().write_all(data)?;
        Ok(())
    }

    fn answer_ws(&mut self) -> R {
        if !self.upgraded {
            let Some((n, key, _)) = ws::parse_server_request(&self.plain)? else {
                return Ok(());
            };
            self.plain.drain(..n);
            self.tls_write(&ws::server_response(&key))?;
            self.upgraded = true;
        }
        while let Some(h) = ws::parse_header(&self.plain)? {
            if self.plain.len() < h.frame_len() {
                break;
            }
            let mut body = self.plain[h.header_len..h.frame_len()].to_vec();
            ws::apply_mask(&mut body, h.mask.ok_or("client frame not masked")?);
            self.plain.drain(..h.frame_len());
            let reply = match h.opcode {
                ws::OP_PING => ws::OP_PONG,
                ws::OP_CLOSE => {
                    self.closing = true;
                    ws::OP_CLOSE
                }
                ws::OP_PONG => continue,
                op => op,
            };
            let mut out = vec![0u8; ws::header_len(body.len(), false) + body.len()];
            let n = ws::encode_frame(reply, &body, None, &mut out)?;
            self.tls_write(&out[..n])?;
        }
        Ok(())
    }

    fn answer_http(&mut self) -> R {
        loop {
            let mut hdrs = [httparse::EMPTY_HEADER; 32];
            let mut req = httparse::Request::new(&mut hdrs);
            let httparse::Status::Complete(n) = req.parse(&self.plain)? else {
                return Ok(());
            };
            let body = req.path.unwrap_or_default().to_owned();
            self.plain.drain(..n);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            self.tls_write(resp.as_bytes())?;
        }
    }

    fn flush(&mut self) -> R {
        if let Some(t) = &mut self.tls {
            while t.wants_write() {
                match t.write_tls(&mut self.sock) {
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e.into()),
                }
            }
            if self.closing && !t.wants_write() {
                t.send_close_notify();
            }
        }
        while !self.out.is_empty() {
            match self.sock.write(&self.out) {
                Ok(n) => {
                    self.out.drain(..n);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}

/// Reads everything queued into `into`; `true` at the peer's end.
fn read_all(sock: &mut FstackTcp, into: &mut Vec<u8>) -> R<bool> {
    let mut b = [0u8; 16_384];
    loop {
        match sock.read(&mut b) {
            Ok(0) => return Ok(true),
            Ok(n) => into.extend_from_slice(&b[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => return Ok(true),
            Err(e) => return Err(e.into()),
        }
    }
}

fn server() -> R {
    let cfg = config()?;
    let started = ergon_io::fstack::init(&cfg)?;
    println!("fstack-test: server F-Stack on lcore {}", started.lcore());
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let cert = CertificateDer::from(ck.cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server_cfg = Arc::new(
        ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone()], key)?,
    );
    std::fs::write(cert_path()?, cert.as_ref())?;
    let listeners = [
        (
            Kind::Echo,
            FstackListener::bind(SocketAddrV4::new(SERVER, ECHO))?,
        ),
        (
            Kind::Ws,
            FstackListener::bind(SocketAddrV4::new(SERVER, WS))?,
        ),
        (
            Kind::Http,
            FstackListener::bind(SocketAddrV4::new(SERVER, HTTP))?,
        ),
        (
            Kind::Ctl,
            FstackListener::bind(SocketAddrV4::new(SERVER, CTL))?,
        ),
    ];
    println!("fstack-test: ready");
    let mut peers: Vec<Peer> = Vec::new();
    let mut stop = false;
    let mut accepted = 0;
    let deadline = Instant::now() + Duration::from_secs(600);
    while !stop {
        ergon_io::fstack::run_once();
        for (kind, l) in &listeners {
            while let Some(sock) = l.accept()? {
                accepted += 1;
                let tls = match kind {
                    Kind::Ws | Kind::Http => Some(ServerConnection::new(Arc::clone(&server_cfg))?),
                    Kind::Echo | Kind::Ctl => None,
                };
                peers.push(Peer {
                    kind: *kind,
                    sock,
                    tls,
                    plain: Vec::new(),
                    out: Vec::new(),
                    upgraded: false,
                    closing: false,
                });
            }
        }
        let mut k = 0;
        while k < peers.len() {
            if peers[k].step(&mut stop)? {
                k += 1;
            } else {
                // Dropped: `ff_close`.
                peers.swap_remove(k);
            }
        }
        if Instant::now() > deadline {
            return Err("server: no stop within 600 s".into());
        }
    }
    println!("fstack-test: server stopped after {accepted} connections");
    Ok(())
}

// ----- the client: ergon-io's endpoints over F-Stack -----

#[derive(Default)]
struct Events {
    opened: Vec<ConnId>,
    closed: Vec<(ConnId, Error)>,
    frames: u64,
    last: Vec<u8>,
    responses: Vec<(u16, Vec<u8>)>,
}

impl Handler for Events {
    fn on_open(&mut self, id: ConnId) {
        self.opened.push(id);
    }
    fn on_closed(&mut self, id: ConnId, error: Error) {
        self.closed.push((id, error));
    }
}

impl FrameHandler for Events {
    fn on_frame(&mut self, _: ConnId, payload: &[u8], _: &RxMeta) {
        self.frames += 1;
        // No allocation once warm: `last` keeps its capacity.
        self.last.clear();
        self.last.extend_from_slice(payload);
    }
}

impl WsHandler for Events {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.frames += 1;
        self.last.clear();
        self.last.extend_from_slice(payload);
    }
}

impl HttpHandler for Events {
    fn on_response(&mut self, _: ConnId, _: u64, r: &Response<'_>, _: &RxMeta) {
        self.responses.push((r.status, r.body.to_vec()));
    }
}

fn target(port: u16, path: &str) -> Target {
    Target::new("localhost", port, path).with_addr(SocketAddr::V4(SocketAddrV4::new(SERVER, port)))
}

/// Polls until `done`, for at most `limit`.
fn poll_until<E, H>(
    d: &mut Driver<E>,
    h: &mut H,
    limit: Duration,
    what: &str,
    done: impl Fn(&H) -> bool,
) -> R
where
    E: ergon_io::Receive<H>,
    H: Handler,
{
    let end = Instant::now() + limit;
    while !done(h) {
        d.poll(h)?;
        if Instant::now() > end {
            return Err(format!("{what}: not within {limit:?}").into());
        }
    }
    Ok(())
}

/// p50, p99 and max of `ns`, µs.
fn summary(ns: &mut [u64]) -> String {
    ns.sort_unstable();
    let at = |q: f64| ns[((ns.len() as f64 - 1.0) * q) as usize] as f64 / 1000.0;
    format!(
        "n {} p50 {:.1} us p99 {:.1} us max {:.1} us",
        ns.len(),
        at(0.5),
        at(0.99),
        at(1.0)
    )
}

fn client() -> R {
    let cfg = config()?;
    // Driver::new starts F-Stack on this thread.
    let mut fd: Driver<FrameConn<FstackTcp>> = Driver::new(cfg.clone())?;
    let started = ergon_io::fstack::initialised().ok_or("F-Stack not started")?;
    assert!(started.conf().contains("pkt_tx_delay=0") && started.conf().contains("delayed_ack=0"));
    println!("fstack-test: client F-Stack on lcore {}", started.lcore());
    frames(&mut fd)?;
    websocket(&cfg)?;
    http(&cfg)?;
    // Stop the peer.
    let mut ctl = FrameDial::<FstackTcp>::on_stack(target(CTL, "/"), FrameOptions::default())?
        .connect_blocking()?;
    ctl.send_frame(b"stop")?;
    let id = fd.add(ctl)?;
    fd.flush()?;
    for _ in 0..1000 {
        fd.poll(&mut Events::default())?;
    }
    fd.close(id);
    println!("fstack-test: client ok");
    Ok(())
}

fn frames(d: &mut Driver<FrameConn<FstackTcp>>) -> R {
    let mut h = Events::default();
    h.last.reserve(64);
    // A connect through the driver (non-blocking, advanced by poll).
    let a = d.connect(FrameDial::<FstackTcp>::on_stack(
        target(ECHO, "/"),
        FrameOptions::default(),
    )?)?;
    poll_until(d, &mut h, Duration::from_secs(30), "frame connect", |h| {
        !h.opened.is_empty()
    })?;
    assert_eq!(h.opened, [a]);
    // And one made on this thread, then added.
    let c = FrameDial::<FstackTcp>::on_stack(target(ECHO, "/"), FrameOptions::default())?
        .connect_blocking()?;
    let b = d.add(c)?;
    for id in [a, b] {
        let before = h.frames;
        d.get_mut(id)
            .ok_or("gone")?
            .send_frame(b"hello over F-Stack")?;
        poll_until(d, &mut h, Duration::from_secs(10), "frame echo", |h| {
            h.frames > before
        })?;
        assert_eq!(h.last, b"hello over F-Stack");
    }

    // Echo latency, and no allocation once warm.
    let msg = [7u8; 64];
    let mut rtt = Vec::with_capacity(TRIPS);
    let mut allocs_after_warmup = 0;
    for i in 0..2 * TRIPS {
        if i == TRIPS {
            allocs_after_warmup = allocs();
        }
        let before = h.frames;
        let t0 = Instant::now();
        d.get_mut(a).ok_or("gone")?.send_frame(&msg)?;
        while h.frames == before {
            d.poll(&mut h)?;
            if t0.elapsed() > Duration::from_secs(10) {
                return Err(format!("echo {i} lost").into());
            }
        }
        if i >= TRIPS {
            rtt.push(t0.elapsed().as_nanos() as u64);
        }
    }
    let grew = allocs() - allocs_after_warmup;
    // `rtt` was sized up front; nothing else may allocate.
    assert_eq!(
        grew, 0,
        "{grew} allocations in {TRIPS} warm framed round trips"
    );
    println!(
        "fstack-test: framed echo over F-Stack: {}",
        summary(&mut rtt)
    );

    // The peer closes (after echoing BYE): Error::Closed, the other stays.
    d.get_mut(b).ok_or("gone")?.send_frame(BYE)?;
    poll_until(d, &mut h, Duration::from_secs(10), "peer close", |h| {
        !h.closed.is_empty()
    })?;
    let (id, e) = h.closed.pop().ok_or("no close")?;
    assert_eq!(id, b);
    assert!(matches!(e, Error::Closed), "{e}");
    assert_eq!(d.len(), 1);

    // Closing from this side, then a reconnect that echoes.
    d.close(a);
    assert!(d.is_empty());
    h.opened.clear();
    let c = d.connect(FrameDial::<FstackTcp>::on_stack(
        target(ECHO, "/"),
        FrameOptions::default(),
    )?)?;
    poll_until(d, &mut h, Duration::from_secs(30), "reconnect", |h| {
        !h.opened.is_empty()
    })?;
    let before = h.frames;
    d.get_mut(c).ok_or("gone")?.send_frame(b"again")?;
    poll_until(
        d,
        &mut h,
        Duration::from_secs(10),
        "echo after reconnect",
        |h| h.frames > before,
    )?;
    assert_eq!(h.last, b"again");

    // A refused connect: the peer's RST, mapped from FreeBSD's errno.
    let r = d.connect(FrameDial::<FstackTcp>::on_stack(
        target(CLOSED, "/"),
        FrameOptions::default(),
    )?)?;
    poll_until(d, &mut h, Duration::from_secs(30), "refused connect", |h| {
        !h.closed.is_empty()
    })?;
    let (id, e) = h.closed.pop().ok_or("no close")?;
    assert_eq!(id, r);
    match &e {
        Error::Io(io) => {
            assert_eq!(io.raw_os_error(), Some(libc::ECONNREFUSED), "{io}");
            assert_eq!(io.kind(), std::io::ErrorKind::ConnectionRefused);
        }
        e => return Err(format!("refused connect: {e}").into()),
    }
    assert_eq!(d.stats().connect_failures, 1);
    d.close(c);
    println!("fstack-test: framed TCP ok ({} frames)", h.frames);
    Ok(())
}

fn websocket(cfg: &Config) -> R {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(std::fs::read(cert_path()?)?))?;
    let tls_cfg = tls::client_config(roots)?;
    let mut d: Driver<WsConn<FstackTcp>> = Driver::new(cfg.clone())?;
    let mut h = Events::default();
    h.last.reserve(256);
    let a = d.connect(WsDial::<FstackTcp>::on_stack(
        target(WS, "/echo"),
        Arc::clone(&tls_cfg),
        WsOptions::default(),
    )?)?;
    poll_until(&mut d, &mut h, Duration::from_secs(30), "ws connect", |h| {
        !h.opened.is_empty()
    })?;
    assert_eq!(h.opened, [a]);
    let before = h.frames;
    d.get_mut(a)
        .ok_or("gone")?
        .send_text("hello over TLS on F-Stack")?;
    poll_until(&mut d, &mut h, Duration::from_secs(10), "ws echo", |h| {
        h.frames > before
    })?;
    assert_eq!(h.last, b"hello over TLS on F-Stack");
    d.get_mut(a).ok_or("gone")?.ping(b"p")?;

    let msg = [b'x'; 128];
    let mut rtt = Vec::with_capacity(TRIPS);
    let mut allocs_after_warmup = 0;
    for i in 0..2 * TRIPS {
        if i == TRIPS {
            allocs_after_warmup = allocs();
        }
        let before = h.frames;
        let t0 = Instant::now();
        d.get_mut(a).ok_or("gone")?.send_binary(&msg)?;
        while h.frames == before {
            d.poll(&mut h)?;
            if t0.elapsed() > Duration::from_secs(10) {
                return Err(format!("ws echo {i} lost").into());
            }
        }
        if i >= TRIPS {
            rtt.push(t0.elapsed().as_nanos() as u64);
        }
    }
    let grew = allocs() - allocs_after_warmup;
    assert_eq!(
        grew, 0,
        "{grew} allocations in {TRIPS} warm WebSocket round trips"
    );
    assert_eq!(h.last, msg);
    println!(
        "fstack-test: WebSocket+TLS echo over F-Stack: {}",
        summary(&mut rtt)
    );
    let info = d.tcp_info(a).ok_or("no TCP_INFO sample")?;
    assert!(info.rtt_us > 0 && info.rto_us > 0, "{info:?}");
    println!("fstack-test: TCP_INFO {info:?}");

    // One made on this thread, then added; then both close (the handshake).
    let c = WsDial::<FstackTcp>::on_stack(target(WS, "/b"), tls_cfg, WsOptions::default())?
        .connect_blocking()?;
    let b = d.add(c)?;
    for id in [a, b] {
        d.get_mut(id).ok_or("gone")?.close(1000, "done")?;
    }
    poll_until(&mut d, &mut h, Duration::from_secs(10), "ws close", |h| {
        h.closed.len() == 2
    })?;
    for (_, e) in &h.closed {
        assert!(
            matches!(e, Error::WsClose(c) if c.code == Some(1000)),
            "{e}"
        );
    }
    println!("fstack-test: WebSocket over TLS ok");
    Ok(())
}

fn http(cfg: &Config) -> R {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(std::fs::read(cert_path()?)?))?;
    let tls_cfg = tls::client_config(roots)?;
    let mut d: Driver<HttpConn<FstackTcp>> = Driver::new(cfg.clone())?;
    let mut h = Events::default();
    let a = d.connect(HttpDial::<FstackTcp>::on_stack(
        target(HTTP, "/"),
        Arc::clone(&tls_cfg),
        HttpOptions::default(),
    )?)?;
    poll_until(
        &mut d,
        &mut h,
        Duration::from_secs(30),
        "http connect",
        |h| !h.opened.is_empty(),
    )?;
    for i in 0..20 {
        let path = format!("/hello?i={i}");
        d.get_mut(a)
            .ok_or("gone")?
            .request("GET", &path, &[], b"")?;
        poll_until(
            &mut d,
            &mut h,
            Duration::from_secs(10),
            "http response",
            |h| h.responses.len() > i,
        )?;
        assert_eq!(h.responses[i], (200, path.into_bytes()));
    }
    let c = HttpDial::<FstackTcp>::on_stack(target(HTTP, "/"), tls_cfg, HttpOptions::default())?
        .connect_blocking()?;
    let b = d.add(c)?;
    d.get_mut(b)
        .ok_or("gone")?
        .request("GET", "/added", &[], b"")?;
    poll_until(
        &mut d,
        &mut h,
        Duration::from_secs(10),
        "http response",
        |h| h.responses.len() == 21,
    )?;
    assert_eq!(h.responses[20], (200, b"/added".to_vec()));
    d.close(a);
    d.close(b);
    println!("fstack-test: HTTP over TLS ok");
    Ok(())
}
