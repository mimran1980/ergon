//! UDP: one socket, whole datagrams, on the `epoll-spin` backend.
//!
//! - [`UdpConn::connect`]: bound to a local address and connected to one
//!   peer, so it sends with `send` and receives only that peer's datagrams.
//!   [`UdpConn::bind`]: receive-only, from any sender.
//! - Receiving: each datagram is one [`UdpHandler::on_datagram`] call, a
//!   slice of the receive buffer the kernel wrote into, with its [`RxMeta`]
//!   (kernel timestamps under `Config::rx_timestamps`). A 0-byte datagram is
//!   delivered, never taken for a close. A datagram longer than
//!   [`UdpOptions::max_datagram`] is dropped and counted
//!   ([`UdpStats::truncated`]). The driver reads at most
//!   `Config::datagram_burst` datagrams from one ready socket per poll.
//! - Sending: [`UdpConn::send`] copies the datagram into a ring of
//!   preallocated slots; the driver's next send pass hands each to the
//!   kernel whole. A full ring is [`Error::Busy`] ([`UdpStats::tx_full`]);
//!   nothing allocates after [`UdpConn::connect`].
//! - `ECONNREFUSED` (the peer's port answered ICMP unreachable) is counted
//!   ([`UdpStats::refused`]) and the socket stays open.
//! - On `smoltcp-spin` the socket is the stack's, `UdpConn<SmolUdp>`:
//!   `smoltcp_dpdk::Stack::udp_connect` and `udp_bind` make it.
//! - `Driver::<UdpConn>::new` refuses `recv-busy` and `uring-sqpoll`
//!   ([`Error::Unsupported`]). No TCP option, keepalive or quick-ACK route
//!   check applies; `SO_RCVBUF`, `SO_BUSY_POLL` and receive timestamps do.

use std::net::{SocketAddr, UdpSocket};
use std::os::fd::RawFd;

use crate::RxMeta;
use crate::endpoint::sealed::{Deliver, Io, RouteSlot};
use crate::endpoint::{ConnId, ConnMut, Handler};
use crate::error::{Error, Result};
use crate::net::DatagramSocket;
use crate::netstack::{Kernel, Sockets};
use crate::stream::TxQueue;

/// The largest UDP payload over IPv4 (65 535 less the IP and UDP headers).
pub const MAX_UDP_PAYLOAD: usize = 65_507;

/// Receives datagrams.
pub trait UdpHandler: Handler {
    /// One datagram from `conn`: its whole payload (possibly empty), a slice
    /// of the receive buffer valid for this call only.
    fn on_datagram(&mut self, conn: ConnId, payload: &[u8], meta: &RxMeta);
}

/// UDP settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpOptions {
    /// The longest datagram sent or received, bytes: the receive buffer and
    /// each send slot. A longer one received is dropped
    /// ([`UdpStats::truncated`]); a longer one offered to
    /// [`UdpConn::send`] is refused. 1 to 65 507, default 2 048.
    pub max_datagram: usize,
    /// Send slots: datagrams queued and not yet handed to the kernel. 1 to
    /// 65 536, default 64.
    pub tx_slots: usize,
    /// `SO_SNDBUF`, bytes (`None`, the default: the kernel's).
    pub sndbuf: Option<u32>,
}

impl Default for UdpOptions {
    fn default() -> Self {
        Self {
            max_datagram: 2_048,
            tx_slots: 64,
            sndbuf: None,
        }
    }
}

impl UdpOptions {
    fn check(&self) -> Result<()> {
        if !(1..=MAX_UDP_PAYLOAD).contains(&self.max_datagram) {
            return Err(Error::Config(format!(
                "udp max_datagram {}: 1 to {MAX_UDP_PAYLOAD}",
                self.max_datagram
            )));
        }
        if !(1..=65_536).contains(&self.tx_slots) {
            return Err(Error::Config(format!(
                "udp tx_slots {}: 1 to 65536",
                self.tx_slots
            )));
        }
        Ok(())
    }
}

/// Per-socket counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UdpStats {
    /// Datagrams delivered.
    pub rx: u64,
    /// Datagrams the kernel took.
    pub tx: u64,
    /// [`UdpConn::send`] calls refused because every slot was taken.
    pub tx_full: u64,
    /// Datagrams dropped for being longer than
    /// [`UdpOptions::max_datagram`].
    pub truncated: u64,
    /// `ECONNREFUSED` reports, on reads and sends.
    pub refused: u64,
}

/// A UDP socket a `Driver` owns. The socket closes when it is dropped.
///
/// `D` is the socket and so its stack ([`crate::net::DatagramSocket`]): the
/// kernel's [`UdpSocket`], or `SmolUdp` on `smoltcp-spin`.
pub struct UdpConn<D = UdpSocket> {
    sock: D,
    /// Connected to a peer: sends are possible.
    connected: bool,
    rx: Box<[u8]>,
    /// `tx_slots` slots of `max_datagram` bytes each.
    ring: Box<[u8]>,
    /// Per slot: the queued datagram's length.
    lens: Box<[usize]>,
    max: usize,
    /// The oldest queued slot, and how many are queued.
    head: usize,
    queued: usize,
    /// Always empty: datagrams leave from the ring, not a byte queue.
    empty: TxQueue,
    route: RouteSlot,
    stats: UdpStats,
}

impl UdpConn {
    /// A socket bound to `local` (port 0: the kernel picks one) and
    /// connected to `peer`: it sends to `peer` and receives only from it.
    ///
    /// # Errors
    /// Bad options, `local` and `peer` of different families, or socket
    /// creation, the bind or the connect fails.
    pub fn connect(local: SocketAddr, peer: SocketAddr, opts: &UdpOptions) -> Result<Self> {
        if local.is_ipv4() != peer.is_ipv4() {
            return Err(Error::Config(format!(
                "udp local {local} and peer {peer} are of different address families"
            )));
        }
        let sock = Self::socket(local, opts)?;
        sock.connect(peer)?;
        Self::build(sock, true, opts)
    }

    fn socket(local: SocketAddr, opts: &UdpOptions) -> Result<UdpSocket> {
        opts.check()?;
        Kernel::udp_bind(local, opts.sndbuf)
    }

    /// A receive-only socket bound to `local`: it takes datagrams from any
    /// sender, and [`UdpConn::send`] is refused.
    ///
    /// # Errors
    /// Bad options, or socket creation or the bind fails.
    pub fn bind(local: SocketAddr, opts: &UdpOptions) -> Result<Self> {
        let sock = Self::socket(local, opts)?;
        Self::build(sock, false, opts)
    }
}

impl<D: DatagramSocket> UdpConn<D> {
    /// A connection over `sock` (made by its stack), sending when
    /// `connected`.
    pub(crate) fn build(sock: D, connected: bool, opts: &UdpOptions) -> Result<Self> {
        opts.check()?;
        let max = opts.max_datagram;
        let ring_len = max
            .checked_mul(opts.tx_slots)
            .ok_or_else(|| Error::Config("udp send ring too large".into()))?;
        Ok(Self {
            sock,
            connected,
            // Non-zero fill writes every page now (zeroed memory is mapped
            // lazily), so the first reads and sends take no page faults.
            rx: vec![1u8; max].into_boxed_slice(),
            ring: vec![1u8; ring_len].into_boxed_slice(),
            lens: vec![0usize; opts.tx_slots].into_boxed_slice(),
            max,
            head: 0,
            queued: 0,
            empty: TxQueue::default(),
            route: RouteSlot::default(),
            stats: UdpStats::default(),
        })
    }

    /// Queues one datagram: copied into the next free slot, handed to the
    /// kernel whole by the driver's next send pass (`Driver::poll` or
    /// `Driver::flush`). A 0-byte datagram is a datagram.
    ///
    /// # Errors
    /// [`Error::Busy`] when every slot holds a datagram not yet sent;
    /// [`Error::Config`] for a payload longer than
    /// [`UdpOptions::max_datagram`] or a receive-only socket.
    #[inline]
    pub fn send(&mut self, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Config(
                "udp: a receive-only socket (UdpConn::bind) has no peer to send to".into(),
            ));
        }
        if payload.len() > self.max {
            return Err(over_max_datagram(payload.len(), self.max));
        }
        if self.queued == self.lens.len() {
            std::hint::cold_path();
            self.stats.tx_full += 1;
            return Err(Error::Busy("udp send slots full"));
        }
        let slot = (self.head + self.queued) % self.lens.len();
        let at = slot * self.max;
        self.ring[at..at + payload.len()].copy_from_slice(payload);
        self.lens[slot] = payload.len();
        self.queued += 1;
        Ok(())
    }

    /// Datagrams queued and not yet handed to the kernel.
    #[must_use]
    pub const fn queued(&self) -> usize {
        self.queued
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> UdpStats {
        self.stats
    }

    /// The bound address.
    ///
    /// # Errors
    /// `getsockname` fails.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.sock.local_addr()
    }

    /// The peer, for a connected socket.
    #[must_use]
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.sock.peer_addr()
    }

    /// The socket descriptor (for inspection; the driver owns the socket).
    #[must_use]
    pub fn raw_fd(&self) -> RawFd {
        self.sock.fd()
    }

    /// `SO_RCVBUF` as the kernel granted it (twice the size it keeps); on
    /// `smoltcp-spin`, the stack's UDP buffer (`StackConfig::udp_buf`).
    ///
    /// # Errors
    /// `getsockopt` fails.
    pub fn recv_buffer_size(&self) -> Result<usize> {
        self.sock.recv_buffer_size()
    }
}

/// A driver-owned [`UdpConn`]: the same send.
impl<D: DatagramSocket> ConnMut<'_, UdpConn<D>> {
    /// [`UdpConn::send`].
    ///
    /// # Errors
    /// As [`UdpConn::send`].
    #[inline]
    pub fn send(&mut self, payload: &[u8]) -> Result<()> {
        self.inner().send(payload)
    }
}

impl<D: DatagramSocket> Io for UdpConn<D> {
    const DATAGRAM: bool = true;
    const STACK: crate::netstack::StackId = D::STACK;

    #[inline]
    fn fd(&self) -> RawFd {
        self.sock.fd()
    }
    #[inline]
    fn rx_space(&mut self) -> &mut [u8] {
        &mut self.rx
    }
    #[inline]
    fn tx_queue(&mut self) -> &mut TxQueue {
        &mut self.empty
    }
    fn has_buffered_input(&self) -> bool {
        false
    }
    fn set_route(&mut self, route: RouteSlot) {
        self.route = route;
    }
    fn route_state(&self) -> &RouteSlot {
        &self.route
    }
    #[inline]
    fn pending_tx(&mut self) -> Option<&[u8]> {
        if self.queued == 0 {
            return None;
        }
        let at = self.head * self.max;
        self.ring.get(at..at + self.lens[self.head])
    }
    #[inline]
    fn sent_tx(&mut self) {
        if self.queued > 0 {
            self.head = (self.head + 1) % self.lens.len();
            self.queued -= 1;
            self.stats.tx += 1;
        }
    }
    #[inline]
    fn has_tx(&mut self) -> bool {
        self.queued > 0
    }
    #[inline]
    fn on_truncated(&mut self) {
        self.stats.truncated += 1;
    }
    #[inline]
    fn on_refused(&mut self) {
        self.stats.refused += 1;
    }
}

impl<D: DatagramSocket, H: UdpHandler> Deliver<H> for UdpConn<D> {
    #[inline]
    fn on_received(
        &mut self,
        id: ConnId,
        n: usize,
        meta: &RxMeta,
        handler: &mut H,
    ) -> Result<usize> {
        // The kernel wrote `n` bytes into `rx_space`, the whole buffer.
        handler.on_datagram(id, &self.rx[..n.min(self.max)], meta);
        self.stats.rx += 1;
        Ok(1)
    }
}
/// The refusal for a datagram over `max_datagram`, built out of line.
#[cold]
#[inline(never)]
fn over_max_datagram(len: usize, max: usize) -> Error {
    Error::Config(format!(
        "udp datagram of {len} bytes is over max_datagram {max}"
    ))
}
