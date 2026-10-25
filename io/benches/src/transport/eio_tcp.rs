//! ergon-io framed TCP on the epoll-spin backend: one `FrameConn` per
//! stream on one `Driver`, the same u32 LE length frames as
//! `ergon_latency_core::frame`.
//!
//! - Receive ([`EioTcpRx`]): `Driver::poll`, one read per ready
//!   connection a pass; each frame is handed over in
//!   `FrameHandler::on_frame`, in place in the connection's receive buffer
//!   ([`TCP_RX_BUF`](super::TCP_RX_BUF) bytes, as the mio arm's). Every
//!   whole frame a read brings is delivered in that pass (ergon-io has no
//!   per-pass frame limit). A peer's close is counted; [`Rx::closed`] once
//!   every connection is gone.
//! - Send ([`EioTcpTx`]): `ConnMut::send_frame` queues header and payload
//!   ([`TCP_TX_BUF`](super::TCP_TX_BUF)), then `Driver::flush` writes at
//!   once. A full queue is busy.
//! - Echo: as the UDP arm, staged while the driver polls and sent right
//!   after that poll.
//! - Driver keys: `rcvbuf`, `epoll_events`, `tcp_rto_min_us`,
//!   `tcp_delack_max_us` (checked against the kernel at `Driver::new`);
//!   `FrameOptions::quickack_per_read`, `sndbuf`. `TCP_NODELAY` always.
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
use std::net::TcpStream;
use std::os::fd::AsRawFd;
use std::rc::Rc;

use ergon_io::{ConnId, Driver, FrameConn, FrameHandler, FrameOptions, Handler, RxMeta};
use ergon_latency_core::frame::FRAME_HEADER;
use ergon_latency_core::message::MESSAGE_LEN;

use super::eio_udp::{Staged, base_config, eio};
use super::mio_tcp::tcp_wire_bytes;
use super::point::{Error, Rx, Sent, Tx};
use super::sock::Inspect;
use super::sockopt::{self, TcpCounters, os};
use super::{MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF};

/// The ergon-io TCP arm's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EioTcpOptions {
    /// `Config::rcvbuf`, bytes.
    pub rcvbuf: u32,
    /// `FrameOptions::sndbuf`; `None`: the kernel's.
    pub sndbuf: Option<u32>,
    /// `Config::epoll_events`.
    pub epoll_events: u32,
    /// `FrameOptions::quickack_per_read`.
    pub quickack_per_read: bool,
    /// `Config::tcp_rto_min_us`.
    pub rto_min_us: Option<u32>,
    /// `Config::tcp_delack_max_us`.
    pub delack_max_us: Option<u32>,
    /// `Config::loop_cpu`.
    pub loop_cpu: Option<u32>,
    /// `Config::helper_cpus` (empty: ergon-io chooses).
    pub helper_cpus: Vec<u32>,
}

struct Shared {
    d: RefCell<Driver<FrameConn>>,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
    /// Each stream's socket through a second descriptor: its `TCP_INFO`
    /// and options stay readable once the driver has closed its own (a
    /// peer close), as the mio arm's sockets do. Read off the hot path only.
    held: Vec<TcpStream>,
    opts: EioTcpOptions,
    ipv6: bool,
}

/// The sending side.
pub struct EioTcpTx {
    s: Rc<Shared>,
}

/// The receiving side (it owns the driver's poll).
pub struct EioTcpRx {
    s: Rc<Shared>,
    gone: usize,
}

/// Takes connected sockets, stream `i` the `i`-th, onto one driver; their
/// sending and receiving sides. Call it on the polling thread, pinned.
///
/// # Errors
/// `Driver::new` (a key out of range, a timer the kernel refuses or
/// rounds), or adding a connection fails.
pub fn pair(socks: Vec<TcpStream>, opts: EioTcpOptions) -> Result<(EioTcpTx, EioTcpRx), Error> {
    if socks.is_empty() {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let ipv6 = socks[0]
        .local_addr()
        .map_err(|e| os("getsockname", e))?
        .is_ipv6();
    let mut cfg = base_config(
        opts.rcvbuf,
        opts.epoll_events,
        opts.loop_cpu,
        &opts.helper_cpus,
    );
    cfg.tcp_rto_min_us = opts.rto_min_us;
    cfg.tcp_delack_max_us = opts.delack_max_us;
    cfg.validate().map_err(|e| eio("config", e))?;
    let mut d: Driver<FrameConn> = Driver::new(cfg).map_err(|e| eio("Driver::new", e))?;
    let fopts = FrameOptions {
        max_frame: MAX_FRAME,
        rx_buf: TCP_RX_BUF,
        tx_buf: TCP_TX_BUF,
        sndbuf: opts.sndbuf,
        quickack_per_read: opts.quickack_per_read,
    };
    let n = socks.len();
    let mut ids = Vec::with_capacity(n);
    let mut held = Vec::with_capacity(n);
    for s in socks {
        held.push(s.try_clone().map_err(|e| os("dup", e))?);
        let c = FrameConn::from_stream(s, fopts).map_err(|e| eio("FrameConn", e))?;
        ids.push(d.add(c).map_err(|e| eio("Driver::add", e))?);
    }
    // one read per ready connection per poll, of at most the receive
    // buffer: at most this many whole frames
    let per_read = TCP_RX_BUF / (FRAME_HEADER + MESSAGE_LEN) + 1;
    let per_poll = (opts.epoll_events as usize).min(n) * per_read;
    let s = Rc::new(Shared {
        d: RefCell::new(d),
        staged: RefCell::new(Staged::new(per_poll)),
        ids,
        held,
        opts,
        ipv6,
    });
    Ok((EioTcpTx { s: Rc::clone(&s) }, EioTcpRx { s, gone: 0 }))
}

/// One frame queued and flushed now; `Ok(false)` when busy.
#[inline]
fn send_now(
    d: &mut Driver<FrameConn>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = *ids
        .get(stream as usize)
        .ok_or_else(|| Error::Transport(format!("stream {stream} of {}", ids.len())))?;
    let mut c = d
        .get_mut(id)
        .ok_or_else(|| Error::Transport(format!("stream {stream}: its connection closed")))?;
    match c.send_frame(msg) {
        Ok(()) => {
            d.flush().map_err(|e| eio("flush", e))?;
            Ok(true)
        }
        Err(ergon_io::Error::Busy(_)) => Ok(false),
        Err(e) => Err(eio("send_frame", e)),
    }
}

impl Tx for EioTcpTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        // inside the receive side's poll (an echo): staged, sent after it
        let Ok(mut d) = self.s.d.try_borrow_mut() else {
            return Ok(if self.s.staged.borrow_mut().push(stream, msg) {
                Sent::Sent
            } else {
                Sent::Busy
            });
        };
        Ok(if send_now(&mut d, &self.s.ids, stream, msg)? {
            Sent::Sent
        } else {
            Sent::Busy
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
            e => {
                if self.err.is_none() {
                    self.err = Some(Error::Transport(format!("tcp {id:?} closed: {e}")));
                }
            }
        }
    }
}

impl<F: FnMut(&[u8])> FrameHandler for Deliver<'_, F> {
    #[inline]
    fn on_frame(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        (self.on)(payload);
    }
}

impl Rx for EioTcpRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let s = &*self.s;
        let mut d = s.d.borrow_mut();
        let mut h = Deliver {
            on,
            gone: 0,
            err: None,
        };
        let n = d.poll(&mut h).map_err(|e| eio("poll", e))?;
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

impl Inspect for EioTcpRx {
    fn tcp_counters(&self) -> Result<Option<TcpCounters>, Error> {
        TcpCounters::of(self.s.held.iter().map(AsRawFd::as_raw_fd)).map(Some)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        let ts = match self.s.held.first() {
            Some(s) => sockopt::tcp_info(s.as_raw_fd())?.timestamps,
            None => false,
        };
        Ok(tcp_wire_bytes(ts, self.s.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let d = self.s.d.borrow();
        let cfg = d.config();
        let opt = |v: Option<u32>| v.map_or_else(|| "kernel".to_owned(), |b| b.to_string());
        let (mut frames_in, mut frames_out, mut bytes_in, mut quickacks, mut compact) =
            (0u64, 0u64, 0u64, 0u64, 0u64);
        for c in self.s.ids.iter().filter_map(|&id| d.get(id)) {
            let s = c.stats();
            frames_in += s.frames_in;
            frames_out += s.frames_out;
            bytes_in += s.bytes_in;
            quickacks += s.quickacks;
            compact += s.compact_bytes;
        }
        let ds = d.stats();
        for (k, v) in [
            ("tcp.max_frame", MAX_FRAME.to_string()),
            ("tcp.rx_buf", TCP_RX_BUF.to_string()),
            ("tcp.tx_buf", TCP_TX_BUF.to_string()),
            ("tcp.frames_per_pass", "one read".to_owned()),
            ("nodelay", "true".to_owned()),
            (
                "quickack_per_read",
                self.s.opts.quickack_per_read.to_string(),
            ),
            ("rto_min_us", opt(cfg.tcp_rto_min_us)),
            ("delack_max_us", opt(cfg.tcp_delack_max_us)),
            ("rcvbuf", opt(cfg.rcvbuf)),
            ("sndbuf", opt(self.s.opts.sndbuf)),
            ("epoll_events", cfg.epoll_events.to_string()),
            ("ergon.backend", cfg.backend.to_string()),
            ("ergon.rx_timestamps", cfg.rx_timestamps.to_string()),
            ("ergon.keepalive_idle_s", cfg.keepalive_idle_s.to_string()),
            (
                "ergon.tcp_user_timeout_ms",
                cfg.tcp_user_timeout_ms.to_string(),
            ),
            ("ergon.quickack_check", format!("{:?}", cfg.quickack_check)),
            ("ergon.helper_cpus", format!("{:?}", cfg.helper_cpus)),
            ("stat.frames_in", frames_in.to_string()),
            ("stat.frames_out", frames_out.to_string()),
            ("stat.bytes_in", bytes_in.to_string()),
            ("stat.quickacks", quickacks.to_string()),
            ("stat.compact_bytes", compact.to_string()),
            ("stat.polls", ds.polls.to_string()),
            ("stat.recv_calls", ds.recvmsg_calls.to_string()),
            ("stat.epoll_wait_calls", ds.epoll_wait_calls.to_string()),
            ("stat.send_calls", ds.send_calls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(s) = self.s.held.first() {
            sockopt::read_back(s.as_raw_fd(), true, out)?;
        }
        Ok(())
    }
}
