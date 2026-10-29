//! `fstack_spin_lat`: ergon-io's `fstack-spin` measuring loop.
//!
//! One process, one F-Stack. The lab runs ping on host A (DPDK, `fstack-up`)
//! and pong on host B (the kernel, where loss is applied). The ping connects
//! with `Driver<FrameConn<FstackTcp>>` and the same `point::ping` as
//! `tcp_lat`. The pong is a kernel `epoll-spin` echo (`eio_tcp`): host B's
//! NIC stays in the kernel, so this process does not start F-Stack there and
//! does not label that socket `fstack-spin`.
//!
//! Frames are `ergon_latency_core::frame`'s u32 LE length plus the 64-byte
//! message, the same bytes `eio_tcp` echoes. Connections are opened in order
//! on one listener; there is no stream-index preamble, because `FrameConn`
//! owns the byte stream.
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
use std::net::{IpAddr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use std::rc::Rc;
use std::time::{Duration, Instant};

use ergon_io::fstack::FstackTcp;
use ergon_io::{
    Backend, ConnId, Driver, FrameConn, FrameDial, FrameHandler, FrameOptions, Handler, RxMeta,
    Target,
};
use ergon_latency_core::message::MESSAGE_LEN;

use super::eio_tcp::{self, EioTcpOptions};
use super::eio_udp::{self, Staged};
use super::fstack_conf::{FstackArgs, spin_config};
use super::mio_tcp::tcp_wire_bytes;
use super::point::{Common, Error, Flags, Role};
use super::sock::{self, End, Impl, Inspect};
use super::sockopt;
use super::{MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF, tcp_echo_capacity};

/// Parses `args` (without the program name) and runs the point. The row, or
/// `None` for a pong.
///
/// # Errors
/// A flag refused, F-Stack not linked or not started, or what the point met.
pub fn main<I: IntoIterator<Item = String>>(args: I) -> Result<Option<String>, Error> {
    let mut f = Flags::parse(args)?;
    let c = Common::parse(&mut f)?;
    let a = FstackArgs::parse(&mut f, c.role, &c.cpus)?;
    f.finish()?;
    if a.imp != super::fstack_conf::FstackImpl::Ergon {
        return Err(Error::Usage(
            "--impl ergon is this binary (fstack-spin). The direct libfstack path is fstack_lat"
                .into(),
        ));
    }
    measure(&c, &a)
}

fn measure(c: &Common, a: &FstackArgs) -> Result<Option<String>, Error> {
    let opt = |v: Option<SocketAddrV4>| v.map_or_else(|| "-".to_owned(), |s| s.to_string());
    let arm = vec![
        (
            "backend".to_owned(),
            if c.role == Role::Pong {
                "epoll-spin".to_owned()
            } else {
                a.imp.backend().to_owned()
            },
        ),
        (
            "stack".to_owned(),
            if c.role == Role::Pong {
                "kernel-peer".to_owned()
            } else {
                "fstack".to_owned()
            },
        ),
        ("listen".to_owned(), opt(a.listen)),
        ("connect".to_owned(), opt(a.connect)),
        ("source".to_owned(), a.conf.addr.to_string()),
        (
            "rexmit_min_ms".to_owned(),
            a.conf
                .rexmit_min_ms
                .map_or_else(|| "-".to_owned(), |ms| ms.to_string()),
        ),
    ];
    match c.role {
        Role::Pong => kernel_pong(c, a, &arm),
        Role::Ping => fstack_ping(c, a, &arm),
        Role::Local | Role::OnewaySend | Role::OnewayRecv => Err(Error::Usage(format!(
            "--role {}: fstack_spin_lat is ping or pong (one F-Stack per process)",
            c.role.name()
        ))),
    }
}

fn kernel_pong(
    c: &Common,
    a: &FstackArgs,
    arm: &[(String, String)],
) -> Result<Option<String>, Error> {
    let listen = a
        .listen
        .ok_or_else(|| Error::Usage("pong: --listen IP:PORT (the kernel peer)".into()))?;
    let listener = TcpListener::bind(SocketAddr::V4(listen))
        .map_err(|e| sockopt::os(&format!("listen {listen}"), e))?;
    println!("ready");
    let streams = c.streams;
    let wait = a.connect_timeout;
    let base = kernel_opts(a);
    let listener = &listener;
    sock::run(c, Impl::Ergon, "tcp", arm, true, move |end, cpu| {
        if end != End::Pong {
            return Err(Error::Usage("the kernel peer is the pong (host b)".into()));
        }
        let socks = accept_ordered(listener, streams, wait)?;
        let mut opts = base.clone();
        opts.loop_cpu = cpu.and_then(|n| u32::try_from(n).ok());
        eio_tcp::pair(socks, opts)
    })
}

const fn kernel_opts(a: &FstackArgs) -> EioTcpOptions {
    EioTcpOptions {
        rcvbuf: a.conf.rcvbuf,
        sndbuf: a.conf.sndbuf,
        epoll_events: 64,
        busy_poll_us: 0,
        // The arm's `--delayed-ack false` is FreeBSD's sysctl on the F-Stack
        // end. The kernel peer acknowledges at once the same way.
        quickack_per_read: !a.conf.delayed_ack,
        rto_min_us: a.rto_min_us,
        delack_max_us: None,
        loop_cpu: None,
        helper_cpus: Vec::new(),
        backend: Backend::EpollSpin,
        sqpoll_cpu: None,
    }
}

fn accept_ordered(listener: &TcpListener, n: u32, wait: Duration) -> Result<Vec<TcpStream>, Error> {
    if n == 0 {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    listener
        .set_nonblocking(true)
        .map_err(|e| sockopt::os("O_NONBLOCK", e))?;
    let until = Instant::now() + wait;
    let need = usize_of(n)?;
    let mut out = Vec::with_capacity(need);
    while out.len() < need {
        match listener.accept() {
            Ok((sock, _)) => out.push(sock),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > until {
                    return Err(Error::Timeout(format!(
                        "{} of {n} connections came within {} s",
                        out.len(),
                        wait.as_secs()
                    )));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => return Err(sockopt::os("accept", e)),
        }
    }
    Ok(out)
}

fn fstack_ping(
    c: &Common,
    a: &FstackArgs,
    arm: &[(String, String)],
) -> Result<Option<String>, Error> {
    let streams = c.streams;
    let wait = a.connect_timeout;
    std::fs::create_dir_all(&c.out)
        .map_err(|e| Error::Transport(format!("{}: {e}", c.out.display())))?;
    let path = c.out.join("fstack-spin.conf");
    sock::run(c, Impl::Ergon, "tcp", arm, false, move |_end, _cpu| {
        let cfg = spin_config(a, &path)?;
        let mut driver: Driver<FrameConn<FstackTcp>> =
            Driver::new(cfg).map_err(|e| eio_udp::eio("Driver::new", e))?;
        let opts = frame_opts(a);
        let n = usize_of(streams)?;
        let mut ids = Vec::with_capacity(n);
        let to = a
            .connect
            .ok_or_else(|| Error::Usage("ping: --source IP --connect IP:PORT".into()))?;
        for _ in 0..streams {
            let conn = connect_one(a, to, opts, wait)?;
            ids.push(
                driver
                    .add(conn)
                    .map_err(|e| eio_udp::eio("Driver::add", e))?,
            );
        }
        pair(driver, ids)
    })
}

const fn frame_opts(a: &FstackArgs) -> FrameOptions {
    FrameOptions {
        max_frame: MAX_FRAME,
        rx_buf: TCP_RX_BUF,
        tx_buf: TCP_TX_BUF,
        sndbuf: a.conf.sndbuf,
        quickack_per_read: false,
    }
}

fn connect_one(
    a: &FstackArgs,
    to: std::net::SocketAddrV4,
    opts: FrameOptions,
    wait: Duration,
) -> Result<FrameConn<FstackTcp>, Error> {
    let target = Target::new(to.ip().to_string(), to.port(), "/")
        .with_addr(SocketAddr::V4(to))
        .with_source(IpAddr::V4(a.conf.addr));
    let until = Instant::now() + wait;
    loop {
        match FrameDial::<FstackTcp>::on_stack(target.clone(), opts)
            .and_then(FrameDial::connect_blocking)
        {
            Ok(conn) => return Ok(conn),
            Err(_) if Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Err(eio_udp::eio(
                    &format!("connect {to} from {}", a.conf.addr),
                    e,
                ));
            }
        }
    }
}

fn usize_of(n: u32) -> Result<usize, Error> {
    usize::try_from(n).map_err(|_| Error::Usage(format!("--streams {n}: does not fit")))
}

struct Shared {
    d: RefCell<Driver<FrameConn<FstackTcp>>>,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
}

struct SpinTx {
    s: Rc<Shared>,
}

struct SpinRx {
    s: Rc<Shared>,
    gone: usize,
}

fn pair(driver: Driver<FrameConn<FstackTcp>>, ids: Vec<ConnId>) -> Result<(SpinTx, SpinRx), Error> {
    if ids.is_empty() {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let room = tcp_echo_capacity(ids.len(), ids.len(), false);
    let s = Rc::new(Shared {
        d: RefCell::new(driver),
        staged: RefCell::new(Staged::new(room)),
        ids,
    });
    Ok((SpinTx { s: Rc::clone(&s) }, SpinRx { s, gone: 0 }))
}

fn send_now(
    d: &mut Driver<FrameConn<FstackTcp>>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = *ids
        .get(usize_of(stream)?)
        .ok_or_else(|| Error::Transport(format!("stream {stream} of {}", ids.len())))?;
    let mut c = d
        .get_mut(id)
        .ok_or_else(|| Error::Transport(format!("stream {stream}: its connection closed")))?;
    match c.send_frame(msg) {
        Ok(()) => {
            d.flush().map_err(|e| eio_udp::eio("flush", e))?;
            Ok(true)
        }
        Err(ergon_io::Error::Busy(_)) => Ok(false),
        Err(e) => Err(eio_udp::eio("send_frame", e)),
    }
}

impl super::point::Tx for SpinTx {
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
        Ok(if send_now(&mut d, &self.s.ids, stream, msg)? {
            super::point::Sent::Sent
        } else {
            super::point::Sent::Busy
        })
    }
}

struct Deliver<'a, F> {
    on: &'a mut F,
    gone: usize,
    err: Option<Error>,
}

impl<F> Handler for Deliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        match error {
            ergon_io::Error::Closed => self.gone += 1,
            ergon_io::Error::Io(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                self.gone += 1;
            }
            e => {
                if self.err.is_none() {
                    self.err = Some(Error::Transport(format!("tcp {id:?} closed: {e}")));
                }
            }
        }
    }
}

impl<F: FnMut(&[u8])> FrameHandler for Deliver<'_, F> {
    fn on_frame(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        (self.on)(payload);
    }
}

impl super::point::Rx for SpinRx {
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let s = &*self.s;
        let mut d = s.d.borrow_mut();
        {
            let mut staged = s.staged.borrow_mut();
            if staged.len > 0 {
                d.flush().map_err(|e| eio_udp::eio("flush", e))?;
                staged.drain(|stream, m| send_now(&mut d, &s.ids, stream, m))?;
                if staged.len > 0 {
                    return Ok(0);
                }
            }
        }
        let mut h = Deliver {
            on,
            gone: 0,
            err: None,
        };
        let n = d.poll(&mut h).map_err(|e| eio_udp::eio("poll", e))?;
        if h.gone > 0 {
            self.gone += h.gone;
        }
        if let Some(e) = h.err {
            return Err(e);
        }
        let mut staged = s.staged.borrow_mut();
        if staged.len > 0 {
            staged.drain(|stream, m| send_now(&mut d, &s.ids, stream, m))?;
        }
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.gone >= self.s.ids.len()
    }
}

impl Inspect for SpinRx {
    fn wire_bytes(&self) -> Result<u64, Error> {
        // F-Stack's shipped sysctl turns `net.inet.tcp.rfc1323` on, so the
        // timestamp option is in the header. IPv4 only.
        Ok(tcp_wire_bytes(true, false))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let d = self.s.d.borrow();
        let cfg = d.config();
        let (mut frames_in, mut frames_out, mut bytes_in) = (0u64, 0u64, 0u64);
        for c in self.s.ids.iter().filter_map(|&id| d.get(id)) {
            let s = c.stats();
            frames_in += s.frames_in;
            frames_out += s.frames_out;
            bytes_in += s.bytes_in;
        }
        let ds = d.stats();
        let rexmit = cfg
            .fstack
            .sysctl
            .iter()
            .find(|(k, _)| k == "net.inet.tcp.rexmit_min")
            .map_or_else(|| "-".to_owned(), |(_, v)| v.clone());
        for (k, v) in [
            ("ergon.backend", cfg.backend.to_string()),
            ("fstack.rexmit_min_ms", rexmit),
            ("fstack.delayed_ack", cfg.fstack.delayed_ack.to_string()),
            (
                "fstack.addr",
                cfg.fstack
                    .addr
                    .map_or_else(|| "-".to_owned(), |a| a.to_string()),
            ),
            ("stat.frames_in", frames_in.to_string()),
            ("stat.frames_out", frames_out.to_string()),
            ("stat.bytes_in", bytes_in.to_string()),
            ("stat.polls", ds.polls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        Ok(())
    }
}
