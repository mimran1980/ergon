//! The runtime owns the loop: an application implements [`Agent`], and
//! [`Runtime`] drives it on one thread, Agrona-style.
//!
//! ```text
//! duty cycle: poll every feed (up to `limit` each) -> agent.poll -> fire due timers
//!             -> Aeron conductor (when the cycle found no work, or 1 ms after its last run)
//!             -> idle(work)
//! ```
//!
//! [`Ctx`] is everything the agent reaches: the event time, the clock, timers,
//! feeds and outputs, metrics and persist. No Aeron or `ClickHouse` type
//! appears in it. It owns the [`Bus`], whose client has no thread of its
//! own: the duty cycle runs its conductor in a cycle that found no work, or
//! once a millisecond has passed. A persistent feed runs it too, and polls
//! its archive client, at each poll before it takes a message (Aeron's
//! persistent subscription does both, the first when the client has no
//! conductor thread): each persistent feed adds both to every cycle, ahead
//! of the messages it and the feeds after it deliver.
//! Housekeeping (`Persist::poll`, the wall-clock offset, the `Source`
//! heartbeat, SIGTERM) runs on runtime-owned timers in the same wheel, never
//! as a per-cycle check, and only in a cycle that found no work unless it
//! has waited too long.
//!
//! SIGTERM is a byte on a pipe ([`sigterm`]): the handler writes it, and a
//! housekeeping timer reads the pipe every 10 ms and stops the loop.
//! [`Invoker::finish`] then stops the agent and closes what the application
//! owns, its outputs and persist's publication, at once, so subscribers turn
//! to the next publisher within seconds.

use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::io::Read;
use std::os::unix::net::UnixStream;

use rusteron_archive::AeronExclusivePublication;

use crate::Error;
use crate::bus::{Bus, DropKind, Drops};
use crate::clock::{Clock, Nanos};
use crate::directory::{Directory, NoDirectory};
use crate::frames::FrameLog;
use crate::idle::Idle;
use crate::journal::{Input, InputEvent};
use crate::metrics::Metrics;
use crate::persist::Persist;
use crate::subscription::{Delivery, PersistentSubscription, Subscription};
use crate::timer::{self, Fired, TimerError, TimerId, TimerWheel};
use crate::trace::Tracer;

mod archive;
mod driver;
pub mod sim;

pub use archive::ArchiveConfig;
pub use driver::{Mode, Runtime};

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
    fn poll(&mut self, _ctx: &mut Ctx) -> usize {
        0
    }
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

    fn poll(&mut self, ctx: &mut Ctx) -> usize {
        self.0.poll(ctx) + self.1.poll(ctx)
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
const HK_SIGNAL: u64 = RUNTIME_TOKEN | 3;
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
    /// Names to addresses for [`Ctx::subscribe`] and [`Ctx::publish`], as
    /// the application defines them.
    pub directory: Box<dyn Directory>,
    /// Where this application runs (`REGION`); `unknown` when unset.
    pub region: String,
    /// What a cycle that found no work does.
    pub idle: Idle,
    /// Messages taken from one feed per cycle.
    pub limit: usize,
    /// The timer wheel's shape.
    pub timers: timer::Settings,
    /// What stops the loop from outside: [`sigterm`], or [`Stop::none`].
    pub stop: Stop,
    /// Pin the loop's thread to this CPU (Linux).
    pub cpu: Option<usize>,
    /// `mlockall(MCL_CURRENT | MCL_FUTURE)` at start, so the first live
    /// message takes no page fault (Linux).
    pub lock_memory: bool,
    /// How late the kernel may wake the loop thread's sleeps and parks
    /// (`TIMER_SLACK`); `None` keeps Linux's 50 µs. Linux only.
    pub timer_slack: Option<std::time::Duration>,
    /// Record actual dispatches to Persist for exact replay (`JOURNAL=on`).
    pub journal: bool,
}

impl Config {
    /// Defaults on `bus`: its region, no directory (open feeds by channel),
    /// no persist, `spin`, 64 messages a feed per cycle, the default wheel,
    /// no SIGTERM, no pinning, locking or timer slack.
    #[must_use]
    pub fn new(bus: Bus) -> Self {
        Self {
            region: bus.region().to_owned(),
            bus,
            persist: None,
            directory: Box::new(NoDirectory),
            idle: Idle::Spin,
            limit: 64,
            timers: timer::Settings::default(),
            stop: Stop::none(),
            cpu: None,
            lock_memory: false,
            timer_slack: None,
            journal: false,
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

    #[inline]
    fn poll_metadata(
        &mut self,
        handler: impl FnMut(&[u8], Delivery, crate::subscription::Metadata),
        limit: usize,
    ) -> usize {
        match self {
            Self::Live(s) => s.poll_metadata(handler, limit),
            Self::Persistent(s) => s.poll_metadata(handler, limit),
        }
    }
}

/// Where an [`Out`] goes.
enum Sink {
    /// Live: an exclusive publication.
    Aeron {
        publication: AeronExclusivePublication,
        max_payload: usize,
        /// Its `Source` message went out since it last had no subscriber,
        /// so its recording names its publisher before its first frame.
        sourced: Cell<bool>,
    },
    /// Simulation: appended to [`Ctx::captured`] under this stream. Never
    /// dropped, so a backtest's output does not depend on back pressure.
    Capture { stream: u32, delay: Option<i64> },
}

/// A simulated publication awaiting local delivery.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Loopback {
    ts: Nanos,
    seq: u64,
    stream: u32,
    frame: Vec<u8>,
}

/// The largest frame a simulated output takes: one UDP frame's payload.
const SIM_MAX_PAYLOAD: usize = 1376;

/// What the agent reaches: time, timers, feeds, outputs, metrics, persist.
pub struct Ctx {
    now: Nanos,
    sim: bool,
    clock: Clock,
    /// Wall clock less [`Clock::read`], re-measured every second.
    wall_offset: i64,
    wheel: TimerWheel,
    fired: Vec<Fired>,
    /// The application's Aeron client, which only this owns. `None` in a
    /// simulation, which keeps its own.
    bus: Option<Bus>,
    persist: Option<Persist>,
    metrics: Metrics,
    directory: Box<dyn Directory>,
    region: String,
    sinks: Vec<Sink>,
    /// Simulation: what the outputs sent, in order.
    captured: RefCell<FrameLog>,
    simulation_error: RefCell<Option<Error>>,
    loopback_routes: BTreeMap<String, i64>,
    loopback: RefCell<BinaryHeap<Reverse<Loopback>>>,
    loopback_seq: std::cell::Cell<u64>,
    /// Simulation: each feed's `service/kind`, by [`FeedId`].
    sim_feeds: Vec<String>,
    /// Feeds opened since the last cycle; the runtime adopts them.
    opened: Vec<Feed>,
    feeds: u32,
    next_id: u64,
    stopping: bool,
    journal: bool,
    journal_seq: u64,
    journal_error: Option<Error>,
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

    /// Resolve later [`Ctx::subscribe`] and [`Ctx::publish`] calls through
    /// `directory`: the application's registry changed while it runs. Feeds
    /// already open keep their addresses.
    pub fn set_directory(&mut self, directory: Box<dyn Directory>) {
        self.directory = directory;
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
            || Tracer::detached(name, stages, attrs),
            |p| p.tracer(name, stages, attrs),
        )
    }

    fn bus(&self) -> Result<&Bus, Error> {
        self.bus
            .as_ref()
            .ok_or_else(|| Error::Aeron("this simulation has no Aeron client".into()))
    }

    /// A simulated feed, by name: the simulation driver delivers the
    /// recorded frames of `service/kind` to it.
    fn sim_feed(&mut self, service: &str, kind: &str) -> FeedId {
        self.sim_feeds.push(format!("{service}/{kind}"));
        let id = FeedId(self.feeds);
        self.feeds += 1;
        id
    }

    /// Simulation: everything the outputs sent, each under its
    /// `service/kind` and the event time it was sent at.
    #[must_use]
    pub fn captured(&self) -> FrameLog {
        self.captured.borrow().clone()
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
    /// The directory does not name it, or names no archive for it.
    pub fn subscribe(&mut self, service: &str, kind: &str) -> Result<FeedId, Error> {
        if self.sim {
            return Ok(self.sim_feed(service, kind));
        }
        let feed = self.persistent(service, kind)?;
        Ok(self.adopt(Feed::Persistent(feed)))
    }

    fn persistent(&self, service: &str, kind: &str) -> Result<PersistentSubscription, Error> {
        let bus = self.bus()?;
        let addr = self.directory.feed(service, kind, bus.host_ip())?;
        bus.subscribe(&format!("{service}/{kind}"), &addr)
    }

    /// [`Ctx::subscribe`], but each new recording replays from its start: for
    /// a consumer that must see what its publisher sent while it was down,
    /// such as an exchange's orders.
    ///
    /// # Errors
    ///
    /// The directory does not name it, or names no archive for it.
    pub fn subscribe_from_start(&mut self, service: &str, kind: &str) -> Result<FeedId, Error> {
        if self.sim {
            return Ok(self.sim_feed(service, kind));
        }
        let feed = self.persistent(service, kind)?.from_start();
        Ok(self.adopt(Feed::Persistent(feed)))
    }

    /// Subscribe to `service`'s `kind` stream off the network only: best
    /// effort, nothing replayed.
    ///
    /// # Errors
    ///
    /// The directory does not name it.
    pub fn subscribe_live(&mut self, service: &str, kind: &str) -> Result<FeedId, Error> {
        if self.sim {
            return Ok(self.sim_feed(service, kind));
        }
        let bus = self.bus()?;
        let addr = self.directory.feed(service, kind, bus.host_ip())?;
        let feed = Feed::Live(bus.subscribe_live(&addr));
        Ok(self.adopt(feed))
    }

    /// Subscribe to a raw `channel` and `stream_id`, off the network. In a
    /// simulation its name is `channel/stream_id`.
    pub fn subscribe_channel(&mut self, channel: &str, stream_id: i32) -> FeedId {
        match &self.bus {
            Some(bus) if !self.sim => {
                let feed = Feed::Live(bus.subscription(channel, stream_id));
                self.adopt(feed)
            }
            _ => self.sim_feed(channel, &stream_id.to_string()),
        }
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
    /// The directory does not name it, or the driver did not add it.
    pub fn publish(&mut self, service: &str, kind: &str) -> Result<Out, Error> {
        if self.sim {
            return Ok(self.capture(service, kind));
        }
        let addr = self
            .directory
            .publication(service, kind, self.bus()?.host_ip())?;
        self.publish_channel(&addr.channel, addr.stream_id)
    }

    fn capture(&mut self, service: &str, kind: &str) -> Out {
        let name = format!("{service}/{kind}");
        let stream = self.captured.borrow_mut().stream(&name);
        let out = Out(u32::try_from(self.sinks.len()).unwrap_or(u32::MAX));
        self.sinks.push(Sink::Capture {
            stream,
            delay: self.loopback_routes.get(&name).copied(),
        });
        out
    }

    /// Publish on a raw `channel` and `stream_id`, from this thread alone.
    ///
    /// # Errors
    ///
    /// The driver did not add the publication.
    pub fn publish_channel(&mut self, channel: &str, stream_id: i32) -> Result<Out, Error> {
        if self.sim {
            return Ok(self.capture(channel, &stream_id.to_string()));
        }
        let publication = self.bus()?.add_exclusive_publication(channel, stream_id)?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        let out = Out(u32::try_from(self.sinks.len()).unwrap_or(u32::MAX));
        self.sinks.push(Sink::Aeron {
            publication,
            max_payload,
            sourced: Cell::new(false),
        });
        self.send_source(out);
        Ok(out)
    }

    /// Publish one message of exactly `len` bytes (header included) that
    /// `encode` writes into the claimed frame, zero-copy. The frame's reserved
    /// value is [`Ctx::now`]: the event time it was published at, which a
    /// replay or backtest uses as the message's event time. A frame Aeron does
    /// not take is counted in [`Ctx::drops`] and returns `Ok(())`; after
    /// [`Invoker::finish`] closed the outputs, nothing is sent or counted.
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
        let (publication, max_payload, sourced) = match self.sinks.get(out.0 as usize) {
            Some(Sink::Aeron {
                publication,
                max_payload,
                sourced,
            }) => (publication, *max_payload, sourced),
            Some(Sink::Capture { stream, delay }) => {
                return self.send_captured(*stream, *delay, template_id, len, encode);
            }
            None => {
                self.count(DropKind::Other);
                return Ok(());
            }
        };
        if !sourced.get() {
            self.send_source(out);
        }
        if len > max_payload {
            self.refused(publication, sourced, DropKind::TooLarge);
            return Ok(());
        }
        let claim = match crate::bus::claim_exclusive(publication, len, self.now.0) {
            Ok(claim) => claim,
            Err(kind) => {
                self.refused(publication, sourced, kind);
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
            self.count(DropKind::Other);
        }
        Ok(())
    }

    #[cold]
    fn count(&self, kind: DropKind) {
        if let Some(bus) = &self.bus {
            bus.count(kind);
        }
    }

    /// Count a frame an output did not send, unless [`Invoker::finish`]
    /// closed it: then it is not a drop. With no subscriber, perhaps a new
    /// recording follows: the output names its publisher again before the
    /// next frame.
    #[cold]
    fn refused(
        &self,
        publication: &AeronExclusivePublication,
        sourced: &Cell<bool>,
        kind: DropKind,
    ) {
        if publication.is_closed() {
            return;
        }
        if kind == DropKind::NotConnected {
            sourced.set(false);
        }
        self.count(kind);
    }

    /// Frames the outputs dropped so far, by reason: [`Bus::drops`].
    /// Persist's own records are [`Persist::drops`]. None in simulation,
    /// which captures every output.
    #[must_use]
    pub fn drops(&self) -> Drops {
        self.bus.as_ref().map_or_else(Drops::default, Bus::drops)
    }

    /// Close every publication the application owns: its outputs and
    /// persist's. A send or a record after it is not published, and not
    /// counted as a drop.
    fn close(&self) {
        if self.bus.is_none() {
            return;
        }
        let outputs = self.sinks.iter().filter_map(|sink| match sink {
            Sink::Aeron { publication, .. } => Some(publication.clone()),
            Sink::Capture { .. } => None,
        });
        crate::bus::close(outputs.chain(self.persist.as_ref().map(Persist::publication)));
    }

    fn send_captured<E>(
        &self,
        stream: u32,
        delay: Option<i64>,
        template_id: u16,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        let mut captured = self.captured.borrow_mut();
        if len > SIM_MAX_PAYLOAD {
            *self.simulation_error.borrow_mut() = Some(Error::Config(format!(
                "simulated output {stream} exceeds its maximum payload"
            )));
            return Ok(());
        }
        let mut queued = delay.map(|_| vec![0; len]);
        let mut recorded = self.persist.as_ref().map(|_| vec![0; len]);
        let appended = captured.append(stream, self.now, len, |slot| {
            let written = encode(slot)?;
            if let Some(frame) = &mut queued {
                frame.copy_from_slice(slot);
            }
            if let Some(frame) = &mut recorded {
                frame.copy_from_slice(slot);
            }
            Ok(written)
        })?;
        // Released before the record: a simulated publish polls the bus.
        drop(captured);
        if appended && let (Some(persist), Some(frame)) = (&self.persist, recorded) {
            let _: Result<(), std::convert::Infallible> =
                persist.record(template_id, len, |slot| {
                    slot.copy_from_slice(&frame);
                    Ok(len)
                });
        }
        if appended && let (Some(delay), Some(frame)) = (delay, queued) {
            let seq = self.loopback_seq.get();
            self.loopback_seq.set(seq + 1);
            self.loopback.borrow_mut().push(Reverse(Loopback {
                ts: Nanos(self.now.0.saturating_add(delay)),
                seq,
                stream,
                frame,
            }));
        }
        if !appended {
            *self.simulation_error.borrow_mut() = Some(Error::Config(format!(
                "simulated output {stream} encoded a different length than {len}"
            )));
        }
        Ok(())
    }

    /// `out` is connected: a subscriber or the archive takes it.
    #[must_use]
    pub fn is_connected(&self, out: Out) -> bool {
        match self.sinks.get(out.0 as usize) {
            Some(Sink::Aeron { publication, .. }) => publication.is_connected(),
            Some(Sink::Capture { .. }) => true,
            None => false,
        }
    }

    /// The longest message one frame of `out` holds.
    #[must_use]
    pub fn max_payload(&self, out: Out) -> usize {
        match self.sinks.get(out.0 as usize) {
            Some(Sink::Aeron { max_payload, .. }) => *max_payload,
            Some(Sink::Capture { .. }) => SIM_MAX_PAYLOAD,
            None => 0,
        }
    }

    #[cold]
    fn send_source(&self, out: Out) {
        let (
            Some(Sink::Aeron {
                publication,
                sourced,
                ..
            }),
            Some(bus),
        ) = (self.sinks.get(out.0 as usize), &self.bus)
        else {
            return;
        };
        let message = bus.source_message();
        if let Ok(claim) = crate::bus::claim_exclusive(publication, message.len(), self.now.0) {
            claim.data().copy_from_slice(message);
            sourced.set(claim.commit().is_ok());
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

    fn journal_input(&mut self, event: InputEvent) -> bool {
        if self.journal_error.is_some() {
            return false;
        }
        self.journal_seq += 1;
        let input = Input {
            sequence: self.journal_seq,
            wall_offset: self.wall_offset,
            next_id: self.next_id,
            ts: self.now,
            event,
        };
        if let Some(persist) = &self.persist
            && let Err(error) = persist.record_input(input)
        {
            self.journal_error = Some(error);
            self.stopping = true;
            return false;
        }
        true
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

/// An embedded live runtime: drives one duty cycle on the caller's thread.
/// [`Runtime::run`] uses the same loop without a mode branch per cycle.
pub struct Invoker {
    feeds: Vec<Feed>,
    ctx: Ctx,
    idle: Idle,
    limit: usize,
    /// Read by housekeeping every 10 ms.
    stop: Stop,
    /// [`Invoker::finish`] ran: it runs once.
    finished: bool,
    cpu: Option<usize>,
    lock_memory: bool,
    timer_slack: Option<std::time::Duration>,
    #[cfg(feature = "mimalloc")]
    alloc: Option<crate::alloc_stats::AllocStats>,
    /// When the loop last ran the conductor itself: in a cycle with no work,
    /// or once a millisecond has passed. A persistent feed's poll runs it as
    /// well, in every cycle.
    conductor_ran: Nanos,
    /// Housekeeping due in a cycle that had work: run at the next idle one,
    /// or once it has waited `MAX_DEFER`.
    deferred: u8,
    deferred_since: Nanos,
}

/// The longest housekeeping waits behind busy cycles.
const MAX_DEFER: i64 = MS / 2;

impl Invoker {
    /// A live runtime on `config`.
    ///
    /// # Errors
    ///
    /// The timer settings are not powers of two, or `journal` is set without
    /// `persist`.
    pub fn new(config: Config) -> Result<Self, Error> {
        if config.journal && config.persist.is_none() {
            return Err(Error::Config("JOURNAL=on requires Persist".into()));
        }
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
            bus: Some(config.bus),
            persist: config.persist,
            metrics,
            directory: config.directory,
            region: config.region,
            sinks: Vec::new(),
            captured: RefCell::new(FrameLog::new()),
            simulation_error: RefCell::new(None),
            loopback_routes: BTreeMap::new(),
            loopback: RefCell::new(BinaryHeap::new()),
            loopback_seq: std::cell::Cell::new(0),
            sim_feeds: Vec::new(),
            opened: Vec::new(),
            feeds: 0,
            next_id: now.0.cast_unsigned() & !RUNTIME_TOKEN,
            stopping: false,
            journal: config.journal,
            journal_seq: 0,
            journal_error: None,
        };
        Ok(Self {
            conductor_ran: Nanos(0),
            feeds: Vec::new(),
            ctx,
            idle: config.idle,
            limit: config.limit,
            stop: config.stop,
            finished: false,
            cpu: config.cpu,
            lock_memory: config.lock_memory,
            timer_slack: config.timer_slack,
            #[cfg(feature = "mimalloc")]
            alloc: None,
            deferred: 0,
            deferred_since: Nanos(0),
        })
    }

    /// From the environment, as [`crate::app::App::start_with`]: logging, the
    /// bus and persist for `schema`, `REGION`, `IDLE` (default `spin`),
    /// `TIMER_SLACK`, `CPU`, `MLOCK`, `JOURNAL`, and
    /// SIGTERM, with the application's `directory` of feed names. Persist's
    /// publication is the calling thread's: run the loop on it.
    ///
    /// # Errors
    ///
    /// As [`crate::app::App::start_with`].
    pub fn from_env(
        schema: &str,
        directory: Box<dyn Directory>,
    ) -> Result<Self, crate::app::Error> {
        let app = crate::app::App::start_with(schema, Idle::Spin)?;
        let timer_slack = std::env::var("TIMER_SLACK")
            .ok()
            .map(|v| {
                crate::idle::duration(&v)
                    .map_err(|e| crate::app::Error::Idle(format!("TIMER_SLACK={v}: {e}")))
            })
            .transpose()?;
        let config = Config {
            persist: Some(app.persist),
            directory,
            idle: app.idle,
            stop: app.stop,
            cpu: std::env::var("CPU").ok().and_then(|c| c.parse().ok()),
            lock_memory: std::env::var("MLOCK").is_ok_and(|v| v == "1" || v == "true"),
            timer_slack,
            journal: std::env::var("JOURNAL").is_ok_and(|v| v == "on"),
            ..Config::new(app.bus)
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
        if let Some(slack) = self.timer_slack {
            crate::os::set_timer_slack(slack);
        }
        #[cfg(feature = "mimalloc")]
        {
            self.alloc = Some(crate::alloc_stats::AllocStats::new(&self.ctx.metrics));
            self.ctx.schedule_runtime(5 * SECOND, HK_ALLOC);
        }
        self.ctx.now = self.ctx.clock.now();
        if self.ctx.journal && !self.ctx.journal_input(InputEvent::Start) {
            return Err(self
                .ctx
                .journal_error
                .take()
                .unwrap_or_else(|| Error::Aeron("input journal start failed".into())));
        }
        agent.start(&mut self.ctx)?;
        self.adopt();
        self.ctx.schedule_runtime(MS, HK_PERSIST);
        self.ctx.schedule_runtime(SECOND, HK_WALL);
        self.ctx.schedule_runtime(5 * SECOND, HK_SOURCE);
        if self.stop.is_some() {
            // A read system call: off the millisecond tick.
            self.ctx.schedule_runtime(10 * MS, HK_SIGNAL);
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
        self.finish(&mut agent)?;
        Ok(agent)
    }

    /// Stop `agent` and close what the application publishes: its outputs
    /// and persist's publication, at once, so subscribers turn to the next
    /// publisher within seconds rather than after this client's timeout.
    /// [`Runtime::run`] does this. It runs once: a later call, from
    /// SIGTERM and a framework's stop both, does nothing.
    ///
    /// # Errors
    ///
    /// The input journal could not record the stop.
    pub fn finish<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        if std::mem::replace(&mut self.finished, true) {
            return Ok(());
        }
        if self.ctx.journal {
            self.ctx.journal_input(InputEvent::Stop);
        }
        agent.stop(&mut self.ctx);
        log::info!("stopping: closing the feeds");
        self.ctx.close();
        self.ctx.journal_error.take().map_or(Ok(()), Err)
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
            if !ctx.journal {
                work += feed.poll(
                    |msg, delivery| {
                        ctx.now = ctx.clock.now();
                        agent.on_message(ctx, id, msg, delivery);
                    },
                    self.limit,
                );
                continue;
            }
            work += feed.poll_metadata(
                |msg, delivery, metadata| {
                    ctx.now = ctx.clock.now();
                    if !ctx.journal_input(InputEvent::Message {
                        feed: id,
                        recording: metadata.recording,
                        position: metadata.position,
                        session: metadata.session,
                        stream: metadata.stream,
                        delivery,
                    }) {
                        return;
                    }
                    agent.on_message(ctx, id, msg, delivery);
                },
                self.limit,
            );
        }
        if self.ctx.journal_error.is_some() {
            return work;
        }
        work += agent.poll(&mut self.ctx);
        if !self.ctx.opened.is_empty() {
            self.adopt();
        }
        self.ctx.now = self.ctx.clock.now();
        if self.ctx.wheel.poll(self.ctx.now, &mut self.ctx.fired, 64) > 0 {
            work += self.fire(agent, work);
        }
        if work == 0 || self.ctx.now.since(self.conductor_ran) > MS {
            self.conductor_ran = self.ctx.now;
            work += self.ctx.bus.as_ref().map_or(0, Bus::poll);
        }
        if self.deferred != 0 && (work == 0 || self.ctx.now.since(self.deferred_since) > MAX_DEFER)
        {
            self.run_deferred();
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
        if self.ctx.journal {
            let count = fired
                .iter()
                .filter(|f| f.token & RUNTIME_TOKEN == 0)
                .count();
            if count > 0 {
                self.record_input_event(InputEvent::TimerBatchStart {
                    count: u64::try_from(count).unwrap_or(u64::MAX),
                });
                for f in fired.iter().filter(|f| f.token & RUNTIME_TOKEN == 0) {
                    self.record_input_event(InputEvent::PendingTimer {
                        token: f.token,
                        deadline: f.deadline,
                        missed: f.missed,
                    });
                }
            }
        }
        let mut n = 0;
        for f in &fired {
            if self.ctx.journal_error.is_some() {
                break;
            }
            if self.ctx.wheel.is_suppressed(f.id) {
                continue;
            }
            if f.token & RUNTIME_TOKEN != 0 {
                self.housekeeping(f.token, busy);
                continue;
            }
            n += 1;
            if self.ctx.journal
                && !self.ctx.journal_input(InputEvent::Timer {
                    token: f.token,
                    deadline: f.deadline,
                    missed: f.missed,
                })
            {
                break;
            }
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

    fn record_input_event(&mut self, event: InputEvent) {
        self.ctx.journal_input(event);
    }

    fn housekeeping(&mut self, token: u64, busy: usize) {
        let bit = 1u8 << (token & 7);
        if busy > 0 {
            if self.deferred == 0 {
                self.deferred_since = self.ctx.now;
            }
            self.deferred |= bit;
            return;
        }
        self.run_one(token);
    }

    #[cold]
    fn run_deferred(&mut self) {
        let deferred = std::mem::take(&mut self.deferred);
        for i in 0..8 {
            if deferred & (1 << i) != 0 {
                self.run_one(RUNTIME_TOKEN | i);
            }
        }
    }

    fn run_one(&mut self, token: u64) {
        match token {
            HK_PERSIST => {
                if let Some(p) = &self.ctx.persist {
                    p.poll(self.ctx.now);
                }
            }
            HK_SIGNAL if self.stop.requested() => {
                log::info!("SIGTERM");
                self.ctx.stopping = true;
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
            _ => {}
        }
    }
}

impl std::fmt::Debug for Invoker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("feeds", &self.feeds.len())
            .field("now", &self.ctx.now)
            .finish_non_exhaustive()
    }
}

/// What stops the loop from outside: the read end of SIGTERM's pipe, or
/// nothing.
#[derive(Debug)]
pub struct Stop(Option<UnixStream>);

impl Stop {
    /// Never stops the loop.
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    const fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// The handler wrote a byte since the last call. The read end is
    /// nonblocking: a system call, never a wait.
    fn requested(&self) -> bool {
        let Some(mut pipe) = self.0.as_ref() else {
            return false;
        };
        let mut bytes = [0; 16];
        matches!(pipe.read(&mut bytes), Ok(n) if n > 0)
    }
}

/// SIGTERM's pipe: [`Config::stop`] for a runtime built by hand.
///
/// The handler writes a byte to a socket pair's write end, and the runtime
/// reads the other every 10 ms. Nothing of ours is shared with the handler,
/// and other SIGTERM handlers (a framework's) still run.
///
/// # Errors
///
/// The socket pair could not be made, or the handler registered.
pub fn sigterm() -> Result<Stop, std::io::Error> {
    let (read, write) = UnixStream::pair()?;
    read.set_nonblocking(true)?;
    signal_hook::low_level::pipe::register(signal_hook::consts::SIGTERM, write)?;
    Ok(Stop(Some(read)))
}
