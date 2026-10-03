//! Metrics: counters, gauges and histograms, declared once with labels and
//! updated through handles or `tracing` events.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ergon_runtime::clock::Clock;
//! # let settings = ergon_runtime::Settings::from_env();
//! # let bus = ergon_runtime::bus::Bus::connect(&settings)?;
//! # let persist = ergon_runtime::persist::Persist::connect("", &bus, settings)?;
//! let metrics = persist.metrics();
//! let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
//! let depth = metrics.gauge("book_depth", &[("side", "bid")]);
//! let t2t = metrics.histogram("tick_to_trade_ns", &[]);
//! let clock = Clock::new();
//! loop {
//!     let now = clock.now();
//!     sent.inc(); // a load and a store
//!     depth.set(12.0); // a store
//!     t2t.record(850); // count, sum, min, and max
//!     persist.poll(now); // two compares, until the next millisecond, the 5 s boundary or a config re-read
//! #   break;
//! }
//! # Ok(()) }
//! ```
//!
//! * A **counter** or **histogram** handle has one writer at a time: it is
//!   `Send` but not `Sync`. Counter increments use a relaxed load and store.
//!   Histogram updates lock their cell so polling from another thread takes
//!   a complete count/sum/min/max summary. Asking for the same series again
//!   gives another cell; [`Metrics::poll`] adds them up.
//! * A **gauge** is `Sync` and `Clone`: every handle of a series shares one
//!   cell, and the last value set wins.
//! * Off the hot path the same series can be updated from a `tracing` event,
//!   once [`crate::persist::Persist::layer`] is installed:
//!   `tracing::info!(counter = "orders_sent", venue = "binance")`,
//!   `tracing::info!(gauge = "book_depth", value = 12.0)`,
//!   `tracing::info!(histogram = "tick_to_trade_ns", value = 850)`.
//!   That path allocates the label strings and shares one cell across threads.
//!   [`Metrics::poll`] still publishes it. A handle and a tracing event for
//!   the same counter or histogram are two cells, added together.
//! * A **histogram** handle records only the count, the sum, the minimum, and
//!   the maximum. Every millisecond that had samples, [`Metrics::poll`]
//!   publishes that summary. The ingester folds those summaries into a 5 s
//!   `HdrHistogram`. [`PRECISION`] is the public log-linear layout; handles do
//!   not update it, and the ingester does not use it.
//!
//! Counters and gauges publish every interval (5 s by default, aligned to
//! multiples of it in UNIX time, so applications line up): each series' name
//! and labels (`MetricDef`, keyed by a hash of them), then the values
//! (`Metrics`). A histogram summary uses the same `MetricDef` and its own
//! 1 ms deadline. A call publishes at most one message, and the next call
//! continues, so no call costs more than one publish. Call it from the
//! thread's loop with the time it already has; any thread may call it.

use std::cell::Cell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex, PoisonError};

use crate::clock::Nanos;
use crate::event::codec;
use crate::persist::Persist;

/// Template id of the `MetricDef` message.
pub const METRIC_DEF_TEMPLATE_ID: u16 = codec::MetricDefEncoder::TEMPLATE_ID;
/// Template id of the `Metrics` message.
pub const METRICS_TEMPLATE_ID: u16 = codec::MetricsEncoder::TEMPLATE_ID;
/// Template id of the `Histogram` message.
pub const HISTOGRAM_TEMPLATE_ID: u16 = codec::HistogramEncoder::TEMPLATE_ID;

/// Sub-bucket bits of the log-linear layout [`bucket`] describes.
///
/// A value is within `2^-5` (3.1%) of its bucket's bounds. Handles do not
/// update buckets. The ingester estimates a 5 s `HdrHistogram` from the 1 ms
/// summaries and does not use this layout.
pub const PRECISION: u8 = 5;

/// Nanoseconds in the histogram summary a [`Metrics::poll`] publishes.
const HISTOGRAM_MS: i64 = 1_000_000;

/// Buckets of a histogram with `precision` sub-bucket bits, over all `u64`.
#[must_use]
pub const fn bucket_count(precision: u8) -> usize {
    (65 - precision as usize) << precision
}

/// The bucket of `value`: the power of two it is in, and its linear
/// sub-bucket inside that. Values below `2^precision` have one each.
#[inline]
#[must_use]
pub fn bucket(value: u64, precision: u8) -> usize {
    let p = u32::from(precision);
    let magnitude = (value | (1 << p)).ilog2();
    let shift = magnitude - p;
    ((shift as usize) << p) + usize::try_from(value >> shift).unwrap_or(usize::MAX)
}

/// The lowest and highest value of bucket `index`.
#[must_use]
pub fn bucket_bounds(index: usize, precision: u8) -> (u64, u64) {
    let p = usize::from(precision);
    let shift = (index >> p).saturating_sub(1);
    let low = ((index - (shift << p)) as u64) << shift;
    (low, low + ((1u64 << shift) - 1))
}

/// What a series is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricKind {
    /// A value that only increases.
    Counter,
    /// A value that is sampled as last-set.
    Gauge,
    /// A distribution of recorded values.
    Histogram,
}

impl MetricKind {
    const fn wire(self) -> codec::MetricKind {
        match self {
            Self::Counter => codec::MetricKind::Counter,
            Self::Gauge => codec::MetricKind::Gauge,
            Self::Histogram => codec::MetricKind::Histogram,
        }
    }

    /// Its name in `ClickHouse`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
        }
    }
}

/// A series: its name, kind and labels, keyed by a hash of all three.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricDef {
    /// Hash of the name, kind, and labels.
    pub series: u64,
    /// Counter, gauge, or histogram.
    pub kind: MetricKind,
    /// Carried on `MetricDef`. The ingester does not bucket with it.
    pub precision: u8,
    /// The metric's name.
    pub name: String,
    /// Sorted by key.
    pub labels: Vec<(String, String)>,
}

impl MetricDef {
    /// The series `name` of `kind` with `labels`, in any order.
    #[must_use]
    pub fn new(name: &str, kind: MetricKind, labels: &[(&str, &str)]) -> Self {
        let mut labels: Vec<(String, String)> = labels
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        labels.sort();
        let mut key = Vec::new();
        for part in std::iter::once(name)
            .chain(std::iter::once(kind.name()))
            .chain(labels.iter().flat_map(|(k, v)| [k.as_str(), v.as_str()]))
        {
            key.extend_from_slice(part.as_bytes());
            key.push(0);
        }
        Self {
            series: crate::event::fnv64(&key),
            kind,
            precision: PRECISION,
            name: name.to_owned(),
            labels,
        }
    }

    /// This series as a `MetricDef` message, header included.
    ///
    /// # Errors
    ///
    /// More labels than a group header can count, or a name the codec rejects.
    pub fn message(&self) -> Result<Vec<u8>, crate::event::EncodeError> {
        let count = crate::event::group_count(self.labels.len())?;
        let len = codec::MetricDefEncodedLength::new()
            .labels_ragged(count, |g| {
                for (k, v) in &self.labels {
                    g.add()?.key(k.len())?.value(v.len())?;
                }
                Ok(())
            })?
            .name(self.name.len())?
            .encoded_length_with_header();
        crate::event::owned_frame(len, |message| {
            Ok(codec::MetricDefEncoder::wrap_and_apply_header(message, 0)
                .fixed(&codec::MetricDefFixedFields {
                    series: self.series,
                    kind: self.kind.wire(),
                    precision: self.precision,
                })
                .labels(count, |g| {
                    for (k, v) in &self.labels {
                        g.add(|e| e.key(k.as_bytes())?.value(v.as_bytes()))?;
                    }
                    Ok(())
                })?
                .name(self.name.as_bytes())?
                .encoded_length_with_header())
        })
    }

    /// A series from its `MetricDef` message; `None` when malformed.
    #[must_use]
    pub fn decode(message: &[u8]) -> Option<Self> {
        let d = codec::MetricDefDecoder::decode(message, 0).ok()?;
        let kind = match d.kind() {
            codec::MetricKind::Counter => MetricKind::Counter,
            codec::MetricKind::Gauge => MetricKind::Gauge,
            codec::MetricKind::Histogram => MetricKind::Histogram,
            codec::MetricKind::NullVal => return None,
        };
        let labels = d
            .labels()
            .ok()?
            .map(|e| {
                let e = e.ok()?;
                Some((
                    e.key_as_str().ok()?.to_owned(),
                    e.value_as_str().ok()?.to_owned(),
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            series: d.series(),
            kind,
            precision: d.precision(),
            name: d.name_as_str().ok()?.to_owned(),
            labels,
        })
    }
}

/// Sharing a cache line with another writer's cell would make every write
/// wait for the other core: each cell has lines of its own.
#[repr(align(128))]
#[derive(Default)]
struct CounterCell {
    value: AtomicU64,
}

#[repr(align(128))]
#[derive(Default)]
struct GaugeCell {
    bits: AtomicU64,
}

/// One millisecond's count, sum, minimum, and maximum. Copied out of a cell.
#[derive(Clone, Copy)]
struct HistSample {
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
}

impl Default for HistSample {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0,
            min: u64::MAX,
            max: 0,
        }
    }
}

#[repr(align(128))]
struct HistogramCell {
    // Polling resets the summary, so it is a second writer even when the
    // handle itself has one producer. All four values must move together.
    sample: Mutex<HistSample>,
}

impl HistogramCell {
    fn new() -> Self {
        Self {
            sample: Mutex::new(HistSample::default()),
        }
    }

    fn sample(&self) -> std::sync::MutexGuard<'_, HistSample> {
        self.sample.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn record(&self, value: u64) {
        let mut sample = self.sample();
        sample.count = sample.count.wrapping_add(1);
        sample.sum = sample.sum.wrapping_add(value);
        sample.min = sample.min.min(value);
        sample.max = sample.max.max(value);
    }
}

/// A histogram cell written and polled by one thread: four relaxed loads and
/// stores per record, no lock and no read-modify-write. Polled from another
/// thread it could split one sample across two summaries; the handle is
/// `!Send`, and the runtime polls on the recording thread.
#[repr(align(128))]
#[derive(Default)]
struct LocalHistCell {
    count: AtomicU64,
    sum: AtomicU64,
    min: AtomicU64,
    max: AtomicU64,
}

impl LocalHistCell {
    fn new() -> Self {
        let cell = Self::default();
        cell.min.store(u64::MAX, Relaxed);
        cell
    }

    #[inline]
    fn record(&self, value: u64) {
        self.count
            .store(self.count.load(Relaxed).wrapping_add(1), Relaxed);
        self.sum
            .store(self.sum.load(Relaxed).wrapping_add(value), Relaxed);
        if value < self.min.load(Relaxed) {
            self.min.store(value, Relaxed);
        }
        if value > self.max.load(Relaxed) {
            self.max.store(value, Relaxed);
        }
    }

    fn take(&self) -> HistSample {
        let sample = HistSample {
            count: self.count.load(Relaxed),
            sum: self.sum.load(Relaxed),
            min: self.min.load(Relaxed),
            max: self.max.load(Relaxed),
        };
        self.count.store(0, Relaxed);
        self.sum.store(0, Relaxed);
        self.min.store(u64::MAX, Relaxed);
        self.max.store(0, Relaxed);
        sample
    }
}

/// A histogram for one thread that also polls it, as a runtime agent's
/// thread does: [`Histogram`] without the lock. `!Send`.
pub struct LocalHistogram {
    cell: Arc<LocalHistCell>,
    _one_thread: PhantomData<*const ()>,
}

impl LocalHistogram {
    /// Record one value: plain loads and stores.
    #[inline]
    pub fn record(&self, value: u64) {
        self.cell.record(value);
    }
}

impl std::fmt::Debug for LocalHistogram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalHistogram").finish_non_exhaustive()
    }
}

/// Written by one thread at a time: `Send`, not `Sync`.
type OneWriter = PhantomData<Cell<()>>;

/// A count that only goes up. One writer at a time; see the module docs.
pub struct Counter {
    cell: Arc<CounterCell>,
    _one_writer: OneWriter,
}

impl Counter {
    /// Add one.
    #[inline]
    pub fn inc(&self) {
        self.add(1);
    }

    /// Add `n`: a relaxed load and store, no locked instruction.
    #[inline]
    pub fn add(&self, n: u64) {
        let v = &self.cell.value;
        v.store(v.load(Relaxed).wrapping_add(n), Relaxed);
    }

    /// The total this handle has added.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.cell.value.load(Relaxed)
    }
}

/// A value that goes up and down; the last one set is sampled.
#[derive(Clone)]
pub struct Gauge {
    cell: Arc<GaugeCell>,
}

impl Gauge {
    /// A relaxed store.
    #[inline]
    pub fn set(&self, value: f64) {
        self.cell.bits.store(value.to_bits(), Relaxed);
    }

    /// The last value stored.
    #[must_use]
    pub fn get(&self) -> f64 {
        f64::from_bits(self.cell.bits.load(Relaxed))
    }
}

/// A distribution, such as a latency in nanoseconds. One writer at a time.
pub struct Histogram {
    cell: Arc<HistogramCell>,
    _one_writer: OneWriter,
}

impl Histogram {
    /// Values recorded into this cell since it was made, until the next poll
    /// swaps the count back to zero.
    #[cfg(test)]
    pub(crate) fn cell_count(&self) -> u64 {
        self.cell.sample().count
    }

    /// Record one value: the count, the sum, the minimum, and the maximum.
    /// Locks this cell so a concurrent poll cannot split or repeat a sample.
    #[inline]
    pub fn record(&self, value: u64) {
        self.cell.record(value);
    }
}

impl std::fmt::Debug for Counter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Counter").field(&self.get()).finish()
    }
}

impl std::fmt::Debug for Gauge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Gauge").field(&self.get()).finish()
    }
}

impl std::fmt::Debug for Histogram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Histogram").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Metrics").finish_non_exhaustive()
    }
}

/// Where a counter's or gauge's value comes from.
enum Source {
    Counter(Arc<CounterCell>),
    Gauge(Arc<GaugeCell>),
    Histogram(Arc<HistogramCell>),
    LocalHistogram(Arc<LocalHistCell>),
    /// Read at each poll: an atomic kept elsewhere.
    Sampled(Box<dyn Fn() -> u64 + Send>),
}

/// One cell of a series, and what the last poll saw of it.
struct Cellref {
    def: usize,
    source: Source,
}

/// A series and what the last poll published of it.
struct Def {
    metric: MetricDef,
    message: Vec<u8>,
    /// A counter's total at the last poll.
    last_total: u64,
    /// A gauge's one cell.
    gauge: Option<Arc<GaugeCell>>,
    /// Its `MetricDef` has been published ahead of a histogram summary.
    /// The 5 s cycle publishes every def again either way.
    announced: bool,
    /// The cell [`Metrics::add_counter`] reuses. A [`Counter`] handle stays
    /// its own cell, so a tracing event does not race that handle's add.
    shared_counter: Option<Arc<CounterCell>>,
    /// The cell [`Metrics::record_histogram`] reuses, for the same reason.
    shared_histogram: Option<Arc<HistogramCell>>,
}

/// The messages one interval still has to publish, in order.
#[derive(Default)]
struct Cycle {
    /// UNIX ns of the counter/gauge interval's end.
    ts: u64,
    next_def: usize,
    counters: Vec<(u64, u64, u64)>,
    gauges: Vec<(u64, f64)>,
    next_counter: usize,
    next_gauge: usize,
    /// UNIX ns of the histogram summary's end, and how long it covers.
    hist_ts: u64,
    hist_interval: u64,
    samples: Vec<(u64, HistSample)>,
    next_sample: usize,
    /// Histogram series whose `MetricDef` has not been sent yet.
    hist_defs: Vec<usize>,
    next_hist_def: usize,
}

/// One message to publish. [`State::next`] has already moved past it, so
/// a message Aeron cannot take is dropped rather than retried by every call.
enum Next {
    Def(usize),
    Metrics {
        counters: std::ops::Range<usize>,
        gauges: std::ops::Range<usize>,
    },
    Histogram {
        start: usize,
        len: usize,
    },
}

struct State {
    defs: Vec<Def>,
    by_series: HashMap<u64, usize>,
    cells: Vec<Cellref>,
    /// The interval being published, while `active`. Counter/gauge buffers
    /// and histogram buffers are reused across intervals.
    cycle: Cycle,
    /// A counter/gauge interval is being published.
    active: bool,
    /// A histogram summary is being published.
    hist_open: bool,
    /// `Nanos` of the next histogram millisecond. `i64::MAX` until a
    /// histogram series exists.
    next_hist: i64,
    /// `Nanos` of the next counter/gauge interval. `i64::MAX` until
    /// [`Metrics::start`].
    next_metrics: i64,
    /// UNIX ns of the last histogram millisecond closed.
    last_hist_end: u64,
    /// A counter series' total over its cells, by series index; reused.
    totals: Vec<Option<u64>>,
    /// One-based summary slot for each definition; zero represents no sample.
    /// `Option<NonZeroUsize>` keeps each reusable slot to one machine word.
    histogram_slots: Vec<Option<NonZeroUsize>>,
}

struct Registry {
    interval_ns: i64,
    sim_now: AtomicI64,
    /// `Nanos` of the next interval's end; `i64::MIN` while one is being
    /// published, `i64::MAX` when nothing publishes.
    next_due: AtomicI64,
    state: Mutex<State>,
}

/// The series of one application, and the publishing of them. Cheap to
/// clone; every clone is the same registry.
#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    persist: Option<Persist>,
}

impl Metrics {
    pub(crate) fn new(interval: std::time::Duration) -> Self {
        let interval_ns = i64::try_from(interval.as_nanos())
            .unwrap_or(i64::MAX)
            .max(1);
        Self {
            registry: Arc::new(Registry {
                interval_ns,
                sim_now: AtomicI64::new(i64::MIN),
                next_due: AtomicI64::new(i64::MAX),
                state: Mutex::new(State {
                    defs: Vec::new(),
                    by_series: HashMap::new(),
                    cells: Vec::new(),
                    cycle: Cycle::default(),
                    active: false,
                    hist_open: false,
                    next_hist: i64::MAX,
                    next_metrics: i64::MAX,
                    last_hist_end: 0,
                    totals: Vec::new(),
                    histogram_slots: Vec::new(),
                }),
            }),
            persist: None,
        }
    }

    /// Series that nothing publishes: handles work, and cost what they
    /// always do. What [`fn@crate::persist::metrics`] returns with no handle installed.
    #[must_use]
    pub fn detached() -> Self {
        Self::new(std::time::Duration::from_secs(5))
    }

    /// Publish from the end of the current interval on, once a handle made
    /// with [`Metrics::published_by`] polls.
    pub(crate) fn start(&self) {
        self.start_at(crate::clock::Clock::new().now());
    }

    pub(crate) fn set_simulation(&self, now: Nanos) {
        self.registry.sim_now.store(now.0, Relaxed);
        self.start_at(now);
    }

    pub(crate) fn sim_time(&self, now: Nanos) {
        self.registry.sim_now.store(now.0, Relaxed);
    }

    fn start_at(&self, now: Nanos) {
        let metrics_due = self.next_boundary(now).0;
        let mut state = self.state();
        state.next_metrics = metrics_due;
        if state.next_hist != i64::MAX {
            // Registered before publishing started: the first summary is the
            // next millisecond, not a deadline that passed while connecting.
            Self::arm_locked(&mut state, now);
        }
        let due = if state.next_hist == i64::MAX {
            metrics_due
        } else {
            state.next_hist.min(metrics_due)
        };
        drop(state);
        self.registry.next_due.store(due, Relaxed);
    }

    /// The first histogram series: publish its summaries on the millisecond.
    fn arm_histogram(&self) {
        let mut state = self.state();
        if state.next_hist != i64::MAX {
            return;
        }
        let sim = self.registry.sim_now.load(Relaxed);
        let now = if sim == i64::MIN {
            crate::clock::Clock::new().now()
        } else {
            Nanos(sim)
        };
        Self::arm_locked(&mut state, now);
        let due = self.registry.next_due.load(Relaxed);
        if due != i64::MIN {
            let next = if due == i64::MAX {
                state.next_hist
            } else {
                due.min(state.next_hist)
            };
            self.registry.next_due.store(next, Relaxed);
        }
    }

    fn arm_locked(state: &mut State, now: Nanos) {
        let end = now.epoch_ns().div_euclid(HISTOGRAM_MS) * HISTOGRAM_MS;
        state.last_hist_end = u64::try_from(end).unwrap_or(0);
        state.next_hist = Nanos::from_epoch(end + HISTOGRAM_MS).0;
    }

    fn store_due(&self, state: &State) {
        let due = if state.next_hist == i64::MAX {
            state.next_metrics
        } else if state.next_metrics == i64::MAX {
            state.next_hist
        } else {
            state.next_hist.min(state.next_metrics)
        };
        self.registry.next_due.store(due, Relaxed);
    }

    /// This registry, published through `persist` when polled.
    pub(crate) fn published_by(&self, persist: Persist) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            persist: Some(persist),
        }
    }

    /// A counter. Each call makes another cell for the series, which
    /// [`Metrics::poll`] adds to the others: give each thread its own.
    #[must_use]
    pub fn counter(&self, name: &str, labels: &[(&str, &str)]) -> Counter {
        let cell = Arc::new(CounterCell::default());
        self.register(name, MetricKind::Counter, labels, |_| {
            Source::Counter(Arc::clone(&cell))
        });
        Counter {
            cell,
            _one_writer: PhantomData,
        }
    }

    /// A gauge: every call for the series returns the same cell.
    #[must_use]
    pub fn gauge(&self, name: &str, labels: &[(&str, &str)]) -> Gauge {
        let cell = {
            let mut state = self.state();
            let def = Self::def(&mut state, name, MetricKind::Gauge, labels);
            let cell = state.defs[def]
                .gauge
                .get_or_insert_with(|| Arc::new(GaugeCell::default()))
                .clone();
            if !state
                .cells
                .iter()
                .any(|c| c.def == def && matches!(c.source, Source::Gauge(_)))
            {
                state.cells.push(Cellref {
                    def,
                    source: Source::Gauge(Arc::clone(&cell)),
                });
            }
            cell
        };
        Gauge { cell }
    }

    /// A histogram. Each call makes another cell for the series, which
    /// [`Metrics::poll`] merges with the others: give each thread its own.
    #[must_use]
    pub fn histogram(&self, name: &str, labels: &[(&str, &str)]) -> Histogram {
        let cell = Arc::new(HistogramCell::new());
        self.register(name, MetricKind::Histogram, labels, |_| {
            Source::Histogram(Arc::clone(&cell))
        });
        self.arm_histogram();
        Histogram {
            cell,
            _one_writer: PhantomData,
        }
    }

    /// A [`LocalHistogram`]: for a thread that both records and polls this
    /// registry, such as a runtime agent's.
    #[must_use]
    pub fn local_histogram(&self, name: &str, labels: &[(&str, &str)]) -> LocalHistogram {
        let cell = Arc::new(LocalHistCell::new());
        self.register(name, MetricKind::Histogram, labels, |_| {
            Source::LocalHistogram(Arc::clone(&cell))
        });
        self.arm_histogram();
        LocalHistogram {
            cell,
            _one_thread: PhantomData,
        }
    }

    /// Add `n` to counter `name`. Calls for one series share one cell and use
    /// `fetch_add`, so more than one thread may call this. A [`Counter`] from
    /// [`Self::counter`] is a different cell: [`Self::poll`] adds the two.
    /// Building `labels` allocates; the handle does not.
    pub fn add_counter(&self, name: &str, labels: &[(&str, &str)], n: u64) {
        self.shared_counter(name, labels)
            .value
            .fetch_add(n, Relaxed);
    }

    /// Set gauge `name` to `value`. Every call for the series is the same
    /// cell as [`Self::gauge`], and the last store wins. Building `labels`
    /// allocates.
    pub fn set_gauge(&self, name: &str, labels: &[(&str, &str)], value: f64) {
        self.gauge(name, labels).set(value);
    }

    /// Record `value` in histogram `name`. Calls for one series share one
    /// cell. A [`Histogram`] from [`Self::histogram`] is a different cell, and
    /// [`Self::poll`] merges them. Building `labels` allocates.
    pub fn record_histogram(&self, name: &str, labels: &[(&str, &str)], value: u64) {
        let cell = self.shared_histogram(name, labels);
        cell.record(value);
    }

    fn shared_counter(&self, name: &str, labels: &[(&str, &str)]) -> Arc<CounterCell> {
        let mut state = self.state();
        let index = Self::def(&mut state, name, MetricKind::Counter, labels);
        if let Some(cell) = &state.defs[index].shared_counter {
            return Arc::clone(cell);
        }
        let cell = Arc::new(CounterCell::default());
        state.defs[index].shared_counter = Some(Arc::clone(&cell));
        state.cells.push(Cellref {
            def: index,
            source: Source::Counter(cell.clone()),
        });
        cell
    }

    fn shared_histogram(&self, name: &str, labels: &[(&str, &str)]) -> Arc<HistogramCell> {
        let mut state = self.state();
        let index = Self::def(&mut state, name, MetricKind::Histogram, labels);
        if let Some(cell) = &state.defs[index].shared_histogram {
            return Arc::clone(cell);
        }
        let cell = Arc::new(HistogramCell::new());
        state.defs[index].shared_histogram = Some(Arc::clone(&cell));
        state.cells.push(Cellref {
            def: index,
            source: Source::Histogram(Arc::clone(&cell)),
        });
        drop(state);
        self.arm_histogram();
        cell
    }

    /// A counter whose total `read` returns at each poll, such as an atomic
    /// that something else increments.
    pub fn counter_fn(
        &self,
        name: &str,
        labels: &[(&str, &str)],
        read: impl Fn() -> u64 + Send + 'static,
    ) {
        self.register(name, MetricKind::Counter, labels, |_| {
            Source::Sampled(Box::new(read))
        });
    }

    fn register(
        &self,
        name: &str,
        kind: MetricKind,
        labels: &[(&str, &str)],
        source: impl FnOnce(usize) -> Source,
    ) {
        let mut state = self.state();
        let def = Self::def(&mut state, name, kind, labels);
        let source = source(def);
        state.cells.push(Cellref { def, source });
    }

    /// The series' index, made the first time.
    fn def(state: &mut State, name: &str, kind: MetricKind, labels: &[(&str, &str)]) -> usize {
        let def = MetricDef::new(name, kind, labels);
        if let Some(&i) = state.by_series.get(&def.series) {
            return i;
        }
        let message = def.message().unwrap_or_else(|e| {
            log::error!("metric {name}: {e}; its values are not published");
            Vec::new()
        });
        state.by_series.insert(def.series, state.defs.len());
        state.defs.push(Def {
            metric: def,
            message,
            last_total: 0,
            gauge: None,
            announced: false,
            shared_counter: None,
            shared_histogram: None,
        });
        state.defs.len() - 1
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.registry
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The end of the interval after `now`, in `Nanos`.
    fn next_boundary(&self, now: Nanos) -> Nanos {
        let i = self.registry.interval_ns;
        Nanos::from_epoch((now.epoch_ns().div_euclid(i) + 1) * i)
    }

    /// Publish the interval that ended, one message per call. Until the
    /// interval ends, one compare.
    #[inline]
    pub fn poll(&self, now: Nanos) {
        if now.0 >= self.registry.next_due.load(Relaxed) {
            self.poll_due(now);
        }
    }

    #[cold]
    fn poll_due(&self, now: Nanos) {
        if let Some(persist) = &self.persist {
            self.publish_due(now, persist);
        }
    }

    /// [`Metrics::poll`], published through `persist`.
    #[inline]
    pub(crate) fn poll_through(&self, now: Nanos, persist: &Persist) {
        if now.0 >= self.registry.next_due.load(Relaxed) {
            self.publish_due(now, persist);
        }
    }

    pub(crate) fn flush_through(&self, now: Nanos, persist: &Persist) {
        self.flush_with(now, persist.max_payload(), |len, write| {
            persist.claim(len, write);
        });
    }

    /// Finish already-open cycles, then snapshot the last partial interval at EOF.
    fn flush_with(
        &self,
        now: Nanos,
        max_payload: usize,
        mut publish: impl FnMut(usize, &mut dyn FnMut(&mut [u8]) -> bool),
    ) {
        type Publish<'a> = dyn FnMut(usize, &mut dyn FnMut(&mut [u8]) -> bool) + 'a;
        let mut state = self.state();
        let interval = self.registry.interval_ns;
        let drain = |state: &mut State, publish: &mut Publish<'_>| {
            while state.hist_open || state.active {
                let next = if state.hist_open {
                    state.next_histogram(max_payload)
                } else {
                    state.next_metrics(max_payload)
                };
                if let Some(next) = next {
                    publish(state.len(&next), &mut |out| {
                        state.encode(&next, interval, out).is_ok()
                    });
                    state.finish_publish(&next);
                } else if state.hist_open {
                    state.hist_open = false;
                } else {
                    state.active = false;
                }
            }
        };
        drain(&mut state, &mut publish);
        state.take_histogram_samples();
        if !state.cycle.samples.is_empty() {
            let end = u64::try_from(now.0).unwrap_or(0);
            state.cycle.hist_ts = end;
            state.cycle.hist_interval = end.saturating_sub(state.last_hist_end).max(1);
            state.last_hist_end = end;
            state.cycle.next_sample = 0;
            state.cycle.next_hist_def = 0;
            state.hist_open = true;
            drain(&mut state, &mut publish);
        }
        state.snapshot_metrics(u64::try_from(now.0).unwrap_or(0));
        drain(&mut state, &mut publish);
        state.next_metrics = self.next_boundary(now).0;
        state.next_hist = if state.next_hist == i64::MAX {
            i64::MAX
        } else {
            now.0.saturating_add(HISTOGRAM_MS)
        };
        self.store_due(&state);
    }

    #[cold]
    fn publish_due(&self, now: Nanos, persist: &Persist) {
        self.poll_with(now, persist.max_payload(), |len, write| {
            persist.claim(len, write);
        });
    }

    /// [`Metrics::poll`] once a deadline is due: publish one message
    /// through `publish(len, write)`.
    fn poll_with(
        &self,
        now: Nanos,
        max_payload: usize,
        publish: impl FnOnce(usize, &mut dyn FnMut(&mut [u8]) -> bool),
    ) {
        // Another thread is polling: it publishes.
        let Ok(mut state) = self.registry.state.try_lock() else {
            return;
        };
        if !state.hist_open && !state.active {
            if now.0 < self.registry.next_due.load(Relaxed) {
                return;
            }
            if state.next_hist != i64::MAX
                && now.0 >= state.next_hist
                && state.next_hist < state.next_metrics
            {
                state.open_histogram(now);
            }
            // An empty histogram interval does not consume a publishing call.
            if !state.hist_open && state.next_metrics != i64::MAX && now.0 >= state.next_metrics {
                let i = self.registry.interval_ns;
                let end = now.epoch_ns().div_euclid(i) * i;
                state.snapshot_metrics(u64::try_from(end).unwrap_or(0));
            }
            if !state.hist_open && !state.active {
                self.store_due(&state);
                return;
            }
            self.registry.next_due.store(i64::MIN, Relaxed);
        }
        let interval = self.registry.interval_ns;
        let next = if state.hist_open {
            state.next_histogram(max_payload)
        } else if state.active {
            state.next_metrics(max_payload)
        } else {
            None
        };
        match next {
            Some(next) => {
                let len = state.len(&next);
                publish(len, &mut |buf| state.encode(&next, interval, buf).is_ok());
                state.finish_publish(&next);
                if !state.hist_open && !state.active {
                    self.store_due(&state);
                }
            }
            None if state.hist_open => {
                state.hist_open = false;
                if !state.active {
                    self.store_due(&state);
                }
            }
            None => {
                state.active = false;
                state.next_metrics = self.next_boundary(now).0;
                self.store_due(&state);
            }
        }
    }

    /// Take the histogram summaries and the counter interval ending at
    /// `end_ns` (UNIX ns) and return every message they make.
    #[cfg(test)]
    fn drain(&self, end_ns: u64, max_payload: usize) -> Vec<Vec<u8>> {
        let mut state = self.state();
        let mut out = Vec::new();
        let now = Nanos::from_epoch(i64::try_from(end_ns).unwrap_or(i64::MAX));
        if state.open_histogram(now) {
            while state.hist_open {
                let Some(next) = state.next_histogram(max_payload) else {
                    state.hist_open = false;
                    break;
                };
                let mut buf = vec![0; state.len(&next)];
                assert!(
                    state
                        .encode(&next, self.registry.interval_ns, &mut buf)
                        .is_ok()
                );
                state.finish_publish(&next);
                out.push(buf);
            }
        }
        state.snapshot_metrics(end_ns);
        while let Some(next) = state.next_metrics(max_payload) {
            let mut buf = vec![0; state.len(&next)];
            assert!(
                state
                    .encode(&next, self.registry.interval_ns, &mut buf)
                    .is_ok()
            );
            state.finish_publish(&next);
            out.push(buf);
        }
        state.active = false;
        out
    }

    /// How many cells [`Self::add_counter`] and [`Self::counter`] have for the series.
    #[cfg(test)]
    pub(crate) fn counter_cells(&self, name: &str, labels: &[(&str, &str)]) -> usize {
        let state = self.state();
        let series = MetricDef::new(name, MetricKind::Counter, labels).series;
        let Some(&index) = state.by_series.get(&series) else {
            return 0;
        };
        state
            .cells
            .iter()
            .filter(|cell| cell.def == index && matches!(cell.source, Source::Counter(_)))
            .count()
    }

    /// The total [`Self::add_counter`] and any [`Counter`] handles have added.
    #[cfg(test)]
    pub(crate) fn counter_total(&self, name: &str, labels: &[(&str, &str)]) -> u64 {
        let state = self.state();
        let series = MetricDef::new(name, MetricKind::Counter, labels).series;
        let Some(&index) = state.by_series.get(&series) else {
            return 0;
        };
        state
            .cells
            .iter()
            .filter(|cell| cell.def == index)
            .map(|cell| match &cell.source {
                Source::Counter(counter) => counter.value.load(Relaxed),
                _ => 0,
            })
            .sum()
    }

    /// Samples recorded by [`Self::record_histogram`] and any [`Histogram`] handles.
    #[cfg(test)]
    pub(crate) fn histogram_samples(&self, name: &str, labels: &[(&str, &str)]) -> u64 {
        let state = self.state();
        let series = MetricDef::new(name, MetricKind::Histogram, labels).series;
        let Some(&index) = state.by_series.get(&series) else {
            return 0;
        };
        state
            .cells
            .iter()
            .filter(|cell| cell.def == index)
            .map(|cell| match &cell.source {
                Source::Histogram(histogram) => histogram.sample().count,
                Source::LocalHistogram(histogram) => histogram.count.load(Relaxed),
                _ => 0,
            })
            .sum()
    }
}

impl State {
    /// Swap every histogram cell into one summary per series. Empty cells
    /// stay for the next millisecond. `false` when nothing was recorded.
    fn open_histogram(&mut self, now: Nanos) -> bool {
        if !self.cells.iter().any(|cell| {
            matches!(
                cell.source,
                Source::Histogram(_) | Source::LocalHistogram(_)
            )
        }) {
            return false;
        }
        let end_i = now.epoch_ns().div_euclid(HISTOGRAM_MS) * HISTOGRAM_MS;
        let end = u64::try_from(end_i).unwrap_or(0);
        if self.last_hist_end != 0 && end <= self.last_hist_end {
            return false;
        }
        let interval = end.saturating_sub(self.last_hist_end);
        self.last_hist_end = end;
        self.next_hist = Nanos::from_epoch(end_i.saturating_add(HISTOGRAM_MS)).0;
        self.take_histogram_samples();
        if self.cycle.samples.is_empty() {
            return false;
        }
        self.cycle.hist_ts = end;
        self.cycle.hist_interval = if interval == 0 {
            u64::try_from(HISTOGRAM_MS).unwrap_or(0)
        } else {
            interval
        };
        self.cycle.next_sample = 0;
        self.cycle.next_hist_def = 0;
        self.hist_open = true;
        true
    }

    fn take_histogram_samples(&mut self) -> &[(u64, HistSample)] {
        self.cycle.samples.clear();
        self.cycle.hist_defs.clear();
        self.histogram_slots.clear();
        self.histogram_slots.resize(self.defs.len(), None);
        for cell in &self.cells {
            let sample = match &cell.source {
                Source::Histogram(histogram) => std::mem::take(&mut *histogram.sample()),
                Source::LocalHistogram(histogram) => histogram.take(),
                _ => continue,
            };
            if sample.count == 0 {
                continue;
            }
            if let Some(index) = self.histogram_slots[cell.def] {
                let acc = &mut self.cycle.samples[index.get() - 1].1;
                acc.count = acc.count.wrapping_add(sample.count);
                acc.sum = acc.sum.wrapping_add(sample.sum);
                acc.min = acc.min.min(sample.min);
                acc.max = acc.max.max(sample.max);
            } else {
                self.histogram_slots[cell.def] = NonZeroUsize::new(self.cycle.samples.len() + 1);
                let def = &self.defs[cell.def];
                self.cycle.samples.push((def.metric.series, sample));
                if !def.announced && !def.message.is_empty() {
                    self.cycle.hist_defs.push(cell.def);
                }
            }
        }
        &self.cycle.samples
    }

    /// Read counters and gauges into the interval ending at `ts` (UNIX ns).
    fn snapshot_metrics(&mut self, ts: u64) {
        let cycle = &mut self.cycle;
        (cycle.ts, cycle.next_def) = (ts, 0);
        (cycle.next_counter, cycle.next_gauge) = (0, 0);
        cycle.counters.clear();
        cycle.gauges.clear();
        let totals = &mut self.totals;
        totals.clear();
        totals.resize(self.defs.len(), None);
        for cell in &mut self.cells {
            let def = &mut self.defs[cell.def];
            match &mut cell.source {
                Source::Counter(c) => {
                    let total = &mut totals[cell.def];
                    *total = Some(total.unwrap_or(0).wrapping_add(c.value.load(Relaxed)));
                }
                Source::Sampled(read) => {
                    let total = &mut totals[cell.def];
                    *total = Some(total.unwrap_or(0).wrapping_add(read()));
                }
                Source::Gauge(g) => cycle
                    .gauges
                    .push((def.metric.series, f64::from_bits(g.bits.load(Relaxed)))),
                Source::Histogram(_) | Source::LocalHistogram(_) => {}
            }
        }
        for (def, total) in self.defs.iter_mut().zip(totals.iter()) {
            let Some(total) = *total else {
                continue;
            };
            let delta = total.wrapping_sub(def.last_total);
            def.last_total = total;
            cycle.counters.push((def.metric.series, total, delta));
        }
        self.active = true;
    }

    /// The next message of the open histogram summary.
    fn next_histogram(&mut self, max_payload: usize) -> Option<Next> {
        if self.cycle.next_hist_def < self.cycle.hist_defs.len() {
            let def = self.cycle.hist_defs[self.cycle.next_hist_def];
            self.cycle.next_hist_def += 1;
            self.defs[def].announced = true;
            return Some(Next::Def(def));
        }
        let left = self
            .cycle
            .samples
            .len()
            .saturating_sub(self.cycle.next_sample);
        if left == 0 {
            return None;
        }
        let base = codec::HistogramEncoder::compute_length_with_header(0);
        let per = codec::HistogramEncoder::compute_length_with_header(1)
            .saturating_sub(base)
            .max(1);
        let n = left
            .min(max_payload.saturating_sub(base) / per)
            .min(usize::from(u16::MAX));
        if n == 0 {
            log::error!("metrics: a {max_payload}-byte message holds no histogram; dropped");
            self.cycle.next_sample = self.cycle.samples.len();
            return None;
        }
        let start = self.cycle.next_sample;
        self.cycle.next_sample += n;
        Some(Next::Histogram { start, len: n })
    }

    /// The next message of the counter/gauge interval, `None` once all are out.
    fn next_metrics(&mut self, max_payload: usize) -> Option<Next> {
        let defs = self.defs.len();
        if !self.active {
            return None;
        }
        let cycle = &mut self.cycle;
        while cycle.next_def < defs {
            cycle.next_def += 1;
            let def = cycle.next_def - 1;
            if !self.defs[def].message.is_empty() {
                self.defs[def].announced = true;
                return Some(Next::Def(def));
            }
        }
        let (rc, rg) = (
            cycle.counters.len() - cycle.next_counter,
            cycle.gauges.len() - cycle.next_gauge,
        );
        if rc + rg > 0 {
            let base = codec::MetricsEncoder::compute_length_with_header(0, 0);
            let room = max_payload.saturating_sub(base);
            let counters = rc.min(room / 24).min(usize::from(u16::MAX));
            let gauges = rg
                .min((room - counters * 24) / 16)
                .min(usize::from(u16::MAX));
            let (c, g) = (cycle.next_counter, cycle.next_gauge);
            if counters + gauges == 0 {
                log::error!("metrics: a {max_payload}-byte message holds no counter; dropped");
                (cycle.next_counter, cycle.next_gauge) = (cycle.counters.len(), cycle.gauges.len());
            } else {
                cycle.next_counter += counters;
                cycle.next_gauge += gauges;
                return Some(Next::Metrics {
                    counters: c..c + counters,
                    gauges: g..g + gauges,
                });
            }
        }
        None
    }

    fn len(&self, next: &Next) -> usize {
        match next {
            Next::Def(def) => self.defs[*def].message.len(),
            Next::Metrics { counters, gauges } => {
                codec::MetricsEncoder::compute_length_with_header(counters.len(), gauges.len())
            }
            Next::Histogram { len, .. } => {
                codec::HistogramEncoder::compute_length_with_header(*len)
            }
        }
    }

    /// `next` was published, or could not be.
    fn finish_publish(&mut self, next: &Next) {
        if let Next::Histogram { .. } = next
            && self.cycle.next_sample >= self.cycle.samples.len()
            && self.cycle.next_hist_def >= self.cycle.hist_defs.len()
        {
            self.hist_open = false;
            self.cycle.samples.clear();
        }
    }

    /// Write `next` into `buf`, exactly [`State::len`] bytes.
    fn encode(
        &self,
        next: &Next,
        interval_ns: i64,
        buf: &mut [u8],
    ) -> Result<(), codec::sbe_rt::EncodeError> {
        let interval = u64::try_from(interval_ns).unwrap_or(0);
        let ts = self.cycle.ts;
        match next {
            Next::Def(def) => {
                buf.copy_from_slice(&self.defs[*def].message);
                Ok(())
            }
            Next::Metrics { counters, gauges } => {
                let cycle = &self.cycle;
                let c = &cycle.counters[counters.clone()];
                let g = &cycle.gauges[gauges.clone()];
                let n_counters = crate::event::group_count(c.len())?;
                let n_gauges = crate::event::group_count(g.len())?;
                let len = codec::MetricsEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&codec::MetricsFixedFields { ts, interval })
                    .counters(n_counters, |group| {
                        for &(series, value, delta) in c {
                            group.add(|e| {
                                e.series(series).value(value).delta(delta);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?
                    .gauges(n_gauges, |group| {
                        for &(series, value) in g {
                            group.add(|e| {
                                e.series(series).value(value);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?
                    .encoded_length_with_header();
                debug_assert_eq!(len, buf.len());
                Ok(())
            }
            &Next::Histogram { start, len: n } => {
                let samples = &self.cycle.samples[start..start + n];
                let n_samples = crate::event::group_count(n)?;
                let len = codec::HistogramEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&codec::HistogramFixedFields {
                        ts: self.cycle.hist_ts,
                        interval: self.cycle.hist_interval,
                    })
                    .samples(n_samples, |group| {
                        for (series, sample) in samples {
                            group.add(|e| {
                                e.series(*series)
                                    .count(sample.count)
                                    .sum(sample.sum)
                                    .min(sample.min)
                                    .max(sample.max);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?
                    .encoded_length_with_header();
                debug_assert_eq!(len, buf.len());
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Values around every power of two, and all small ones.
    fn values() -> impl Iterator<Item = u64> {
        (0..4096)
            .chain((12..64).flat_map(|b| {
                let p = 1u64 << b;
                [p - 3, p - 1, p, p + 1, p + p / 3, p + p / 2 + 7]
            }))
            .chain([u64::MAX - 1, u64::MAX])
    }

    #[test]
    fn eof_before_the_first_interval_still_publishes_rows_at_eof() -> TestResult {
        let metrics = Metrics::detached();
        metrics.set_simulation(Nanos(0));
        metrics.counter("c", &[]).add(7);
        metrics.local_histogram("h", &[]).record(13);
        let mut messages = Vec::new();
        metrics.flush_with(Nanos(123), 64 * 1024, |len, write| {
            let mut buffer = vec![0; len];
            assert!(write(&mut buffer));
            messages.push(buffer);
        });
        let decoded = decode(&messages)?;
        assert_eq!(decoded.histograms.len(), 1);
        assert_eq!(decoded.histograms[0].ts, 123);
        assert_eq!(decoded.histograms[0].count, 1);
        assert!(decoded.counters.iter().any(|(_, total, _)| *total == 7));
        Ok(())
    }

    #[test]
    fn eof_flush_finishes_a_partial_batch_and_preserves_final_samples() -> TestResult {
        let metrics = Metrics::detached();
        metrics.set_simulation(Nanos(0));
        let counter = metrics.counter("c", &[]);
        let histogram = metrics.local_histogram("h", &[]);
        counter.add(7);
        histogram.record(13);
        let now = Nanos(5_000_000_123);
        let mut messages = Vec::new();
        metrics.poll_with(now, 64 * 1024, |len, write| {
            let mut buffer = vec![0; len];
            assert!(write(&mut buffer));
            messages.push(buffer);
        });
        assert!(
            metrics.state().hist_open || metrics.state().active,
            "one poll leaves the batch incomplete"
        );
        metrics.flush_with(now, 64 * 1024, |len, write| {
            let mut buffer = vec![0; len];
            assert!(write(&mut buffer));
            messages.push(buffer);
        });
        let decoded = decode(&messages)?;
        assert_eq!(
            decoded
                .histograms
                .iter()
                .map(|sample| sample.count)
                .sum::<u64>(),
            1
        );
        assert!(decoded.counters.iter().any(|(_, total, _)| *total == 7));
        let state = metrics.state();
        assert!(!state.hist_open && !state.active);
        assert!(state.next_metrics > now.0);
        drop(state);
        Ok(())
    }

    #[test]
    fn simulation_arms_metrics_and_new_histograms_at_simulated_epoch() {
        let metrics = Metrics::detached();
        metrics.set_simulation(Nanos(123_000_000));
        assert_eq!(metrics.state().next_metrics, 5_000_000_000);
        metrics.sim_time(Nanos(456_000_000));
        let hist = metrics.local_histogram("latency", &[]);
        hist.record(10);
        assert_eq!(metrics.state().next_hist, 457_000_000);
        assert_eq!(metrics.registry.next_due.load(Relaxed), 457_000_000);
    }

    #[test]
    fn buckets_cover_every_value_within_their_precision() {
        for precision in [0, 1, 3, PRECISION, 7] {
            let last = bucket_count(precision) - 1;
            assert_eq!(bucket(u64::MAX, precision), last);
            assert_eq!(bucket_bounds(last, precision).1, u64::MAX);
            let mut previous = 0;
            for v in values() {
                let i = bucket(v, precision);
                assert!(i >= previous, "bucket index falls at {v}");
                previous = i;
                let (low, high) = bucket_bounds(i, precision);
                assert!(
                    low <= v && v <= high,
                    "{v} not in bucket {i} = {low}..={high}"
                );
                #[allow(clippy::cast_precision_loss)] // relative width, not an integer bound
                let within =
                    (high - low) as f64 <= low as f64 / f64::from(1u32 << precision) || high == low;
                assert!(
                    within,
                    "bucket {i} ({low}..={high}) wider than 2^-{precision} of its values"
                );
            }
            // Contiguous: each bucket starts where the last one ended.
            for i in 1..=last {
                assert_eq!(
                    bucket_bounds(i, precision).0,
                    bucket_bounds(i - 1, precision).1 + 1
                );
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Hist {
        series: u64,
        count: u64,
        sum: u64,
        min: u64,
        max: u64,
        ts: u64,
        interval: u64,
    }

    struct Decoded {
        defs: Vec<MetricDef>,
        counters: Vec<(u64, u64, u64)>,
        gauges: Vec<(u64, f64)>,
        histograms: Vec<Hist>,
        ts: Vec<u64>,
    }

    fn decode(messages: &[Vec<u8>]) -> Result<Decoded, Box<dyn std::error::Error>> {
        let mut d = Decoded {
            defs: Vec::new(),
            counters: Vec::new(),
            gauges: Vec::new(),
            histograms: Vec::new(),
            ts: Vec::new(),
        };
        for m in messages {
            match u16::from_le_bytes([m[2], m[3]]) {
                METRIC_DEF_TEMPLATE_ID => d.defs.push(MetricDef::decode(m).ok_or("bad def")?),
                METRICS_TEMPLATE_ID => {
                    let msg = codec::MetricsDecoder::decode(m, 0)?;
                    d.ts.push(msg.ts());
                    for e in msg.counters()? {
                        d.counters.push((e.series(), e.value(), e.delta()));
                    }
                    for e in msg.gauges()? {
                        d.gauges.push((e.series(), e.value()));
                    }
                }
                HISTOGRAM_TEMPLATE_ID => {
                    let h = codec::HistogramDecoder::decode(m, 0)?;
                    let ts = h.ts();
                    let interval = h.interval();
                    for e in h.samples()? {
                        d.histograms.push(Hist {
                            series: e.series(),
                            count: e.count(),
                            sum: e.sum(),
                            min: e.min(),
                            max: e.max(),
                            ts,
                            interval,
                        });
                    }
                }
                other => return Err(format!("template {other}").into()),
            }
        }
        Ok(d)
    }

    #[test]
    fn counters_add_up_across_cells_and_send_deltas() -> TestResult {
        let metrics = Metrics::detached();
        let a = metrics.counter("sent", &[("venue", "x"), ("side", "buy")]);
        // The same series in another order, as another thread would ask.
        let b = metrics.counter("sent", &[("side", "buy"), ("venue", "x")]);
        let other = metrics.counter("sent", &[("venue", "y")]);
        a.add(3);
        b.inc();
        let first = decode(&metrics.drain(5_000_000_000, 64 * 1024))?;
        assert_eq!(first.defs.len(), 2, "one def per series, not per cell");
        let series = MetricDef::new(
            "sent",
            MetricKind::Counter,
            &[("venue", "x"), ("side", "buy")],
        )
        .series;
        assert!(first.counters.contains(&(series, 4, 4)));
        assert!(first.counters.contains(&(
            MetricDef::new("sent", MetricKind::Counter, &[("venue", "y")]).series,
            0,
            0
        )));
        assert_eq!(first.ts, [5_000_000_000]);

        a.add(10);
        other.inc();
        let second = decode(&metrics.drain(10_000_000_000, 64 * 1024))?;
        assert!(
            second.counters.contains(&(series, 14, 10)),
            "{:?}",
            second.counters
        );
        Ok(())
    }

    #[test]
    fn tracing_counter_reuses_one_cell_and_publishes_the_sum() -> TestResult {
        let metrics = Metrics::detached();
        metrics.add_counter("orders_sent", &[("venue", "binance")], 1);
        metrics.add_counter("orders_sent", &[("venue", "binance")], 4);
        let handle = metrics.counter("orders_sent", &[("venue", "binance")]);
        handle.inc();
        assert_eq!(
            metrics.counter_cells("orders_sent", &[("venue", "binance")]),
            2,
            "the shared tracing cell stays separate from a one-writer handle"
        );
        let decoded = decode(&metrics.drain(5_000_000_000, 64 * 1024))?;
        let series =
            MetricDef::new("orders_sent", MetricKind::Counter, &[("venue", "binance")]).series;
        assert_eq!(decoded.defs.len(), 1);
        assert!(decoded.counters.contains(&(series, 6, 6)));
        Ok(())
    }

    #[test]
    fn gauges_share_one_cell_and_the_last_value_wins() -> TestResult {
        let metrics = Metrics::detached();
        let g = metrics.gauge("depth", &[]);
        let h = metrics.gauge("depth", &[]);
        g.set(1.5);
        h.set(-2.0);
        let d = decode(&metrics.drain(1, 64 * 1024))?;
        assert_eq!(
            d.gauges,
            [(MetricDef::new("depth", MetricKind::Gauge, &[]).series, -2.0)]
        );
        Ok(())
    }

    #[test]
    fn local_histograms_merge_with_locked_ones_and_reset_on_publish() -> TestResult {
        let metrics = Metrics::detached();
        let local = metrics.local_histogram("lat", &[]);
        let locked = metrics.histogram("lat", &[]);
        for v in [100, 5_000, 1_000_000] {
            local.record(v);
        }
        locked.record(7);
        {
            let mut state = metrics.state();
            state.last_hist_end = 0;
        }
        let d = decode(&metrics.drain(1_000_000, 64 * 1024))?;
        let [sample] = d.histograms.as_slice() else {
            return Err(format!("{} histogram summaries", d.histograms.len()).into());
        };
        assert_eq!(
            (sample.count, sample.sum, sample.min, sample.max),
            (4, 1_005_107, 7, 1_000_000)
        );
        local.record(42);
        let d = decode(&metrics.drain(2_000_000, 64 * 1024))?;
        let [sample] = d.histograms.as_slice() else {
            return Err(format!("{} histogram summaries", d.histograms.len()).into());
        };
        assert_eq!(
            (sample.count, sample.sum, sample.min, sample.max),
            (1, 42, 42, 42)
        );
        Ok(())
    }

    #[test]
    fn histograms_publish_the_millisecond_summary_once() -> TestResult {
        let metrics = Metrics::detached();
        let h = metrics.histogram("lat", &[]);
        let other_thread = metrics.histogram("lat", &[]);
        for v in [100, 100, 101, 5_000, 1_000_000] {
            h.record(v);
        }
        other_thread.record(7);
        // `histogram()` arms the clock at the real time. These drains use
        // small UNIX timestamps, so the window starts at 0.
        {
            let mut state = metrics.state();
            state.last_hist_end = 0;
        }
        let d = decode(&metrics.drain(1_000_000, 64 * 1024))?;
        let [sample] = d.histograms.as_slice() else {
            return Err(format!("{} histogram summaries", d.histograms.len()).into());
        };
        assert_eq!(
            (sample.count, sample.sum, sample.min, sample.max),
            (6, 1_005_308, 7, 1_000_000)
        );
        assert_eq!(sample.ts, 1_000_000);
        assert_eq!(sample.interval, 1_000_000);
        assert_eq!(
            sample.series,
            MetricDef::new("lat", MetricKind::Histogram, &[]).series
        );

        assert!(
            decode(&metrics.drain(2_000_000, 64 * 1024))?
                .histograms
                .is_empty()
        );

        h.record(42);
        let d = decode(&metrics.drain(3_000_000, 64 * 1024))?;
        let [sample] = d.histograms.as_slice() else {
            return Err(format!("{} histogram summaries", d.histograms.len()).into());
        };
        assert_eq!(
            (sample.count, sample.sum, sample.min, sample.max),
            (1, 42, 42, 42)
        );
        assert_eq!(sample.ts, 3_000_000);
        assert_eq!(sample.interval, 1_000_000);
        Ok(())
    }

    #[test]
    fn what_does_not_fit_one_message_is_split() -> TestResult {
        let metrics = Metrics::detached();
        let counters: Vec<Counter> = (0..100)
            .map(|i| metrics.counter("c", &[("i", &i.to_string())]))
            .collect();
        for (i, c) in counters.iter().enumerate() {
            c.add(i as u64);
        }
        let histograms: Vec<Histogram> = (0..40)
            .map(|i| metrics.histogram("h", &[("i", &i.to_string())]))
            .collect();
        for h in &histograms {
            h.record(1);
        }
        {
            let mut state = metrics.state();
            state.last_hist_end = 0;
        }
        let messages = metrics.drain(1_000_000, 1024);
        assert!(messages.iter().all(|m| m.len() <= 1024));
        let d = decode(&messages)?;
        assert_eq!(
            d.counters.len(),
            100,
            "every counter, over several messages"
        );
        assert_eq!(d.histograms.len(), 40);
        assert!(d.histograms.iter().all(|h| h.count == 1));
        assert_eq!(d.histograms.iter().map(|h| h.count).sum::<u64>(), 40);
        Ok(())
    }

    #[test]
    #[allow(clippy::float_cmp, clippy::significant_drop_tightening)]
    fn busy_histograms_do_not_starve_counters_or_gauges() -> TestResult {
        let metrics = Metrics::detached();
        let counter = metrics.counter("sent", &[]);
        let gauge = metrics.gauge("depth", &[]);
        let histogram = metrics.histogram("latency", &[]);
        counter.add(3);
        gauge.set(12.0);
        let start: u64 = 10_000_000_000;
        let next_hist = Nanos::from_epoch(start.cast_signed() + HISTOGRAM_MS).0;
        let next_metrics = Nanos::from_epoch(start.cast_signed() + 5 * HISTOGRAM_MS).0;
        {
            let mut state = metrics.state();
            state.last_hist_end = start;
            state.next_hist = next_hist;
            state.next_metrics = next_metrics;
            metrics.store_due(&state);
        }
        let mut messages = Vec::new();
        for millisecond in 1..=20 {
            histogram.record(7);
            metrics.poll_with(
                Nanos::from_epoch(start.cast_signed() + millisecond * HISTOGRAM_MS),
                64 * 1024,
                |len, write| {
                    let mut bytes = vec![0; len];
                    assert!(write(&mut bytes));
                    messages.push(bytes);
                },
            );
        }
        let decoded = decode(&messages)?;
        assert_eq!(
            decoded.counters.len(),
            1,
            "counter deadline must be serviced"
        );
        assert_eq!(decoded.counters[0].1, 3);
        assert_eq!(decoded.gauges.len(), 1);
        assert_eq!(decoded.gauges[0].1, 12.0);
        assert!(!decoded.histograms.is_empty());
        Ok(())
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn short_counter_intervals_do_not_starve_histograms() -> TestResult {
        let metrics = Metrics::new(std::time::Duration::from_millis(1));
        let counter = metrics.counter("sent", &[]);
        let histogram = metrics.histogram("latency", &[]);
        let start: u64 = 10_000_000_000;
        let next = Nanos::from_epoch(start.cast_signed() + HISTOGRAM_MS).0;
        {
            let mut state = metrics.state();
            state.last_hist_end = start;
            state.next_hist = next;
            state.next_metrics = next;
            metrics.store_due(&state);
        }
        let mut messages = Vec::new();
        for millisecond in 1..=20 {
            counter.inc();
            histogram.record(7);
            metrics.poll_with(
                Nanos::from_epoch(start.cast_signed() + millisecond * HISTOGRAM_MS),
                64 * 1024,
                |len, write| {
                    let mut bytes = vec![0; len];
                    assert!(write(&mut bytes));
                    messages.push(bytes);
                },
            );
        }
        let decoded = decode(&messages)?;
        assert!(!decoded.counters.is_empty());
        assert!(
            !decoded.histograms.is_empty(),
            "histogram deadline must be serviced"
        );
        Ok(())
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn histogram_snapshots_keep_first_nonempty_order_across_cycles() {
        let metrics = Metrics::detached();
        let a_empty = metrics.histogram("a", &[]);
        let b = metrics.histogram("b", &[]);
        let a = metrics.histogram("a", &[]);
        let series_a = MetricDef::new("a", MetricKind::Histogram, &[]).series;
        let series_b = MetricDef::new("b", MetricKind::Histogram, &[]).series;
        b.record(9);
        a.record(3);
        a.record(7);
        let samples = {
            let mut state = metrics.state();
            let samples = state.take_histogram_samples().to_vec();
            assert!(state.take_histogram_samples().is_empty());
            samples
        };
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].0, series_b);
        assert_eq!(samples[1].0, series_a);
        let sample = samples[1].1;
        assert_eq!(
            (sample.count, sample.sum, sample.min, sample.max),
            (2, 10, 3, 7)
        );
        let c = metrics.histogram("c", &[]);
        c.record(11);
        a_empty.record(1);
        a.record(5);
        let samples = {
            let mut state = metrics.state();
            state.take_histogram_samples().to_vec()
        };
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].0, series_a);
        let sample = samples[0].1;
        assert_eq!(
            (sample.count, sample.sum, sample.min, sample.max),
            (2, 6, 1, 5)
        );
        assert_eq!(
            samples[1].0,
            MetricDef::new("c", MetricKind::Histogram, &[]).series
        );
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn mixed_metrics_keep_totals_and_histogram_order_across_cycles() {
        let metrics = Metrics::detached();
        let first = metrics.histogram("first", &[]);
        let counters: Vec<_> = (0..4096)
            .map(|i| metrics.counter(&format!("counter-{i}"), &[]))
            .collect();
        let gauge = metrics.gauge("gauge", &[]);
        let last = metrics.histogram("last", &[]);
        let first_extra = metrics.histogram("first", &[]);
        let first_id = MetricDef::new("first", MetricKind::Histogram, &[]).series;
        let last_id = MetricDef::new("last", MetricKind::Histogram, &[]).series;
        for counter in &counters {
            counter.add(3);
        }
        gauge.set(7.0);
        first.record(1);
        first_extra.record(5);
        last.record(9);
        let samples = {
            let mut state = metrics.state();
            state.snapshot_metrics(5_000_000_000);
            assert!(
                state
                    .cycle
                    .counters
                    .iter()
                    .all(|(_, total, delta)| (*total, *delta) == (3, 3))
            );
            assert_eq!(state.cycle.counters.len(), counters.len());
            let samples = state.take_histogram_samples().to_vec();
            assert!(state.take_histogram_samples().is_empty());
            samples
        };
        assert_eq!(
            samples.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [first_id, last_id]
        );
        assert_eq!(samples[0].1.count, 2);
        assert_eq!(samples[0].1.sum, 6);
        counters[0].add(2);
        first_extra.record(4);
        last.record(8);
        let samples = {
            let mut state = metrics.state();
            state.take_histogram_samples().to_vec()
        };
        assert_eq!(
            samples.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [last_id, first_id]
        );
        assert_eq!(samples[1].1.sum, 4);
        let gauge = {
            let mut state = metrics.state();
            state.snapshot_metrics(10_000_000_000);
            assert_eq!(
                (state.cycle.counters[0].1, state.cycle.counters[0].2),
                (5, 2)
            );
            assert!(
                state.cycle.counters[1..]
                    .iter()
                    .all(|(_, total, delta)| (*total, *delta) == (3, 0))
            );
            assert_eq!(state.cycle.gauges.len(), 1);
            state.cycle.gauges[0].1
        };
        #[allow(clippy::float_cmp)] // the gauge was set to this exact value
        {
            assert_eq!(gauge, 7.0);
        }
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn histogram_cycles_reuse_warmed_summary_and_definition_storage() {
        let metrics = Metrics::detached();
        let histogram = metrics.histogram("latency", &[]);
        let (samples_capacity, defs_capacity) = {
            let mut state = metrics.state();
            state.cycle.samples.reserve(128);
            state.cycle.hist_defs.reserve(128);
            state.last_hist_end = 10_000_000_000;
            (
                state.cycle.samples.capacity(),
                state.cycle.hist_defs.capacity(),
            )
        };
        for millisecond in 1..=3 {
            histogram.record(7);
            let mut state = metrics.state();
            assert!(state.open_histogram(Nanos::from_epoch(
                10_000_000_000 + millisecond * HISTOGRAM_MS
            )));
            assert_eq!(state.cycle.samples.len(), 1);
            assert_eq!(state.cycle.hist_defs.len(), usize::from(millisecond == 1));
            assert_eq!(state.cycle.samples.capacity(), samples_capacity);
            assert_eq!(state.cycle.hist_defs.capacity(), defs_capacity);
            while let Some(next) = state.next_histogram(usize::MAX) {
                state.finish_publish(&next);
            }
            assert!(!state.hist_open);
        }
    }

    #[test]
    fn concurrent_histogram_polling_preserves_complete_samples() {
        let metrics = Metrics::detached();
        let histogram = metrics.histogram("latency", &[]);
        let done = std::sync::atomic::AtomicBool::new(false);
        let mut count = 0;
        let mut inconsistent = false;
        std::thread::scope(|scope| {
            let done = &done;
            scope.spawn(move || {
                for _ in 0..100_000 {
                    histogram.record(7);
                }
                done.store(true, std::sync::atomic::Ordering::Release);
            });
            loop {
                let samples = metrics.state().take_histogram_samples().to_vec();
                for (_, sample) in samples {
                    count += sample.count;
                    inconsistent |=
                        sample.sum != sample.count * 7 || sample.min != 7 || sample.max != 7;
                }
                if done.load(std::sync::atomic::Ordering::Acquire) {
                    break;
                }
            }
        });
        let samples = metrics.state().take_histogram_samples().to_vec();
        for (_, sample) in samples {
            count += sample.count;
            inconsistent |= sample.sum != sample.count * 7 || sample.min != 7 || sample.max != 7;
        }
        assert!(
            !inconsistent,
            "polling must take count/sum/min/max together"
        );
        assert_eq!(count, 100_000, "each observation must be published once");
    }

    #[test]
    fn a_due_poll_publishes_one_message_per_call() -> TestResult {
        let metrics = Metrics::detached();
        metrics.start();
        let c = metrics.counter("c", &[]);
        let h = metrics.histogram("h", &[]);
        c.inc();
        h.record(5);
        // The next millisecond and the next 5 s boundary can be the same
        // instant. Pin them apart so the histogram summary is not also the
        // counter interval.
        let hist_due = {
            let mut state = metrics.state();
            let boundary = Nanos(state.next_hist).epoch_ns();
            state.next_metrics = Nanos::from_epoch(boundary + 5_000_000_000).0;
            state.next_hist
        };
        metrics.registry.next_due.store(hist_due, Relaxed);
        let mut published = Vec::new();
        let poll = |at: i64| {
            let mut sent = Vec::new();
            if at < metrics.registry.next_due.load(Relaxed) {
                return sent;
            }
            metrics.poll_with(Nanos(at), 64 * 1024, |len, write| {
                let mut buf = vec![0; len];
                assert!(write(&mut buf));
                sent.push(buf);
            });
            sent
        };
        assert!(poll(hist_due - 1).is_empty(), "not yet due");
        published.extend(poll(hist_due));
        assert_eq!(published.len(), 1, "the histogram def");
        published.extend(poll(hist_due));
        assert_eq!(published.len(), 2, "the summary");
        assert!(poll(hist_due).is_empty(), "that millisecond is out");
        let d = decode(&published)?;
        assert_eq!(d.defs.len(), 1);
        let [sample] = d.histograms.as_slice() else {
            return Err(format!("{} histogram summaries", d.histograms.len()).into());
        };
        assert_eq!((sample.count, sample.min, sample.max), (1, 5, 5));
        assert_eq!(sample.ts, u64::try_from(Nanos(hist_due).epoch_ns())?);

        published.clear();
        h.record(9);
        published.extend(poll(hist_due + 1_000_000));
        assert_eq!(published.len(), 1);
        let d = decode(&published)?;
        assert!(d.defs.is_empty(), "the series was already announced");
        let [sample] = d.histograms.as_slice() else {
            return Err(format!("{} histogram summaries", d.histograms.len()).into());
        };
        let series = MetricDef::new("h", MetricKind::Histogram, &[]).series;
        assert_eq!(
            (
                sample.series,
                sample.count,
                sample.sum,
                sample.min,
                sample.max
            ),
            (series, 1, 9, 9, 9)
        );

        published.clear();
        let metrics_due = metrics.state().next_metrics;
        metrics.registry.next_due.store(metrics_due, Relaxed);
        let mut sent = Vec::new();
        for _ in 0..6 {
            let batch = poll(metrics_due);
            sent.push(batch.len());
            published.extend(batch);
        }
        assert_eq!(sent, [1, 1, 1, 0, 0, 0]);
        let d = decode(&published)?;
        assert!(
            d.histograms.is_empty(),
            "counters do not republish histograms"
        );
        assert_eq!(d.defs.len(), 2);
        assert_eq!(d.counters.len(), 1);
        Ok(())
    }

    #[test]
    fn a_def_round_trips_through_its_message() -> TestResult {
        let def = MetricDef::new(
            "t2t",
            MetricKind::Histogram,
            &[("venue", "binance"), ("app", "a")],
        );
        let decoded = MetricDef::decode(&def.message()?).ok_or("undecodable")?;
        assert_eq!(decoded, def);
        assert_eq!(decoded.labels[0].0, "app", "sorted by key");
        assert_ne!(
            def.series,
            MetricDef::new(
                "t2t",
                MetricKind::Counter,
                &[("venue", "binance"), ("app", "a")]
            )
            .series
        );
        Ok(())
    }
}
