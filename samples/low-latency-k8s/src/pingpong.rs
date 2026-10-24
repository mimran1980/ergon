//! The Aeron ping/pong and its hot thread.
//!
//! Aeron handles are `!Send` (the client's children hold an `Rc` to it), so
//! the hot thread builds everything itself, in this order:
//!
//! 1. While still on the shared CPUs: `AeronContext`, `Aeron::start`
//!    (retried while no driver answers, up to `LL_DRIVER_WAIT_S`), which
//!    spawns the client conductor `aeron-client` with the hot thread's
//!    shared mask; then the publication and the subscription, and a wait
//!    until both are connected.
//! 2. Pin to its exclusive CPU and check `sched_getcpu()`; log one `info!`
//!    (it warms the thread's formatting buffer); report "pinned".
//! 3. Wait for `main`'s self-check, then run: warm-up, then the measured
//!    round trips (ping) or the echo loop (pong).
//!
//! The ping stamps each message with the clock's ordered read and its
//! sequence number; the pong echoes it unchanged. Closed loop
//! (`LL_RATE=0`): one message in flight, as the lab's `rtt xping`. Open
//! loop (`LL_RATE>0`): a message every `1/rate` s, each round trip timed
//! from when it was due, so a stall counts against every message behind
//! it. The hot loop logs only rate-limited warnings.

use std::ffi::CString;
use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use rusteron_client::{
    Aeron, AeronContext, AeronExclusivePublication, AeronImage, AeronSubscription, Handlers,
};
use tracing::level_filters::LevelFilter;

use crate::LlError;
use crate::cpus::{CpuList, Spec};
use crate::logging::parse_level;
use crate::ratelimit::RateLimit;
use crate::stats::{Counters, Latency, LoopGap, Percentiles};

/// Which side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Sends, times the echo.
    Ping,
    /// Echoes.
    Pong,
}

impl Role {
    /// `ping` or `pong`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::Pong => "pong",
        }
    }
}

/// The smallest message: a sequence number and a stamp.
pub const MIN_MSG_BYTES: usize = 16;
/// The largest message this sample sends (one Aeron frame under a 1408 MTU).
pub const MAX_MSG_BYTES: usize = 1344;

/// `value` parsed, or `default` when unset.
fn num<T: std::str::FromStr>(name: &str, value: Option<String>, default: T) -> Result<T, LlError> {
    value.map_or(Ok(default), |v| {
        v.trim()
            .parse()
            .map_err(|_| LlError::bad(name, v.as_str(), "not a number"))
    })
}

/// The app's settings, from the environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    /// `LL_ROLE`: `ping` or `pong`.
    pub role: Role,
    /// `AERON_DIR` (Aeron reads it itself when unset here).
    pub aeron_dir: Option<String>,
    /// `LL_PUB_CHANNEL`: where this side publishes.
    pub pub_channel: String,
    /// `LL_SUB_CHANNEL`: where it subscribes.
    pub sub_channel: String,
    /// `LL_PING_STREAM` (1001): the ping's stream.
    pub ping_stream: i32,
    /// `LL_PONG_STREAM` (1002): the echo's stream.
    pub pong_stream: i32,
    /// `LL_MSG_BYTES` (32).
    pub msg_bytes: usize,
    /// `LL_WARMUP` (100000): round trips before measuring.
    pub warmup: u64,
    /// `LL_SAMPLES` (1000000): round trips measured.
    pub samples: u64,
    /// `LL_DURATION_S` (0: no limit): stop measuring after this long.
    pub duration: Duration,
    /// `LL_RATE` (0: closed loop): messages a second.
    pub rate: u64,
    /// `LL_DRIVER_WAIT_S` (60): how long to wait for the driver.
    pub driver_wait: Duration,
    /// `LL_CONNECT_WAIT_S` (60): how long to wait for the peer.
    pub connect_wait: Duration,
    /// `LL_ECHO_TIMEOUT_MS` (1000): when an echo counts as lost.
    pub echo_timeout: Duration,
    /// `LL_BACKPRESSURE_WARN_NS` (1000000): back-pressure longer than this
    /// warns.
    pub backpressure_warn_ns: u64,
    /// `LL_LOG_LEVEL` (`info`).
    pub log_level: LevelFilter,
    /// `LL_LOG_QUEUE_LINES` (4096).
    pub log_queue_lines: usize,
    /// `LL_HOT_CPU` (`isolated:0`).
    pub hot_spec: Spec,
}

impl AppConfig {
    /// Reads the settings through `lookup`.
    ///
    /// # Errors
    ///
    /// [`LlError::MissingEnv`] for `LL_ROLE`, `LL_PUB_CHANNEL` or
    /// `LL_SUB_CHANNEL`; [`LlError::BadValue`] for a value that does not
    /// parse or is out of range.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, LlError> {
        let get = |name: &str| lookup(name).filter(|v| !v.is_empty());
        let required = |name: &'static str| get(name).ok_or(LlError::MissingEnv(name));
        let role = match required("LL_ROLE")?.as_str() {
            "ping" => Role::Ping,
            "pong" => Role::Pong,
            other => return Err(LlError::bad("LL_ROLE", other, "ping or pong")),
        };
        let msg_bytes: usize = num("LL_MSG_BYTES", get("LL_MSG_BYTES"), 32)?;
        if !(MIN_MSG_BYTES..=MAX_MSG_BYTES).contains(&msg_bytes) {
            return Err(LlError::bad(
                "LL_MSG_BYTES",
                msg_bytes.to_string(),
                format!("{MIN_MSG_BYTES} to {MAX_MSG_BYTES}"),
            ));
        }
        let queue: usize = num("LL_LOG_QUEUE_LINES", get("LL_LOG_QUEUE_LINES"), 4096)?;
        if queue == 0 {
            return Err(LlError::bad("LL_LOG_QUEUE_LINES", "0", "at least 1"));
        }
        let secs =
            |name: &str, default: u64| num(name, get(name), default).map(Duration::from_secs);
        Ok(Self {
            role,
            aeron_dir: get("AERON_DIR"),
            pub_channel: required("LL_PUB_CHANNEL")?,
            sub_channel: required("LL_SUB_CHANNEL")?,
            ping_stream: num("LL_PING_STREAM", get("LL_PING_STREAM"), 1001)?,
            pong_stream: num("LL_PONG_STREAM", get("LL_PONG_STREAM"), 1002)?,
            msg_bytes,
            warmup: num("LL_WARMUP", get("LL_WARMUP"), 100_000)?,
            samples: num("LL_SAMPLES", get("LL_SAMPLES"), 1_000_000)?,
            duration: secs("LL_DURATION_S", 0)?,
            rate: num("LL_RATE", get("LL_RATE"), 0)?,
            driver_wait: secs("LL_DRIVER_WAIT_S", 60)?,
            connect_wait: secs("LL_CONNECT_WAIT_S", 60)?,
            echo_timeout: Duration::from_millis(num(
                "LL_ECHO_TIMEOUT_MS",
                get("LL_ECHO_TIMEOUT_MS"),
                1000,
            )?),
            backpressure_warn_ns: num(
                "LL_BACKPRESSURE_WARN_NS",
                get("LL_BACKPRESSURE_WARN_NS"),
                1_000_000,
            )?,
            log_level: get("LL_LOG_LEVEL").map_or(Ok(LevelFilter::INFO), |v| parse_level(&v))?,
            log_queue_lines: queue,
            hot_spec: get("LL_HOT_CPU").map_or(Ok(Spec::Isolated(0)), |v| Spec::parse(&v))?,
        })
    }

    /// From the process environment.
    ///
    /// # Errors
    ///
    /// As [`AppConfig::from_lookup`].
    pub fn from_process() -> Result<Self, LlError> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// The stream this side publishes on.
    #[must_use]
    pub const fn pub_stream(&self) -> i32 {
        match self.role {
            Role::Ping => self.ping_stream,
            Role::Pong => self.pong_stream,
        }
    }

    /// The stream this side subscribes to.
    #[must_use]
    pub const fn sub_stream(&self) -> i32 {
        match self.role {
            Role::Ping => self.pong_stream,
            Role::Pong => self.ping_stream,
        }
    }
}

/// The clock, in ns from its creation.
///
/// It is quanta's: the CPU counter (`rdtsc` on x86-64, `CNTVCT_EL0` on
/// aarch64), scaled against `CLOCK_MONOTONIC` at start. A stamp is taken after a `SeqCst` fence (`mfence` on x86-64,
/// `dmb ish` on aarch64): this sample has no `unsafe`, so it cannot issue
/// the lab's `lfence`/`isb`, and a fence orders memory, not the counter
/// read itself. The difference is a few ns of reordering at most.
#[derive(Clone, Debug)]
pub struct Clock {
    inner: quanta::Clock,
    origin: u64,
    realtime_origin_ns: u64,
}

impl Clock {
    /// Calibrates (once a process, up to about 200 ms) and starts at 0.
    #[must_use]
    pub fn new() -> Self {
        let inner = quanta::Clock::new();
        let origin = inner.raw();
        Self {
            inner,
            origin,
            realtime_origin_ns: realtime_ns(),
        }
    }

    /// A latency stamp, ns.
    #[inline]
    #[must_use]
    pub fn stamp(&self) -> u64 {
        std::sync::atomic::fence(Ordering::SeqCst);
        self.inner.delta_as_nanos(self.origin, self.inner.raw())
    }

    /// The time now, ns, unordered: deadlines and loop gaps.
    #[inline]
    #[must_use]
    pub fn now(&self) -> u64 {
        self.inner.delta_as_nanos(self.origin, self.inner.raw())
    }

    /// A stamp of this clock on `CLOCK_REALTIME`, ns.
    #[must_use]
    pub const fn realtime(&self, stamp: u64) -> u64 {
        self.realtime_origin_ns.saturating_add(stamp)
    }

    /// The raw counter's rate, Hz, measured against `Instant` over `over`.
    #[must_use]
    pub fn counter_hz(&self, over: Duration) -> u64 {
        let (r0, t0) = (self.inner.raw(), Instant::now());
        while t0.elapsed() < over {
            std::thread::sleep(Duration::from_millis(5));
        }
        let (r1, elapsed) = (self.inner.raw(), t0.elapsed());
        let ticks = u128::from(r1.wrapping_sub(r0));
        u64::try_from(ticks * 1_000_000_000 / elapsed.as_nanos().max(1)).unwrap_or(u64::MAX)
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

/// `CLOCK_REALTIME`, ns since the epoch.
#[must_use]
pub fn realtime_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

/// What the hot thread shares with `main` and `ll-stats`.
#[derive(Debug, Default)]
pub struct Shared {
    /// Progress counters.
    pub counters: Counters,
    /// Set by SIGTERM/SIGINT or by `main`: stop at the next check.
    pub stop: AtomicBool,
}

impl Shared {
    /// Whether to stop.
    #[inline]
    #[must_use]
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

/// The hot thread's report that it is pinned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pinned {
    /// Its thread id (0 off Linux).
    pub tid: u32,
    /// The CPU it is pinned to and runs on.
    pub cpu: usize,
}

/// What the hot thread measured.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HotOutcome {
    /// Round trips measured (ping) or messages echoed (pong).
    pub messages: u64,
    /// Round trips warmed up.
    pub warmed: u64,
    /// Echoes that never came back.
    pub lost: u64,
    /// Percentiles of the round trips (ping).
    pub rtt: Option<Percentiles>,
    /// The largest round trips, `(realtime_ns, rtt_ns)`.
    pub top: Vec<(u64, u64)>,
    /// Its loop gaps.
    pub gap: LoopGap,
    /// Its CPU, and where it ran at the end.
    pub cpu: usize,
    /// `sched_getcpu()` at the end.
    pub ran_on: usize,
    /// Context switches over the run: voluntary, involuntary.
    pub ctxt: (u64, u64),
    /// Warning lines it emitted.
    pub warn_events: u64,
    /// Warning events its limits suppressed.
    pub suppressed: u64,
    /// Why it ended (`samples`, `duration`, `stop`, `peer closed`).
    pub ended: String,
}

/// The running hot thread.
pub struct HotHandle {
    /// Its join handle.
    pub join: std::thread::JoinHandle<Result<HotOutcome, LlError>>,
    /// Reports once, when pinned (or why it could not get there).
    pub pinned: mpsc::Receiver<Result<Pinned, LlError>>,
    /// `true`: run; `false`: stop without running.
    pub go: mpsc::Sender<bool>,
}

/// Spawns `ll-hot` from the calling thread (which must be on the shared
/// CPUs: `aeron-client` inherits this thread's mask through `ll-hot`).
///
/// # Errors
///
/// [`LlError::Thread`] when the thread cannot be spawned.
pub fn spawn_hot(
    cfg: AppConfig,
    hot_cpu: CpuList,
    shared: Arc<Shared>,
) -> Result<HotHandle, LlError> {
    let (pinned_tx, pinned) = mpsc::channel();
    let (go, go_rx) = mpsc::channel();
    let join = std::thread::Builder::new()
        .name("ll-hot".to_owned())
        .spawn(move || hot_main(&cfg, &hot_cpu, &shared, &pinned_tx, &go_rx))
        .map_err(|e| LlError::Thread(format!("spawn ll-hot: {e}")))?;
    Ok(HotHandle { join, pinned, go })
}

fn hot_main(
    cfg: &AppConfig,
    hot_cpu: &CpuList,
    shared: &Shared,
    pinned_tx: &mpsc::Sender<Result<Pinned, LlError>>,
    go: &mpsc::Receiver<bool>,
) -> Result<HotOutcome, LlError> {
    let setup = (|| -> Result<(Session, AeronImage, Pinned), LlError> {
        let session = Session::connect(cfg, shared)?;
        let image = session.wait_connected(cfg, shared)?;
        let cpu = crate::cpus::pin_and_check(hot_cpu)?;
        // Warms this thread's fmt buffer: the first event on a thread
        // allocates more than the rest (tests/logger_alloc.rs).
        tracing::info!(cpu, role = cfg.role.name(), "ll-hot pinned");
        let tid = crate::cpus::current_tid().unwrap_or(0);
        Ok((session, image, Pinned { tid, cpu }))
    })();
    let (session, image, pinned) = match setup {
        Ok(s) => s,
        Err(e) => {
            let _ = pinned_tx.send(Err(LlError::Aeron(e.to_string())));
            return Err(e);
        }
    };
    let _ = pinned_tx.send(Ok(pinned));
    if !go.recv().unwrap_or(false) {
        return Ok(HotOutcome {
            ended: "stopped before running".to_owned(),
            cpu: pinned.cpu,
            ..HotOutcome::default()
        });
    }
    let clock = Clock::new();
    let before = ctxt_switches();
    let mut out = match cfg.role {
        Role::Ping => run_ping(cfg, &session, &image, &clock, shared)?,
        Role::Pong => run_pong(cfg, &session, &image, &clock, shared)?,
    };
    let after = ctxt_switches();
    out.ctxt = (
        after.0.saturating_sub(before.0),
        after.1.saturating_sub(before.1),
    );
    out.cpu = pinned.cpu;
    out.ran_on = crate::cpus::current_cpu().unwrap_or(usize::MAX);
    Ok(out)
}

/// The calling thread's voluntary and involuntary context switches, from
/// `/proc/thread-self/status` (0, 0 where it cannot be read).
#[must_use]
pub fn ctxt_switches() -> (u64, u64) {
    std::fs::read_to_string("/proc/thread-self/status")
        .ok()
        .and_then(|s| crate::probe::proc::parse_status(&s).ok())
        .map_or((0, 0), |s| (s.voluntary, s.nonvoluntary))
}

/// One side's Aeron client, publication and subscription. Lives on the
/// thread that made it.
pub struct Session {
    /// The client.
    pub aeron: Aeron,
    /// Where this side sends.
    pub publication: AeronExclusivePublication,
    /// Where it receives.
    pub subscription: AeronSubscription,
}

fn aeron_err(what: &str, e: impl std::fmt::Debug) -> LlError {
    LlError::Aeron(format!("{what}: {e:?}"))
}

fn cstring(what: &str, s: &str) -> Result<CString, LlError> {
    CString::new(s).map_err(|_| LlError::bad(what, s, "holds a NUL byte"))
}

impl Session {
    /// Starts the client (retrying while no driver answers, up to
    /// `driver_wait`) and adds the publication and the subscription.
    ///
    /// # Errors
    ///
    /// [`LlError::Timeout`] when no driver answered in time;
    /// [`LlError::Aeron`] for any other Aeron failure.
    pub fn connect(cfg: &AppConfig, shared: &Shared) -> Result<Self, LlError> {
        let deadline = Instant::now() + cfg.driver_wait;
        let aeron = loop {
            match start_client(cfg) {
                Ok(a) => break a,
                Err(e) if Instant::now() < deadline && !shared.stopping() => {
                    tracing::debug!(error = %e, "no media driver yet; retrying");
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(e) => {
                    return Err(LlError::Timeout(format!(
                        "no media driver at AERON_DIR within {:?}: {e}",
                        cfg.driver_wait
                    )));
                }
            }
        };
        let pub_channel = cstring("LL_PUB_CHANNEL", &cfg.pub_channel)?;
        let sub_channel = cstring("LL_SUB_CHANNEL", &cfg.sub_channel)?;
        let publication = aeron
            .async_add_exclusive_publication(&pub_channel, cfg.pub_stream())
            .and_then(|p| p.poll_blocking(Duration::from_secs(10)))
            .map_err(|e| aeron_err("add publication", e))?;
        let subscription = aeron
            .async_add_subscription(
                &sub_channel,
                cfg.sub_stream(),
                Handlers::NONE,
                Handlers::NONE,
            )
            .and_then(|s| s.poll_blocking(Duration::from_secs(10)))
            .map_err(|e| aeron_err("add subscription", e))?;
        tracing::info!(
            publication = %cfg.pub_channel,
            pub_stream = cfg.pub_stream(),
            subscription = %cfg.sub_channel,
            sub_stream = cfg.sub_stream(),
            "aeron connected to the driver"
        );
        Ok(Self {
            aeron,
            publication,
            subscription,
        })
    }

    /// Waits until the publication is connected and the subscription has
    /// the peer's image: the image.
    ///
    /// # Errors
    ///
    /// [`LlError::Timeout`] after `connect_wait`.
    pub fn wait_connected(&self, cfg: &AppConfig, shared: &Shared) -> Result<AeronImage, LlError> {
        let deadline = Instant::now() + cfg.connect_wait;
        loop {
            if self.publication.is_connected()
                && let Some(image) = self.subscription.image_at_index(0)
            {
                return Ok(image);
            }
            if Instant::now() > deadline || shared.stopping() {
                return Err(LlError::Timeout(format!(
                    "the peer within {:?}: publication connected={}, images={}",
                    cfg.connect_wait,
                    self.publication.is_connected(),
                    self.subscription.image_count().unwrap_or(-1)
                )));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn start_client(cfg: &AppConfig) -> Result<Aeron, LlError> {
    let context = AeronContext::new().map_err(|e| aeron_err("context", e))?;
    if let Some(dir) = &cfg.aeron_dir {
        context
            .set_dir(&cstring("AERON_DIR", dir)?)
            .map_err(|e| aeron_err("set_dir", e))?;
    }
    let aeron = Aeron::new(&context).map_err(|e| aeron_err("Aeron::new", e))?;
    aeron.start().map_err(|e| aeron_err("Aeron::start", e))?;
    Ok(aeron)
}

/// The hot loop's warning sites, each limited to one line a second.
struct Warnings {
    echo_timeout: RateLimit,
    backpressure: RateLimit,
    not_connected: RateLimit,
}

impl Warnings {
    const fn new() -> Self {
        Self {
            echo_timeout: RateLimit::per_second(),
            backpressure: RateLimit::per_second(),
            not_connected: RateLimit::per_second(),
        }
    }

    const fn emitted(&self) -> u64 {
        self.echo_timeout.emitted() + self.backpressure.emitted() + self.not_connected.emitted()
    }

    const fn suppressed(&self) -> u64 {
        self.echo_timeout.total_suppressed()
            + self.backpressure.total_suppressed()
            + self.not_connected.total_suppressed()
    }
}

/// Offers `msg` until it goes, back-pressure warned past the threshold:
/// `false` on stop or a fatal error (`fatal` set).
#[inline]
fn offer(
    publication: &AeronExclusivePublication,
    msg: &[u8],
    clock: &Clock,
    shared: &Shared,
    warn: &mut Warnings,
    warn_ns: u64,
    fatal: &mut Option<LlError>,
) -> bool {
    let start = clock.now();
    let mut warned = false;
    loop {
        match publication.offer(msg) {
            Ok(_) => return true,
            Err(e) if e.is_retryable() => {
                let now = clock.now();
                if !warned && now.saturating_sub(start) > warn_ns {
                    warned = true;
                    let site = if matches!(e, rusteron_client::AeronOfferError::NotConnected) {
                        &mut warn.not_connected
                    } else {
                        &mut warn.backpressure
                    };
                    if let Some(suppressed) = site.check(now) {
                        tracing::warn!(error = %e, waited_ns = now - start, suppressed, "offer held back");
                        Counters::bump(&shared.counters.warn_events);
                    }
                }
                if shared.stopping() {
                    return false;
                }
                spin_loop();
            }
            Err(e) => {
                *fatal = Some(aeron_err("offer", e));
                return false;
            }
        }
    }
}

const FRAGMENT_LIMIT: usize = 10;

#[allow(clippy::too_many_lines)] // one loop, read top to bottom
fn run_ping(
    cfg: &AppConfig,
    session: &Session,
    image: &AeronImage,
    clock: &Clock,
    shared: &Shared,
) -> Result<HotOutcome, LlError> {
    let mut latency = Latency::new()?;
    let mut gap = LoopGap::default();
    let mut warn = Warnings::new();
    let mut msg = vec![0_u8; cfg.msg_bytes];
    let timeout_ns = u64::try_from(cfg.echo_timeout.as_nanos()).unwrap_or(u64::MAX);
    let period_ns = if cfg.rate == 0 {
        0
    } else {
        1_000_000_000 / cfg.rate.max(1)
    };
    let total = cfg.warmup + cfg.samples;
    let mut fatal = None;
    let (mut seq, mut lost, mut measured, mut warmed) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut measure_start = 0_u64;
    let budget_ns = u64::try_from(cfg.duration.as_nanos()).unwrap_or(u64::MAX);
    let mut ended = "samples";
    let mut next_due = clock.now();
    while seq < total {
        if shared.stopping() {
            ended = "stop";
            break;
        }
        if image.is_closed() {
            ended = "peer closed";
            break;
        }
        if seq == cfg.warmup {
            measure_start = clock.now();
            gap.restart();
        }
        if seq >= cfg.warmup
            && budget_ns > 0
            && clock.now().saturating_sub(measure_start) > budget_ns
        {
            ended = "duration";
            break;
        }
        seq += 1;
        // Open loop: wait for this message's due time, spinning.
        let due = if period_ns == 0 {
            0
        } else {
            while clock.now() < next_due {
                gap.pass(clock.now());
                spin_loop();
            }
            let due = next_due;
            next_due = next_due.wrapping_add(period_ns);
            due
        };
        let sent = clock.stamp();
        let t0 = if period_ns == 0 { sent } else { due };
        msg[..8].copy_from_slice(&seq.to_le_bytes());
        msg[8..16].copy_from_slice(&t0.to_le_bytes());
        if !offer(
            &session.publication,
            &msg,
            clock,
            shared,
            &mut warn,
            cfg.backpressure_warn_ns,
            &mut fatal,
        ) {
            ended = "stop";
            break;
        }
        let mut echoed = None;
        while echoed.is_none() {
            let now = clock.now();
            gap.pass(now);
            let fragments = image
                .poll_fn(
                    |reply, _header| {
                        if reply.len() >= MIN_MSG_BYTES && reply[..8] == seq.to_le_bytes() {
                            let mut stamp = [0_u8; 8];
                            stamp.copy_from_slice(&reply[8..16]);
                            echoed = Some((u64::from_le_bytes(stamp), clock.stamp()));
                        }
                    },
                    FRAGMENT_LIMIT,
                )
                .map_err(|e| aeron_err("poll", e))?;
            if fragments == 0 && echoed.is_none() {
                if now.saturating_sub(sent) > timeout_ns {
                    lost += 1;
                    Counters::bump(&shared.counters.lost);
                    if let Some(suppressed) = warn.echo_timeout.check(now) {
                        tracing::warn!(seq, timeout_ns, suppressed, "echo timeout");
                        Counters::bump(&shared.counters.warn_events);
                    }
                    break;
                }
                if image.is_closed() {
                    break;
                }
                spin_loop();
            }
        }
        if let Some((t0, t1)) = echoed {
            if seq > cfg.warmup {
                latency.record(t1.saturating_sub(t0), || clock.realtime(t1));
                measured += 1;
                Counters::bump(&shared.counters.messages);
                Counters::set(&shared.counters.max_gap_ns, gap.max_ns);
            } else {
                warmed += 1;
            }
        }
    }
    if let Some(e) = fatal {
        return Err(e);
    }
    Ok(HotOutcome {
        messages: measured,
        warmed,
        lost,
        rtt: Some(latency.percentiles()),
        top: latency.top(),
        gap,
        warn_events: warn.emitted(),
        suppressed: warn.suppressed(),
        ended: ended.to_owned(),
        ..HotOutcome::default()
    })
}

fn run_pong(
    cfg: &AppConfig,
    session: &Session,
    image: &AeronImage,
    clock: &Clock,
    shared: &Shared,
) -> Result<HotOutcome, LlError> {
    let mut gap = LoopGap::default();
    let mut warn = Warnings::new();
    let mut echo = vec![0_u8; MAX_MSG_BYTES];
    let mut fatal = None;
    let mut echoed = 0_u64;
    let ended;
    loop {
        if shared.stopping() {
            ended = "stop";
            break;
        }
        if image.is_closed() {
            ended = "peer closed";
            break;
        }
        gap.pass(clock.now());
        let mut len = 0;
        let fragments = image
            .poll_fn(
                |ping, _header| {
                    len = ping.len().min(echo.len());
                    echo[..len].copy_from_slice(&ping[..len]);
                },
                1,
            )
            .map_err(|e| aeron_err("poll", e))?;
        if fragments == 0 {
            spin_loop();
            continue;
        }
        if !offer(
            &session.publication,
            &echo[..len],
            clock,
            shared,
            &mut warn,
            cfg.backpressure_warn_ns,
            &mut fatal,
        ) {
            ended = "stop";
            break;
        }
        echoed += 1;
        Counters::bump(&shared.counters.messages);
        Counters::set(&shared.counters.max_gap_ns, gap.max_ns);
    }
    if let Some(e) = fatal {
        return Err(e);
    }
    Ok(HotOutcome {
        messages: echoed,
        gap,
        warn_events: warn.emitted(),
        suppressed: warn.suppressed(),
        ended: ended.to_owned(),
        ..HotOutcome::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(vars: &[(&str, &str)]) -> Result<AppConfig, LlError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        AppConfig::from_lookup(|k| map.get(k).cloned())
    }

    const BASE: [(&str, &str); 3] = [
        ("LL_ROLE", "ping"),
        ("LL_PUB_CHANNEL", "aeron:udp?endpoint=10.9.0.3:40001"),
        ("LL_SUB_CHANNEL", "aeron:udp?endpoint=10.9.0.3:40002"),
    ];

    #[test]
    fn defaults_and_streams() -> Result<(), LlError> {
        let c = cfg(&BASE)?;
        assert_eq!(c.role, Role::Ping);
        assert_eq!((c.pub_stream(), c.sub_stream()), (1001, 1002));
        assert_eq!(
            (c.msg_bytes, c.warmup, c.samples, c.rate),
            (32, 100_000, 1_000_000, 0)
        );
        assert_eq!(c.hot_spec, Spec::Isolated(0));
        assert_eq!(c.log_level, LevelFilter::INFO);
        let mut pong = BASE.to_vec();
        pong[0] = ("LL_ROLE", "pong");
        let p = cfg(&pong)?;
        assert_eq!((p.pub_stream(), p.sub_stream()), (1002, 1001));
        Ok(())
    }

    #[test]
    fn refuses_bad_settings() {
        assert!(matches!(
            cfg(&BASE[1..]),
            Err(LlError::MissingEnv("LL_ROLE"))
        ));
        assert!(matches!(
            cfg(&BASE[..2]),
            Err(LlError::MissingEnv("LL_SUB_CHANNEL"))
        ));
        for (k, v) in [
            ("LL_ROLE", "pang"),
            ("LL_MSG_BYTES", "8"),
            ("LL_MSG_BYTES", "4096"),
            ("LL_SAMPLES", "many"),
            ("LL_LOG_LEVEL", "loud"),
            ("LL_LOG_QUEUE_LINES", "0"),
            ("LL_HOT_CPU", "core:1"),
        ] {
            let mut vars = BASE.to_vec();
            vars.retain(|(name, _)| *name != k);
            vars.push((k, v));
            assert!(cfg(&vars).is_err(), "{k}={v} accepted");
        }
    }

    #[test]
    fn the_clock_moves_forward() {
        let c = Clock::new();
        let a = c.stamp();
        let b = c.stamp();
        assert!(b >= a);
        assert!(c.realtime(b) >= c.realtime(a));
    }
}
