//! Backend selection: which kernel interface the `Driver` uses, chosen at
//! run time from configuration.
//!
//! | backend | how it waits | NIC polling | sleeps in the kernel | extra core |
//! |---|---|---|---|---|
//! | `epoll-spin` | `epoll_wait(0)` every poll, then `recvmsg` on ready sockets | no (interrupts) | never | no |
//! | `recv-busy` | non-blocking `recvmsg` on every socket every poll (with `per_queue`: one per NIC queue, then the sockets `epoll_wait(0)` reports ready), `SO_BUSY_POLL` set: an empty read runs one NAPI pass | yes, one pass per socket (or queue) per poll | never | no |
//! | `uring-spin` | `IORING_OP_RECVMSG`, `DEFER_TASKRUN`; enters the kernel only when it flags work | no (interrupts) | never | no |
//! | `uring-sqpoll` | `SQPOLL` kernel thread submits, completes and polls the NIC (NAPI) | yes, on the kernel thread | never (the application thread) | yes |
//! | `epoll-busy` | `epoll_pwait2(wait_us)` with epoll busy poll for `busy_poll_us` | yes, inside the wait | up to `wait_us` | no |
//! | `uring-napi` | `io_uring_enter(GETEVENTS, 1, wait_us)` with NAPI busy poll | yes, inside the wait | up to `wait_us`, and on every idle poll even with `wait_us = 0` | no |
//!
//! Which one: it depends on sockets per loop and whether a second core is
//! free (evidence: `io/benches/BENCHMARK.md`).
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
//! | the loop's CPU time in the kernel matters | `uring-spin`: interrupt-driven, far fewer syscalls than `epoll-spin` (about 800 a second against 4.1 million at 800 msg/s, Azure) |
//! | the thread may nap | `epoll-busy` or `uring-napi`: they poll the NIC only inside a timed wait |
//!
//! - `epoll_wait(0)` never blocks, but it also skips the kernel's NIC
//!   busy-poll step; mio (and so tokio) is this path.
//! - `epoll-spin`, `uring-spin` and mio are not yet measured fairly against
//!   the NIC-polling backends: every AWS comparison ran with ENA's adaptive
//!   interrupt moderation on. Do not choose against them on those numbers.
//! - `uring-sqpoll` spins one kernel thread per ring; rings in one process
//!   can share one, other processes cannot.
//!
//! Configuration strings are `backend[,key=value...]`, for example
//! `uring-sqpoll,sqpoll_cpu=3` or `epoll-busy,busy_poll_us=50,wait_us=100`.

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
    /// `io_uring`, `DEFER_TASKRUN`; enters the kernel only when it flags work.
    UringSpin,
    /// `io_uring` with an `SQPOLL` kernel thread that also polls the NIC.
    UringSqpoll,
    /// `epoll_pwait2` with epoll busy poll; sleeps up to `wait_us`.
    EpollBusy,
    /// `io_uring` waiting in `io_uring_enter` with NAPI busy poll; sleeps.
    UringNapi,
}

impl Backend {
    /// Every backend, never-sleeping ones first.
    pub const ALL: [Self; 6] = [
        Self::EpollSpin,
        Self::RecvBusy,
        Self::UringSpin,
        Self::UringSqpoll,
        Self::EpollBusy,
        Self::UringNapi,
    ];

    /// The configuration name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::EpollSpin => "epoll-spin",
            Self::RecvBusy => "recv-busy",
            Self::UringSpin => "uring-spin",
            Self::UringSqpoll => "uring-sqpoll",
            Self::EpollBusy => "epoll-busy",
            Self::UringNapi => "uring-napi",
        }
    }

    /// Whether `Driver::poll` can sleep in the kernel with this backend.
    #[must_use]
    pub const fn may_sleep(self) -> bool {
        matches!(self, Self::EpollBusy | Self::UringNapi)
    }

    /// Whether the backend polls the NIC instead of waiting for interrupts.
    #[must_use]
    pub const fn polls_nic(self) -> bool {
        matches!(
            self,
            Self::RecvBusy | Self::UringSqpoll | Self::EpollBusy | Self::UringNapi
        )
    }

    /// Whether the backend is built on `io_uring`.
    #[must_use]
    pub const fn is_uring(self) -> bool {
        matches!(self, Self::UringSpin | Self::UringSqpoll | Self::UringNapi)
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
    /// The kernel interface.
    pub backend: Backend,
    /// NIC busy-poll budget in microseconds. `epoll-busy` and `uring-napi` poll
    /// this long before sleeping; `recv-busy` and `uring-sqpoll` only need it
    /// non-zero (they make one pass per call).
    pub busy_poll_us: u32,
    /// Longest kernel wait for `epoll-busy` and `uring-napi`, in microseconds.
    pub wait_us: u32,
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
    /// `Driver::connect` does instead, the handshake on the polling thread
    /// (about 0.7 ms of CPU per TLS handshake, once per connect or
    /// reconnect). `HttpClient`'s pool and `Reconnect` follow it.
    pub inline_connect_fallback: bool,
    /// Preferred busy polling (`SO_PREFER_BUSY_POLL`, or the `io_uring` / epoll
    /// equivalent): while the application keeps polling, the kernel leaves the
    /// NIC queue to it instead of also processing it from interrupts. It only
    /// suppresses interrupts when the device has `napi_defer_hard_irqs` and
    /// `gro_flush_timeout` set (sysfs). Backends that poll the NIC only; the
    /// socket option needs `CAP_NET_ADMIN`.
    pub prefer_busy_poll: bool,
    /// Packets per NIC busy-poll pass (`SO_BUSY_POLL_BUDGET`, or the epoll
    /// budget); `None` keeps the kernel default (8). `recv-busy` and
    /// `epoll-busy` only; raising it needs `CAP_NET_ADMIN`.
    pub busy_poll_budget: Option<u16>,
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
}

impl Config {
    /// Defaults for `backend`.
    #[must_use]
    pub const fn new(backend: Backend) -> Self {
        let (busy_poll_us, wait_us) = match backend {
            Backend::EpollBusy | Backend::UringNapi => (50, 100),
            Backend::RecvBusy | Backend::UringSqpoll => (1, 0),
            Backend::EpollSpin | Backend::UringSpin => (0, 0),
        };
        Self {
            backend,
            busy_poll_us,
            wait_us,
            loop_cpu: None,
            sqpoll_cpu: None,
            sqpoll_idle_ms: 600_000,
            ring_entries: 256,
            rcvbuf: Some(4 << 20),
            rx_timestamps: true,
            connect_timeout_ms: 10_000,
            connect_cpu: None,
            helper_cpus: Vec::new(),
            inline_connect_fallback: false,
            prefer_busy_poll: false,
            busy_poll_budget: None,
            per_queue: false,
            quickack_check: QuickAckCheck::Warn,
            keepalive_idle_s: 10,
            keepalive_interval_s: 5,
            keepalive_count: 3,
            // Idle plus every probe: the same deadline for unacknowledged data.
            tcp_user_timeout_ms: 25_000,
        }
    }

    /// Checks that every setting applies to the backend.
    ///
    /// # Errors
    /// `busy_poll_us` or `tcp_user_timeout_ms` above `i32::MAX`,
    /// `prefer_busy_poll` on a backend that does not poll the NIC,
    /// `busy_poll_budget` on one that does not poll through the socket or
    /// epoll, or keepalive settings outside the kernel's limits.
    pub fn validate(&self) -> Result<()> {
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
        // The kernel takes the busy-poll time as a signed int.
        if i32::try_from(self.busy_poll_us).is_err() {
            return Err(Error::Config(format!(
                "busy_poll_us {} is above {}",
                self.busy_poll_us,
                i32::MAX
            )));
        }
        if self.prefer_busy_poll && !self.backend.polls_nic() {
            return Err(Error::Config(format!(
                "prefer_busy_poll needs a backend that polls the NIC, not {}",
                self.backend
            )));
        }
        if self.per_queue && self.backend != Backend::RecvBusy {
            return Err(Error::Config(format!(
                "per_queue applies to recv-busy, not {}",
                self.backend
            )));
        }
        if self.busy_poll_budget.is_some()
            && !matches!(self.backend, Backend::RecvBusy | Backend::EpollBusy)
        {
            return Err(Error::Config(format!(
                "busy_poll_budget applies to recv-busy and epoll-busy, not {}",
                self.backend
            )));
        }
        Ok(())
    }

    /// Parses `backend[,key=value...]`. Keys: `busy_poll_us`, `wait_us`,
    /// `loop_cpu`, `sqpoll_cpu`, `sqpoll_idle_ms`, `ring_entries`, `connect_timeout_ms`,
    /// `connect_cpu`, `rcvbuf` (0 keeps the kernel default), `rx_timestamps`
    /// `prefer_busy_poll`, `per_queue` and `inline_connect_fallback`
    /// (`true`/`false`), `busy_poll_budget` (0 keeps the kernel default),
    /// `quickack_check` (`off`, `warn` or `require`), `keepalive_idle_s` (0:
    /// off), `keepalive_interval_s`, `keepalive_count`, `tcp_user_timeout_ms`
    /// (0: the kernel default), `helper_cpus` (a kernel CPU list:
    /// `helper_cpus=1,3-4` keeps the parts after its commas that have no
    /// `=`).
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
                "busy_poll_us" => cfg.busy_poll_us = num()?,
                "wait_us" => cfg.wait_us = num()?,
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
                "prefer_busy_poll" => cfg.prefer_busy_poll = flag()?,
                "per_queue" => cfg.per_queue = flag()?,
                "quickack_check" => cfg.quickack_check = v.parse()?,
                "keepalive_idle_s" => cfg.keepalive_idle_s = num()?,
                "keepalive_interval_s" => cfg.keepalive_interval_s = num()?,
                "keepalive_count" => cfg.keepalive_count = num()?,
                "tcp_user_timeout_ms" => cfg.tcp_user_timeout_ms = num()?,
                "busy_poll_budget" => {
                    let b = u16::try_from(num()?)
                        .map_err(|_| Error::Config(format!("{k}: above 65535: {v:?}")))?;
                    cfg.busy_poll_budget = Some(b).filter(|&b| b > 0);
                }
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
