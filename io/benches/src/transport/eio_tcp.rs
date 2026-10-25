//! ergon-io framed TCP on the epoll-spin backend: one `FrameConn` per
//! stream on one `Driver`, the same u32 LE length frames as
//! `ergon_latency_core::frame`.
//!
//! - Receive: `Driver::poll`; each frame is handed over in
//!   `FrameHandler::on_frame`, in place in the connection's receive buffer
//!   ([`TCP_RX_BUF`](super::TCP_RX_BUF) bytes, as the mio arm's).
//! - Send: `ConnMut::send_frame` queues header and payload
//!   ([`TCP_TX_BUF`](super::TCP_TX_BUF)), then `Driver::flush` writes at
//!   once. A full queue is back-pressure.
//! - Echo: as the UDP arm, a preallocated batch made during the poll, sent
//!   right after it.
//! - Driver keys: `rcvbuf`, `epoll_events`, `tcp_rto_min_us`,
//!   `tcp_delack_max_us` (checked against the kernel at `Driver::new`);
//!   `FrameOptions::quickack_per_read`, `sndbuf`. `TCP_NODELAY` always.

use std::net::TcpStream;

use ergon_io::{ConnId, Driver, FrameConn, FrameHandler, FrameOptions, Handler, RxMeta};
use ergon_latency_core::frame::FRAME_HEADER;
use ergon_latency_core::message::MESSAGE_LEN;

use super::eio_udp::{Echoes, base_config};
use super::mio_tcp::tcp_wire_bytes;
use super::{Error, Link, MAX_FRAME, TCP_RX_BUF, TCP_TX_BUF, sockopt};

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

struct Deliver<'a, F> {
    stream_of: &'a [u32],
    on_msg: &'a mut F,
    err: Option<Error>,
}

impl<F> Handler for Deliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        if self.err.is_none() {
            self.err = Some(match error {
                ergon_io::Error::Closed => {
                    Error::Closed(format!("connection {id:?}: the peer closed"))
                }
                e => Error::Closed(format!("connection {id:?}: {e}")),
            });
        }
    }
}

impl<F: FnMut(u32, &[u8]) -> Result<(), Error>> FrameHandler for Deliver<'_, F> {
    #[inline]
    fn on_frame(&mut self, conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        if self.err.is_some() {
            return;
        }
        let stream = self
            .stream_of
            .get(conn.index())
            .copied()
            .unwrap_or(u32::MAX);
        if let Err(e) = (self.on_msg)(stream, payload) {
            self.err = Some(e);
        }
    }
}

impl FrameHandler for Echoes {
    #[inline]
    fn on_frame(&mut self, conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        self.take(conn, payload);
    }
}

/// N `FrameConn`s on one epoll-spin `Driver`.
pub struct EioTcp {
    d: Driver<FrameConn>,
    ids: Vec<ConnId>,
    stream_of: Vec<u32>,
    echoes: Echoes,
    opts: EioTcpOptions,
    ipv6: bool,
}

impl EioTcp {
    /// Takes connected sockets, stream `i` the `i`-th. Call it on the
    /// polling thread, pinned.
    ///
    /// # Errors
    /// `Driver::new` (a key out of range, a timer the kernel refuses or
    /// rounds), or adding a connection fails.
    pub fn new(socks: Vec<TcpStream>, opts: EioTcpOptions) -> Result<Self, Error> {
        if socks.is_empty() {
            return Err(Error::Config("streams must be at least 1".into()));
        }
        let ipv6 = socks[0].local_addr()?.is_ipv6();
        let mut cfg = base_config(
            opts.rcvbuf,
            opts.epoll_events,
            opts.loop_cpu,
            &opts.helper_cpus,
        );
        cfg.tcp_rto_min_us = opts.rto_min_us;
        cfg.tcp_delack_max_us = opts.delack_max_us;
        cfg.validate()?;
        let mut d: Driver<FrameConn> = Driver::new(cfg)?;
        let fopts = FrameOptions {
            max_frame: MAX_FRAME,
            rx_buf: TCP_RX_BUF,
            tx_buf: TCP_TX_BUF,
            sndbuf: opts.sndbuf,
            quickack_per_read: opts.quickack_per_read,
        };
        let streams = u32::try_from(socks.len()).unwrap_or(u32::MAX);
        let mut ids = Vec::with_capacity(socks.len());
        for s in socks {
            ids.push(d.add(FrameConn::from_stream(s, fopts)?)?);
        }
        let mut stream_of = vec![u32::MAX; ids.iter().map(|id| id.index() + 1).max().unwrap_or(0)];
        for (i, id) in (0u32..).zip(&ids) {
            stream_of[id.index()] = i;
        }
        // one read per ready connection per poll, of at most the receive
        // buffer: at most this many whole frames
        let per_read = TCP_RX_BUF / (FRAME_HEADER + MESSAGE_LEN) + 1;
        let per_poll = opts.epoll_events.min(streams) as usize * per_read;
        Ok(Self {
            d,
            ids,
            stream_of,
            echoes: Echoes::new(per_poll),
            opts,
            ipv6,
        })
    }

    /// The driver, for its counters.
    #[must_use]
    pub const fn driver(&self) -> &Driver<FrameConn> {
        &self.d
    }
}

impl Link for EioTcp {
    fn streams(&self) -> u32 {
        u32::try_from(self.ids.len()).unwrap_or(u32::MAX)
    }

    #[inline]
    fn send(&mut self, stream: u32, payload: &[u8]) -> Result<bool, Error> {
        let id = *self
            .ids
            .get(stream as usize)
            .ok_or_else(|| Error::Protocol(format!("stream {stream} of {}", self.ids.len())))?;
        let mut c = self
            .d
            .get_mut(id)
            .ok_or_else(|| Error::Closed(format!("stream {stream}")))?;
        match c.send_frame(payload) {
            Ok(()) => {
                self.d.flush()?;
                Ok(true)
            }
            Err(ergon_io::Error::Busy(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    #[inline]
    fn poll<F>(&mut self, on_msg: &mut F) -> Result<usize, Error>
    where
        F: FnMut(u32, &[u8]) -> Result<(), Error>,
    {
        let mut h = Deliver {
            stream_of: &self.stream_of,
            on_msg,
            err: None,
        };
        let n = self.d.poll(&mut h)?;
        h.err.map_or(Ok(n), Err)
    }

    #[inline]
    fn echo(&mut self) -> Result<usize, Error> {
        self.echoes.len = 0;
        self.d.poll(&mut self.echoes)?;
        if let Some(e) = self.echoes.err.take() {
            return Err(e);
        }
        let n = self.echoes.len;
        for k in 0..n {
            let (id, m) = self.echoes.batch[k];
            let mut c = self
                .d
                .get_mut(id)
                .ok_or_else(|| Error::Closed(format!("connection {id:?}")))?;
            match c.send_frame(&m) {
                Ok(()) => {}
                Err(ergon_io::Error::Busy(_)) => {
                    return Err(Error::Protocol(format!("echo on {id:?}: send queue full")));
                }
                Err(e) => return Err(e.into()),
            }
        }
        if n > 0 {
            self.d.flush()?;
        }
        Ok(n)
    }

    fn retrans(&self) -> Result<Option<u64>, Error> {
        let mut sum = 0u64;
        for c in self.ids.iter().filter_map(|&id| self.d.get(id)) {
            sum += u64::from(sockopt::tcp_info(c.raw_fd())?.total_retrans);
        }
        Ok(Some(sum))
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        let ts = match self.ids.first().and_then(|&id| self.d.get(id)) {
            Some(c) => sockopt::tcp_info(c.raw_fd())?.timestamps,
            None => false,
        };
        Ok(tcp_wire_bytes(ts, self.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let cfg = self.d.config();
        let opt = |v: Option<u32>| v.map_or_else(|| "kernel".to_owned(), |b| b.to_string());
        let mut frames_in = 0u64;
        let mut frames_out = 0u64;
        let mut bytes_in = 0u64;
        let mut quickacks = 0u64;
        let mut compact = 0u64;
        for c in self.ids.iter().filter_map(|&id| self.d.get(id)) {
            let s = c.stats();
            frames_in += s.frames_in;
            frames_out += s.frames_out;
            bytes_in += s.bytes_in;
            quickacks += s.quickacks;
            compact += s.compact_bytes;
        }
        let ds = self.d.stats();
        for (k, v) in [
            ("tcp.max_frame", MAX_FRAME.to_string()),
            ("tcp.rx_buf", TCP_RX_BUF.to_string()),
            ("tcp.tx_buf", TCP_TX_BUF.to_string()),
            ("nodelay", "true".to_owned()),
            ("quickack_per_read", self.opts.quickack_per_read.to_string()),
            ("rto_min_us", opt(cfg.tcp_rto_min_us)),
            ("delack_max_us", opt(cfg.tcp_delack_max_us)),
            ("rcvbuf", opt(cfg.rcvbuf)),
            ("sndbuf", opt(self.opts.sndbuf)),
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
        if let Some(c) = self.ids.first().and_then(|&id| self.d.get(id)) {
            sockopt::read_back(c.raw_fd(), true, out)?;
        }
        Ok(())
    }
}
