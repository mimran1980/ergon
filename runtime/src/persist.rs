//! Record rows, metrics, and traces for the ingester.
//!
//! The ingester writes `ClickHouse`. The application does not talk to
//! `ClickHouse` and does not wait for it. [`Persist`] publishes on its own
//! stream through the application's [`Bus`].
//!
//! * [`Persist::record`] encodes an SBE message into the Aeron term. The table
//!   is the message name.
//! * [`Persist::layer`] records `tracing` events that set `table`, and
//!   `counter`, `gauge`, or `histogram`. Columns and labels are the other
//!   fields. See [`event`].
//! * [`mod@metrics`], [`trace`], and [`clock`](crate::clock) are the hot-path
//!   tools. The same metrics and span traces can also be emitted with the
//!   `tracing` macros.
//!
//! Keep a [`Persist`] instance and use its methods, [`Persist::metrics`],
//! [`Persist::tracer`], and [`Persist::layer`] without installing a static.
//! The layer owns a clone of that instance; tracing macros use its registry.
//! Poll that instance's metrics to publish macro updates.
//!
//! Optionally [`Persist::install`] one handle per process. The free functions then work
//! from any thread. With nothing installed they do nothing and do not call
//! `encode`.
//!
//! A disabled SBE table is one relaxed load, and `encode` is not called. An
//! enabled one is a claim, the encode, and a commit. If a dictionary heartbeat
//! is queued, the call also publishes one of those messages. A term rotation
//! is tried eight times and then dropped.
//!
//! [`Persist::poll`] from the application's loop re-reads `tables.yaml` every
//! second and publishes the metrics. `enabled` is applied here. `kind` is
//! applied by the ingester.
//!
//! ```yaml
//! tables:
//!   trade:         { kind: static }
//!   book_snapshot: { kind: dynamic, enabled: false }
//! ```

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};
use std::time::Instant;

use rusteron_archive::AeronPublication;
use serde::Deserialize;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::dynamic_filter_fn;
use tracing_subscriber::registry::LookupSpan;

use crate::bus::{Bus, Claim, DropKind, Drops};
use crate::clock::{Clock, Nanos};
use crate::{Error, Settings, event, metrics, spans, trace};

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
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        tables: BTreeMap<String, TableConfig>,
    }
    serde_yaml::from_str::<File>(text)
        .map(|f| f.tables)
        .map_err(|e| Error::Config(e.to_string()))
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

/// The handle the free functions use; see [`Persist::install`].
static INSTALLED: OnceLock<Persist> = OnceLock::new();

/// The handle [`Persist::install`] installed, if any.
#[inline]
#[must_use]
pub fn installed() -> Option<&'static Persist> {
    INSTALLED.get()
}

/// [`Persist::record`] with the installed handle. With none installed it
/// does nothing and never calls `encode`.
///
/// # Errors
///
/// Returns the error from `encode`.
#[inline]
pub fn record<E>(
    template_id: u16,
    len: usize,
    encode: impl FnOnce(&mut [u8]) -> Result<usize, E>,
) -> Result<(), E> {
    INSTALLED.get().map_or_else(
        || Ok(()),
        |persist| persist.record(template_id, len, encode),
    )
}

/// [`Persist::enabled`] with the installed handle; `false` with none.
#[inline]
#[must_use]
pub fn enabled(template_id: u16) -> bool {
    INSTALLED.get().is_some_and(|p| p.enabled(template_id))
}

/// [`Persist::event_enabled`] with the installed handle; `false` with none.
#[inline]
#[must_use]
pub fn event_enabled(table: &str) -> bool {
    INSTALLED.get().is_some_and(|p| p.event_enabled(table))
}

/// [`Persist::record_row`] with the installed handle. With none installed it
/// does nothing.
pub fn record_row<'a>(table: &str, fields: impl IntoIterator<Item = (&'a str, event::Value<'a>)>) {
    if let Some(persist) = INSTALLED.get() {
        persist.record_row(table, fields);
    }
}

/// [`Persist::record_value`] with the installed handle. With none installed
/// it does nothing.
#[inline]
pub fn record_value<T: ?Sized + serde::Serialize>(table: &str, value: &T) {
    if let Some(persist) = INSTALLED.get() {
        persist.record_value(table, value);
    }
}

/// [`Persist::metrics`] of the installed handle. With none installed, a
/// registry that publishes nothing: its handles still work.
#[must_use]
pub fn metrics() -> metrics::Metrics {
    INSTALLED
        .get()
        .map_or_else(metrics::Metrics::detached, Persist::metrics)
}

/// [`Persist::tracer`] of the installed handle. With none installed, a
/// tracer that publishes nothing: its stage histograms still count.
#[must_use]
pub fn tracer(name: &str, stages: &[&str], attrs: &[&str]) -> trace::Tracer {
    INSTALLED.get().map_or_else(
        || {
            trace::Tracer::new(
                &trace::TraceDef::new(name, stages, attrs),
                Arc::new(trace::DefMessage {
                    message: Vec::new(),
                    sent: AtomicBool::new(true),
                }),
                Arc::default(),
                &metrics::Metrics::detached(),
                None,
                0,
            )
        },
        |persist| persist.tracer(name, stages, attrs),
    )
}

/// The recording handle. Cheap to clone; share it with every callback, or
/// [`install`](Persist::install) it once and use the free functions.
#[derive(Clone)]
pub struct Persist {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    /// Unique per `Persist` ever made: keys the per-thread call-site cache,
    /// which an address could not (a new `Persist` may reuse an old one's).
    pub(crate) id: u64,
    publication: Publication,
    /// Exclusive mode: records from other threads, published by the owner's
    /// [`Persist::poll`].
    handoff: Option<Handoff>,
    /// Largest `try_claim`. Longer records are dropped.
    max_payload: usize,
    /// Series and their publishing; see [`Persist::metrics`].
    metrics: metrics::Metrics,
    /// Its `Source` message is sent ahead of the shapes.
    bus: Bus,
    pub(crate) shared: Arc<Shared>,
    /// `Nanos` of the next [`Watcher::tick`].
    watch_due: AtomicI64,
    watcher: Mutex<Watcher>,
}

/// The publication persist records on.
enum Publication {
    /// Any thread may record.
    Shared(AeronPublication),
    /// Only its owner claims: no CAS on the term tail.
    Exclusive(crate::owned::OwnedPublication),
}

/// Records made off the owner thread in exclusive mode: copied, queued, and
/// published by the owner. Off the hot path by construction (a `tracing`
/// span from a library's worker thread, say), so a copy is fine.
struct Handoff {
    send: SyncSender<Vec<u8>>,
    receive: Mutex<Receiver<Vec<u8>>>,
}

/// Messages a hand-off holds before it drops and counts.
const HANDOFF: usize = 4_096;

/// One message of a heartbeat round.
struct Beat {
    bytes: Vec<u8>,
    shape: Option<Arc<event::Shape>>,
    def: Option<Arc<trace::DefMessage>>,
}

/// State the config watcher writes and the hot path reads.
pub(crate) struct Shared {
    /// Indexed by SBE template id.
    enabled: Vec<AtomicBool>,
    /// Event tables' switches by name, including tables `tables.yaml` does
    /// not list (off). Call sites cache theirs, so the lock is taken when a
    /// call site first names a table, not per event.
    events: RwLock<HashMap<String, Arc<AtomicBool>>>,
    /// Every event shape made so far, by id. A heartbeat round sends them
    /// again, one message per record, after the watcher marks them due.
    pub(crate) shapes: Mutex<HashMap<u32, Arc<event::Shape>>>,
    shapes_due: AtomicBool,
    /// The heartbeat round still to send: `Source`, then each shape, then
    /// each trace definition. Empty when nothing is due.
    heartbeat_queue: Mutex<VecDeque<Beat>>,
    /// [`Shared::heartbeat_queue`]'s length, so an idle record is one load.
    heartbeat_left: AtomicUsize,
    /// Each trace's switch by name, set from `otel_traces` by the watcher.
    traces: RwLock<HashMap<String, Arc<trace::TraceSwitch>>>,
    /// Every trace definition made, re-sent with the shapes.
    pub(crate) trace_defs: Mutex<Vec<Arc<trace::DefMessage>>>,
    /// `otel_traces` is on for this app: `tracing` spans are recorded, and a
    /// new tracer starts on.
    spans_on: AtomicBool,
    /// `otel_traces`' sampling by trace name, as last applied: what a new
    /// tracer starts with.
    trace_rules: RwLock<BTreeMap<String, TraceConfig>>,
    pub(crate) span_defs: spans::SpanDefs,
}

impl Persist {
    /// Read the schema and `tables.yaml`, and publish on `settings.channel`
    /// through `bus`. [`Persist::poll`] applies later edits.
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
        let shared = Arc::new(Shared {
            enabled: (0..slots.unwrap_or(0))
                .map(|_| AtomicBool::new(false))
                .collect(),
            events: RwLock::new(HashMap::new()),
            shapes: Mutex::new(HashMap::new()),
            // The first record sends the `Source` message first.
            shapes_due: AtomicBool::new(true),
            heartbeat_queue: Mutex::new(VecDeque::new()),
            heartbeat_left: AtomicUsize::new(0),
            traces: RwLock::new(HashMap::new()),
            trace_defs: Mutex::new(Vec::new()),
            spans_on: AtomicBool::new(false),
            trace_rules: RwLock::new(BTreeMap::new()),
            span_defs: spans::SpanDefs::default(),
        });
        let metrics = metrics::Metrics::new(settings.metrics_interval);
        let reasons: [(&str, Read); 4] = [
            ("not_connected", |d| d.not_connected),
            ("back_pressure", |d| d.back_pressure),
            ("too_large", |d| d.too_large),
            ("other", |d| d.other),
        ];
        for (reason, read) in reasons {
            let bus = bus.clone();
            metrics.counter_fn("persist_dropped", &[("reason", reason)], move || {
                read(bus.drops())
            });
        }
        metrics.start();
        let (publication, max_payload) = if settings.exclusive {
            let p = bus.add_exclusive_publication(&settings.channel, settings.stream_id)?;
            let max = p
                .max_payload_length()
                .map_err(|e| Error::Aeron(e.to_string()))?;
            (
                Publication::Exclusive(crate::owned::OwnedPublication::new(p)),
                max,
            )
        } else {
            let p = bus.add_publication(&settings.channel, settings.stream_id)?;
            let max = p
                .max_payload_length()
                .map_err(|e| Error::Aeron(e.to_string()))?;
            (Publication::Shared(p), max)
        };
        let handoff = settings.exclusive.then(|| {
            let (send, receive) = std::sync::mpsc::sync_channel(HANDOFF);
            Handoff {
                send,
                receive: Mutex::new(receive),
            }
        });
        wait_for_subscriber(bus, &publication, &settings)?;
        let mut watcher = Watcher {
            ticks: 0,
            path: settings.config_path.clone(),
            app: settings.app,
            text: String::new(),
            config: BTreeMap::new(),
            applied: false,
            seen: Drops::default(),
            schema,
            shared: Arc::clone(&shared),
            bus: bus.clone(),
        };
        watcher.reload()?;
        watcher.apply(jiff::Timestamp::now());
        Ok(Self {
            inner: Arc::new(Inner {
                id: {
                    static NEXT: AtomicU64 = AtomicU64::new(0);
                    NEXT.fetch_add(1, Ordering::Relaxed)
                },
                publication,
                handoff,
                max_payload,
                metrics,
                bus: bus.clone(),
                shared,
                watch_due: AtomicI64::new(Clock::new().now().0 + Watcher::EVERY_NS),
                watcher: Mutex::new(watcher),
            }),
        })
    }

    /// Publish the metrics that are due and, once a second, apply
    /// `tables.yaml` and queue the dictionary heartbeat. Call it from the
    /// application's loop with its clock's time: nothing else does. Until
    /// something is due, two relaxed loads and compares.
    #[inline]
    pub fn poll(&self, now: Nanos) {
        self.drain_handoff();
        self.inner.metrics.poll_through(now, self);
        if now.0 >= self.inner.watch_due.load(Ordering::Relaxed) {
            self.watch(now);
        }
    }

    #[cold]
    fn watch(&self, now: Nanos) {
        // Another thread is polling: it applies.
        let Ok(mut watcher) = self.inner.watcher.try_lock() else {
            return;
        };
        if now.0 < self.inner.watch_due.load(Ordering::Relaxed) {
            return;
        }
        self.inner
            .watch_due
            .store(now.0 + Watcher::EVERY_NS, Ordering::Relaxed);
        watcher.tick();
    }

    /// A checkpoint trace (see [`trace`]): make it once, then
    /// [`trace::Tracer::start`] one per event. At most
    /// [`trace::MAX_STAGES`] stages and [`trace::MAX_ATTRS`] attributes.
    #[must_use]
    pub fn tracer(&self, name: &str, stages: &[&str], attrs: &[&str]) -> trace::Tracer {
        static NONCE: AtomicU64 = AtomicU64::new(0);
        let def = trace::TraceDef::new(name, stages, attrs);
        let bytes = def.message().unwrap_or_else(|e| {
            log::error!("trace {name}: {e}");
            Vec::new()
        });
        let shared = &self.inner.shared;
        // One definition however many threads make this tracer.
        let message = shared
            .trace_defs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|d| d.message == bytes)
            .map(Arc::clone);
        let message = message.unwrap_or_else(|| {
            let made = Arc::new(trace::DefMessage {
                message: bytes,
                sent: AtomicBool::new(false),
            });
            let mut defs = shared
                .trace_defs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(known) = defs.iter().find(|d| d.message == made.message) {
                return Arc::clone(known);
            }
            defs.push(Arc::clone(&made));
            made
        });
        // A new switch starts as the watcher last decided. Made under the
        // lock the watcher sets switches under, so it is never left behind.
        let switch = Arc::clone(
            shared
                .traces
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(name.to_owned())
                .or_insert_with(|| {
                    let rule = shared
                        .trace_rules
                        .read()
                        .unwrap_or_else(PoisonError::into_inner)
                        .get(name)
                        .copied();
                    let switch = trace::TraceSwitch::default();
                    switch.set(shared.spans_on.load(Ordering::Relaxed), rule.as_ref());
                    Arc::new(switch)
                }),
        );
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
        trace::Tracer::new(
            &def,
            message,
            switch,
            &self.inner.metrics,
            Some(self.clone()),
            self.inner.bus.source().id.rotate_left(17) ^ nonce,
        )
    }

    /// This application's metrics: make counters, gauges and histograms
    /// from it, and call [`Persist::poll`] from the loop. With
    /// [`Self::layer`] installed, `tracing` counter, gauge, and histogram
    /// events update the same registry. The handles are the path that does
    /// not allocate.
    #[must_use]
    pub fn metrics(&self) -> metrics::Metrics {
        self.inner.metrics.published_by(self.clone())
    }

    /// Make this the process's handle, for [`record`], [`enabled`],
    /// [`record_row`] and [`event_enabled`] to use from anywhere. It stays
    /// installed until the process exits. Only the first call installs; a
    /// later one returns `false` and changes nothing.
    pub fn install(&self) -> bool {
        INSTALLED.set(self.clone()).is_ok()
    }

    /// Is `template_id` recorded right now? One relaxed atomic load. Check it
    /// before preparing a message that is costly to size or encode.
    #[inline]
    #[must_use]
    pub fn enabled(&self, template_id: u16) -> bool {
        self.inner
            .shared
            .enabled
            .get(usize::from(template_id))
            .is_some_and(|e| e.load(Ordering::Relaxed))
    }

    /// Record one message of `template_id` that is exactly `len` bytes,
    /// header included (the generated `compute_length_with_header`). If its
    /// table is enabled, `encode` writes the message into a slot of exactly
    /// `len` bytes of the Aeron term buffer and returns the length it wrote;
    /// otherwise `encode` is never called.
    ///
    /// The record is dropped and counted in [`Bus::drops`] when Aeron
    /// cannot take it (no subscriber yet, back pressure, larger than one
    /// claim), when a term rotation does not finish within eight retries,
    /// when `encode` wrote another length or
    /// template than claimed (debug builds panic on that). An error from
    /// `encode` is returned as is, and nothing is published.
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
        if !self.on_owner() {
            debug_assert!(
                false,
                "an exclusive persist recorded template {template_id} off its owner thread"
            );
            let mut copy = vec![0; len];
            let written = encode(&mut copy)?;
            if written == len {
                self.hand_off(copy);
            } else {
                self.drop_one();
            }
            return Ok(());
        }
        self.flush_due_shapes();
        if len > self.inner.max_payload {
            self.count(DropKind::TooLarge);
            return Ok(());
        }
        let claim = match self.try_claim_slot(len) {
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

    /// A `tracing` layer that records events naming an enabled table
    /// (`tracing::info!(table = "signal", instrument = %id, edge = 0.25)`),
    /// counter, gauge, and histogram events
    /// (`tracing::info!(counter = "orders_sent", venue = "binance")`),
    /// and, while `otel_traces` is on for this app, spans at INFO and above
    /// (below INFO: libraries' internals).
    /// Add it to the application's subscriber. Its filter is its own: it
    /// takes only those, and leaves every other callsite to the other
    /// layers (or disabled, when it is alone).
    #[must_use]
    pub fn layer<S>(&self) -> impl Layer<S> + Send + Sync + 'static
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let shared = Arc::clone(&self.inner.shared);
        event::PersistLayer {
            persist: self.clone(),
        }
        .with_filter(
            dynamic_filter_fn(move |meta, _| {
                if meta.is_span() {
                    shared.spans_on.load(Ordering::Relaxed)
                } else {
                    event::layer_wants(meta)
                }
            })
            // A span's answer changes with `tables.yaml`, so it is asked
            // each time (one relaxed load); an event's never does.
            .with_callsite_filter(move |meta| {
                // Spans at INFO and above: the application's own. Libraries'
                // internals (h2, hyper, tokio) are DEBUG and TRACE spans.
                if meta.is_span() && *meta.level() <= tracing::Level::INFO {
                    tracing::subscriber::Interest::sometimes()
                } else if event::layer_wants(meta) {
                    tracing::subscriber::Interest::always()
                } else {
                    tracing::subscriber::Interest::never()
                }
            }),
        )
    }

    /// Is the event table `table` recorded right now? Check it before
    /// building a row that is costly to make.
    #[must_use]
    pub fn event_enabled(&self, table: &str) -> bool {
        let events = self
            .inner
            .shared
            .events
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        events
            .get(table)
            .is_some_and(|on| on.load(Ordering::Relaxed))
    }

    /// The switch of event table `table`, made (off) if `tables.yaml` does
    /// not list it; the config watcher flips it.
    pub(crate) fn event_switch(&self, table: &str) -> Arc<AtomicBool> {
        let events = &self.inner.shared.events;
        if let Some(on) = events
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(table)
        {
            return Arc::clone(on);
        }
        let mut events = events.write().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(events.entry(table.to_owned()).or_default())
    }

    /// Publish a message that is already built. `false` when it was not
    /// published; the drop is already counted.
    pub(crate) fn publish(&self, bytes: &[u8]) -> bool {
        self.flush_due_shapes();
        self.publish_owned(bytes)
            .map_err(|kind| self.count(kind))
            .is_ok()
    }

    /// Claim exactly `len` bytes, let `write` fill them, and commit when
    /// `write` returns `true`. A `false` write aborts the claim. Either
    /// failure is counted.
    pub(crate) fn claim(&self, len: usize, write: impl FnOnce(&mut [u8]) -> bool) {
        if !self.on_owner() {
            let mut copy = vec![0; len];
            if write(&mut copy) {
                self.hand_off(copy);
            } else {
                self.drop_one();
            }
            return;
        }
        self.flush_due_shapes();
        if len > self.inner.max_payload {
            self.count(DropKind::TooLarge);
            return;
        }
        let claim = match self.try_claim_slot(len) {
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

    /// One dictionary message, when a heartbeat round is due or unfinished.
    /// An idle call is two relaxed loads. A new shape is still published
    /// ahead of its first row by `send_shape`; this only repeats what was
    /// already sent.
    #[inline]
    fn flush_due_shapes(&self) {
        let shared = &self.inner.shared;
        if shared.heartbeat_left.load(Ordering::Relaxed) == 0
            && !shared.shapes_due.load(Ordering::Relaxed)
        {
            return;
        }
        self.send_one_heartbeat();
    }

    /// Publish the next heartbeat message. These repeat, so one Aeron cannot
    /// take is not a dropped record: an unsent `Source` stays at the front
    /// of the queue, and a shape is sent by its next row or the next round.
    fn send_one_heartbeat(&self) {
        let shared = &self.inner.shared;
        let mut queue = shared
            .heartbeat_queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if queue.is_empty() {
            if !shared.shapes_due.swap(false, Ordering::Relaxed) {
                return;
            }
            queue.extend(self.heartbeat_round());
        }
        let Some(beat) = queue.pop_front() else {
            shared.heartbeat_left.store(0, Ordering::Relaxed);
            return;
        };
        // Held across the publish so two threads cannot reorder the round.
        let sent = self.publish_owned(&beat.bytes).is_ok();
        if sent {
            shared.heartbeat_left.store(queue.len(), Ordering::Relaxed);
            if let Some(shape) = &beat.shape {
                shape.mark_sent();
            }
            if let Some(def) = &beat.def {
                def.sent.store(true, Ordering::Relaxed);
            }
            return;
        }
        // The `Source` message has to stay ahead of the rows that follow.
        // A shape or a trace definition can wait for its own next send.
        if beat.shape.is_none() && beat.def.is_none() {
            queue.push_front(beat);
        }
        shared.heartbeat_left.store(queue.len(), Ordering::Relaxed);
    }

    /// `Source`, then every shape, then every trace definition.
    fn heartbeat_round(&self) -> VecDeque<Beat> {
        let mut round = VecDeque::new();
        round.push_back(Beat {
            bytes: self.inner.bus.source_message().to_vec(),
            shape: None,
            def: None,
        });
        let shapes: Vec<_> = self
            .inner
            .shared
            .shapes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect();
        for shape in shapes {
            round.push_back(Beat {
                bytes: shape.message().to_vec(),
                shape: Some(shape),
                def: None,
            });
        }
        let defs = self
            .inner
            .shared
            .trace_defs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for def in defs {
            round.push_back(Beat {
                bytes: def.message.clone(),
                shape: None,
                def: Some(def),
            });
        }
        round
    }

    /// Publish a built message; why not, when Aeron could not take it.
    fn publish_owned(&self, bytes: &[u8]) -> Result<(), DropKind> {
        if !self.on_owner() {
            self.hand_off(bytes.to_vec());
            return Ok(());
        }
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
        match &self.inner.publication {
            Publication::Shared(p) => self.inner.bus.try_claim(p, len),
            Publication::Exclusive(owned) => owned.claimable().map_or(Err(DropKind::Other), |p| {
                self.inner.bus.try_claim_exclusive(p, len)
            }),
        }
    }

    /// Shared mode, or the owner thread of an exclusive one.
    #[inline]
    fn on_owner(&self) -> bool {
        match &self.inner.publication {
            Publication::Shared(_) => true,
            Publication::Exclusive(owned) => owned.is_owner(),
        }
    }

    /// Queue a record made off the owner thread; dropped and counted when
    /// the owner has fallen this far behind.
    #[cold]
    fn hand_off(&self, bytes: Vec<u8>) {
        let Some(handoff) = &self.inner.handoff else {
            self.drop_one();
            return;
        };
        if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) =
            handoff.send.try_send(bytes)
        {
            self.count(DropKind::BackPressure);
        }
    }

    /// Publish what other threads handed off, on the owner thread.
    fn drain_handoff(&self) {
        let Some(handoff) = &self.inner.handoff else {
            return;
        };
        if !self.on_owner() {
            return;
        }
        let Ok(receive) = handoff.receive.try_lock() else {
            return;
        };
        while let Ok(bytes) = receive.try_recv() {
            if let Err(kind) = self.publish_owned(&bytes) {
                self.count(kind);
            }
        }
    }

    pub(crate) fn count(&self, kind: DropKind) {
        self.inner.bus.count(kind);
    }

    /// The longest message one claim holds.
    pub(crate) fn max_payload(&self) -> usize {
        self.inner.max_payload
    }

    /// Is the archive recording this stream yet? Until it is, every record
    /// is dropped and counted.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        match &self.inner.publication {
            Publication::Shared(p) => p.is_connected(),
            Publication::Exclusive(owned) => owned.is_connected(),
        }
    }

    #[cold]
    pub(crate) fn drop_one(&self) {
        self.inner.bus.drop_one();
    }

    /// The bus this records through: its [`Bus::drops`] count this
    /// handle's dropped records, and [`Bus::shutdown`] closes it.
    #[must_use]
    pub fn bus(&self) -> &Bus {
        &self.inner.bus
    }
}

impl std::fmt::Debug for Persist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Persist")
            .field("bus", &self.inner.bus)
            .finish_non_exhaustive()
    }
}

fn wait_for_subscriber(
    bus: &Bus,
    publication: &Publication,
    settings: &Settings,
) -> Result<(), Error> {
    let connected = || match publication {
        Publication::Shared(p) => p.is_connected(),
        Publication::Exclusive(owned) => owned.is_connected(),
    };
    if settings.subscriber_timeout.is_zero() {
        return Ok(());
    }
    let deadline = Instant::now() + settings.subscriber_timeout;
    while !connected() {
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

/// Applies `tables.yaml` to [`Shared::enabled`], from [`Persist::poll`].
struct Watcher {
    ticks: u64,
    path: PathBuf,
    /// This application's name, for the tables' `apps` switches.
    app: String,
    /// The last `tables.yaml` read, good or bad, and the last good one's
    /// tables.
    text: String,
    config: BTreeMap<String, TableConfig>,
    /// The switches have been applied once: after that, only changes are logged.
    applied: bool,
    seen: Drops,
    schema: Vec<(String, u16)>,
    shared: Arc<Shared>,
    bus: Bus,
}

impl Watcher {
    const EVERY_NS: i64 = 1_000_000_000;

    fn tick(&mut self) {
        // Every 5 s, ask the next record to send every event shape again.
        // An ingester that starts after the first one, with none saved,
        // learns them from that record.
        self.ticks += 1;
        if self.ticks.is_multiple_of(5) {
            self.shared.shapes_due.store(true, Ordering::Relaxed);
            self.bus.beat();
        }
        if let Err(e) = self.reload() {
            log::error!("{e}; keeping the previous configuration");
        }
        // Every second, changed or not: an `until` passes by itself.
        self.apply(jiff::Timestamp::now());
        let drops = self.bus.drops();
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

    /// Read `tables.yaml` again if it changed since the last call. A bad
    /// version is reported once.
    fn reload(&mut self) -> Result<(), Error> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| Error::Config(format!("{}: {e}", self.path.display())))?;
        if text != self.text {
            let parsed = parse_config(&text);
            self.text = text;
            self.config = parsed?;
        }
        Ok(())
    }

    /// Switch every table on or off for this app at `now`.
    fn apply(&mut self, now: jiff::Timestamp) {
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
            toggled(
                name,
                self.shared.enabled[usize::from(*id)].swap(on, Ordering::Relaxed),
                on,
            );
        }
        // Traces: `otel_traces` switches them all, and names their sampling.
        let traces = config.get(OTEL_TRACES);
        let traces_on = is_on(OTEL_TRACES);
        // First what a new tracer starts with, then every existing one.
        self.shared.spans_on.store(traces_on, Ordering::Relaxed);
        *self
            .shared
            .trace_rules
            .write()
            .unwrap_or_else(PoisonError::into_inner) =
            traces.map(|c| c.traces.clone()).unwrap_or_default();
        for (name, switch) in self
            .shared
            .traces
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            switch.set(traces_on, traces.and_then(|c| c.traces.get(name)));
        }
        // Every other table is recorded from `tracing` events. Log after the
        // map lock so a slow log does not hold it.
        let notes = {
            let mut switches = self
                .shared
                .events
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            let mut notes = Vec::new();
            for (name, switch) in switches.iter() {
                if !config.contains_key(name) && switch.swap(false, Ordering::Relaxed) {
                    notes.push((name.clone(), true, false));
                }
            }
            for name in config
                .keys()
                .filter(|name| !self.schema.iter().any(|(n, _)| n == *name))
            {
                let on = is_on(name);
                let was = switches
                    .entry(name.clone())
                    .or_default()
                    .swap(on, Ordering::Relaxed);
                notes.push((name.clone(), was, on));
            }
            drop(switches);
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

    /// Nothing in this test binary installs a handle.
    #[test]
    fn without_an_installed_handle_recording_does_nothing() -> TestResult {
        assert!(installed().is_none());
        record(1, 64, |_| Err("encoded with nothing installed"))?;
        assert!(!enabled(1) && !event_enabled("spread"));
        record_row("spread", [("bps", event::Value::F64(1.0))]);
        Ok(())
    }

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
