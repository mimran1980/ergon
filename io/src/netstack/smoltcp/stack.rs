//! The stack: smoltcp's interface and sockets over the DPDK device, the
//! sockets' handles, and the housekeeping that runs between passes.
//!
//! A socket's handle (the `c_int` the seam carries, `SmolTcp`'s and
//! `SmolUdp`'s) is the slot it holds in the low 16 bits and that slot's
//! generation above them: a handle of a closed socket never reaches the
//! socket that took its slot next. No kernel descriptor is involved.

use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::fd::RawFd;

use smoltcp::iface::{
    Config as IfaceConfig, Interface, PollResult, SocketHandle, SocketSet, SocketStorage,
};
use smoltcp::socket::{tcp, udp};
use smoltcp::time::{Duration as SmolDuration, Instant};
use smoltcp::wire::{
    EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpListenEndpoint, Ipv4Address,
};

use super::port::{self, Dev, Port};
use crate::clock::monotonic_ns;
use crate::smoltcp_dpdk::{Congestion, StackConfig, StackStats, TcpStats};

/// Ephemeral ports: IANA's dynamic range.
const EPHEMERAL: std::ops::RangeInclusive<u16> = 49_152..=65_535;
/// Housekeeping every millisecond of polling.
const HOUSEKEEPING_NS: i64 = 1_000_000;
/// The PMDs' timers (ENA's watchdog) run this often.
const TIMERS_NS: i64 = 100_000_000;
/// smoltcp's neighbour entry lifetime (`neighbor::Cache::ENTRY_LIFETIME`),
/// in its own clock.
const NEIGHBOR_LIFETIME_US: u64 = 60_000_000;
/// A closed connection still finishing (FIN exchange) is aborted after this
/// long without a segment from the peer, real time, unless its user timeout
/// is shorter.
const LINGER_S: u64 = 10;

/// smoltcp's clock: real time since the stack started, `scale` times faster.
#[derive(Clone, Copy)]
pub struct Clock {
    start_ns: i64,
    /// Virtual µs per real ns, 32-bit fixed point: `1000 / rto_min_us`.
    mult: u128,
    rto_min_us: u32,
}

impl Clock {
    pub fn new(rto_min_us: u32, now_ns: i64) -> Self {
        Self {
            start_ns: now_ns,
            mult: (1000u128 << 32) / u128::from(rto_min_us.max(1)),
            rto_min_us,
        }
    }

    /// smoltcp's time at real time `ns` (`CLOCK_MONOTONIC`).
    pub fn at(&self, ns: i64) -> Instant {
        let real = u128::try_from(ns.saturating_sub(self.start_ns)).unwrap_or(0);
        Instant::from_micros(i64::try_from((real * self.mult) >> 32).unwrap_or(i64::MAX))
    }

    /// A real duration in smoltcp's clock.
    pub fn scaled(&self, real_us: u64) -> SmolDuration {
        let v = u128::from(real_us) * 1_000_000 / u128::from(self.rto_min_us.max(1));
        SmolDuration::from_micros(u64::try_from(v).unwrap_or(u64::MAX))
    }

    /// A duration of smoltcp's clock in real ns.
    pub fn real_ns(&self, virt_us: u64) -> i64 {
        let ns = u128::from(virt_us) * u128::from(self.rto_min_us) / 1_000;
        i64::try_from(ns).unwrap_or(i64::MAX)
    }
}

/// Who holds a socket.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    /// A dial or a new `UdpConn`, not yet handed to a driver.
    Setup,
    /// A driver's poller, reporting it as `token`.
    Driver { poller: u32, token: u64 },
}

struct Slot {
    fd: RawFd,
    handle: SocketHandle,
    udp: bool,
    owner: Owner,
    /// Stamp reads with the burst time.
    stamps: bool,
    /// A connected UDP socket's peer.
    peer: Option<IpEndpoint>,
    port: u16,
}

/// The stack's counters kept outside the device.
#[derive(Default)]
struct Counters {
    polls: u64,
    arp_refreshes: u64,
    arp_pinned: u64,
    housekeeping: u64,
    housekeeping_max_ns: u64,
}

pub struct Inner {
    iface: Interface,
    pub dev: Dev,
    sockets: SocketSet<'static>,
    slots: Vec<Option<Slot>>,
    free: Vec<usize>,
    /// The next handle's generation (15 bits).
    generation: u32,
    /// Ports in use, one bit each.
    ports: Box<[u64]>,
    next_port: u16,
    pub clock: Clock,
    cfg: StackConfig,
    ip: Ipv4Addr,
    mac: [u8; 6],
    pub port_id: u16,
    pub driver: String,
    /// Per poller: a socket may be ready (a pass changed state, or a socket
    /// was ready last time and level triggering reports it again).
    rescan: Vec<bool>,
    zombies: Vec<(SocketHandle, u16)>,
    /// Closed TCP and UDP sockets kept for reuse, buffers and all.
    spare_tcp: Vec<SocketHandle>,
    spare_udp: Vec<SocketHandle>,
    next_hk_ns: i64,
    next_keep_ns: i64,
    next_timers_ns: i64,
    keep_ns: i64,
    counters: Counters,
    /// Scratch for the neighbour refresh (kept to avoid an allocation).
    neighbors: Vec<Ipv4Addr>,
}

fn tsval() -> u32 {
    // RFC 7323's clock: milliseconds, wrapping.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ms = (monotonic_ns() / 1_000_000) as u32;
    ms
}

pub fn no_slot() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "not a socket of this smoltcp stack",
    )
}

impl Inner {
    pub fn new(cfg: &StackConfig, port: &Port) -> io::Result<Self> {
        let mut dev = Dev::new(port);
        let mut seed = [0u8; 8];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut seed)
            .map_err(|_| io::Error::other("no system randomness"))?;
        let mut icfg = IfaceConfig::new(HardwareAddress::Ethernet(EthernetAddress(port.mac)));
        icfg.random_seed = u64::from_le_bytes(seed);
        let now_ns = monotonic_ns();
        let clock = Clock::new(cfg.rto_min_us, now_ns);
        let mut iface = Interface::new(icfg, &mut dev, clock.at(now_ns));
        let ip = Ipv4Address::from(cfg.ip.octets());
        iface.update_ip_addrs(|a| {
            let _ = a.push(IpCidr::new(IpAddress::Ipv4(ip), cfg.prefix_len));
        });
        if let Some(gw) = cfg.gateway {
            iface
                .routes_mut()
                .add_default_ipv4_route(Ipv4Address::from(gw.octets()))
                .map_err(|_| io::Error::other("smoltcp: no room for the default route"))?;
        }
        let storage: Vec<SocketStorage<'static>> =
            (0..cfg.max_sockets).map(|_| SocketStorage::EMPTY).collect();
        let mut start = [0u8; 2];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut start)
            .map_err(|_| io::Error::other("no system randomness"))?;
        let span = EPHEMERAL.end() - EPHEMERAL.start();
        let lifetime = clock.real_ns(NEIGHBOR_LIFETIME_US);
        let mut s = Self {
            iface,
            dev,
            sockets: SocketSet::new(storage),
            slots: Vec::with_capacity(cfg.max_sockets),
            free: Vec::with_capacity(cfg.max_sockets),
            generation: 0,
            ports: vec![0u64; 1024].into_boxed_slice(),
            next_port: EPHEMERAL.start() + u16::from_le_bytes(start) % span,
            clock,
            cfg: cfg.clone(),
            ip: cfg.ip,
            mac: port.mac,
            port_id: port.id,
            driver: port.driver.clone(),
            rescan: Vec::new(),
            zombies: Vec::with_capacity(cfg.max_sockets),
            spare_tcp: Vec::with_capacity(cfg.max_sockets),
            spare_udp: Vec::with_capacity(cfg.max_sockets),
            next_hk_ns: now_ns,
            next_keep_ns: now_ns,
            next_timers_ns: now_ns,
            // A third of an entry's life: two refreshes can be lost.
            keep_ns: (lifetime / 3).max(1_000_000),
            counters: Counters::default(),
            neighbors: Vec::with_capacity(16),
        };
        // Learn (or pin) the gateway now, so the first SYN does not wait.
        s.refresh_neighbors();
        s.dev.tx.flush();
        Ok(s)
    }

    pub const fn mac(&self) -> [u8; 6] {
        self.mac
    }

    pub const fn ip(&self) -> Ipv4Addr {
        self.ip
    }

    /// Each UDP socket's buffer, bytes.
    pub const fn udp_buf(&self) -> usize {
        self.cfg.udp_buf
    }

    pub const fn tx_checksum_offload(&self) -> bool {
        self.dev.tx_offload()
    }

    pub fn stats(&self) -> StackStats {
        let tx = self.dev.tx.stats;
        let rx = self.dev.rx_stats;
        StackStats {
            polls: self.counters.polls,
            rx_bursts: rx.bursts,
            rx_frames: rx.frames,
            tx_frames: tx.frames,
            rx_chained: rx.chained,
            tx_no_mbuf: tx.no_mbuf,
            tx_ring_full: tx.ring_full,
            tcp_retransmits: tx.retransmits,
            arp_refreshes: self.counters.arp_refreshes,
            arp_pinned: self.counters.arp_pinned,
            sockets_open: self.slots.iter().flatten().count() as u64,
            sockets_closing: self.zombies.len() as u64,
            sockets_spare: (self.spare_tcp.len() + self.spare_udp.len()) as u64,
            housekeeping: self.counters.housekeeping,
            housekeeping_max_ns: self.counters.housekeeping_max_ns,
            port: port::port_stats(self.port_id).unwrap_or_default(),
        }
    }

    // ---- descriptors and slots -------------------------------------------

    fn slot_index(&self, fd: RawFd) -> Option<usize> {
        let i = usize::try_from(fd & 0xffff).ok()?;
        self.slots
            .get(i)?
            .as_ref()
            .is_some_and(|s| s.fd == fd)
            .then_some(i)
    }

    fn slot(&self, fd: RawFd) -> io::Result<&Slot> {
        self.slot_index(fd)
            .and_then(|i| self.slots.get(i)?.as_ref())
            .ok_or_else(no_slot)
    }

    pub fn owns(&self, fd: RawFd) -> bool {
        self.slot_index(fd).is_some()
    }

    /// Takes a free slot for `make`'s socket, under a new handle.
    fn insert(&mut self, make: impl FnOnce(RawFd) -> Slot) -> io::Result<RawFd> {
        let i = match self.free.pop() {
            Some(i) => i,
            None if self.slots.len() < 1 << 16 => {
                self.slots.push(None);
                self.slots.len() - 1
            }
            None => return Err(io::Error::other("the smoltcp stack has no free handle")),
        };
        self.generation = (self.generation + 1) & 0x7fff;
        let fd = RawFd::try_from((self.generation << 16) | u32::try_from(i).unwrap_or(0))
            .map_err(|_| io::Error::other("handle out of range"))?;
        self.slots[i] = Some(make(fd));
        Ok(fd)
    }

    /// Removes slot `i`: its socket aborted and freed at once (`abort`), or
    /// closed gracefully and kept until smoltcp has finished with it.
    fn forget(&mut self, i: usize, abort: bool) {
        let Some(s) = self.slots.get_mut(i).and_then(Option::take) else {
            return;
        };
        self.free.push(i);
        if s.udp {
            self.sockets.get_mut::<udp::Socket>(s.handle).close();
            self.spare_udp.push(s.handle);
            self.release_port(s.port);
            return;
        }
        let t = self.sockets.get_mut::<tcp::Socket>(s.handle);
        if abort || t.can_recv() {
            // Unread data: an RST, as the kernel's close does.
            t.abort();
        } else {
            t.close();
        }
        let linger = self.clock.scaled(LINGER_S * 1_000_000);
        if t.timeout().is_none_or(|d| d > linger) {
            t.set_timeout(Some(linger));
        }
        t.set_keep_alive(None);
        self.zombies.push((s.handle, s.port));
    }

    fn take_port(&mut self, want: u16) -> io::Result<u16> {
        let used = |ports: &[u64], p: u16| ports[usize::from(p) / 64] >> (p % 64) & 1 == 1;
        let port = if want != 0 {
            if used(&self.ports, want) {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("port {want} is in use on the smoltcp stack"),
                ));
            }
            want
        } else {
            let span = usize::from(EPHEMERAL.end() - EPHEMERAL.start()) + 1;
            let mut p = None;
            for _ in 0..span {
                let c = self.next_port;
                self.next_port = if c == *EPHEMERAL.end() {
                    *EPHEMERAL.start()
                } else {
                    c + 1
                };
                if !used(&self.ports, c) {
                    p = Some(c);
                    break;
                }
            }
            p.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "no free ephemeral port on the smoltcp stack",
                )
            })?
        };
        self.ports[usize::from(port) / 64] |= 1 << (port % 64);
        Ok(port)
    }

    fn release_port(&mut self, port: u16) {
        self.ports[usize::from(port) / 64] &= !(1 << (port % 64));
    }

    fn room(&self) -> io::Result<()> {
        let live = self.slots.iter().flatten().count()
            + self.zombies.len()
            + self.spare_tcp.len()
            + self.spare_udp.len();
        if live >= self.cfg.max_sockets {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                format!(
                    "the smoltcp stack holds max_sockets ({}) sockets",
                    self.cfg.max_sockets
                ),
            ));
        }
        Ok(())
    }

    // ---- sockets -----------------------------------------------------------

    /// A TCP connect to `addr` from this stack's address: its handle. The
    /// SYN leaves before this returns.
    pub fn tcp_connect(&mut self, addr: SocketAddrV4) -> io::Result<RawFd> {
        let spare = self.spare_tcp.pop();
        if spare.is_none() {
            self.room()?;
        }
        let port = match self.take_port(0) {
            Ok(p) => p,
            Err(e) => {
                self.spare_tcp.extend(spare);
                return Err(e);
            }
        };
        let handle = spare.unwrap_or_else(|| {
            // Written once now (fresh pages are mapped on first touch), so
            // the first segments take no page faults.
            self.sockets.add(tcp::Socket::new(
                tcp::SocketBuffer::new(vec![1u8; self.cfg.tcp_rx_buf]),
                tcp::SocketBuffer::new(vec![1u8; self.cfg.tcp_tx_buf]),
            ))
        });
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::from(addr.ip().octets())),
            addr.port(),
        );
        let local = IpListenEndpoint {
            addr: Some(IpAddress::Ipv4(Ipv4Address::from(self.ip.octets()))),
            port,
        };
        let sock = self.sockets.get_mut::<tcp::Socket>(handle);
        // A reused socket keeps its settings and its congestion state:
        // each is set again, the controller made new.
        sock.set_timeout(None);
        sock.set_keep_alive(None);
        sock.set_nagle_enabled(false);
        sock.set_ack_delay(None);
        sock.set_congestion_control(match self.cfg.congestion {
            Congestion::Reno => tcp::CongestionControl::Reno,
            Congestion::None => tcp::CongestionControl::None,
        });
        let stamp: fn() -> u32 = tsval;
        sock.set_tsval_generator(self.cfg.tcp_timestamps.then_some(stamp));
        if let Err(e) = sock.connect(self.iface.context(), remote, local) {
            self.release_port(port);
            self.spare_tcp.push(handle);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("smoltcp connect to {addr}: {e}"),
            ));
        }
        self.dev.tx.reset_port(port);
        let made = self.insert(|fd| Slot {
            fd,
            handle,
            udp: false,
            owner: Owner::Setup,
            stamps: false,
            peer: Some(remote),
            port,
        });
        let fd = match made {
            Ok(fd) => fd,
            Err(e) => {
                self.sockets.get_mut::<tcp::Socket>(handle).abort();
                self.zombies.push((handle, port));
                return Err(e);
            }
        };
        self.egress();
        Ok(fd)
    }

    /// Whether the connect on `fd` finished: `Ok(false)` while the handshake
    /// runs, an error once it failed (refused, reset or timed out).
    pub fn connect_finished(&self, fd: RawFd) -> io::Result<bool> {
        let s = self.slot(fd)?;
        let t = self.sockets.get::<tcp::Socket>(s.handle);
        match t.state() {
            tcp::State::SynSent | tcp::State::SynReceived => Ok(false),
            tcp::State::Established | tcp::State::CloseWait => Ok(true),
            st => Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("smoltcp: the connect ended in {st} (refused, reset or timed out)"),
            )),
        }
    }

    /// A UDP socket from `port` (0: ephemeral), sending to `peer` if given:
    /// its handle.
    pub fn udp_socket(&mut self, port: u16, peer: Option<SocketAddrV4>) -> io::Result<RawFd> {
        let spare = self.spare_udp.pop();
        if spare.is_none() {
            self.room()?;
        }
        let port = match self.take_port(port) {
            Ok(p) => p,
            Err(e) => {
                self.spare_udp.extend(spare);
                return Err(e);
            }
        };
        let (n, bytes) = (self.cfg.udp_packets, self.cfg.udp_buf);
        let handle = spare.unwrap_or_else(|| {
            self.sockets.add(udp::Socket::new(
                udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; n], vec![1u8; bytes]),
                udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; n], vec![1u8; bytes]),
            ))
        });
        let local = IpListenEndpoint {
            addr: Some(IpAddress::Ipv4(Ipv4Address::from(self.ip.octets()))),
            port,
        };
        if let Err(e) = self.sockets.get_mut::<udp::Socket>(handle).bind(local) {
            self.release_port(port);
            self.spare_udp.push(handle);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("smoltcp udp bind {port}: {e}"),
            ));
        }
        let peer = peer.map(|p| {
            IpEndpoint::new(
                IpAddress::Ipv4(Ipv4Address::from(p.ip().octets())),
                p.port(),
            )
        });
        self.insert(|fd| Slot {
            fd,
            handle,
            udp: true,
            owner: Owner::Setup,
            stamps: false,
            peer,
            port,
        })
        .inspect_err(|_| {
            self.sockets.get_mut::<udp::Socket>(handle).close();
            self.spare_udp.push(handle);
            self.release_port(port);
        })
    }

    /// A UDP socket's local address and peer.
    pub fn udp_addrs(&self, fd: RawFd) -> Option<(SocketAddrV4, Option<SocketAddrV4>)> {
        let s = self.slot(fd).ok().filter(|s| s.udp)?;
        let peer = s.peer.map(|e| {
            let IpAddress::Ipv4(a) = e.addr;
            SocketAddrV4::new(Ipv4Addr::from(a.octets()), e.port)
        });
        Some((SocketAddrV4::new(self.ip, s.port), peer))
    }

    pub fn tcp_stats(&self, fd: RawFd) -> Option<TcpStats> {
        let s = self.slot(fd).ok().filter(|s| !s.udp)?;
        let t = self.sockets.get::<tcp::Socket>(s.handle);
        Some(TcpStats {
            state: state_name(t.state()),
            local_port: s.port,
            retransmits: u64::from(self.dev.tx.retransmits(s.port)),
            send_queue: t.send_queue(),
            recv_queue: t.recv_queue(),
        })
    }

    // ---- the poller's side -------------------------------------------------

    pub fn new_poller(&mut self) -> u32 {
        self.rescan.push(true);
        u32::try_from(self.rescan.len() - 1).unwrap_or(u32::MAX)
    }

    /// Hands `fd`'s socket to poller `poller`, reported as `token`.
    pub fn register(&mut self, fd: RawFd, poller: u32, token: u64) -> io::Result<()> {
        let i = self.slot_index(fd).ok_or_else(no_slot)?;
        let Some(s) = self.slots.get_mut(i).and_then(Option::as_mut) else {
            return Err(no_slot());
        };
        s.owner = Owner::Driver { poller, token };
        if let Some(r) = self.rescan.get_mut(poller as usize) {
            *r = true;
        }
        Ok(())
    }

    /// `fd` is done with (its driver or its handle let go): the connection
    /// closes (FIN, or RST with unread data) and the FIN leaves now.
    pub fn release(&mut self, fd: RawFd) {
        if let Some(i) = self.slot_index(fd) {
            self.forget(i, false);
            self.egress();
        }
    }

    /// A poller went away: its sockets close.
    pub fn drop_poller(&mut self, poller: u32) {
        for i in 0..self.slots.len() {
            if matches!(&self.slots[i], Some(Slot { owner: Owner::Driver { poller: p, .. }, .. }) if *p == poller)
            {
                self.forget(i, false);
            }
        }
        self.egress();
    }

    /// One pass: an RX burst through smoltcp, its egress, a TX burst, and
    /// housekeeping when due.
    pub fn poll(&mut self) {
        self.counters.polls += 1;
        let ns = monotonic_ns();
        let now = self.clock.at(ns);
        self.dev.burst_done = false;
        let r = self.iface.poll(now, &mut self.dev, &mut self.sockets);
        self.dev.tx.flush();
        if r == PollResult::SocketStateChanged {
            self.rescan.fill(true);
        }
        if ns >= self.next_hk_ns {
            self.housekeeping(ns);
        }
    }

    /// Sends what the sockets hold (after a send pass) and bursts it.
    pub fn egress(&mut self) {
        let now = self.clock.at(monotonic_ns());
        let mut changed = false;
        while self
            .iface
            .poll_egress(now, &mut self.dev, &mut self.sockets)
            == PollResult::SocketStateChanged
        {
            changed = true;
        }
        self.dev.tx.flush();
        // A timer may have closed a socket (a timeout, its last
        // retransmission): its driver must look.
        if changed {
            self.rescan.fill(true);
        }
    }

    /// Fills `events` with the tokens of poller `poller`'s readable sockets:
    /// data, a peer close or a reset. Level-triggered.
    pub fn ready(&mut self, poller: u32, events: &mut [u64]) -> usize {
        let Some(flag) = self.rescan.get_mut(poller as usize) else {
            return 0;
        };
        if !*flag {
            return 0;
        }
        *flag = false;
        let mut n = 0;
        let mut any = false;
        for s in self.slots.iter().flatten() {
            let Owner::Driver { poller: p, token } = s.owner else {
                continue;
            };
            if p != poller {
                continue;
            }
            let ready = if s.udp {
                self.sockets.get::<udp::Socket>(s.handle).can_recv()
            } else {
                let t = self.sockets.get::<tcp::Socket>(s.handle);
                t.can_recv() || !t.may_recv()
            };
            if ready {
                any = true;
                if n < events.len() {
                    events[n] = token;
                    n += 1;
                }
            }
        }
        // Level-triggered: a socket reported now is looked at again next
        // time, read or not.
        if any && let Some(flag) = self.rescan.get_mut(poller as usize) {
            *flag = true;
        }
        n
    }

    /// Reads `fd` into `buf`: `Ok(None)` when nothing is queued, `Ok(Some(0))`
    /// at the peer's FIN, an error after a reset. For UDP, `truncated` is set
    /// when the datagram did not fit (it is dropped).
    pub fn recv(
        &mut self,
        fd: RawFd,
        buf: &mut [u8],
        truncated: &mut bool,
        stamp: &mut i64,
    ) -> io::Result<Option<usize>> {
        let i = self.slot_index(fd).ok_or_else(no_slot)?;
        let Some(s) = self.slots.get(i).and_then(Option::as_ref) else {
            return Err(no_slot());
        };
        *truncated = false;
        if s.stamps {
            *stamp = self.dev.burst_ns;
        }
        if s.udp {
            let peer = s.peer;
            let u = self.sockets.get_mut::<udp::Socket>(s.handle);
            loop {
                return match u.recv_slice(buf) {
                    Ok((n, meta)) => {
                        if peer.is_some_and(|p| p != meta.endpoint) {
                            // A connected socket takes only its peer's.
                            continue;
                        }
                        Ok(Some(n))
                    }
                    Err(udp::RecvError::Exhausted) => Ok(None),
                    Err(udp::RecvError::Truncated) => {
                        *truncated = true;
                        Ok(Some(buf.len().max(1)))
                    }
                };
            }
        }
        let t = self.sockets.get_mut::<tcp::Socket>(s.handle);
        match t.recv_slice(buf) {
            Ok(0) => Ok(None),
            Ok(n) => Ok(Some(n)),
            Err(tcp::RecvError::Finished) => Ok(Some(0)),
            Err(tcp::RecvError::InvalidState) => match t.state() {
                tcp::State::SynSent | tcp::State::SynReceived => Ok(None),
                st => Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    format!("smoltcp: connection reset or timed out ({st})"),
                )),
            },
        }
    }

    /// Queues `buf` on `fd`'s connection: the bytes smoltcp took (0 when its
    /// send buffer is full). They leave at the next egress.
    pub fn send(&mut self, fd: RawFd, buf: &[u8]) -> io::Result<usize> {
        let s = self.slot(fd)?;
        let t = self.sockets.get_mut::<tcp::Socket>(s.handle);
        match t.send_slice(buf) {
            Ok(n) => Ok(n),
            Err(tcp::SendError::InvalidState) => match t.state() {
                tcp::State::SynSent | tcp::State::SynReceived => Ok(0),
                st => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    format!("smoltcp: the connection cannot send ({st})"),
                )),
            },
        }
    }

    /// Queues one datagram to `fd`'s peer: `Ok(false)` when the send buffer
    /// is full.
    pub fn send_datagram(&mut self, fd: RawFd, buf: &[u8]) -> io::Result<bool> {
        let s = self.slot(fd)?;
        let peer = s
            .peer
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "udp socket has no peer"))?;
        let u = self.sockets.get_mut::<udp::Socket>(s.handle);
        match u.send_slice(buf, peer) {
            Ok(()) => Ok(true),
            Err(udp::SendError::BufferFull) => Ok(false),
            Err(udp::SendError::Unaddressable) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("smoltcp: cannot send to {peer}"),
            )),
        }
    }

    /// Reads stamped with the burst time from now on.
    pub fn enable_stamps(&mut self, fd: RawFd) -> io::Result<()> {
        let i = self.slot_index(fd).ok_or_else(no_slot)?;
        if let Some(s) = self.slots.get_mut(i).and_then(Option::as_mut) {
            s.stamps = true;
        }
        Ok(())
    }

    /// Keepalive probes every `idle_s` (smoltcp has one interval, no probe
    /// count), and the abort after `user_timeout_ms` without a segment in.
    pub fn set_keepalive(&mut self, fd: RawFd, idle_s: u32) -> io::Result<()> {
        let s = self.slot(fd)?;
        let d = self.clock.scaled(u64::from(idle_s) * 1_000_000);
        if !s.udp {
            self.sockets
                .get_mut::<tcp::Socket>(s.handle)
                .set_keep_alive(Some(d));
        }
        Ok(())
    }

    pub fn set_user_timeout(&mut self, fd: RawFd, ms: u32) -> io::Result<()> {
        let s = self.slot(fd)?;
        let d = self.clock.scaled(u64::from(ms) * 1_000);
        if !s.udp {
            self.sockets
                .get_mut::<tcp::Socket>(s.handle)
                .set_timeout(Some(d));
        }
        Ok(())
    }

    pub fn set_ack_delay(&mut self, fd: RawFd, us: u32) -> io::Result<()> {
        let s = self.slot(fd)?;
        let d = self.clock.scaled(u64::from(us));
        if !s.udp {
            self.sockets
                .get_mut::<tcp::Socket>(s.handle)
                .set_ack_delay(Some(d));
        }
        Ok(())
    }

    // ---- set-up I/O ---------------------------------------------------------

    /// A set-up read (TLS handshake, upgrade) on `fd`: `WouldBlock` when
    /// nothing is queued.
    pub fn setup_read(&mut self, fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
        let (mut t, mut s) = (false, 0);
        self.recv(fd, buf, &mut t, &mut s)?
            .ok_or_else(|| io::ErrorKind::WouldBlock.into())
    }

    /// A set-up write on `fd`; the bytes leave before it returns.
    pub fn setup_write(&mut self, fd: RawFd, buf: &[u8]) -> io::Result<usize> {
        let n = self.send(fd, buf)?;
        if n == 0 && !buf.is_empty() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.egress();
        Ok(n)
    }

    // ---- housekeeping ------------------------------------------------------

    #[cold]
    fn housekeeping(&mut self, ns: i64) {
        self.next_hk_ns = ns + HOUSEKEEPING_NS;
        self.counters.housekeeping += 1;
        // Closed connections smoltcp has finished with: kept for reuse.
        let mut k = 0;
        while k < self.zombies.len() {
            let (h, port) = self.zombies[k];
            if self.sockets.get::<tcp::Socket>(h).state() == tcp::State::Closed {
                self.spare_tcp.push(h);
                self.release_port(port);
                self.zombies.swap_remove(k);
            } else {
                k += 1;
            }
        }
        // What changed without a packet (a timer), the drivers see within a
        // millisecond.
        self.rescan.fill(true);
        if ns >= self.next_keep_ns {
            self.next_keep_ns = ns + self.keep_ns;
            self.refresh_neighbors();
        }
        if ns >= self.next_timers_ns {
            self.next_timers_ns = ns + TIMERS_NS;
            port::timer_service();
        }
        self.dev.tx.flush();
        let took = u64::try_from(monotonic_ns() - ns).unwrap_or(0);
        self.counters.housekeeping_max_ns = self.counters.housekeeping_max_ns.max(took);
    }

    /// Keeps the neighbour entries the connections use from expiring: an
    /// ARP request to the gateway and to each on-link peer, or for a pinned
    /// gateway its answer fed to smoltcp directly.
    #[cold]
    fn refresh_neighbors(&mut self) {
        self.neighbors.clear();
        if let Some(gw) = self.cfg.gateway {
            self.neighbors.push(gw);
        }
        for s in self.slots.iter().flatten() {
            if let Some(p) = s.peer {
                let IpAddress::Ipv4(a) = p.addr;
                let a = Ipv4Addr::from(a.octets());
                if self.cfg.on_link(a) && !self.neighbors.contains(&a) && self.neighbors.len() < 16
                {
                    self.neighbors.push(a);
                }
            }
        }
        for k in 0..self.neighbors.len() {
            let n = self.neighbors[k];
            match (self.cfg.gateway, self.cfg.gateway_mac) {
                (Some(gw), Some(mac)) if gw == n => {
                    self.dev
                        .inject_arp_reply(self.mac, self.ip.octets(), mac, gw.octets());
                    self.counters.arp_pinned += 1;
                }
                _ => {
                    self.dev
                        .tx
                        .send_arp_request(self.mac, self.ip.octets(), n.octets());
                    self.counters.arp_refreshes += 1;
                }
            }
        }
    }

    /// Resets every connection (an RST each) and sends them.
    pub fn abort_all(&mut self) {
        for i in 0..self.slots.len() {
            self.forget(i, true);
        }
        for &(h, _) in &self.zombies {
            self.sockets.get_mut::<tcp::Socket>(h).abort();
        }
        self.egress();
    }
}

/// smoltcp's name for a TCP state.
const fn state_name(s: tcp::State) -> &'static str {
    match s {
        tcp::State::Closed => "CLOSED",
        tcp::State::Listen => "LISTEN",
        tcp::State::SynSent => "SYN-SENT",
        tcp::State::SynReceived => "SYN-RECEIVED",
        tcp::State::Established => "ESTABLISHED",
        tcp::State::FinWait1 => "FIN-WAIT-1",
        tcp::State::FinWait2 => "FIN-WAIT-2",
        tcp::State::CloseWait => "CLOSE-WAIT",
        tcp::State::Closing => "CLOSING",
        tcp::State::LastAck => "LAST-ACK",
        tcp::State::TimeWait => "TIME-WAIT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_runs_one_second_per_rto_floor() {
        let c = Clock::new(10_000, 1_000);
        // 1 ms real is 100 ms of smoltcp's time at a 10 ms floor.
        assert_eq!(c.at(1_000 + 1_000_000).total_micros(), 100_000);
        assert_eq!(
            c.scaled(10_000).total_micros(),
            1_000_000,
            "the floor is smoltcp's 1 s"
        );
        assert_eq!(
            c.real_ns(60_000_000),
            600_000_000,
            "60 s of neighbour lifetime is 600 ms"
        );
        let native = Clock::new(1_000_000, 0);
        assert_eq!(native.at(5_000_000).total_micros(), 5_000);
        assert_eq!(native.scaled(7).total_micros(), 7);
        // Hours of uptime do not overflow.
        assert!(c.at(i64::from(i32::MAX) * 1_000_000).total_micros() > 0);
    }
}
