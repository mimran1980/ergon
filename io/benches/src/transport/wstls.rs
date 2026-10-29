//! `wstls_lat`: TCP echo and WebSocket-over-TLS echo on the suite's point
//! runner ([`point::ping`], [`point::echo`], [`point::finish`]).
//!
//! Kernel `epoll-spin` is a real `TcpStream` (loopback for `--role local`,
//! the peer's address for `ping` and `pong`). A bypass backend is not a
//! kernel socket.
//!
//! With `smoltcp-dpdk`, ping measures on smoltcp and pong is the kernel
//! peer. With `fstack-spin` and without `smoltcp-dpdk`, WebSocket ping
//! measures on F-Stack (`wstls_fstack_lat`) and pong is the kernel peer.
//! A build without the feature still refuses the flag and names the
//! backend. `--rto-min-us` (8 ms in the bypass matrix) is `TCP_RTO_MIN_US`
//! on the kernel, `StackConfig::rto_min_us` on smoltcp, and
//! `net.inet.tcp.rexmit_min` on F-Stack.
//!
//! Ping and pong are different processes. WebSocket-over-TLS uses one
//! baked-in certificate, so each process presents it and trusts it.
//!
//! One connection per stream. The 64-byte message is the payload. TCP
//! carries it after a `u32` little-endian length. WebSocket carries it as
//! one binary frame, masked client to server, after the TLS 1.3 handshake.

#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::MESSAGE_LEN;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection, StreamOwned,
};

use super::point::{
    self, Common, Error, Flags, POLL_LIMIT, Progress, Recorder, Report, Role, Rx, START_DELAY_NS,
    SendStats, Sent, Tx, thread_allocs,
};

/// What the connection carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Echo {
    /// Length-prefixed TCP (`--echo tcp`).
    Tcp,
    /// WebSocket binary frames over TLS 1.3 (`--echo wstls`).
    Wstls,
}

impl Echo {
    /// Its `--echo` value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Wstls => "wstls",
        }
    }

    fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "tcp" => Ok(Self::Tcp),
            "wstls" => Ok(Self::Wstls),
            other => Err(Error::Usage(format!("--echo {other}: tcp or wstls"))),
        }
    }
}

/// The stack the arm records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// The kernel's TCP, busy-polled.
    EpollSpin,
    /// ergon-io's smoltcp stack on DPDK.
    SmoltcpSpin,
    /// ergon-io's F-Stack stack on DPDK.
    FstackSpin,
}

impl Backend {
    /// Its `--backend` value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::EpollSpin => "epoll-spin",
            Self::SmoltcpSpin => "smoltcp-spin",
            Self::FstackSpin => "fstack-spin",
        }
    }

    fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "epoll-spin" => Ok(Self::EpollSpin),
            "smoltcp-spin" => Ok(Self::SmoltcpSpin),
            "fstack-spin" => Ok(Self::FstackSpin),
            other => Err(Error::Usage(format!(
                "--backend {other}: epoll-spin, smoltcp-spin or fstack-spin"
            ))),
        }
    }
}

/// `wstls_lat`'s own flags, after the common ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WstlsArgs {
    /// `--echo`.
    pub echo: Echo,
    /// `--backend`.
    pub backend: Backend,
    /// `--local` (the pong listens here; the ping binds this address).
    pub local: SocketAddr,
    /// `--peer` (the ping connects here).
    pub peer: SocketAddr,
    /// `--rto-min-us`, when the runner passed it.
    pub rto_min_us: Option<u32>,
    /// `--delack-max-us`, when the runner passed it.
    pub delack_max_us: Option<u32>,
    /// `--busy-read-us`.
    pub busy_read_us: u32,
    /// `--rcvbuf`.
    pub rcvbuf: u32,
    /// `--sndbuf`.
    pub sndbuf: u32,
    /// `--nodelay`.
    pub nodelay: bool,
    /// `--quickack-per-read`.
    pub quickack: bool,
    /// `--source`, the bypass stack's address. Absent on a kernel arm.
    pub source: Option<IpAddr>,
    /// `--prefix-len`. Absent on a kernel arm.
    pub prefix_len: Option<u8>,
    /// `--gateway`. Absent on a kernel arm.
    pub gateway: Option<Ipv4Addr>,
    /// `--pci`. Absent on a kernel arm, and when [`Self::vdev`] is set.
    pub pci: Option<String>,
    /// `--vdev`. Absent on a kernel arm, and when [`Self::pci`] is set.
    pub vdev: Option<String>,
}

impl WstlsArgs {
    /// Takes `wstls_lat`'s flags.
    ///
    /// `--impl ergon` is required: this binary is the ergon-io arm. The
    /// kernel socket flags the runner adds for `setup sockets` are taken so
    /// [`Flags::finish`] sees none left.
    ///
    /// # Errors
    /// [`Error::Usage`] for a flag missing, malformed or refused.
    pub fn parse(f: &mut Flags) -> Result<Self, Error> {
        let imp = f.req("impl")?;
        if imp != "ergon" {
            return Err(Error::Usage(format!(
                "--impl {imp}: ergon (wstls_lat is the ergon-io arm)"
            )));
        }
        let echo = Echo::parse(&f.req("echo")?)?;
        let backend = Backend::parse(&f.req("backend")?)?;
        let local = addr(f, "local")?;
        let peer = addr(f, "peer")?;
        let rto_min_us = opt_num(f, "rto-min-us")?;
        let delack_max_us = opt_num(f, "delack-max-us")?;
        let source = take_ip(f, "source")?;
        let prefix_len = take_prefix(f)?;
        let gateway = take_gateway(f)?;
        Ok(Self {
            echo,
            backend,
            local,
            peer,
            rto_min_us,
            delack_max_us,
            busy_read_us: f.num("busy-read-us", 0)?,
            rcvbuf: f.num("rcvbuf", super::DEFAULT_RCVBUF)?,
            sndbuf: f.num("sndbuf", super::DEFAULT_RCVBUF)?,
            nodelay: f.bool("nodelay", true)?,
            quickack: f.bool("quickack-per-read", true)?,
            source,
            prefix_len,
            gateway,
            pci: f.take("pci"),
            vdev: f.take("vdev"),
        })
    }

    /// The bypass ping's NIC. The kernel arm does not call this.
    ///
    /// # Errors
    /// [`Error::Usage`] when a NIC flag is missing or not IPv4.
    pub fn nic(&self) -> Result<super::bypass::BypassNic, Error> {
        let ip = match self.source.unwrap_or_else(|| self.local.ip()) {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(ip) => {
                return Err(Error::Usage(format!(
                    "--source {ip}: IPv4 ({})",
                    self.backend.name()
                )));
            }
        };
        let prefix_len = self.prefix_len.ok_or_else(|| {
            Error::Usage(format!(
                "--prefix-len is required for {}",
                self.backend.name()
            ))
        })?;
        let gateway = self.gateway.ok_or_else(|| {
            Error::Usage(format!("--gateway is required for {}", self.backend.name()))
        })?;
        if self.pci.is_some() == self.vdev.is_some() {
            return Err(Error::Usage(format!(
                "--pci or --vdev, not both, is required for {}",
                self.backend.name()
            )));
        }
        let rto_min_us = self.rto_min_us.ok_or_else(|| {
            Error::Usage(format!(
                "--rto-min-us is required for {} (the stack's RTO floor)",
                self.backend.name()
            ))
        })?;
        Ok(super::bypass::BypassNic {
            ip,
            prefix_len,
            gateway,
            pci: self.pci.clone(),
            vdev: self.vdev.clone(),
            rto_min_us,
        })
    }

    /// What an `fstack-spin` WebSocket ping passes to `fstack_spin_config`.
    ///
    /// Exactly one `--cpus` entry: an empty list is refused here even though
    /// the point runner allows it. Delayed ACK is the opposite of
    /// `--quickack-per-read`.
    ///
    /// # Errors
    /// [`Error::Usage`] when the CPU count is not one, or [`Self::nic`] fails.
    pub fn fstack_in(&self, cpus: &[usize]) -> Result<super::bypass::FstackSpinIn, Error> {
        let &[lcore] = cpus else {
            return Err(Error::Usage(format!(
                "--cpus: exactly one ({})",
                self.backend.name()
            )));
        };
        Ok(super::bypass::FstackSpinIn {
            nic: self.nic()?,
            lcore,
            delayed_ack: !self.quickack,
            rcvbuf: self.rcvbuf,
            sndbuf: self.sndbuf,
            connect_timeout: Duration::from_secs(30),
        })
    }
}

fn addr(f: &mut Flags, key: &str) -> Result<SocketAddr, Error> {
    let v = f.req(key)?;
    v.parse()
        .map_err(|_| Error::Usage(format!("--{key} {v}: IP:PORT")))
}

fn opt_num(f: &mut Flags, key: &str) -> Result<Option<u32>, Error> {
    f.take(key).map_or(Ok(None), |v| {
        v.parse()
            .map(Some)
            .map_err(|_| Error::Usage(format!("--{key} {v}: microseconds")))
    })
}

fn take_ip(f: &mut Flags, key: &str) -> Result<Option<IpAddr>, Error> {
    f.take(key).map_or(Ok(None), |v| {
        v.parse()
            .map(Some)
            .map_err(|_| Error::Usage(format!("--{key} {v}: an IP address")))
    })
}

fn take_prefix(f: &mut Flags) -> Result<Option<u8>, Error> {
    f.take("prefix-len").map_or(Ok(None), |v| {
        v.parse::<u8>()
            .ok()
            .filter(|n| (1..=32).contains(n))
            .map(Some)
            .ok_or_else(|| Error::Usage(format!("--prefix-len {v}: 1 to 32")))
    })
}

fn take_gateway(f: &mut Flags) -> Result<Option<Ipv4Addr>, Error> {
    f.take("gateway").map_or(Ok(None), |v| {
        v.parse()
            .map(Some)
            .map_err(|_| Error::Usage(format!("--gateway {v}: an IPv4 address")))
    })
}

/// `wstls_lat`'s usage.
pub const WSTLS_USAGE: &str = "\
wstls_lat --impl ergon --echo tcp|wstls --backend epoll-spin|smoltcp-spin|fstack-spin
        --role ping|pong|local --local IP:PORT --peer IP:PORT
        --rate MSG_PER_S --label SET:ARM --out DIR
        [--streams 1] [--warmup-s 30] [--duration-s 30] [--size 64] [--round-trip]
        [--cpus CSV] [--spike-ns 100000] [--stall-ns 10000]
        [--nodelay] [--quickack-per-read] [--rto-min-us N] [--delack-max-us N]
        [--busy-read-us N] [--rcvbuf BYTES] [--sndbuf BYTES]
        [--source IP] [--pci BDF | --vdev SPEC] [--prefix-len N] [--gateway IP]
TCP echo or WebSocket-over-TLS echo, one connection per stream, the same point
runner as tcp_lat. epoll-spin is the kernel. smoltcp-spin measures when this
binary is built with smoltcp-dpdk; fstack-spin measures in wstls_fstack_lat
(--features fstack-spin, not combined with smoltcp-dpdk). Without the feature
the flag is refused and a kernel socket is not labelled as that stack.
A bypass ping requires the NIC flags; its pong parses them and stays on the
kernel. --rto-min-us is the 8 ms floor: TCP_RTO_MIN_US on epoll-spin,
StackConfig on smoltcp-spin, net.inet.tcp.rexmit_min on fstack-spin.";

/// Parses `args` and runs the point. The row, or `None` for a pong.
///
/// # Errors
/// A flag refused, a bypass backend with no userspace stack in this process,
/// or what the point met.
pub fn run<I: IntoIterator<Item = String>>(args: I) -> Result<Option<String>, Error> {
    let mut f = Flags::parse(args)?;
    let c = Common::parse(&mut f)?;
    let a = WstlsArgs::parse(&mut f)?;
    f.finish()?;
    match a.backend {
        Backend::EpollSpin => kernel_run(&c, &a),
        Backend::SmoltcpSpin => smoltcp_wstls(&c, &a),
        Backend::FstackSpin => fstack_wstls(&c, &a),
    }
}

#[cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
fn smoltcp_wstls(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    super::smoltcp_spin::wstls(c, a)
}

#[cfg(not(all(target_os = "linux", feature = "smoltcp-dpdk")))]
fn smoltcp_wstls(_c: &Common, _a: &WstlsArgs) -> Result<Option<String>, Error> {
    Err(Error::Usage(
        "--backend smoltcp-spin is ergon-io's userspace TCP (StackConfig::rto_min_us from \
         --rto-min-us). A kernel socket will not be labelled smoltcp-spin"
            .into(),
    ))
}

#[cfg(all(
    target_os = "linux",
    feature = "fstack-spin",
    not(feature = "smoltcp-dpdk")
))]
fn fstack_wstls(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    super::fstack_ws::wstls(c, a)
}

#[cfg(not(all(
    target_os = "linux",
    feature = "fstack-spin",
    not(feature = "smoltcp-dpdk")
)))]
fn fstack_wstls(_c: &Common, _a: &WstlsArgs) -> Result<Option<String>, Error> {
    Err(Error::Usage(
        "--backend fstack-spin is ergon-io's fstack-spin backend (net.inet.tcp.rexmit_min \
         from --rto-min-us). A kernel socket will not be labelled fstack-spin"
            .into(),
    ))
}

fn kernel_run(c: &Common, a: &WstlsArgs) -> Result<Option<String>, Error> {
    match c.role {
        Role::Pong => {
            c.check_cpus(1, "echo")?;
            kernel_serve(c, a)?;
            Ok(None)
        }
        Role::Ping => {
            c.check_cpus(1, "ping")?;
            Ok(Some(ping_remote(c, a)?))
        }
        Role::Local => {
            c.check_cpus(2, "ping,echo (--round-trip) or sender,receiver")?;
            Ok(Some(local(c, a)?))
        }
        Role::OnewaySend | Role::OnewayRecv => Err(Error::Usage(
            "--role oneway-send and oneway-recv are refused; use --role local".into(),
        )),
    }
}

/// The kernel peer: listens, prints `ready`, accepts one connection per
/// stream and echoes. A bypass ping's pong runs this and writes no row.
///
/// # Errors
/// Bind, accept or the echo fails.
pub(crate) fn kernel_serve(c: &Common, a: &WstlsArgs) -> Result<(), Error> {
    let listener = listen(a.local)?;
    println!("ready");
    let tls = tls_pair(a.echo)?;
    let mut links = Vec::with_capacity(usize_of(c.streams)?);
    for i in 0..c.streams {
        let (sock, _) = listener
            .accept()
            .map_err(|e| Error::Timeout(format!("pong accept stream {i}: {e}")))?;
        links.push(server_link(sock, i, a, tls.as_ref())?);
    }
    let (mut tx, mut rx) = ends(links);
    point::echo(&mut rx, &mut tx, &mut || true)?;
    Ok(())
}

fn ping_remote(c: &Common, a: &WstlsArgs) -> Result<String, Error> {
    let tls = tls_pair(a.echo)?;
    let mut links = Vec::with_capacity(usize_of(c.streams)?);
    for i in 0..c.streams {
        let sock = connect_one(a, i)?;
        links.push(client_link(sock, i, a, tls.as_ref())?);
    }
    measure(c, a, links, None)
}

fn local(c: &Common, a: &WstlsArgs) -> Result<String, Error> {
    if c.round_trip {
        local_round_trip(c, a)
    } else {
        local_oneway(c, a)
    }
}

fn local_round_trip(c: &Common, a: &WstlsArgs) -> Result<String, Error> {
    let listener = listen(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    let at = listener
        .local_addr()
        .map_err(|e| Error::Transport(format!("local addr: {e}")))?;
    let tls_echo = Arc::new(tls_pair(a.echo)?);
    let tls_ping = Arc::clone(&tls_echo);
    let stop = AtomicBool::new(false);
    let barrier = Barrier::new(2);
    let clock = TscClock::new();
    let n = c.streams;
    std::thread::scope(|scope| {
        let echo = scope.spawn(|| -> Result<u64, Error> {
            point::pin(c.cpu(1))?;
            let mut links = Vec::with_capacity(usize_of(n)?);
            for i in 0..n {
                let (sock, _) = listener
                    .accept()
                    .map_err(|e| Error::Timeout(format!("local accept stream {i}: {e}")))?;
                links.push(server_link(sock, i, a, tls_echo.as_ref().as_ref())?);
            }
            let (mut tx, mut rx) = ends(links);
            barrier.wait();
            let before = thread_allocs();
            point::echo(&mut rx, &mut tx, &mut || !stop.load(Ordering::Acquire))?;
            Ok(thread_allocs() - before)
        });
        let ping = scope.spawn(|| -> Result<Ends, Error> {
            point::pin(c.cpu(0))?;
            let mut links = Vec::with_capacity(usize_of(n)?);
            for i in 0..n {
                let sock = TcpStream::connect(at)
                    .map_err(|e| Error::Transport(format!("local connect stream {i}: {e}")))?;
                links.push(client_link(sock, i, a, tls_ping.as_ref().as_ref())?);
            }
            barrier.wait();
            let (t, rt) = point::start_at(&clock, START_DELAY_NS);
            let mut rec = Recorder::new(&clock, t, c)?;
            let (mut tx, mut rx) = ends(links);
            let before = thread_allocs();
            let send = point::ping(&clock, c, t, &mut tx, &mut rx, &mut rec)?;
            Ok(Ends {
                rec,
                send,
                allocs: thread_allocs() - before,
                start_realtime_ns: rt,
            })
        });
        let ping_r = ping
            .join()
            .map_err(|_| Error::Transport("the ping thread panicked".into()))?;
        stop.store(true, Ordering::Release);
        let echo_r = echo
            .join()
            .map_err(|_| Error::Transport("the echo thread panicked".into()))?;
        match (ping_r, echo_r) {
            (Ok(mut ends), Ok(echo_allocs)) => {
                ends.allocs += echo_allocs;
                finish(c, a, &clock, ends, clock.raw())
            }
            (Err(ping_e), Err(echo_e)) => {
                Err(Error::Transport(format!("ping: {ping_e}; echo: {echo_e}")))
            }
            (Err(ping_e), Ok(_)) => Err(ping_e),
            (Ok(_), Err(echo_e)) => Err(echo_e),
        }
    })
}

/// One-way local: the receiver accepts, the sender connects, on a fresh port.
fn local_oneway(c: &Common, a: &WstlsArgs) -> Result<String, Error> {
    let listener = listen(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    let at = listener
        .local_addr()
        .map_err(|e| Error::Transport(format!("local addr: {e}")))?;
    let tls_recv = Arc::new(tls_pair(a.echo)?);
    let tls_send = Arc::clone(&tls_recv);
    let clock = TscClock::new();
    let barrier = Barrier::new(2);
    let start = std::sync::atomic::AtomicU64::new(0);
    let start_rt = std::sync::atomic::AtomicU64::new(0);
    let progress = Progress::default();
    let n = c.streams;
    std::thread::scope(|scope| {
        let receiver = scope.spawn(|| -> Result<(Recorder, u64), Error> {
            point::pin(c.cpu(1))?;
            let mut links = Vec::with_capacity(usize_of(n)?);
            for i in 0..n {
                let (sock, _) = listener
                    .accept()
                    .map_err(|e| Error::Timeout(format!("oneway accept stream {i}: {e}")))?;
                links.push(server_link(sock, i, a, tls_recv.as_ref().as_ref())?);
            }
            let (_tx, mut rx) = ends(links);
            barrier.wait();
            barrier.wait();
            let t = start.load(Ordering::Acquire);
            let mut rec = Recorder::new(&clock, t, c).inspect_err(|_| progress.fail())?;
            let before = thread_allocs();
            point::recv_oneway(&clock, c, t, &mut rx, &mut rec, &progress)?;
            Ok((rec, thread_allocs() - before))
        });
        let sender = scope.spawn(|| -> Result<(SendStats, u64), Error> {
            point::pin(c.cpu(0))?;
            let mut links = Vec::with_capacity(usize_of(n)?);
            for i in 0..n {
                let sock = TcpStream::connect(at)
                    .map_err(|e| Error::Transport(format!("oneway connect stream {i}: {e}")))?;
                links.push(client_link(sock, i, a, tls_send.as_ref().as_ref())?);
            }
            let (mut tx, _rx) = ends(links);
            barrier.wait();
            let (t, rt) = point::start_at(&clock, START_DELAY_NS);
            let rt_u = u64::try_from(rt)
                .map_err(|_| Error::Transport("the start's realtime does not fit a u64".into()))?;
            start.store(t, Ordering::Release);
            start_rt.store(rt_u, Ordering::Release);
            barrier.wait();
            let before = thread_allocs();
            let send = point::send_oneway(&clock, c, t, &mut tx, &progress)?;
            Ok((send, thread_allocs() - before))
        });
        let from_sender = sender
            .join()
            .map_err(|_| Error::Transport("the sender thread panicked".into()))?;
        let from_receiver = receiver
            .join()
            .map_err(|_| Error::Transport("the receiver thread panicked".into()))?;
        let ((stats, sent_allocs), (record, recv_allocs)) =
            point::both(from_sender, from_receiver)?;
        let ends = Ends {
            rec: record,
            send: stats,
            allocs: sent_allocs + recv_allocs,
            start_realtime_ns: u128::from(start_rt.load(Ordering::Acquire)),
        };
        finish(c, a, &clock, ends, clock.raw())
    })
}

struct Ends {
    rec: Recorder,
    send: SendStats,
    allocs: u64,
    start_realtime_ns: u128,
}

fn measure(
    c: &Common,
    a: &WstlsArgs,
    links: Vec<Rc<RefCell<Shared>>>,
    echo_allocs: Option<u64>,
) -> Result<String, Error> {
    let clock = TscClock::new();
    point::pin(c.cpu(0))?;
    let (t, rt) = point::start_at(&clock, START_DELAY_NS);
    let mut rec = Recorder::new(&clock, t, c)?;
    let (mut tx, mut rx) = ends(links);
    let a0 = thread_allocs();
    let send = point::ping(&clock, c, t, &mut tx, &mut rx, &mut rec)?;
    let mut allocs = thread_allocs() - a0;
    allocs += echo_allocs.unwrap_or(0);
    let ends = Ends {
        rec,
        send,
        allocs,
        start_realtime_ns: rt,
    };
    finish(c, a, &clock, ends, clock.raw())
}

fn finish(
    c: &Common,
    a: &WstlsArgs,
    clock: &TscClock,
    ends: Ends,
    end: u64,
) -> Result<String, Error> {
    let mut manifest = RunManifest::new(
        a.echo.name(),
        &ergon_latency_core::message::template(),
        MESSAGE_LEN as u64,
    );
    manifest
        .setting("transport", a.echo.name())
        .setting("backend", a.backend.name())
        .setting("impl", "ergon")
        .setting("echo", a.echo.name())
        .setting("nodelay", &a.nodelay.to_string())
        .setting("quickack_per_read", &a.quickack.to_string())
        .setting("busy_read_us", &a.busy_read_us.to_string())
        .setting("rcvbuf", &a.rcvbuf.to_string())
        .setting("sndbuf", &a.sndbuf.to_string())
        .setting(
            "rto_min_us",
            &a.rto_min_us
                .map_or_else(|| "kernel".to_owned(), |v| v.to_string()),
        )
        .setting(
            "delack_max_us",
            &a.delack_max_us
                .map_or_else(|| "kernel".to_owned(), |v| v.to_string()),
        );
    let wire = match a.echo {
        Echo::Tcp => super::ip_overhead(false) + 20 + 4 + MESSAGE_LEN as u64,
        Echo::Wstls => super::ip_overhead(false) + 20 + 5 + 6 + MESSAGE_LEN as u64,
    };
    point::finish(
        c,
        clock,
        ends.rec,
        ends.send,
        ends.start_realtime_ns,
        end,
        Report {
            channel: a.echo.name(),
            retrans: None,
            wire_bytes: wire,
            busy_threads: if c.role == Role::Local { 2 } else { 1 },
            hot_path_allocs: ends.allocs,
            manifest,
        },
    )
}

fn usize_of(n: u32) -> Result<usize, Error> {
    usize::try_from(n).map_err(|_| Error::Usage(format!("--streams {n}: does not fit")))
}

fn listen(at: SocketAddr) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(at).map_err(|e| Error::Transport(format!("bind {at}: {e}")))?;
    socket2::SockRef::from(&listener)
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| Error::Transport(format!("listen timeout: {e}")))?;
    Ok(listener)
}

fn connect_one(a: &WstlsArgs, stream: u32) -> Result<TcpStream, Error> {
    TcpStream::connect(a.peer)
        .map_err(|e| Error::Transport(format!("connect stream {stream} {}: {e}", a.peer)))
}

fn tune(sock: &TcpStream, a: &WstlsArgs) -> Result<(), Error> {
    sock.set_nodelay(a.nodelay)
        .map_err(|e| Error::Transport(format!("TCP_NODELAY: {e}")))?;
    let s = socket2::SockRef::from(sock);
    let recv =
        usize::try_from(a.rcvbuf).map_err(|_| Error::Transport("SO_RCVBUF does not fit".into()))?;
    let send =
        usize::try_from(a.sndbuf).map_err(|_| Error::Transport("SO_SNDBUF does not fit".into()))?;
    s.set_recv_buffer_size(recv)
        .map_err(|e| Error::Transport(format!("SO_RCVBUF: {e}")))?;
    s.set_send_buffer_size(send)
        .map_err(|e| Error::Transport(format!("SO_SNDBUF: {e}")))?;
    // Only when the runner passed the flag. Applying quick-ACK here would
    // change the Mac loopback smoke, which leaves the flag at its default.
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let fd = sock.as_raw_fd();
        if let Some(us) = a.rto_min_us {
            super::sockopt::set_tcp_timer(fd, super::sockopt::TCP_RTO_MIN_US, us)?;
        }
        if let Some(us) = a.delack_max_us {
            super::sockopt::set_tcp_timer(fd, super::sockopt::TCP_DELACK_MAX_US, us)?;
        }
    }
    Ok(())
}

type TlsPair = Option<(Arc<ServerConfig>, Arc<ClientConfig>)>;

/// Bench certificate (P-256, SAN `localhost`). The same bytes in every
/// process: a ping on one host trusts the pong on the other.
const BENCH_CERT_PEM: &str = "\
-----BEGIN CERTIFICATE-----
MIIBlDCCATqgAwIBAgIUR3SoADnFwZAcFDmyh7+8ArjWwf8wCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MTAyOTAxMTE1MVoXDTM2MTAyNjAx
MTE1MVowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0D
AQcDQgAEgSntyQgZc713aBLK5KqGINo8EwZnwFPOrPWdxS3IMpEYu/3CktVExB2X
XbDY1Lp9+VM38rWLBHWrVJQcsHqEQ6NqMGgwHQYDVR0OBBYEFGsWoCJ5W7LAuRpo
Ayt8yrePaPpQMAwGA1UdEwEB/wQCMAAwFAYDVR0RBA0wC4IJbG9jYWxob3N0MA4G
A1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDATAKBggqhkjOPQQDAgNI
ADBFAiAnPeM1K41VFQuPtmOsZZNjnUMeJAGsmL0ql60NbtLVggIhAMONWTYDF4u0
r96gAeO14i2pTdJ/zI6pAvA5z1rWQZG8
-----END CERTIFICATE-----
";

/// PKCS#8 key for [`BENCH_CERT_PEM`]. A bench key, not a secret.
const BENCH_KEY_PEM: &str = "\
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgji2UAMYYfoYkCj71
G/TOMhcLV0UDvQAAK7gLHX/isF2hRANCAASBKe3JCBlzvXdoEsrkqoYg2jwTBmfA
U86s9Z3FLcgykRi7/cKS1UTEHZddsNjUun35UzfytYsEdatUlByweoRD
-----END PRIVATE KEY-----
";

fn tls_pair(echo: Echo) -> Result<TlsPair, Error> {
    if echo == Echo::Tcp {
        return Ok(None);
    }
    let cert = CertificateDer::from_pem_slice(BENCH_CERT_PEM.as_bytes())
        .map_err(|e| Error::Transport(format!("certificate: {e}")))?;
    let key = PrivateKeyDer::from_pem_slice(BENCH_KEY_PEM.as_bytes())
        .map_err(|e| Error::Transport(format!("certificate key: {e}")))?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server = ServerConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| Error::Transport(format!("tls: {e}")))?
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], key)
        .map_err(|e| Error::Transport(format!("tls cert: {e}")))?;
    let mut roots = RootCertStore::empty();
    roots
        .add(cert)
        .map_err(|e| Error::Transport(format!("tls root: {e}")))?;
    let mut client = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| Error::Transport(format!("tls: {e}")))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    // The bypass dial's record layer reads the traffic secrets. No ALPN:
    // the kernel peer's server config does not offer one.
    client.enable_secret_extraction = true;
    Ok(Some((Arc::new(server), Arc::new(client))))
}

/// The TLS 1.3 client that trusts [`BENCH_CERT_PEM`].
///
/// Secret extraction is on: `WsDial::on_stack` refuses a client that cannot
/// feed the record layer. No ALPN: the kernel peer's server config does not
/// offer one. TLS 1.3 only.
///
/// # Errors
/// The baked certificate does not load.
pub fn bench_client_config() -> Result<Arc<ClientConfig>, Error> {
    match tls_pair(Echo::Wstls)? {
        Some((_, client)) => Ok(client),
        None => Err(Error::Transport(
            "websocket client config: the bench certificate did not load".into(),
        )),
    }
}

enum Io {
    Plain(TcpStream),
    Client(StreamOwned<ClientConnection, TcpStream>),
    Server(StreamOwned<ServerConnection, TcpStream>),
}

impl Io {
    fn set_nonblocking(&self) -> Result<(), Error> {
        let sock = match self {
            Self::Plain(s) => s,
            Self::Client(s) => s.get_ref(),
            Self::Server(s) => s.get_ref(),
        };
        sock.set_nonblocking(true)
            .map_err(|e| Error::Transport(format!("nonblocking: {e}")))
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Client(s) => s.read(buf),
            Self::Server(s) => s.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Client(s) => s.write(buf),
            Self::Server(s) => s.write(buf),
        }
    }
}

struct Shared {
    io: Io,
    inbox: Vec<u8>,
    outbox: Vec<u8>,
    closed: bool,
    echo: Echo,
    /// Outbound WebSocket frames are masked (the client).
    mask_out: bool,
    /// Inbound WebSocket frames are masked (the server's view of the client).
    expect_mask: bool,
    mask: [u8; 4],
}

impl Shared {
    fn fill(&mut self) -> Result<(), Error> {
        let mut tmp = [0u8; 2048];
        for _ in 0..8 {
            match self.io.read(&mut tmp) {
                Ok(0) => {
                    self.closed = true;
                    return Ok(());
                }
                Ok(n) => self.inbox.extend_from_slice(&tmp[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                // Frames are length-prefixed, so a TCP close without
                // `close_notify` is the peer leaving. rustls reports that as
                // `UnexpectedEof` instead of `Ok(0)`.
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    self.closed = true;
                    return Ok(());
                }
                Err(e) => return Err(Error::Transport(format!("read: {e}"))),
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Error> {
        while !self.outbox.is_empty() {
            match self.io.write(&self.outbox) {
                Ok(0) => return Err(Error::Transport("write wrote nothing".into())),
                Ok(n) => {
                    self.outbox.drain(..n);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(Error::Transport(format!("write: {e}"))),
            }
        }
        Ok(())
    }

    fn enqueue(&mut self, msg: &[u8; MESSAGE_LEN]) -> Result<(), Error> {
        match self.echo {
            Echo::Tcp => {
                let len = u32::try_from(MESSAGE_LEN).map_err(|_| {
                    Error::Transport("the message length does not fit a u32".into())
                })?;
                self.outbox.extend_from_slice(&len.to_le_bytes());
                self.outbox.extend_from_slice(msg);
            }
            Echo::Wstls => {
                let mut frame = [0u8; 96];
                let n = ergon_io::ws::encode_frame(
                    0x2,
                    msg,
                    self.mask_out.then_some(self.mask),
                    &mut frame,
                )
                .map_err(|e| Error::Transport(format!("websocket: {e}")))?;
                self.outbox.extend_from_slice(&frame[..n]);
            }
        }
        Ok(())
    }

    fn pop(&mut self) -> Result<Option<[u8; MESSAGE_LEN]>, Error> {
        match self.echo {
            Echo::Tcp => pop_tcp(&mut self.inbox),
            Echo::Wstls => pop_ws(&mut self.inbox, self.expect_mask),
        }
    }
}

fn pop_tcp(buf: &mut Vec<u8>) -> Result<Option<[u8; MESSAGE_LEN]>, Error> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let n = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let Ok(n) = usize::try_from(n) else {
        return Err(Error::Transport(format!("tcp length {n} does not fit")));
    };
    if n != MESSAGE_LEN {
        return Err(Error::Transport(format!(
            "tcp length {n}, want {MESSAGE_LEN}"
        )));
    }
    if buf.len() < 4 + MESSAGE_LEN {
        return Ok(None);
    }
    let mut msg = [0u8; MESSAGE_LEN];
    msg.copy_from_slice(&buf[4..4 + MESSAGE_LEN]);
    buf.drain(..4 + MESSAGE_LEN);
    Ok(Some(msg))
}

fn pop_ws(buf: &mut Vec<u8>, expect_mask: bool) -> Result<Option<[u8; MESSAGE_LEN]>, Error> {
    if buf.len() < 2 {
        return Ok(None);
    }
    let opcode = buf[0] & 0x0f;
    let masked = buf[1] & 0x80 != 0;
    let mut len = usize::from(buf[1] & 0x7f);
    let mut hdr = 2usize;
    if len == 126 {
        if buf.len() < 4 {
            return Ok(None);
        }
        len = usize::from(u16::from_be_bytes([buf[2], buf[3]]));
        hdr = 4;
    } else if len == 127 {
        return Err(Error::Transport(
            "websocket frame longer than 64 KiB".into(),
        ));
    }
    if masked {
        hdr += 4;
    }
    if buf.len() < hdr + len {
        return Ok(None);
    }
    if opcode != 0x2 {
        return Err(Error::Transport(format!(
            "websocket opcode {opcode}, want binary"
        )));
    }
    if masked != expect_mask {
        return Err(Error::Transport(
            "websocket mask bit is not the peer's".into(),
        ));
    }
    if len != MESSAGE_LEN {
        return Err(Error::Transport(format!(
            "websocket length {len}, want {MESSAGE_LEN}"
        )));
    }
    let mut msg = [0u8; MESSAGE_LEN];
    msg.copy_from_slice(&buf[hdr..hdr + MESSAGE_LEN]);
    if masked {
        let key = [buf[hdr - 4], buf[hdr - 3], buf[hdr - 2], buf[hdr - 1]];
        for (i, b) in msg.iter_mut().enumerate() {
            *b ^= key[i % 4];
        }
    }
    buf.drain(..hdr + len);
    Ok(Some(msg))
}

fn blank(io: Io, echo: Echo, mask_out: bool) -> Rc<RefCell<Shared>> {
    let mut inbox = Vec::new();
    let mut outbox = Vec::new();
    inbox.reserve(8192);
    outbox.reserve(8192);
    Rc::new(RefCell::new(Shared {
        io,
        inbox,
        outbox,
        closed: false,
        echo,
        mask_out,
        expect_mask: !mask_out && echo == Echo::Wstls,
        mask: [0x5a, 0xa5, 0x3c, 0xc3],
    }))
}

fn server_link(
    sock: TcpStream,
    _stream: u32,
    a: &WstlsArgs,
    tls: Option<&(Arc<ServerConfig>, Arc<ClientConfig>)>,
) -> Result<Rc<RefCell<Shared>>, Error> {
    tune(&sock, a)?;
    let io = match (a.echo, tls) {
        (Echo::Tcp, _) => Io::Plain(sock),
        (Echo::Wstls, Some((server, _))) => {
            let conn = ServerConnection::new(Arc::clone(server))
                .map_err(|e| Error::Transport(format!("tls server: {e}")))?;
            let mut stream = StreamOwned::new(conn, sock);
            ws_server(&mut stream)?;
            Io::Server(stream)
        }
        (Echo::Wstls, None) => {
            return Err(Error::Transport("websocket needs a certificate".into()));
        }
    };
    let link = blank(io, a.echo, false);
    link.borrow_mut().io.set_nonblocking()?;
    Ok(link)
}

fn client_link(
    sock: TcpStream,
    _stream: u32,
    a: &WstlsArgs,
    tls: Option<&(Arc<ServerConfig>, Arc<ClientConfig>)>,
) -> Result<Rc<RefCell<Shared>>, Error> {
    tune(&sock, a)?;
    let io = match (a.echo, tls) {
        (Echo::Tcp, _) => Io::Plain(sock),
        (Echo::Wstls, Some((_, client))) => {
            let name = ServerName::try_from("localhost")
                .map_err(|e| Error::Transport(format!("tls name: {e}")))?;
            let conn = ClientConnection::new(Arc::clone(client), name)
                .map_err(|e| Error::Transport(format!("tls client: {e}")))?;
            let mut stream = StreamOwned::new(conn, sock);
            ws_client(&mut stream)?;
            Io::Client(stream)
        }
        (Echo::Wstls, None) => {
            return Err(Error::Transport("websocket needs a certificate".into()));
        }
    };
    let link = blank(io, a.echo, a.echo == Echo::Wstls);
    link.borrow_mut().io.set_nonblocking()?;
    Ok(link)
}

fn ws_client(stream: &mut impl ReadWrite) -> Result<(), Error> {
    let key = ergon_io::ws::client_key([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let req = ergon_io::ws::client_request("localhost", "/", &key, &[]);
    stream
        .write_all(&req)
        .map_err(|e| Error::Transport(format!("websocket request: {e}")))?;
    stream
        .flush()
        .map_err(|e| Error::Transport(format!("websocket request: {e}")))?;
    let mut buf = Vec::new();
    read_until(stream, &mut buf, |b| {
        ergon_io::ws::parse_client_response(b, &key)
            .map_err(|e| Error::Transport(format!("websocket response: {e}")))
    })?;
    Ok(())
}

fn ws_server(stream: &mut impl ReadWrite) -> Result<(), Error> {
    let mut buf = Vec::new();
    let key = read_until(
        stream,
        &mut buf,
        |b| match ergon_io::ws::parse_server_request(b) {
            Ok(None) => Ok(None),
            Ok(Some((_, key, _))) => Ok(Some(key)),
            Err(e) => Err(Error::Transport(format!("websocket request: {e}"))),
        },
    )?;
    let resp = ergon_io::ws::server_response(&key);
    stream
        .write_all(&resp)
        .map_err(|e| Error::Transport(format!("websocket response: {e}")))?;
    stream
        .flush()
        .map_err(|e| Error::Transport(format!("websocket response: {e}")))?;
    Ok(())
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn read_until<T>(
    stream: &mut impl Read,
    buf: &mut Vec<u8>,
    mut parse: impl FnMut(&[u8]) -> Result<Option<T>, Error>,
) -> Result<T, Error> {
    let mut tmp = [0u8; 1024];
    for _ in 0..64 {
        if let Some(v) = parse(buf)? {
            return Ok(v);
        }
        let n = stream
            .read(&mut tmp)
            .map_err(|e| Error::Transport(format!("handshake read: {e}")))?;
        if n == 0 {
            return Err(Error::Transport("handshake closed".into()));
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    Err(Error::Timeout("handshake did not finish".into()))
}

struct LinkTx {
    links: Vec<Rc<RefCell<Shared>>>,
}

struct LinkRx {
    links: Vec<Rc<RefCell<Shared>>>,
    closed: bool,
}

fn ends(links: Vec<Rc<RefCell<Shared>>>) -> (LinkTx, LinkRx) {
    (
        LinkTx {
            links: links.clone(),
        },
        LinkRx {
            links,
            closed: false,
        },
    )
}

impl Tx for LinkTx {
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        let Ok(i) = usize::try_from(stream) else {
            return Err(Error::Transport(format!("stream {stream} does not fit")));
        };
        let Some(link) = self.links.get(i) else {
            return Err(Error::Transport(format!(
                "no connection for stream {stream}"
            )));
        };
        let mut shared = link.borrow_mut();
        if shared.closed {
            return Err(Error::Transport("the peer closed".into()));
        }
        if shared.outbox.is_empty() {
            shared.enqueue(msg)?;
        }
        shared.flush()?;
        if shared.outbox.is_empty() {
            Ok(Sent::Sent)
        } else {
            Ok(Sent::Busy)
        }
    }
}

impl Rx for LinkRx {
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let mut batch = [[0u8; MESSAGE_LEN]; POLL_LIMIT];
        let mut n = 0usize;
        let mut open = false;
        for link in &self.links {
            let mut shared = link.borrow_mut();
            shared.fill()?;
            if !shared.closed {
                open = true;
            }
            while n < POLL_LIMIT {
                match shared.pop()? {
                    Some(m) => {
                        batch[n] = m;
                        n += 1;
                    }
                    None => break,
                }
            }
        }
        self.closed = !open && !self.links.is_empty();
        for m in &batch[..n] {
            on(m);
        }
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.closed
    }
}
