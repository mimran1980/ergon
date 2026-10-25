//! ergon-io UDP on the epoll-spin backend: one `UdpConn::connect` per
//! stream (stream `i` bound to local base port + `i`, connected to peer
//! base port + `i`) on one `Driver`.
//!
//! - Receive: `Driver::poll`; each datagram is handed over in
//!   `UdpHandler::on_datagram`, a slice of the connection's receive buffer.
//! - Send: `ConnMut::send` queues into the connection's send slots, then
//!   `Driver::flush` hands it to the kernel at once. All slots full is
//!   back-pressure.
//! - Echo: the handler has no driver to send with, so each ping is turned
//!   into its echo in a preallocated batch during the poll, and the batch is
//!   sent right after it.
//! - Driver keys: `epoll_events`, `datagram_burst`, `rcvbuf`, receive
//!   timestamps off (nothing reads them), keepalive and
//!   `TCP_USER_TIMEOUT` left alone (the mio arm sets neither).

use std::net::SocketAddr;

use ergon_io::{Backend, Config, ConnId, Driver, Handler, RxMeta, UdpConn, UdpHandler, UdpOptions};
use ergon_latency_core::message::{MESSAGE_LEN, echo_in_place};

use super::mio_udp::port_plus;
use super::{Error, Link, MAX_DATAGRAM, TX_SLOTS, ip_overhead, sockopt};

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
    /// `Config::loop_cpu`: the CPU the polling thread is pinned to.
    pub loop_cpu: Option<u32>,
    /// `Config::helper_cpus` (empty: ergon-io chooses).
    pub helper_cpus: Vec<u32>,
}

/// The driver configuration both UDP and TCP ergon-io arms start from:
/// epoll-spin, receive timestamps off, keepalive and `TCP_USER_TIMEOUT`
/// left as the socket has them (off), as the mio arms leave them.
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

/// A message received, by stream.
struct Deliver<'a, F> {
    stream_of: &'a [u32],
    on_msg: &'a mut F,
    err: Option<Error>,
}

impl<F> Handler for Deliver<'_, F> {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        if self.err.is_none() {
            self.err = Some(Error::Closed(format!("connection {id:?}: {error}")));
        }
    }
}

impl<F: FnMut(u32, &[u8]) -> Result<(), Error>> UdpHandler for Deliver<'_, F> {
    #[inline]
    fn on_datagram(&mut self, conn: ConnId, payload: &[u8], _meta: &RxMeta) {
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

/// The echoes one poll makes, sent right after it.
pub(crate) struct Echoes {
    pub(crate) batch: Box<[(ConnId, [u8; MESSAGE_LEN])]>,
    pub(crate) len: usize,
    pub(crate) err: Option<Error>,
}

impl Echoes {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            batch: vec![(ConnId::default(), [1u8; MESSAGE_LEN]); capacity.max(1)]
                .into_boxed_slice(),
            len: 0,
            err: None,
        }
    }

    /// Takes one ping and turns it into its echo.
    #[inline]
    pub(crate) fn take(&mut self, conn: ConnId, payload: &[u8]) {
        if self.err.is_some() {
            return;
        }
        let Some(slot) = self.batch.get_mut(self.len) else {
            self.err = Some(Error::Protocol("echo batch full".into()));
            return;
        };
        if payload.len() != MESSAGE_LEN {
            self.err = Some(Error::Core(ergon_latency_core::Error::MessageLength(
                payload.len(),
            )));
            return;
        }
        slot.1.copy_from_slice(payload);
        match echo_in_place(&mut slot.1) {
            Ok(_) => {
                slot.0 = conn;
                self.len += 1;
            }
            Err(e) => self.err = Some(e.into()),
        }
    }
}

impl Handler for Echoes {
    fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
        if self.err.is_none() {
            self.err = Some(Error::Closed(format!("connection {id:?}: {error}")));
        }
    }
}

impl UdpHandler for Echoes {
    #[inline]
    fn on_datagram(&mut self, conn: ConnId, payload: &[u8], _meta: &RxMeta) {
        self.take(conn, payload);
    }
}

/// N `UdpConn`s on one epoll-spin `Driver`.
pub struct EioUdp {
    d: Driver<UdpConn>,
    ids: Vec<ConnId>,
    stream_of: Vec<u32>,
    echoes: Echoes,
    opts: EioUdpOptions,
    ipv6: bool,
}

impl EioUdp {
    /// `streams` sockets: stream `i` bound to `local` + `i`, connected to
    /// `peer` + `i`. Call it on the polling thread, pinned.
    ///
    /// # Errors
    /// `Driver::new` (a key out of range), a bind or connect fails.
    pub fn new(
        local: SocketAddr,
        peer: SocketAddr,
        streams: u32,
        opts: EioUdpOptions,
    ) -> Result<Self, Error> {
        if streams == 0 {
            return Err(Error::Config("streams must be at least 1".into()));
        }
        let mut cfg = base_config(
            opts.rcvbuf,
            opts.epoll_events,
            opts.loop_cpu,
            &opts.helper_cpus,
        );
        cfg.datagram_burst = opts.datagram_burst;
        cfg.validate()?;
        let mut d: Driver<UdpConn> = Driver::new(cfg)?;
        let uopts = UdpOptions {
            max_datagram: MAX_DATAGRAM,
            tx_slots: TX_SLOTS,
            sndbuf: opts.sndbuf,
        };
        let mut ids = Vec::with_capacity(streams as usize);
        for i in 0..streams {
            let c = UdpConn::connect(port_plus(local, i)?, port_plus(peer, i)?, &uopts)?;
            ids.push(d.add(c)?);
        }
        let mut stream_of = vec![u32::MAX; ids.iter().map(|id| id.index() + 1).max().unwrap_or(0)];
        for (i, id) in (0u32..).zip(&ids) {
            stream_of[id.index()] = i;
        }
        let per_poll = (opts.epoll_events.min(streams) * opts.datagram_burst) as usize;
        Ok(Self {
            d,
            ids,
            stream_of,
            echoes: Echoes::new(per_poll),
            opts,
            ipv6: local.is_ipv6(),
        })
    }

    /// The driver, for its counters.
    #[must_use]
    pub const fn driver(&self) -> &Driver<UdpConn> {
        &self.d
    }
}

impl Link for EioUdp {
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
        match c.send(payload) {
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
            let mut sent = false;
            for _ in 0..2 {
                let mut c = self
                    .d
                    .get_mut(id)
                    .ok_or_else(|| Error::Closed(format!("connection {id:?}")))?;
                match c.send(&m) {
                    Ok(()) => {
                        sent = true;
                        break;
                    }
                    // every slot taken: hand them to the kernel, once
                    Err(ergon_io::Error::Busy(_)) => self.d.flush()?,
                    Err(e) => return Err(e.into()),
                }
            }
            if !sent {
                return Err(Error::Protocol(format!("echo on {id:?}: send slots full")));
            }
        }
        if n > 0 {
            self.d.flush()?;
        }
        Ok(n)
    }

    fn retrans(&self) -> Result<Option<u64>, Error> {
        Ok(None)
    }

    fn wire_bytes(&self) -> Result<u64, Error> {
        Ok(MESSAGE_LEN as u64 + 8 + ip_overhead(self.ipv6))
    }

    fn report(&self, out: &mut Vec<(String, String)>) -> Result<(), Error> {
        let cfg = self.d.config();
        let mut sum = ergon_io::UdpStats::default();
        for c in self.ids.iter().filter_map(|&id| self.d.get(id)) {
            let s = c.stats();
            sum.rx += s.rx;
            sum.tx += s.tx;
            sum.tx_full += s.tx_full;
            sum.truncated += s.truncated;
            sum.refused += s.refused;
        }
        let ds = self.d.stats();
        for (k, v) in [
            ("udp.max_datagram", MAX_DATAGRAM.to_string()),
            ("udp.tx_slots", TX_SLOTS.to_string()),
            (
                "rcvbuf",
                cfg.rcvbuf
                    .map_or_else(|| "kernel".to_owned(), |b| b.to_string()),
            ),
            (
                "sndbuf",
                self.opts
                    .sndbuf
                    .map_or_else(|| "kernel".to_owned(), |b| b.to_string()),
            ),
            ("epoll_events", cfg.epoll_events.to_string()),
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
        if let Some(c) = self.ids.first().and_then(|&id| self.d.get(id)) {
            sockopt::read_back(c.raw_fd(), false, out)?;
        }
        Ok(())
    }
}
