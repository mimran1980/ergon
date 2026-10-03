//! The runtime owns the loop: an application implements [`Agent`], and
//! [`Runtime`] drives it on one thread, Agrona-style.
//!
//! ```text
//! duty cycle: poll every feed (up to `limit` each) -> fire due timers
//!             -> Aeron conductor (invoker mode) -> idle(work)
//! ```
//!
//! [`Ctx`] is everything the agent reaches: the event time, the clock, timers,
//! feeds and outputs, metrics and persist. No Aeron or `ClickHouse` type
//! appears in it. Housekeeping (`Persist::poll`, the wall-clock offset, the
//! `Source` heartbeat, `streams.yaml`, SIGTERM) runs on runtime-owned timers
//! in the same wheel, never as a per-cycle check, and only in a cycle that
//! found no work unless it has waited too long.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusteron_archive::{AeronBufferClaim, AeronExclusivePublication};

use crate::Error;
use crate::bus::{Bus, DropKind};
use crate::clock::{Clock, Nanos};
use crate::idle::Idle;
use crate::metrics::Metrics;
use crate::persist::Persist;
use crate::streams::{Streams, Watch};
use crate::subscription::{Delivery, PersistentSubscription, Subscription};
use crate::timer::{self, Fired, TimerError, TimerId, TimerWheel};
use crate::trace::Tracer;

/// A feed this agent subscribed to, as numbered by [`Ctx::subscribe`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeedId(pub u32);

/// An output stream this agent publishes, as returned by [`Ctx::publish`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Out(u32);

/// A timer of this agent's that came due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expiry {
    /// The id [`Ctx::cancel`] takes; a repeating timer keeps it.
    pub id: TimerId,
    /// The token the agent scheduled it with.
    pub token: u64,
    /// When it was due, which is not when the poll found it in live mode.
    pub deadline: Nanos,
    /// Whole periods a repeating timer skipped because the loop stalled.
    /// Always 0 for a one-shot, and in simulation.
    pub missed: u64,
}

/// An application on the runtime. Every method runs on the runtime's one
/// thread; none may block, and none should panic (a panic in a callback
/// aborts the process inside Aeron).
pub trait Agent {
    /// Once, before the first event: open feeds, schedule timers.
    ///
    /// # Errors
    ///
    /// The agent could not start; the runtime stops.
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), Error>;

    /// One message of `feed`. [`Ctx::now`] is its event time.
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, msg: &[u8], delivery: Delivery);

    /// A timer this agent scheduled is due. [`Ctx::now`] is the time of the
    /// poll that found it (live) or its deadline (simulation).
    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry);

    /// Work not driven by an event, once per duty cycle. Returns the work
    /// count for the idle strategy.
    fn do_work(&mut self, _ctx: &mut Ctx) -> usize {
        0
    }

    /// `streams.yaml` changed (live only); [`Ctx::streams`] is the new
    /// registry. Open feeds it added.
    fn on_streams(&mut self, _ctx: &mut Ctx) {}

    /// Once, after the last event, before the feeds close.
    fn stop(&mut self, _ctx: &mut Ctx) {}
}

/// Two agents in one thread: every message and timer goes to both, `A`
/// first. Their timer tokens must not collide.
impl<A: Agent, B: Agent> Agent for (A, B) {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), Error> {
        self.0.start(ctx)?;
        self.1.start(ctx)
    }

    #[inline]
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, msg: &[u8], delivery: Delivery) {
        self.0.on_message(ctx, feed, msg, delivery);
        self.1.on_message(ctx, feed, msg, delivery);
    }

    #[inline]
    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        self.0.on_timer(ctx, timer);
        self.1.on_timer(ctx, timer);
    }

    fn do_work(&mut self, ctx: &mut Ctx) -> usize {
        self.0.do_work(ctx) + self.1.do_work(ctx)
    }

    fn on_streams(&mut self, ctx: &mut Ctx) {
        self.0.on_streams(ctx);
        self.1.on_streams(ctx);
    }

    fn stop(&mut self, ctx: &mut Ctx) {
        self.0.stop(ctx);
        self.1.stop(ctx);
    }
}

/// Timer tokens with this bit set are the runtime's own.
const RUNTIME_TOKEN: u64 = 1 << 63;
const HK_PERSIST: u64 = RUNTIME_TOKEN;
const HK_WALL: u64 = RUNTIME_TOKEN | 1;
const HK_SOURCE: u64 = RUNTIME_TOKEN | 2;
const HK_STREAMS: u64 = RUNTIME_TOKEN | 3;
#[cfg(feature = "mimalloc")]
const HK_ALLOC: u64 = RUNTIME_TOKEN | 4;

const MS: i64 = 1_000_000;
const SECOND: i64 = 1_000_000_000;

/// How the runtime is built: the bus and persist it runs on, and the knobs.
pub struct Config {
    /// The Aeron client feeds and outputs are opened on.
    pub bus: Bus,
    /// Rows, metrics and traces; `None` records nothing.
    pub persist: Option<Persist>,
    /// The feed registry [`Ctx::subscribe`] and [`Ctx::publish`] resolve names in.
    pub streams: Streams,
    /// Followed for new feeds, if set.
    pub streams_path: Option<String>,
    /// `REGION`.
    pub region: String,
    /// What a cycle that found no work does.
    pub idle: Idle,
    /// Messages taken from one feed per cycle.
    pub limit: usize,
    /// The timer wheel's shape.
    pub timers: timer::Settings,
    /// Set (by SIGTERM) to stop the loop.
    pub stop: Arc<AtomicBool>,
    /// Pin the loop's thread to this CPU (Linux).
    pub cpu: Option<usize>,
    /// `mlockall(MCL_CURRENT | MCL_FUTURE)` at start, so the first live
    /// message takes no page fault (Linux).
    pub lock_memory: bool,
}

impl Config {
    /// Defaults on `bus` and `streams`: no persist, `spin`, 64 messages a
    /// feed per cycle, the default wheel, no pinning or locking.
    #[must_use]
    pub fn new(bus: Bus, streams: Streams) -> Self {
        Self {
            bus,
            persist: None,
            streams,
            streams_path: None,
            region: String::new(),
            idle: Idle::Spin,
            limit: 64,
            timers: timer::Settings::default(),
            stop: Arc::new(AtomicBool::new(false)),
            cpu: None,
            lock_memory: false,
        }
    }
}

/// One opened feed.
enum Feed {
    Live(Subscription),
    Persistent(PersistentSubscription),
}

impl Feed {
    #[inline]
    fn poll(&mut self, handler: impl FnMut(&[u8], Delivery), limit: usize) -> usize {
        match self {
            Self::Live(s) => s.poll(handler, limit),
            Self::Persistent(s) => s.poll(handler, limit),
        }
    }
}

/// Where an [`Out`] goes.
struct Sink {
    publication: AeronExclusivePublication,
    max_payload: usize,
}

/// What the agent reaches: time, timers, feeds, outputs, metrics, persist.
pub struct Ctx {
    now: Nanos,
    sim: bool,
    clock: Clock,
    /// Wall clock less [`Clock::read`], re-measured every second.
    wall_offset: i64,
    wheel: TimerWheel,
    fired: Vec<Fired>,
    bus: Bus,
    persist: Option<Persist>,
    metrics: Metrics,
    streams: Streams,
    region: String,
    sinks: Vec<Sink>,
    /// Feeds opened since the last cycle; the runtime adopts them.
    opened: Vec<Feed>,
    feeds: u32,
    next_id: u64,
    stopping: bool,
}

impl Ctx {
    /// The event time: the time this message or timer is handled at. A load.
    #[inline]
    #[must_use]
    pub const fn now(&self) -> Nanos {
        self.now
    }

    /// The clock now, for latency marks inside one event: a TSC read live,
    /// [`Ctx::now`] in simulation.
    #[inline]
    #[must_use]
    pub fn read(&self) -> Nanos {
        if self.sim {
            self.now
        } else {
            self.clock.read()
        }
    }

    /// The wall clock now: a TSC read plus the cached offset, no system
    /// call. For code outside a dispatch, such as a callback another
    /// framework drives; inside one, [`Ctx::wall_ns`] is the event's.
    #[inline]
    #[must_use]
    pub fn wall_now(&self) -> Nanos {
        Nanos(self.read().0 + self.wall_offset)
    }

    /// The event time on the wall clock, for stamping what another process
    /// compares with its own clock. No system call.
    #[inline]
    #[must_use]
    pub const fn wall_ns(&self) -> Nanos {
        Nanos(self.now.0 + self.wall_offset)
    }

    /// The event-time instant of `epoch_ns`, a timestamp another process
    /// took by its wall clock: comparable with [`Ctx::now`]. No system call.
    #[inline]
    #[must_use]
    pub const fn from_remote(&self, epoch_ns: i64) -> Nanos {
        Nanos(epoch_ns - self.wall_offset)
    }

    /// Running on simulated time (replay or backtest).
    #[inline]
    #[must_use]
    pub const fn is_sim(&self) -> bool {
        self.sim
    }

    /// A process-unique, run-deterministic id: a per-run sequence in
    /// simulation, the start time plus a counter live.
    #[inline]
    pub const fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// `REGION`.
    #[must_use]
    pub fn region(&self) -> &str {
        &self.region
    }

    /// The feed registry.
    #[must_use]
    pub const fn streams(&self) -> &Streams {
        &self.streams
    }

    /// This application's metrics.
    #[must_use]
    pub const fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Rows, metrics and traces, when the runtime records them.
    #[must_use]
    pub const fn persist(&self) -> Option<&Persist> {
        self.persist.as_ref()
    }

    /// A tracer on [`Ctx::persist`], or one that publishes nothing.
    #[must_use]
    pub fn tracer(&self, name: &str, stages: &[&str], attrs: &[&str]) -> Tracer {
        self.persist.as_ref().map_or_else(
            || crate::persist::tracer(name, stages, attrs),
            |p| p.tracer(name, stages, attrs),
        )
    }

    /// Stop after this event: `stop`, then the feeds close.
    pub const fn stop(&mut self) {
        self.stopping = true;
    }

    /// Subscribe to `service`'s `kind` stream through its archive, catching
    /// up after a restart or a slow spell.
    ///
    /// # Errors
    ///
    /// `service` or `kind` is not in the registry.
    pub fn subscribe(&mut self, service: &str, kind: &str) -> Result<FeedId, Error> {
        let feed = Feed::Persistent(self.bus.subscribe(&self.streams, service, kind)?);
        Ok(self.adopt(feed))
    }

    /// [`Ctx::subscribe`], but each new recording replays from its start: for
    /// a consumer that must see what its publisher sent while it was down,
    /// such as an exchange's orders.
    ///
    /// # Errors
    ///
    /// `service` or `kind` is not in the registry.
    pub fn subscribe_from_start(&mut self, service: &str, kind: &str) -> Result<FeedId, Error> {
        let feed = self
            .bus
            .subscribe(&self.streams, service, kind)?
            .from_start();
        Ok(self.adopt(Feed::Persistent(feed)))
    }

    /// Subscribe to `service`'s `kind` stream off the network only: best
    /// effort, nothing replayed.
    ///
    /// # Errors
    ///
    /// `service` or `kind` is not in the registry.
    pub fn subscribe_live(&mut self, service: &str, kind: &str) -> Result<FeedId, Error> {
        let feed = Feed::Live(self.bus.subscribe_live(&self.streams, service, kind)?);
        Ok(self.adopt(feed))
    }

    /// Subscribe to a raw `channel` and `stream_id`, off the network.
    pub fn subscribe_channel(&mut self, channel: &str, stream_id: i32) -> FeedId {
        let feed = Feed::Live(self.bus.subscription(channel, stream_id));
        self.adopt(feed)
    }

    fn adopt(&mut self, feed: Feed) -> FeedId {
        self.opened.push(feed);
        let id = FeedId(self.feeds);
        self.feeds += 1;
        id
    }

    /// Publish `service`'s `kind` stream from this node, from this thread
    /// alone (an exclusive publication).
    ///
    /// # Errors
    ///
    /// The registry does not name it, or the driver did not add it.
    pub fn publish(&mut self, service: &str, kind: &str) -> Result<Out, Error> {
        let channel = self.streams.publication(service, self.bus.host_ip())?;
        let stream_id = self.streams.stream(service, kind)?;
        self.publish_channel(&channel, stream_id)
    }

    /// Publish on a raw `channel` and `stream_id`, from this thread alone.
    ///
    /// # Errors
    ///
    /// The driver did not add the publication.
    pub fn publish_channel(&mut self, channel: &str, stream_id: i32) -> Result<Out, Error> {
        let publication = self.bus.add_exclusive_publication(channel, stream_id)?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        let out = Out(u32::try_from(self.sinks.len()).unwrap_or(u32::MAX));
        self.sinks.push(Sink {
            publication,
            max_payload,
        });
        self.send_source(out);
        Ok(out)
    }

    /// Publish one message of exactly `len` bytes (header included) that
    /// `encode` writes into the claimed frame, zero-copy. A frame Aeron does
    /// not take is counted in [`Bus::drops`] and returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// The error from `encode`; the claim is aborted.
    #[inline]
    pub fn send<E>(
        &self,
        out: Out,
        template_id: u16,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        let Some(sink) = self.sinks.get(out.0 as usize) else {
            self.bus.count(DropKind::Other);
            return Ok(());
        };
        if len > sink.max_payload {
            self.bus.count(DropKind::TooLarge);
            return Ok(());
        }
        let claim = match self.bus.try_claim_exclusive(&sink.publication, len) {
            Ok(claim) => claim,
            Err(kind) => {
                self.bus.count(kind);
                return Ok(());
            }
        };
        let slot = claim.data();
        let written = encode(slot)?;
        debug_assert_eq!(
            slot.get(2..4),
            Some(&template_id.to_le_bytes()[..]),
            "encode wrote another template than {template_id}"
        );
        // A wrong length would corrupt the stream: one compare, kept.
        if written != len || claim.commit().is_err() {
            self.bus.drop_one();
        }
        Ok(())
    }

    /// `out` is connected: a subscriber or the archive takes it.
    #[must_use]
    pub fn is_connected(&self, out: Out) -> bool {
        self.sinks
            .get(out.0 as usize)
            .is_some_and(|s| s.publication.is_connected())
    }

    /// The longest message one frame of `out` holds.
    #[must_use]
    pub fn max_payload(&self, out: Out) -> usize {
        self.sinks.get(out.0 as usize).map_or(0, |s| s.max_payload)
    }

    #[cold]
    fn send_source(&self, out: Out) {
        let Some(sink) = self.sinks.get(out.0 as usize) else {
            return;
        };
        let message = self.bus.source_message();
        if let Ok(claim) = self
            .bus
            .try_claim_exclusive(&sink.publication, message.len())
        {
            claim.data().copy_from_slice(message);
            let _ = claim.commit();
        }
    }

    /// One-shot, `delay` ns after [`Ctx::now`].
    ///
    /// # Errors
    ///
    /// See [`TimerWheel::schedule`].
    #[inline]
    pub fn after(&mut self, delay: i64, token: u64) -> Result<TimerId, TimerError> {
        self.at(Nanos(self.now.0.saturating_add(delay)), token)
    }

    /// One-shot at `deadline`.
    ///
    /// # Errors
    ///
    /// See [`TimerWheel::schedule`].
    #[inline]
    pub fn at(&mut self, deadline: Nanos, token: u64) -> Result<TimerId, TimerError> {
        debug_assert!(
            token & RUNTIME_TOKEN == 0,
            "token {token:#x} uses the runtime's bit"
        );
        self.wheel.schedule(deadline, token & !RUNTIME_TOKEN)
    }

    /// Repeating, first one `period` from now.
    ///
    /// # Errors
    ///
    /// See [`TimerWheel::schedule_repeating`].
    pub fn every(&mut self, period: i64, token: u64) -> Result<TimerId, TimerError> {
        self.every_from(Nanos(self.now.0.saturating_add(period)), period, token)
    }

    /// Repeating, first at `first`.
    ///
    /// # Errors
    ///
    /// See [`TimerWheel::schedule_repeating`].
    pub fn every_from(
        &mut self,
        first: Nanos,
        period: i64,
        token: u64,
    ) -> Result<TimerId, TimerError> {
        debug_assert!(
            token & RUNTIME_TOKEN == 0,
            "token {token:#x} uses the runtime's bit"
        );
        self.wheel
            .schedule_repeating(first, period, token & !RUNTIME_TOKEN)
    }

    /// Repeating on whole multiples of `period` in epoch time, so
    /// `every_aligned(1 s)` fires on the second in live and in simulation.
    ///
    /// # Errors
    ///
    /// See [`TimerWheel::schedule_aligned`].
    pub fn every_aligned(&mut self, period: i64, token: u64) -> Result<TimerId, TimerError> {
        let first = timer::aligned_deadline(self.now.0.saturating_add(1), period)?;
        self.every_from(Nanos(first), period, token)
    }

    /// Cancel a one-shot or repeating timer, also from inside `on_timer`
    /// (a timer still to come in the same batch then does not fire).
    #[inline]
    pub fn cancel(&mut self, id: TimerId) -> bool {
        self.wheel.cancel(id)
    }

    fn schedule_runtime(&mut self, period: i64, token: u64) {
        let Ok(first) = timer::aligned_deadline(self.now.0.saturating_add(1), period) else {
            return;
        };
        if let Err(e) = self.wheel.schedule_repeating(Nanos(first), period, token) {
            log::error!("runtime timer {token:#x}: {e}");
        }
    }
}

/// The runtime: feeds, the agent's context, and the duty cycle.
pub struct Runtime {
    feeds: Vec<Feed>,
    ctx: Ctx,
    idle: Idle,
    limit: usize,
    stop: Arc<AtomicBool>,
    watch: Option<Watch>,
    /// The Aeron conductor runs in this loop.
    invoker: bool,
    cpu: Option<usize>,
    lock_memory: bool,
    #[cfg(feature = "mimalloc")]
    alloc: Option<crate::alloc_stats::AllocStats>,
    /// When the conductor last ran: it waits for a cycle with no work, or a
    /// millisecond, so it never delays a message already waiting.
    conductor_ran: Nanos,
    /// Housekeeping due in a cycle that had work: run at the next idle one,
    /// or once it has waited `MAX_DEFER`.
    deferred: u8,
    deferred_since: Nanos,
}

/// The longest housekeeping waits behind busy cycles.
const MAX_DEFER: i64 = MS / 2;

impl Runtime {
    /// A live runtime on `config`.
    ///
    /// # Errors
    ///
    /// The timer settings are not powers of two.
    pub fn new(config: Config) -> Result<Self, Error> {
        let mut wheel = TimerWheel::new(config.timers)
            .map_err(|e| Error::Config(format!("timer wheel: {e}")))?;
        wheel.prefault();
        let clock = Clock::new();
        let now = clock.now();
        let metrics = config
            .persist
            .as_ref()
            .map_or_else(Metrics::detached, Persist::metrics);
        let ctx = Ctx {
            now,
            sim: false,
            wall_offset: clock.wall_offset(),
            clock,
            fired: Vec::with_capacity(64),
            wheel,
            bus: config.bus,
            persist: config.persist,
            metrics,
            streams: config.streams,
            region: config.region,
            sinks: Vec::new(),
            opened: Vec::new(),
            feeds: 0,
            next_id: now.0.cast_unsigned() & !RUNTIME_TOKEN,
            stopping: false,
        };
        Ok(Self {
            invoker: ctx.bus.is_invoker(),
            conductor_ran: Nanos(0),
            feeds: Vec::new(),
            ctx,
            idle: config.idle,
            limit: config.limit,
            stop: config.stop,
            watch: config.streams_path.map(Watch::new),
            cpu: config.cpu,
            lock_memory: config.lock_memory,
            #[cfg(feature = "mimalloc")]
            alloc: None,
            deferred: 0,
            deferred_since: Nanos(0),
        })
    }

    /// From the environment, as [`crate::app::App::start`]: logging, the
    /// node check, the bus and persist for `schema`, `streams.yaml`,
    /// `IDLE` (default `spin`), and SIGTERM.
    ///
    /// # Errors
    ///
    /// As [`crate::app::App::start`].
    pub fn from_env(schema: &str) -> Result<Self, crate::app::Error> {
        let app = crate::app::App::start_with(schema, Idle::Spin, true)?;
        let config = Config {
            persist: Some(app.persist),
            streams_path: Some(app.streams_path),
            region: app.region,
            idle: app.idle,
            stop: app.stop,
            cpu: std::env::var("CPU").ok().and_then(|c| c.parse().ok()),
            lock_memory: std::env::var("MLOCK").is_ok_and(|v| v == "1" || v == "true"),
            ..Config::new(app.bus, app.streams)
        };
        Ok(Self::new(config)?)
    }

    /// The agent, SIGTERM or [`Ctx::stop`] asked the loop to stop: a loop
    /// that drives [`Runtime::cycle`] itself stops here.
    #[must_use]
    pub const fn is_stopping(&self) -> bool {
        self.ctx.stopping
    }

    /// The agent's context, read-only: [`Ctx::send`] needs no more.
    #[must_use]
    pub const fn ctx_ref(&self) -> &Ctx {
        &self.ctx
    }

    /// The agent's context, for set-up outside a callback.
    pub const fn ctx(&mut self) -> &mut Ctx {
        &mut self.ctx
    }

    /// Start `agent` and the runtime's housekeeping timers.
    ///
    /// # Errors
    ///
    /// The agent's [`Agent::start`] failed.
    pub fn start<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        crate::os::check_clock();
        if let Some(cpu) = self.cpu {
            crate::os::pin_thread(cpu);
        }
        if self.lock_memory {
            crate::os::lock_memory();
        }
        #[cfg(feature = "mimalloc")]
        {
            self.alloc = Some(crate::alloc_stats::AllocStats::new(&self.ctx.metrics));
            self.ctx.schedule_runtime(5 * SECOND, HK_ALLOC);
        }
        self.ctx.now = self.ctx.clock.now();
        agent.start(&mut self.ctx)?;
        self.adopt();
        self.ctx.schedule_runtime(MS, HK_PERSIST);
        self.ctx.schedule_runtime(SECOND, HK_WALL);
        self.ctx.schedule_runtime(5 * SECOND, HK_SOURCE);
        if self.watch.is_some() {
            self.ctx.schedule_runtime(SECOND, HK_STREAMS);
        }
        Ok(())
    }

    /// Run `agent` until it or SIGTERM stops it, then close the feeds.
    ///
    /// # Errors
    ///
    /// The agent's [`Agent::start`] failed.
    pub fn run<A: Agent>(mut self, mut agent: A) -> Result<A, Error> {
        self.start(&mut agent)?;
        while !self.ctx.stopping {
            let work = self.cycle(&mut agent);
            self.idle.idle(work);
        }
        self.finish(&mut agent);
        Ok(agent)
    }

    /// Stop `agent` and close the feeds; [`Runtime::run`] does this.
    pub fn finish<A: Agent>(&mut self, agent: &mut A) {
        agent.stop(&mut self.ctx);
        log::info!("stopping: closing the feeds");
        self.ctx.bus.shutdown();
    }

    /// One duty cycle: every feed, then due timers. Returns the work count.
    /// [`Runtime::run`] calls it in a loop; an embedding loop (a callback
    /// another framework owns) calls it directly.
    #[inline]
    pub fn cycle<A: Agent>(&mut self, agent: &mut A) -> usize {
        let mut work = 0;
        let ctx = &mut self.ctx;
        for (i, feed) in self.feeds.iter_mut().enumerate() {
            let id = FeedId(u32::try_from(i).unwrap_or(u32::MAX));
            work += feed.poll(
                |msg, delivery| {
                    ctx.now = ctx.clock.now();
                    agent.on_message(ctx, id, msg, delivery);
                },
                self.limit,
            );
        }
        work += agent.do_work(&mut self.ctx);
        if !self.ctx.opened.is_empty() {
            self.adopt();
        }
        self.ctx.now = self.ctx.clock.now();
        if self.ctx.wheel.poll(self.ctx.now, &mut self.ctx.fired, 64) > 0 {
            work += self.fire(agent, work);
        }
        if self.invoker && (work == 0 || self.ctx.now.since(self.conductor_ran) > MS) {
            self.conductor_ran = self.ctx.now;
            work += self.ctx.bus.do_work();
        }
        if self.deferred != 0 && (work == 0 || self.ctx.now.since(self.deferred_since) > MAX_DEFER)
        {
            self.run_deferred(agent);
        }
        work
    }

    fn adopt(&mut self) {
        self.feeds.append(&mut self.ctx.opened);
    }

    #[cold]
    #[inline(never)]
    fn fire<A: Agent>(&mut self, agent: &mut A, busy: usize) -> usize {
        let fired = std::mem::take(&mut self.ctx.fired);
        let mut n = 0;
        for f in &fired {
            if self.ctx.wheel.is_suppressed(f.id) {
                continue;
            }
            if f.token & RUNTIME_TOKEN != 0 {
                self.housekeeping(agent, f.token, busy);
                continue;
            }
            n += 1;
            agent.on_timer(
                &mut self.ctx,
                Expiry {
                    id: f.id,
                    token: f.token,
                    deadline: f.deadline,
                    missed: f.missed,
                },
            );
        }
        self.ctx.fired = fired;
        n
    }

    fn housekeeping<A: Agent>(&mut self, agent: &mut A, token: u64, busy: usize) {
        let bit = 1u8 << (token & 7);
        if busy > 0 {
            if self.deferred == 0 {
                self.deferred_since = self.ctx.now;
            }
            self.deferred |= bit;
            return;
        }
        self.run_one(agent, token);
    }

    #[cold]
    fn run_deferred<A: Agent>(&mut self, agent: &mut A) {
        let deferred = std::mem::take(&mut self.deferred);
        for i in 0..8 {
            if deferred & (1 << i) != 0 {
                self.run_one(agent, RUNTIME_TOKEN | i);
            }
        }
    }

    fn run_one<A: Agent>(&mut self, agent: &mut A, token: u64) {
        match token {
            HK_PERSIST => {
                if self.stop.load(Ordering::Relaxed) {
                    log::info!("SIGTERM");
                    self.ctx.stopping = true;
                }
                if let Some(p) = &self.ctx.persist {
                    p.poll(self.ctx.now);
                }
            }
            HK_WALL => self.ctx.wall_offset = self.ctx.clock.wall_offset(),
            #[cfg(feature = "mimalloc")]
            HK_ALLOC => {
                if let Some(alloc) = &mut self.alloc {
                    alloc.sample();
                }
            }
            HK_SOURCE => {
                for i in 0..self.ctx.sinks.len() {
                    self.ctx
                        .send_source(Out(u32::try_from(i).unwrap_or(u32::MAX)));
                }
            }
            HK_STREAMS => {
                if let Some(streams) = self.watch.as_mut().and_then(Watch::changed) {
                    self.ctx.streams = streams;
                    agent.on_streams(&mut self.ctx);
                    self.adopt();
                }
            }
            _ => {}
        }
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("feeds", &self.feeds.len())
            .field("now", &self.ctx.now)
            .finish_non_exhaustive()
    }
}

/// A flag SIGTERM sets: [`Config::stop`] for a runtime built by hand.
///
/// # Errors
///
/// The handler could not be registered.
pub fn sigterm() -> Result<Arc<AtomicBool>, std::io::Error> {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop))?;
    Ok(stop)
}

/// Write `message` on an exclusive publication, stamped as [`Bus`] stamps.
impl Bus {
    /// Claim `len` bytes of an exclusive `publication`: no CAS on the term
    /// tail and no shutdown check (the publication is closed under it).
    #[inline]
    pub(crate) fn try_claim_exclusive(
        &self,
        publication: &AeronExclusivePublication,
        len: usize,
    ) -> Result<crate::bus::Claim, DropKind> {
        let claim = AeronBufferClaim::new_zeroed_on_stack();
        crate::bus::retry_admin(|| publication.try_claim(len, &claim))
            .map_err(|err| crate::bus::classify(&err))?;
        claim.frame_header_mut().reserved_value = self.source().id.cast_signed();
        Ok(crate::bus::Claim::new(claim))
    }
}
