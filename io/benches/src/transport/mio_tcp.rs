//! Raw mio TCP: a minimal `mio::Poll` loop over one `mio::net::TcpStream`
//! per stream, framed with `ergon_latency_core::frame` (u32 LE length,
//! then the payload).
//!
//! - Receive ([`MioTcpRx`]): `epoll_wait(0)`; each ready connection is
//!   read into its `Deframer` ([`TCP_RX_BUF`](super::TCP_RX_BUF) bytes, the
//!   room ergon-io's framed TCP reads into) and its whole frames handed
//!   over in place, up to [`POLL_LIMIT`](super::point::POLL_LIMIT) a pass. mio
//!   registers edge-triggered, so a connection not yet read to
//!   `WouldBlock` (or with frames still held) stays on the ready list for
//!   the next pass. With `quickack_per_read`, `TCP_QUICKACK` after every
//!   read with data. A peer's close takes its connection off the poll;
//!   [`Rx::closed`] once every one is.
//! - Send ([`MioTcpTx`]): the frame is encoded into the connection's
//!   preallocated send queue ([`TCP_TX_BUF`](super::TCP_TX_BUF)) and
//!   written at once; what the kernel has no room for goes on the next
//!   pass. A full queue is busy.
//! - Socket options by `setsockopt`: `TCP_NODELAY`, `SO_RCVBUF`,
//!   `SO_SNDBUF`, `SO_BUSY_POLL`, `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US`
//!   (read back; refused on a kernel without them). Nothing allocates after
//!   [`pair`].
//!
//! The connections, their send queues and the per-stream read are
//! [`MioTcpStreams`] and [`MioTcpReader`], built on any `mio::Registry`
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

use std::cell::RefCell;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream as StdTcpStream;
use std::os::fd::AsRawFd;
use std::rc::Rc;
use std::time::Duration;

use ergon_latency_core::frame::{self, Deframer, FRAME_HEADER};
use ergon_latency_core::message::MESSAGE_LEN;
use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Registry, Token};

use super::point::{Error, POLL_LIMIT, Rx, Sent, Tx};
use super::sock::Inspect;
use super::sockopt::{self, TcpCounters, os};
use super::{MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF, ip_overhead};

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
struct TxQueue {
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
                Ok(0) => return Err(Error::Transport("tcp write returned 0".into())),
                Ok(n) => self.head += n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    stats.would_block += 1;
                    return Ok(true);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(os("tcp write", e)),
            }
        }
        self.head = 0;
        self.tail = 0;
        Ok(false)
    }
}

struct TxState {
    queues: Vec<TxQueue>,
    dirty: Vec<u32>,
    in_dirty: Vec<bool>,
    stats: MioTcpStats,
}

impl TxState {
    #[inline]
    fn flush(&mut self, socks: &[TcpStream]) -> Result<(), Error> {
        let mut k = 0;
        while let Some(&s) = self.dirty.get(k) {
            let i = s as usize;
            if self.queues[i].write_out(&socks[i], &mut self.stats)? {
                k += 1;
            } else {
                self.in_dirty[i] = false;
                self.dirty.swap_remove(k);
            }
        }
        Ok(())
    }
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
    s.set_nonblocking(true).map_err(|e| os("O_NONBLOCK", e))
}

/// The mio TCP arm's connections and send side.
///
/// Stream `i` is the `i`-th connection, registered readable on a caller's
/// `Registry` as `Token(token_base + i)`. Every method takes `&self`, so a
/// read's callback may send (an echo).
pub struct MioTcpStreams {
    socks: Vec<TcpStream>,
    tx: RefCell<TxState>,
    opts: MioTcpOptions,
    ipv6: bool,
    token_base: usize,
}

impl MioTcpStreams {
    /// Takes connected sockets, stream `i` the `i`-th: [`prepare`]d and
    /// registered on `registry` from `Token(token_base)`.
    ///
    /// # Errors
    /// [`prepare`] fails, registering one does, or none is given.
    pub fn new(
        registry: &Registry,
        token_base: usize,
        socks: Vec<StdTcpStream>,
        opts: &MioTcpOptions,
    ) -> Result<Self, Error> {
        if socks.is_empty() {
            return Err(Error::Usage("streams must be at least 1".into()));
        }
        let n = socks.len();
        let ipv6 = socks[0]
            .local_addr()
            .map_err(|e| os("getsockname", e))?
            .is_ipv6();
        let mut mio_socks = Vec::with_capacity(n);
        for (i, s) in socks.into_iter().enumerate() {
            prepare(&s, opts)?;
            let mut sock = TcpStream::from_std(s);
            registry
                .register(&mut sock, Token(token_base + i), Interest::READABLE)
                .map_err(|e| os("epoll_ctl", e))?;
            mio_socks.push(sock);
        }
        Ok(Self {
            socks: mio_socks,
            tx: RefCell::new(TxState {
                queues: (0..n).map(|_| TxQueue::new()).collect(),
                dirty: Vec::with_capacity(n),
                in_dirty: vec![false; n],
                stats: MioTcpStats::default(),
            }),
            opts: opts.clone(),
            ipv6,
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

    /// Frames `payload` onto `stream`'s send queue and writes it at once;
    /// what the kernel has no room for goes on [`flush`](Self::flush).
    /// [`Sent::Busy`] when the queue is full.
    ///
    /// # Errors
    /// No such stream, a frame too long, or the write failed for good.
    #[inline]
    pub fn send(&self, stream: u32, payload: &[u8]) -> Result<Sent, Error> {
        let i = stream as usize;
        let mut tx = self.tx.borrow_mut();
        let tx = &mut *tx;
        let (Some(sock), Some(q)) = (self.socks.get(i), tx.queues.get_mut(i)) else {
            return Err(Error::Transport(format!(
                "stream {i} of {}",
                self.socks.len()
            )));
        };
        if !q.queue(payload)? {
            tx.stats.tx_full += 1;
            return Ok(Sent::Busy);
        }
        tx.stats.frames_out += 1;
        if q.write_out(sock, &mut tx.stats)? && !tx.in_dirty[i] {
            tx.in_dirty[i] = true;
            tx.dirty.push(stream);
        }
        Ok(Sent::Sent)
    }

    /// Every queued byte to the kernel, until a connection has no room.
    ///
    /// # Errors
    /// A write failed for good.
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
    pub fn tx_stats(&self) -> MioTcpStats {
        self.tx.borrow().stats
    }

    /// The TCP counters so far, summed over every connection (`TCP_INFO`).
    ///
    /// # Errors
    /// `getsockopt` fails.
    pub fn tcp_counters(&self) -> Result<TcpCounters, Error> {
        TcpCounters::of(self.socks.iter().map(AsRawFd::as_raw_fd))
    }

    /// Bytes on the wire per 64-byte message.
    ///
    /// # Errors
    /// `TCP_INFO` cannot be read.
    pub fn wire_bytes(&self) -> Result<u64, Error> {
        let ts = match self.socks.first() {
            Some(s) => sockopt::tcp_info(s.as_raw_fd())?.timestamps,
            None => false,
        };
        Ok(tcp_wire_bytes(ts, self.ipv6))
    }

    /// The settings and counters (`reader`'s receive side), and stream 0's
    /// socket options read back.
    ///
    /// # Errors
    /// A read-back fails.
    pub fn report(
        &self,
        reader: &MioTcpReader,
        out: &mut Vec<(String, String)>,
    ) -> Result<(), Error> {
        let t = self.tx_stats();
        let r = reader.stats;
        let o = &self.opts;
        let opt = |v: Option<u32>| v.map_or_else(|| "kernel".to_owned(), |b| b.to_string());
        for (k, v) in [
            ("tcp.max_frame", MAX_FRAME.to_string()),
            ("tcp.rx_buf", TCP_RX_BUF.to_string()),
            ("tcp.tx_buf", TCP_TX_BUF.to_string()),
            ("tcp.frames_per_pass", POLL_LIMIT.to_string()),
            ("nodelay", "true".to_owned()),
            ("quickack_per_read", o.quickack_per_read.to_string()),
            ("rto_min_us", opt(o.rto_min_us)),
            ("delack_max_us", opt(o.delack_max_us)),
            ("rcvbuf", o.rcvbuf.to_string()),
            ("sndbuf", opt(o.sndbuf)),
            ("busy_read_us", o.busy_read_us.to_string()),
            ("epoll_events", o.epoll_events.to_string()),
            ("stat.frames_in", r.frames_in.to_string()),
            ("stat.frames_out", t.frames_out.to_string()),
            ("stat.bytes_in", r.bytes_in.to_string()),
            ("stat.reads", r.reads.to_string()),
            ("stat.writes", t.writes.to_string()),
            ("stat.quickacks", r.quickacks.to_string()),
            ("stat.tx_full", t.tx_full.to_string()),
            ("stat.would_block", t.would_block.to_string()),
            ("stat.polls", r.polls.to_string()),
        ] {
            out.push((k.to_owned(), v));
        }
        if let Some(s) = self.socks.first() {
            sockopt::read_back(s.as_raw_fd(), true, out)?;
        }
        Ok(())
    }
}

/// What one [`MioTcpReader::read`] of a stream did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpRead {
    /// Frames handed over.
    pub frames: usize,
    /// Read to `WouldBlock` (or the peer's close) with every frame handed
    /// over: no need to read again before its next event.
    pub done: bool,
    /// The peer closed it in this read (the first time only).
    pub closed: bool,
}

/// A receive side's deframers (one per stream, every page touched up front),
/// closed streams and counters: reads one stream of a [`MioTcpStreams`] at a
/// time.
pub struct MioTcpReader {
    de: Vec<Deframer>,
    closed: Vec<bool>,
    open: usize,
    /// The receive side's counters (`polls` is the caller's to count).
    pub stats: MioTcpStats,
}

impl MioTcpReader {
    /// One deframer per stream of `streams`.
    ///
    /// # Errors
    /// A deframer of another size than [`TCP_RX_BUF`].
    pub fn new(streams: &MioTcpStreams) -> Result<Self, Error> {
        let n = streams.socks.len();
        let mut de = Vec::with_capacity(n);
        for _ in 0..n {
            let mut d = Deframer::new(MAX_FRAME)?;
            // touch the receive buffer now: no page fault on the first reads
            let spare = d.spare();
            if spare.len() != TCP_RX_BUF {
                return Err(Error::Transport(format!(
                    "deframer holds {} bytes, not {TCP_RX_BUF}",
                    spare.len()
                )));
            }
            spare.fill(1);
            de.push(d);
        }
        Ok(Self {
            de,
            closed: vec![false; n],
            open: n,
            stats: MioTcpStats::default(),
        })
    }

    /// Whether `stream`'s peer closed it (it is not read again).
    #[must_use]
    pub fn is_closed(&self, stream: u32) -> bool {
        self.closed.get(stream as usize).copied().unwrap_or(true)
    }

    /// Whether every peer closed.
    #[must_use]
    pub const fn all_closed(&self) -> bool {
        self.open == 0
    }

    /// Reads `stream` and hands its whole frames to `on`, up to
    /// [`POLL_LIMIT`]; with `quickack_per_read`, `TCP_QUICKACK` after every
    /// read with data. mio is edge-triggered: a stream not `done` must be
    /// read again without an event.
    ///
    /// # Errors
    /// No such stream, a malformed frame, a full receive buffer, or the read
    /// failed for good.
    #[inline]
    pub fn read<F: FnMut(&[u8])>(
        &mut self,
        streams: &MioTcpStreams,
        stream: u32,
        on: &mut F,
    ) -> Result<TcpRead, Error> {
        let i = stream as usize;
        let (Some(sock), Some(de)) = (streams.socks.get(i), self.de.get_mut(i)) else {
            return Err(Error::Transport(format!(
                "stream {i} of {}",
                streams.socks.len()
            )));
        };
        let quickack = streams.opts.quickack_per_read;
        let mut delivered = 0;
        let mut done = false;
        let mut peer_closed = false;
        loop {
            while delivered < POLL_LIMIT {
                match de.next_frame()? {
                    Some(f) => {
                        self.stats.frames_in += 1;
                        delivered += 1;
                        on(f);
                    }
                    None => break,
                }
            }
            if delivered == POLL_LIMIT {
                break;
            }
            let spare = de.spare();
            if spare.is_empty() {
                return Err(Error::Transport(format!("stream {i}: receive buffer full")));
            }
            self.stats.reads += 1;
            match (&*sock).read(spare) {
                Ok(0) => {
                    peer_closed = true;
                    done = true;
                    break;
                }
                Ok(len) => {
                    de.filled(len)?;
                    self.stats.bytes_in += len as u64;
                    if quickack {
                        sockopt::quickack(sock.as_raw_fd()).map_err(|e| os("TCP_QUICKACK", e))?;
                        self.stats.quickacks += 1;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    done = true;
                    break;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) if e.kind() == ErrorKind::ConnectionReset => {
                    peer_closed = true;
                    done = true;
                    break;
                }
                Err(e) => return Err(os("tcp read", e)),
            }
        }
        // the peer's close: not read again (edge-triggered, its end of file
        // reports no more events); the socket stays open, the send side
        // holds it
        let closed = peer_closed && !self.closed[i];
        if closed {
            self.closed[i] = true;
            self.open -= 1;
        }
        Ok(TcpRead {
            frames: delivered,
            done,
            closed,
        })
    }
}

/// The sending side.
pub struct MioTcpTx {
    s: Rc<MioTcpStreams>,
}

/// The receiving side.
pub struct MioTcpRx {
    s: Rc<MioTcpStreams>,
    poll: Poll,
    events: Events,
    reader: MioTcpReader,
    ready: Vec<u32>,
    in_ready: Vec<bool>,
}

/// Takes connected sockets, stream `i` the `i`-th; their sending and
/// receiving sides, on a poll of their own.
///
/// # Errors
/// [`prepare`] fails, or registering one does.
pub fn pair(socks: Vec<StdTcpStream>, opts: &MioTcpOptions) -> Result<(MioTcpTx, MioTcpRx), Error> {
    if socks.is_empty() || opts.epoll_events == 0 {
        return Err(Error::Usage(
            "streams and epoll events must be at least 1".into(),
        ));
    }
    let poll = Poll::new().map_err(|e| os("epoll_create", e))?;
    let n = socks.len();
    let s = Rc::new(MioTcpStreams::new(poll.registry(), 0, socks, opts)?);
    let reader = MioTcpReader::new(&s)?;
    Ok((
        MioTcpTx { s: Rc::clone(&s) },
        MioTcpRx {
            s,
            poll,
            events: Events::with_capacity(opts.epoll_events),
            reader,
            ready: Vec::with_capacity(n),
            in_ready: vec![false; n],
        },
    ))
}

impl Tx for MioTcpTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        self.s.send(stream, msg)
    }
}

impl Rx for MioTcpRx {
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
                if !self.in_ready[i] && !self.reader.is_closed(r) {
                    self.in_ready[i] = true;
                    self.ready.push(r);
                }
            }
        }
        let mut n = 0;
        let mut k = 0;
        while let Some(&r) = self.ready.get(k) {
            let read = self.reader.read(s, r, on)?;
            n += read.frames;
            if read.done {
                self.in_ready[r as usize] = false;
                self.ready.swap_remove(k);
            } else {
                k += 1;
            }
        }
        s.flush()?;
        Ok(n)
    }

    fn closed(&self) -> bool {
        self.reader.all_closed()
    }
}

impl Inspect for MioTcpRx {
    fn tcp_counters(&self) -> Result<Option<TcpCounters>, Error> {
        self.s.tcp_counters().map(Some)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        self.s.wire_bytes()
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        self.s.report(&self.reader, out)
    }
}

/// A 64-byte message's bytes on the wire over TCP: frame header, payload,
/// TCP header (20, plus 12 with timestamps), IP and Ethernet.
#[must_use]
pub const fn tcp_wire_bytes(timestamps: bool, ipv6: bool) -> u64 {
    (FRAME_HEADER + MESSAGE_LEN) as u64 + 20 + if timestamps { 12 } else { 0 } + ip_overhead(ipv6)
}
