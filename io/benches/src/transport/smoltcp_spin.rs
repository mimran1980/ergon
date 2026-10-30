//! `smoltcp-spin` measuring loops for `wstls_lat` and `udp_lat`.
//!
//! Ping is `Stack` plus a `Driver` on this thread. Pong is the kernel peer
//! (`wstls::kernel_serve`, or `udp_lat`'s kernel sockets): host B has no
//! DPDK port, and smoltcp has no listen side. The row is labelled
//! `smoltcp-spin`. Linux, `--features smoltcp-dpdk`.

#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ergon_io::smoltcp_dpdk::{SmolTcp, SmolUdp, Stack};
use ergon_io::{
    Backend, Config, ConnId, Driver, FrameConn, FrameDial, FrameHandler, FrameOptions, Handler,
    RxMeta, Target, UdpConn, UdpHandler, UdpOptions, WsConn, WsDial, WsHandler, WsOptions,
};
use ergon_latency_core::message::MESSAGE_LEN;

use super::bypass::{self, BypassNic};
use super::eio_udp::{self, Staged};
use super::point::{Common, Error, Role};
use super::sock::{self, Impl, Inspect};
use super::wstls::{self, Echo, WstlsArgs};
use super::{
    MAX_DATAGRAM, MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF, TX_SLOTS, ip_overhead, port_plus,
    tcp_echo_capacity,
};

const CONNECT_WAIT: Duration = Duration::from_secs(30);

/// TCP or WebSocket-over-TLS echo. The row, or `None` for a pong.
///
/// # Errors
/// A flag the stack refuses, DPDK or the connect fails, or what the point met.
pub fn wstls(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    match c.role {
        Role::Pong => {
            c.check_cpus(1, "echo")?;
            wstls::kernel_serve(c, a)?;
            Ok(None)
        }
        Role::Ping => ping(c, a),
        Role::Local | Role::OnewaySend | Role::OnewayRecv => Err(Error::Usage(format!(
            "--role {}: one smoltcp-spin stack per process",
            c.role.name()
        ))),
    }
}

/// UDP ping. The pong is chosen earlier and stays on the kernel.
///
/// # Errors
/// A flag the stack refuses, DPDK or a connect fails, or what the point met.
pub fn udp(
    c: &Common,
    nic: BypassNic,
    local: SocketAddr,
    peer: SocketAddr,
    rcvbuf: u32,
    sndbuf: Option<u32>,
    datagram_burst: u32,
) -> Result<Option<String>, Error> {
    if c.role != Role::Ping {
        return Err(Error::Usage(format!(
            "--role {}: smoltcp-spin UDP ping is the only userspace role",
            c.role.name()
        )));
    }
    let arm = arm_line("smoltcp-spin", "smoltcp", nic.rto_min_us, false);
    let streams = c.streams;
    sock::run(c, Impl::Ergon, "udp", &arm, false, move |_end, cpu| {
        let stack = start_stack(&nic)?;
        let mut driver: Driver<UdpConn<SmolUdp>> =
            Driver::new(driver_cfg(cpu, rcvbuf, Some(datagram_burst))?)
                .map_err(|e| eio_udp::eio("Driver::new", e))?;
        let opts = UdpOptions {
            max_datagram: MAX_DATAGRAM,
            tx_slots: TX_SLOTS,
            sndbuf,
            ..UdpOptions::default()
        };
        let n = usize_of(streams)?;
        let mut ids = Vec::with_capacity(n);
        for i in 0..streams {
            let local_addr = port_plus(local, i)?;
            let peer_addr = peer_v4(port_plus(peer, i)?, "smoltcp-spin")?;
            let conn = stack
                .udp_connect(local_addr.port(), peer_addr, &opts)
                .map_err(|e| eio_udp::eio("udp_connect", e))?;
            ids.push(
                driver
                    .add(conn)
                    .map_err(|e| eio_udp::eio("Driver::add", e))?,
            );
        }
        udp_pair(driver, stack, ids)
    })
}

fn ping(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    match a.echo {
        Echo::Tcp => ping_tcp(c, a),
        Echo::Wstls => ping_ws(c, a),
    }
}

fn ping_tcp(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    let nic = a.nic()?;
    let peer = peer_v4(a.peer, a.backend.name())?;
    let arm = arm_line("smoltcp-spin", "smoltcp", nic.rto_min_us, true);
    let streams = c.streams;
    let rcvbuf = a.rcvbuf;
    let sndbuf = a.sndbuf;
    sock::run(c, Impl::Ergon, "tcp", &arm, false, move |_end, cpu| {
        let stack = start_stack(&nic)?;
        let mut driver: Driver<FrameConn<SmolTcp>> = Driver::new(driver_cfg(cpu, rcvbuf, None)?)
            .map_err(|e| eio_udp::eio("Driver::new", e))?;
        let opts = frame_opts(sndbuf);
        let mut ids = Vec::with_capacity(usize_of(streams)?);
        for _ in 0..streams {
            let conn = connect_frame(peer, nic.ip, opts)?;
            ids.push(
                driver
                    .add(conn)
                    .map_err(|e| eio_udp::eio("Driver::add", e))?,
            );
        }
        frame_pair(driver, stack, ids)
    })
}

fn ping_ws(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    let nic = a.nic()?;
    let peer = peer_v4(a.peer, a.backend.name())?;
    let arm = arm_line("smoltcp-spin", "smoltcp", nic.rto_min_us, true);
    let streams = c.streams;
    let rcvbuf = a.rcvbuf;
    sock::run(c, Impl::Ergon, "wstls", &arm, false, move |_end, cpu| {
        let stack = start_stack(&nic)?;
        let mut driver: Driver<WsConn<SmolTcp>> = Driver::new(driver_cfg(cpu, rcvbuf, None)?)
            .map_err(|e| eio_udp::eio("Driver::new", e))?;
        let tls = wstls::bench_client_config()?;
        let mut ids = Vec::with_capacity(usize_of(streams)?);
        for _ in 0..streams {
            let conn = connect_ws(peer, nic.ip, Arc::clone(&tls))?;
            ids.push(
                driver
                    .add(conn)
                    .map_err(|e| eio_udp::eio("Driver::add", e))?,
            );
        }
        ws_pair(driver, stack, ids)
    })
}

fn start_stack(nic: &BypassNic) -> Result<Stack, Error> {
    let cfg = bypass::stack_config(nic)?;
    Stack::init(&cfg).map_err(|e| eio_udp::eio("Stack::init", e))
}

fn driver_cfg(
    cpu: Option<usize>,
    rcvbuf: u32,
    datagram_burst: Option<u32>,
) -> Result<Config, Error> {
    let mut cfg = Config::new(Backend::SmoltcpSpin);
    cfg.loop_cpu = cpu.and_then(|n| u32::try_from(n).ok());
    cfg.rcvbuf = Some(rcvbuf);
    if let Some(burst) = datagram_burst {
        cfg.datagram_burst = burst;
    }
    // Receive timestamps are the driver's clock, not the TCP option. The
    // stack keeps `tcp_timestamps` (recorded on the arm). Wire bytes follow
    // the kernel wstls formula and do not add the 12-byte option.
    cfg.rx_timestamps = false;
    cfg.rx_read_timestamp = false;
    cfg.tcp_user_timeout_ms = 0;
    cfg.keepalive_idle_s = 0;
    cfg.tcp_info_interval_ms = 0;
    cfg.validate()
        .map_err(|e| Error::Usage(format!("smoltcp-spin config: {e}")))?;
    Ok(cfg)
}

const fn frame_opts(sndbuf: u32) -> FrameOptions {
    FrameOptions {
        max_frame: MAX_FRAME,
        rx_buf: TCP_RX_BUF,
        tx_buf: TCP_TX_BUF,
        sndbuf: Some(sndbuf),
        quickack_per_read: false,
    }
}

fn connect_frame(
    to: SocketAddrV4,
    source: std::net::Ipv4Addr,
    opts: FrameOptions,
) -> Result<FrameConn<SmolTcp>, Error> {
    let target = Target::new(to.ip().to_string(), to.port(), "/")
        .with_addr(SocketAddr::V4(to))
        .with_source(IpAddr::V4(source));
    let until = Instant::now() + CONNECT_WAIT;
    loop {
        match FrameDial::<SmolTcp>::on_stack(target.clone(), opts)
            .and_then(FrameDial::connect_blocking)
        {
            Ok(conn) => return Ok(conn),
            Err(_) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(eio_udp::eio(&format!("connect {to}"), e)),
        }
    }
}

fn connect_ws(
    to: SocketAddrV4,
    source: std::net::Ipv4Addr,
    tls: Arc<rustls::ClientConfig>,
) -> Result<WsConn<SmolTcp>, Error> {
    let target = Target::new("localhost", to.port(), "/")
        .with_addr(SocketAddr::V4(to))
        .with_source(IpAddr::V4(source));
    let opts = WsOptions {
        ping_interval_ms: 0,
        pong_timeout_ms: 0,
        idle_timeout_ms: 0,
        close_timeout_ms: 0,
        ..WsOptions::default()
    };
    WsDial::<SmolTcp>::on_stack(target, tls, opts)
        .and_then(WsDial::connect_blocking)
        .map_err(|e| eio_udp::eio(&format!("websocket connect {to}"), e))
}

fn arm_line(backend: &str, stack: &str, rto: u32, timestamps: bool) -> Vec<(String, String)> {
    let mut arm = vec![
        ("backend".to_owned(), backend.to_owned()),
        ("stack".to_owned(), stack.to_owned()),
        ("rto_min_us".to_owned(), rto.to_string()),
    ];
    if timestamps {
        arm.push(("tcp.timestamps".to_owned(), "true".to_owned()));
    }
    arm
}

fn peer_v4(addr: SocketAddr, backend: &str) -> Result<SocketAddrV4, Error> {
    match addr {
        SocketAddr::V4(v) => Ok(v),
        SocketAddr::V6(v) => Err(Error::Usage(format!("--peer {v}: IPv4 ({backend})"))),
    }
}

fn usize_of(n: u32) -> Result<usize, Error> {
    usize::try_from(n).map_err(|_| Error::Usage(format!("--streams {n}: does not fit")))
}

fn id_at(ids: &[ConnId], stream: u32) -> Result<ConnId, Error> {
    ids.get(usize_of(stream)?)
        .copied()
        .ok_or_else(|| Error::Transport(format!("stream {stream} of {}", ids.len())))
}

fn wrote(result: Result<(), ergon_io::Error>, what: &str) -> Result<bool, Error> {
    match result {
        Ok(()) => Ok(true),
        Err(ergon_io::Error::Busy(_)) => Ok(false),
        Err(e) => Err(eio_udp::eio(what, e)),
    }
}

const fn tcp_wire() -> u64 {
    ip_overhead(false) + 20 + 4 + MESSAGE_LEN as u64
}

const fn ws_wire() -> u64 {
    ip_overhead(false) + 20 + 5 + 6 + MESSAGE_LEN as u64
}

const fn udp_wire() -> u64 {
    MESSAGE_LEN as u64 + 8 + ip_overhead(false)
}

struct FrameShared {
    d: RefCell<Driver<FrameConn<SmolTcp>>>,
    stack: Stack,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
}

struct FrameTx {
    s: Rc<FrameShared>,
}

struct FrameRx {
    s: Rc<FrameShared>,
    gone: usize,
}

fn frame_pair(
    driver: Driver<FrameConn<SmolTcp>>,
    stack: Stack,
    ids: Vec<ConnId>,
) -> Result<(FrameTx, FrameRx), Error> {
    if ids.is_empty() {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let room = tcp_echo_capacity(ids.len(), ids.len(), false);
    let s = Rc::new(FrameShared {
        d: RefCell::new(driver),
        stack,
        staged: RefCell::new(Staged::new(room)),
        ids,
    });
    Ok((FrameTx { s: Rc::clone(&s) }, FrameRx { s, gone: 0 }))
}

fn send_frame(
    d: &mut Driver<FrameConn<SmolTcp>>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = id_at(ids, stream)?;
    let result = {
        let mut c = d
            .get_mut(id)
            .ok_or_else(|| Error::Transport(format!("stream {stream}: its connection closed")))?;
        c.send_frame(msg)
    };
    if wrote(result, "send_frame")? {
        d.flush().map_err(|e| eio_udp::eio("flush", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

impl super::point::Tx for FrameTx {
    fn try_send(
        &mut self,
        stream: u32,
        msg: &[u8; MESSAGE_LEN],
    ) -> Result<super::point::Sent, Error> {
        let Ok(mut d) = self.s.d.try_borrow_mut() else {
            return Ok(if self.s.staged.borrow_mut().push(stream, msg) {
                super::point::Sent::Sent
            } else {
                super::point::Sent::Busy
            });
        };
        Ok(if send_frame(&mut d, &self.s.ids, stream, msg)? {
            super::point::Sent::Sent
        } else {
            super::point::Sent::Busy
        })
    }
}

struct FrameDeliver<'a, F> {
    on: &'a mut F,
    gone: usize,
    err: Option<Error>,
}

impl<F> Handler for FrameDeliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        note_closed(&mut self.gone, &mut self.err, id, error, "tcp");
    }
}

impl<F: FnMut(&[u8])> FrameHandler for FrameDeliver<'_, F> {
    fn on_frame(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        (self.on)(payload);
    }
}

impl super::point::Rx for FrameRx {
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let mut d = self.s.d.borrow_mut();
        if drain_staged(&self.s.staged, &mut d, &self.s.ids, send_frame)? {
            return Ok(0);
        }
        let mut h = FrameDeliver {
            on,
            gone: 0,
            err: None,
        };
        let n = d.poll(&mut h).map_err(|e| eio_udp::eio("poll", e))?;
        self.gone += h.gone;
        if let Some(e) = h.err {
            return Err(e);
        }
        let _still_staged = drain_staged(&self.s.staged, &mut d, &self.s.ids, send_frame)?;
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.gone >= self.s.ids.len()
    }
}

impl Inspect for FrameRx {
    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(tcp_wire())
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        report_stack(&self.s.d.borrow(), &self.s.stack, out);
        let d = self.s.d.borrow();
        let (mut frames_in, mut frames_out) = (0u64, 0u64);
        for c in self.s.ids.iter().filter_map(|&id| d.get(id)) {
            let s = c.stats();
            frames_in += s.frames_in;
            frames_out += s.frames_out;
        }
        out.push(("stat.frames_in".to_owned(), frames_in.to_string()));
        out.push(("stat.frames_out".to_owned(), frames_out.to_string()));
        Ok(())
    }
}

struct WsShared {
    d: RefCell<Driver<WsConn<SmolTcp>>>,
    stack: Stack,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
}

struct WsTx {
    s: Rc<WsShared>,
}
struct WsRx {
    s: Rc<WsShared>,
    gone: usize,
}

fn ws_pair(
    driver: Driver<WsConn<SmolTcp>>,
    stack: Stack,
    ids: Vec<ConnId>,
) -> Result<(WsTx, WsRx), Error> {
    if ids.is_empty() {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let room = tcp_echo_capacity(ids.len(), ids.len(), false);
    let s = Rc::new(WsShared {
        d: RefCell::new(driver),
        stack,
        staged: RefCell::new(Staged::new(room)),
        ids,
    });
    Ok((WsTx { s: Rc::clone(&s) }, WsRx { s, gone: 0 }))
}

fn send_ws(
    d: &mut Driver<WsConn<SmolTcp>>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = id_at(ids, stream)?;
    let result = {
        let mut c = d
            .get_mut(id)
            .ok_or_else(|| Error::Transport(format!("stream {stream}: its connection closed")))?;
        c.send_binary(msg)
    };
    if wrote(result, "send_binary")? {
        d.flush().map_err(|e| eio_udp::eio("flush", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

impl super::point::Tx for WsTx {
    fn try_send(
        &mut self,
        stream: u32,
        msg: &[u8; MESSAGE_LEN],
    ) -> Result<super::point::Sent, Error> {
        let Ok(mut d) = self.s.d.try_borrow_mut() else {
            return Ok(if self.s.staged.borrow_mut().push(stream, msg) {
                super::point::Sent::Sent
            } else {
                super::point::Sent::Busy
            });
        };
        Ok(if send_ws(&mut d, &self.s.ids, stream, msg)? {
            super::point::Sent::Sent
        } else {
            super::point::Sent::Busy
        })
    }
}

struct WsDeliver<'a, F> {
    on: &'a mut F,
    gone: usize,
    err: Option<Error>,
}

impl<F> Handler for WsDeliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        note_closed(&mut self.gone, &mut self.err, id, error, "websocket");
    }
}

impl<F: FnMut(&[u8])> WsHandler for WsDeliver<'_, F> {
    fn on_message(&mut self, _conn: ConnId, opcode: u8, payload: &[u8], _meta: &RxMeta, _pos: u16) {
        if opcode == 0x2 {
            (self.on)(payload);
        }
    }
}

impl super::point::Rx for WsRx {
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let mut d = self.s.d.borrow_mut();
        if drain_staged(&self.s.staged, &mut d, &self.s.ids, send_ws)? {
            return Ok(0);
        }
        let mut h = WsDeliver {
            on,
            gone: 0,
            err: None,
        };
        let n = d.poll(&mut h).map_err(|e| eio_udp::eio("poll", e))?;
        self.gone += h.gone;
        if let Some(e) = h.err {
            return Err(e);
        }
        let _still_staged = drain_staged(&self.s.staged, &mut d, &self.s.ids, send_ws)?;
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.gone >= self.s.ids.len()
    }
}

impl Inspect for WsRx {
    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(ws_wire())
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        report_stack(&self.s.d.borrow(), &self.s.stack, out);
        let d = self.s.d.borrow();
        let mut messages = 0u64;
        for c in self.s.ids.iter().filter_map(|&id| d.get(id)) {
            messages += c.stats().messages;
        }
        out.push(("stat.messages".to_owned(), messages.to_string()));
        Ok(())
    }
}

struct UdpShared {
    d: RefCell<Driver<UdpConn<SmolUdp>>>,
    stack: Stack,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
}

struct UdpTx {
    s: Rc<UdpShared>,
}
struct UdpRx {
    s: Rc<UdpShared>,
    gone: usize,
}

fn udp_pair(
    driver: Driver<UdpConn<SmolUdp>>,
    stack: Stack,
    ids: Vec<ConnId>,
) -> Result<(UdpTx, UdpRx), Error> {
    if ids.is_empty() {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let s = Rc::new(UdpShared {
        d: RefCell::new(driver),
        stack,
        staged: RefCell::new(Staged::new(ids.len().saturating_mul(64).max(1))),
        ids,
    });
    Ok((UdpTx { s: Rc::clone(&s) }, UdpRx { s, gone: 0 }))
}

fn send_udp(
    d: &mut Driver<UdpConn<SmolUdp>>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = id_at(ids, stream)?;
    let result = {
        let mut c = d
            .get_mut(id)
            .ok_or_else(|| Error::Transport(format!("stream {stream}: its socket closed")))?;
        c.send(msg)
    };
    if wrote(result, "send")? {
        d.flush().map_err(|e| eio_udp::eio("flush", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

impl super::point::Tx for UdpTx {
    fn try_send(
        &mut self,
        stream: u32,
        msg: &[u8; MESSAGE_LEN],
    ) -> Result<super::point::Sent, Error> {
        let Ok(mut d) = self.s.d.try_borrow_mut() else {
            return Ok(if self.s.staged.borrow_mut().push(stream, msg) {
                super::point::Sent::Sent
            } else {
                super::point::Sent::Busy
            });
        };
        Ok(if send_udp(&mut d, &self.s.ids, stream, msg)? {
            super::point::Sent::Sent
        } else {
            super::point::Sent::Busy
        })
    }
}

struct UdpDeliver<'a, F> {
    on: &'a mut F,
    gone: usize,
    err: Option<Error>,
}

impl<F> Handler for UdpDeliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        note_closed(&mut self.gone, &mut self.err, id, error, "udp");
    }
}

impl<F: FnMut(&[u8])> UdpHandler for UdpDeliver<'_, F> {
    fn on_datagram(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        (self.on)(payload);
    }
}

impl super::point::Rx for UdpRx {
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let mut d = self.s.d.borrow_mut();
        if drain_staged(&self.s.staged, &mut d, &self.s.ids, send_udp)? {
            return Ok(0);
        }
        let mut h = UdpDeliver {
            on,
            gone: 0,
            err: None,
        };
        let n = d.poll(&mut h).map_err(|e| eio_udp::eio("poll", e))?;
        self.gone += h.gone;
        if let Some(e) = h.err {
            return Err(e);
        }
        let _still_staged = drain_staged(&self.s.staged, &mut d, &self.s.ids, send_udp)?;
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.gone >= self.s.ids.len()
    }
}

impl Inspect for UdpRx {
    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(udp_wire())
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        report_stack(&self.s.d.borrow(), &self.s.stack, out);
        Ok(())
    }
}

type SendOne<E> = fn(&mut Driver<E>, &[ConnId], u32, &[u8; MESSAGE_LEN]) -> Result<bool, Error>;

fn drain_staged<E: ergon_io::Endpoint>(
    staged: &RefCell<Staged>,
    d: &mut Driver<E>,
    ids: &[ConnId],
    send: SendOne<E>,
) -> Result<bool, Error> {
    let mut staged = staged.borrow_mut();
    if staged.len == 0 {
        return Ok(false);
    }
    d.flush().map_err(|e| eio_udp::eio("flush", e))?;
    staged.drain(|stream, msg| send(d, ids, stream, msg))?;
    Ok(staged.len > 0)
}

fn note_closed(
    gone: &mut usize,
    err: &mut Option<Error>,
    id: ConnId,
    error: ergon_io::Error,
    what: &str,
) {
    match error {
        ergon_io::Error::Closed => *gone += 1,
        ergon_io::Error::Io(e) if e.kind() == std::io::ErrorKind::ConnectionReset => *gone += 1,
        e => {
            if err.is_none() {
                *err = Some(Error::Transport(format!("{what} {id:?} closed: {e}")));
            }
        }
    }
}

fn report_stack<E: ergon_io::Endpoint>(
    d: &Driver<E>,
    stack: &Stack,
    out: &mut Vec<(String, String)>,
) {
    let cfg = d.config();
    out.push(("ergon.backend".to_owned(), cfg.backend.to_string()));
    out.push(("smoltcp.driver".to_owned(), stack.driver()));
    out.push(("stat.polls".to_owned(), d.stats().polls.to_string()));
}
