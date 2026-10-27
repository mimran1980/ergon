//! The NIC checks (feature `nic`): N1 ENA adaptive RX moderation, N2
//! deferred interrupts without a busy poller, N3 the Azure
//! accelerated-networking VF, N4 the kernel RX ring.
//!
//! N1 and N4 ask the kernel through [`Ethtool`]: [`Netlink`] sends ethtool's
//! generic-netlink `COALESCE_GET` and `RINGS_GET`, which need no privilege
//! and work from any network namespace. [`EthtoolCache`], owned by the
//! caller, keeps the answers per (interface index, `carrier_changes`), so a
//! reset (an ENA reset turns adaptive moderation back on and re-hints the
//! IRQs) makes the next check ask again.

mod genl;

use std::fmt;
use std::time::Duration;

use crate::finding::{Finding, Severity};
use crate::host::{Host, ReadError};
use crate::net::{carrier_changes, check_interface, interface_device, lower_interfaces, no_device};
use crate::platform::Platform;

/// A NIC's interrupt coalescing, as ethtool's `COALESCE_GET` reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coalesce {
    /// Adaptive RX moderation; `None` when the driver does not offer it.
    pub use_adaptive_rx: Option<bool>,
    /// The RX interrupt delay (µs); `None` when not offered.
    pub rx_usecs: Option<u32>,
}

/// A NIC's ring sizes, as ethtool's `RINGS_GET` reports them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rings {
    /// RX ring entries; `None` when not reported.
    pub rx: Option<u32>,
    /// The most RX ring entries the NIC takes; `None` when not reported.
    pub rx_max: Option<u32>,
}

/// Why an ethtool query has no answer.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EthtoolError {
    /// Not Linux.
    NotLinux,
    /// The host is a tree standing for one ([`Host::at`]): the kernel
    /// answers only for the running host.
    NotLive,
    /// The interface's index or `carrier_changes` could not be read.
    Interface(ReadError),
    /// The netlink socket failed (the error number).
    Socket(i32),
    /// The kernel did not answer in time.
    Timeout,
    /// The kernel has no ethtool netlink family (before Linux 5.6, or built
    /// without `CONFIG_ETHTOOL_NETLINK`).
    NoFamily,
    /// The kernel refused (the error number): `EOPNOTSUPP` when the driver
    /// has no such settings, `ENODEV` when the interface went away.
    Kernel(i32),
    /// The answer did not parse.
    Malformed(&'static str),
}

impl fmt::Display for EthtoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLinux => f.write_str("ethtool netlink: only Linux has it"),
            Self::NotLive => f.write_str("ethtool netlink: not the running host"),
            Self::Interface(e) => write!(f, "{e}"),
            Self::Socket(errno) => write!(f, "ethtool netlink: socket error (errno {errno})"),
            Self::Timeout => write!(
                f,
                "ethtool netlink: no answer within {} ms",
                TIMEOUT.as_millis()
            ),
            Self::NoFamily => f.write_str(
                "ethtool netlink: the kernel has no ethtool family (Linux 5.6 or later with CONFIG_ETHTOOL_NETLINK)",
            ),
            Self::Kernel(errno) => {
                write!(f, "ethtool netlink: the kernel refused (errno {errno})")
            }
            Self::Malformed(what) => write!(f, "ethtool netlink: {what}"),
        }
    }
}

impl std::error::Error for EthtoolError {}

/// `EOPNOTSUPP` as this architecture's kernel numbers it (95 on x86-64 and
/// arm64; Alpha, MIPS, PA-RISC and SPARC differ).
#[cfg(target_os = "linux")]
const EOPNOTSUPP: i32 = rustix::io::Errno::OPNOTSUPP.raw_os_error();
/// Elsewhere no kernel answers ([`EthtoolError::NotLinux`]); Linux's
/// x86-64 and arm64 number, for stand-in sources in tests.
#[cfg(not(target_os = "linux"))]
const EOPNOTSUPP: i32 = 95;

/// How long one netlink exchange may wait, as for ergon-io's route lookup.
const TIMEOUT: Duration = Duration::from_millis(100);

/// Where ethtool settings come from: [`Netlink`] on the running host, or a
/// stand-in in tests.
pub trait Ethtool {
    /// The coalescing settings of interface `ifindex` on `host`.
    ///
    /// # Errors
    /// No answer: see [`EthtoolError`].
    fn coalesce(&mut self, host: &Host, ifindex: u32) -> Result<Coalesce, EthtoolError>;

    /// The ring sizes of interface `ifindex` on `host`.
    ///
    /// # Errors
    /// No answer: see [`EthtoolError`].
    fn rings(&mut self, host: &Host, ifindex: u32) -> Result<Rings, EthtoolError>;
}

/// Ethtool over generic netlink, unprivileged.
///
/// One request and its reply on a fresh `NETLINK_GENERIC` socket per
/// query, with 100 ms send and receive timeouts on each exchange. Resolves
/// the `ethtool` family once, in an exchange of its own, and keeps its id.
#[derive(Clone, Debug, Default)]
pub struct Netlink {
    family: Option<u16>,
    seq: u32,
}

impl Netlink {
    /// A client that has not yet resolved the family.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            family: None,
            seq: 0,
        }
    }

    const fn next_seq(&mut self) -> u32 {
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    fn family(&mut self) -> Result<u16, EthtoolError> {
        if let Some(f) = self.family {
            return Ok(f);
        }
        let seq = self.next_seq();
        let mut reply = vec![0u8; 8192];
        let n = exchange(&genl::get_family(seq), &mut reply)?;
        let f = genl::decode_family(&reply[..n], seq)?;
        self.family = Some(f);
        Ok(f)
    }

    fn query(
        &mut self,
        host: &Host,
        cmd: u8,
        ifindex: u32,
    ) -> Result<(Vec<u8>, u32, u16), EthtoolError> {
        if !host.is_live() {
            return Err(EthtoolError::NotLive);
        }
        let family = self.family()?;
        let seq = self.next_seq();
        let mut reply = vec![0u8; 4096];
        let n = exchange(&genl::ethtool_get(family, cmd, ifindex, seq), &mut reply)?;
        reply.truncate(n);
        Ok((reply, seq, family))
    }
}

impl Ethtool for Netlink {
    fn coalesce(&mut self, host: &Host, ifindex: u32) -> Result<Coalesce, EthtoolError> {
        let (reply, seq, family) = self.query(host, genl::ETHTOOL_MSG_COALESCE_GET, ifindex)?;
        genl::decode_coalesce(&reply, seq, family, ifindex)
    }

    fn rings(&mut self, host: &Host, ifindex: u32) -> Result<Rings, EthtoolError> {
        let (reply, seq, family) = self.query(host, genl::ETHTOOL_MSG_RINGS_GET, ifindex)?;
        genl::decode_rings(&reply, seq, family, ifindex)
    }
}

/// One request and its reply on a fresh `NETLINK_GENERIC` socket, as
/// ergon-io's rtnetlink exchange: the kernel answers inside `sendto`, a
/// signal is the only reason to look twice, and a reply must come from the
/// kernel (port 0) and fit the buffer.
#[cfg(target_os = "linux")]
fn exchange(req: &[u8], reply: &mut [u8]) -> Result<usize, EthtoolError> {
    use rustix::io::Errno;
    use rustix::net::netlink::{self, SocketAddrNetlink};
    use rustix::net::sockopt::{self, Timeout};
    use rustix::net::{AddressFamily, RecvFlags, SendFlags, SocketFlags, SocketType};

    let failed = |e: Errno| {
        if e == Errno::AGAIN || e == Errno::WOULDBLOCK {
            EthtoolError::Timeout
        } else {
            EthtoolError::Socket(e.raw_os_error())
        }
    };
    let fd = rustix::net::socket_with(
        AddressFamily::NETLINK,
        SocketType::RAW,
        SocketFlags::CLOEXEC,
        Some(netlink::GENERIC),
    )
    .map_err(failed)?;
    sockopt::set_socket_timeout(&fd, Timeout::Recv, Some(TIMEOUT)).map_err(failed)?;
    sockopt::set_socket_timeout(&fd, Timeout::Send, Some(TIMEOUT)).map_err(failed)?;
    let kernel = SocketAddrNetlink::new(0, 0);
    let sent = rustix::net::sendto(&fd, req, SendFlags::empty(), &kernel).map_err(failed)?;
    if sent != req.len() {
        return Err(EthtoolError::Malformed("netlink request cut short"));
    }
    for _ in 0..3 {
        match rustix::net::recvfrom(&fd, &mut *reply, RecvFlags::TRUNC) {
            Ok((_, n, from)) => {
                let from = from
                    .and_then(|a| SocketAddrNetlink::try_from(a).ok())
                    .ok_or(EthtoolError::Malformed("netlink reply without its sender"))?;
                if from.pid() != 0 {
                    return Err(EthtoolError::Malformed("netlink reply not from the kernel"));
                }
                if n > reply.len() {
                    return Err(EthtoolError::Malformed(
                        "netlink reply larger than the buffer",
                    ));
                }
                return Ok(n);
            }
            Err(Errno::INTR) => {}
            Err(e) => return Err(failed(e)),
        }
    }
    Err(EthtoolError::Socket(Errno::INTR.raw_os_error()))
}

#[cfg(not(target_os = "linux"))]
const fn exchange(req: &[u8], reply: &mut [u8]) -> Result<usize, EthtoolError> {
    let _ = (req, reply);
    Err(EthtoolError::NotLinux)
}

/// What is kept for one interface.
#[derive(Debug)]
struct Entry {
    ifindex: u32,
    carrier_changes: u64,
    coalesce: Option<Result<Coalesce, EthtoolError>>,
    rings: Option<Result<Rings, EthtoolError>>,
}

/// Ethtool answers per interface, valid while its index and
/// `carrier_changes` stay the same.
///
/// Owned by the caller (no global state): ergon-io's warning thread or the
/// media driver's housekeeping keeps one. Timeouts and socket errors are
/// not kept.
#[derive(Debug, Default)]
pub struct EthtoolCache<E: Ethtool = Netlink> {
    source: E,
    entries: Vec<Entry>,
    queries: u64,
}

impl EthtoolCache<Netlink> {
    /// An empty cache over [`Netlink`].
    #[must_use]
    pub const fn new() -> Self {
        Self::with_source(Netlink::new())
    }
}

/// Whether an answer may be kept: not a transient failure.
const fn keep<T>(r: &Result<T, EthtoolError>) -> bool {
    !matches!(
        r,
        Err(EthtoolError::Timeout
            | EthtoolError::Socket(_)
            | EthtoolError::Interface(_)
            | EthtoolError::NotLive
            | EthtoolError::NotLinux)
    )
}

impl<E: Ethtool> EthtoolCache<E> {
    /// An empty cache over `source`.
    #[must_use]
    pub const fn with_source(source: E) -> Self {
        Self {
            source,
            entries: Vec::new(),
            queries: 0,
        }
    }

    /// How many queries reached the source.
    #[must_use]
    pub const fn queries(&self) -> u64 {
        self.queries
    }

    /// The source, to change a stand-in's answers.
    pub const fn source_mut(&mut self) -> &mut E {
        &mut self.source
    }

    /// The entry for `interface`, emptied when its `carrier_changes` moved.
    fn entry(&mut self, host: &Host, interface: &str) -> Result<usize, EthtoolError> {
        check_interface(interface).map_err(EthtoolError::Interface)?;
        let path = format!("/sys/class/net/{interface}/ifindex");
        let ifindex = host
            .read_i64(&path)
            .and_then(|i| {
                u32::try_from(i).map_err(|_| ReadError::Malformed {
                    path,
                    detail: "not an interface index",
                })
            })
            .map_err(EthtoolError::Interface)?;
        let carrier = carrier_changes(host, interface).map_err(EthtoolError::Interface)?;
        if let Some(i) = self.entries.iter().position(|e| e.ifindex == ifindex) {
            let e = &mut self.entries[i];
            if e.carrier_changes != carrier {
                e.carrier_changes = carrier;
                e.coalesce = None;
                e.rings = None;
            }
            return Ok(i);
        }
        self.entries.push(Entry {
            ifindex,
            carrier_changes: carrier,
            coalesce: None,
            rings: None,
        });
        Ok(self.entries.len() - 1)
    }

    /// `interface`'s coalescing settings, asked once per (index,
    /// `carrier_changes`).
    ///
    /// # Errors
    /// No answer: see [`EthtoolError`].
    pub fn coalesce(&mut self, host: &Host, interface: &str) -> Result<Coalesce, EthtoolError> {
        let i = self.entry(host, interface)?;
        if let Some(r) = &self.entries[i].coalesce {
            return r.clone();
        }
        self.queries += 1;
        let r = self.source.coalesce(host, self.entries[i].ifindex);
        if keep(&r) {
            self.entries[i].coalesce = Some(r.clone());
        }
        r
    }

    /// `interface`'s ring sizes, asked once per (index, `carrier_changes`).
    ///
    /// # Errors
    /// No answer: see [`EthtoolError`].
    pub fn rings(&mut self, host: &Host, interface: &str) -> Result<Rings, EthtoolError> {
        let i = self.entry(host, interface)?;
        if let Some(r) = &self.entries[i].rings {
            return r.clone();
        }
        self.queries += 1;
        let r = self.source.rings(host, self.entries[i].ifindex);
        if keep(&r) {
            self.entries[i].rings = Some(r.clone());
        }
        r
    }
}

/// `interface`'s driver, or the unknown finding when its device is not
/// visible.
fn driver_of(
    host: &Host,
    interface: &str,
    id: &'static str,
    evidence: &'static str,
    what: &str,
) -> Result<String, Finding> {
    match interface_device(host, interface) {
        Ok(Some(d)) => Ok(d.driver.unwrap_or_else(|| "none".to_owned())),
        Ok(None) => Err(Finding::unknown(
            id,
            Severity::Warn,
            evidence,
            format!("cannot tell {what}"),
            no_device(interface),
        )),
        Err(e) => Err(Finding::unknown(
            id,
            Severity::Warn,
            evidence,
            format!("cannot tell {what}"),
            e,
        )),
    }
}

const N1_EVIDENCE: &str = "git show \
     128747b9^:io/benches/results/flags-aws-20261015T070339Z/tables.md (c1/c2-base, -nodim, \
     -nocoalesce: lines 22-29, 50-57, 63-70; loop counters lines 386, 598, 1232, 1450; \
     pre-empting worker kworker/2:1, lines 186 and 190): AWS c7i, recv-busy, two 8-minute \
     reps each; io/benches/results/flags-aws-20261019T175248Z/env-rcv.txt \
     (lines 81-82, 146-149: on with rx-usecs 20 as found, back on after a reset); Linux \
     drivers/net/ethernet/amazon/ena/ena_netdev.c (ena_up enables adaptive moderation); \
     aws-graviton-getting-started perfrunbook (adaptive-rx off for latency-sensitive work)";

/// N1: ENA's adaptive RX interrupt moderation is off on `interface`. For
/// the kernel-socket paths (ergon-io, the media driver's socket transport)
/// on AWS, x86 and Graviton; other drivers are not checked.
#[must_use]
pub fn n1_adaptive_rx<E: Ethtool>(
    host: &Host,
    interface: &str,
    cache: &mut EthtoolCache<E>,
) -> Finding {
    let id = "N1";
    let sev = Severity::Warn;
    let what = format!("whether {interface}'s adaptive RX moderation is on");
    let driver = match driver_of(host, interface, id, N1_EVIDENCE, &what) {
        Ok(d) => d,
        Err(f) => return f,
    };
    if driver != "ena" {
        return Finding::ok(
            id,
            sev,
            N1_EVIDENCE,
            format!("{interface}'s driver is {driver}: N1 is for AWS's ENA"),
        );
    }
    match cache.coalesce(host, interface) {
        Ok(Coalesce {
            use_adaptive_rx: Some(true),
            ..
        }) => {
            let platform = if Platform::detect(host).is_graviton() {
                " On Graviton it is not A/B'd here; AWS's Graviton performance runbook says to \
                 turn it off for latency-sensitive work."
            } else {
                " Its cost with an interrupt-driven socket is not measured here."
            };
            Finding::found(
                id,
                sev,
                N1_EVIDENCE,
                format!(
                    "{interface}: adaptive RX interrupt moderation is on: on AWS c7i a kernel \
                     worker pre-empted a busy-polling loop 8,841–8,906 times in 8 minutes (about 18 a \
                     second), and 0 times with it off. Off, the WebSocket round trip's p99.99 was \
                     72–76 against 117–119 µs (max 354–970 against 259–1,024 µs); the HTTP round \
                     trip's p99.99 was 83–116 against 77–119 µs (max 202–981 against 433–446 µs); \
                     market data's p99 rose to 9.3–12.9 from 6.5 µs (max 25–29 against 34 µs). \
                     With rx-usecs 0 as well: WebSocket p99.99 78–80 µs (max 233–344), HTTP 74–81 µs \
                     (max 202–227), market-data p99 7.9–8.6 µs (max 34–41). One run, two reps \
                     each.{platform} Fix: `ethtool -C {interface} adaptive-rx off rx-usecs 0`; ENA \
                     turns adaptive moderation back on at every reset, so apply it again after one \
                     (when carrier_changes moves)"
                ),
            )
        }
        Ok(Coalesce {
            use_adaptive_rx: Some(false),
            rx_usecs,
        }) => Finding::ok(
            id,
            sev,
            N1_EVIDENCE,
            format!(
                "{interface}: adaptive RX moderation is off{}",
                rx_usecs.map_or_else(String::new, |u| format!(" (rx-usecs {u})"))
            ),
        ),
        Ok(Coalesce {
            use_adaptive_rx: None,
            ..
        }) => Finding::ok(
            id,
            sev,
            N1_EVIDENCE,
            format!("{interface}: the driver does not offer adaptive RX moderation"),
        ),
        Err(EthtoolError::Kernel(EOPNOTSUPP)) => Finding::ok(
            id,
            sev,
            N1_EVIDENCE,
            format!("{interface}: the driver reports no interrupt coalescing (EOPNOTSUPP)"),
        ),
        Err(e) => Finding::unknown(id, sev, N1_EVIDENCE, format!("cannot tell {what}"), e),
    }
}

const N2_EVIDENCE: &str = "Linux Documentation/networking/napi.rst; the lab sets them only with busy \
     polling (md-dpdk/benches/cloud/vm.sh); not measured here";

/// One sysfs NAPI setting: absent (an older kernel) is 0.
fn napi_setting(host: &Host, interface: &str, name: &str) -> Result<u64, ReadError> {
    let path = format!("/sys/class/net/{interface}/{name}");
    host.read_optional(&path)?.map_or(Ok(0), |t| {
        crate::host::first_line(&t)
            .parse()
            .map_err(|_| ReadError::Malformed {
                path,
                detail: "not a number",
            })
    })
}

/// N2: `interface` (and on Azure its VF) defers no interrupts when no
/// socket on it busy-polls.
///
/// Deferral is `napi_defer_hard_irqs` and `gro_flush_timeout` both above 0.
/// `busy_polls` is whether the caller's sockets on it busy-poll: `None`
/// when that is not known (`net.core.busy_read` unreadable in a pod), which
/// makes a deferring device unknown (one that defers nothing is ok either
/// way).
#[must_use]
pub fn n2_deferred_irqs(host: &Host, interface: &str, busy_polls: Option<bool>) -> Vec<Finding> {
    let id = "N2";
    let sev = Severity::Warn;
    let what = format!("whether {interface} defers its interrupts");
    if let Err(f) = driver_of(host, interface, id, N2_EVIDENCE, &what) {
        return vec![f];
    }
    if busy_polls == Some(true) {
        return vec![Finding::ok(
            id,
            sev,
            N2_EVIDENCE,
            format!("{interface}'s sockets busy-poll: deferred interrupts are how that is set up"),
        )];
    }
    let mut targets = vec![interface.to_owned()];
    match lower_interfaces(host, interface) {
        Ok(l) => targets.extend(l),
        Err(e) => {
            return vec![Finding::unknown(
                id,
                sev,
                N2_EVIDENCE,
                format!("cannot tell {what}"),
                e,
            )];
        }
    }
    let graviton = Platform::detect(host).is_graviton();
    let mut out = Vec::new();
    for t in &targets {
        let read = napi_setting(host, t, "napi_defer_hard_irqs")
            .and_then(|n| napi_setting(host, t, "gro_flush_timeout").map(|ns| (n, ns)));
        let (defer, timeout) = match read {
            Ok(v) => v,
            Err(e) => {
                out.push(Finding::unknown(
                    id,
                    sev,
                    N2_EVIDENCE,
                    format!("cannot tell whether {t} defers its interrupts"),
                    e,
                ));
                continue;
            }
        };
        if defer == 0 || timeout == 0 {
            continue;
        }
        if busy_polls.is_none() {
            out.push(Finding::unknown(
                id,
                sev,
                N2_EVIDENCE,
                format!(
                    "cannot tell whether {t}'s deferred interrupts (napi_defer_hard_irqs {defer}, \
                     gro_flush_timeout {timeout} ns) matter"
                ),
                "whether its sockets busy-poll is not known",
            ));
            continue;
        }
        let wait = if timeout % 1000 == 0 {
            format!("{} µs", timeout / 1000)
        } else {
            format!("{timeout} ns")
        };
        let alternative = if graviton {
            " (on Graviton, busy polling is not suggested: no archived run supports it there)"
        } else {
            ", or turn busy polling on"
        };
        out.push(Finding::found(
            id,
            sev,
            N2_EVIDENCE,
            format!(
                "{t}: interrupts are deferred (napi_defer_hard_irqs {defer}, gro_flush_timeout \
                 {timeout} ns) but no socket on it busy-polls: packets wait for the {wait} timer. \
                 Fix: `echo 0 > /sys/class/net/{t}/napi_defer_hard_irqs; echo 0 > \
                 /sys/class/net/{t}/gro_flush_timeout`{alternative}"
            ),
        ));
    }
    if out.is_empty() {
        out.push(Finding::ok(
            id,
            sev,
            N2_EVIDENCE,
            format!("{}: no deferred interrupts", targets.join(", ")),
        ));
    }
    out
}

const N3_EVIDENCE: &str = "Azure's accelerated networking documentation; not measured here";

/// N3: an Azure `hv_netvsc` interface has an accelerated-networking VF
/// (`lower_*`) that is up. Other drivers are not checked.
#[must_use]
pub fn n3_azure_vf(host: &Host, interface: &str) -> Finding {
    let id = "N3";
    let sev = Severity::Warn;
    let what = format!("whether {interface} has an accelerated-networking VF");
    let driver = match driver_of(host, interface, id, N3_EVIDENCE, &what) {
        Ok(d) => d,
        Err(f) => return f,
    };
    if driver != "hv_netvsc" {
        return Finding::ok(
            id,
            sev,
            N3_EVIDENCE,
            format!("{interface}'s driver is {driver}: N3 is for Azure's hv_netvsc"),
        );
    }
    let lowers = match lower_interfaces(host, interface) {
        Ok(l) => l,
        Err(e) => return Finding::unknown(id, sev, N3_EVIDENCE, format!("cannot tell {what}"), e),
    };
    let mut states = Vec::new();
    let mut unreadable = None;
    for vf in &lowers {
        match host.read_line(&format!("/sys/class/net/{vf}/operstate")) {
            Ok(s) if s == "up" => {
                return Finding::ok(id, sev, N3_EVIDENCE, format!("{interface}'s VF {vf} is up"));
            }
            Ok(s) => states.push(format!("{vf} is {s}")),
            Err(e) => unreadable = Some(e),
        }
    }
    if let Some(e) = unreadable {
        return Finding::unknown(id, sev, N3_EVIDENCE, format!("cannot tell {what}"), e);
    }
    let detail = if states.is_empty() {
        "no accelerated-networking VF".to_owned()
    } else {
        format!("no accelerated-networking VF up ({})", states.join(", "))
    };
    Finding::found(
        id,
        sev,
        N3_EVIDENCE,
        format!(
            "{interface} has {detail}: its traffic takes the synthetic VMBus path. Fix: enable \
             accelerated networking on the NIC with the VM stopped or deallocated (`az vm \
             deallocate`, then `az network nic update --name <NIC> --resource-group <resource \
             group> --accelerated-networking true`, then `az vm start`; node-level: on AKS, a node \
             pool VM size with accelerated networking)"
        ),
    )
}

const N4_EVIDENCE: &str = "ethtool(8) (-G rx); not measured with archived data";

/// N4: the kernel RX ring of `interface` (on Azure its VF) holds at least
/// min(4096, its maximum) entries. For the media driver's socket transport
/// with busy polling.
#[must_use]
pub fn n4_rx_ring<E: Ethtool>(
    host: &Host,
    interface: &str,
    cache: &mut EthtoolCache<E>,
) -> Finding {
    let id = "N4";
    let sev = Severity::Warn;
    let what = format!("{interface}'s RX ring");
    let driver = match driver_of(host, interface, id, N4_EVIDENCE, &what) {
        Ok(d) => d,
        Err(f) => return f,
    };
    let (target, via) = if driver == "hv_netvsc" {
        match lower_interfaces(host, interface) {
            Ok(l) => match l.into_iter().next() {
                Some(vf) => (vf, format!(" ({interface}'s VF)")),
                None => {
                    return Finding::unknown(
                        id,
                        sev,
                        N4_EVIDENCE,
                        format!("cannot tell {what}"),
                        format!(
                            "{interface} has no VF: the synthetic path's ring is not the NIC's"
                        ),
                    );
                }
            },
            Err(e) => {
                return Finding::unknown(id, sev, N4_EVIDENCE, format!("cannot tell {what}"), e);
            }
        }
    } else {
        (interface.to_owned(), String::new())
    };
    let ena = interface_device(host, &target)
        .ok()
        .flatten()
        .and_then(|d| d.driver)
        .is_some_and(|d| d == "ena");
    match cache.rings(host, &target) {
        Ok(Rings {
            rx: Some(rx),
            rx_max: Some(max),
        }) => {
            let want = max.min(4096);
            if rx >= want {
                return Finding::ok(
                    id,
                    sev,
                    N4_EVIDENCE,
                    format!("{target}{via}: RX ring {rx} of at most {max}"),
                );
            }
            let reset = if ena {
                " (on ENA this resets the interface: re-pin its IRQs and turn adaptive-rx off again)"
            } else {
                ""
            };
            Finding::found(
                id,
                sev,
                N4_EVIDENCE,
                format!(
                    "{target}{via}: RX ring {rx} of at most {max}: a burst larger than the ring is \
                     dropped at the NIC, and an Aeron receiver asks for it again with a NAK (not \
                     measured here). Fix: `ethtool \
                     -G {target} rx {want}`{reset}"
                ),
            )
        }
        Ok(_) => Finding::unknown(
            id,
            sev,
            N4_EVIDENCE,
            format!("cannot tell {what}"),
            format!("{target}'s driver does not report its RX ring and maximum"),
        ),
        Err(e) => Finding::unknown(id, sev, N4_EVIDENCE, format!("cannot tell {what}"), e),
    }
}
