//! Connection set-up without blocking: the [`Target`] description and the TCP
//! connect plus TLS handshake every protocol shares.

use std::io::{ErrorKind, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::os::fd::RawFd;
use std::sync::Arc;

use rustls::ClientConfig;

use crate::error::{Error, Result};
use crate::netstack::{Datagram, Kernel, Socket, Sockets};
use crate::route::RouteCheck;
use crate::tls::{RecordLayer, TlsHandshake};

/// Where to connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Host name: resolved through DNS (unless [`Target::addr`] is set), sent as
    /// TLS SNI, checked against the certificate and used in the `Host` header.
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Request path including the query (`/ws`, `/resource?...`).
    pub path: String,
    /// Extra request headers, e.g. `X-API-Key`.
    pub headers: Vec<(String, String)>,
    /// A fixed address: skips DNS (the host is still used for TLS and `Host`).
    pub addr: Option<SocketAddr>,
    /// The local address to connect from: the socket binds it (port 0)
    /// before connecting. `None`: the kernel picks one by route. It must be
    /// an address of this host, of the destination's family; on Linux the
    /// port is chosen at connect, per destination
    /// (`IP_BIND_ADDRESS_NO_PORT`), so binding uses up no ephemeral ports.
    /// For several IPs each with its own connect limit, see
    /// [`crate::reconnect::Reconnect::poll_sourced`].
    pub source: Option<IpAddr>,
}

impl Target {
    /// `host:port` with `path`.
    #[must_use]
    pub fn new(host: impl Into<String>, port: u16, path: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            path: path.into(),
            headers: Vec::new(),
            addr: None,
            source: None,
        }
    }

    /// Pins the address (no DNS).
    #[must_use]
    pub const fn with_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    /// Connects from `ip` ([`Target::source`]).
    #[must_use]
    pub const fn with_source(mut self, ip: IpAddr) -> Self {
        self.source = Some(ip);
        self
    }

    /// Adds a request header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// The `Host` header value: the host, plus the port unless it is 443.
    #[must_use]
    pub fn host_header(&self) -> String {
        if self.port == 443 {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

/// A dial whose source IP ([`Target::source`]) can be set once built.
///
/// [`crate::reconnect::Reconnect::poll_sourced`] uses it to bind each
/// attempt to the IP its gate picked.
pub trait BindSource {
    /// Connect from `ip`.
    fn bind_source(&mut self, ip: IpAddr);
}

/// The TCP socket an endpoint is built on, and so its stack.
///
/// The kernel's [`std::net::TcpStream`] (every endpoint's default), or with
/// the `fstack` feature `ergon_io::fstack::FstackTcp`, a socket of
/// F-Stack's user-space stack. `FrameConn<T>`, `WsConn<T>` and `HttpConn<T>` (and their dials)
/// take it as their type parameter: an endpoint is made, read, written and
/// closed through its own stack, and a `Driver` refuses an endpoint whose
/// stack is not its backend's.
///
/// Sealed: only this crate's socket types implement it.
#[allow(private_bounds)] // sealed on purpose: the bound is unnameable outside the crate
pub trait StreamSocket: Socket {}
impl<T: Socket> StreamSocket for T {}

/// The UDP socket a `UdpConn` is built on, and so its stack: the kernel's
/// [`std::net::UdpSocket`] (the default), or with the `smoltcp-dpdk`
/// feature `ergon_io::smoltcp_dpdk::SmolUdp`.
///
/// Sealed: only this crate's socket types implement it.
#[allow(private_bounds)] // sealed on purpose: the bound is unnameable outside the crate
pub trait DatagramSocket: Datagram {}
impl<T: Datagram> DatagramSocket for T {}

/// Starts a non-blocking TCP connect (`TCP_NODELAY` set), from `source`
/// when given ([`Target::source`]).
///
/// # Errors
/// [`Error::Config`]: `source` is not of `addr`'s family. Socket creation,
/// the bind (`AddrNotAvailable`: not an address of this host), or an
/// immediate connect failure.
pub fn connect_nonblocking(addr: SocketAddr, source: Option<IpAddr>) -> Result<TcpStream> {
    Kernel::tcp_connect(addr, source)
}

/// Whether a non-blocking connect has finished (`Ok(false)`: still in progress).
///
/// # Errors
/// The connect failed.
pub fn connect_finished(sock: &TcpStream) -> Result<bool> {
    Kernel::connect_finished(sock)
}

enum Stage<T> {
    Idle,
    Tcp(T),
    /// TCP is connected; the TLS handshake waits for the route check.
    Gated(T),
    Tls(T, Box<TlsHandshake>),
    Done,
}

/// TCP connect then TLS handshake, advanced by [`TlsDial::step`], over
/// socket type `T`.
pub(crate) struct TlsDial<T = TcpStream> {
    cfg: Arc<ClientConfig>,
    sni: String,
    stage: Stage<T>,
    /// Stop at TCP connect until the route is checked
    /// ([`TlsDial::gate_route`]).
    gate: bool,
    /// The route check's outcome, for the connection.
    route: Option<RouteCheck>,
}

impl<T: Socket> TlsDial<T> {
    pub(crate) fn new(cfg: Arc<ClientConfig>, sni: String) -> Result<Self> {
        crate::tls::check_client_config(&cfg)?;
        Ok(Self {
            cfg,
            sni,
            stage: Stage::Idle,
            gate: false,
            route: None,
        })
    }

    pub(crate) fn start(&mut self, addr: SocketAddr, source: Option<IpAddr>) -> Result<()> {
        self.stage = Stage::Tcp(T::start_connect(addr, source)?);
        Ok(())
    }

    /// From now on, stop once TCP is connected, before the TLS handshake,
    /// until [`TlsDial::route_checked`].
    pub(crate) const fn gate_route(&mut self) {
        self.gate = true;
    }

    /// The socket, while the dial waits for its route check.
    pub(crate) fn awaiting_route(&self) -> Option<RawFd> {
        match &self.stage {
            Stage::Gated(sock) => Some(sock.fd()),
            _ => None,
        }
    }

    /// The route check's outcome: the next [`TlsDial::step`] starts TLS.
    pub(crate) fn route_checked(&mut self, check: RouteCheck) {
        self.route = Some(check);
    }

    /// The route check's outcome, once, for the connection.
    pub(crate) const fn take_route(&mut self) -> Option<RouteCheck> {
        self.route.take()
    }

    /// `Some` once TLS is established: the socket, the record layer and bytes
    /// already received after the handshake.
    pub(crate) fn step(&mut self) -> Result<Option<(T, RecordLayer, Vec<u8>)>> {
        loop {
            match std::mem::replace(&mut self.stage, Stage::Done) {
                Stage::Idle => {
                    self.stage = Stage::Idle;
                    return Ok(None);
                }
                Stage::Tcp(sock) => {
                    if !sock.connect_finished()? {
                        self.stage = Stage::Tcp(sock);
                        return Ok(None);
                    }
                    self.stage = Stage::Gated(sock);
                }
                Stage::Gated(sock) => {
                    if self.gate && self.route.is_none() {
                        self.stage = Stage::Gated(sock);
                        return Ok(None);
                    }
                    let hs = Box::new(TlsHandshake::new(self.cfg.clone(), &self.sni)?);
                    self.stage = Stage::Tls(sock, hs);
                }
                Stage::Tls(mut sock, mut hs) => {
                    if !hs.step(&mut sock)? {
                        self.stage = Stage::Tls(sock, hs);
                        return Ok(None);
                    }
                    let (layer, rest) = hs.finish()?;
                    return Ok(Some((sock, layer, rest)));
                }
                Stage::Done => return Err(Error::Config("dial already finished".into())),
            }
        }
    }
}

/// Writes as much of `q` as the socket takes; `true` when it is empty.
pub(crate) fn flush_nonblocking<W: Write>(sock: &mut W, q: &mut Vec<u8>) -> Result<bool> {
    while !q.is_empty() {
        match sock.write(q) {
            Ok(0) => return Err(Error::Closed),
            Ok(n) => {
                q.drain(..n);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(false),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

/// Steps a dial over socket type `T` until it yields, running `T`'s stack
/// between steps (F-Stack: one pass), for at most `limit`.
///
/// # Errors
/// What `step` returns, or [`Error::Timeout`] (`what`) after `limit`.
pub(crate) fn wait_for<T: Socket, C>(
    limit: std::time::Duration,
    what: &'static str,
    mut step: impl FnMut() -> Result<Option<C>>,
) -> Result<C> {
    let start = std::time::Instant::now();
    loop {
        if let Some(c) = step()? {
            return Ok(c);
        }
        if start.elapsed() > limit {
            return Err(Error::Timeout(what));
        }
        T::pump();
        if T::STACK == crate::netstack::StackId::Kernel {
            std::thread::yield_now();
        }
    }
}
