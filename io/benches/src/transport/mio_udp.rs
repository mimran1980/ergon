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
//! - Batched ([`MioUdpOptions::mmsg`] > 0): the same loop, but each read is
//!   a `recvmmsg` of up to `mmsg` datagrams, and a send only queues: the
//!   point's flush (after each group of sends) and the end of each receive
//!   pass hand every queued datagram to the kernel in one `sendmmsg` per
//!   socket. mio has no batched call, so these two go through
//!   `ergon_io::sys` (thin `libc` wrappers) on the mio socket's fd.
//!
//! The sockets, their send rings and the per-stream read are
//! [`MioUdpStreams`] and [`MioUdpReader`], built on any `mio::Registry`
//! with a token base: another binary can poll them in its own loop, beside
//! its other sockets, with exactly this arm's socket code.
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

use ergon_io::sys::MmsgBatch;
use ergon_latency_core::message::MESSAGE_LEN;
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Registry, Token};

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
    /// 0: one `recv` and one `send` a datagram. N: `recvmmsg` of up to N a
    /// call, and queued sends leave in `sendmmsg` (module docs).
    pub mmsg: usize,
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
    /// `recv` (or `recvmmsg`) calls.
    pub recv_calls: u64,
    /// `send` (or `sendmmsg`) calls.
    pub send_calls: u64,
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
    /// batched sends (`mmsg` > 0)
    mm: Option<MmsgBatch>,
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
            stats.send_calls += 1;
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
            if let Some(mm) = self.mm.as_mut() {
                Self::flush_batched(sock, ring, mm, &mut self.stats)?;
            } else {
                while let Some(d) = ring.front() {
                    match Self::attempt(sock, d, &mut self.stats)? {
                        Attempt::Taken => ring.pop(),
                        Attempt::Again => break,
                    }
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

impl TxState {
    /// `ring`'s datagrams to the kernel in `sendmmsg` calls, until it is
    /// empty or the socket has no room; `ECONNREFUSED` once is tried again.
    #[inline]
    fn flush_batched(
        sock: &UdpSocket,
        ring: &mut Ring,
        mm: &mut MmsgBatch,
        stats: &mut MioUdpStats,
    ) -> Result<(), Error> {
        let fd = sock.as_raw_fd();
        let slots = ring.lens.len();
        let mut retried = false;
        while ring.queued > 0 {
            let n = ring.queued.min(mm.capacity());
            for j in 0..n {
                let slot = (ring.head + j) % slots;
                let at = slot * MAX_DATAGRAM;
                mm.arm_tx(j, &ring.buf[at..at + ring.lens[slot]]);
            }
            stats.send_calls += 1;
            match ergon_io::sys::sendmmsg(fd, mm, n) {
                Ok(0) => {
                    stats.would_block += 1;
                    return Ok(());
                }
                Ok(k) => {
                    stats.tx += k as u64;
                    for _ in 0..k {
                        ring.pop();
                    }
                }
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    stats.tx_refused += 1;
                    if retried {
                        return Ok(());
                    }
                    retried = true;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(os("udp sendmmsg", e)),
            }
        }
        Ok(())
    }
}

/// The mio UDP arm's sockets and send side.
///
/// Stream `i` is the socket on `local` port + `i`, connected to `peer`
/// port + `i`, registered readable on a caller's `Registry` as
/// `Token(token_base + i)`. Every method takes `&self`, so a read's callback
/// may send (an echo).
pub struct MioUdpStreams {
    socks: Vec<UdpSocket>,
    tx: RefCell<TxState>,
    opts: MioUdpOptions,
    ipv6: bool,
    token_base: usize,
}

impl MioUdpStreams {
    /// `streams` sockets bound, connected, set up as [`MioUdpOptions`] says
    /// and registered on `registry` from `Token(token_base)`.
    ///
    /// # Errors
    /// A bind, connect, socket option or registration fails, or no stream.
    pub fn new(
        registry: &Registry,
        token_base: usize,
        local: SocketAddr,
        peer: SocketAddr,
        streams: u32,
        opts: MioUdpOptions,
    ) -> Result<Self, Error> {
        if streams == 0 {
            return Err(Error::Usage("streams must be at least 1".into()));
        }
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
            registry
                .register(&mut s, Token(token_base + i as usize), Interest::READABLE)
                .map_err(|e| os("epoll_ctl", e))?;
            socks.push(s);
        }
        let n = streams as usize;
        Ok(Self {
            socks,
            tx: RefCell::new(TxState {
                rings: (0..n).map(|_| Ring::new()).collect(),
                dirty: Vec::with_capacity(n),
                in_dirty: vec![false; n],
                stats: MioUdpStats::default(),
                mm: (opts.mmsg > 0).then(|| MmsgBatch::new(opts.mmsg)),
            }),
            opts,
            ipv6: local.is_ipv6(),
            token_base,
        })
    }

    /// The streams.
    #[must_use]
    pub fn streams(&self) -> u32 {
        u32::try_from(self.socks.len()).unwrap_or(u32::MAX)
    }

    /// The stream a token of this set stands for.
    #[must_use]
    pub fn stream_of(&self, token: Token) -> Option<u32> {
        token
            .0
            .checked_sub(self.token_base)
            .filter(|&i| i < self.socks.len())
            .and_then(|i| u32::try_from(i).ok())
    }

    /// Sends `payload` (up to [`MAX_DATAGRAM`] bytes) on `stream`: to the
    /// kernel at once, else into the stream's ring behind what waits there;
    /// [`Sent::Busy`] when the ring is full.
    ///
    /// # Errors
    /// No such stream, a datagram too long, or the send failed for good.
    #[inline]
    pub fn send(&self, stream: u32, payload: &[u8]) -> Result<Sent, Error> {
        let i = stream as usize;
        if payload.len() > MAX_DATAGRAM {
            return Err(Error::Transport(format!(
                "a {}-byte datagram",
                payload.len()
            )));
        }
        let mut tx = self.tx.borrow_mut();
        let tx = &mut *tx;
        let (Some(sock), Some(ring)) = (self.socks.get(i), tx.rings.get_mut(i)) else {
            return Err(Error::Transport(format!(
                "stream {i} of {}",
                self.socks.len()
            )));
        };
        // a datagram already waiting goes first: order is kept; batched,
        // every send waits for the flush
        if ring.queued == 0
            && tx.mm.is_none()
            && matches!(
                TxState::attempt(sock, payload, &mut tx.stats)?,
                Attempt::Taken
            )
        {
            return Ok(Sent::Sent);
        }
        if !ring.push(payload) {
            tx.stats.tx_full += 1;
            return Ok(Sent::Busy);
        }
        if !tx.in_dirty[i] {
            tx.in_dirty[i] = true;
            tx.dirty.push(stream);
        }
        Ok(Sent::Sent)
    }

    /// Every datagram waiting in a ring to the kernel, until a socket has no
    /// room.
    ///
    /// # Errors
    /// A send failed for good.
    #[inline]
    pub fn flush(&self) -> Result<(), Error> {
        let mut tx = self.tx.borrow_mut();
        if tx.dirty.is_empty() {
            return Ok(());
        }
        tx.flush(&self.socks)
    }

    /// The send side's counters.
    #[must_use]
    pub fn tx_stats(&self) -> MioUdpStats {
        self.tx.borrow().stats
    }

    /// Bytes on the wire per 64-byte message.
    #[must_use]
    pub const fn wire_bytes(&self) -> u64 {
        MESSAGE_LEN as u64 + 8 + ip_overhead(self.ipv6)
    }

    /// The settings and counters (`reader`'s receive side), and stream 0's
    /// socket options read back.
    ///
    /// # Errors
    /// A read-back fails.
    pub fn report(
        &self,
        reader: &MioUdpReader,
        out: &mut Vec<(String, String)>,
    ) -> Result<(), Error> {
        let t = self.tx_stats();
        let r = reader.stats;
        let o = &self.opts;
        for (k, v) in [
            ("udp.max_datagram", MAX_DATAGRAM.to_string()),
            ("udp.tx_slots", TX_SLOTS.to_string()),
            ("udp.rx_per_pass", POLL_LIMIT.to_string()),
            ("udp.mmsg", o.mmsg.to_string()),
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
            ("stat.send_calls", t.send_calls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(s) = self.socks.first() {
            sockopt::read_back(s.as_raw_fd(), false, out)?;
        }
        Ok(())
    }
}

/// A receive side's buffer and counters: reads one stream of a
/// [`MioUdpStreams`] at a time.
pub struct MioUdpReader {
    buf: Box<[u8]>,
    /// batched reads (`mmsg` > 0)
    mm: Option<MmsgBatch>,
    /// The receive side's counters (`polls` is the caller's to count).
    pub stats: MioUdpStats,
}

impl Default for MioUdpReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MioUdpReader {
    /// One [`MAX_DATAGRAM`] buffer, every page touched now.
    #[must_use]
    pub fn new() -> Self {
        Self::with_mmsg(0)
    }

    /// 0: as [`new`](Self::new). N: `N` × [`MAX_DATAGRAM`] bytes, read
    /// with `recvmmsg` of up to N datagrams a call.
    #[must_use]
    pub fn with_mmsg(mmsg: usize) -> Self {
        Self {
            buf: vec![1u8; MAX_DATAGRAM * mmsg.max(1)].into_boxed_slice(),
            mm: (mmsg > 0).then(|| MmsgBatch::new(mmsg)),
            stats: MioUdpStats::default(),
        }
    }

    /// Reads `stream` up to [`POLL_LIMIT`] datagrams, each handed to `on`:
    /// (datagrams read, read to `WouldBlock`). mio is edge-triggered: a
    /// stream not read to `WouldBlock` must be read again without an event.
    ///
    /// # Errors
    /// No such stream, or the read failed for good.
    #[inline]
    pub fn read<F: FnMut(&[u8])>(
        &mut self,
        streams: &MioUdpStreams,
        stream: u32,
        on: &mut F,
    ) -> Result<(usize, bool), Error> {
        let sock = streams.socks.get(stream as usize).ok_or_else(|| {
            Error::Transport(format!("stream {stream} of {}", streams.socks.len()))
        })?;
        if self.mm.is_some() {
            return self.read_batched(sock, on);
        }
        let mut n = 0;
        for _ in 0..POLL_LIMIT {
            self.stats.recv_calls += 1;
            match sock.recv(&mut self.buf) {
                Ok(len) => {
                    self.stats.rx += 1;
                    n += 1;
                    on(&self.buf[..len]);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok((n, true)),
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    self.stats.rx_refused += 1;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(os("udp recv", e)),
            }
        }
        Ok((n, false))
    }
}

impl MioUdpReader {
    /// [`read`](Self::read) in `recvmmsg` calls: a call that comes back
    /// short read the socket dry.
    #[inline]
    fn read_batched<F: FnMut(&[u8])>(
        &mut self,
        sock: &UdpSocket,
        on: &mut F,
    ) -> Result<(usize, bool), Error> {
        let Some(mm) = self.mm.as_mut() else {
            return Ok((0, true));
        };
        let fd = sock.as_raw_fd();
        let mut n = 0;
        while n < POLL_LIMIT {
            let armed = mm.arm_rx(&mut self.buf, MAX_DATAGRAM, POLL_LIMIT - n);
            self.stats.recv_calls += 1;
            match ergon_io::sys::recvmmsg(fd, mm, armed) {
                Ok(None) => return Ok((n, true)),
                Ok(Some(k)) => {
                    for j in 0..k {
                        let at = j * MAX_DATAGRAM;
                        on(&self.buf[at..at + mm.len(j).min(MAX_DATAGRAM)]);
                    }
                    self.stats.rx += k as u64;
                    n += k;
                    if k < armed {
                        return Ok((n, true));
                    }
                }
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    self.stats.rx_refused += 1;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(os("udp recvmmsg", e)),
            }
        }
        Ok((n, false))
    }
}

/// The sending side.
pub struct MioUdpTx {
    s: Rc<MioUdpStreams>,
}

/// The receiving side.
pub struct MioUdpRx {
    s: Rc<MioUdpStreams>,
    poll: Poll,
    events: Events,
    reader: MioUdpReader,
    /// streams with datagrams maybe left to read (edge-triggered)
    ready: Vec<u32>,
    in_ready: Vec<bool>,
}

/// `streams` sockets: stream `i` bound to `local` + `i`, connected to
/// `peer` + `i`; their sending and receiving sides, on a poll of their own.
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
    let opts_mmsg = opts.mmsg;
    let poll = Poll::new().map_err(|e| os("epoll_create", e))?;
    let events = Events::with_capacity(opts.epoll_events);
    let s = Rc::new(MioUdpStreams::new(
        poll.registry(),
        0,
        local,
        peer,
        streams,
        opts,
    )?);
    let n = streams as usize;
    Ok((
        MioUdpTx { s: Rc::clone(&s) },
        MioUdpRx {
            s,
            poll,
            events,
            reader: MioUdpReader::with_mmsg(opts_mmsg),
            ready: Vec::with_capacity(n),
            in_ready: vec![false; n],
        },
    ))
}

impl Tx for MioUdpTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        self.s.send(stream, msg)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Error> {
        self.s.flush()
    }
}

impl Rx for MioUdpRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let s = &*self.s;
        s.flush()?;
        self.reader.stats.polls += 1;
        match self.poll.poll(&mut self.events, Some(Duration::ZERO)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(os("epoll_wait", e)),
        }
        for ev in &self.events {
            if let Some(r) = s.stream_of(ev.token()) {
                let i = r as usize;
                if !self.in_ready[i] {
                    self.in_ready[i] = true;
                    self.ready.push(r);
                }
            }
        }
        let mut n = 0;
        let mut k = 0;
        while let Some(&r) = self.ready.get(k) {
            let (read, drained) = self.reader.read(s, r, on)?;
            n += read;
            if drained {
                self.in_ready[r as usize] = false;
                self.ready.swap_remove(k);
            } else {
                k += 1;
            }
        }
        s.flush()?;
        Ok(n)
    }
}

impl Inspect for MioUdpRx {
    fn retrans(&self) -> Result<Option<u64>, Error> {
        Ok(None)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(self.s.wire_bytes())
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        self.s.report(&self.reader, out)
    }
}
