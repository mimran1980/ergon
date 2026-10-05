//! Record rows, metrics, and traces for the ingester.
//!
//! The ingester writes `ClickHouse`. The application does not talk to
//! `ClickHouse` and does not wait for it. [`Persist`] publishes on its own
//! stream, from the application's loop, on an exclusive publication of its
//! own that it adds through the application's [`Bus`]: one writer, no CAS on
//! the term tail. It keeps a handle to the Aeron client, to drive the
//! conductor while a record waits, and a copy of the application's identity,
//! not the bus: the runtime owns that, and closes persist's publication with
//! its outputs at shutdown.
//!
//! * [`Persist::record`] encodes an SBE message into the Aeron term. The table
//!   is the message name.
//! * [`Persist::record_row`] and [`Persist::record_value`] record rows of
//!   tables with no SBE message. See [`event`].
//! * [`mod@metrics`], [`trace`], and [`clock`](crate::clock) are the hot-path
//!   tools: [`Persist::metrics`] and [`Persist::tracer`].
//! * [`Persist::layer`] makes the [`bridge`]: `tracing` events
//!   that name a `table`, and spans, from any thread, on a publication of
//!   its own. It records nothing through this handle.
//!
//! The runtime owns the [`Persist`] and its loop calls the methods; there is
//! no process-wide handle.
//!
//! A disabled SBE table is one load, and `encode` is not called. An enabled
//! one is a claim, the encode, and a commit, after the application's `Source`
//! message the first time, and again once the stream had no subscriber. A
//! term rotation is tried eight times and then dropped.
//!
//! [`Persist::poll`] from the application's loop checks `tables.yaml` every
//! second (one `stat`, and a read only when the file changed), publishes the
//! metrics, and every 5 s sends the dictionary again (`Source`, the event
//! shapes, the trace definitions), a few messages a call. `enabled` is
//! applied here. `kind` is applied by the ingester.
//!
//! ```yaml
//! tables:
//!   trade:         { kind: static }
//!   book_snapshot: { kind: dynamic, enabled: false }
//! ```

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use rusteron_archive::{Aeron, AeronExclusivePublication};
use serde::Deserialize;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::registry::LookupSpan;

use crate::bus::{Bus, Claim, DropCounts, DropKind, Drops};
use crate::clock::{Clock, Nanos};
use crate::{Error, Settings, bridge, event, metrics, trace, value};

/// The stream applications publish on and the ingester's archive records.
pub const STREAM_ID: i32 = 1001;

/// IPC through the one media driver the application, the archive and the
/// ingester share. The 64 KiB MTU fits a message of up to 65 472 bytes in
/// one `try_claim`.
pub const CHANNEL: &str = "aeron:ipc?term-length=16m|mtu=65504";

/// Whether the ingester may change a table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableKind {
    /// Created if missing, never altered.
    Static,
    /// Created and extended to follow the schema.
    Dynamic,
}

/// Whether a table is recorded: `true`, `false`, or `{ until: <time> }`,
/// on until that RFC 3339 time (`2026-09-27T18:00:00Z`) and off after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawSwitch")]
pub enum Switch {
    /// Recorded.
    On,
    /// Not recorded.
    Off,
    /// Recorded until this instant, then not.
    Until(jiff::Timestamp),
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum RawSwitch {
    Flag(bool),
    Until { until: String },
}

impl TryFrom<RawSwitch> for Switch {
    type Error = String;

    fn try_from(raw: RawSwitch) -> Result<Self, String> {
        match raw {
            RawSwitch::Flag(on) => Ok(if on { Self::On } else { Self::Off }),
            RawSwitch::Until { until } => until
                .parse()
                .map(Self::Until)
                .map_err(|e| format!("until: {until:?} is not an RFC 3339 time: {e}")),
        }
    }
}

impl Switch {
    /// On at `now`?
    #[must_use]
    pub fn is_on(self, now: jiff::Timestamp) -> bool {
        match self {
            Self::On => true,
            Self::Off => false,
            Self::Until(end) => now < end,
        }
    }
}

/// One entry of `tables.yaml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    /// Static or dynamic.
    pub kind: TableKind,
    /// Record this table in every app not in [`TableConfig::apps`]
    /// (default `true`).
    #[serde(default = "on")]
    pub enabled: Switch,
    /// Apps that decide for themselves, by name (`PERSIST_APP`).
    #[serde(default)]
    pub apps: BTreeMap<String, Switch>,
    /// [`OTEL_TRACES`] only: each trace's sampling, by name. A trace not
    /// listed publishes every one while the table is on.
    #[serde(default)]
    pub traces: BTreeMap<String, TraceConfig>,
    /// The `ClickHouse` database the ingester keeps the table in; `None` is
    /// its default (`CLICKHOUSE_DATABASE`). Applications ignore it.
    #[serde(default)]
    pub database: Option<String>,
}

/// The table traces are published to, and switched by.
pub const OTEL_TRACES: &str = "otel_traces";

/// Which traces of one name are published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceConfig {
    /// One in this many (default 1: every one; 0: none).
    #[serde(default = "one")]
    pub sample: u64,
    /// Also every one slower than this: `50us`, `1ms`, `2s`.
    #[serde(default)]
    pub slower_than: Option<Threshold>,
}

const fn one() -> u64 {
    1
}

/// A duration in `tables.yaml`, in nanoseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Threshold(pub i64);

impl TryFrom<String> for Threshold {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let d: jiff::SignedDuration = text
            .parse()
            .map_err(|e| format!("{text:?} is not a duration such as 50us or 1ms: {e}"))?;
        i64::try_from(d.as_nanos())
            .map(Self)
            .map_err(|_| format!("{text:?} is too long"))
    }
}

const fn on() -> Switch {
    Switch::On
}

impl TableConfig {
    /// Is the table recorded by `app` at `now`?
    #[must_use]
    pub fn is_on(&self, app: &str, now: jiff::Timestamp) -> bool {
        self.apps
            .get(app)
            .copied()
            .unwrap_or(self.enabled)
            .is_on(now)
    }
}

/// Parse `tables.yaml`.
///
/// # Errors
///
/// `text` is not that file, or a duration such as `slower_than` does not parse.
pub fn parse_config(text: &str) -> Result<BTreeMap<String, TableConfig>, Error> {
    Config::parse(text).map(|config| config.tables)
}

/// Per-feed historical raw-frame recording, independent of decoded table switches.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedConfig {
    /// Store this feed's raw frames, doubling storage alongside decoded rows.
    #[serde(default)]
    pub frames: bool,
}

/// The recording configuration shared by applications and the ingester.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Table switches and schema management policy.
    pub tables: BTreeMap<String, TableConfig>,
    /// Exact `service/kind` names, with optional `*` fallback.
    #[serde(default)]
    pub feeds: BTreeMap<String, FeedConfig>,
}

impl Config {
    /// Parse tables and optional raw feed recording policy.
    ///
    /// # Errors
    /// The YAML or a feed name is invalid.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let config: Self = serde_yaml::from_str(text).map_err(|e| Error::Config(e.to_string()))?;
        for name in config.feeds.keys() {
            if name != "*"
                && !name.split_once('/').is_some_and(|(service, kind)| {
                    !service.is_empty() && !kind.is_empty() && !kind.contains('/')
                })
            {
                return Err(Error::Config(format!(
                    "feed must name service/kind or *: {name}"
                )));
            }
        }
        Ok(config)
    }
}

/// `(table name, template id)` of every message in an SBE schema.
fn schema_tables(xml: &str) -> Result<Vec<(String, u16)>, Error> {
    let ir = ergo_sbe::parse(xml).map_err(|e| Error::Schema(e.to_string()))?;
    ir.tokens
        .iter()
        .filter(|t| t.signal == ergo_sbe::Signal::BeginMessage)
        .map(|t| {
            t.id.map_or_else(
                || Err(Error::Schema(format!("message {} has no id", t.name))),
                |id| Ok((snake_case(&t.name), id)),
            )
        })
        .collect()
}

/// `bidPrice` -> `bid_price`, `BookSnapshot` -> `book_snapshot`.
#[must_use]
pub fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') && !out.ends_with('.') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// The recording handle. The runtime owns it; the loop records through it,
/// on its one thread: neither `Send` nor `Sync`.
#[derive(Clone)]
pub struct Persist {
    pub(crate) inner: Rc<Inner>,
}

/// Heartbeat messages one poll publishes at most.
const BEATS: usize = 8;

/// No heartbeat round under way.
const IDLE: usize = usize::MAX;

pub(crate) struct Inner {
    /// Only this handle claims on it: no CAS on the term tail.
    publication: AeronExclusivePublication,
    /// Stamps rows live.
    clock: Clock,
    simulation: Cell<bool>,
    sim_now: Cell<i64>,
    /// [`Persist::dropped`] when the simulation began.
    sim_drops: Cell<u64>,
    /// The next tracer's id namespace, counted from the start time live,
    /// so runs differ, and from 0 in simulation, so a replay mints the same
    /// trace ids.
    trace_ids: Cell<u64>,
    /// Simulation: when `tables.yaml` was last read, by the wall clock.
    wall_watch: Cell<Instant>,
    /// Largest `try_claim`. Longer records are dropped.
    max_payload: usize,
    /// The stream it records on; the `tracing` bridge publishes there too.
    stream_id: i32,
    /// Series and their publishing; see [`Persist::metrics`].
    metrics: metrics::Metrics,
    /// The application's client: driven while a record waits on Aeron.
    aeron: Aeron,
    /// The application's source id, stamped into every frame.
    source: u64,
    /// Its `Source` message goes ahead of the rows.
    source_message: Vec<u8>,
    /// This handle's records Aeron did not take, by reason; the
    /// `persist_dropped` series read them too.
    drops: DropCounts,
    /// The bus's feeds' drops, which [`Persist::drops`] adds.
    feeds: DropCounts,
    /// `Nanos` of the next [`Watcher::tick`].
    watch_due: Cell<i64>,
    watcher: RefCell<Watcher>,
    /// The SBE tables' switches, indexed by template id.
    enabled: Box<[Cell<bool>]>,
    /// Event tables' switches by name, including tables `tables.yaml` does
    /// not list (off). Call sites keep theirs.
    events: RefCell<HashMap<String, Rc<Cell<bool>>>>,
    /// Every event shape made so far, in the order made.
    pub(crate) shapes: RefCell<Vec<Rc<event::ShapeEntry>>>,
    /// `record_value`'s call sites.
    pub(crate) values: RefCell<value::Sites>,
    /// The `Source` message is in the stream this round: rows may follow.
    source_sent: Cell<bool>,
    /// The heartbeat round's next message: 0 is `Source`, then each shape,
    /// then each trace definition; [`IDLE`] between rounds.
    beat: Cell<usize>,
    /// Each trace's switch by name, set from `otel_traces` by the watcher.
    traces: RefCell<HashMap<String, Rc<trace::TraceSwitch>>>,
    /// Every trace definition made, re-sent with the shapes.
    trace_defs: RefCell<Vec<Rc<trace::DefMessage>>>,
    /// `otel_traces` is on for this app: a new tracer starts on.
    traces_on: Cell<bool>,
    /// `otel_traces`' sampling by trace name, as last applied: what a new
    /// tracer starts with.
    trace_rules: RefCell<BTreeMap<String, TraceConfig>>,
}

/// A heartbeat message after `Source`, and the flag its send sets.
enum Beat {
    Shape(Rc<event::ShapeEntry>),
    Def(Rc<trace::DefMessage>),
}

impl Persist {
    /// Read the schema and `tables.yaml`, and publish on `settings.channel`
    /// through `bus`, on an exclusive publication of this handle's.
    /// [`Persist::poll`] applies later edits.
    ///
    /// # Errors
    ///
    /// The schema or `tables.yaml` is invalid, the publication could not be
    /// added, or no subscriber recorded it within
    /// `settings.subscriber_timeout`.
    pub fn connect(schema_xml: &str, bus: &Bus, settings: Settings) -> Result<Self, Error> {
        type Read = fn(Drops) -> u64;
        let schema = schema_tables(schema_xml)?;
        let slots = schema.iter().map(|(_, id)| usize::from(*id) + 1).max();
        let metrics = metrics::Metrics::new(settings.metrics_interval);
        let drops = DropCounts::default();
        let reasons: [(&str, Read); 4] = [
            ("not_connected", |d| d.not_connected),
            ("back_pressure", |d| d.back_pressure),
            ("too_large", |d| d.too_large),
            ("other", |d| d.other),
        ];
        for (reason, read) in reasons {
            let (feeds, drops) = (bus.drop_counts().clone(), drops.clone());
            metrics.counter_fn("persist_dropped", &[("reason", reason)], move || {
                read(drops.get() + feeds.get())
            });
        }
        if let Some(now) = settings.sim_start {
            metrics.set_simulation(now);
        } else {
            metrics.start();
        }
        let publication = bus.add_exclusive_publication(&settings.channel, settings.stream_id)?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        wait_for_subscriber(bus, &publication, &settings)?;
        let clock = Clock::new();
        let started = clock.read();
        let sim_start = settings.sim_start;
        let persist = Self {
            inner: Rc::new(Inner {
                publication,
                clock,
                simulation: Cell::new(sim_start.is_some()),
                sim_now: Cell::new(sim_start.unwrap_or_default().0),
                sim_drops: Cell::new(bus.dropped()),
                trace_ids: Cell::new(sim_start.map_or_else(|| started.0.cast_unsigned(), |_| 0)),
                wall_watch: Cell::new(Instant::now()),
                max_payload,
                stream_id: settings.stream_id,
                metrics,
                aeron: bus.aeron().clone(),
                source: bus.source().id,
                source_message: bus.source_message().to_vec(),
                drops,
                feeds: bus.drop_counts().clone(),
                watch_due: Cell::new(sim_start.unwrap_or(started).0 + Watcher::EVERY_NS),
                watcher: RefCell::new(Watcher {
                    ticks: 0,
                    path: settings.config_path,
                    app: settings.app,
                    stamp: None,
                    text: String::new(),
                    config: BTreeMap::new(),
                    applied: false,
                    seen: Drops::default(),
                    schema,
                }),
                enabled: (0..slots.unwrap_or(0)).map(|_| Cell::new(false)).collect(),
                events: RefCell::default(),
                shapes: RefCell::default(),
                values: RefCell::default(),
                source_sent: Cell::new(false),
                // Due at once: the first round starts the stream.
                beat: Cell::new(0),
                traces: RefCell::default(),
                trace_defs: RefCell::default(),
                traces_on: Cell::new(false),
                trace_rules: RefCell::default(),
            }),
        };
        persist.inner.watcher.borrow_mut().reload()?;
        persist.apply(
            sim_start
                .and_then(|n| jiff::Timestamp::from_nanosecond(i128::from(n.0)).ok())
                .unwrap_or_else(jiff::Timestamp::now),
        );
        Ok(persist)
    }

    /// Publish the metrics that are due, apply `tables.yaml` once a second,
    /// and every 5 s send the dictionary again: at most one metrics message
    /// and 8 dictionary messages a call. Call it from the application's loop
    /// with its clock's time: nothing else does. Until something is due,
    /// three compares.
    #[inline]
    pub fn poll(&self, now: Nanos) {
        self.inner.metrics.poll_through(now, self);
        if now.0 >= self.inner.watch_due.get() {
            self.watch(now);
        }
        if self.inner.beat.get() != IDLE {
            self.beats(BEATS);
        }
    }

    #[cold]
    fn watch(&self, now: Nanos) {
        // Reached from inside a publish: the outer call applies.
        let Ok(mut watcher) = self.inner.watcher.try_borrow_mut() else {
            return;
        };
        self.inner.watch_due.set(now.0 + Watcher::EVERY_NS);
        watcher.tick(self, now);
    }

    /// Switch every table as `tables.yaml`, as last read, says at `now`.
    fn apply(&self, now: jiff::Timestamp) {
        // Reached from inside a publish: the outer call applies.
        if let Ok(mut watcher) = self.inner.watcher.try_borrow_mut() {
            watcher.apply(self, now);
        }
    }

    /// Enable retrying transport pressure and apply table switches at the simulation epoch.
    pub fn set_simulation(&self, now: Nanos) {
        let inner = &self.inner;
        inner.sim_drops.set(self.dropped());
        if !inner.simulation.replace(true) {
            // Tracers made from here on count from 0, as a replay's do.
            inner.trace_ids.set(0);
        }
        inner.sim_now.set(now.0);
        inner.watch_due.set(now.0.saturating_add(Watcher::EVERY_NS));
        inner.metrics.set_simulation(now);
        if let Ok(timestamp) = jiff::Timestamp::from_nanosecond(i128::from(now.0)) {
            self.apply(timestamp);
        }
    }

    /// Advance the timestamp used by event rows before agent dispatch.
    #[inline]
    pub fn sim_time(&self, now: Nanos) {
        self.inner.sim_now.set(now.0);
        self.inner.metrics.sim_time(now);
    }

    /// Current simulation epoch, or a read of this handle's clock.
    #[inline]
    #[must_use]
    pub fn now(&self) -> Nanos {
        if self.inner.simulation.get() {
            Nanos(self.inner.sim_now.get())
        } else {
            self.inner.clock.read()
        }
    }

    /// Publish metrics at simulation time; gate config I/O and heartbeats by wall elapsed time.
    pub fn poll_sim(&self, now: Nanos) {
        self.sim_time(now);
        let inner = &self.inner;
        inner.metrics.poll_through(now, self);
        if now.0 >= inner.watch_due.get() {
            inner.watch_due.set(now.0.saturating_add(Watcher::EVERY_NS));
            if let Ok(timestamp) = jiff::Timestamp::from_nanosecond(i128::from(now.0)) {
                self.apply(timestamp);
            }
        }
        if inner.wall_watch.get().elapsed() >= Duration::from_secs(1)
            && let Ok(mut watcher) = inner.watcher.try_borrow_mut()
        {
            inner.wall_watch.set(Instant::now());
            watcher.tick(self, now);
        }
        if inner.beat.get() != IDLE {
            self.beats(BEATS);
        }
    }

    /// Finish the dictionary round under way and partial metrics at
    /// simulation EOF.
    ///
    /// # Errors
    /// Any record was dropped since the simulation began, the final metrics
    /// included: a simulation waits out back pressure, so a drop is a record
    /// the run lost (too large or mis-sized; a record after the publication
    /// closed is not counted). Or a dictionary message could not be
    /// published.
    pub fn flush_sim(&self, now: Nanos) -> Result<(), Error> {
        self.sim_time(now);
        let drops = self.inner.sim_drops.get();
        self.inner.metrics.flush_through(now, self);
        let lost = self.dropped().saturating_sub(drops);
        if lost != 0 {
            return Err(Error::Aeron(format!(
                "{lost} records were dropped during the simulation"
            )));
        }
        if !self.beats(usize::MAX) {
            return Err(Error::Aeron("failed to flush simulation dictionary".into()));
        }
        Ok(())
    }

    /// A checkpoint trace (see [`trace`]): make it once, then
    /// [`trace::Tracer::start`] one per event. At most
    /// [`trace::MAX_STAGES`] stages and [`trace::MAX_ATTRS`] attributes.
    #[must_use]
    pub fn tracer(&self, name: &str, stages: &[&str], attrs: &[&str]) -> trace::Tracer {
        let def = trace::TraceDef::new(name, stages, attrs);
        let bytes = def.message().unwrap_or_else(|e| {
            log::error!("trace {name}: {e}");
            Vec::new()
        });
        let inner = &self.inner;
        // One definition however many tracers share it.
        let known = inner
            .trace_defs
            .borrow()
            .iter()
            .find(|d| d.message == bytes)
            .cloned();
        let message = known.unwrap_or_else(|| {
            let (len, max) = (bytes.len(), inner.max_payload);
            if len > max {
                log::error!(
                    "trace {name}: its definition is {len} bytes, more than one claim holds ({max}): its traces are dropped"
                );
            }
            let made = Rc::new(trace::DefMessage {
                message: bytes,
                sent: Cell::new(false),
            });
            inner.trace_defs.borrow_mut().push(Rc::clone(&made));
            made
        });
        // A new switch starts as the watcher last decided.
        let switch = Rc::clone(
            inner
                .traces
                .borrow_mut()
                .entry(name.to_owned())
                .or_insert_with(|| {
                    let rule = inner.trace_rules.borrow().get(name).copied();
                    let switch = trace::TraceSwitch::default();
                    switch.set(inner.traces_on.get(), rule.as_ref());
                    Rc::new(switch)
                }),
        );
        let nonce = inner.trace_ids.get();
        inner.trace_ids.set(nonce.wrapping_add(1));
        trace::Tracer::new(
            &def,
            message,
            switch,
            &inner.metrics,
            Some(self.clone()),
            inner.source.rotate_left(17) ^ nonce,
        )
    }

    /// This application's metrics: make counters, gauges and histograms
    /// from it, and call [`Persist::poll`] from the loop.
    #[must_use]
    pub fn metrics(&self) -> metrics::Metrics {
        self.inner.metrics.clone()
    }

    /// Is `template_id` recorded right now? One load. Check it before
    /// preparing a message that is costly to size or encode.
    #[inline]
    #[must_use]
    pub fn enabled(&self, template_id: u16) -> bool {
        self.inner
            .enabled
            .get(usize::from(template_id))
            .is_some_and(Cell::get)
    }

    /// Record one message of `template_id` that is exactly `len` bytes,
    /// header included (the generated `compute_length_with_header`). If its
    /// table is enabled, `encode` writes the message into a slot of exactly
    /// `len` bytes of the Aeron term buffer and returns the length it wrote;
    /// otherwise `encode` is never called.
    ///
    /// The record is dropped and counted in [`Persist::drops`] when Aeron
    /// cannot take it, or the `Source` message that goes first (no
    /// subscriber yet, back pressure, larger than one claim), when a term
    /// rotation does not finish within eight retries, when `encode` wrote
    /// another length or template than claimed (debug builds panic on
    /// that). An error from `encode` is returned as is, and nothing is
    /// published. After [`Invoker::finish`](crate::rt::Invoker::finish)
    /// closed the publication, nothing is published or counted.
    ///
    /// # Errors
    ///
    /// Returns the error from `encode`.
    #[inline]
    pub fn record<E>(
        &self,
        template_id: u16,
        len: usize,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
    ) -> Result<(), E> {
        if !self.enabled(template_id) {
            return Ok(());
        }
        if len > self.inner.max_payload {
            self.count(DropKind::TooLarge);
            return Ok(());
        }
        let claim = match self.sourced().and_then(|()| self.try_claim_slot(len)) {
            Ok(claim) => claim,
            Err(kind) => {
                self.count(kind);
                return Ok(());
            }
        };
        let slot = claim.data();
        // An `encode` error drops the claim, which aborts it.
        let written = encode(slot)?;
        let honest = written == len && slot.get(2..4) == Some(&template_id.to_le_bytes()[..]);
        debug_assert!(
            honest,
            "encode wrote {written} bytes of template {}, but claimed {len} bytes of {template_id}",
            u16::from_le_bytes([slot[2], slot[3]])
        );
        if !(honest && claim.commit().is_ok()) {
            self.drop_one();
        }
        Ok(())
    }

    /// The `tracing` bridge ([`bridge`]): a layer that records, from any
    /// thread, events naming a table
    /// (`tracing::info!(table = "signal", instrument = %id, edge = 0.25)`)
    /// as rows, and spans at INFO and above (below INFO: libraries'
    /// internals) as `otel_traces` spans, on a concurrent publication of its
    /// own on this stream. It records nothing through this handle and
    /// switches nothing: the ingester keeps what `tables.yaml` has on for
    /// this app. Make it once, on the loop thread at start-up (it drives the
    /// conductor until its publication is added), and add it to the
    /// application's subscriber. Its filter is its own: it takes only those,
    /// and leaves every other call site to the other layers (or disabled,
    /// when it is alone).
    ///
    /// # Errors
    ///
    /// The bridge's publication could not be added: the driver refuses
    /// another concurrent publication of this stream that is not on
    /// [`bridge::BRIDGE_CHANNEL`].
    pub fn layer<S>(&self) -> Result<impl Layer<S> + Send + Sync + 'static, Error>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let inner = &self.inner;
        Ok(bridge::Bridge::new(
            &inner.aeron,
            inner.source,
            &inner.source_message,
            inner.stream_id,
        )?
        .with_filter(filter_fn(bridge::wants)))
    }

    /// Is the event table `table` recorded right now? Check it before
    /// building a row that is costly to make.
    #[must_use]
    pub fn event_enabled(&self, table: &str) -> bool {
        self.inner
            .events
            .try_borrow()
            .is_ok_and(|events| events.get(table).is_some_and(|on| on.get()))
    }

    /// The switch of event table `table`, made (off) if `tables.yaml` does
    /// not list it; the config watcher flips it.
    pub(crate) fn event_switch(&self, table: &str) -> Rc<Cell<bool>> {
        let mut events = self.inner.events.borrow_mut();
        if let Some(on) = events.get(table) {
            return Rc::clone(on);
        }
        Rc::clone(events.entry(table.to_owned()).or_default())
    }

    /// Publish a message that is already built, after the `Source` message.
    /// `false` when it was not published; the drop is already counted.
    pub(crate) fn publish(&self, bytes: &[u8]) -> bool {
        self.sourced()
            .and_then(|()| self.publish_raw(bytes))
            .map_err(|kind| self.count(kind))
            .is_ok()
    }

    /// Claim exactly `len` bytes, after the `Source` message, let `write`
    /// fill them, and commit when `write` returns `true`. A `false` write
    /// aborts the claim. Either failure is counted.
    pub(crate) fn claim(&self, len: usize, write: impl FnOnce(&mut [u8]) -> bool) {
        if len > self.inner.max_payload {
            self.count(DropKind::TooLarge);
            return;
        }
        let claim = match self.sourced().and_then(|()| self.try_claim_slot(len)) {
            Ok(claim) => claim,
            Err(kind) => {
                self.count(kind);
                return;
            }
        };
        if !write(claim.data()) || claim.commit().is_err() {
            self.drop_one();
        }
    }

    /// The `Source` message ahead of this round's first record: one load
    /// once it is out. `Err` when Aeron could not take it, and nothing may
    /// follow it yet.
    #[inline]
    fn sourced(&self) -> Result<(), DropKind> {
        if self.inner.source_sent.get() {
            return Ok(());
        }
        self.send_source()
    }

    #[cold]
    fn send_source(&self) -> Result<(), DropKind> {
        self.publish_raw(&self.inner.source_message)?;
        self.inner.source_sent.set(true);
        Ok(())
    }

    /// Publish up to `n` messages of the heartbeat round under way, from
    /// the bytes they keep: `Source` (unless a record sent it this round),
    /// each shape, then each trace definition, `Source` first again if the
    /// stream lost its subscriber meanwhile. They repeat, so one Aeron
    /// cannot take is not a drop. With no subscriber or back pressure it is
    /// the next one tried, at the next poll. One refused otherwise is passed
    /// over, and the round goes on: longer than a claim (logged once, when
    /// it was made; a length compare, no claim), the publication closed, or
    /// another error. `false` when any of them happened.
    #[cold]
    fn beats(&self, n: usize) -> bool {
        let inner = &self.inner;
        let mut all = true;
        for _ in 0..n {
            let at = inner.beat.get();
            if at == IDLE {
                return all;
            }
            let published = match at.checked_sub(1).map(|i| self.dictionary(i)) {
                None => self.sourced(),
                Some(None) => {
                    inner.beat.set(IDLE);
                    return all;
                }
                Some(Some(beat)) => {
                    let (bytes, sent) = match &beat {
                        Beat::Shape(entry) => (entry.shape.message(), &entry.sent),
                        Beat::Def(def) => (def.message.as_slice(), &def.sent),
                    };
                    let published = self.sourced().and_then(|()| self.publish_raw(bytes));
                    published.map(|()| sent.set(true))
                }
            };
            match published {
                Ok(()) => {}
                Err(DropKind::NotConnected | DropKind::BackPressure) => return false,
                Err(DropKind::TooLarge | DropKind::Other | DropKind::Closed) => all = false,
            }
            inner.beat.set(at + 1);
        }
        all
    }

    /// The heartbeat round's message `i` after `Source`: each shape, then
    /// each trace definition. `None` past the last, or when reached while
    /// one is being added, which ends the round.
    fn dictionary(&self, i: usize) -> Option<Beat> {
        let shapes = self.inner.shapes.try_borrow().ok()?;
        if let Some(entry) = shapes.get(i) {
            return Some(Beat::Shape(Rc::clone(entry)));
        }
        let i = i - shapes.len();
        drop(shapes);
        let defs = self.inner.trace_defs.try_borrow().ok()?;
        defs.get(i).cloned().map(Beat::Def)
    }

    /// Persist one runtime Input journal row independently of application
    /// table switches, after the `Source` message as every record, so the
    /// ingester knows the row's run before it stores it.
    pub(crate) fn record_input(&self, input: crate::journal::Input) -> Result<(), Error> {
        let mut stalled = None;
        let claim = loop {
            match self
                .sourced()
                .and_then(|()| self.try_claim_slot(crate::journal::Input::LENGTH))
            {
                Ok(claim) => break claim,
                Err(DropKind::BackPressure | DropKind::NotConnected) => {
                    let began = stalled.get_or_insert_with(Instant::now);
                    if began.elapsed() >= Duration::from_secs(5) {
                        return Err(Error::Aeron("input journal remained back pressured or disconnected for five seconds".into()));
                    }
                    crate::bus::pause(&self.inner.aeron);
                }
                Err(kind) => {
                    return Err(Error::Aeron(format!(
                        "input journal claim failed: {kind:?}"
                    )));
                }
            }
        };
        if input.encode(claim.data()) != crate::journal::Input::LENGTH {
            return Err(Error::Aeron("input journal encode length mismatch".into()));
        }
        claim
            .commit()
            .map_err(|e| Error::Aeron(format!("input journal commit failed: {e}")))
    }

    /// Publish a built message; why not, when Aeron could not take it.
    fn publish_raw(&self, bytes: &[u8]) -> Result<(), DropKind> {
        if bytes.len() > self.inner.max_payload {
            return Err(DropKind::TooLarge);
        }
        let claim = self.try_claim_slot(bytes.len())?;
        claim.data().copy_from_slice(bytes);
        claim.commit().map_err(|_| DropKind::Other)
    }

    /// Claim `len` bytes, stamped with this application's source id.
    #[inline]
    fn try_claim_slot(&self, len: usize) -> Result<Claim, DropKind> {
        let inner = &self.inner;
        loop {
            match crate::bus::claim_exclusive(&inner.publication, len, inner.source.cast_signed()) {
                Ok(claim) => return Ok(claim),
                Err(DropKind::BackPressure | DropKind::NotConnected) if inner.simulation.get() => {
                    crate::bus::pause(&inner.aeron);
                }
                Err(kind) => return Err(self.refused(kind)),
            }
        }
    }

    /// Why a claim was refused. After the runtime closed the publication at
    /// shutdown, it is closed, which is not a drop. With no subscriber,
    /// perhaps a new recording follows: the `Source` message goes again
    /// before the next record.
    #[cold]
    fn refused(&self, kind: DropKind) -> DropKind {
        if self.inner.publication.is_closed() {
            return DropKind::Closed;
        }
        if kind == DropKind::NotConnected {
            self.inner.source_sent.set(false);
        }
        kind
    }

    /// One more drop of `kind`, unless the runtime closed the publication at
    /// shutdown: a record after that is not a drop, whatever stopped it.
    pub(crate) fn count(&self, kind: DropKind) {
        if !self.inner.publication.is_closed() {
            self.inner.drops.count(kind);
        }
    }

    /// The longest message one claim holds.
    pub(crate) fn max_payload(&self) -> usize {
        self.inner.max_payload
    }

    /// Is the archive recording this stream yet? Until it is, every record
    /// is dropped and counted.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.inner.publication.is_connected()
    }

    #[cold]
    pub(crate) fn drop_one(&self) {
        self.count(DropKind::Other);
    }

    /// Records dropped so far, by reason: this handle's, and those of the
    /// feeds published through the bus it connected with ([`Bus::drops`]).
    /// What the `persist_dropped` series publish; also logged, once a second
    /// while the total grows.
    #[must_use]
    pub fn drops(&self) -> Drops {
        self.inner.drops.get() + self.inner.feeds.get()
    }

    /// [`Drops::total`] of [`Persist::drops`].
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.drops().total()
    }

    /// Its publication, for the runtime to close at shutdown: closing this
    /// handle closes every one, and a record after it is not a drop.
    pub(crate) fn publication(&self) -> AeronExclusivePublication {
        self.inner.publication.clone()
    }
}

impl std::fmt::Debug for Persist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Persist")
            .field("source", &self.inner.source)
            .field("dropped", &self.dropped())
            .finish_non_exhaustive()
    }
}

fn wait_for_subscriber(
    bus: &Bus,
    publication: &AeronExclusivePublication,
    settings: &Settings,
) -> Result<(), Error> {
    if settings.subscriber_timeout.is_zero() {
        return Ok(());
    }
    let deadline = Instant::now() + settings.subscriber_timeout;
    while !publication.is_connected() {
        if Instant::now() >= deadline {
            return Err(Error::Aeron(format!(
                "{}: no subscriber is recording the stream after {:?}",
                settings.channel, settings.subscriber_timeout
            )));
        }
        bus.pause();
    }
    Ok(())
}

/// Applies `tables.yaml` to the switches of the [`Persist`] it is handed,
/// from [`Persist::poll`].
struct Watcher {
    ticks: u64,
    path: PathBuf,
    /// This application's name, for the tables' `apps` switches.
    app: String,
    /// The file's length and modification time when it was last read.
    stamp: Option<(u64, SystemTime)>,
    /// The last `tables.yaml` read, good or bad, and the last good one's
    /// tables.
    text: String,
    config: BTreeMap<String, TableConfig>,
    /// The switches have been applied once: after that, only changes are logged.
    applied: bool,
    seen: Drops,
    schema: Vec<(String, u16)>,
}

impl Watcher {
    const EVERY_NS: i64 = 1_000_000_000;

    fn tick(&mut self, persist: &Persist, now: Nanos) {
        let inner = &persist.inner;
        // Every 5 s, start a round that sends the `Source` message, every
        // event shape and every trace definition again. An ingester that
        // starts after the first one, with none saved, learns them from it.
        self.ticks += 1;
        if self.ticks.is_multiple_of(5) {
            inner.source_sent.set(false);
            inner.beat.set(0);
        }
        if let Err(e) = self.reload() {
            log::error!("{e}; keeping the previous configuration");
        }
        // Every second, changed or not: an `until` passes by itself.
        if let Ok(timestamp) = jiff::Timestamp::from_nanosecond(i128::from(now.0)) {
            self.apply(persist, timestamp);
        }
        let drops = persist.drops();
        if drops.total() > self.seen.total() {
            log::warn!(
                "{} records dropped in the last second ({} in all: {} not connected, {} back pressure, {} too large, {} other)",
                drops.total() - self.seen.total(),
                drops.total(),
                drops.not_connected,
                drops.back_pressure,
                drops.too_large,
                drops.other,
            );
            self.seen = drops;
        }
    }

    /// Read `tables.yaml` again if it changed since the last call: one
    /// `stat` while it has not, so the loop allocates nothing. A bad version
    /// is reported once.
    fn reload(&mut self) -> Result<(), Error> {
        let stamp = std::fs::metadata(&self.path)
            .ok()
            .and_then(|meta| Some((meta.len(), meta.modified().ok()?)));
        if stamp.is_some() && stamp == self.stamp {
            return Ok(());
        }
        let in_file =
            |e: &dyn std::fmt::Display| Error::Config(format!("{}: {e}", self.path.display()));
        let text = std::fs::read_to_string(&self.path).map_err(|e| in_file(&e))?;
        self.stamp = stamp;
        if text != self.text {
            let parsed = parse_config(&text).map_err(|e| match e {
                Error::Config(m) => in_file(&m),
                other => other,
            });
            self.text = text;
            self.config = parsed?;
        }
        Ok(())
    }

    /// Switch every table on or off for this app at `now`.
    fn apply(&mut self, persist: &Persist, now: jiff::Timestamp) {
        let inner = &persist.inner;
        let first = !std::mem::replace(&mut self.applied, true);
        let (app, config) = (&self.app, &self.config);
        let toggled = |name: &str, was: bool, on: bool| {
            if was == on && !(first && on) {
                return;
            }
            let why = match config
                .get(name)
                .map(|c| *c.apps.get(app).unwrap_or(&c.enabled))
            {
                Some(Switch::Until(end)) if on => format!(" until {end}"),
                Some(Switch::Until(end)) => format!(" ({end} passed)"),
                _ => String::new(),
            };
            log::info!("recording {name}: {}{why}", if on { "on" } else { "off" });
        };
        let is_on = |name: &str| config.get(name).is_some_and(|c| c.is_on(app, now));
        for (name, id) in &self.schema {
            let on = is_on(name);
            toggled(name, inner.enabled[usize::from(*id)].replace(on), on);
        }
        // Traces: `otel_traces` switches them all, and names their sampling.
        let traces = config.get(OTEL_TRACES);
        let traces_on = is_on(OTEL_TRACES);
        // First what a new tracer starts with, then every existing one.
        // Copied only when they changed: this runs every second.
        inner.traces_on.set(traces_on);
        let mut rules = inner.trace_rules.borrow_mut();
        match traces {
            Some(c) if c.traces != *rules => rules.clone_from(&c.traces),
            None if !rules.is_empty() => rules.clear(),
            _ => {}
        }
        drop(rules);
        for (name, switch) in inner.traces.borrow().iter() {
            switch.set(traces_on, traces.and_then(|c| c.traces.get(name)));
        }
        // Every other table is an event table (`record_row`,
        // `record_value`). Logged once the map is released. A switch is made
        // only for a table seen for the first time, and a note kept only for
        // what may be logged: a second that changed nothing allocates
        // nothing.
        let notes = {
            let mut switches = inner.events.borrow_mut();
            let mut notes = Vec::new();
            for (name, switch) in switches.iter() {
                if !config.contains_key(name) && switch.replace(false) {
                    notes.push((name.clone(), true, false));
                }
            }
            for name in config
                .keys()
                .filter(|name| !self.schema.iter().any(|(n, _)| n == *name))
            {
                let on = is_on(name);
                let known = switches.get(name).map(|switch| switch.replace(on));
                if known.is_none() {
                    switches.insert(name.clone(), Rc::new(Cell::new(on)));
                }
                let was = known.unwrap_or(false);
                if first || was != on {
                    notes.push((name.clone(), was, on));
                }
            }
            notes
        };
        for (name, was, on) in &notes {
            toggled(name, *was, *on);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn a_table_is_switched_per_app_and_until_a_time() -> TestResult {
        let config = parse_config(
            "tables:
  trade: { kind: static }
  book: { kind: dynamic, enabled: false }
  fills:
    kind: dynamic
    enabled: false
    apps: { binance: true, deribit: { until: 2026-09-27T18:00:00Z } }
  signal: { kind: dynamic, enabled: { until: 2026-09-27T18:00:00Z }, apps: { okx: false } }
",
        )?;
        let before: jiff::Timestamp = "2026-09-27T17:59:59Z".parse()?;
        let after: jiff::Timestamp = "2026-09-27T18:00:00Z".parse()?;
        let on = |table: &str, app: &str, now| config[table].is_on(app, now);

        assert!(
            on("trade", "binance", after) && on("trade", "", after),
            "default: every app"
        );
        assert!(!on("book", "binance", before), "enabled: false: no app");
        // Only the apps listed, the second only until its time.
        assert!(on("fills", "binance", after));
        assert!(on("fills", "deribit", before) && !on("fills", "deribit", after));
        assert!(
            !on("fills", "bybit", before),
            "an app not listed takes `enabled`"
        );
        // Every app until a time, except one that is off.
        assert!(on("signal", "bybit", before) && !on("signal", "bybit", after));
        assert!(!on("signal", "okx", before));
        assert_eq!(config["trade"].kind, TableKind::Static);

        for bad in [
            "tables:\n  t: { kind: dynamic, enabled: { until: tomorrow } }\n",
            "tables:\n  t: { kind: dynamic, apps: { a: { until: 2026-09-27T18:00:00Z, x: 1 } } }\n",
            "tables:\n  t: { kind: dynamic, apps: { a: yes } }\n",
            "tables:\n  t: { kind: dynamic, until: 2026-09-27T18:00:00Z }\n",
        ] {
            assert!(parse_config(bad).is_err(), "accepted {bad}");
        }
        Ok(())
    }

    #[test]
    fn a_table_names_its_database_or_takes_the_default() -> TestResult {
        let config = parse_config(
            "tables:\n  trade: { kind: static, database: md }\n  ema: { kind: dynamic }\n",
        )?;
        assert_eq!(config["trade"].database.as_deref(), Some("md"));
        assert_eq!(config["ema"].database, None);
        Ok(())
    }

    #[test]
    fn traces_are_sampled_by_name_with_durations() -> TestResult {
        let config = parse_config(
            "tables:
  otel_traces:
    kind: static
    enabled: false
    apps: { binance: true }
    traces:
      t2t: { sample: 1000, slower_than: 50us }
      order: { slower_than: 1ms }
      rare: { slower_than: 2s }
",
        )?;
        let traces = &config[OTEL_TRACES].traces;
        assert_eq!(
            traces["t2t"],
            TraceConfig {
                sample: 1000,
                slower_than: Some(Threshold(50_000))
            }
        );
        assert_eq!(traces["order"].sample, 1, "every one by default");
        assert_eq!(traces["order"].slower_than, Some(Threshold(1_000_000)));
        assert_eq!(traces["rare"].slower_than, Some(Threshold(2_000_000_000)));
        for bad in ["fast", "50", "50 parsecs"] {
            let text = format!(
                "tables:\n  otel_traces: {{ kind: static, traces: {{ t: {{ slower_than: {bad} }} }} }}\n"
            );
            assert!(parse_config(&text).is_err(), "accepted slower_than: {bad}");
        }
        Ok(())
    }

    #[test]
    fn snake_case_names() {
        assert_eq!(snake_case("BookSnapshot"), "book_snapshot");
        assert_eq!(snake_case("bidPrice"), "bid_price");
        assert_eq!(snake_case("tsEvent"), "ts_event");
        assert_eq!(snake_case("price"), "price");
    }
}
