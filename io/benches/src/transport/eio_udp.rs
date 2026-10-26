//! ergon-io UDP on the epoll-spin backend: one `UdpConn::connect` per
//! stream (stream `i` bound to local base port + `i`, connected to peer
//! base port + `i`) on one `Driver`.
//!
//! - Receive ([`EioUdpRx`]): `Driver::poll`; each datagram is handed over
//!   in `UdpHandler::on_datagram`, a slice of the connection's receive
//!   buffer. `datagram_burst` (default
//!   [`POLL_LIMIT`](super::point::POLL_LIMIT), the other arms' per-stream
//!   limit) caps the reads per socket per pass.
//! - Send ([`EioUdpTx`]): `ConnMut::send` copies into the connection's send
//!   slots, then `Driver::flush` hands it to the kernel at once. All slots
//!   full is busy.
//! - Echo: the handler has no driver to send with, so an echo made while
//!   the driver polls ([`Staged`]) goes out right after that poll, before
//!   `Rx::poll` returns.
//! - Driver keys: `epoll_events`, `datagram_burst`, `rcvbuf`,
//!   `busy_poll_us`; receive timestamps off (nothing reads them); keepalive and
//!   `TCP_USER_TIMEOUT` left alone (the mio arms set neither).
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
use std::net::SocketAddr;
use std::rc::Rc;

use ergon_io::{Backend, Config, ConnId, Driver, Handler, RxMeta, UdpConn, UdpHandler, UdpOptions};
use ergon_latency_core::message::MESSAGE_LEN;

use super::point::{Error, Rx, Sent, Tx};
use super::sock::{Inspect, port_plus};
use super::sockopt;
use super::{MAX_DATAGRAM, TX_SLOTS, ip_overhead};

/// The ergon-io UDP arm's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EioUdpOptions {
    /// `Config::rcvbuf`, bytes.
    pub rcvbuf: u32,
    /// `UdpOptions::sndbuf`; `None`: the kernel's.
    pub sndbuf: Option<u32>,
    /// `Config::epoll_events`.
    pub epoll_events: u32,
    /// `Config::datagram_burst`.
    pub datagram_burst: u32,
    /// `Config::busy_poll_us` (`SO_BUSY_POLL`, µs; 0: none).
    pub busy_poll_us: u32,
    /// `Config::loop_cpu`: the CPU the polling thread is pinned to.
    pub loop_cpu: Option<u32>,
    /// `Config::helper_cpus` (empty: ergon-io chooses).
    pub helper_cpus: Vec<u32>,
}

/// The driver configuration both ergon-io arms start from: epoll-spin,
/// receive timestamps off, keepalive and `TCP_USER_TIMEOUT` left as the
/// socket has them (off), as the mio arms leave them.
#[must_use]
pub fn base_config(
    rcvbuf: u32,
    epoll_events: u32,
    loop_cpu: Option<u32>,
    helper_cpus: &[u32],
) -> Config {
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.rcvbuf = Some(rcvbuf);
    cfg.rx_timestamps = false;
    cfg.rx_read_timestamp = false;
    cfg.keepalive_idle_s = 0;
    cfg.tcp_user_timeout_ms = 0;
    cfg.tcp_info_interval_ms = 0;
    cfg.epoll_events = epoll_events;
    cfg.loop_cpu = loop_cpu;
    cfg.helper_cpus = helper_cpus.to_vec();
    cfg
}

/// ergon-io's error as the point's.
#[must_use]
pub fn eio(what: &str, e: ergon_io::Error) -> Error {
    match e {
        ergon_io::Error::Config(why) => Error::Usage(format!("{what}: {why}")),
        e => Error::Transport(format!("{what}: {e}")),
    }
}

/// Echoes made while the driver polls, sent right after: `(stream,
/// message)`, preallocated.
pub struct Staged {
    pub(crate) batch: Box<[(u32, [u8; MESSAGE_LEN])]>,
    pub(crate) len: usize,
}

impl Staged {
    /// Room for `capacity`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            batch: vec![(0, [1u8; MESSAGE_LEN]); capacity.max(1)].into_boxed_slice(),
            len: 0,
        }
    }

    /// Takes one; `false` when full.
    #[inline]
    pub fn push(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> bool {
        let Some(slot) = self.batch.get_mut(self.len) else {
            return false;
        };
        *slot = (stream, *msg);
        self.len += 1;
        true
    }

    /// Sends what it holds with `send` (`Ok(false)`: busy), in order;
    /// what does not go stays, first, for the next time.
    ///
    /// # Errors
    /// What `send` returns.
    #[inline]
    pub fn drain(
        &mut self,
        mut send: impl FnMut(u32, &[u8; MESSAGE_LEN]) -> Result<bool, Error>,
    ) -> Result<usize, Error> {
        let mut done = 0;
        while done < self.len {
            let (stream, m) = self.batch[done];
            if !send(stream, &m)? {
                break;
            }
            done += 1;
        }
        self.batch.copy_within(done..self.len, 0);
        self.len -= done;
        Ok(done)
    }
}

struct Shared {
    d: RefCell<Driver<UdpConn>>,
    staged: RefCell<Staged>,
    ids: Vec<ConnId>,
    opts: EioUdpOptions,
    ipv6: bool,
}

/// The sending side.
pub struct EioUdpTx {
    s: Rc<Shared>,
}

/// The receiving side (it owns the driver's poll).
pub struct EioUdpRx {
    s: Rc<Shared>,
}

/// `streams` sockets on one driver: stream `i` bound to `local` + `i`,
/// connected to `peer` + `i`; their sending and receiving sides. Call it on
/// the polling thread, pinned.
///
/// # Errors
/// `Driver::new` (a key out of range), a bind or connect fails.
pub fn pair(
    local: SocketAddr,
    peer: SocketAddr,
    streams: u32,
    opts: EioUdpOptions,
) -> Result<(EioUdpTx, EioUdpRx), Error> {
    if streams == 0 {
        return Err(Error::Usage("streams must be at least 1".into()));
    }
    let mut cfg = base_config(
        opts.rcvbuf,
        opts.epoll_events,
        opts.loop_cpu,
        &opts.helper_cpus,
    );
    cfg.busy_poll_us = opts.busy_poll_us;
    cfg.datagram_burst = opts.datagram_burst;
    cfg.validate().map_err(|e| eio("config", e))?;
    let mut d: Driver<UdpConn> = Driver::new(cfg).map_err(|e| eio("Driver::new", e))?;
    let uopts = UdpOptions {
        max_datagram: MAX_DATAGRAM,
        tx_slots: TX_SLOTS,
        sndbuf: opts.sndbuf,
    };
    let mut ids = Vec::with_capacity(streams as usize);
    for i in 0..streams {
        let (l, p) = (port_plus(local, i)?, port_plus(peer, i)?);
        let c = UdpConn::connect(l, p, &uopts).map_err(|e| eio(&format!("udp {l} -> {p}"), e))?;
        ids.push(d.add(c).map_err(|e| eio("Driver::add", e))?);
    }
    // one poll reads at most this many datagrams
    let per_poll = (opts.epoll_events.min(streams) * opts.datagram_burst) as usize;
    let s = Rc::new(Shared {
        d: RefCell::new(d),
        staged: RefCell::new(Staged::new(per_poll + TX_SLOTS * streams as usize)),
        ids,
        opts,
        ipv6: local.is_ipv6(),
    });
    Ok((EioUdpTx { s: Rc::clone(&s) }, EioUdpRx { s }))
}

/// One datagram queued and flushed now; `Ok(false)` when busy.
#[inline]
fn send_now(
    d: &mut Driver<UdpConn>,
    ids: &[ConnId],
    stream: u32,
    msg: &[u8; MESSAGE_LEN],
) -> Result<bool, Error> {
    let id = *ids
        .get(stream as usize)
        .ok_or_else(|| Error::Transport(format!("stream {stream} of {}", ids.len())))?;
    let mut c = d
        .get_mut(id)
        .ok_or_else(|| Error::Transport(format!("stream {stream}: its socket closed")))?;
    match c.send(msg) {
        Ok(()) => {
            d.flush().map_err(|e| eio("flush", e))?;
            Ok(true)
        }
        Err(ergon_io::Error::Busy(_)) => Ok(false),
        Err(e) => Err(eio("udp send", e)),
    }
}

impl Tx for EioUdpTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        // Inside the receive side's poll (an echo) the driver is taken: the
        // echo is staged and goes as soon as that poll returns.
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
    err: Option<Error>,
}

impl<F> Handler for Deliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        if self.err.is_none() {
            self.err = Some(Error::Transport(format!("udp {id:?} closed: {error}")));
        }
    }
}

impl<F: FnMut(&[u8])> UdpHandler for Deliver<'_, F> {
    #[inline]
    fn on_datagram(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        (self.on)(payload);
    }
}

impl Rx for EioUdpRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let s = &*self.s;
        let mut d = s.d.borrow_mut();
        let mut h = Deliver { on, err: None };
        let n = d.poll(&mut h).map_err(|e| eio("poll", e))?;
        if let Some(e) = h.err {
            return Err(e);
        }
        let mut staged = s.staged.borrow_mut();
        if staged.len > 0 {
            staged.drain(|stream, m| {
                if send_now(&mut d, &s.ids, stream, m)? {
                    return Ok(true);
                }
                // every slot taken: hand them to the kernel, once
                d.flush().map_err(|e| eio("flush", e))?;
                send_now(&mut d, &s.ids, stream, m)
            })?;
        }
        Ok(n)
    }
}

impl Inspect for EioUdpRx {
    fn retrans(&self) -> Result<Option<u64>, Error> {
        Ok(None)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(MESSAGE_LEN as u64 + 8 + ip_overhead(self.s.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let d = self.s.d.borrow();
        let cfg = d.config();
        let mut sum = ergon_io::UdpStats::default();
        for c in self.s.ids.iter().filter_map(|&id| d.get(id)) {
            let s = c.stats();
            sum.rx += s.rx;
            sum.tx += s.tx;
            sum.tx_full += s.tx_full;
            sum.truncated += s.truncated;
            sum.refused += s.refused;
        }
        let ds = d.stats();
        for (k, v) in [
            ("udp.max_datagram", MAX_DATAGRAM.to_string()),
            ("udp.tx_slots", TX_SLOTS.to_string()),
            ("udp.rx_per_pass", cfg.datagram_burst.to_string()),
            (
                "rcvbuf",
                cfg.rcvbuf
                    .map_or_else(|| "kernel".to_owned(), |b| b.to_string()),
            ),
            (
                "sndbuf",
                self.s
                    .opts
                    .sndbuf
                    .map_or_else(|| "kernel".to_owned(), |b| b.to_string()),
            ),
            ("epoll_events", cfg.epoll_events.to_string()),
            ("busy_read_us", cfg.busy_poll_us.to_string()),
            ("datagram_burst", cfg.datagram_burst.to_string()),
            ("ergon.backend", cfg.backend.to_string()),
            ("ergon.rx_timestamps", cfg.rx_timestamps.to_string()),
            ("ergon.helper_cpus", format!("{:?}", cfg.helper_cpus)),
            ("stat.rx", sum.rx.to_string()),
            ("stat.tx", sum.tx.to_string()),
            ("stat.tx_full", sum.tx_full.to_string()),
            ("stat.truncated", sum.truncated.to_string()),
            ("stat.refused", sum.refused.to_string()),
            ("stat.polls", ds.polls.to_string()),
            ("stat.recv_calls", ds.recvmsg_calls.to_string()),
            ("stat.epoll_wait_calls", ds.epoll_wait_calls.to_string()),
            ("stat.send_calls", ds.send_calls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(c) = self.s.ids.first().and_then(|&id| d.get(id)) {
            sockopt::read_back(c.raw_fd(), false, out)?;
        }
        Ok(())
    }
}
