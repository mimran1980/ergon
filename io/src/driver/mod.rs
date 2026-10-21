//! The driver: owns a set of connections and moves bytes between them and the
//! kernel with the configured [`Backend`]. Everything above the socket (TLS,
//! framing, the application) is the same whichever backend is chosen.
//!
//! [`Driver::poll`] does one non-blocking iteration and returns, so the caller's
//! own loop stays in control:
//!
//! ```rust,no_run
//! # use ergon_io::{Config, Driver, ConnId, Error, Handler, WsConn, WsHandler, RxMeta};
//! # struct App;
//! # impl Handler for App {}
//! # impl WsHandler for App {
//! #     fn on_message(&mut self, _: ConnId, _: u8, _: &[u8], _: &RxMeta, _: u16) {}
//! # }
//! # fn conn() -> WsConn { unimplemented!() }
//! # fn main() -> ergon_io::Result<()> {
//! let mut driver: Driver<WsConn> = Driver::new(Config::parse("recv-busy")?)?;
//! let id = driver.add(conn())?;
//! let mut app = App;
//! loop {
//!     driver.poll(&mut app)?; // receive, decode, deliver, flush
//!     // ... the rest of the duty cycle; sends queued here go out on the next
//!     // poll, or call driver.flush() to send now.
//! #   break;
//! }
//! # Ok(()) }
//! ```
//!
//! A connection that fails (protocol error, reset, peer close) is closed and
//! reported through [`Handler::on_closed`]; the others keep running. `poll`
//! itself returns an error only when the backend fails.

use std::net::{IpAddr, SocketAddr};
use std::os::fd::RawFd;
use std::sync::{Arc, OnceLock};

use crate::backend::{Backend, Config};
use crate::checks::{self, DevContext, HostReport};
use crate::clock::{monotonic_ns, rdtsc, ticks_per_us};
use crate::dns::{DnsConfig, Lookup, Resolver};
use crate::endpoint::sealed::RouteSlot;
use crate::endpoint::{ConnId, ConnMut, Dial, DriverStats, Endpoint, Handler, Receive};
use crate::error::{Error, Result};
use crate::placement::{self, CpuError};
use crate::route::{self, Flow, QuickAckCheck, QuickAckError, RouteCheck};
use crate::{RxMeta, sys};

use conn_checks::{ConnJob, RouteIn};

mod background;
mod conn_checks;
mod epoll;
#[cfg(test)]
mod placement_tests;
#[cfg(feature = "uring")]
mod uring;

/// The connection slots: endpoints plus a generation per slot.
struct Conns<E> {
    slots: Vec<Option<E>>,
    gens: Vec<u32>,
    /// Closed but still referenced by in-flight `io_uring` operations: hidden from
    /// the API, not reusable, dropped once the kernel lets go.
    closing: Vec<bool>,
    /// Reserved for a connection still being set up (no endpoint yet).
    reserved: Vec<bool>,
    /// Closed endpoints the kernel no longer references, waiting to be freed
    /// (on the helper thread when there is one: `Driver::free_closed`).
    closed: Vec<E>,
}

impl<E> Conns<E> {
    const fn new() -> Self {
        Self {
            slots: Vec::new(),
            gens: Vec::new(),
            closing: Vec::new(),
            reserved: Vec::new(),
            closed: Vec::new(),
        }
    }

    const fn len_slots(&self) -> usize {
        self.slots.len()
    }

    /// The current id of slot `i` (valid whether or not it is occupied).
    fn id(&self, i: usize) -> ConnId {
        ConnId::new(
            u32::try_from(i).unwrap_or(u32::MAX),
            self.gens.get(i).copied().unwrap_or(0),
        )
    }

    fn at(&mut self, i: usize) -> Option<&mut E> {
        self.slots.get_mut(i).and_then(Option::as_mut)
    }

    fn lookup(&self, id: ConnId) -> Option<usize> {
        let i = id.index();
        (self.gens.get(i) == Some(&id.generation())
            && self.closing.get(i) == Some(&false)
            && self.slots.get(i).is_some_and(Option::is_some))
        .then_some(i)
    }

    fn is_open(&self, i: usize) -> bool {
        self.closing.get(i) == Some(&false) && self.slots.get(i).is_some_and(Option::is_some)
    }

    /// Reserves a free slot.
    fn reserve(&mut self) -> usize {
        let i = (0..self.slots.len())
            .find(|&i| self.slots[i].is_none() && !self.closing[i] && !self.reserved[i])
            .unwrap_or_else(|| {
                self.slots.push(None);
                // Generation 1 first, so `ConnId::default()` never matches.
                self.gens.push(1);
                self.closing.push(false);
                self.reserved.push(false);
                self.slots.len() - 1
            });
        self.reserved[i] = true;
        i
    }

    /// Fills a reserved slot.
    fn install(&mut self, i: usize, e: E) {
        self.slots[i] = Some(e);
        self.reserved[i] = false;
    }

    /// Frees a reserved slot without filling it, invalidating its id.
    fn release(&mut self, i: usize) {
        if self.reserved.get(i) == Some(&true) {
            self.reserved[i] = false;
            self.gens[i] = self.gens[i].wrapping_add(1);
        }
    }

    /// Slot of a connection still being set up.
    fn lookup_reserved(&self, id: ConnId) -> Option<usize> {
        let i = id.index();
        (self.gens.get(i) == Some(&id.generation()) && self.reserved.get(i) == Some(&true))
            .then_some(i)
    }

    /// Invalidates slot `i`'s id but keeps the endpoint alive until
    /// [`Conns::finish_close`].
    #[cfg_attr(not(feature = "uring"), allow(dead_code))]
    fn begin_close(&mut self, i: usize) {
        if self.is_open(i) {
            self.gens[i] = self.gens[i].wrapping_add(1);
            self.closing[i] = true;
        }
    }

    fn is_closing(&self, i: usize) -> bool {
        self.closing.get(i) == Some(&true)
    }

    /// Retires a closing endpoint (freed later, see `closed`).
    #[cfg_attr(not(feature = "uring"), allow(dead_code))]
    fn finish_close(&mut self, i: usize) {
        if self.is_closing(i) {
            if let Some(e) = self.slots[i].take() {
                self.closed.push(e);
            }
            self.closing[i] = false;
        }
    }

    /// Never frees the endpoints again (last resort when the kernel may still
    /// write into their buffers).
    #[cfg_attr(not(feature = "uring"), allow(dead_code))]
    fn leak(&mut self) {
        std::mem::forget(std::mem::take(&mut self.slots));
    }

    /// Removes slot `i` at once, invalidating its id; the endpoint is freed
    /// later (see `closed`).
    fn remove(&mut self, i: usize) {
        if self.is_closing(i) {
            return;
        }
        if let Some(e) = self.slots.get_mut(i).and_then(Option::take) {
            self.gens[i] = self.gens[i].wrapping_add(1);
            self.closed.push(e);
        }
    }
}

// One per driver, so the size does not matter; boxing would add a pointer
// chase to every poll.
#[allow(clippy::large_enum_variant)]
enum Imp {
    Epoll(epoll::Epoll),
    #[cfg(feature = "uring")]
    Uring(uring::Uring),
}

/// How often deadlines and timers are checked, in microseconds (only for
/// endpoints with deadlines).
const EXPIRY_CHECK_US: u32 = 250;

/// Polls between two counter reads for that check on backends that never
/// sleep: a counter read costs more than a whole idle `uring-spin` poll, so
/// most polls only count down.
const EXPIRY_POLLS: u32 = 64;

/// How often connections being set up are advanced, in microseconds.
const SETUP_STEP_US: u32 = 20;

/// Per-socket options every backend needs, and what the connection checks
/// need from the driver.
#[derive(Clone)]
struct SockOpts {
    timestamps: bool,
    rcvbuf: Option<u32>,
    busy_poll_us: u32,
    /// `recv-busy` only: preferred busy polling and the packet budget (epoll
    /// and `io_uring` take theirs on the instance).
    prefer_busy_poll: bool,
    busy_poll_budget: Option<u16>,
    quickack: QuickAckCheck,
    backend: Backend,
    /// TCP keepalive: idle, interval and count (idle 0: left alone), and
    /// `TCP_USER_TIMEOUT` (0: left alone).
    keepalive: (u32, u32, u32),
    user_timeout_ms: u32,
    /// The egress-device checks' context (the driver's hot CPUs).
    ctx: Arc<DevContext>,
}

/// The route check's cost on the thread that set the connection up.
#[derive(Clone, Copy, Default)]
struct RouteCost {
    lookups: u64,
    ticks: u64,
}

impl SockOpts {
    fn of(cfg: &Config, ctx: &Arc<DevContext>) -> Self {
        Self {
            timestamps: cfg.rx_timestamps,
            rcvbuf: cfg.rcvbuf,
            // Only the backends that poll through the socket set SO_BUSY_POLL;
            // the others set 0 so a sysctl default cannot leak in.
            busy_poll_us: match cfg.backend {
                Backend::EpollBusy | Backend::RecvBusy => cfg.busy_poll_us.max(1),
                _ => 0,
            },
            prefer_busy_poll: cfg.backend == Backend::RecvBusy && cfg.prefer_busy_poll,
            busy_poll_budget: cfg
                .busy_poll_budget
                .filter(|_| cfg.backend == Backend::RecvBusy),
            quickack: cfg.quickack_check,
            backend: cfg.backend,
            keepalive: (
                cfg.keepalive_idle_s,
                cfg.keepalive_interval_s,
                cfg.keepalive_count,
            ),
            user_timeout_ms: cfg.tcp_user_timeout_ms,
            ctx: Arc::clone(ctx),
        }
    }

    fn apply(&self, fd: RawFd) -> Result<()> {
        if self.timestamps {
            sys::enable_rx_timestamps(fd)?;
        }
        if let Some(b) = self.rcvbuf {
            sys::set_rcvbuf(fd, b)?;
        }
        sys::set_busy_poll(fd, self.busy_poll_us)?;
        let (idle, interval, count) = self.keepalive;
        if idle > 0 {
            sys::set_tcp_keepalive(fd, idle, interval, count)?;
        }
        if self.user_timeout_ms > 0 {
            sys::set_tcp_user_timeout(fd, self.user_timeout_ms)?;
        }
        let privileged = |what: &str, e: std::io::Error| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::Config(format!("{what} needs CAP_NET_ADMIN"))
            } else {
                Error::Io(e)
            }
        };
        if self.prefer_busy_poll {
            sys::set_prefer_busy_poll(fd, true)
                .map_err(|e| privileged("prefer_busy_poll (SO_PREFER_BUSY_POLL)", e))?;
        }
        if let Some(b) = self.busy_poll_budget {
            sys::set_busy_poll_budget(fd, b)
                .map_err(|e| privileged("busy_poll_budget (SO_BUSY_POLL_BUDGET)", e))?;
        }
        Ok(())
    }

    /// Socket options, then the connection's checks (see
    /// [`crate::checks`]), once per connection, before it is registered, on
    /// whichever thread sets it up (`on_loop`: the polling thread).
    ///
    /// `added`: a connection handed to `Driver::add`, whose socket options
    /// are unknown: `TCP_NODELAY` is set and `TCP_CORK` reported (the
    /// driver's own connects set `TCP_NODELAY` when they create the socket).
    ///
    /// The route: decided already (a dial gated under `require`), or looked
    /// up here (`require`; `warn` off the polling thread), or under `warn` on
    /// the polling thread queued to the warning thread, which fills in the
    /// connection's [`RouteCheck`] later. Either way the warning thread then
    /// checks the NAPI id and the egress device.
    fn prepare<E: Endpoint>(
        &self,
        ep: &mut E,
        id: ConnId,
        added: bool,
        on_loop: bool,
    ) -> Result<RouteCost> {
        let fd = ep.fd();
        self.apply(fd)?;
        if added && sys::tcp_nodelay_and_cork(fd)? == Some(true) {
            crate::warn::emit(checks::cork_line(id));
        }
        if self.quickack == QuickAckCheck::Off {
            return Ok(RouteCost::default());
        }
        let napi = if self.backend.polls_nic() {
            sys::incoming_napi_id(fd).ok()
        } else {
            None
        };
        let t0 = rdtsc();
        let mut lookups = 0;
        let route = if ep.route_state().decided() {
            RouteIn::Known(ep.route_state().get().clone())
        } else if self.quickack == QuickAckCheck::Warn && on_loop {
            let cell = Arc::new(OnceLock::new());
            ep.set_route(RouteSlot::Pending(Arc::clone(&cell)));
            RouteIn::Lookup(Flow::of_socket(fd), cell)
        } else {
            let found =
                route::check_socket(self.quickack, fd).map_err(|e| Error::QuickAck(Box::new(e)))?;
            lookups = 1;
            ep.set_route(RouteSlot::Ready(found.clone()));
            RouteIn::Known(found)
        };
        // A route known to lead to this host has no NIC to check (I4 and
        // I-dev skip local routes): nothing to queue.
        let local =
            matches!(&route, RouteIn::Known(c) if c.route().is_none_or(route::Route::is_local));
        if !local {
            let job = ConnJob {
                conn: id,
                backend: self.backend,
                ctx: Arc::clone(&self.ctx),
                napi,
                route,
            };
            crate::warn::run(move |st| job.run(st));
        }
        Ok(RouteCost {
            lookups,
            ticks: rdtsc().wrapping_sub(t0),
        })
    }
}

/// The route check of a dial gated under `require` (see `advance_one`):
/// `route::check_socket`, which tests replace with a stand-in.
fn require_route(fd: RawFd) -> std::result::Result<RouteCheck, QuickAckError> {
    route::check_socket(QuickAckCheck::Require, fd)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn ticks_ns(ticks: u64, tpu: f64) -> u64 {
    (ticks as f64 / tpu * 1000.0) as u64
}

/// A connection being set up.
struct Pending<E> {
    slot: usize,
    dial: Box<dyn Dial<Conn = E>>,
    started: bool,
    deadline_ns: i64,
}

/// Owns connections of type `E` and drives them with the configured backend.
///
/// Dropping the driver closes every connection; with `io_uring` it first cancels
/// in-flight operations and waits until the kernel has released every buffer.
pub struct Driver<E> {
    cfg: Config,
    imp: Imp,
    conns: Conns<E>,
    stats: DriverStats,
    read_seq: u64,
    /// An added endpoint holds input that arrived during its handshake; the next
    /// `poll` delivers it before reading the socket.
    buffered: bool,
    pending: Vec<Pending<E>>,
    /// Connections `flush` closed (it has no handler): reported by the next
    /// `poll`.
    unreported: Vec<(ConnId, Error)>,
    resolver: Resolver,
    ticks_per_us: f64,
    /// Counter value at which deadlines are next checked (`E::DEADLINES`).
    next_expiry: u64,
    expiry_ticks: u64,
    /// Polls until the counter is read for that check, and the reset value.
    expiry_countdown: u32,
    expiry_polls: u32,
    /// Counter value at which connection set-up is next advanced.
    next_setup: u64,
    setup_ticks: u64,
    /// The background connect thread, started by the first `connect_background`.
    /// It also frees closed connections.
    background: Option<background::Background<E>>,
    /// Resolver settings for the background thread's own resolver.
    dns: DnsConfig,
    /// What `Driver::new` found about the host.
    report: HostReport,
    /// The egress-device checks' context, shared with queued checks.
    ctx: Arc<DevContext>,
    /// A socket with receive timestamps enabled for the driver's lifetime. The
    /// first timestamping socket in a process enables a kernel static key (code
    /// patched on every CPU) and the last one to close disables it again; this
    /// one keeps it enabled, so connections opened later never flip it.
    _timestamp_keeper: Option<socket2::Socket>,
}

impl<E: Endpoint> Driver<E> {
    /// A driver with no connections.
    ///
    /// Also chooses the CPUs of ergon-io's helper threads
    /// ([`crate::placement`]), reported in [`Driver::host_report`] and one
    /// start-up line.
    ///
    /// # Errors
    /// The backend is unavailable (`io_uring` without the `uring` feature, or a
    /// kernel without the needed features) or fails to initialise.
    /// [`Error::Cpu`]: `Config::helper_cpus` or `ERGON_HELPER_CPUS` names a
    /// CPU the process may not use (or only the polling CPU); `uring-sqpoll`
    /// with `sqpoll_cpu` outside the process's cpuset, or without
    /// `sqpoll_cpu` in a process that may use the polling CPU only (a pod
    /// with one CPU: `SQPOLL` would spin there).
    /// [`Error::HostCheck`]: `ERGON_HOST_CHECKS` is set to something other
    /// than `off`, `on` or `require`, or it is `require` and a host check
    /// warned (after its lines were printed).
    pub fn new(cfg: Config) -> Result<Self> {
        Self::with_dns(cfg, DnsConfig::system())
    }

    /// Like [`Driver::new`] with explicit resolver settings (`Driver::new` reads
    /// `/etc/resolv.conf` and `/etc/hosts`).
    ///
    /// # Errors
    /// As [`Driver::new`].
    pub fn with_dns(cfg: Config, dns: DnsConfig) -> Result<Self> {
        Self::on_host(cfg, dns, &ergon_tune::Host::system())
    }

    /// [`Driver::with_dns`] with the host checks and the helper CPUs read
    /// from `host` (a fixture tree in tests).
    pub(crate) fn on_host(
        mut cfg: Config,
        dns: DnsConfig,
        host: &ergon_tune::Host,
    ) -> Result<Self> {
        cfg.validate()?;
        // The one switch over every host-check warning (`ERGON_HOST_CHECKS`).
        let policy = ergon_tune::HostCheckPolicy::from_env()?;
        // I-host and I3 (see `crate::checks`), and where the helper threads
        // go (`crate::placement`), on the thread creating the driver.
        let report = checks::host_report(host, &cfg);
        let helpers = report.helpers.clone()?;
        if cfg.backend == Backend::UringSqpoll {
            // Before the ring exists: `io_uring_setup` refuses a CPU outside
            // the cpuset (EINVAL), and without one of its own SQPOLL would
            // spin on the polling CPU.
            cfg.sqpoll_cpu = Some(placement::sqpoll_cpu(&helpers, cfg.sqpoll_cpu)?);
        }
        // The warning thread, started now rather than on the polling thread
        // at the first line: it writes every finding and runs the route
        // lookups under `warn` and the egress-device checks.
        crate::warn::prepare_on(
            &helpers,
            &placement::thread_cpus(&helpers, placement::sqpoll_of(&cfg)),
        );
        // Each line once per process, under the switch (`off` drops the
        // warning lines).
        for l in report
            .lines_under(policy)
            .into_iter()
            .chain(report.helper_line())
        {
            crate::warn::once(l);
        }
        // `require`: a warning fails the start, its lines printed above.
        if let Some(e) = report.refusal(policy) {
            return Err(e.into());
        }
        if let Some(asked) = cfg.rcvbuf
            && let Ok(granted) = sys::granted_rcvbuf(asked)
            && let Some(l) = checks::rcvbuf_line(asked, granted)
        {
            crate::warn::once(l);
        }
        let ctx = Arc::new(DevContext::new(&cfg, report.hot.clone()));
        let tpu = ticks_per_us();
        let imp = match cfg.backend {
            Backend::EpollSpin | Backend::EpollBusy | Backend::RecvBusy => {
                Imp::Epoll(epoll::Epoll::new(&cfg)?)
            }
            #[cfg(feature = "uring")]
            Backend::UringSpin | Backend::UringNapi | Backend::UringSqpoll => {
                Imp::Uring(uring::Uring::new(&cfg)?)
            }
            #[cfg(not(feature = "uring"))]
            Backend::UringSpin | Backend::UringNapi | Backend::UringSqpoll => {
                return Err(Error::Unsupported(
                    "io_uring backends need the `uring` feature",
                ));
            }
        };
        Ok(Self {
            imp,
            conns: Conns::new(),
            stats: DriverStats::default(),
            read_seq: 0,
            buffered: false,
            pending: Vec::new(),
            unreported: Vec::new(),
            resolver: Resolver::new(dns.clone()),
            dns,
            report,
            ctx,
            _timestamp_keeper: cfg.rx_timestamps.then(timestamp_keeper).flatten(),
            // Calibrated once here, not on the first connect.
            ticks_per_us: tpu,
            next_expiry: 0,
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            expiry_ticks: (f64::from(EXPIRY_CHECK_US) * tpu) as u64,
            expiry_countdown: 1,
            // A backend that sleeps has slow polls: read the counter on each.
            expiry_polls: if cfg.backend.may_sleep() {
                1
            } else {
                EXPIRY_POLLS
            },
            next_setup: 0,
            background: None,
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            setup_ticks: (f64::from(SETUP_STEP_US) * tpu) as u64,
            cfg,
        })
    }

    /// The configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.cfg
    }

    /// Counters.
    #[must_use]
    pub const fn stats(&self) -> &DriverStats {
        &self.stats
    }

    /// What `Driver::new` found about the host: the loop's CPU and the
    /// findings of the CPU, scheduling and THP checks (see
    /// [`crate::checks`]); each was also printed once.
    #[must_use]
    pub const fn host_report(&self) -> &HostReport {
        &self.report
    }

    /// Takes over a connected, non-blocking endpoint. Sets its socket options
    /// (receive timestamps, `SO_RCVBUF`, `SO_BUSY_POLL`, `TCP_NODELAY`, TCP
    /// keepalive and `TCP_USER_TIMEOUT`; `TCP_CORK` left on is reported),
    /// checks its route for quick ACK
    /// (`Config::quickack_check`, see [`crate::route`]: under `warn` on
    /// ergon-io's warning thread, under `require` here) and starts
    /// receiving.
    ///
    /// # Errors
    /// A socket option or the backend registration fails, or
    /// [`Error::QuickAck`] under `quickack_check=require`; the endpoint is
    /// dropped (its socket closed).
    pub fn add(&mut self, ep: E) -> Result<ConnId> {
        let i = self.conns.reserve();
        let id = self.conns.id(i);
        if let Err(e) = self.install(i, ep, true) {
            self.conns.release(i);
            return Err(e);
        }
        Ok(id)
    }

    /// Starts connecting without blocking and returns the connection's id at
    /// once. Each later `poll` advances it (DNS, TCP, TLS, protocol upgrade);
    /// [`Handler::on_open`] reports success, [`Handler::on_closed`] failure or
    /// the `connect_timeout_ms` deadline. Sends queued before it opens are
    /// not possible (`get_mut` returns `None` until then).
    ///
    /// Under `quickack_check=require` the route is checked on the polling
    /// thread as soon as TCP is connected, before the TLS handshake, so a
    /// refused route ([`Error::QuickAck`], to [`Handler::on_closed`]) costs
    /// no handshake; [`DriverStats::route_check_ns`] counts the time. Under
    /// `warn` the lookup runs on ergon-io's warning thread after the
    /// connection opens (see [`crate::route`]).
    ///
    /// # Errors
    /// The connect could not even be started (socket creation failed).
    pub fn connect<D: Dial<Conn = E> + 'static>(&mut self, dial: D) -> Result<ConnId> {
        let i = self.conns.reserve();
        let id = self.conns.id(i);
        let mut p = Pending {
            slot: i,
            dial: Box::new(dial),
            started: false,
            deadline_ns: monotonic_ns() + i64::from(self.cfg.connect_timeout_ms) * 1_000_000,
        };
        if self.cfg.quickack_check == QuickAckCheck::Require {
            // Checked at TCP connect, before the TLS handshake.
            p.dial.gate_route();
        }
        if let Some(addr) = p.dial.addr() {
            if let Err(e) = p.dial.start(addr) {
                self.conns.release(i);
                return Err(e);
            }
            p.started = true;
        }
        self.pending.push(p);
        Ok(id)
    }

    /// Like [`Driver::connect`], but DNS (the helper's own non-blocking
    /// resolver, built from the driver's `DnsConfig`), TCP, TLS, any
    /// upgrade and the quick-ACK route check (under `require` before the TLS
    /// handshake) run on a helper thread the driver starts on first use, so
    /// the polling thread spends no handshake CPU and makes no route lookup.
    /// `poll` only takes the finished connection; [`Handler::on_open`] /
    /// [`Handler::on_closed`] report it as for `connect`. Closing the id first, or dropping the driver, discards
    /// the result and closes its socket.
    ///
    /// The helper runs on `Config::connect_cpu`, else on the helper CPUs
    /// `Driver::new` chose ([`crate::placement`]); never on the polling CPU
    /// when it is known.
    ///
    /// # Errors
    /// [`Error::Cpu`] at once, starting no thread:
    /// [`CpuError::NoSpareCpu`] when the process may use the polling CPU
    /// only (a pod with one CPU), where the helper would time-slice the
    /// polling thread: use [`Driver::connect`], or set
    /// `Config::inline_connect_fallback` to have this call do that, or give
    /// the pod a second CPU; [`CpuError::Outside`] or
    /// [`CpuError::PollingCpu`] for a `connect_cpu` outside the process's
    /// cpuset, or that is the polling CPU. Otherwise: the helper thread
    /// could not be started.
    pub fn connect_background<D>(&mut self, dial: D) -> Result<ConnId>
    where
        D: Dial<Conn = E> + Send + 'static,
        E: Send + 'static,
    {
        if self.background.is_none() {
            match self.background_cpus()? {
                Some(cpus) => {
                    let step = placement::dial_step(
                        &cpus,
                        placement::sqpoll_of(&self.cfg).and_then(|s| usize::try_from(s).ok()),
                    );
                    self.background = Some(background::Background::start(
                        &cpus,
                        self.dns.clone(),
                        step,
                    )?);
                }
                None => return self.connect(dial),
            }
        }
        let i = self.conns.reserve();
        let id = self.conns.id(i);
        let timeout = std::time::Duration::from_millis(u64::from(self.cfg.connect_timeout_ms));
        let opts = SockOpts::of(&self.cfg, &self.ctx);
        let mut dial = dial;
        if self.cfg.quickack_check == QuickAckCheck::Require {
            // Checked on the helper thread at TCP connect, before TLS.
            dial.gate_route();
        }
        if let Some(bg) = self.background.as_mut()
            && let Err(e) = bg.submit(
                i,
                id.generation(),
                Box::new(dial),
                Box::new(move |c: &mut E, id: ConnId| {
                    opts.prepare(c, id, false, false).map(|_| ())
                }),
                timeout,
            )
        {
            self.conns.release(i);
            return Err(e);
        }
        Ok(id)
    }

    /// Where `connect_background` connects: `Some` CPUs for its helper
    /// thread, or `None` on the polling thread (a process that may use the
    /// polling CPU only, under `Config::inline_connect_fallback`). Reads no
    /// file: `Driver::new` chose the CPUs.
    fn background_cpus(&self) -> Result<Option<Vec<usize>>> {
        let cpus = match &self.report.helpers {
            Ok(p) => placement::dial_cpus(p, &self.cfg),
            Err(e) => Err(e.clone()),
        };
        match cpus {
            Ok(cpus) => Ok(Some(cpus)),
            Err(CpuError::NoSpareCpu { .. }) if self.cfg.inline_connect_fallback => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// `Err` with [`Error::Cpu`] exactly when `connect_background` would
    /// refuse for want of a CPU (see there), so a caller that can only ever
    /// connect in the background fails at once rather than on every attempt.
    pub(crate) fn background_ready(&self) -> Result<()> {
        if self.background.is_none() {
            self.background_cpus()?;
        }
        Ok(())
    }

    /// Frees the connections closed during the current call: on the helper
    /// thread when there is one (their sockets close moments later, there),
    /// otherwise here before the call returns. The helper takes the `munmap`
    /// calls and the wait for their TLB shootdowns; the shootdown IPI itself
    /// still reaches the polling CPU, which runs the same address space.
    #[cold]
    fn free_closed(&mut self) {
        let Self {
            conns, background, ..
        } = self;
        for c in conns.closed.drain(..) {
            match background {
                Some(b) => b.free(c),
                None => drop(c),
            }
        }
    }

    /// Installs connections finished on the helper thread.
    #[cold]
    fn take_background<H: Handler>(&mut self, handler: &mut H) {
        while let Some(done) = self
            .background
            .as_mut()
            .and_then(background::Background::try_take)
        {
            let id = ConnId::new(
                u32::try_from(done.slot).unwrap_or(u32::MAX),
                done.generation,
            );
            // Closed while connecting: the slot was released; free the result
            // with the other closed connections.
            if self.conns.lookup_reserved(id).is_none() {
                if let Ok(ep) = done.result {
                    self.conns.closed.push(ep);
                }
                continue;
            }
            match done
                .result
                .and_then(|ep| self.install_prepared(done.slot, ep))
            {
                Ok(()) => {
                    self.stats.connects += 1;
                    handler.on_open(id);
                }
                Err(e) => {
                    self.conns.release(done.slot);
                    self.stats.connect_failures += 1;
                    handler.on_closed(id, e);
                }
            }
        }
    }

    /// Starts resolving `host` now so a later [`Driver::connect`] finds it
    /// cached. Answers are kept for their TTL.
    ///
    /// # Errors
    /// The name is invalid or no name server is configured.
    pub fn prefetch(&mut self, host: &str) -> Result<()> {
        self.resolver.prefetch(host, monotonic_ns())
    }

    /// Whether connections are still being set up.
    #[must_use]
    pub const fn connecting(&self) -> usize {
        self.pending.len()
    }

    /// Socket options and the connection's checks on the polling thread
    /// (`added`: handed to `Driver::add`), backend registration and the slot.
    fn install(&mut self, i: usize, mut ep: E, added: bool) -> Result<()> {
        let id = self.conns.id(i);
        match SockOpts::of(&self.cfg, &self.ctx).prepare(&mut ep, id, added, true) {
            Ok(cost) => self.count_route(cost),
            Err(e) => {
                self.conns.closed.push(ep);
                return Err(e);
            }
        }
        self.install_prepared(i, ep)
    }

    /// Adds a route check made on the polling thread to the counters.
    fn count_route(&mut self, cost: RouteCost) {
        let ns = ticks_ns(cost.ticks, self.ticks_per_us);
        self.stats.route_lookups += cost.lookups;
        self.stats.route_check_ns += ns;
        self.stats.route_check_max_ns = self.stats.route_check_max_ns.max(ns);
    }

    /// Backend registration and the slot, for an endpoint whose socket options
    /// are already set.
    #[cfg_attr(not(feature = "uring"), allow(unused_mut))]
    fn install_prepared(&mut self, i: usize, mut ep: E) -> Result<()> {
        let fd = ep.fd();
        let id = self.conns.id(i);
        let added = match &mut self.imp {
            Imp::Epoll(e) => e.add(fd, id),
            #[cfg(feature = "uring")]
            Imp::Uring(u) => u.add(&mut ep, i),
        };
        if let Err(e) = added {
            // Nothing was armed: freed with the other closed connections.
            self.conns.closed.push(ep);
            return Err(e);
        }
        self.buffered |= ep.has_buffered_input();
        self.conns.install(i, ep);
        Ok(())
    }

    /// Advances connections being set up. Each dial step's CPU time is tracked
    /// in [`DriverStats::connect_step_max_ns`].
    #[cold]
    fn advance_pending<H: Handler>(&mut self, handler: &mut H) {
        let now = monotonic_ns();
        self.resolver.poll(now);
        let mut k = 0;
        while k < self.pending.len() {
            let p = &mut self.pending[k];
            let outcome = advance_one(
                p,
                &mut self.resolver,
                &mut self.stats,
                now,
                self.ticks_per_us,
                &require_route,
            );
            let ep = match outcome {
                Ok(None) => {
                    k += 1;
                    continue;
                }
                Ok(Some(ep)) => Ok(ep),
                Err(e) => Err(e),
            };
            let p = self.pending.swap_remove(k);
            let id = self.conns.id(p.slot);
            let installed = ep.and_then(|ep| self.install(p.slot, ep, false));
            match installed {
                Ok(()) => {
                    self.stats.connects += 1;
                    handler.on_open(id);
                }
                Err(e) => {
                    self.conns.release(p.slot);
                    self.stats.connect_failures += 1;
                    handler.on_closed(id, e);
                }
            }
        }
    }

    /// The connection, if `id` is still open.
    #[must_use]
    pub fn get(&self, id: ConnId) -> Option<&E> {
        self.conns
            .lookup(id)
            .and_then(|i| self.conns.slots[i].as_ref())
    }

    /// The connection, if `id` is still open, for queuing sends. The handle
    /// offers the connection's methods but cannot replace or move it.
    pub fn get_mut(&mut self, id: ConnId) -> Option<ConnMut<'_, E>> {
        self.conns
            .lookup(id)
            .and_then(|i| self.conns.at(i))
            .map(ConnMut::new)
    }

    /// Open connections.
    pub fn iter(&self) -> impl Iterator<Item = (ConnId, &E)> {
        self.conns
            .slots
            .iter()
            .enumerate()
            .filter(|&(i, _)| !self.conns.is_closing(i))
            .filter_map(|(i, c)| c.as_ref().map(|c| (self.conns.id(i), c)))
    }

    /// Number of open connections.
    #[must_use]
    pub fn len(&self) -> usize {
        (0..self.conns.len_slots())
            .filter(|&i| self.conns.is_open(i))
            .count()
    }

    /// Whether there are no open connections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Closes a connection at once; stale or unknown ids are ignored. No
    /// WebSocket Close is sent (for a close handshake use `WsConn::close`)
    /// and [`Handler::on_closed`] is not called.
    /// With `io_uring` the endpoint is dropped once the kernel has released
    /// its buffers (on a later `poll`, or when the driver is dropped).
    pub fn close(&mut self, id: ConnId) {
        if let Some(i) = self.conns.lookup(id) {
            close_index(&mut self.imp, &mut self.conns, i);
            self.free_closed();
        } else if let Some(i) = self.conns.lookup_reserved(id) {
            // Still connecting: dropping the dial closes its socket (a
            // background result for the released slot is dropped on arrival).
            self.pending.retain(|p| p.slot != i);
            self.conns.release(i);
        }
    }

    /// Whether every connection's NIC queue can be polled: the backend polls
    /// the NIC and every socket has a NAPI id (known once it has received a
    /// packet).
    #[must_use]
    pub fn nic_polling_active(&self) -> bool {
        self.cfg.backend.polls_nic()
            && self
                .iter()
                .all(|(_, c)| sys::incoming_napi_id(c.fd()).unwrap_or(0) != 0)
    }

    /// One iteration: receive whatever the kernel has, decode and deliver it,
    /// then send queued bytes. Returns messages delivered. Never sleeps unless
    /// the backend is [`Backend::EpollBusy`] or [`Backend::UringNapi`].
    ///
    /// # Errors
    /// The backend itself failed. Connection errors close that connection and
    /// go to [`Handler::on_closed`].
    #[inline]
    pub fn poll<H: Handler>(&mut self, handler: &mut H) -> Result<usize>
    where
        E: Receive<H>,
    {
        self.stats.polls += 1;
        let before = self.stats.messages;
        if !self.unreported.is_empty() {
            for (id, e) in self.unreported.drain(..) {
                handler.on_closed(id, e);
            }
        }
        if self
            .background
            .as_ref()
            .is_some_and(background::Background::has_outstanding)
        {
            self.take_background(handler);
        }
        if !self.pending.is_empty() || self.resolver.busy() {
            // Set-up steps cost syscalls; take them at most every few µs.
            let t = rdtsc();
            if t >= self.next_setup {
                self.next_setup = t.wrapping_add(self.setup_ticks);
                self.advance_pending(handler);
            }
        }
        if self.buffered {
            self.buffered = false;
            self.deliver_buffered(handler);
        }
        let mut r = match &mut self.imp {
            Imp::Epoll(e) => e.poll(
                &mut self.conns,
                handler,
                &mut self.stats,
                &mut self.read_seq,
            ),
            // Receives, sends and the kernel entry in one pass, so re-arms and
            // sends share one io_uring_enter.
            #[cfg(feature = "uring")]
            Imp::Uring(u) => u.poll(
                &mut self.conns,
                handler,
                &mut self.stats,
                &mut self.read_seq,
            ),
        };
        if r.is_ok() && matches!(self.imp, Imp::Epoll(_)) {
            r = self.send_queued(handler);
        }
        // After receiving, so a response already in the socket is not timed out.
        self.check_deadlines(handler);
        if !self.conns.closed.is_empty() {
            self.free_closed();
        }
        r?;
        Ok(usize::try_from(self.stats.messages - before).unwrap_or(usize::MAX))
    }

    /// Every `expiry_polls` polls, reads the counter; every 250 µs, runs
    /// the endpoints' deadlines and timers.
    #[inline]
    fn check_deadlines<H: Handler>(&mut self, handler: &mut H) {
        if E::DEADLINES {
            self.expiry_countdown -= 1;
            if self.expiry_countdown == 0 {
                self.expiry_countdown = self.expiry_polls;
                let t = rdtsc();
                if t >= self.next_expiry {
                    self.next_expiry = t.wrapping_add(self.expiry_ticks);
                    self.expire_all(handler);
                }
            }
        }
    }

    /// Closes connections whose deadlines passed.
    #[cold]
    fn expire_all<H: Handler>(&mut self, handler: &mut H) {
        let now = monotonic_ns();
        for i in 0..self.conns.len_slots() {
            let id = self.conns.id(i);
            if !self.conns.is_open(i) {
                continue;
            }
            let Some(c) = self.conns.at(i) else {
                continue;
            };
            if let Err(e) = c.expire(now) {
                let e = c.close_reason(e);
                close_index(&mut self.imp, &mut self.conns, i);
                self.stats.closed += 1;
                handler.on_closed(id, e);
            }
        }
    }

    /// Delivers input that endpoints already held when they were added.
    #[cold]
    fn deliver_buffered<H: Handler>(&mut self, handler: &mut H)
    where
        E: Receive<H>,
    {
        let meta = RxMeta::default();
        for i in 0..self.conns.len_slots() {
            let id = self.conns.id(i);
            if !self.conns.is_open(i) {
                continue;
            }
            let Some(c) = self.conns.at(i) else {
                continue;
            };
            if !c.has_buffered_input() {
                continue;
            }
            // io_uring arms this connection's receive only now, after the
            // buffer layout has settled.
            let r = c.on_received(id, 0, &meta, handler);
            #[cfg(feature = "uring")]
            if let (Ok(_), Imp::Uring(u)) = (&r, &mut self.imp) {
                u.arm_initial(c, i);
            }
            match r {
                Ok(n) => self.stats.messages += n as u64,
                Err(e) => {
                    let e = c.close_reason(e);
                    close_index(&mut self.imp, &mut self.conns, i);
                    self.stats.closed += 1;
                    handler.on_closed(id, e);
                }
            }
        }
    }

    /// Sends queued bytes now (`poll` also does this). Connections that fail are
    /// closed and reported to the handler of the next `poll`;
    /// [`Driver::flush_with`] reports them at once.
    ///
    /// # Errors
    /// The backend itself failed.
    pub fn flush(&mut self) -> Result<()> {
        struct Defer(Vec<(ConnId, Error)>);
        impl Handler for Defer {
            fn on_closed(&mut self, id: ConnId, error: Error) {
                self.0.push((id, error));
            }
        }
        let mut d = Defer(std::mem::take(&mut self.unreported));
        let r = self.flush_with(&mut d);
        self.unreported = d.0;
        r
    }

    /// Sends queued bytes now, reporting connections that fail to `handler`.
    ///
    /// # Errors
    /// The backend itself failed.
    #[inline]
    pub fn flush_with<H: Handler>(&mut self, handler: &mut H) -> Result<()> {
        let r = self.send_queued(handler);
        if !self.conns.closed.is_empty() {
            self.free_closed();
        }
        r
    }

    #[inline]
    #[cfg_attr(not(feature = "uring"), allow(clippy::unnecessary_wraps))]
    fn send_queued<H: Handler>(&mut self, handler: &mut H) -> Result<()> {
        match &mut self.imp {
            Imp::Epoll(e) => {
                for i in 0..self.conns.len_slots() {
                    let Some(c) = self.conns.at(i) else {
                        continue;
                    };
                    if c.tx_queue().is_empty() {
                        continue;
                    }
                    self.stats.send_calls += 1;
                    self.stats.sends += 1;
                    let fd = c.fd();
                    match sys::send(fd, c.tx_queue()) {
                        Ok(n) => {
                            c.tx_queue().drain(..n);
                            c.on_handed(n);
                        }
                        Err(err) => {
                            let err = c.close_reason(err.into());
                            let id = self.conns.id(i);
                            e.remove(fd);
                            self.conns.remove(i);
                            self.stats.closed += 1;
                            handler.on_closed(id, err);
                        }
                    }
                }
                Ok(())
            }
            #[cfg(feature = "uring")]
            Imp::Uring(u) => u.flush(&mut self.conns, &mut self.stats),
        }
    }
}

impl<E> Drop for Driver<E> {
    fn drop(&mut self) {
        // io_uring may still reference endpoint buffers: cancel and reap before
        // the connections are freed (fields drop after this body).
        #[cfg(feature = "uring")]
        if let Imp::Uring(u) = &mut self.imp
            && !u.shutdown()
        {
            self.conns.leak();
        }
    }
}

/// A UDP socket with software receive timestamps enabled (see
/// `Driver::_timestamp_keeper`); `None` if the kernel refuses.
fn timestamp_keeper() -> Option<socket2::Socket> {
    use std::os::fd::AsRawFd;
    let s = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, None).ok()?;
    sys::enable_rx_timestamps(s.as_raw_fd()).ok()?;
    Some(s)
}

fn close_index<E: Endpoint>(imp: &mut Imp, conns: &mut Conns<E>, i: usize) {
    match imp {
        Imp::Epoll(e) => {
            if let Some(fd) = conns.at(i).map(|c| c.fd()) {
                e.remove(fd);
                conns.remove(i);
            }
        }
        #[cfg(feature = "uring")]
        Imp::Uring(u) => u.close(conns, i),
    }
}

fn advance_one<E: Endpoint>(
    p: &mut Pending<E>,
    resolver: &mut Resolver,
    stats: &mut DriverStats,
    now: i64,
    tpu: f64,
    check: &dyn Fn(RawFd) -> std::result::Result<RouteCheck, QuickAckError>,
) -> Result<Option<E>> {
    if now > p.deadline_ns {
        return Err(Error::Timeout("connect"));
    }
    if !p.started {
        match resolver.lookup(p.dial.host(), now)? {
            Lookup::Ready(ip) => {
                p.dial
                    .start(SocketAddr::new(IpAddr::V4(ip), p.dial.port()))?;
                p.started = true;
            }
            Lookup::Pending => return Ok(None),
        }
    }
    let t0 = rdtsc();
    let r = p.dial.step();
    let ns = ticks_ns(rdtsc().wrapping_sub(t0), tpu);
    stats.connect_step_max_ns = stats.connect_step_max_ns.max(ns);
    if let (Ok(None), Some(fd)) = (&r, p.dial.awaiting_route()) {
        // Under `require`: TCP is connected and the TLS handshake has not
        // started, so a refused route costs no handshake. The next step
        // starts it.
        let t0 = rdtsc();
        let found = check(fd);
        let ns = ticks_ns(rdtsc().wrapping_sub(t0), tpu);
        stats.route_lookups += 1;
        stats.route_check_ns += ns;
        stats.route_check_max_ns = stats.route_check_max_ns.max(ns);
        p.dial
            .route_checked(found.map_err(|e| Error::QuickAck(Box::new(e)))?);
    }
    r
}

/// Reads once from `conn` through `slot` and delivers what arrived.
fn recv_one<E, H>(
    conn: &mut E,
    id: ConnId,
    slot: &mut sys::MsgSlot,
    handler: &mut H,
    stats: &mut DriverStats,
    read_seq: &mut u64,
) -> Result<()>
where
    E: Receive<H>,
    H: Handler,
{
    slot.arm(conn.rx_space());
    stats.recvmsg_calls += 1;
    match sys::recvmsg(conn.fd(), slot)? {
        None => {
            stats.recvmsg_eagain += 1;
            Ok(())
        }
        Some(0) => Err(Error::Closed),
        Some(n) => {
            *read_seq += 1;
            stats.completions += 1;
            let meta = slot.meta(*read_seq);
            stats.messages += conn.on_received(id, n, &meta, handler)? as u64;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::endpoint::sealed::DialIo;
    use crate::http::{HttpConn, HttpDial, HttpOptions};
    use crate::net::Target;
    use crate::route::RouteError;

    type R<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// A dial gated under `require`, connecting to a plain TCP listener (no
    /// TLS server: the test reads what the client sends), and the accepted
    /// server side.
    fn gated() -> R<(Pending<HttpConn>, TcpStream)> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let target = Target::new("localhost", addr.port(), "/").with_addr(addr);
        let tls = crate::tls::client_config(rustls::RootCertStore::empty())?;
        let mut dial = HttpDial::new(target, tls, HttpOptions::default())?;
        dial.gate_route();
        dial.start(addr)?;
        let p = Pending {
            slot: 0,
            dial: Box::new(dial),
            started: true,
            deadline_ns: monotonic_ns() + 5_000_000_000,
        };
        let (server, _) = listener.accept()?;
        Ok((p, server))
    }

    /// Steps `p` with `check` as the route check until it fails, finishes,
    /// or `until` holds.
    fn step(
        p: &mut Pending<HttpConn>,
        stats: &mut DriverStats,
        check: &dyn Fn(RawFd) -> std::result::Result<RouteCheck, QuickAckError>,
        until: &dyn Fn(&DriverStats) -> bool,
    ) -> R<Result<Option<HttpConn>>> {
        let mut resolver = Resolver::new(DnsConfig::system());
        let tpu = ticks_per_us();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let r = advance_one(p, &mut resolver, stats, monotonic_ns(), tpu, check);
            if !matches!(r, Ok(None)) || until(stats) {
                return Ok(r);
            }
            if Instant::now() > end {
                return Err(format!("no progress: {stats:?}").into());
            }
        }
    }

    #[test]
    fn a_refused_route_costs_no_handshake() -> R {
        let (mut p, mut server) = gated()?;
        let mut stats = DriverStats::default();
        let asked = Cell::new(0);
        let refuse = |_: RawFd| {
            asked.set(asked.get() + 1);
            Err(QuickAckError::Lookup {
                flow: None,
                error: RouteError::Timeout,
            })
        };
        let out = step(&mut p, &mut stats, &refuse, &|_| false)?;
        assert!(matches!(out, Err(Error::QuickAck(_))), "{:?}", out.err());
        assert_eq!(asked.get(), 1);
        assert_eq!(stats.route_lookups, 1);
        drop(p);
        // The client closed without sending a byte: no ClientHello.
        server.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut b = [0u8; 64];
        assert_eq!(server.read(&mut b)?, 0, "TLS bytes before the route check");
        Ok(())
    }

    #[test]
    fn the_handshake_starts_only_after_the_route_is_checked() -> R {
        let (mut p, server) = gated()?;
        server.set_nonblocking(true)?;
        let mut stats = DriverStats::default();
        // When the check runs, the server has received nothing yet.
        let silent_then = Cell::new(None);
        let pass = |_: RawFd| {
            let mut b = [0u8; 1];
            silent_then.set(Some(matches!(
                server.peek(&mut b),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
            )));
            Ok(RouteCheck::NotChecked)
        };
        let out = step(&mut p, &mut stats, &pass, &|s| s.route_lookups > 0)?;
        assert!(matches!(out, Ok(None)), "{:?}", out.err());
        assert_eq!(silent_then.get(), Some(true), "TLS bytes before the check");
        // Then the next step sends the ClientHello: a TLS handshake record.
        let mut b = [0u8; 1];
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = step(&mut p, &mut stats, &pass, &|_| true)?;
            match server.peek(&mut b) {
                Ok(1) => break,
                _ if Instant::now() > end => return Err("no ClientHello after the check".into()),
                _ => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        assert_eq!(b[0], 0x16, "not a TLS handshake record");
        assert_eq!(stats.route_lookups, 1, "checked once");
        Ok(())
    }
}
