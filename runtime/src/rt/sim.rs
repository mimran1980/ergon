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

use crate::Error;
use crate::clock::{Clock, Nanos};
use crate::frames::{self, FrameLog};
use crate::metrics::Metrics;
use crate::persist::Persist;
use crate::streams::Streams;
use crate::subscription::{Delivery, Origin};
use crate::timer::{self, SimKey, TimerWheel};

use super::{Agent, Ctx, Expiry, FeedId, RUNTIME_TOKEN};

/// What a simulation runs on.
pub struct SimConfig {
    /// The registry an agent resolves names in (`engine-<region>`, `md-*`).
    pub streams: Streams,
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
}

impl SimConfig {
    /// No persist, the default wheel, the whole input.
    #[must_use]
    pub fn new(streams: Streams) -> Self {
        Self {
            streams,
            region: String::new(),
            persist: None,
            timers: timer::Settings::default(),
            from: None,
            to: None,
        }
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

/// A simulation: sources, the agent's context, and the merge.
pub struct Sim {
    ctx: Ctx,
    sources: Vec<LogSource>,
    from: Option<Nanos>,
    to: Option<Nanos>,
    /// Feeds that have had their first message.
    delivered: Vec<bool>,
    bound: usize,
}

impl Sim {
    /// A simulation of the frame logs `logs`, its clock at the earliest
    /// record (or `from`).
    ///
    /// # Errors
    ///
    /// A log does not parse, or the timer settings are not powers of two.
    pub fn new(config: SimConfig, logs: Vec<Vec<u8>>) -> Result<Self, Error> {
        let sources = logs
            .into_iter()
            .map(LogSource::new)
            .collect::<Result<Vec<_>, _>>()?;
        let first = sources.iter().filter_map(LogSource::first).min();
        let start = config.from.or(first).unwrap_or_default();
        let wheel = TimerWheel::new(config.timers)
            .map_err(|e| Error::Config(format!("timer wheel: {e}")))?;
        let metrics = config
            .persist
            .as_ref()
            .map_or_else(Metrics::detached, Persist::metrics);
        let ctx = Ctx {
            now: start,
            sim: true,
            clock: Clock::new(),
            wall_offset: 0,
            wheel,
            fired: Vec::with_capacity(64),
            bus: None,
            persist: config.persist,
            metrics,
            streams: config.streams,
            region: config.region,
            sinks: Vec::new(),
            captured: std::cell::RefCell::new(FrameLog::new()),
            sim_feeds: Vec::new(),
            opened: Vec::new(),
            feeds: 0,
            next_id: 0,
            stopping: false,
        };
        Ok(Self {
            ctx,
            sources,
            from: config.from,
            to: config.to,
            delivered: Vec::new(),
            bound: 0,
        })
    }

    /// The agent's context, for set-up before [`Sim::run`].
    pub const fn ctx(&mut self) -> &mut Ctx {
        &mut self.ctx
    }

    /// Run `agent` over the whole input, or until `to` or [`Ctx::stop`].
    ///
    /// # Errors
    ///
    /// The agent's [`Agent::start`] failed.
    pub fn run<A: Agent>(&mut self, agent: &mut A) -> Result<(), Error> {
        agent.start(&mut self.ctx)?;
        while !self.ctx.stopping {
            self.rebind();
            let event = self.next_event();
            let timer = self.ctx.wheel.next_deadline();
            let end = self.to.unwrap_or(Nanos(i64::MAX));
            match (event, timer) {
                (Some((key, _)), Some(t)) if SimKey::timer(t.0, 0) < key && t <= end => {
                    self.fire(agent, t);
                }
                (None, Some(t)) if self.to.is_some() && t <= end => self.fire(agent, t),
                (Some((key, source)), _) if key.ts <= end.0 => self.dispatch(agent, source),
                _ => break,
            }
        }
        agent.stop(&mut self.ctx);
        Ok(())
    }

    /// The earliest head across the sources' bound streams, by the total
    /// order.
    fn next_event(&self) -> Option<(SimKey, (usize, usize))> {
        let mut best: Option<(SimKey, (usize, usize))> = None;
        for (i, source) in self.sources.iter().enumerate() {
            for (stream, feeds) in source.feeds.iter().enumerate() {
                let (Some(feed), Some(r)) = (feeds.first(), source.head(stream)) else {
                    continue;
                };
                let key = SimKey::event(
                    r.ts.0,
                    u64::from(feed.0),
                    i64::try_from(i).unwrap_or(i64::MAX),
                    i64::try_from(r.offset).unwrap_or(i64::MAX),
                );
                if best.is_none_or(|(b, _)| key < b) {
                    best = Some((key, (i, stream)));
                }
            }
        }
        best
    }

    fn dispatch<A: Agent>(&mut self, agent: &mut A, (source, stream): (usize, usize)) {
        let s = &self.sources[source];
        let Some(record) = s.head(stream) else {
            return;
        };
        let skip = self.from.is_some_and(|from| record.ts < from);
        if !skip {
            self.ctx.now = record.ts;
            for &feed in &s.feeds[stream] {
                let seen = &mut self.delivered[feed.0 as usize];
                let delivery = Delivery {
                    first: !*seen,
                    origin: Origin::Live,
                };
                *seen = true;
                agent.on_message(&mut self.ctx, feed, record.frame, delivery);
            }
        }
        self.sources[source].cursor[stream] += 1;
    }

    fn fire<A: Agent>(&mut self, agent: &mut A, at: Nanos) {
        self.ctx.now = at;
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
    }

    /// Point each source's streams at the feeds subscribed so far: a feed
    /// subscribed mid-run takes its frames from then on.
    fn rebind(&mut self) {
        let subscribed = self.ctx.sim_feeds.len();
        if subscribed == self.bound {
            return;
        }
        for source in &mut self.sources {
            source.bind(&self.ctx.sim_feeds, self.ctx.now);
        }
        self.delivered.resize(subscribed, false);
        self.bound = subscribed;
    }
}
