//! The `smoltcp-spin` backend's stack: [smoltcp](https://github.com/smoltcp-rs/smoltcp)
//! (0BSD), a TCP/IP stack in this process, over a DPDK port.
//!
//! - Cargo feature `smoltcp-dpdk`, Linux `x86_64` or `aarch64`, libdpdk >= 23.11.
//!   Without it (or elsewhere) [`Stack::init`] returns
//!   [`Error::Unsupported`], and so does `Driver::new` with
//!   [`Backend::SmoltcpSpin`](crate::Backend::SmoltcpSpin).
//! - One stack per process, on the polling thread: [`Stack::init`] starts
//!   DPDK (once per process) and the port, and every `Driver` with the
//!   `smoltcp-spin` backend on that thread runs on it. Its `poll` runs the
//!   stack: one RX burst, smoltcp's ingress and egress, one TX burst, and
//!   after the send pass another egress, so a reply leaves in the poll that
//!   received its request.
//! - TCP endpoints on the stack are `FrameConn<SmolTcp>`, `WsConn<SmolTcp>`
//!   and `HttpConn<SmolTcp>`, made by their dials' `on_stack`
//!   (`FrameDial::<SmolTcp>::on_stack`) and `Driver::connect` (TLS is rustls
//!   over the stack's byte stream, as on the kernel). Like F-Stack's,
//!   `SmolTcp` is not `Send`, so there is no `connect_background`. UDP:
//!   `UdpConn<SmolUdp>` from [`Stack::udp_connect`] or [`Stack::udp_bind`].
//!   A driver refuses endpoints of another stack. Names are resolved by the
//!   driver's resolver over the kernel's network.
//! - IPv4 only, one address and an optional default gateway, Ethernet.
//!   ARP is smoltcp's; [`StackConfig::gateway_mac`] pins the gateway's MAC.
//!
//! # TCP tuning
//!
//! | setting | here | smoltcp default |
//! |---|---|---|
//! | Nagle | off (`TCP_NODELAY`) | on |
//! | delayed ACK | off: every segment acknowledged at once (`Config::tcp_delack_max_us` sets a delay) | 10 ms |
//! | RTO floor | [`StackConfig::rto_min_us`], default 10 ms | 1 s (RFC 6298), not settable |
//! | timestamps (RFC 7323) | on, ms clock ([`StackConfig::tcp_timestamps`]) | off |
//! | window scaling | from the receive buffer ([`StackConfig::tcp_rx_buf`], 256 KiB: shift 3) | same |
//! | SACK | SACK-permitted offered; SACK blocks sent for out-of-order data | same |
//! | congestion control | Reno ([`StackConfig::congestion`]) | Reno |
//! | keepalive, `TCP_USER_TIMEOUT` | `Config::keepalive_idle_s` (probe interval), `Config::tcp_user_timeout_ms` (abort after that long with no segment in) | off |
//!
//! smoltcp's RTO floor is a constant, so the stack runs smoltcp's clock
//! `1 s / rto_min_us` times faster than real time: at 10 ms that is 100×.
//! Every smoltcp timer scales with it: the first RTO (before an RTT sample)
//! is `rto_min_us`, the RTO cap 60 s becomes 600 ms, TIME-WAIT 10 s becomes
//! 100 ms, the ARP entry lifetime 60 s becomes 600 ms (the stack refreshes
//! the gateway's and on-link peers' entries before they expire). Keepalive,
//! user timeout and delayed-ACK settings are scaled back, so they stay real.
//! RTT samples are smoltcp's whole milliseconds of that clock, so they
//! resolve 10 µs at the default.
//!
//! What smoltcp does not do, against Linux's or FreeBSD's TCP: it ignores
//! received SACK blocks (a retransmission resends from the first unACKed
//! byte: fast retransmit sends one segment, a timeout goes back N); no RACK,
//! TLP, F-RTO, PRR or ECN; no pacing; the first RTO before any RTT sample is
//! the floor, so a peer further than `rto_min_us` away sees a spurious
//! retransmission of the SYN and the first segment (raise `rto_min_us` for
//! such peers); no listen side in this backend (a client stack); IPv4
//! fragments are dropped; no TSO or LRO (one segment per MTU).
//!
//! The process also keeps its kernel network: DNS, the warning thread's
//! checks and other kernel backends use it.

#[cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
pub use crate::netstack::smoltcp::{SmolTcp, SmolUdp};
use std::net::Ipv4Addr;
use std::str::FromStr;

use crate::error::{Error, Result};

/// smoltcp's congestion control, per connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Congestion {
    /// Reno (RFC 5681): slow start from 2 KiB, halving on loss.
    #[default]
    Reno,
    /// None: the peer's receive window is the only limit.
    None,
}

/// The stack's settings. [`StackConfig::parse`] takes them as
/// `key=value,...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackConfig {
    /// DPDK EAL arguments, without the program name: the device (`-a
    /// 0000:00:06.0` for an ENA bound to vfio-pci, `--vdev=net_af_packet0,iface=eth1`
    /// for a test), memory (`--no-huge -m 512`, or hugepages by default).
    /// Unless given, the stack adds `--lcores=0@(<the calling thread's
    /// CPUs>)` (no EAL worker threads; the thread keeps its affinity),
    /// `--no-telemetry`, and `--in-memory` (no runtime files; `--no-shconf`
    /// with `--no-huge`) unless `--file-prefix` or `--no-shconf` is given. Used by the first
    /// [`Stack::init`] in the process only.
    pub eal_args: Vec<String>,
    /// The port: a DPDK device name (`0000:00:06.0`, `net_af_packet0`) or
    /// a port index. Default `0`.
    pub port: String,
    /// This stack's IPv4 address.
    pub ip: Ipv4Addr,
    /// The address's prefix length: peers inside it are on-link (ARP),
    /// others go through [`StackConfig::gateway`]. 1 to 32, default 24.
    pub prefix_len: u8,
    /// The default gateway, inside the prefix. `None`: on-link peers only.
    pub gateway: Option<Ipv4Addr>,
    /// The gateway's MAC, pinned: the stack answers its own ARP lookups for
    /// the gateway with it and sends no ARP request for it. `None`: learnt
    /// by ARP and kept fresh by a request before the entry expires.
    pub gateway_mac: Option<[u8; 6]>,
    /// IP MTU, bytes: 576 to 9216 (ENA: up to 9001), default 1500. Each
    /// mbuf holds one whole frame.
    pub mtu: u16,
    /// RX and TX descriptors (the port may round them), default 1024 each.
    pub rx_desc: u16,
    /// See [`StackConfig::rx_desc`].
    pub tx_desc: u16,
    /// Mbufs in the pool, default 8191; at least the descriptors plus 512.
    pub mbufs: u32,
    /// Sockets the stack holds at once, closing ones (TIME-WAIT) included:
    /// 1 to 65 536, default 64.
    pub max_sockets: usize,
    /// Each TCP connection's receive buffer, bytes: its window. 1 KiB to
    /// 1 GiB, default 256 KiB.
    pub tcp_rx_buf: usize,
    /// Each TCP connection's send buffer, bytes. 1 KiB to 1 GiB, default
    /// 256 KiB.
    pub tcp_tx_buf: usize,
    /// Each UDP socket's receive and send buffers, bytes of payload. 1 KiB
    /// to 64 MiB, default 256 KiB.
    pub udp_buf: usize,
    /// Each UDP socket's receive and send buffers, datagrams. 1 to 65 536,
    /// default 256.
    pub udp_packets: usize,
    /// The TCP retransmission-timeout floor, µs: 2 000 to 1 000 000 (smoltcp's
    /// own), default 10 000. See the module's TCP tuning.
    pub rto_min_us: u32,
    /// Congestion control, default Reno.
    pub congestion: Congestion,
    /// TCP timestamps (RFC 7323), default on.
    pub tcp_timestamps: bool,
    /// The IPv4, TCP and UDP transmit checksums on the NIC when the port
    /// offers all three (ENA does; `net_af_packet` does not), default on.
    /// Received checksums are always checked in software.
    pub tx_checksum_offload: bool,
    /// How long [`Stack::init`] waits for the port's link, ms. Default
    /// 5 000.
    pub link_wait_ms: u32,
}

impl Default for StackConfig {
    fn default() -> Self {
        Self {
            eal_args: Vec::new(),
            port: "0".into(),
            ip: Ipv4Addr::UNSPECIFIED,
            prefix_len: 24,
            gateway: None,
            gateway_mac: None,
            mtu: 1500,
            rx_desc: 1024,
            tx_desc: 1024,
            mbufs: 8191,
            max_sockets: 64,
            tcp_rx_buf: 256 << 10,
            tcp_tx_buf: 256 << 10,
            udp_buf: 256 << 10,
            udp_packets: 256,
            rto_min_us: 10_000,
            congestion: Congestion::Reno,
            tcp_timestamps: true,
            tx_checksum_offload: true,
            link_wait_ms: 5_000,
        }
    }
}

/// What [`StackConfig::rto_min_us`] may be.
const RTO_MIN_US: std::ops::RangeInclusive<u32> = 2_000..=1_000_000;
/// TCP buffer bounds: smoltcp refuses receive buffers over 1 GiB.
const TCP_BUF: std::ops::RangeInclusive<usize> = 1 << 10..=1 << 30;

impl StackConfig {
    /// Checks every setting.
    ///
    /// # Errors
    /// [`Error::Config`] naming the first bad one.
    pub fn validate(&self) -> Result<()> {
        let bad = |m: String| Err(Error::Config(format!("smoltcp stack: {m}")));
        if self.ip.is_unspecified() || self.ip.is_broadcast() || self.ip.is_multicast() {
            return bad(format!("ip {} is not a unicast address", self.ip));
        }
        if !(1..=32).contains(&self.prefix_len) {
            return bad(format!("prefix length {}: 1 to 32", self.prefix_len));
        }
        if let Some(gw) = self.gateway {
            if gw == self.ip || !self.on_link(gw) {
                return bad(format!(
                    "gateway {gw} must be another address inside {}/{}",
                    self.ip, self.prefix_len
                ));
            }
        } else if self.gateway_mac.is_some() {
            return bad("gw_mac without gw".into());
        }
        if let Some(mac) = self.gateway_mac
            && (mac[0] & 1 == 1 || mac == [0; 6])
        {
            return bad(format!("gw_mac {} is not a unicast MAC", fmt_mac(mac)));
        }
        if !(576..=9216).contains(&self.mtu) {
            return bad(format!("mtu {}: 576 to 9216", self.mtu));
        }
        if self.rx_desc == 0 || self.tx_desc == 0 {
            return bad("rx_desc and tx_desc must be above 0".into());
        }
        let need = u32::from(self.rx_desc) + u32::from(self.tx_desc) + 512;
        if self.mbufs < need {
            return bad(format!(
                "mbufs {}: at least rx_desc + tx_desc + 512 ({need})",
                self.mbufs
            ));
        }
        if !(1..=65_536).contains(&self.max_sockets) {
            return bad(format!("max_sockets {}: 1 to 65536", self.max_sockets));
        }
        for (k, v) in [
            ("tcp_rx_buf", self.tcp_rx_buf),
            ("tcp_tx_buf", self.tcp_tx_buf),
        ] {
            if !TCP_BUF.contains(&v) {
                return bad(format!("{k} {v}: 1 KiB to 1 GiB"));
            }
        }
        if !(1 << 10..=64 << 20).contains(&self.udp_buf) {
            return bad(format!("udp_buf {}: 1 KiB to 64 MiB", self.udp_buf));
        }
        if !(1..=65_536).contains(&self.udp_packets) {
            return bad(format!("udp_packets {}: 1 to 65536", self.udp_packets));
        }
        if !RTO_MIN_US.contains(&self.rto_min_us) {
            return bad(format!(
                "rto_min_us {}: {} to {}",
                self.rto_min_us,
                RTO_MIN_US.start(),
                RTO_MIN_US.end()
            ));
        }
        if self.port.is_empty() {
            return bad("port is empty".into());
        }
        Ok(())
    }

    /// Whether `addr` is inside this stack's prefix.
    #[must_use]
    pub fn on_link(&self, addr: Ipv4Addr) -> bool {
        let mask = u32::MAX
            .checked_shl(32 - u32::from(self.prefix_len.clamp(1, 32)))
            .unwrap_or(0);
        u32::from(addr) & mask == u32::from(self.ip) & mask
    }

    /// smoltcp's clock rate against real time (see the module's TCP
    /// tuning): `1 s / rto_min_us`, so 100 at the default 10 ms.
    #[must_use]
    pub fn clock_scale(&self) -> f64 {
        1e6 / f64::from(self.rto_min_us.max(1))
    }

    /// Parses `key=value` pairs separated by commas. Keys: `port`, `ip`
    /// (`a.b.c.d/len`, or `a.b.c.d` for /24), `gw`, `gw_mac`
    /// (`aa:bb:cc:dd:ee:ff`), `mtu`, `rx_desc`, `tx_desc`, `mbufs`,
    /// `max_sockets`, `tcp_rx_buf`, `tcp_tx_buf`, `udp_buf`, `udp_packets`,
    /// `rto_min_us`, `cc` (`reno` or `none`), `timestamps` and
    /// `tx_csum_offload` (`true`/`false`), `link_wait_ms`, and `eal`, which
    /// must come last: everything after `eal=` (commas included) is the EAL
    /// arguments, split on whitespace.
    ///
    /// ```
    /// let c = ergon_io::smoltcp_dpdk::StackConfig::parse(
    ///     "ip=10.0.0.2/24,gw=10.0.0.1,eal=--no-huge -m 512 --vdev=net_af_packet0,iface=eth1",
    /// )?;
    /// assert_eq!(c.eal_args, ["--no-huge", "-m", "512", "--vdev=net_af_packet0,iface=eth1"]);
    /// # Ok::<(), ergon_io::Error>(())
    /// ```
    ///
    /// # Errors
    /// An unknown key, a malformed value, or a setting
    /// [`StackConfig::validate`] refuses.
    pub fn parse(s: &str) -> Result<Self> {
        let mut cfg = Self::default();
        let (pairs, eal) = match s.find("eal=") {
            Some(i) if i == 0 || s[..i].ends_with(',') => (&s[..i], Some(&s[i + 4..])),
            _ => (s, None),
        };
        if let Some(eal) = eal {
            cfg.eal_args = eal.split_whitespace().map(str::to_owned).collect();
        }
        for kv in pairs.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (k, v) = kv.split_once('=').ok_or_else(|| {
                Error::Config(format!("smoltcp stack: expected key=value, got {kv:?}"))
            })?;
            let bad = || Error::Config(format!("smoltcp stack: {k}: bad value {v:?}"));
            let flag = || v.parse::<bool>().map_err(|_| bad());
            match k {
                "port" => v.clone_into(&mut cfg.port),
                "ip" => {
                    let (ip, len) = v.split_once('/').unwrap_or((v, "24"));
                    cfg.ip = Ipv4Addr::from_str(ip).map_err(|_| bad())?;
                    cfg.prefix_len = len.parse().map_err(|_| bad())?;
                }
                "gw" => cfg.gateway = Some(Ipv4Addr::from_str(v).map_err(|_| bad())?),
                "gw_mac" => cfg.gateway_mac = Some(parse_mac(v).ok_or_else(bad)?),
                "mtu" => cfg.mtu = v.parse().map_err(|_| bad())?,
                "rx_desc" => cfg.rx_desc = v.parse().map_err(|_| bad())?,
                "tx_desc" => cfg.tx_desc = v.parse().map_err(|_| bad())?,
                "mbufs" => cfg.mbufs = v.parse().map_err(|_| bad())?,
                "max_sockets" => cfg.max_sockets = v.parse().map_err(|_| bad())?,
                "tcp_rx_buf" => cfg.tcp_rx_buf = v.parse().map_err(|_| bad())?,
                "tcp_tx_buf" => cfg.tcp_tx_buf = v.parse().map_err(|_| bad())?,
                "udp_buf" => cfg.udp_buf = v.parse().map_err(|_| bad())?,
                "udp_packets" => cfg.udp_packets = v.parse().map_err(|_| bad())?,
                "rto_min_us" => cfg.rto_min_us = v.parse().map_err(|_| bad())?,
                "cc" => {
                    cfg.congestion = match v {
                        "reno" => Congestion::Reno,
                        "none" => Congestion::None,
                        _ => return Err(bad()),
                    };
                }
                "timestamps" => cfg.tcp_timestamps = flag()?,
                "tx_csum_offload" => cfg.tx_checksum_offload = flag()?,
                "link_wait_ms" => cfg.link_wait_ms = v.parse().map_err(|_| bad())?,
                _ => return Err(Error::Config(format!("smoltcp stack: unknown key {k:?}"))),
            }
        }
        cfg.validate()?;
        Ok(cfg)
    }
}

impl FromStr for StackConfig {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

fn parse_mac(s: &str) -> Option<[u8; 6]> {
    let mut out = [0u8; 6];
    let mut parts = s.split(':');
    for b in &mut out {
        let p = parts.next()?;
        if p.len() != 2 {
            return None;
        }
        *b = u8::from_str_radix(p, 16).ok()?;
    }
    parts.next().is_none().then_some(out)
}

/// `aa:bb:cc:dd:ee:ff`.
pub(crate) fn fmt_mac(mac: [u8; 6]) -> String {
    mac.map(|b| format!("{b:02x}")).join(":")
}

/// The stack's counters ([`Stack::stats`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StackStats {
    /// Stack passes (one per `Driver::poll` of each driver on it).
    pub polls: u64,
    /// RX bursts that returned frames.
    pub rx_bursts: u64,
    /// Frames received from the port.
    pub rx_frames: u64,
    /// Frames handed to the port.
    pub tx_frames: u64,
    /// Received frames dropped for spanning several mbufs.
    pub rx_chained: u64,
    /// Frames dropped on transmit: the mbuf pool was empty.
    pub tx_no_mbuf: u64,
    /// Frames dropped on transmit: the port's TX ring and the stack's own
    /// queue were full.
    pub tx_ring_full: u64,
    /// TCP segments sent again (data, SYN or FIN at or below the highest
    /// sequence already sent on the connection), counted as they leave.
    /// Keepalive probes are not counted.
    pub tcp_retransmits: u64,
    /// ARP requests the stack sent to keep a neighbour's entry fresh.
    pub arp_refreshes: u64,
    /// ARP answers for the pinned gateway MAC fed to smoltcp.
    pub arp_pinned: u64,
    /// Sockets open (set up or driven).
    pub sockets_open: u64,
    /// Sockets closed by the application that smoltcp still holds (FIN
    /// exchange, TIME-WAIT).
    pub sockets_closing: u64,
    /// Closed sockets kept, with their buffers, for the next connect.
    pub sockets_spare: u64,
    /// Housekeeping passes (every millisecond while the stack is polled).
    pub housekeeping: u64,
    /// The longest housekeeping pass, ns.
    pub housekeeping_max_ns: u64,
    /// The port's own counters (`rte_eth_stats`): frames received and
    /// sent, receive and send errors, frames missed (the RX ring was full),
    /// RX mbuf allocation failures.
    pub port: [u64; 6],
}

/// One TCP connection's state and counters ([`Stack::tcp_stats`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TcpStats {
    /// The TCP state, as smoltcp names it (`ESTABLISHED`, `SYN-SENT`, ...).
    pub state: &'static str,
    /// The local port.
    pub local_port: u16,
    /// Segments sent again ([`StackStats::tcp_retransmits`] for this
    /// connection).
    pub retransmits: u64,
    /// Bytes in smoltcp's send buffer: sent and not yet acknowledged, or
    /// not sent.
    pub send_queue: usize,
    /// Bytes in smoltcp's receive buffer, not yet read by the driver.
    pub recv_queue: usize,
}

/// The stack on the calling thread ([`Stack::init`]).
///
/// Dropping it resets every connection it holds (an RST each) and frees its
/// sockets; the DPDK port stays started for the next `Stack::init` in the
/// process. Not `Send`: the stack belongs to its thread.
#[derive(Debug)]
pub struct Stack {
    _thread: std::marker::PhantomData<*const ()>,
}

#[cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
impl Stack {
    /// Starts the stack on the calling thread (the polling thread): DPDK's
    /// EAL (the first time in the process), the port (likewise), and the
    /// smoltcp interface with `cfg`'s address and gateway. Blocks while the
    /// port starts and its link comes up.
    ///
    /// # Errors
    /// [`Error::Config`] for bad settings; [`Error::Busy`] when a stack is
    /// already running in this process; [`Error::Io`] when DPDK, the port or
    /// its link fails (the message is DPDK's).
    pub fn init(cfg: &StackConfig) -> Result<Self> {
        cfg.validate()?;
        crate::netstack::smoltcp::init(cfg)?;
        Ok(Self {
            _thread: std::marker::PhantomData,
        })
    }

    /// Counters.
    #[must_use]
    pub fn stats(&self) -> StackStats {
        crate::netstack::smoltcp::stats()
    }

    /// The port's MAC.
    #[must_use]
    pub fn mac(&self) -> [u8; 6] {
        crate::netstack::smoltcp::mac()
    }

    /// The port's DPDK driver (`net_ena`, `net_af_packet`, ...).
    #[must_use]
    pub fn driver(&self) -> String {
        crate::netstack::smoltcp::driver()
    }

    /// Whether transmit checksums are on the NIC
    /// ([`StackConfig::tx_checksum_offload`] and the port offers them).
    #[must_use]
    pub fn tx_checksum_offload(&self) -> bool {
        crate::netstack::smoltcp::tx_checksum_offload()
    }

    /// The state and counters of the TCP connection whose handle is
    /// `raw_fd` (`FrameConn::raw_fd`, `SmolTcp::raw`); `None` when it is
    /// no connection of this stack.
    #[must_use]
    pub fn tcp_stats(&self, raw_fd: std::os::fd::RawFd) -> Option<TcpStats> {
        crate::netstack::smoltcp::tcp_stats(raw_fd)
    }

    /// A UDP socket on the stack from `local_port` (0: an ephemeral port)
    /// to `peer`: it sends to `peer` and delivers only `peer`'s datagrams.
    /// Hand it to a `Driver` with the `smoltcp-spin` backend.
    ///
    /// # Errors
    /// Bad options, the port is taken, or the stack is full.
    pub fn udp_connect(
        &self,
        local_port: u16,
        peer: std::net::SocketAddrV4,
        opts: &crate::UdpOptions,
    ) -> Result<crate::UdpConn<SmolUdp>> {
        crate::UdpConn::build(SmolUdp::open(local_port, Some(peer))?, true, opts)
    }

    /// A receive-only UDP socket on the stack, bound to `port`.
    ///
    /// # Errors
    /// Bad options, the port is taken, or the stack is full.
    pub fn udp_bind(&self, port: u16, opts: &crate::UdpOptions) -> Result<crate::UdpConn<SmolUdp>> {
        crate::UdpConn::build(SmolUdp::open(port, None)?, false, opts)
    }
}

#[cfg(not(all(target_os = "linux", feature = "smoltcp-dpdk")))]
impl Stack {
    /// Starts the stack on the calling thread. Built without the
    /// `smoltcp-dpdk` feature, or off Linux: always
    /// [`Error::Unsupported`] (the rest of `Stack` comes with the feature).
    ///
    /// # Errors
    /// [`Error::Config`] for bad settings, else [`Error::Unsupported`].
    pub fn init(cfg: &StackConfig) -> Result<Self> {
        cfg.validate()?;
        Err(Error::Unsupported(
            "the smoltcp-spin backend needs the `smoltcp-dpdk` feature on Linux",
        ))
    }
}

#[cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
impl Drop for Stack {
    fn drop(&mut self) {
        crate::netstack::smoltcp::shutdown();
    }
}
