//! Raw mio UDP: a minimal `mio::Poll` loop over one connected
//! `mio::net::UdpSocket` per stream (stream `i` bound to local base port +
//! `i`, connected to peer base port + `i`).
//!
//! - Receive: `epoll_wait(0)` (`Poll::poll` with a zero timeout), then each
//!   ready socket read until `WouldBlock` (mio registers edge-triggered),
//!   each datagram handed over straight from the one receive buffer.
//! - Send: straight to `send(2)`. When the kernel has no room the datagram
//!   waits in the stream's ring of [`TX_SLOTS`](super::TX_SLOTS) preallocated
//!   slots (as ergon-io's `UdpConn` does) and goes on the next pass; a full
//!   ring is back-pressure.
//! - `ECONNREFUSED` (the peer's port answered ICMP unreachable) is counted
//!   and the socket stays open; a refused send is tried once more, then
//!   waits in the ring.
//! - Socket options by `setsockopt`: `SO_RCVBUF`, `SO_SNDBUF`,
//!   `SO_BUSY_POLL`. Nothing allocates after [`MioUdp::new`].

use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket as StdUdpSocket};
use std::os::fd::AsRawFd;
use std::time::Duration;

use ergon_latency_core::message::{MESSAGE_LEN, echo_in_place};
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};

use super::{Error, Link, MAX_DATAGRAM, TX_SLOTS, ip_overhead, sockopt};

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
    /// `ECONNREFUSED` reports, on reads and sends.
    pub refused: u64,
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
enum Sent {
    Taken,
    Again,
}

/// The send side: rings and the streams with datagrams waiting.
struct Tx {
    rings: Vec<Ring>,
    dirty: Vec<u32>,
    in_dirty: Vec<bool>,
    stats: MioUdpStats,
}

impl Tx {
    /// One datagram to the kernel: `ECONNREFUSED` once is tried again.
    #[inline]
    fn attempt(sock: &UdpSocket, payload: &[u8], stats: &mut MioUdpStats) -> Result<Sent, Error> {
        let mut retried = false;
        loop {
            match sock.send(payload) {
                Ok(_) => {
                    stats.tx += 1;
                    return Ok(Sent::Taken);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    stats.would_block += 1;
                    return Ok(Sent::Again);
                }
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    stats.refused += 1;
                    if retried {
                        return Ok(Sent::Again);
                    }
                    retried = true;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
    }

    #[inline]
    fn send(&mut self, socks: &[UdpSocket], i: usize, payload: &[u8]) -> Result<bool, Error> {
        let (Some(sock), Some(ring)) = (socks.get(i), self.rings.get_mut(i)) else {
            return Err(Error::Protocol(format!("stream {i} of {}", socks.len())));
        };
        if payload.len() > MAX_DATAGRAM {
            return Err(Error::Config(format!(
                "datagram of {} bytes over {MAX_DATAGRAM}",
                payload.len()
            )));
        }
        // a datagram already waiting goes first: order is kept
        if ring.queued == 0 && matches!(Self::attempt(sock, payload, &mut self.stats)?, Sent::Taken)
        {
            return Ok(true);
        }
        if !ring.push(payload) {
            self.stats.tx_full += 1;
            return Ok(false);
        }
        if !self.in_dirty[i] {
            self.in_dirty[i] = true;
            self.dirty.push(u32::try_from(i).unwrap_or(u32::MAX));
        }
        Ok(true)
    }

    /// Every waiting datagram to the kernel, until a socket has no room.
    #[inline]
    fn flush(&mut self, socks: &[UdpSocket]) -> Result<(), Error> {
        let mut k = 0;
        while let Some(&s) = self.dirty.get(k) {
            let i = s as usize;
            let (Some(sock), Some(ring)) = (socks.get(i), self.rings.get_mut(i)) else {
                return Err(Error::Protocol(format!("stream {i} of {}", socks.len())));
            };
            while let Some(d) = ring.front() {
                match Self::attempt(sock, d, &mut self.stats)? {
                    Sent::Taken => ring.pop(),
                    Sent::Again => break,
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

/// N connected UDP sockets on one `mio::Poll`.
pub struct MioUdp {
    poll: Poll,
    events: Events,
    socks: Vec<UdpSocket>,
    rx: Box<[u8]>,
    tx: Tx,
    opts: MioUdpOptions,
    ipv6: bool,
}

/// `base` with its port moved up by `i`.
///
/// # Errors
/// [`Error::Config`] for port 0 or a port past 65 535.
pub fn port_plus(base: SocketAddr, i: u32) -> Result<SocketAddr, Error> {
    if base.port() == 0 {
        return Err(Error::Config(format!(
            "{base}: a base port is needed (stream i uses port + i)"
        )));
    }
    let port = u32::from(base.port()) + i;
    let port =
        u16::try_from(port).map_err(|_| Error::Config(format!("{base} + {i}: past port 65535")))?;
    Ok(SocketAddr::new(base.ip(), port))
}

impl MioUdp {
    /// `streams` sockets: stream `i` bound to `local` + `i`, connected to
    /// `peer` + `i`.
    ///
    /// # Errors
    /// A bind, connect or socket option fails.
    pub fn new(
        local: SocketAddr,
        peer: SocketAddr,
        streams: u32,
        opts: MioUdpOptions,
    ) -> Result<Self, Error> {
        if streams == 0 || opts.epoll_events == 0 {
            return Err(Error::Config(
                "streams and epoll_events must be at least 1".into(),
            ));
        }
        let poll = Poll::new()?;
        let mut socks = Vec::with_capacity(streams as usize);
        for i in 0..streams {
            let s = StdUdpSocket::bind(port_plus(local, i)?)?;
            s.connect(port_plus(peer, i)?)?;
            s.set_nonblocking(true)?;
            let fd = s.as_raw_fd();
            sockopt::set_rcvbuf(fd, opts.rcvbuf)?;
            if let Some(b) = opts.sndbuf {
                sockopt::set_sndbuf(fd, b)?;
            }
            // 0 as well, so a sysctl default cannot leak in
            sockopt::set_busy_poll(fd, opts.busy_read_us)?;
            let mut s = UdpSocket::from_std(s);
            poll.registry()
                .register(&mut s, Token(i as usize), Interest::READABLE)?;
            socks.push(s);
        }
        let n = streams as usize;
        Ok(Self {
            poll,
            events: Events::with_capacity(opts.epoll_events),
            socks,
            rx: vec![1u8; MAX_DATAGRAM].into_boxed_slice(),
            tx: Tx {
                rings: (0..n).map(|_| Ring::new()).collect(),
                dirty: Vec::with_capacity(n),
                in_dirty: vec![false; n],
                stats: MioUdpStats::default(),
            },
            opts,
            ipv6: local.is_ipv6(),
        })
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> MioUdpStats {
        self.tx.stats
    }

    /// One receive pass: `on_datagram(stream, payload, sockets, send side)`
    /// for each datagram.
    #[inline]
    fn pass<F>(&mut self, mut on_datagram: F) -> Result<usize, Error>
    where
        F: FnMut(usize, &mut [u8], &[UdpSocket], &mut Tx) -> Result<(), Error>,
    {
        let Self {
            poll,
            events,
            socks,
            rx,
            tx,
            ..
        } = self;
        if !tx.dirty.is_empty() {
            tx.flush(socks)?;
        }
        tx.stats.polls += 1;
        match poll.poll(events, Some(Duration::ZERO)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => return Ok(0),
            Err(e) => return Err(e.into()),
        }
        let mut n = 0;
        for ev in events.iter() {
            let i = ev.token().0;
            let Some(sock) = socks.get(i) else {
                continue;
            };
            loop {
                tx.stats.recv_calls += 1;
                match sock.recv(rx) {
                    Ok(len) => {
                        tx.stats.rx += 1;
                        n += 1;
                        on_datagram(i, &mut rx[..len], socks, tx)?;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == ErrorKind::ConnectionRefused => tx.stats.refused += 1,
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        if !tx.dirty.is_empty() {
            tx.flush(socks)?;
        }
        Ok(n)
    }
}

impl Link for MioUdp {
    fn streams(&self) -> u32 {
        u32::try_from(self.socks.len()).unwrap_or(u32::MAX)
    }

    #[inline]
    fn send(&mut self, stream: u32, payload: &[u8]) -> Result<bool, Error> {
        self.tx.send(&self.socks, stream as usize, payload)
    }

    #[inline]
    fn poll<F>(&mut self, on_msg: &mut F) -> Result<usize, Error>
    where
        F: FnMut(u32, &[u8]) -> Result<(), Error>,
    {
        self.pass(|i, payload, _, _| on_msg(u32::try_from(i).unwrap_or(u32::MAX), payload))
    }

    #[inline]
    fn echo(&mut self) -> Result<usize, Error> {
        self.pass(|i, payload, socks, tx| {
            echo_in_place(payload)?;
            if tx.send(socks, i, payload)? {
                Ok(())
            } else {
                Err(Error::Protocol(format!(
                    "echo on stream {i}: send ring full"
                )))
            }
        })
    }

    fn retrans(&self) -> Result<Option<u64>, Error> {
        Ok(None)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(MESSAGE_LEN as u64 + 8 + ip_overhead(self.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let s = self.tx.stats;
        for (k, v) in [
            ("udp.max_datagram", MAX_DATAGRAM.to_string()),
            ("udp.tx_slots", TX_SLOTS.to_string()),
            ("rcvbuf", self.opts.rcvbuf.to_string()),
            (
                "sndbuf",
                self.opts
                    .sndbuf
                    .map_or_else(|| "kernel".to_owned(), |b| b.to_string()),
            ),
            ("busy_read_us", self.opts.busy_read_us.to_string()),
            ("epoll_events", self.opts.epoll_events.to_string()),
            ("stat.rx", s.rx.to_string()),
            ("stat.tx", s.tx.to_string()),
            ("stat.tx_full", s.tx_full.to_string()),
            ("stat.refused", s.refused.to_string()),
            ("stat.would_block", s.would_block.to_string()),
            ("stat.polls", s.polls.to_string()),
            ("stat.recv_calls", s.recv_calls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(s) = self.socks.first() {
            sockopt::read_back(s.as_raw_fd(), false, out)?;
        }
        Ok(())
    }
}
