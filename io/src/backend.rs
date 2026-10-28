//! Backend selection: which kernel interface the `Driver` uses, chosen at
//! run time from configuration.
//!
//! Three backends, each measured in a busy-spin loop (evidence:
//! `io/benches/BENCHMARK.md`); none sleeps in the kernel.
//!
//! | backend | how it waits | NIC polling | extra core |
//! |---|---|---|---|
//! | `epoll-spin` | `epoll_wait(0)` every poll, then `recvmsg` on ready sockets | no (interrupts); with `busy_poll_us`, one pass per socket after each receive | no |
//! | `recv-busy` | non-blocking `recvmsg` on every socket every poll (with `per_queue`: one per NIC queue, then the sockets `epoll_wait(0)` reports ready), `SO_BUSY_POLL` set: an empty read runs one NAPI pass | yes, one pass per socket (or queue) per poll | no |
//! | `uring-sqpoll` | `SQPOLL` kernel thread submits, completes and polls the NIC (NAPI) | yes, on the kernel thread | yes |
//!
//! UDP endpoints ([`crate::udp`]) run on `epoll-spin` only: the other two
//! refuse them at `Driver::new` ([`Error::Unsupported`]). Every other
//! endpoint runs on all three.
//!
//! A fourth, `fstack-spin` ([`crate::fstack`]), bypasses the kernel: F-Stack
//! (FreeBSD's TCP/IP stack on DPDK) owns a NIC of its own and the polling
//! thread runs it, one pass per poll. It drives TCP endpoints made on
//! F-Stack (`FrameConn<FstackTcp>`, `WsConn<FstackTcp>`,
//! `HttpConn<FstackTcp>`), needs the `fstack` feature (Linux), and is
//! lab-tested only.
//!
//! A fifth, `smoltcp-spin` ([`crate::smoltcp_dpdk`]), also bypasses the
//! kernel: smoltcp, a TCP/IP stack in this process, over a DPDK port it
//! polls once per poll. It drives TCP endpoints made on it
//! (`FrameConn<SmolTcp>`, `WsConn<SmolTcp>`, `HttpConn<SmolTcp>`) and UDP
//! (`UdpConn<SmolUdp>`), needs the `smoltcp-dpdk` feature (Linux) and a
//! stack started on the polling thread, and is lab-tested only.
//!
//! Count every socket the loop thread polls (feeds, order connections,
//! `HttpClient` pools).
//!
//! | situation | backend |
//! |---|---|
//! | 1–4 sockets, one core | `recv-busy`: it polls the NIC itself; each poll costs one `recvmsg` per socket (about 0.6 µs each on AWS `c7i`) |
//! | 5 or more sockets, a second core to spare | `uring-sqpoll`: its kernel thread polls each NIC queue once per pass |
//! | 5 or more sockets, one physical core (two hyperthreads) | `uring-sqpoll`: its `SQPOLL` thread goes on the loop CPU's sibling by default ([`crate::placement`]) |
//! | 5 or more sockets, a single CPU | `recv-busy` with `per_queue` (`uring-sqpoll` refuses: `SQPOLL` would spin on the loop's CPU) |
//! | the NIC's interrupts on a core of their own (node-level), moderation off | `epoll-spin`: level with `uring-sqpoll` from 4 sockets in the one fair comparison (Graviton) |
//!
//! - `epoll_wait(0)` never blocks, but it also skips the kernel's NIC
//!   busy-poll step; mio (and so tokio) is this path. `busy_poll_us` gives
//!   `epoll-spin` what mio gets from `SO_BUSY_POLL`: the read that empties a
//!   socket polls the NIC once ([`Config::busy_poll_us`]).
//! - `uring-sqpoll` spins one kernel thread per ring; rings in one process
//!   can share one, other processes cannot.
//!
//! Configuration strings are `backend[,key=value...]`, for example
//! `uring-sqpoll,sqpoll_cpu=3` or `recv-busy,per_queue=true`.

use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};
use crate::route::QuickAckCheck;

/// The kernel interface a `Driver` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// `epoll_wait(0)` spin, interrupt-driven.
    EpollSpin,
    /// Non-blocking `recvmsg` spin with `SO_BUSY_POLL`: one NAPI pass per empty read.
    RecvBusy,
    /// `io_uring` with an `SQPOLL` kernel thread that also polls the NIC.
    UringSqpoll,
    /// F-Stack (FreeBSD's TCP/IP stack on DPDK) on a NIC of its own, one
    /// pass per poll on the polling thread ([`crate::fstack`]).
    FstackSpin,
    /// smoltcp, a TCP/IP stack in this process, over a DPDK port polled
    /// once per poll ([`crate::smoltcp_dpdk`]).
    SmoltcpSpin,
}

impl Backend {
    /// Every backend.
    pub const ALL: [Self; 5] = [
        Self::EpollSpin,
        Self::RecvBusy,
        Self::UringSqpoll,
        Self::FstackSpin,
        Self::SmoltcpSpin,
    ];

    /// The backends over the kernel's sockets.
    pub const KERNEL: [Self; 3] = [Self::EpollSpin, Self::RecvBusy, Self::UringSqpoll];

    /// The configuration name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::EpollSpin => "epoll-spin",
            Self::RecvBusy => "recv-busy",
            Self::UringSqpoll => "uring-sqpoll",
            Self::FstackSpin => "fstack-spin",
            Self::SmoltcpSpin => "smoltcp-spin",
        }
    }

    /// Whether the backend polls the NIC instead of waiting for interrupts.
    #[must_use]
    pub const fn polls_nic(self) -> bool {
        matches!(
            self,
            Self::RecvBusy | Self::UringSqpoll | Self::FstackSpin | Self::SmoltcpSpin
        )
    }

    /// Whether the backend's TCP/IP stack is in user space (F-Stack or
    /// smoltcp, each on a DPDK port of its own), not the kernel's: the
    /// kernel's routes, devices and socket options do not apply.
    #[must_use]
    pub const fn bypasses_kernel(self) -> bool {
        matches!(self, Self::FstackSpin | Self::SmoltcpSpin)
    }

    /// The stack whose sockets it drives.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))] // the driver's
    pub(crate) const fn stack(self) -> crate::netstack::StackId {
        match self {
            Self::FstackSpin => crate::netstack::StackId::Fstack,
            Self::SmoltcpSpin => crate::netstack::StackId::Smoltcp,
            _ => crate::netstack::StackId::Kernel,
        }
    }

    /// Whether the backend is built on `io_uring`.
    #[must_use]
    pub const fn is_uring(self) -> bool {
        matches!(self, Self::UringSqpoll)
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Backend {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|b| b.name() == s)
            .ok_or_else(|| Error::Config(format!("unknown backend {s:?}")))
    }
}

/// Driver configuration. [`Config::new`] gives the defaults for a backend.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent switches, each its own key
pub struct Config {
    /// The kernel interface. `recv-busy` and `uring-sqpoll` make one NIC
    /// busy-poll pass per call (`SO_BUSY_POLL` 1 µs, a NAPI busy-poll
    /// timeout of 1 µs); `epoll-spin` sockets get
    /// [`busy_poll_us`](Self::busy_poll_us) (0 by default), `uring-sqpoll`
    /// ones 0.
    pub backend: Backend,
    /// `epoll-spin` only: `SO_BUSY_POLL`, µs, on every socket the driver
    /// owns. `0` (the default): no busy polling, the loop waits for the
    /// NIC's interrupt. Above 0, a read with data is followed by one more
    /// read of that socket on the next poll, and that read, finding the
    /// receive queue empty, makes one NAPI pass (`sk_busy_loop`) before it
    /// returns `EAGAIN`: one NIC poll per socket per receive, after the
    /// replies the receive queued went out, as mio with the same option
    /// (it reads to `WouldBlock`). A datagram socket epoll reports is read
    /// once; its `datagram_burst` runs from that next-poll read. The reads are
    /// non-blocking, so any value above 0 makes the same single pass; the
    /// value matters only to blocking reads. `epoll_wait(0)` itself never
    /// busy-polls (the kernel returns before `ep_busy_loop` on a zero
    /// timeout), so `net.core.busy_poll`, `EPIOCSPARAMS` and
    /// `SO_PREFER_BUSY_POLL` change nothing here. Needs a kernel with
    /// `CONFIG_NET_RX_BUSY_POLL` and a NIC with NAPI ids (none on loopback
    /// or a plain veth: there the option is set and does nothing). 0 to
    /// `i32::MAX`; other backends refuse it.
    pub busy_poll_us: u32,
    /// The polling loop's CPU, for the host checks `Driver::new` runs
    /// ([`crate::checks`]: isolation, SMT siblings, and the NIC interrupts of
    /// each connection's egress device). `None` takes the CPU the thread
    /// calling `Driver::new` is pinned to, when it is pinned to exactly one;
    /// otherwise those checks are skipped with one information line. ergon-io
    /// does not pin the thread itself.
    pub loop_cpu: Option<u32>,
    /// CPU for the `SQPOLL` kernel thread: ideally an isolated core of its own.
    /// `None` uses the first helper CPU ([`crate::placement`]: inside the
    /// process's cpuset, never the polling CPU); `Driver::config` then shows
    /// the one chosen. A CPU outside the cpuset fails `Driver::new`
    /// ([`crate::placement::CpuError::Outside`]).
    pub sqpoll_cpu: Option<u32>,
    /// Idle time after which the `SQPOLL` thread sleeps, in milliseconds. NAPI
    /// passes do not count as work, so keep it long.
    pub sqpoll_idle_ms: u32,
    /// `io_uring` submission queue entries.
    pub ring_entries: u32,
    /// `SO_RCVBUF` in bytes for every socket (`None` keeps the kernel default).
    pub rcvbuf: Option<u32>,
    /// Request kernel software (and, where supported, NIC) receive timestamps.
    pub rx_timestamps: bool,
    /// Stamp every read with ergon-io's own receive time,
    /// `RxMeta::rx_read_ns` ([`crate::RxMeta::rx_read_ns`]): one
    /// `CLOCK_REALTIME` read (vDSO) per read with data. Off (the default):
    /// no clock read, the field stays 0.
    pub rx_read_timestamp: bool,
    /// Deadline for `Driver::connect` (DNS, TCP, TLS, upgrade), in milliseconds.
    pub connect_timeout_ms: u32,
    /// CPU for the `Driver::connect_background` helper thread. `None` runs it
    /// on the helper CPUs ([`crate::placement`]), so the handshake never
    /// competes with the polling thread for its core. One outside the
    /// process's cpuset, or the polling CPU, makes `connect_background` fail
    /// ([`crate::Error::Cpu`]).
    pub connect_cpu: Option<u32>,
    /// CPUs for ergon-io's helper threads (the connect helper, the warning
    /// thread, `SQPOLL`): CPUs a node-level mechanism adds to the pod's
    /// cpuset for shared work. Empty: `ERGON_HELPER_CPUS`, else chosen at
    /// `Driver::new` ([`crate::placement`]). Each must be one of the
    /// process's CPUs, or `Driver::new` fails; the polling CPU is left out.
    pub helper_cpus: Vec<u32>,
    /// `Driver::connect_background` when the process may use the polling CPU
    /// only (a pod with one CPU, [`crate::placement`]): `false` (the
    /// default) returns [`crate::Error::Cpu`]; `true` connects as
    /// `Driver::connect` does instead, the handshake on the polling thread:
    /// one long poll per connect or reconnect (measured worst poll per
    /// connect: `io/benches/BENCHMARK.md`, "Connection set-up").
    /// `HttpClient`'s pool and `Reconnect` follow it.
    pub inline_connect_fallback: bool,
    /// `recv-busy` with many sockets: read one socket per NIC queue every
    /// poll (which polls each queue once), then only the sockets
    /// `epoll_wait(0)` reports ready, instead of reading every socket every
    /// poll. Costs one `epoll_wait` per poll; pays off once sockets outnumber
    /// the queues they share.
    pub per_queue: bool,
    /// The quick-ACK route check, run once per connection when the driver
    /// takes it: whether the route the kernel sends it on must disable
    /// delayed ACKs (`quickack 1`). [`QuickAckCheck::Warn`] (the default)
    /// warns once per route without it, [`QuickAckCheck::Require`] refuses
    /// such connections, [`QuickAckCheck::Off`] skips the lookup. See
    /// [`crate::route`].
    pub quickack_check: QuickAckCheck,
    /// TCP keepalive (`SO_KEEPALIVE`, `TCP_KEEPIDLE`): seconds without
    /// traffic before the first probe; `0` leaves keepalive as the socket
    /// has it (off for sockets the driver creates). At most 32767. Each
    /// socket then has a kernel timer that fires about this often.
    pub keepalive_idle_s: u32,
    /// Seconds between probes (`TCP_KEEPINTVL`), 1 to 32767.
    pub keepalive_interval_s: u32,
    /// Unanswered probes before the connection is dropped
    /// (`TCP_KEEPCNT`), 1 to 127.
    pub keepalive_count: u32,
    /// `TCP_USER_TIMEOUT`, ms: sent data unacknowledged this long drops the
    /// connection (and it bounds keepalive). `0` leaves the kernel default
    /// (about 15 minutes of retransmissions).
    pub tcp_user_timeout_ms: u32,
    /// `TCP_RTO_MIN_US` (Linux 6.15+), µs: the floor of the retransmission
    /// timeout on each socket, so a lost segment that needs a timeout is
    /// sent again after `max(floor, srtt + 4 × rttvar)` instead of at least
    /// 200 ms. Only this side's retransmissions; a route `rto_min` that is
    /// locked still wins. `None` (the default) leaves the kernel's. 2 000 to
    /// 200 000, and whole ticks of the running kernel (`Driver::new` sets it
    /// on a probe socket and refuses a value the kernel would round). Too
    /// low a floor sends spurious retransmissions.
    pub tcp_rto_min_us: Option<u32>,
    /// `TCP_DELACK_MAX_US` (Linux 6.15+), µs: the longest this socket delays
    /// an ACK (the kernel default is 200 ms, and never above the RTO floor
    /// less one tick). `None` (the default) leaves the kernel's. Same range
    /// and checks as `tcp_rto_min_us`.
    pub tcp_delack_max_us: Option<u32>,
    /// Passive `TCP_INFO` sampling, ms between two samples of a connection
    /// (`Driver::tcp_info`); `0` (the default) is off. The polling thread
    /// reads one connection per deadline check (every 250 µs) while a round
    /// is due: one `getsockopt` per poll at most, which takes the socket's
    /// lock briefly.
    pub tcp_info_interval_ms: u32,
    /// `epoll_wait` batch: the most ready sockets one poll learns of
    /// (`epoll-spin`, and `recv-busy` with `per_queue`). Sockets past it are
    /// reported again on the next poll (level-triggered). 1 to 65 536,
    /// default 64.
    pub epoll_events: u32,
    /// UDP ([`crate::udp`]): the most datagrams one poll reads from one ready
    /// socket before it moves on, so one busy socket cannot hold up the
    /// others. Each costs one `recvmsg`; the last read of a drained socket
    /// returns `EAGAIN`. 1 to 1 024, default 16.
    pub datagram_burst: u32,
    /// `fstack-spin`: F-Stack's interface and EAL (the `fstack_*` keys,
    /// [`crate::fstack::FstackConf`]). Other backends refuse any setting
    /// here.
    pub fstack: crate::fstack::FstackConf,
}

/// What [`Config::epoll_events`] may be: `epoll_wait` takes a positive
/// `int`; past 65 536 the batch is only memory.
const EPOLL_EVENTS: std::ops::RangeInclusive<u32> = 1..=65_536;

/// What [`Config::datagram_burst`] may be.
const DATAGRAM_BURST: std::ops::RangeInclusive<u32> = 1..=1_024;

/// The kernel's bounds for `TCP_RTO_MIN_US` and `TCP_DELACK_MAX_US`, µs: two
/// ticks at the finest `CONFIG_HZ` (1000; `TCP_TIMEOUT_MIN_US`), and `HZ / 5`
/// ticks (200 ms on every `CONFIG_HZ`), `include/net/tcp.h`.
const TCP_TIMER_US: std::ops::RangeInclusive<u32> = 2_000..=200_000;

impl Config {
    /// Defaults for `backend`. `fstack-spin` starts with no receive
    /// timestamps (F-Stack has none), the quick-ACK route check off (F-Stack
    /// acknowledges at once: `FstackConf::delayed_ack`) and no
    /// `TCP_USER_TIMEOUT` (FreeBSD has none). `smoltcp-spin` starts with the
    /// quick-ACK route check off (it acknowledges every segment at once).
    #[must_use]
    pub const fn new(backend: Backend) -> Self {
        let user_space = matches!(backend, Backend::FstackSpin);
        Self {
            backend,
            loop_cpu: None,
            sqpoll_cpu: None,
            busy_poll_us: 0,
            sqpoll_idle_ms: 600_000,
            ring_entries: 256,
            rcvbuf: Some(4 << 20),
            rx_timestamps: !user_space,
            rx_read_timestamp: false,
            connect_timeout_ms: 10_000,
            connect_cpu: None,
            helper_cpus: Vec::new(),
            inline_connect_fallback: false,
            per_queue: false,
            quickack_check: if backend.bypasses_kernel() {
                QuickAckCheck::Off
            } else {
                QuickAckCheck::Warn
            },
            keepalive_idle_s: 10,
            keepalive_interval_s: 5,
            keepalive_count: 3,
            // Idle plus every probe: the same deadline for unacknowledged data.
            tcp_user_timeout_ms: if user_space { 0 } else { 25_000 },
            tcp_rto_min_us: None,
            tcp_delack_max_us: None,
            tcp_info_interval_ms: 0,
            epoll_events: 64,
            datagram_burst: 16,
            fstack: crate::fstack::FstackConf::new(),
        }
    }

    /// Checks that every setting applies to the backend.
    ///
    /// # Errors
    /// `tcp_user_timeout_ms` or `busy_poll_us` above `i32::MAX`, `per_queue`
    /// on a backend other than `recv-busy`, `busy_poll_us` on one other than
    /// `epoll-spin`, keepalive settings outside the kernel's
    /// limits, or
    /// `tcp_rto_min_us` / `tcp_delack_max_us` outside 2 000 to 200 000 µs
    /// (what no kernel accepts; the running kernel's tick is checked at
    /// `Driver::new`).
    pub fn validate(&self) -> Result<()> {
        for (key, v) in [
            ("tcp_rto_min_us", self.tcp_rto_min_us),
            ("tcp_delack_max_us", self.tcp_delack_max_us),
        ] {
            if let Some(us) = v.filter(|us| !TCP_TIMER_US.contains(us)) {
                return Err(Error::Config(format!(
                    "{key} {us} µs: the kernel takes {} to {} µs (two ticks at CONFIG_HZ=1000 \
                     up to HZ/5)",
                    TCP_TIMER_US.start(),
                    TCP_TIMER_US.end()
                )));
            }
        }
        if i32::try_from(self.tcp_user_timeout_ms).is_err() {
            return Err(Error::Config(format!(
                "tcp_user_timeout_ms {} is above {}",
                self.tcp_user_timeout_ms,
                i32::MAX
            )));
        }
        // The kernel's limits (MAX_TCP_KEEPIDLE, MAX_TCP_KEEPINTVL,
        // MAX_TCP_KEEPCNT); probe settings matter only with keepalive on.
        if self.keepalive_idle_s > 32_767
            || (self.keepalive_idle_s > 0
                && (!(1..=32_767).contains(&self.keepalive_interval_s)
                    || !(1..=127).contains(&self.keepalive_count)))
        {
            return Err(Error::Config(format!(
                "keepalive: idle {} s (0 to 32767), interval {} s (1 to 32767), count {} (1 to 127)",
                self.keepalive_idle_s, self.keepalive_interval_s, self.keepalive_count
            )));
        }
        for (key, v, range) in [
            ("epoll_events", self.epoll_events, &EPOLL_EVENTS),
            ("datagram_burst", self.datagram_burst, &DATAGRAM_BURST),
        ] {
            if !range.contains(&v) {
                return Err(Error::Config(format!(
                    "{key} {v}: {} to {}",
                    range.start(),
                    range.end()
                )));
            }
        }
        if self.busy_poll_us > 0 && self.backend != Backend::EpollSpin {
            return Err(Error::Config(format!(
                "busy_poll_us applies to epoll-spin, not {} (it sets its own)",
                self.backend
            )));
        }
        if i32::try_from(self.busy_poll_us).is_err() {
            return Err(Error::Config(format!(
                "busy_poll_us {}: 0 to {}",
                self.busy_poll_us,
                i32::MAX
            )));
        }
        if self.per_queue && self.backend != Backend::RecvBusy {
            return Err(Error::Config(format!(
                "per_queue applies to recv-busy, not {}",
                self.backend
            )));
        }
        if self.backend == Backend::FstackSpin {
            self.validate_user_space()
        } else if !self.fstack.is_default() {
            Err(Error::Config(format!(
                "the fstack_* keys apply to fstack-spin, not {}",
                self.backend
            )))
        } else if self.backend == Backend::SmoltcpSpin {
            self.validate_smoltcp()
        } else {
            Ok(())
        }
    }

    /// `fstack-spin`: the kernel-only settings are refused, each with what
    /// F-Stack does instead.
    fn validate_user_space(&self) -> Result<()> {
        let refuse = |key: &str, why: &str| {
            Err(Error::Config(format!(
                "{key} does not apply to {}: {why}",
                self.backend
            )))
        };
        if self.tcp_user_timeout_ms > 0 {
            return refuse("tcp_user_timeout_ms", "FreeBSD has no TCP_USER_TIMEOUT");
        }
        if self.tcp_rto_min_us.is_some() || self.tcp_delack_max_us.is_some() {
            return refuse(
                "tcp_rto_min_us / tcp_delack_max_us",
                "FreeBSD has no per-socket timers (fstack_sysctl sets net.inet.tcp.rexmit_min)",
            );
        }
        if self.quickack_check != QuickAckCheck::Off {
            return refuse(
                "quickack_check",
                "it reads kernel routes; F-Stack acknowledges at once (fstack_delayed_ack=false)",
            );
        }
        if self.rx_timestamps {
            return refuse("rx_timestamps", "F-Stack has no receive timestamps");
        }
        Ok(())
    }

    /// `smoltcp-spin`: what its stack cannot do per socket is refused.
    fn validate_smoltcp(&self) -> Result<()> {
        if self.tcp_rto_min_us.is_some() {
            return Err(Error::Config(
                "tcp_rto_min_us is the kernel's; smoltcp-spin takes the RTO floor from \
                 smoltcp_dpdk::StackConfig::rto_min_us"
                    .into(),
            ));
        }
        if self.quickack_check == QuickAckCheck::Require {
            return Err(Error::Config(
                "quickack_check=require checks kernel routes; smoltcp-spin acknowledges every \
                 segment at once and has no kernel route (use off or warn)"
                    .into(),
            ));
        }
        // smoltcp's timeout counts from the last segment heard, idle or not:
        // only keepalive probes, sent more often, keep a quiet connection
        // alive under it.
        if self.tcp_user_timeout_ms > 0
            && (self.keepalive_idle_s == 0
                || u64::from(self.keepalive_idle_s) * 1000 >= u64::from(self.tcp_user_timeout_ms))
        {
            return Err(Error::Config(format!(
                "smoltcp-spin aborts a connection that hears nothing for tcp_user_timeout_ms \
                 ({} ms), idle or not: keepalive_idle_s ({} s) must be on and shorter",
                self.tcp_user_timeout_ms, self.keepalive_idle_s
            )));
        }
        Ok(())
    }

    /// Parses `backend[,key=value...]`. Keys: `loop_cpu`, `sqpoll_cpu`,
    /// `sqpoll_idle_ms`, `ring_entries`, `connect_timeout_ms`, `connect_cpu`,
    /// `rcvbuf` (0 keeps the kernel default), `rx_timestamps`,
    /// `rx_read_timestamp`, `per_queue` and `inline_connect_fallback`
    /// (`true`/`false`), `quickack_check` (`off`, `warn` or `require`),
    /// `keepalive_idle_s` (0: off), `keepalive_interval_s`,
    /// `keepalive_count`, `tcp_user_timeout_ms`
    /// (0: the kernel default), `tcp_rto_min_us` and `tcp_delack_max_us` (0:
    /// the kernel's), `tcp_info_interval_ms` (0: off), `epoll_events`,
    /// `datagram_burst`, `busy_poll_us` (`epoll-spin` only), `helper_cpus` (a
    /// kernel CPU list:
    /// `helper_cpus=1,3-4` keeps the parts after its commas that have no
    /// `=`), and `fstack-spin`'s `fstack_*` keys ([`crate::fstack::FstackConf`]).
    ///
    /// # Errors
    /// Unknown backend or key, a malformed value, or a setting the backend
    /// cannot use ([`Config::validate`]).
    pub fn parse(s: &str) -> Result<Self> {
        let mut parts = s.split(',').map(str::trim);
        let mut cfg = Self::new(parts.next().unwrap_or_default().parse()?);
        let cpu_list = |k: &str, v: &str| -> Result<Vec<u32>> {
            ergon_tune::CpuSet::parse_list(v)
                .map(|s| s.iter().collect())
                .map_err(|e| Error::Config(format!("{k}: {e}: {v:?}")))
        };
        // After `helper_cpus=`, parts without `=` continue its list.
        let mut in_list = false;
        for kv in parts.filter(|p| !p.is_empty()) {
            if in_list && !kv.contains('=') {
                cfg.helper_cpus.extend(cpu_list("helper_cpus", kv)?);
                continue;
            }
            in_list = false;
            let (k, v) = kv
                .split_once('=')
                .ok_or_else(|| Error::Config(format!("expected key=value, got {kv:?}")))?;
            let num = || -> Result<u32> {
                v.parse()
                    .map_err(|_| Error::Config(format!("{k}: bad number {v:?}")))
            };
            let flag = || -> Result<bool> {
                v.parse()
                    .map_err(|_| Error::Config(format!("{k}: bad bool {v:?}")))
            };
            match k {
                "loop_cpu" => cfg.loop_cpu = Some(num()?),
                "sqpoll_cpu" => cfg.sqpoll_cpu = Some(num()?),
                "sqpoll_idle_ms" => cfg.sqpoll_idle_ms = num()?,
                "ring_entries" => cfg.ring_entries = num()?,
                "connect_timeout_ms" => cfg.connect_timeout_ms = num()?,
                "connect_cpu" => cfg.connect_cpu = Some(num()?),
                "helper_cpus" => {
                    cfg.helper_cpus = cpu_list(k, v)?;
                    in_list = true;
                }
                "inline_connect_fallback" => cfg.inline_connect_fallback = flag()?,
                "rcvbuf" => cfg.rcvbuf = Some(num()?).filter(|&b| b > 0),
                "rx_timestamps" => cfg.rx_timestamps = flag()?,
                "rx_read_timestamp" => cfg.rx_read_timestamp = flag()?,
                "per_queue" => cfg.per_queue = flag()?,
                "quickack_check" => cfg.quickack_check = v.parse()?,
                "keepalive_idle_s" => cfg.keepalive_idle_s = num()?,
                "keepalive_interval_s" => cfg.keepalive_interval_s = num()?,
                "keepalive_count" => cfg.keepalive_count = num()?,
                "tcp_user_timeout_ms" => cfg.tcp_user_timeout_ms = num()?,
                "tcp_rto_min_us" => cfg.tcp_rto_min_us = Some(num()?).filter(|&us| us > 0),
                "tcp_delack_max_us" => cfg.tcp_delack_max_us = Some(num()?).filter(|&us| us > 0),
                "tcp_info_interval_ms" => cfg.tcp_info_interval_ms = num()?,
                "epoll_events" => cfg.epoll_events = num()?,
                "datagram_burst" => cfg.datagram_burst = num()?,
                "busy_poll_us" => cfg.busy_poll_us = num()?,
                _ if cfg.fstack.set(k, v)? => {}
                _ => return Err(Error::Config(format!("unknown key {k:?}"))),
            }
        }
        cfg.validate()?;
        Ok(cfg)
    }
}

impl FromStr for Config {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}
