//! Raw mio UDP: a minimal `mio::Poll` loop over one connected
//! `mio::net::UdpSocket` per stream (stream `i` bound to local base port +
//! `i`, connected to peer base port + `i`).
//!
//! - Receive ([`MioUdpRx`]): `epoll_wait(0)` (`Poll::poll` with a zero
//!   timeout); each ready socket is read up to
//!   [`POLL_LIMIT`](super::point::POLL_LIMIT) datagrams a pass, each handed
//!   over straight from the one receive buffer. mio registers
//!   edge-triggered, so a socket not yet read to `WouldBlock` stays on the
//!   ready list for the next pass.
//! - Send ([`MioUdpTx`]): straight to `send(2)`. When the kernel has no
//!   room the datagram waits in the stream's ring of
//!   [`TX_SLOTS`](super::TX_SLOTS) preallocated slots (as ergon-io's
//!   `UdpConn` does) and goes on the next pass; a full ring is busy.
//! - `ECONNREFUSED` (the peer's port answered ICMP unreachable) is counted
//!   and the socket stays open; a refused send is tried once more, then
//!   waits in the ring.
//! - Socket options by `setsockopt`: `SO_RCVBUF`, `SO_SNDBUF`,
//!   `SO_BUSY_POLL`. Nothing allocates after [`pair`].
#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
#![allow(clippy::many_single_char_names)] // s, n, k, i, r: the loop's indices

use std::cell::RefCell;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket as StdUdpSocket};
use std::os::fd::AsRawFd;
use std::rc::Rc;
use std::time::Duration;

use ergon_latency_core::message::MESSAGE_LEN;
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};

use super::point::{Error, POLL_LIMIT, Rx, Sent, Tx};
use super::sock::{Inspect, port_plus};
use super::sockopt::{self, os};
use super::{MAX_DATAGRAM, TX_SLOTS, ip_overhead};

/// The mio UDP arm's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MioUdpOptions {
    /// `SO_RCVBUF`, bytes.
    pub rcvbuf: u32,
    /// `SO_SNDBUF`, bytes; `None`: the kernel's.
    pub sndbuf: Option<u32>,
    /// `SO_BUSY_POLL`, µs (0: none; raising it needs `CAP_NET_ADMIN`).
    pub busy_read_us: u32,
    /// `Events` capacity: the most ready sockets one `epoll_wait` reports.
    pub epoll_events: usize,
}

/// Counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MioUdpStats {
    /// Datagrams received.
    pub rx: u64,
    /// Datagrams the kernel took.
    pub tx: u64,
    /// Sends refused because the stream's ring was full.
    pub tx_full: u64,
    /// `ECONNREFUSED` reports on sends.
    pub tx_refused: u64,
    /// `ECONNREFUSED` reports on reads.
    pub rx_refused: u64,
    /// Sends the kernel had no room for (queued in the ring).
    pub would_block: u64,
    /// `Poll::poll` calls.
    pub polls: u64,
    /// `recv` calls.
    pub recv_calls: u64,
}

/// A stream's queued datagrams.
struct Ring {
    buf: Box<[u8]>,
    lens: Box<[usize]>,
    head: usize,
    queued: usize,
}

impl Ring {
    fn new() -> Self {
        Self {
            // non-zero fill maps every page now: no page fault on first use
            buf: vec![1u8; MAX_DATAGRAM * TX_SLOTS].into_boxed_slice(),
            lens: vec![0usize; TX_SLOTS].into_boxed_slice(),
            head: 0,
            queued: 0,
        }
    }

    fn push(&mut self, payload: &[u8]) -> bool {
        if self.queued == self.lens.len() {
            return false;
        }
        let slot = (self.head + self.queued) % self.lens.len();
        let at = slot * MAX_DATAGRAM;
        self.buf[at..at + payload.len()].copy_from_slice(payload);
        self.lens[slot] = payload.len();
        self.queued += 1;
        true
    }

    fn front(&self) -> Option<&[u8]> {
        (self.queued > 0).then(|| {
            let at = self.head * MAX_DATAGRAM;
            &self.buf[at..at + self.lens[self.head]]
        })
    }

    fn pop(&mut self) {
        self.head = (self.head + 1) % self.lens.len();
        self.queued -= 1;
    }
}

/// What one `send(2)` attempt did.
enum Attempt {
    Taken,
    Again,
}

/// The send side's state: rings, the streams with datagrams waiting, and
/// the counters. The receive side flushes it every pass.
struct TxState {
    rings: Vec<Ring>,
    dirty: Vec<u32>,
    in_dirty: Vec<bool>,
    stats: MioUdpStats,
}

impl TxState {
    /// One datagram to the kernel: `ECONNREFUSED` once is tried again.
    #[inline]
    fn attempt(
        sock: &UdpSocket,
        payload: &[u8],
        stats: &mut MioUdpStats,
    ) -> Result<Attempt, Error> {
        let mut retried = false;
        loop {
            match sock.send(payload) {
                Ok(_) => {
                    stats.tx += 1;
                    return Ok(Attempt::Taken);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    stats.would_block += 1;
                    return Ok(Attempt::Again);
                }
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    stats.tx_refused += 1;
                    if retried {
                        return Ok(Attempt::Again);
                    }
                    retried = true;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(os("udp send", e)),
            }
        }
    }

    /// Every waiting datagram to the kernel, until a socket has no room.
    #[inline]
    fn flush(&mut self, socks: &[UdpSocket]) -> Result<(), Error> {
        let mut k = 0;
        while let Some(&s) = self.dirty.get(k) {
            let i = s as usize;
            let (Some(sock), Some(ring)) = (socks.get(i), self.rings.get_mut(i)) else {
                return Err(Error::Transport(format!("stream {i} of {}", socks.len())));
            };
            while let Some(d) = ring.front() {
                match Self::attempt(sock, d, &mut self.stats)? {
                    Attempt::Taken => ring.pop(),
                    Attempt::Again => break,
                }
            }
            if ring.queued == 0 {
                self.in_dirty[i] = false;
                self.dirty.swap_remove(k);
            } else {
                k += 1;
            }
        }
        Ok(())
    }
}

struct Shared {
    socks: Vec<UdpSocket>,
    tx: RefCell<TxState>,
    opts: MioUdpOptions,
    ipv6: bool,
}

/// The sending side.
pub struct MioUdpTx {
    s: Rc<Shared>,
}

/// The receiving side.
pub struct MioUdpRx {
    s: Rc<Shared>,
    poll: Poll,
    events: Events,
    rx: Box<[u8]>,
    /// streams with datagrams maybe left to read (edge-triggered)
    ready: Vec<u32>,
    in_ready: Vec<bool>,
    rx_stats: MioUdpStats,
}

/// `streams` sockets: stream `i` bound to `local` + `i`, connected to
/// `peer` + `i`; their sending and receiving sides.
///
/// # Errors
/// A bind, connect or socket option fails.
pub fn pair(
    local: SocketAddr,
    peer: SocketAddr,
    streams: u32,
    opts: MioUdpOptions,
) -> Result<(MioUdpTx, MioUdpRx), Error> {
    if streams == 0 || opts.epoll_events == 0 {
        return Err(Error::Usage(
            "streams and epoll events must be at least 1".into(),
        ));
    }
    let poll = Poll::new().map_err(|e| os("epoll_create", e))?;
    let mut socks = Vec::with_capacity(streams as usize);
    for i in 0..streams {
        let (l, p) = (port_plus(local, i)?, port_plus(peer, i)?);
        let s = StdUdpSocket::bind(l).map_err(|e| os(&format!("bind {l}"), e))?;
        s.connect(p).map_err(|e| os(&format!("connect {p}"), e))?;
        s.set_nonblocking(true).map_err(|e| os("O_NONBLOCK", e))?;
        let fd = s.as_raw_fd();
        sockopt::set_rcvbuf(fd, opts.rcvbuf)?;
        if let Some(b) = opts.sndbuf {
            sockopt::set_sndbuf(fd, b)?;
        }
        // 0 as well, so a sysctl default cannot leak in
        sockopt::set_busy_poll(fd, opts.busy_read_us)?;
        let mut s = UdpSocket::from_std(s);
        poll.registry()
            .register(&mut s, Token(i as usize), Interest::READABLE)
            .map_err(|e| os("epoll_ctl", e))?;
        socks.push(s);
    }
    let n = streams as usize;
    let events = Events::with_capacity(opts.epoll_events);
    let s = Rc::new(Shared {
        socks,
        tx: RefCell::new(TxState {
            rings: (0..n).map(|_| Ring::new()).collect(),
            dirty: Vec::with_capacity(n),
            in_dirty: vec![false; n],
            stats: MioUdpStats::default(),
        }),
        opts,
        ipv6: local.is_ipv6(),
    });
    Ok((
        MioUdpTx { s: Rc::clone(&s) },
        MioUdpRx {
            s,
            poll,
            events,
            rx: vec![1u8; MAX_DATAGRAM].into_boxed_slice(),
            ready: Vec::with_capacity(n),
            in_ready: vec![false; n],
            rx_stats: MioUdpStats::default(),
        },
    ))
}

impl Tx for MioUdpTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        let i = stream as usize;
        let mut tx = self.s.tx.borrow_mut();
        let tx = &mut *tx;
        let (Some(sock), Some(ring)) = (self.s.socks.get(i), tx.rings.get_mut(i)) else {
            return Err(Error::Transport(format!(
                "stream {i} of {}",
                self.s.socks.len()
            )));
        };
        // a datagram already waiting goes first: order is kept
        if ring.queued == 0 && matches!(TxState::attempt(sock, msg, &mut tx.stats)?, Attempt::Taken)
        {
            return Ok(Sent::Sent);
        }
        if !ring.push(msg) {
            tx.stats.tx_full += 1;
            return Ok(Sent::Busy);
        }
        if !tx.in_dirty[i] {
            tx.in_dirty[i] = true;
            tx.dirty.push(stream);
        }
        Ok(Sent::Sent)
    }
}

impl Rx for MioUdpRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let s = &*self.s;
        {
            let mut tx = s.tx.borrow_mut();
            if !tx.dirty.is_empty() {
                tx.flush(&s.socks)?;
            }
        }
        self.rx_stats.polls += 1;
        match self.poll.poll(&mut self.events, Some(Duration::ZERO)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(os("epoll_wait", e)),
        }
        for ev in &self.events {
            let i = ev.token().0;
            if i < self.in_ready.len() && !self.in_ready[i] {
                self.in_ready[i] = true;
                self.ready.push(u32::try_from(i).unwrap_or(u32::MAX));
            }
        }
        let mut n = 0;
        let mut k = 0;
        while let Some(&r) = self.ready.get(k) {
            let i = r as usize;
            let sock = &s.socks[i];
            let mut drained = false;
            for _ in 0..POLL_LIMIT {
                self.rx_stats.recv_calls += 1;
                match sock.recv(&mut self.rx) {
                    Ok(len) => {
                        self.rx_stats.rx += 1;
                        n += 1;
                        on(&self.rx[..len]);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        drained = true;
                        break;
                    }
                    Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                        self.rx_stats.rx_refused += 1;
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => return Err(os("udp recv", e)),
                }
            }
            if drained {
                self.in_ready[i] = false;
                self.ready.swap_remove(k);
            } else {
                k += 1;
            }
        }
        let mut tx = s.tx.borrow_mut();
        if !tx.dirty.is_empty() {
            tx.flush(&s.socks)?;
        }
        Ok(n)
    }
}

impl Inspect for MioUdpRx {
    fn retrans(&self) -> Result<Option<u64>, Error> {
        Ok(None)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(MESSAGE_LEN as u64 + 8 + ip_overhead(self.s.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let t = self.s.tx.borrow().stats;
        let r = self.rx_stats;
        let o = &self.s.opts;
        for (k, v) in [
            ("udp.max_datagram", MAX_DATAGRAM.to_string()),
            ("udp.tx_slots", TX_SLOTS.to_string()),
            ("udp.rx_per_pass", POLL_LIMIT.to_string()),
            ("rcvbuf", o.rcvbuf.to_string()),
            (
                "sndbuf",
                o.sndbuf
                    .map_or_else(|| "kernel".to_owned(), |b| b.to_string()),
            ),
            ("busy_read_us", o.busy_read_us.to_string()),
            ("epoll_events", o.epoll_events.to_string()),
            ("stat.rx", r.rx.to_string()),
            ("stat.tx", t.tx.to_string()),
            ("stat.tx_full", t.tx_full.to_string()),
            ("stat.refused", (t.tx_refused + r.rx_refused).to_string()),
            ("stat.would_block", t.would_block.to_string()),
            ("stat.polls", r.polls.to_string()),
            ("stat.recv_calls", r.recv_calls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(s) = self.s.socks.first() {
            sockopt::read_back(s.as_raw_fd(), false, out)?;
        }
        Ok(())
    }
}
