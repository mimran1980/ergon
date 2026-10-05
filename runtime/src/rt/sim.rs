//! The simulation driver: the same [`Agent`] on simulated time.
//!
//! Sources yield `(ts, feed, frame)` in time order; the driver merges them
//! and the agent's timers into one total order, sets the clock to each item's
//! time, and dispatches it:
//!
//! ```text
//! key = (ts, rank, a, b, c)   events: rank 0, (feed, recording, position)
//!                             timers: rank 1, (sequence)
//! ```
//!
//! A timer due strictly earlier fires first; at an equal time every event
//! goes first, then the timers, as the live loop polls feeds before timers.
//! Each timer fires exactly at its deadline, so `missed` is always 0. Time
//! never comes from the hardware: [`Ctx::now`] and [`Ctx::read`] are the
//! simulated time, [`Ctx::wall_ns`] equals it, and [`Ctx::next_id`] counts
//! from 1. Outputs are captured ([`Ctx::captured`]), never dropped. Two runs
//! over the same input give the same output, byte for byte.
//!
//! A simulation that records through persist, or replays an Aeron Archive,
//! owns the [`Bus`] it does it through ([`SimConfig::bus`]). The client has
//! no conductor thread: the simulation runs the conductor every 1024
//! dispatches, while it paces, and around each `ClickHouse` query, so no
//! stretch of the run goes the client's 10 s liveness timeout without it
//! unless one query does.

use std::time::{Duration, Instant};

use crate::Error;
use crate::clock::{Clock, Nanos};
use crate::directory::{Directory, NoDirectory};
use crate::frames::{self, FrameLog};
use crate::metrics::Metrics;
use crate::persist::Persist;
use crate::subscription::{Delivery, Origin};
use crate::timer::{self, SimKey, TimerWheel};
use std::collections::{BTreeMap, BinaryHeap};

use super::archive::{ArchiveConfig, ArchiveSource};
use super::{Agent, Ctx, Expiry, FeedId, RUNTIME_TOKEN};
use crate::bus::Bus;

/// What a simulation runs on.
pub struct SimConfig {
    /// Names to stream ids, for an archive replay to find a subscribed
    /// feed's recordings; a simulation of logs or `ClickHouse` frames needs
    /// none.
    pub directory: Box<dyn Directory>,
    /// `REGION`.
    pub region: String,
    /// Rows and metrics, stamped with simulated time.
    pub persist: Option<Persist>,
    /// The timer wheel's shape.
    pub timers: timer::Settings,
    /// Skip input before this time.
    pub from: Option<Nanos>,
    /// End the run here; otherwise at the last input.
    pub to: Option<Nanos>,
    /// Pace the run against the wall clock: `Some(1.0)` replays in real time,
    /// `Some(10.0)` ten times faster; `None` runs as fast as it can.
    pub speed: Option<f64>,
    /// The Aeron client the run records through ([`SimConfig::persist`]) or
    /// replays [`SimConfig::archive`] from; the simulation owns it and runs
    /// its conductor. `None` for a run that touches no Aeron.
    pub bus: Option<Bus>,
    /// Replay subscribed feeds from this Aeron Archive, through
    /// [`SimConfig::bus`]. Needs `from`: an agent schedules its timers before
    /// the first frame.
    pub archive: Option<ArchiveConfig>,
    /// Receive delay in ns, by `service/kind`, applied before the merge.
    pub route_delays: BTreeMap<String, i64>,
    /// Local outputs delivered to matching feeds after this latency in ns.
    pub loopback: BTreeMap<String, i64>,
    /// Actual live dispatches; replaces the timestamp merge and timer polling.
    pub journal: Option<crate::journal::Journal>,
    /// Streamed raw frames from the ingester's table; requires `from`.
    #[cfg(feature = "clickhouse")]
    pub clickhouse: Option<crate::clickhouse_source::ClickHouseConfig>,
}

#[derive(Default)]
struct ReplayTimers {
    pending: Vec<(u64, Nanos, u64)>,
    remaining: u64,
    staged: Vec<crate::timer::Fired>,
}

impl ReplayTimers {
    fn complete(&self) -> Result<(), Error> {
        if self.remaining != 0 {
            return Err(Error::Config("incomplete journal timer batch".into()));
        }
        Ok(())
    }

    fn prepare(
        &mut self,
        wheel: &mut TimerWheel,
        input: &crate::journal::Input,
    ) -> Result<(), Error> {
        use crate::journal::InputEvent;
        match input.event {
            InputEvent::TimerBatchStart { count } => {
                self.complete()?;
                self.remaining = count;
                self.pending.clear();
                self.staged.clear();
            }
            InputEvent::PendingTimer {
                token,
                deadline,
                missed,
            } => {
                if self.remaining == 0 {
                    return Err(Error::Config("journal timer outside a batch".into()));
                }
                self.pending.push((token, deadline, missed));
                self.remaining -= 1;
                if self.remaining == 0 {
                    self.staged =
                        wheel
                            .journal_batch(&self.pending, input.ts)
                            .ok_or_else(|| {
                                Error::Config(
                                    "journal batch does not match scheduled timers".into(),
                                )
                            })?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn fire(
        &mut self,
        wheel: &mut TimerWheel,
        token: u64,
        deadline: Nanos,
        at: Nanos,
        missed: u64,
    ) -> Result<crate::timer::Fired, Error> {
        self.complete()?;
        let prepared = self
            .staged
            .iter()
            .position(|f| f.token == token && f.deadline == deadline);
        let fired = prepared
            .map(|i| self.staged.remove(i))
            .or_else(|| wheel.journal_fire(token, deadline, at, missed))
            .ok_or_else(|| {
                Error::Config(format!(
                    "journal timer {token} was not scheduled or was cancelled"
                ))
            })?;
        if wheel.is_suppressed(fired.id) || fired.missed != missed {
            return Err(Error::Config(
                "journal fired a suppressed or mismatched timer".into(),
            ));
        }
        Ok(fired)
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl SimConfig {
    /// No directory (an archive replay needs one), bus or persist, the
    /// default wheel, the whole input, region `unknown`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            directory: Box::new(NoDirectory),
            region: crate::UNKNOWN_REGION.into(),
            persist: None,
            timers: timer::Settings::default(),
            from: None,
            to: None,
            speed: None,
            bus: None,
            archive: None,
            route_delays: BTreeMap::new(),
            loopback: BTreeMap::new(),
            journal: None,
            #[cfg(feature = "clickhouse")]
            clickhouse: None,
        }
    }
}

/// An input to the merge.
enum Source {
    Log(LogSource),
    Archive(Box<ArchiveSource>),
    #[cfg(feature = "clickhouse")]
    ClickHouse(crate::clickhouse_source::ClickHouseSource),
}

/// A head's record: `(ts, recording, position, frame)`.
type Head<'a> = (Nanos, i64, i64, &'a [u8]);

impl Source {
    fn heads(&self) -> usize {
        match self {
            Self::Log(l) => l.names.len(),
            Self::Archive(a) => a.len(),
            #[cfg(feature = "clickhouse")]
            Self::ClickHouse(c) => c.len(),
        }
    }

    fn feeds(&self, head: usize) -> &[FeedId] {
        match self {
            Self::Log(l) => &l.feeds[head],
            Self::Archive(a) => &a.feeds[head],
            #[cfg(feature = "clickhouse")]
            Self::ClickHouse(c) => &c.feeds[head],
        }
    }

    fn head(&self, i: usize, number: usize) -> Option<Head<'_>> {
        match self {
            Self::Log(l) => l.head(i).map(|r| {
                (
                    r.ts,
                    i64::try_from(number).unwrap_or(i64::MAX),
                    i64::try_from(r.offset).unwrap_or(i64::MAX),
                    r.frame,
                )
            }),
            Self::Archive(a) => a.head(i),
            #[cfg(feature = "clickhouse")]
            Self::ClickHouse(c) => c.head(i),
        }
    }

    // Only the ClickHouse source can fail to advance.
    #[cfg_attr(not(feature = "clickhouse"), allow(clippy::unnecessary_wraps))]
    fn advance(&mut self, head: usize) -> Result<(), Error> {
        match self {
            Self::Log(l) => l.cursor[head] += 1,
            Self::Archive(a) => a.advance(head),
            #[cfg(feature = "clickhouse")]
            Self::ClickHouse(c) => c.advance(head)?,
        }
        Ok(())
    }
}

/// One input: a frame log, indexed by stream so each stream has its own
/// head in the merge.
struct LogSource {
    bytes: Vec<u8>,
    names: Vec<String>,
    /// Where the record section starts.
    records: usize,
    /// Each stream's record offsets, in order, and the next one to deliver.
    index: Vec<Vec<usize>>,
    cursor: Vec<usize>,
    /// Each stream's subscribed feeds, rebuilt when the agent subscribes.
    feeds: Vec<Vec<FeedId>>,
}

impl LogSource {
    fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        let parsed = frames::parse(&bytes)?;
        let records = bytes.len() - parsed.records.remaining();
        let names = parsed.names;
        let mut index = vec![Vec::new(); names.len()];
        for r in parsed.records {
            if let Some(stream) = index.get_mut(r.stream as usize) {
                stream.push(r.offset);
            }
        }
        Ok(Self {
            cursor: vec![0; names.len()],
            feeds: vec![Vec::new(); names.len()],
            names,
            records,
            index,
            bytes,
        })
    }

    fn record(&self, offset: usize) -> Option<frames::Record<'_>> {
        frames::Records::at(&self.bytes[self.records..], offset).next()
    }

    /// The next record of `stream`.
    fn head(&self, stream: usize) -> Option<frames::Record<'_>> {
        let offset = *self.index.get(stream)?.get(*self.cursor.get(stream)?)?;
        self.record(offset)
    }

    /// The earliest record across every stream: where the clock starts.
    fn first(&self) -> Option<Nanos> {
        (0..self.names.len())
            .filter_map(|s| self.head(s).map(|r| r.ts))
            .min()
    }

    /// Bind each stream to the feeds subscribed to its name. A stream bound
    /// now for the first time starts at `now`: a feed subscribed mid-run
    /// takes what is published from then on.
    fn bind(&mut self, subscribed: &[String], now: Nanos) {
        for stream in 0..self.names.len() {
            let feeds: Vec<FeedId> = subscribed
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == self.names[stream])
                .map(|(id, _)| FeedId(u32::try_from(id).unwrap_or(u32::MAX)))
                .collect();
            if self.feeds[stream].is_empty() && !feeds.is_empty() {
                while self.head(stream).is_some_and(|r| r.ts < now) {
                    self.cursor[stream] += 1;
                }
            }
            self.feeds[stream] = feeds;
        }
    }
}

/// Dispatches between two runs of the conductor.
const CONDUCT_EVERY: u32 = 1024;

/// A simulation: sources, the agent's context, and the merge.
pub struct Sim {
    ctx: Ctx,
    /// The client the run records or replays through: see [`SimConfig::bus`].
    bus: Option<Bus>,
    /// Dispatches since the conductor last ran.
    undriven: u32,
    sources: Vec<Source>,
    started: bool,
    /// Wall-clock pacing: the speed, and the wall and simulated time it
    /// started from.
    pace: Option<(f64, std::time::Instant, Nanos)>,
    from: Option<Nanos>,
    to: Option<Nanos>,
    /// Feeds that have had their first message.
    delivered: Vec<bool>,
    bound: usize,
    route_delays: BTreeMap<String, i64>,
    journal: Option<crate::journal::Journal>,
    exact: bool,
    #[cfg(feature = "clickhouse")]
    journal_frames: Option<crate::clickhouse_source::ClickHouseConfig>,
    /// The journal's frames fetched so far, and the input they run up to.
    #[cfg(feature = "clickhouse")]
    journal_fetched: (crate::journal::clickhouse::Frames, usize),
}

/// Journal inputs whose frames one `ClickHouse` fetch brings: a few
/// megabytes, and one query per feed and publication.
#[cfg(feature = "clickhouse")]
const JOURNAL_FETCH: usize = 16_384;

impl Sim {
    /// A simulation of the frame logs `logs`, and of the archive when the
    /// config names one, its clock at `from` or the earliest log record.
    ///
    /// # Errors
    ///
    /// A log does not parse, the timer settings are not powers of two,
    /// `persist` was given without a bus, or an archive was named without
    /// `from` or a bus, or did not connect.
    pub fn new(mut config: SimConfig, logs: Vec<Vec<u8>>) -> Result<Self, Error> {
        let (sources, start) = Self::open_sources(&config, logs)?;
        let bus = config.bus.take();
        #[cfg(feature = "clickhouse")]
        let journal_frames = config.clickhouse.clone();
        if let Some(persist) = &config.persist {
            persist.set_simulation(start);
        }
        let wheel = TimerWheel::new(config.timers)
            .map_err(|e| Error::Config(format!("timer wheel: {e}")))?;
        let metrics = config
            .persist
            .as_ref()
            .map_or_else(Metrics::detached, Persist::metrics);
        let ctx = Ctx {
            now: start,
            sim: true,
            journal: false,
            journal_seq: 0,
            journal_error: None,
            clock: Clock::new(),
            wall_offset: 0,
            wheel,
            fired: Vec::with_capacity(64),
            bus: None,
            persist: config.persist,
            metrics,
            directory: config.directory,
            region: config.region,
            sinks: Vec::new(),
            captured: std::cell::RefCell::new(FrameLog::new()),
            simulation_error: std::cell::RefCell::new(None),
            loopback_routes: config.loopback,
            loopback: std::cell::RefCell::new(BinaryHeap::new()),
            loopback_seq: std::cell::Cell::new(0),
            sim_feeds: Vec::new(),
            opened: Vec::new(),
            feeds: 0,
            next_id: 0,
            stopping: false,
        };
        Ok(Self {
            ctx,
            bus,
            undriven: 0,
            sources,
            started: false,
            pace: config
                .speed
                .map(|speed| (speed, std::time::Instant::now(), start)),
            from: config.from,
            to: config.to,
            delivered: Vec::new(),
            bound: 0,
            route_delays: config.route_delays,
            exact: config.journal.is_some(),
            journal: config.journal,
            #[cfg(feature = "clickhouse")]
            journal_frames,
            #[cfg(feature = "clickhouse")]
            journal_fetched: Default::default(),
        })
    }

    fn open_sources(config: &SimConfig, logs: Vec<Vec<u8>>) -> Result<(Vec<Source>, Nanos), Error> {
        if config.speed.is_some_and(|s| !s.is_finite() || s <= 0.0)
            || config
                .route_delays
                .values()
                .chain(config.loopback.values())
                .any(|d| *d < 0)
        {
            return Err(Error::Config(
                "simulation speed must be finite and positive; delays nonnegative".into(),
            ));
        }
        // Persist's client is driven only through the bus the simulation
        // owns: without it, the driver closes the client after 10 s.
        if config.persist.is_some() && config.bus.is_none() {
            return Err(Error::Config(
                "recording through persist needs the bus that drives its client".into(),
            ));
        }
        let logs = logs
            .into_iter()
            .map(LogSource::new)
            .collect::<Result<Vec<_>, _>>()?;
        let first = logs.iter().filter_map(LogSource::first).min();
        let start = config
            .journal
            .as_ref()
            .and_then(|j| j.inputs.first().map(|i| i.ts))
            .or(config.from)
            .or(first)
            .unwrap_or_default();
        let mut sources: Vec<Source> = logs.into_iter().map(Source::Log).collect();
        if let Some(archive) = &config.archive {
            if config.from.is_none() {
                return Err(Error::Config("an archive replay needs `from`".into()));
            }
            let bus = config
                .bus
                .as_ref()
                .ok_or_else(|| Error::Config("an archive replay needs a bus".into()))?;
            sources.push(Source::Archive(Box::new(ArchiveSource::connect(
                bus, archive,
            )?)));
        }
        #[cfg(feature = "clickhouse")]
        if let Some(clickhouse) = config.clickhouse.clone()
            && config.journal.is_none()
        {
            let from = config
                .from
                .ok_or_else(|| Error::Config("ClickHouse backtest needs `from`".into()))?;
            sources.push(Source::ClickHouse(
                crate::clickhouse_source::ClickHouseSource::new(
                    clickhouse,
                    from,
                    config.to.unwrap_or(Nanos(i64::MAX)),
                ),
            ));
        }
        Ok((sources, start))
    }

    /// The agent's context, for set-up before [`Sim::run`].
    pub const fn ctx(&mut self) -> &mut Ctx {
        &mut self.ctx
    }

    /// The agent context without mutating its state.
    pub const fn ctx_ref(&self) -> &Ctx {
        &self.ctx
    }

    /// Run `agent` over the whole input, or until `to` or [`Ctx::stop`].
    ///
    /// # Errors
    ///
    /// The agent's [`Agent::start`] failed, or an archive replay stalled.
    pub fn run<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        if let Some(first) = self.journal.as_ref().and_then(|j| j.inputs.first())
            && matches!(first.event, crate::journal::InputEvent::Start)
        {
            self.ctx.now = first.ts;
            self.ctx.next_id = first.next_id;
            self.ctx.wall_offset = first.wall_offset;
        }
        self.set_persist_time();
        agent.start(&mut self.ctx)?;
        let result = if let Some(journal) = self.journal.take() {
            self.run_journal(agent, &journal)
        } else {
            self.run_events(agent)
        };
        agent.stop(&mut self.ctx);
        result?;
        self.check_output()?;
        self.flush_persist()
    }

    fn run_events<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        self.check_output()?;
        while !self.ctx.stopping {
            self.check_output()?;
            self.rebind()?;
            self.started = true;
            for source in &mut self.sources {
                if let Source::Archive(a) = source {
                    a.fill_all()?;
                }
            }
            let event = self.next_event();
            let local = self.next_loopback();
            let event_key = event.map(|(key, _)| key);
            if local.is_some_and(|key| event_key.is_none_or(|e| key < e)) {
                let key = local.unwrap_or_else(|| SimKey::event(i64::MAX, 0, 0, 0));
                let timer = self.ctx.wheel.next_deadline();
                if timer.is_some_and(|t| SimKey::timer(t.0, 0) < key) {
                    let t = timer.unwrap_or(Nanos(i64::MAX));
                    if self.to.is_none_or(|end| t <= end) {
                        self.fire(agent, t);
                    } else {
                        break;
                    }
                } else if self.to.is_none_or(|end| key.ts <= end.0) {
                    self.dispatch_loopback(agent);
                } else {
                    break;
                }
                continue;
            }
            let timer = self.ctx.wheel.next_deadline();
            let end = self.to.unwrap_or(Nanos(i64::MAX));
            match (event, timer) {
                (Some((key, _)), Some(t)) if SimKey::timer(t.0, 0) < key && t <= end => {
                    self.fire(agent, t);
                }
                (None, Some(t)) if self.to.is_some() && t <= end => self.fire(agent, t),
                (Some((key, source)), _) if key.ts <= end.0 => self.dispatch(agent, source)?,
                _ => break,
            }
        }
        Ok(())
    }

    fn run_journal<A: Agent>(
        &mut self,
        agent: &mut A,
        journal: &crate::journal::Journal,
    ) -> Result<(), Error> {
        use crate::journal::InputEvent;
        let mut previous = None;
        let mut timers = ReplayTimers::default();
        for (index, input) in journal.inputs.iter().enumerate() {
            self.check_output()?;
            if self.ctx.stopping && !matches!(input.event, InputEvent::Stop) {
                break;
            }
            if previous.is_some_and(|(seq, ts)| input.sequence <= seq || input.ts < ts) {
                return Err(Error::Config("journal sequence or time regressed".into()));
            }
            previous = Some((input.sequence, input.ts));
            if self.to.is_some_and(|end| input.ts > end) {
                break;
            }
            self.rebind()?;
            self.pace(input.ts);
            self.ctx.now = input.ts;
            self.ctx.wall_offset = input.wall_offset;
            self.ctx.next_id = input.next_id;
            self.set_persist_time();
            match input.event {
                InputEvent::TimerBatchStart { .. } | InputEvent::PendingTimer { .. } => {
                    timers.prepare(&mut self.ctx.wheel, input)?;
                }
                InputEvent::Start => {
                    if index != 0 {
                        return Err(Error::Config("invalid start checkpoint".into()));
                    }
                }
                InputEvent::Stop => {
                    if index + 1 != journal.inputs.len() {
                        return Err(Error::Config("journal Stop was not the final input".into()));
                    }
                }
                InputEvent::Message { feed, delivery, .. } => {
                    if feed.0 as usize >= self.ctx.sim_feeds.len() {
                        return Err(Error::Config(format!(
                            "journal feed {} was not subscribed",
                            feed.0
                        )));
                    }
                    let frame = self.resolve_journal(&journal.inputs, index)?;
                    agent.on_message(&mut self.ctx, feed, &frame, delivery);
                }
                InputEvent::Timer {
                    token,
                    deadline,
                    missed,
                } => {
                    let fired =
                        timers.fire(&mut self.ctx.wheel, token, deadline, input.ts, missed)?;
                    agent.on_timer(
                        &mut self.ctx,
                        Expiry {
                            id: fired.id,
                            token,
                            deadline,
                            missed,
                        },
                    );
                }
            }
            self.started = true;
            self.poll_persist();
        }
        timers.complete()?;
        Ok(())
    }

    fn resolve_journal(
        &mut self,
        inputs: &[crate::journal::Input],
        index: usize,
    ) -> Result<Vec<u8>, Error> {
        let input = &inputs[index];
        let crate::journal::InputEvent::Message {
            feed,
            recording,
            position,
            ..
        } = input.event
        else {
            return Err(Error::Config("timer is not a frame".into()));
        };
        for (number, source) in self.sources.iter_mut().enumerate() {
            match source {
                Source::Log(log) if i64::try_from(number).ok() == Some(recording) => {
                    if let Ok(at) = usize::try_from(position)
                        && let Some(record) = log.record(at)
                        && log
                            .feeds
                            .get(record.stream as usize)
                            .is_some_and(|feeds| feeds.contains(&feed))
                    {
                        return Ok(record.frame.to_vec());
                    }
                }
                Source::Archive(archive) => loop {
                    archive.fill_all()?;
                    let mut advanced = false;
                    for head in 0..archive.len() {
                        if archive.feeds[head].contains(&feed)
                            && let Some((_, r, p, frame)) = archive.head(head)
                            && r == recording
                        {
                            if p == position {
                                return Ok(frame.to_vec());
                            }
                            if p < position {
                                archive.advance(head);
                                advanced = true;
                            }
                        }
                    }
                    if !advanced {
                        break;
                    }
                },
                Source::Log(_) => {}
                #[cfg(feature = "clickhouse")]
                Source::ClickHouse(_) => {}
            }
        }
        #[cfg(feature = "clickhouse")]
        if let Some(config) = &self.journal_frames {
            let name = self
                .ctx
                .sim_feeds
                .get(feed.0 as usize)
                .ok_or_else(|| Error::Config("journal feed is unknown".into()))?;
            // From the frames fetched with this stretch of the journal; past
            // it, or on a feed subscribed since that fetch, fetch from here.
            for fetch in [index >= self.journal_fetched.1, true] {
                if fetch {
                    let end = inputs.len().min(index + JOURNAL_FETCH);
                    self.journal_fetched = (
                        crate::journal::clickhouse::fetch(
                            &config.client,
                            &config.table,
                            &inputs[index..end],
                            &self.ctx.sim_feeds,
                            || conduct(self.bus.as_ref()),
                        )?,
                        end,
                    );
                }
                if let Some(frame) = self.journal_fetched.0.take(name, input) {
                    return Ok(frame);
                }
            }
        }
        Err(Error::Config(format!(
            "journal frame {recording}/{position} was not found"
        )))
    }

    /// The earliest head across the sources' bound streams, by the total
    /// order.
    fn next_event(&self) -> Option<(SimKey, (usize, usize))> {
        let mut best: Option<(SimKey, (usize, usize))> = None;
        for (i, source) in self.sources.iter().enumerate() {
            for stream in 0..source.heads() {
                let (Some(feed), Some((ts, recording, position, _))) =
                    (source.feeds(stream).first(), source.head(stream, i))
                else {
                    continue;
                };
                let delay = self
                    .route_delays
                    .get(&self.ctx.sim_feeds[feed.0 as usize])
                    .copied()
                    .unwrap_or(0);
                let key = SimKey::event(
                    ts.0.saturating_add(delay),
                    u64::from(feed.0),
                    recording,
                    position,
                );
                if best.is_none_or(|(b, _)| key < b) {
                    best = Some((key, (i, stream)));
                }
            }
        }
        best
    }

    fn next_loopback(&self) -> Option<SimKey> {
        let queue = self.ctx.loopback.borrow();
        let head = &queue.peek()?.0;
        let captured = self.ctx.captured.borrow();
        let name = captured.names().get(head.stream as usize)?;
        let feed = self
            .ctx
            .sim_feeds
            .iter()
            .position(|n| n == name)
            .unwrap_or(usize::MAX);
        Some(SimKey::event(
            head.ts.0,
            u64::try_from(feed).unwrap_or(u64::MAX),
            i64::MAX,
            i64::try_from(head.seq).unwrap_or(i64::MAX),
        ))
    }

    fn dispatch_loopback<A: Agent>(&mut self, agent: &mut A) {
        let Some(std::cmp::Reverse(head)) = self.ctx.loopback.borrow_mut().pop() else {
            return;
        };
        let name = self.ctx.captured.borrow().names()[head.stream as usize].clone();
        self.pace(head.ts);
        self.ctx.now = head.ts;
        self.set_persist_time();
        for i in 0..self.ctx.sim_feeds.len() {
            if self.ctx.sim_feeds[i] != name {
                continue;
            }
            let delivery = Delivery {
                first: !self.delivered[i],
                origin: Origin::Live,
            };
            self.delivered[i] = true;
            agent.on_message(
                &mut self.ctx,
                FeedId(u32::try_from(i).unwrap_or(u32::MAX)),
                &head.frame,
                delivery,
            );
        }
        self.poll_persist();
    }

    /// Wait until the wall clock reaches simulated time `at` at the speed,
    /// asleep in slices of at most a millisecond, running the conductor
    /// between them.
    fn pace(&self, at: Nanos) {
        let Some((speed, wall, start)) = self.pace else {
            return;
        };
        let ahead = Duration::from_nanos(u64::try_from(at.0.saturating_sub(start.0)).unwrap_or(0));
        let due = wall + ahead.div_f64(speed);
        loop {
            let left = due.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return;
            }
            conduct(self.bus.as_ref());
            std::thread::sleep(left.min(Duration::from_millis(1)));
        }
    }

    fn dispatch<A: Agent>(
        &mut self,
        agent: &mut A,
        (source, stream): (usize, usize),
    ) -> Result<(), Error> {
        let s = &self.sources[source];
        let Some((stamp, _, _, frame)) = s.head(stream, source) else {
            return Ok(());
        };
        let delay = s
            .feeds(stream)
            .first()
            .and_then(|f| self.ctx.sim_feeds.get(f.0 as usize))
            .and_then(|name| self.route_delays.get(name))
            .copied()
            .unwrap_or(0);
        let ts = Nanos(stamp.0.saturating_add(delay));
        let skip = self.from.is_some_and(|from| ts < from);
        if !skip {
            self.pace(ts);
            self.ctx.now = ts;
            if let Some(persist) = &self.ctx.persist {
                persist.sim_time(ts);
            }
            for &feed in s.feeds(stream) {
                let seen = &mut self.delivered[feed.0 as usize];
                let delivery = Delivery {
                    first: !*seen,
                    origin: Origin::Live,
                };
                *seen = true;
                agent.on_message(&mut self.ctx, feed, frame, delivery);
            }
        }
        // Its next page waits on `ClickHouse`: the conductor runs around it.
        #[cfg(feature = "clickhouse")]
        let fetches = matches!(&self.sources[source], Source::ClickHouse(c) if c.fetches(stream));
        #[cfg(not(feature = "clickhouse"))]
        let fetches = false;
        if fetches {
            conduct(self.bus.as_ref());
        }
        self.sources[source].advance(stream)?;
        if fetches {
            conduct(self.bus.as_ref());
        }
        self.poll_persist();
        Ok(())
    }

    fn fire<A: Agent>(&mut self, agent: &mut A, at: Nanos) {
        self.pace(at);
        self.ctx.now = at;
        self.set_persist_time();
        if self.ctx.wheel.poll(at, &mut self.ctx.fired, 64) == 0 {
            return;
        }
        let fired = std::mem::take(&mut self.ctx.fired);
        for f in &fired {
            if self.ctx.wheel.is_suppressed(f.id) || f.token & RUNTIME_TOKEN != 0 {
                continue;
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
        self.poll_persist();
    }

    /// After each dispatch: persist's housekeeping, and every
    /// [`CONDUCT_EVERY`] dispatches the conductor.
    fn poll_persist(&mut self) {
        if let Some(persist) = &self.ctx.persist {
            persist.poll_sim(self.ctx.now);
        }
        self.undriven += 1;
        if self.undriven >= CONDUCT_EVERY {
            self.undriven = 0;
            conduct(self.bus.as_ref());
        }
    }

    fn check_output(&self) -> Result<(), Error> {
        if let Some(error) = self.ctx.simulation_error.borrow_mut().take() {
            return Err(error);
        }
        Ok(())
    }

    fn flush_persist(&self) -> Result<(), Error> {
        if let Some(persist) = &self.ctx.persist {
            persist.flush_sim(self.ctx.now)?;
        }
        Ok(())
    }

    fn set_persist_time(&self) {
        if let Some(persist) = &self.ctx.persist {
            persist.sim_time(self.ctx.now);
        }
    }

    /// Point each source's streams at the feeds subscribed so far: a feed
    /// subscribed mid-run takes its frames from then on.
    fn rebind(&mut self) -> Result<(), Error> {
        let subscribed = self.ctx.sim_feeds.len();
        if subscribed == self.bound {
            return Ok(());
        }
        let remote_feeds: Vec<_> = self
            .ctx
            .sim_feeds
            .iter()
            .map(|name| {
                if self.ctx.loopback_routes.contains_key(name) {
                    String::new()
                } else {
                    name.clone()
                }
            })
            .collect();
        let (feeds, directory, now) = (&remote_feeds, &*self.ctx.directory, self.ctx.now);
        for source in &mut self.sources {
            match source {
                Source::Log(l) => l.bind(feeds, now),
                Source::Archive(a) => a.bind(feeds, directory, now, self.started && !self.exact)?,
                #[cfg(feature = "clickhouse")]
                Source::ClickHouse(c) => c.bind(feeds, now, || conduct(self.bus.as_ref()))?,
            }
        }
        self.delivered.resize(subscribed, false);
        self.bound = subscribed;
        Ok(())
    }
}

/// Run `bus`'s conductor once, when the run has one.
fn conduct(bus: Option<&Bus>) {
    if let Some(bus) = bus {
        let _ = bus.poll();
    }
}
