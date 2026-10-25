//! Raw mio TCP: a minimal `mio::Poll` loop over one `mio::net::TcpStream`
//! per stream, framed with `ergon_latency_core::frame` (u32 LE length,
//! then the payload).
//!
//! - Receive: `epoll_wait(0)`, then each ready connection read until
//!   `WouldBlock` (edge-triggered) into its `Deframer`
//!   ([`TCP_RX_BUF`](super::TCP_RX_BUF) bytes, the room ergon-io's framed
//!   TCP reads into), each whole frame handed over in place. With
//!   `quickack_per_read`, `TCP_QUICKACK` after every read with data.
//! - Send: the frame is encoded into the connection's preallocated send
//!   queue ([`TCP_TX_BUF`](super::TCP_TX_BUF)) and written at once; what
//!   the kernel has no room for goes on the next pass. A full queue is
//!   back-pressure.
//! - Echo: each frame's echo is made in a copy and queued; the queue is
//!   written once after each read.
//! - Socket options by `setsockopt`: `TCP_NODELAY`, `SO_RCVBUF`,
//!   `SO_SNDBUF`, `SO_BUSY_POLL`, `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US`
//!   (read back; refused on a kernel without them). Nothing allocates after
//!   [`MioTcp::new`].

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream as StdTcpStream;
use std::os::fd::AsRawFd;
use std::time::Duration;

use ergon_latency_core::frame::{self, Deframer, FRAME_HEADER};
use ergon_latency_core::message::{MESSAGE_LEN, echo_in_place};
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};

use super::{Error, Link, MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF, ip_overhead, sockopt};

/// The mio TCP arm's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MioTcpOptions {
    /// `SO_RCVBUF`, bytes.
    pub rcvbuf: u32,
    /// `SO_SNDBUF`, bytes; `None`: the kernel's.
    pub sndbuf: Option<u32>,
    /// `SO_BUSY_POLL`, µs (0: none).
    pub busy_read_us: u32,
    /// `Events` capacity.
    pub epoll_events: usize,
    /// `TCP_QUICKACK` after every read with data.
    pub quickack_per_read: bool,
    /// `TCP_RTO_MIN_US`, µs; `None`: the kernel's.
    pub rto_min_us: Option<u32>,
    /// `TCP_DELACK_MAX_US`, µs; `None`: the kernel's.
    pub delack_max_us: Option<u32>,
}

/// Counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MioTcpStats {
    /// Frames received.
    pub frames_in: u64,
    /// Frames queued to send.
    pub frames_out: u64,
    /// Bytes received.
    pub bytes_in: u64,
    /// `read` calls.
    pub reads: u64,
    /// `write` calls.
    pub writes: u64,
    /// `TCP_QUICKACK` re-arms.
    pub quickacks: u64,
    /// Sends refused because the send queue was full.
    pub tx_full: u64,
    /// Writes the kernel had no room for.
    pub would_block: u64,
    /// `Poll::poll` calls.
    pub polls: u64,
}

/// A connection's send queue: bytes `head..tail` not yet taken by the
/// kernel.
pub struct TxQueue {
    buf: Box<[u8]>,
    head: usize,
    tail: usize,
}

impl TxQueue {
    fn new() -> Self {
        // non-zero fill maps every page now: no page fault on first use
        Self {
            buf: vec![1u8; TCP_TX_BUF].into_boxed_slice(),
            head: 0,
            tail: 0,
        }
    }

    /// Encodes one frame onto the queue; `false` when it has no room.
    #[inline]
    fn queue(&mut self, payload: &[u8]) -> Result<bool, Error> {
        let need = FRAME_HEADER + payload.len();
        if self.tail + need > self.buf.len() {
            if self.head > 0 {
                self.buf.copy_within(self.head..self.tail, 0);
                self.tail -= self.head;
                self.head = 0;
            }
            if self.tail + need > self.buf.len() {
                return Ok(false);
            }
        }
        self.tail += frame::encode(payload, &mut self.buf[self.tail..])?;
        Ok(true)
    }

    /// Writes the queue until the kernel has no room; `true` when bytes are
    /// left.
    #[inline]
    fn write_out(&mut self, mut sock: &TcpStream, stats: &mut MioTcpStats) -> Result<bool, Error> {
        while self.head < self.tail {
            stats.writes += 1;
            match sock.write(&self.buf[self.head..self.tail]) {
                Ok(0) => return Err(Error::Closed("write returned 0".into())),
                Ok(n) => self.head += n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    stats.would_block += 1;
                    return Ok(true);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
        self.head = 0;
        self.tail = 0;
        Ok(false)
    }
}

struct Conn {
    sock: TcpStream,
    de: Deframer,
    tx: TxQueue,
}

/// N TCP connections on one `mio::Poll`.
pub struct MioTcp {
    poll: Poll,
    events: Events,
    conns: Vec<Conn>,
    dirty: Vec<u32>,
    in_dirty: Vec<bool>,
    opts: MioTcpOptions,
    stats: MioTcpStats,
    ipv6: bool,
}

/// Applies the mio arm's socket options to a connected socket.
///
/// # Errors
/// A `setsockopt` fails, or a timer is refused (kernel before 6.15) or
/// rounded.
pub fn prepare(s: &StdTcpStream, opts: &MioTcpOptions) -> Result<(), Error> {
    let fd = s.as_raw_fd();
    sockopt::set_nodelay(fd)?;
    sockopt::set_rcvbuf(fd, opts.rcvbuf)?;
    if let Some(b) = opts.sndbuf {
        sockopt::set_sndbuf(fd, b)?;
    }
    // 0 as well, so a sysctl default cannot leak in
    sockopt::set_busy_poll(fd, opts.busy_read_us)?;
    if let Some(us) = opts.rto_min_us {
        sockopt::set_tcp_timer(fd, sockopt::TCP_RTO_MIN_US, us)?;
    }
    if let Some(us) = opts.delack_max_us {
        sockopt::set_tcp_timer(fd, sockopt::TCP_DELACK_MAX_US, us)?;
    }
    s.set_nonblocking(true)?;
    Ok(())
}

impl MioTcp {
    /// Takes connected sockets, stream `i` the `i`-th.
    ///
    /// # Errors
    /// [`prepare`] fails, or registering one does.
    pub fn new(socks: Vec<StdTcpStream>, opts: MioTcpOptions) -> Result<Self, Error> {
        if socks.is_empty() || opts.epoll_events == 0 {
            return Err(Error::Config(
                "streams and epoll_events must be at least 1".into(),
            ));
        }
        let poll = Poll::new()?;
        let n = socks.len();
        let ipv6 = socks[0].local_addr()?.is_ipv6();
        let mut conns = Vec::with_capacity(n);
        for (i, s) in socks.into_iter().enumerate() {
            prepare(&s, &opts)?;
            let mut sock = TcpStream::from_std(s);
            poll.registry()
                .register(&mut sock, Token(i), Interest::READABLE)?;
            let mut de = Deframer::new(MAX_FRAME)?;
            // touch the receive buffer now: no page fault on the first reads
            let spare = de.spare();
            if spare.len() != TCP_RX_BUF {
                return Err(Error::Config(format!(
                    "deframer holds {} bytes, not {TCP_RX_BUF}",
                    spare.len()
                )));
            }
            spare.fill(1);
            conns.push(Conn {
                sock,
                de,
                tx: TxQueue::new(),
            });
        }
        Ok(Self {
            poll,
            events: Events::with_capacity(opts.epoll_events),
            conns,
            dirty: Vec::with_capacity(n),
            in_dirty: vec![false; n],
            opts,
            stats: MioTcpStats::default(),
            ipv6,
        })
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> MioTcpStats {
        self.stats
    }

    #[inline]
    fn mark(dirty: &mut Vec<u32>, in_dirty: &mut [bool], i: usize) {
        if !in_dirty[i] {
            in_dirty[i] = true;
            dirty.push(u32::try_from(i).unwrap_or(u32::MAX));
        }
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Error> {
        let mut k = 0;
        while let Some(&s) = self.dirty.get(k) {
            let i = s as usize;
            let c = &mut self.conns[i];
            if c.tx.write_out(&c.sock, &mut self.stats)? {
                k += 1;
            } else {
                self.in_dirty[i] = false;
                self.dirty.swap_remove(k);
            }
        }
        Ok(())
    }

    /// One receive pass: `on_frame(stream, payload, send queue, stats)` for
    /// each whole frame, in place in the receive buffer; after each read's
    /// frames, the send queue is written out when `write_after`.
    #[inline]
    fn pass<F>(&mut self, mut on_frame: F, write_after: bool) -> Result<usize, Error>
    where
        F: FnMut(usize, &[u8], &mut TxQueue, &mut MioTcpStats) -> Result<(), Error>,
    {
        if !self.dirty.is_empty() {
            self.flush()?;
        }
        self.stats.polls += 1;
        match self.poll.poll(&mut self.events, Some(Duration::ZERO)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => return Ok(0),
            Err(e) => return Err(e.into()),
        }
        let quickack = self.opts.quickack_per_read;
        let mut n = 0;
        for ev in &self.events {
            let i = ev.token().0;
            let Some(c) = self.conns.get_mut(i) else {
                continue;
            };
            loop {
                let spare = c.de.spare();
                if spare.is_empty() {
                    return Err(Error::Protocol(format!("stream {i}: receive buffer full")));
                }
                self.stats.reads += 1;
                match (&c.sock).read(spare) {
                    Ok(0) => return Err(Error::Closed(format!("stream {i}: the peer closed"))),
                    Ok(len) => {
                        c.de.filled(len)?;
                        self.stats.bytes_in += len as u64;
                        if quickack {
                            sockopt::quickack(c.sock.as_raw_fd())?;
                            self.stats.quickacks += 1;
                        }
                        while let Some(f) = c.de.next_frame()? {
                            self.stats.frames_in += 1;
                            n += 1;
                            on_frame(i, f, &mut c.tx, &mut self.stats)?;
                        }
                        if write_after && c.tx.write_out(&c.sock, &mut self.stats)? {
                            Self::mark(&mut self.dirty, &mut self.in_dirty, i);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        if !self.dirty.is_empty() {
            self.flush()?;
        }
        Ok(n)
    }
}

impl Link for MioTcp {
    fn streams(&self) -> u32 {
        u32::try_from(self.conns.len()).unwrap_or(u32::MAX)
    }

    #[inline]
    fn send(&mut self, stream: u32, payload: &[u8]) -> Result<bool, Error> {
        let i = stream as usize;
        let n = self.conns.len();
        let c = self
            .conns
            .get_mut(i)
            .ok_or_else(|| Error::Protocol(format!("stream {stream} of {n}")))?;
        if payload.len() > MAX_FRAME {
            return Err(Error::Config(format!(
                "frame of {} bytes over {MAX_FRAME}",
                payload.len()
            )));
        }
        if !c.tx.queue(payload)? {
            self.stats.tx_full += 1;
            return Ok(false);
        }
        self.stats.frames_out += 1;
        if c.tx.write_out(&c.sock, &mut self.stats)? {
            Self::mark(&mut self.dirty, &mut self.in_dirty, i);
        }
        Ok(true)
    }

    #[inline]
    fn poll<F>(&mut self, on_msg: &mut F) -> Result<usize, Error>
    where
        F: FnMut(u32, &[u8]) -> Result<(), Error>,
    {
        self.pass(
            |i, payload, _, _| on_msg(u32::try_from(i).unwrap_or(u32::MAX), payload),
            false,
        )
    }

    #[inline]
    fn echo(&mut self) -> Result<usize, Error> {
        self.pass(
            |i, payload, tx, stats| {
                // the frame is in the receive buffer: the echo is made in a
                // copy and queued
                let mut m: [u8; MESSAGE_LEN] = payload
                    .try_into()
                    .map_err(|_| ergon_latency_core::Error::MessageLength(payload.len()))?;
                echo_in_place(&mut m)?;
                if !tx.queue(&m)? {
                    return Err(Error::Protocol(format!(
                        "echo on stream {i}: send queue full"
                    )));
                }
                stats.frames_out += 1;
                Ok(())
            },
            // one write per read: every echo that read produced
            true,
        )
    }

    fn retrans(&self) -> Result<Option<u64>, Error> {
        let mut sum = 0u64;
        for c in &self.conns {
            sum += u64::from(sockopt::tcp_info(c.sock.as_raw_fd())?.total_retrans);
        }
        Ok(Some(sum))
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        let ts = match self.conns.first() {
            Some(c) => sockopt::tcp_info(c.sock.as_raw_fd())?.timestamps,
            None => false,
        };
        Ok(tcp_wire_bytes(ts, self.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let s = self.stats;
        let opt = |v: Option<u32>| v.map_or_else(|| "kernel".to_owned(), |b| b.to_string());
        for (k, v) in [
            ("tcp.max_frame", MAX_FRAME.to_string()),
            ("tcp.rx_buf", TCP_RX_BUF.to_string()),
            ("tcp.tx_buf", TCP_TX_BUF.to_string()),
            ("nodelay", "true".to_owned()),
            ("quickack_per_read", self.opts.quickack_per_read.to_string()),
            ("rto_min_us", opt(self.opts.rto_min_us)),
            ("delack_max_us", opt(self.opts.delack_max_us)),
            ("rcvbuf", self.opts.rcvbuf.to_string()),
            ("sndbuf", opt(self.opts.sndbuf)),
            ("busy_read_us", self.opts.busy_read_us.to_string()),
            ("epoll_events", self.opts.epoll_events.to_string()),
            ("stat.frames_in", s.frames_in.to_string()),
            ("stat.frames_out", s.frames_out.to_string()),
            ("stat.bytes_in", s.bytes_in.to_string()),
            ("stat.reads", s.reads.to_string()),
            ("stat.writes", s.writes.to_string()),
            ("stat.quickacks", s.quickacks.to_string()),
            ("stat.tx_full", s.tx_full.to_string()),
            ("stat.would_block", s.would_block.to_string()),
            ("stat.polls", s.polls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(c) = self.conns.first() {
            sockopt::read_back(c.sock.as_raw_fd(), true, out)?;
        }
        Ok(())
    }
}

/// A 64-byte message's bytes on the wire over TCP: frame header, payload,
/// TCP header (20, plus 12 with timestamps), IP and Ethernet.
#[must_use]
pub const fn tcp_wire_bytes(timestamps: bool, ipv6: bool) -> u64 {
    (FRAME_HEADER + MESSAGE_LEN) as u64 + 20 + if timestamps { 12 } else { 0 } + ip_overhead(ipv6)
}
