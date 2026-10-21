//! The TCP quick-ACK route check.
//!
//! - Each connection a `Driver` takes (and each libcurl connection of
//!   `HttpClient`) is checked once, at set-up: does its route have
//!   `quickack 1`?
//! - Never on the read or write path.
//!
//! # Why
//!
//! - Linux delays ACKs except in quick-ACK mode. `TCP_QUICKACK` switches the
//!   mode only for now (tcp(7)); the route attribute `quickack 1`
//!   (`RTAX_QUICKACK`) keeps every connection on the route in it.
//! - A peer using Nagle's algorithm sends its next small segment only once
//!   the last one is acknowledged, so our delayed ACK (up to tens of ms)
//!   holds it back.
//! - Its latency effect is not measured here: every ergon-io benchmark ran
//!   with `quickack 1 rto_min 5ms` (`io/benches/BENCHMARK.md`). No figure is
//!   claimed.
//! - The route's `rto_min` is reported as information only (kernel default
//!   200 ms; the benchmarks set 5 ms).
//!
//! # What it does
//!
//! - One rtnetlink exchange per connection: `RTM_GETROUTE` with
//!   `RTM_F_FIB_MATCH` for the socket's addresses, bound interface, mark,
//!   type of service, protocol and ports (as `ip route get fibmatch <peer>
//!   from <local>`), on a fresh netlink socket with a 100 ms timeout.
//!   [`lookups`] counts the exchanges.
//! - It reads `quickack` (`RTAX_QUICKACK`), which decides, and `rto_min`.
//! - The result stays on the connection as a [`RouteCheck`]
//!   (`WsConn::route`, `HttpConn::route`, `AnyConn::route`,
//!   `HttpClient::route_checks`).
//!
//! [`QuickAckCheck`] (`Config::quickack_check`, key `quickack_check`):
//!
//! | value | a route without quickack | where the lookup runs |
//! |---|---|---|
//! | `warn` (default) | goes ahead; one stderr line per route, with the `ip route change` that fixes it (a failed lookup: one line per distinct failure) | ergon-io's warning thread. The polling thread only reads the socket's addresses (five `getsockopt`-class calls) and queues the lookup; the connection reads [`RouteCheck::Pending`] until it is done |
//! | `require` | fails with [`crate::Error::QuickAck`]: [`QuickAckError::Missing`], or [`QuickAckError::Lookup`] when the route cannot be read | where the connection is set up: `Driver::connect` on the polling thread before TLS (a refused route costs no handshake), `connect_background` on its helper thread, `add` in `add`. About a dozen system calls |
//! | `off` | not looked up | nowhere |
//!
//! - Under `require`, prefer `Driver::connect_background`: the lookup stays
//!   off the loop. `DriverStats::route_lookups`, `route_check_ns` and
//!   `route_check_max_ns` count the polling thread's share.
//! - The warning thread then checks the route's egress device (see
//!   [`crate::checks`]).
//! - libcurl client: checked from libcurl's pre-request callback (libcurl
//!   7.80 or later), once per connection; a request on a connection already
//!   checked makes no system call.
//! - Routes to this host (type `local`: loopback, the host's own addresses)
//!   are reported, never warned about or refused, so local test servers and
//!   sidecars work under `require`.
//! - Off Linux, and with libcurl before 7.80: [`RouteCheck::Unsupported`];
//!   `require` refuses ([`QuickAckError::Lookup`] with
//!   [`RouteError::Unsupported`]).
//!
//! # Enabling quickack on a route
//!
//! As root, repeat the route exactly as `ip route show` prints it, `metric`
//! included (`ip route change` replaces the whole entry), and add
//! `quickack 1`:
//!
//! ```text
//! ip route show default
//! # default via 172.31.32.1 dev ens5 proto dhcp src 172.31.39.155 metric 100
//! ip route change default via 172.31.32.1 dev ens5 proto dhcp src 172.31.39.155 metric 100 quickack 1 rto_min 5ms
//! ```
//!
//! - The warning and the error print this command for the connection's own
//!   route; `rto_min 5ms` is optional.
//! - The subnet route needs the same if peers on the subnet matter.
//! - No command for routes from IPv6 router advertisements (`proto ra`),
//!   routes with `expires`, multipath routes or next-hop objects: set
//!   quickack where they come from.
//!
//! `ip route change` lasts until the route is rewritten (reboot, a DHCP
//! lease, a router advertisement). To keep it:
//!
//! | network manager | where |
//! |---|---|
//! | systemd-networkd (netplan renders to it; Amazon Linux 2023) | `QuickAck=yes` in `[DHCPv4]` and `[IPv6AcceptRA]` (systemd 253+) or in a static `[Route]` (237+; `TCPRetransmissionTimeoutSec=` sets `rto_min`, 255+). Netplan has no option: add a drop-in such as `/etc/systemd/network/10-netplan-ens5.network.d/quickack.conf` and restart `systemd-networkd` |
//! | `NetworkManager` (RHEL) | the `quickack` and `rto_min` attributes of `ipv4.routes` / `ipv6.routes` (static routes; DHCP routes need a hook) |
//! | any | a networkd-dispatcher or `NetworkManager` dispatcher hook that re-runs `ip route change` after each lease renewal |
//!
//! In a Kubernetes pod the routes are the pod's own: change them in the pod
//! (an init container with `NET_ADMIN`) or in the CNI configuration.

use std::fmt::{self, Write as _};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Mutex, PoisonError};

use crate::error::Error;

// Netlink and rtnetlink (linux/netlink.h, linux/rtnetlink.h): kernel ABI,
// the same numbers on every architecture (fields in host byte order, which
// the code below uses), so encoding and decoding are plain functions that
// build and run everywhere.
const NLMSG_HDRLEN: usize = 16;
const NLMSG_ERROR: u16 = 2;
const NLM_F_REQUEST: u16 = 1;
const RTM_NEWROUTE: u16 = 24;
const RTM_GETROUTE: u16 = 26;
const RTMSG_LEN: usize = 12;
const RTM_F_CLONED: u32 = 0x200;
const RTM_F_LOOKUP_TABLE: u32 = 0x1000;
const RTM_F_FIB_MATCH: u32 = 0x2000;
const RTNH_F_ONLINK: u32 = 4;
const AF_INET: u8 = 2;
const AF_INET6: u8 = 10;
const IPPROTO_TCP: u8 = 6;
/// The ECN bits of a TOS or traffic class: TCP sets them once ECN is agreed,
/// and routing ignores them.
const ECN_MASK: u8 = 3;
const NLA_TYPE_MASK: u16 = 0x3fff;

const RTA_DST: u16 = 1;
const RTA_SRC: u16 = 2;
const RTA_IIF: u16 = 3;
const RTA_OIF: u16 = 4;
const RTA_GATEWAY: u16 = 5;
const RTA_PRIORITY: u16 = 6;
const RTA_PREFSRC: u16 = 7;
const RTA_METRICS: u16 = 8;
const RTA_MULTIPATH: u16 = 9;
const RTA_FLOW: u16 = 11;
const RTA_CACHEINFO: u16 = 12;
const RTA_TABLE: u16 = 15;
const RTA_MARK: u16 = 16;
const RTA_VIA: u16 = 18;
const RTA_NEWDST: u16 = 19;
const RTA_PREF: u16 = 20;
const RTA_ENCAP_TYPE: u16 = 21;
const RTA_ENCAP: u16 = 22;
const RTA_EXPIRES: u16 = 23;
const RTA_PAD: u16 = 24;
const RTA_UID: u16 = 25;
const RTA_TTL_PROPAGATE: u16 = 26;
const RTA_IP_PROTO: u16 = 27;
const RTA_SPORT: u16 = 28;
const RTA_DPORT: u16 = 29;
const RTA_NH_ID: u16 = 30;

/// `RTAX_LOCK`: the bit mask of locked metrics (bit `n` locks metric `n`).
///
/// Hidden test lever: the netlink metric ABI. A caller reads the named
/// accessors on [`Route`] and [`Metrics`] instead.
#[doc(hidden)]
pub const RTAX_LOCK: u16 = 1;
/// `RTAX_MTU`.
#[doc(hidden)]
pub const RTAX_MTU: u16 = 2;
/// `RTAX_RTT`, in units of 1/8 ms.
#[doc(hidden)]
pub const RTAX_RTT: u16 = 4;
/// `RTAX_RTTVAR`, in units of 1/4 ms.
#[doc(hidden)]
pub const RTAX_RTTVAR: u16 = 5;
/// `RTAX_FEATURES`.
#[doc(hidden)]
pub const RTAX_FEATURES: u16 = 12;
/// `RTAX_RTO_MIN`: the route's minimum retransmission timeout, milliseconds.
#[doc(hidden)]
pub const RTAX_RTO_MIN: u16 = 13;
/// `RTAX_QUICKACK`: non-zero when the route disables delayed ACKs.
#[doc(hidden)]
pub const RTAX_QUICKACK: u16 = 15;
/// `RTAX_CC_ALGO`: the congestion control algorithm (a name).
#[doc(hidden)]
pub const RTAX_CC_ALGO: u16 = 16;
const RTAX_MAX: u16 = 17;
const RTAX_FEATURE_ECN: u32 = 1;

/// `ip route` names of `RTAX_*` (index `n - 1`).
const METRIC_NAMES: [&str; RTAX_MAX as usize] = [
    "lock",
    "mtu",
    "window",
    "rtt",
    "rttvar",
    "ssthresh",
    "cwnd",
    "advmss",
    "reordering",
    "hoplimit",
    "initcwnd",
    "features",
    "rto_min",
    "initrwnd",
    "quickack",
    "congctl",
    "fastopen_no_cookie",
];

const RTN_UNICAST: u8 = 1;
const RTN_LOCAL: u8 = 2;
const RT_TABLE_DEFAULT: u32 = 253;
const RT_TABLE_MAIN: u32 = 254;
const RT_TABLE_LOCAL: u32 = 255;
const RTPROT_BOOT: u8 = 3;
const RTPROT_RA: u8 = 9;
const RT_SCOPE_UNIVERSE: u8 = 0;
const RT_SCOPE_LINK: u8 = 253;

/// Largest request [`encode_request`] builds: header, route message,
/// IPv6 destination and source, interface, mark, protocol and both ports.
///
/// Hidden test lever: see [`encode_request`].
#[doc(hidden)]
pub const REQUEST_MAX: usize = NLMSG_HDRLEN + RTMSG_LEN + 2 * 20 + 5 * 8;

/// What to do when a connection's route does not disable delayed ACKs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum QuickAckCheck {
    /// Nothing is looked up.
    Off,
    /// The connection goes ahead; one warning per distinct route (or
    /// distinct lookup failure) names the route and the command that fixes
    /// it.
    #[default]
    Warn,
    /// The connection fails with [`Error::QuickAck`], as does one whose
    /// route cannot be read (including off Linux).
    Require,
}

impl QuickAckCheck {
    /// The configuration name: `off`, `warn` or `require`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Require => "require",
        }
    }
}

impl fmt::Display for QuickAckCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for QuickAckCheck {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        [Self::Off, Self::Warn, Self::Require]
            .into_iter()
            .find(|p| p.name() == s)
            .ok_or_else(|| {
                Error::Config(format!(
                    "quickack_check: expected off, warn or require, got {s:?}"
                ))
            })
    }
}

/// A connection's addressing as the route lookup needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Flow {
    /// Local address and port.
    pub local: SocketAddr,
    /// Peer address and port.
    pub peer: SocketAddr,
    /// Interface the socket is bound to (`SO_BINDTOIFINDEX`), 0 when none.
    pub oif: u32,
    /// The socket's mark (`SO_MARK`), 0 when none: policy routing may use it.
    pub mark: u32,
    /// The socket's type of service (`IP_TOS`) for an IPv4 flow, its traffic
    /// class (`IPV6_TCLASS`) for an IPv6 one, ECN bits cleared: routes and
    /// policy rules may match it (`ip route ... tos`, `ip rule ... tos`).
    pub tos: u8,
}

impl Flow {
    /// A flow from `local` to `peer`; IPv4-mapped IPv6 addresses become IPv4,
    /// as the kernel routes them.
    #[must_use]
    pub fn new(local: SocketAddr, peer: SocketAddr) -> Self {
        Self {
            local: unmap(local),
            peer: unmap(peer),
            oif: 0,
            mark: 0,
            tos: 0,
        }
    }

    /// With the interface the socket is bound to.
    #[must_use]
    pub const fn with_oif(mut self, oif: u32) -> Self {
        self.oif = oif;
        self
    }

    /// With the socket's mark.
    #[must_use]
    pub const fn with_mark(mut self, mark: u32) -> Self {
        self.mark = mark;
        self
    }

    /// With the socket's type of service or traffic class (ECN bits are
    /// cleared).
    #[must_use]
    pub const fn with_tos(mut self, tos: u8) -> Self {
        self.tos = tos & !ECN_MASK;
        self
    }

    /// The flow of a connected TCP socket: its addresses, bound interface,
    /// mark and type of service (or traffic class).
    ///
    /// # Errors
    /// [`RouteError::Socket`] when the socket has no peer (not connected) or
    /// is not an IP socket.
    #[cfg(target_os = "linux")]
    pub fn of_socket(fd: std::os::fd::RawFd) -> Result<Self, RouteError> {
        let s = crate::sys::socket_flow(fd).map_err(|e| RouteError::Socket(errno(&e)))?;
        Ok(Self::new(s.local, s.peer)
            .with_oif(s.oif)
            .with_mark(s.mark)
            .with_tos(s.tos))
    }

    fn describe(flow: Option<&Self>) -> String {
        flow.map_or_else(
            || "a TCP connection".to_owned(),
            |f| format!("TCP {} -> {}", f.local, f.peer),
        )
    }
}

fn unmap(a: SocketAddr) -> SocketAddr {
    match a {
        SocketAddr::V6(v6) => v6
            .ip()
            .to_ipv4_mapped()
            .map_or(a, |v4| SocketAddr::new(IpAddr::V4(v4), v6.port())),
        SocketAddr::V4(_) => a,
    }
}

/// The `errno` of an I/O error; `EIO` for one that has none.
#[cfg(target_os = "linux")]
fn errno(e: &std::io::Error) -> i32 {
    e.raw_os_error().unwrap_or(libc::EIO)
}

/// Why a connection's route could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RouteError {
    /// The lookup is not possible here (the reason says why: not Linux, or a
    /// libcurl without the pre-request callback).
    Unsupported(&'static str),
    /// The socket or the netlink exchange failed with this `errno`.
    Socket(i32),
    /// The kernel did not answer within the timeout.
    Timeout,
    /// The kernel refused the lookup with this `errno` (`ENETUNREACH` or
    /// `EHOSTUNREACH`: no route).
    Kernel(i32),
    /// The reply does not parse.
    Malformed(&'static str),
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(why) => write!(f, "{why}"),
            Self::Socket(e) => write!(f, "socket: {}", std::io::Error::from_raw_os_error(*e)),
            Self::Timeout => write!(f, "netlink: no answer within {LOOKUP_TIMEOUT_MS} ms"),
            Self::Kernel(e) => write!(f, "route lookup: {}", std::io::Error::from_raw_os_error(*e)),
            Self::Malformed(why) => write!(f, "netlink reply: {why}"),
        }
    }
}

impl std::error::Error for RouteError {}

/// How long the netlink exchange may take, in milliseconds.
pub(crate) const LOOKUP_TIMEOUT_MS: u32 = 100;

/// A route's metrics (`RTA_METRICS`), as the kernel reports them: a metric
/// set to 0 is not reported, so 0 and absent are the same.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metrics {
    values: [Option<u32>; RTAX_MAX as usize],
    cc_algo: Option<String>,
}

impl Metrics {
    /// Metric `rtax` (an `RTAX_*` number), if the route sets it. The
    /// congestion control algorithm is a name: see [`Metrics::cc_algo`].
    ///
    /// Hidden test lever: raw netlink metric numbers. Prefer [`Route::quickack`],
    /// [`Route::rto_min_ms`] and [`Metrics::cc_algo`].
    #[doc(hidden)]
    #[must_use]
    pub fn get(&self, rtax: u16) -> Option<u32> {
        let i = usize::from(rtax).checked_sub(1)?;
        self.values.get(i).copied().flatten()
    }

    /// Whether metric `rtax` is locked (`ip route ... mtu lock 1400`).
    ///
    /// Hidden test lever: see [`Metrics::get`].
    #[doc(hidden)]
    #[must_use]
    pub fn locked(&self, rtax: u16) -> bool {
        self.get(RTAX_LOCK)
            .is_some_and(|m| rtax < 32 && m & (1 << rtax) != 0)
    }

    /// The congestion control algorithm (`congctl`), if the route sets one.
    #[must_use]
    pub fn cc_algo(&self) -> Option<&str> {
        self.cc_algo.as_deref()
    }

    /// Writes the metrics as `ip route` takes them, quickack excepted.
    fn write(&self, out: &mut String, command: bool) {
        for rtax in 2..=RTAX_MAX {
            if rtax == RTAX_QUICKACK && command {
                continue;
            }
            let name = METRIC_NAMES[usize::from(rtax) - 1];
            let lock = if self.locked(rtax) && !(command && rtax == RTAX_RTO_MIN) {
                " lock"
            } else {
                ""
            };
            if rtax == RTAX_CC_ALGO {
                if let Some(cc) = &self.cc_algo {
                    let _ = write!(out, " {name}{lock} {cc}");
                }
                continue;
            }
            let Some(v) = self.get(rtax) else {
                continue;
            };
            let _ = match rtax {
                // `ip` shows these as times; in a command the raw number
                // (no unit) is taken unscaled, which is exact.
                RTAX_RTT | RTAX_RTTVAR | RTAX_RTO_MIN => {
                    let ms = match rtax {
                        RTAX_RTT if !command => v / 8,
                        RTAX_RTTVAR if !command => v / 4,
                        _ => v,
                    };
                    if command && rtax != RTAX_RTO_MIN {
                        write!(out, " {name}{lock} {v}")
                    } else if !command && ms >= 1000 {
                        write!(out, " {name}{lock} {}s", f64::from(ms) / 1000.0)
                    } else {
                        write!(out, " {name}{lock} {ms}ms")
                    }
                }
                RTAX_FEATURES if v == RTAX_FEATURE_ECN => write!(out, " {name} ecn"),
                _ => write!(out, " {name}{lock} {v}"),
            };
        }
    }
}

/// The routing-table entry the kernel uses for a connection, as
/// `ip route get fibmatch` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Route {
    /// Destination prefix (the unspecified address for the default route).
    pub dst: IpAddr,
    /// Prefix length (0: the default route).
    pub dst_len: u8,
    /// Source prefix of a source-specific (IPv6 `from`) route.
    pub src: Option<IpAddr>,
    /// Its length (0 when the route is not source-specific).
    pub src_len: u8,
    /// Type of service the route matches (0: any).
    pub tos: u8,
    /// Routing table (254: `main`, 255: `local`).
    pub table: u32,
    /// Who installed the route (`RTPROT_*`: 2 kernel, 3 boot, 4 static,
    /// 16 dhcp, ...).
    pub protocol: u8,
    /// Scope (`RT_SCOPE_*`: 0 global, 253 link, 254 host).
    pub scope: u8,
    /// Route type (`RTN_*`: 1 unicast, 2 local, ...).
    pub kind: u8,
    /// Route flags (`RTNH_F_*`, e.g. onlink).
    pub flags: u32,
    /// Next hop.
    pub gateway: Option<IpAddr>,
    /// Output interface index.
    pub oif: Option<u32>,
    /// Output interface name (filled in by a live [`lookup`]).
    pub dev: Option<String>,
    /// Preferred source address.
    pub prefsrc: Option<IpAddr>,
    /// Route metric (`ip route ... metric N`).
    pub priority: Option<u32>,
    /// IPv6 router preference (0 medium, 1 high, 3 low).
    pub pref: Option<u8>,
    /// The route's metrics: quickack, `rto_min`, MTU, ...
    pub metrics: Metrics,
    /// Something one `ip route change` cannot reproduce (multipath next
    /// hops, a next-hop object, encapsulation, ...): then there is no
    /// [`Route::fix_command`].
    pub unrendered: Option<&'static str>,
}

impl Route {
    /// Whether the route disables delayed ACKs (`quickack 1`).
    #[must_use]
    pub fn quickack(&self) -> bool {
        self.metrics.get(RTAX_QUICKACK).is_some_and(|v| v != 0)
    }

    /// The route's `rto_min` in milliseconds, if it sets one (the kernel's
    /// default is 200 ms).
    #[must_use]
    pub fn rto_min_ms(&self) -> Option<u32> {
        self.metrics.get(RTAX_RTO_MIN)
    }

    /// Whether the route leads to this host (type `local`: loopback or one of
    /// the host's own addresses). The check never warns about or refuses
    /// such a route.
    #[must_use]
    pub const fn is_local(&self) -> bool {
        self.kind == RTN_LOCAL
    }

    /// The `ip route change` command that adds `quickack 1` to this route and
    /// keeps every other attribute: run it as root. `None` when one command
    /// cannot reproduce the route (see [`Route::unrendered`]), when the
    /// interface's name is unknown, or for a route that is not unicast.
    #[must_use]
    pub fn fix_command(&self) -> Option<String> {
        if self.unrendered.is_some()
            || self.kind != RTN_UNICAST
            || (self.oif.is_some() && self.dev.is_none())
        {
            return None;
        }
        let mut s = String::from(if self.dst.is_ipv4() {
            "ip route change"
        } else {
            "ip -6 route change"
        });
        self.write(&mut s, true);
        s.push_str(" quickack 1");
        Some(s)
    }

    /// The route as `ip route show` prints it (`command`: as `ip route
    /// change` takes it, without the leading command).
    fn write(&self, out: &mut String, command: bool) {
        let v4 = self.dst.is_ipv4();
        let max = if v4 { 32 } else { 128 };
        if self.kind != RTN_UNICAST {
            let _ = write!(out, "{} ", kind_name(self.kind));
        } else if command {
            out.push(' ');
        }
        if self.dst_len == 0 {
            out.push_str("default");
        } else if self.dst_len == max {
            let _ = write!(out, "{}", self.dst);
        } else {
            let _ = write!(out, "{}/{}", self.dst, self.dst_len);
        }
        if let (Some(src), true) = (self.src, self.src_len > 0) {
            let _ = write!(out, " from {src}/{}", self.src_len);
        }
        if self.tos != 0 {
            let _ = write!(out, " tos 0x{:02x}", self.tos);
        }
        if let Some(gw) = self.gateway {
            let _ = write!(out, " via {gw}");
        }
        match (&self.dev, self.oif) {
            (Some(d), _) => {
                let _ = write!(out, " dev {d}");
            }
            (None, Some(i)) => {
                let _ = write!(out, " dev if{i}");
            }
            (None, None) => {}
        }
        if self.table != RT_TABLE_MAIN {
            let _ = match self.table {
                RT_TABLE_LOCAL => write!(out, " table local"),
                RT_TABLE_DEFAULT => write!(out, " table default"),
                t => write!(out, " table {t}"),
            };
        }
        if self.protocol != RTPROT_BOOT {
            let _ = write!(out, " proto {}", proto_name(self.protocol));
        }
        // `ip route add` gives a route without a gateway scope link unless
        // told otherwise, so a command spells out a global one. IPv6 has no
        // route scopes.
        if v4
            && (self.scope != RT_SCOPE_UNIVERSE
                || (command && self.gateway.is_none() && self.scope != RT_SCOPE_LINK))
        {
            let _ = write!(out, " scope {}", scope_name(self.scope));
        }
        if let Some(p) = self.prefsrc {
            let _ = write!(out, " src {p}");
        }
        if let Some(m) = self.priority {
            let _ = write!(out, " metric {m}");
        }
        if self.flags & RTNH_F_ONLINK != 0 {
            out.push_str(" onlink");
        }
        self.metrics.write(out, command);
        if let Some(p) = self.pref {
            let _ = write!(out, " pref {}", pref_name(p));
        }
        if let (Some(why), false) = (self.unrendered, command) {
            let _ = write!(out, " ({why})");
        }
    }

    /// Identifies the routing-table entry, for warning once per route.
    const fn key(&self) -> WarnKey {
        WarnKey::Route {
            table: self.table,
            dst: self.dst,
            dst_len: self.dst_len,
            src: self.src,
            src_len: self.src_len,
            tos: self.tos,
            priority: self.priority,
        }
    }

    /// The advice line: the exact command, or what to do without one.
    fn advice(&self) -> String {
        self.fix_command().map_or_else(
            || {
                format!(
                    "Fix: add `quickack 1` to this route in the network configuration \
                     (one `ip route change` cannot reproduce it: {})",
                    self.unrendered.unwrap_or("its interface name is unknown")
                )
            },
            |cmd| format!("Fix (as root): `{cmd}`"),
        )
    }

    fn rto_min_text(&self) -> String {
        self.rto_min_ms().map_or_else(
            || {
                "rto_min not set: the kernel's default of 200 ms, which a lost segment with \
                 nothing else in flight waits for before it is resent; the benchmarks use 5ms"
                    .to_owned()
            },
            |ms| format!("rto_min {ms}ms (the kernel's default is 200 ms; the benchmarks use 5ms)"),
        )
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        self.write(&mut s, false);
        f.write_str(&s)
    }
}

fn kind_name(kind: u8) -> String {
    match kind {
        2 => "local".into(),
        3 => "broadcast".into(),
        4 => "anycast".into(),
        5 => "multicast".into(),
        6 => "blackhole".into(),
        7 => "unreachable".into(),
        8 => "prohibit".into(),
        9 => "throw".into(),
        10 => "nat".into(),
        k => k.to_string(),
    }
}

fn proto_name(p: u8) -> String {
    match p {
        2 => "kernel".into(),
        3 => "boot".into(),
        4 => "static".into(),
        9 => "ra".into(),
        16 => "dhcp".into(),
        p => p.to_string(),
    }
}

fn scope_name(s: u8) -> String {
    match s {
        0 => "global".into(),
        200 => "site".into(),
        253 => "link".into(),
        254 => "host".into(),
        255 => "nowhere".into(),
        s => s.to_string(),
    }
}

fn pref_name(p: u8) -> String {
    match p {
        0 => "medium".into(),
        1 => "high".into(),
        3 => "low".into(),
        p => p.to_string(),
    }
}

/// Whether a connection's route disables delayed ACKs, as far as is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuickAck {
    /// The route sets `quickack 1`.
    On,
    /// The route does not: the kernel delays ACKs.
    Off,
    /// Not known: not checked, not checkable here, or the lookup failed.
    Unknown,
}

/// What the quick-ACK check found for one connection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RouteCheck {
    /// Not checked: the policy is [`QuickAckCheck::Off`], or the connection
    /// has not been through a driver yet.
    #[default]
    NotChecked,
    /// The route cannot be read here (the reason says why); the connection
    /// went ahead.
    Unsupported(&'static str),
    /// The lookup failed ([`QuickAckCheck::Warn`] only: `Require` fails the
    /// connection instead).
    Failed(RouteError),
    /// The route the connection uses.
    Route(Box<Route>),
    /// The lookup is queued to ergon-io's warning thread
    /// ([`QuickAckCheck::Warn`] on `Driver::connect` and `Driver::add`): the
    /// connection went ahead, and this reads as the outcome once that
    /// thread has made it.
    Pending,
}

impl RouteCheck {
    /// Quick ACK as observed.
    #[must_use]
    pub fn quickack(&self) -> QuickAck {
        match self {
            Self::Route(r) if r.quickack() => QuickAck::On,
            Self::Route(_) => QuickAck::Off,
            _ => QuickAck::Unknown,
        }
    }

    /// The route's `rto_min` in milliseconds, if known and set.
    #[must_use]
    pub fn rto_min_ms(&self) -> Option<u32> {
        self.route().and_then(Route::rto_min_ms)
    }

    /// The route, if it was read.
    #[must_use]
    pub fn route(&self) -> Option<&Route> {
        match self {
            Self::Route(r) => Some(r),
            _ => None,
        }
    }
}

/// Why [`QuickAckCheck::Require`] refused a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickAckError {
    /// The connection's route does not set `quickack 1`.
    Missing {
        /// The connection.
        flow: Flow,
        /// Its route.
        route: Box<Route>,
    },
    /// The route could not be read, so quickack could not be confirmed.
    Lookup {
        /// The connection, when its addresses could be read.
        flow: Option<Flow>,
        /// Why the route could not be read.
        error: RouteError,
    },
}

impl fmt::Display for QuickAckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { flow, route } => write!(
                f,
                "{}: route `{route}` has no quickack, so a peer that uses Nagle's algorithm \
                 waits for our delayed ACK (tcp(7); quickack_check=require). {}. Info: {}",
                Flow::describe(Some(flow)),
                route.advice(),
                route.rto_min_text()
            ),
            Self::Lookup { flow, error } => write!(
                f,
                "cannot confirm quickack for {} (quickack_check=require): {error}",
                Flow::describe(flow.as_ref())
            ),
        }
    }
}

impl std::error::Error for QuickAckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lookup { error, .. } => Some(error),
            Self::Missing { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WarnKey {
    Route {
        table: u32,
        dst: IpAddr,
        dst_len: u8,
        src: Option<IpAddr>,
        src_len: u8,
        tos: u8,
        priority: Option<u32>,
    },
    Failed(RouteError),
}

/// The routes and failures already warned about, so [`QuickAckCheck::Warn`]
/// reports each once. The driver and the libcurl client share one per
/// process; tests make their own.
///
/// Hidden test lever: a test injects its own warn state through
/// [`check_with`].
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct Warned(Mutex<Vec<WarnKey>>);

impl Warned {
    /// Nothing warned about yet.
    #[must_use]
    pub const fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    /// Records `key`; `true` the first time.
    fn first(&self, key: WarnKey) -> bool {
        let mut seen = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.contains(&key) {
            return false;
        }
        seen.push(key);
        true
    }
}

/// The process's record of what was warned about.
#[cfg(target_os = "linux")]
pub(crate) static WARNED: Warned = Warned::new();

/// Builds the `RTM_GETROUTE` request for `flow` into `out` and returns its
/// length.
///
/// The request asks for the routing-table entry and its table
/// (`RTM_F_FIB_MATCH | RTM_F_LOOKUP_TABLE`) for the flow's TOS or traffic
/// class (`rtm_tos`), with the peer as `RTA_DST`, the
/// local address as `RTA_SRC` (when of the same family), the bound interface
/// (`RTA_OIF`) and mark (`RTA_MARK`) when set, then TCP and both ports
/// (`RTA_IP_PROTO`, `RTA_SPORT`, `RTA_DPORT`) for policy rules that match
/// them.
///
/// Hidden test lever: the netlink request codec is an implementation detail
/// of [`lookup`]; `io/tests/route.rs` drives it against a stand-in peer.
#[doc(hidden)]
#[must_use]
pub fn encode_request(flow: &Flow, seq: u32, out: &mut [u8; REQUEST_MAX]) -> usize {
    let (family, bits, dst) = ip_parts(flow.peer.ip());
    let (src_family, _, src) = ip_parts(flow.local.ip());
    let same = src_family == family;
    let mut w = Writer { buf: out, len: 0 };
    w.put(&[0; NLMSG_HDRLEN]);
    let src_len = if same { bits } else { 0 };
    w.put(&[family, bits, src_len, flow.tos, 0, 0, 0, 0]);
    w.put(&(RTM_F_FIB_MATCH | RTM_F_LOOKUP_TABLE).to_ne_bytes());
    w.attr(RTA_DST, &dst[..usize::from(bits / 8)]);
    if same {
        w.attr(RTA_SRC, &src[..usize::from(bits / 8)]);
    }
    if flow.oif != 0 {
        w.attr(RTA_OIF, &flow.oif.to_ne_bytes());
    }
    if flow.mark != 0 {
        w.attr(RTA_MARK, &flow.mark.to_ne_bytes());
    }
    w.attr(RTA_IP_PROTO, &[IPPROTO_TCP]);
    w.attr(RTA_SPORT, &flow.local.port().to_be_bytes());
    w.attr(RTA_DPORT, &flow.peer.port().to_be_bytes());
    let len = w.len;
    #[allow(clippy::cast_possible_truncation)] // at most REQUEST_MAX
    let total = len as u32;
    out[0..4].copy_from_slice(&total.to_ne_bytes());
    out[4..6].copy_from_slice(&RTM_GETROUTE.to_ne_bytes());
    out[6..8].copy_from_slice(&NLM_F_REQUEST.to_ne_bytes());
    out[8..12].copy_from_slice(&seq.to_ne_bytes());
    len
}

/// Family, address length in bits and the address bytes (IPv4 in the first
/// four).
fn ip_parts(ip: IpAddr) -> (u8, u8, [u8; 16]) {
    let mut b = [0u8; 16];
    match ip {
        IpAddr::V4(a) => {
            b[..4].copy_from_slice(&a.octets());
            (AF_INET, 32, b)
        }
        IpAddr::V6(a) => (AF_INET6, 128, a.octets()),
    }
}

struct Writer<'a> {
    buf: &'a mut [u8; REQUEST_MAX],
    len: usize,
}

impl Writer<'_> {
    fn put(&mut self, b: &[u8]) {
        self.buf[self.len..self.len + b.len()].copy_from_slice(b);
        self.len += b.len();
    }

    /// One attribute, padded to 4 bytes.
    fn attr(&mut self, kind: u16, payload: &[u8]) {
        #[allow(clippy::cast_possible_truncation)] // at most 20
        let len = (4 + payload.len()) as u16;
        self.put(&len.to_ne_bytes());
        self.put(&kind.to_ne_bytes());
        self.put(payload);
        let pad = align4(self.len) - self.len;
        self.put(&[0; 3][..pad]);
    }
}

const fn align4(n: usize) -> usize {
    (n + 3) & !3
}

fn ne_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_ne_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn ne_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Netlink attributes of `b`, each as (type, payload).
fn attrs(mut b: &[u8]) -> impl Iterator<Item = Result<(u16, &[u8]), RouteError>> {
    std::iter::from_fn(move || {
        if b.is_empty() {
            return None;
        }
        let (Some(len), Some(kind)) = (ne_u16(b, 0), ne_u16(b, 2)) else {
            b = &[];
            return Some(Err(RouteError::Malformed("truncated attribute header")));
        };
        let len = usize::from(len);
        if len < 4 || len > b.len() {
            b = &[];
            return Some(Err(RouteError::Malformed(
                "attribute length outside its message",
            )));
        }
        let payload = &b[4..len];
        b = b.get(align4(len)..).unwrap_or_default();
        Some(Ok((kind & NLA_TYPE_MASK, payload)))
    })
}

fn ip_of(family: u8, b: &[u8]) -> Result<IpAddr, RouteError> {
    match family {
        AF_INET => <[u8; 4]>::try_from(b).map(|a| IpAddr::V4(Ipv4Addr::from(a))),
        _ => <[u8; 16]>::try_from(b).map(|a| IpAddr::V6(Ipv6Addr::from(a))),
    }
    .map_err(|_| RouteError::Malformed("address of the wrong length"))
}

fn u32_of(b: &[u8]) -> Result<u32, RouteError> {
    <[u8; 4]>::try_from(b)
        .map(u32::from_ne_bytes)
        .map_err(|_| RouteError::Malformed("32-bit attribute of the wrong length"))
}

/// Parses the kernel's answer to the request numbered `seq`: the route, or
/// the kernel's error.
///
/// Hidden test lever: see [`encode_request`].
///
/// # Errors
/// [`RouteError::Kernel`] when the kernel refused the lookup (no route),
/// [`RouteError::Malformed`] for anything that is not a complete, consistent
/// answer to `seq`. Every length is checked; nothing in the reply can make
/// this read out of bounds or panic.
#[doc(hidden)]
pub fn decode_reply(reply: &[u8], seq: u32) -> Result<Route, RouteError> {
    let malformed = RouteError::Malformed;
    let len = ne_u32(reply, 0).ok_or(RouteError::Malformed("shorter than a netlink header"))?;
    let len = usize::try_from(len).map_err(|_| malformed("netlink length out of range"))?;
    if len < NLMSG_HDRLEN {
        return Err(malformed("netlink length below its header"));
    }
    if len > reply.len() {
        return Err(malformed("netlink message longer than the reply"));
    }
    let kind = ne_u16(reply, 4).ok_or(RouteError::Malformed("shorter than a netlink header"))?;
    if ne_u32(reply, 8) != Some(seq) {
        return Err(malformed("answer to another request"));
    }
    let msg = &reply[NLMSG_HDRLEN..len];
    if kind == NLMSG_ERROR {
        let code = ne_u32(msg, 0).ok_or(RouteError::Malformed("error message without its code"))?;
        let code = i32::from_ne_bytes(code.to_ne_bytes());
        return Err(match code.checked_neg() {
            Some(errno) if errno > 0 => RouteError::Kernel(errno),
            _ if code == 0 => malformed("acknowledgement instead of a route"),
            _ => malformed("error code out of range"),
        });
    }
    if kind != RTM_NEWROUTE {
        return Err(malformed("not a route message"));
    }
    let rtm = msg
        .get(..RTMSG_LEN)
        .ok_or(RouteError::Malformed("truncated route message"))?;
    let family = rtm[0];
    let max = match family {
        AF_INET => 32,
        AF_INET6 => 128,
        _ => return Err(malformed("not an IPv4 or IPv6 route")),
    };
    let (dst_len, src_len) = (rtm[1], rtm[2]);
    if dst_len > max || src_len > max {
        return Err(malformed("prefix longer than the address"));
    }
    let flags = ne_u32(rtm, 8).ok_or(RouteError::Malformed("truncated route message"))?;
    let mut r = Route {
        dst: if family == AF_INET {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        } else {
            IpAddr::V6(Ipv6Addr::UNSPECIFIED)
        },
        dst_len,
        src: None,
        src_len,
        tos: rtm[3],
        table: u32::from(rtm[4]),
        protocol: rtm[5],
        scope: rtm[6],
        kind: rtm[7],
        flags,
        gateway: None,
        oif: None,
        dev: None,
        prefsrc: None,
        priority: None,
        pref: None,
        metrics: Metrics::default(),
        unrendered: None,
    };
    let mut has_dst = false;
    for a in attrs(&msg[RTMSG_LEN..]) {
        let (kind, p) = a?;
        has_dst |= kind == RTA_DST;
        decode_attr(kind, p, family, &mut r)?;
    }
    if dst_len > 0 && !has_dst {
        return Err(malformed("prefix without its destination"));
    }
    if src_len == 0 {
        r.src = None;
    }
    if r.protocol == RTPROT_RA {
        // The kernel keeps such a route by its router's advertisements; a
        // route `ip route change` puts in its place is no longer theirs.
        r.unrendered = Some("a route the kernel keeps from router advertisements (proto ra)");
    }
    if flags & RTM_F_CLONED != 0 {
        // Kernels before 4.13 ignore RTM_F_FIB_MATCH and answer with the
        // cached host route: its metrics are the routing entry's, its prefix
        // is not.
        r.unrendered = Some("the kernel answered with a cached route (before Linux 4.13)");
    }
    Ok(r)
}

/// Why a route with an expiry gets no command: `ip route change` would make
/// it permanent.
const EXPIRY: &str = "an expiry, as router advertisements set";

/// One attribute of a route message into `r`.
fn decode_attr(kind: u16, p: &[u8], family: u8, r: &mut Route) -> Result<(), RouteError> {
    match kind {
        RTA_DST => r.dst = ip_of(family, p)?,
        RTA_SRC => r.src = Some(ip_of(family, p)?),
        RTA_GATEWAY => r.gateway = Some(ip_of(family, p)?),
        RTA_PREFSRC => r.prefsrc = Some(ip_of(family, p)?),
        RTA_OIF => r.oif = Some(u32_of(p)?),
        RTA_PRIORITY => r.priority = Some(u32_of(p)?),
        RTA_TABLE => r.table = u32_of(p)?,
        RTA_PREF => {
            r.pref = Some(
                *p.first()
                    .ok_or(RouteError::Malformed("empty route preference"))?,
            );
        }
        RTA_METRICS => decode_metrics(p, r)?,
        RTA_MULTIPATH => r.unrendered = Some("multipath next hops"),
        RTA_NH_ID => r.unrendered = Some("a next-hop object (nhid)"),
        RTA_VIA => r.unrendered = Some("a gateway of another address family (via)"),
        RTA_ENCAP | RTA_ENCAP_TYPE => r.unrendered = Some("encapsulation"),
        RTA_NEWDST | RTA_TTL_PROPAGATE => r.unrendered = Some("MPLS"),
        RTA_EXPIRES => r.unrendered = Some(EXPIRY),
        RTA_FLOW => r.unrendered = Some("realms"),
        // `struct rta_cacheinfo`: the time left before the route expires,
        // in clock ticks, at byte 8 (0: it does not expire).
        RTA_CACHEINFO => {
            let expires =
                ne_u32(p, 8).ok_or(RouteError::Malformed("truncated cache information"))?;
            if expires != 0 {
                r.unrendered = Some(EXPIRY);
            }
        }
        // Lookup details, not part of the routing-table entry.
        RTA_IIF | RTA_MARK | RTA_PAD | RTA_UID | RTA_IP_PROTO | RTA_SPORT | RTA_DPORT => {}
        _ => r.unrendered = Some("an attribute this check does not know"),
    }
    Ok(())
}

fn decode_metrics(b: &[u8], r: &mut Route) -> Result<(), RouteError> {
    for a in attrs(b) {
        let (rtax, p) = a?;
        if rtax == RTAX_CC_ALGO {
            let name = p.split(|&c| c == 0).next().unwrap_or_default();
            let name = std::str::from_utf8(name)
                .ok()
                .filter(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_graphic()))
                .ok_or(RouteError::Malformed(
                    "congestion control name not printable",
                ))?;
            r.metrics.cc_algo = Some(name.to_owned());
        } else if (1..=RTAX_MAX).contains(&rtax) {
            let v = <[u8; 4]>::try_from(p)
                .map(u32::from_ne_bytes)
                .map_err(|_| RouteError::Malformed("metric of the wrong length"))?;
            r.metrics.values[usize::from(rtax) - 1] = Some(v);
        } else {
            r.unrendered = Some("a metric this check does not know");
        }
    }
    let features = r.metrics.get(RTAX_FEATURES).unwrap_or(0);
    if features & !RTAX_FEATURE_ECN != 0 {
        r.unrendered = Some("route features other than ecn");
    }
    Ok(())
}

/// Route lookups made so far in this process.
static LOOKUPS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How many route lookups (netlink exchanges, see [`lookup`]) this process
/// has made. The check makes one per connection it sets up and none per
/// request or poll; this lets a test or a health check confirm it.
///
/// Hidden test lever: a lookup counter for tests and health checks.
#[doc(hidden)]
#[must_use]
pub fn lookups() -> u64 {
    LOOKUPS.load(std::sync::atomic::Ordering::Relaxed)
}

thread_local! {
    /// Route lookups made so far by this thread.
    static LOOKUPS_HERE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many route lookups the calling thread has made: a test (or a
/// health check) can confirm with it that a polling thread makes none under
/// [`QuickAckCheck::Warn`].
///
/// Hidden test lever: see [`lookups`].
#[doc(hidden)]
#[must_use]
pub fn lookups_here() -> u64 {
    LOOKUPS_HERE.with(std::cell::Cell::get)
}

/// Asks the kernel which route `flow` uses.
///
/// On Linux: one request and one reply on a fresh `NETLINK_ROUTE` socket,
/// waiting at most 100 ms; the interface name is filled in
/// from its index. Elsewhere: [`RouteError::Unsupported`].
///
/// # Errors
/// The socket or the exchange failed, no answer came, the kernel found no
/// route, or the answer does not parse.
#[allow(clippy::missing_const_for_fn)] // const only where the body is compiled out
pub fn lookup(flow: &Flow) -> Result<Route, RouteError> {
    #[cfg(target_os = "linux")]
    {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(1);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        LOOKUPS.fetch_add(1, Ordering::Relaxed);
        LOOKUPS_HERE.with(|n| n.set(n.get() + 1));
        let mut req = [0u8; REQUEST_MAX];
        let n = encode_request(flow, seq, &mut req);
        let mut reply = [0u8; 8192];
        let got = crate::sys::rtnetlink_exchange(
            &req[..n],
            &mut reply,
            std::time::Duration::from_millis(u64::from(LOOKUP_TIMEOUT_MS)),
        )
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::TimedOut => RouteError::Timeout,
            std::io::ErrorKind::InvalidData => {
                RouteError::Malformed("no complete answer from the kernel")
            }
            _ => RouteError::Socket(errno(&e)),
        })?;
        let mut route = decode_reply(&reply[..got], seq)?;
        route.dev = route.oif.and_then(crate::sys::if_name);
        Ok(route)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = flow;
        Err(RouteError::Unsupported(
            "route metrics can only be read on Linux",
        ))
    }
}

/// Applies `policy` to one connection.
///
/// `flow` is its addressing (or why it could not be read), `lookup` finds
/// its route ([`lookup`], or a stand-in in tests), `warned` remembers what
/// was reported and `log` takes each warning line. Under
/// [`QuickAckCheck::Off`] neither `lookup` nor `log` is called.
///
/// # Errors
/// Under [`QuickAckCheck::Require`] only: the route has no quickack and is
/// not local ([`QuickAckError::Missing`]), or it could not be read
/// ([`QuickAckError::Lookup`]).
///
/// Hidden test lever: the `lookup` and `warned` parameters exist so a test
/// can inject a stand-in peer and its own warn state. Use [`check_socket`].
#[doc(hidden)]
pub fn check_with(
    policy: QuickAckCheck,
    flow: Result<Flow, RouteError>,
    lookup: impl FnOnce(&Flow) -> Result<Route, RouteError>,
    warned: &Warned,
    log: impl FnOnce(String),
) -> Result<RouteCheck, QuickAckError> {
    if policy == QuickAckCheck::Off {
        return Ok(RouteCheck::NotChecked);
    }
    let found = match flow {
        Ok(f) => lookup(&f).map(|r| (f, r)).map_err(|e| (Some(f), e)),
        Err(e) => Err((None, e)),
    };
    match found {
        Ok((flow, route)) => {
            if route.quickack() || route.is_local() {
                return Ok(RouteCheck::Route(Box::new(route)));
            }
            if policy == QuickAckCheck::Require {
                return Err(QuickAckError::Missing {
                    flow,
                    route: Box::new(route),
                });
            }
            if warned.first(route.key()) {
                log(format!(
                    "ergon-io warning: {} goes over route `{route}`, which has no quickack: \
                     the kernel delays our ACKs, and a peer that uses Nagle's algorithm holds \
                     its next small segment until one arrives (tcp(7)). {}, and make it \
                     persistent in the network configuration (see ergon_io::route). Info: {}. \
                     Reported once per route; quickack_check=require refuses such connections, \
                     quickack_check=off skips the check.",
                    Flow::describe(Some(&flow)),
                    route.advice(),
                    route.rto_min_text()
                ));
            }
            Ok(RouteCheck::Route(Box::new(route)))
        }
        Err((flow, error)) => {
            if policy == QuickAckCheck::Require {
                return Err(QuickAckError::Lookup { flow, error });
            }
            if warned.first(WarnKey::Failed(error)) {
                log(format!(
                    "ergon-io warning: cannot check quickack on the route of {}: {error}; \
                     connections go ahead unchecked. Reported once per distinct failure; \
                     quickack_check=off skips the check.",
                    Flow::describe(flow.as_ref())
                ));
            }
            Ok(match error {
                RouteError::Unsupported(why) => RouteCheck::Unsupported(why),
                e => RouteCheck::Failed(e),
            })
        }
    }
}

/// Checks a connected TCP socket under `policy` with the kernel's routes,
/// the process's record of what was warned about, and ergon-io's warning
/// writer. Under [`QuickAckCheck::Off`] it makes no system call.
///
/// # Errors
/// As [`check_with`].
#[cfg(target_os = "linux")]
pub fn check_socket(
    policy: QuickAckCheck,
    fd: std::os::fd::RawFd,
) -> Result<RouteCheck, QuickAckError> {
    if policy == QuickAckCheck::Off {
        return Ok(RouteCheck::NotChecked);
    }
    check_with(
        policy,
        Flow::of_socket(fd),
        lookup,
        &WARNED,
        crate::warn::emit,
    )
}
