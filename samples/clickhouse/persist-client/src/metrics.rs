//! Metrics: counters, gauges and histograms, declared once with labels and
//! updated from the hot path with plain loads and stores.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use persist_client::clock::Clock;
//! # let persist = persist_client::Persist::connect("", persist_client::Settings::from_env())?;
//! let metrics = persist.metrics();
//! let sent = metrics.counter("orders_sent", &[("venue", "binance")]);
//! let depth = metrics.gauge("book_depth", &[("side", "bid")]);
//! let t2t = metrics.histogram("tick_to_trade_ns", &[]);
//! let clock = Clock::new();
//! loop {
//!     let now = clock.now();
//!     sent.inc(); // a load and a store
//!     depth.set(12.0); // a store
//!     t2t.record(850); // a bucket index, three load/stores, two compares
//!     metrics.poll(now); // one compare, until the interval ends
//! #   break;
//! }
//! # Ok(()) }
//! ```
//!
//! * A **counter** or **histogram** handle has one writer at a time: it is
//!   `Send` but not `Sync`, so an increment is a relaxed load and store, no
//!   locked instruction. Asking for the same series again (on another
//!   thread, say) gives another cell; [`Metrics::poll`] adds them up.
//! * A **gauge** is `Sync` and `Clone`: every handle of a series shares one
//!   cell, and the last value set wins.
//! * Off the hot path the same series can be updated from a `tracing` event,
//!   once [`crate::Persist::layer`] is installed:
//!   `tracing::info!(counter = "orders_sent", venue = "binance")`,
//!   `tracing::info!(gauge = "book_depth", value = 12.0)`,
//!   `tracing::info!(histogram = "tick_to_trade_ns", value = 850)`.
//!   That path allocates the label strings and shares one cell across threads.
//!   [`Metrics::poll`] still publishes it. A handle and a tracing event for
//!   the same counter or histogram are two cells, added together.
//! * A **histogram** keeps log-linear buckets (as HdrHistogram does): each
//!   power of two is split into `2^PRECISION` equal buckets, so a value is
//!   known to within `2^-PRECISION` (3.1%) of itself, from 0 to `u64::MAX`.
//!   Buckets merge exactly across intervals, threads and applications.
//!
//! Every interval (5 s by default, aligned to multiples of it in UNIX time,
//! so applications line up), [`Metrics::poll`] publishes each series' name
//! and labels (`MetricDef`, keyed by a hash of them), then the counters and
//! gauges (`Metrics`), then one `Histogram` per series that had samples. A
//! call publishes at most one message, and the next call continues, so no
//! call costs more than one publish. Call it from the thread's loop with
//! the time it already has; any thread may call it.

use std::cell::Cell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex, PoisonError};

use crate::Persist;
use crate::clock::Nanos;
use crate::event::codec;

/// Template id of the `MetricDef` message.
pub const METRIC_DEF_TEMPLATE_ID: u16 = codec::MetricDefEncoder::TEMPLATE_ID;
/// Template id of the `Metrics` message.
pub const METRICS_TEMPLATE_ID: u16 = codec::MetricsEncoder::TEMPLATE_ID;
/// Template id of the `Histogram` message.
pub const HISTOGRAM_TEMPLATE_ID: u16 = codec::HistogramEncoder::TEMPLATE_ID;

/// Sub-bucket bits of a histogram: every value is within `2^-5` (3.1%) of
/// its bucket's bounds.
pub const PRECISION: u8 = 5;

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
    let magnitude = 63 - (value | (1 << p)).leading_zeros();
    let shift = magnitude - p;
    ((shift as usize) << p) + (value >> shift) as usize
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
    Counter,
    Gauge,
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

    /// Its name in ClickHouse.
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
    pub series: u64,
    pub kind: MetricKind,
    /// A histogram's sub-bucket bits.
    pub precision: u8,
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
            _ => return None,
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

#[repr(align(128))]
struct HistogramCell {
    sum: AtomicU64,
    /// Since the last poll: [`Metrics::poll`] swaps them back.
    min: AtomicU64,
    max: AtomicU64,
    /// Totals since the cell was made.
    buckets: Box<[AtomicU64]>,
}

impl HistogramCell {
    fn new() -> Self {
        Self {
            sum: AtomicU64::new(0),
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
            buckets: (0..bucket_count(PRECISION))
                .map(|_| AtomicU64::new(0))
                .collect(),
        }
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
    /// Values recorded into this cell, all time.
    #[cfg(test)]
    pub(crate) fn cell_count(&self) -> u64 {
        self.cell.buckets.iter().map(|b| b.load(Relaxed)).sum()
    }

    /// Record one value: its bucket, the sum, the minimum and the maximum,
    /// each a relaxed load and store.
    #[inline]
    pub fn record(&self, value: u64) {
        let c = &*self.cell;
        if let Some(b) = c.buckets.get(bucket(value, PRECISION)) {
            b.store(b.load(Relaxed) + 1, Relaxed);
        }
        c.sum
            .store(c.sum.load(Relaxed).wrapping_add(value), Relaxed);
        if value > c.max.load(Relaxed) {
            c.max.store(value, Relaxed);
        }
        if value < c.min.load(Relaxed) {
            c.min.store(value, Relaxed);
        }
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
    Histogram(Arc<HistogramCell>, Box<[u64]>, u64),
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
    def: MetricDef,
    message: Vec<u8>,
    /// A counter's total at the last poll.
    last_total: u64,
    /// A gauge's one cell.
    gauge: Option<Arc<GaugeCell>>,
    /// A histogram's interval being merged from its cells.
    hist: Option<Box<Snap>>,
    /// The cell [`Metrics::add_counter`] reuses. A [`Counter`] handle stays
    /// its own cell, so a tracing event does not race that handle's add.
    shared_counter: Option<Arc<CounterCell>>,
    /// The cell [`Metrics::record_histogram`] reuses, for the same reason.
    shared_histogram: Option<Arc<HistogramCell>>,
}

/// One histogram series over one interval.
struct Snap {
    count: u64,
    sum: u64,
    min: u64,
    max: u64,
    buckets: Box<[u64]>,
    /// Lowest and highest bucket with counts, to skip the empty rest.
    low: usize,
    high: usize,
}

impl Snap {
    fn new() -> Box<Self> {
        Box::new(Self {
            count: 0,
            sum: 0,
            min: u64::MAX,
            max: 0,
            buckets: vec![0; bucket_count(PRECISION)].into_boxed_slice(),
            low: usize::MAX,
            high: 0,
        })
    }

    fn clear(&mut self) {
        if self.low <= self.high {
            self.buckets[self.low..=self.high].fill(0);
        }
        (self.count, self.sum, self.min, self.max) = (0, 0, u64::MAX, 0);
        (self.low, self.high) = (usize::MAX, 0);
    }

    /// The non-empty buckets at `precision` (at most [`PRECISION`]):
    /// `(index, count)` in order. Coarser buckets merge finer ones.
    fn buckets(&self, precision: u8) -> impl Iterator<Item = (usize, u64)> + '_ {
        let range = self.low..self.high.max(self.low.saturating_sub(1)) + 1;
        let range = range.start.min(self.buckets.len())..range.end.min(self.buckets.len());
        let mut merged = self.buckets[range.clone()]
            .iter()
            .zip(range)
            .filter(|(c, _)| **c > 0)
            .map(move |(c, i)| (bucket(bucket_bounds(i, PRECISION).0, precision), *c))
            .peekable();
        std::iter::from_fn(move || {
            let (index, mut count) = merged.next()?;
            while let Some((_, more)) = merged.next_if(|(i, _)| *i == index) {
                count += more;
            }
            Some((index, count))
        })
    }
}

/// The messages one interval still has to publish, in order.
#[derive(Default)]
struct Cycle {
    /// UNIX ns of the interval's end.
    ts: u64,
    next_def: usize,
    counters: Vec<(u64, u64, u64)>,
    gauges: Vec<(u64, f64)>,
    next_counter: usize,
    next_gauge: usize,
    /// Series indexes with a histogram to publish.
    histograms: Vec<usize>,
    next_histogram: usize,
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
        def: usize,
        precision: u8,
        buckets: usize,
    },
}

struct State {
    defs: Vec<Def>,
    by_series: HashMap<u64, usize>,
    cells: Vec<Cellref>,
    /// The interval being published, while `active`. Reused, so polling
    /// allocates nothing once every series has been seen.
    cycle: Cycle,
    active: bool,
    /// A counter series' total over its cells, by series index; reused.
    totals: Vec<Option<u64>>,
}

struct Registry {
    interval_ns: i64,
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
                next_due: AtomicI64::new(i64::MAX),
                state: Mutex::new(State {
                    defs: Vec::new(),
                    by_series: HashMap::new(),
                    cells: Vec::new(),
                    cycle: Cycle::default(),
                    active: false,
                    totals: Vec::new(),
                }),
            }),
            persist: None,
        }
    }

    /// Series that nothing publishes: handles work, and cost what they
    /// always do. What [`fn@crate::metrics`] returns with no handle installed.
    #[must_use]
    pub fn detached() -> Self {
        Self::new(std::time::Duration::from_secs(5))
    }

    /// Publish from the end of the current interval on, once a handle made
    /// with [`Metrics::published_by`] polls.
    pub(crate) fn start(&self) {
        let now = crate::clock::Clock::new().now();
        self.registry
            .next_due
            .store(self.next_boundary(now).0, Relaxed);
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
        let mut state = self.state();
        let def = self.def(&mut state, name, MetricKind::Gauge, labels);
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
        Gauge { cell }
    }

    /// A histogram. Each call makes another cell for the series, which
    /// [`Metrics::poll`] merges with the others: give each thread its own.
    #[must_use]
    pub fn histogram(&self, name: &str, labels: &[(&str, &str)]) -> Histogram {
        let cell = Arc::new(HistogramCell::new());
        self.register(name, MetricKind::Histogram, labels, |_| {
            Source::Histogram(
                Arc::clone(&cell),
                vec![0; bucket_count(PRECISION)].into_boxed_slice(),
                0,
            )
        });
        Histogram {
            cell,
            _one_writer: PhantomData,
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
        if let Some(bucket) = cell.buckets.get(bucket(value, PRECISION)) {
            bucket.fetch_add(1, Relaxed);
        }
        cell.sum.fetch_add(value, Relaxed);
        cell.max.fetch_max(value, Relaxed);
        cell.min.fetch_min(value, Relaxed);
    }

    fn shared_counter(&self, name: &str, labels: &[(&str, &str)]) -> Arc<CounterCell> {
        let mut state = self.state();
        let index = self.def(&mut state, name, MetricKind::Counter, labels);
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
        let index = self.def(&mut state, name, MetricKind::Histogram, labels);
        if let Some(cell) = &state.defs[index].shared_histogram {
            return Arc::clone(cell);
        }
        let cell = Arc::new(HistogramCell::new());
        state.defs[index].shared_histogram = Some(Arc::clone(&cell));
        state.cells.push(Cellref {
            def: index,
            source: Source::Histogram(
                Arc::clone(&cell),
                vec![0; bucket_count(PRECISION)].into_boxed_slice(),
                0,
            ),
        });
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
        let def = self.def(&mut state, name, kind, labels);
        let source = source(def);
        state.cells.push(Cellref { def, source });
    }

    /// The series' index, made the first time.
    fn def(
        &self,
        state: &mut State,
        name: &str,
        kind: MetricKind,
        labels: &[(&str, &str)],
    ) -> usize {
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
            def,
            message,
            last_total: 0,
            gauge: None,
            hist: None,
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
            self.poll_with(now, persist.max_payload(), |len, write| {
                persist.claim(len, write);
            });
        }
    }

    /// [`Metrics::poll`] once the interval is due: publish one message
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
        if !state.active {
            let i = self.registry.interval_ns;
            let end = now.epoch_ns().div_euclid(i) * i;
            state.snapshot(u64::try_from(end).unwrap_or(0));
            self.registry.next_due.store(i64::MIN, Relaxed);
        }
        match state.next(max_payload) {
            Some(next) => {
                let len = state.len(&next);
                publish(len, &mut |buf| {
                    state.encode(&next, self.registry.interval_ns, buf).is_ok()
                });
                state.done(&next);
            }
            None => {
                state.active = false;
                self.registry
                    .next_due
                    .store(self.next_boundary(now).0, Relaxed);
            }
        }
    }

    /// Take the interval now and return every message it makes, encoded:
    /// what [`Metrics::poll`] publishes over successive calls.
    #[cfg(test)]
    fn drain(&self, end_ns: u64, max_payload: usize) -> Vec<Vec<u8>> {
        let mut state = self.state();
        state.snapshot(end_ns);
        let mut out = Vec::new();
        while let Some(next) = state.next(max_payload) {
            let mut buf = vec![0; state.len(&next)];
            assert!(
                state
                    .encode(&next, self.registry.interval_ns, &mut buf)
                    .is_ok()
            );
            state.done(&next);
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
                Source::Histogram(histogram, _, _) => histogram
                    .buckets
                    .iter()
                    .map(|bucket| bucket.load(Relaxed))
                    .sum(),
                _ => 0,
            })
            .sum()
    }
}

impl State {
    /// Read every cell into the interval ending at `ts` (UNIX ns).
    fn snapshot(&mut self, ts: u64) {
        let cycle = &mut self.cycle;
        (cycle.ts, cycle.next_def) = (ts, 0);
        (cycle.next_counter, cycle.next_gauge, cycle.next_histogram) = (0, 0, 0);
        cycle.counters.clear();
        cycle.gauges.clear();
        cycle.histograms.clear();
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
                    .push((def.def.series, f64::from_bits(g.bits.load(Relaxed)))),
                Source::Histogram(h, seen, seen_sum) => {
                    // Swapped first: a value recorded after the swap is in
                    // the next interval's range.
                    let min = h.min.swap(u64::MAX, Relaxed);
                    let max = h.max.swap(0, Relaxed);
                    if min > max {
                        continue; // nothing recorded
                    }
                    let snap = def.hist.get_or_insert_with(Snap::new);
                    if snap.low == usize::MAX {
                        cycle.histograms.push(cell.def);
                    }
                    let (low, high) = (bucket(min, PRECISION), bucket(max, PRECISION));
                    for i in low..=high {
                        let total = h.buckets[i].load(Relaxed);
                        let delta = total.wrapping_sub(seen[i]);
                        seen[i] = total;
                        snap.buckets[i] += delta;
                        snap.count += delta;
                    }
                    let sum = h.sum.load(Relaxed);
                    snap.sum = snap.sum.wrapping_add(sum.wrapping_sub(*seen_sum));
                    *seen_sum = sum;
                    snap.min = snap.min.min(min);
                    snap.max = snap.max.max(max);
                    snap.low = snap.low.min(low);
                    snap.high = snap.high.max(high);
                }
            }
        }
        for (def, total) in self.defs.iter_mut().zip(totals.iter()) {
            let Some(total) = *total else {
                continue;
            };
            let delta = total.wrapping_sub(def.last_total);
            def.last_total = total;
            cycle.counters.push((def.def.series, total, delta));
        }
        self.active = true;
    }

    /// The next message of the interval, `None` once all are out.
    fn next(&mut self, max_payload: usize) -> Option<Next> {
        let defs = self.defs.len();
        if !self.active {
            return None;
        }
        let cycle = &mut self.cycle;
        while cycle.next_def < defs {
            cycle.next_def += 1;
            let def = cycle.next_def - 1;
            if !self.defs[def].message.is_empty() {
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
        while cycle.next_histogram < cycle.histograms.len() {
            let def = cycle.histograms[cycle.next_histogram];
            cycle.next_histogram += 1;
            let Some(snap) = self.defs[def].hist.as_deref() else {
                continue;
            };
            // Coarser buckets until they fit one message.
            for precision in (0..=PRECISION).rev() {
                let buckets = snap.buckets(precision).count();
                if codec::HistogramEncoder::compute_length_with_header(buckets) <= max_payload {
                    return Some(Next::Histogram {
                        def,
                        precision,
                        buckets,
                    });
                }
            }
            log::error!("metrics: a {max_payload}-byte message holds no histogram; dropped");
        }
        None
    }

    fn len(&self, next: &Next) -> usize {
        match next {
            Next::Def(def) => self.defs[*def].message.len(),
            Next::Metrics { counters, gauges } => {
                codec::MetricsEncoder::compute_length_with_header(counters.len(), gauges.len())
            }
            Next::Histogram { buckets, .. } => {
                codec::HistogramEncoder::compute_length_with_header(*buckets)
            }
        }
    }

    /// `next` was published, or could not be: a histogram starts again.
    fn done(&mut self, next: &Next) {
        if let Next::Histogram { def, .. } = *next
            && let Some(snap) = self.defs[def].hist.as_deref_mut()
        {
            snap.clear();
        }
    }

    /// Write `next` into `buf`, exactly [`State::len`] bytes, and move past it.
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
                let len = codec::MetricsEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&codec::MetricsFixedFields { ts, interval })
                    .counters(c.len() as u16, |group| {
                        for &(series, value, delta) in c {
                            group.add(|e| {
                                e.series(series).value(value).delta(delta);
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?
                    .gauges(g.len() as u16, |group| {
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
            &Next::Histogram {
                def,
                precision,
                buckets,
            } => {
                let def = &self.defs[def];
                let Some(snap) = def.hist.as_deref() else {
                    return Ok(());
                };
                let len = codec::HistogramEncoder::wrap_and_apply_header(buf, 0)
                    .fixed(&codec::HistogramFixedFields {
                        ts,
                        interval,
                        series: def.def.series,
                        count: snap.count,
                        sum: snap.sum,
                        min: snap.min,
                        max: snap.max,
                        precision,
                    })
                    .buckets(buckets as u16, |group| {
                        for (index, count) in snap.buckets(precision) {
                            group.add(|e| {
                                e.index(index as u16).count(count);
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
                assert!(
                    (high - low) as f64 <= low as f64 / f64::from(1u32 << precision) || high == low,
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

    /// (series, count, sum, min, max, precision, buckets)
    type Hist = (u64, u64, u64, u64, u64, u8, Vec<(u16, u64)>);

    struct Decoded {
        defs: Vec<MetricDef>,
        counters: Vec<(u64, u64, u64)>,
        gauges: Vec<(u64, f64)>,
        /// (series, count, sum, min, max, precision, buckets)
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
                    d.ts.push(h.ts());
                    let mut buckets = Vec::new();
                    for e in h.buckets()? {
                        buckets.push((e.index(), e.count()));
                    }
                    d.histograms.push((
                        h.series(),
                        h.count(),
                        h.sum(),
                        h.min(),
                        h.max(),
                        h.precision(),
                        buckets,
                    ));
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
    fn histograms_publish_each_interval_exactly_once() -> TestResult {
        let metrics = Metrics::detached();
        let h = metrics.histogram("lat", &[]);
        let other_thread = metrics.histogram("lat", &[]);
        for v in [100, 100, 101, 5_000, 1_000_000] {
            h.record(v);
        }
        other_thread.record(7);
        let d = decode(&metrics.drain(1, 64 * 1024))?;
        let [(_, count, sum, min, max, precision, buckets)] = d.histograms.as_slice() else {
            return Err(format!("{:?}", d.histograms.len()).into());
        };
        assert_eq!(
            (*count, *sum, *min, *max, *precision),
            (6, 1_005_308, 7, 1_000_000, PRECISION)
        );
        assert_eq!(buckets.iter().map(|b| b.1).sum::<u64>(), 6);
        // 100 and 101 share a bucket: at 64..128 each is two values wide.
        assert_eq!(bucket(101, PRECISION), bucket(100, PRECISION));
        assert!(
            buckets.contains(&(bucket(100, PRECISION) as u16, 3)),
            "{buckets:?}"
        );
        assert!(
            buckets.windows(2).all(|w| w[0].0 < w[1].0),
            "in order: {buckets:?}"
        );

        // Nothing new: no histogram message at all.
        assert!(decode(&metrics.drain(2, 64 * 1024))?.histograms.is_empty());

        // Only what came after the last poll.
        h.record(42);
        let d = decode(&metrics.drain(3, 64 * 1024))?;
        assert_eq!(d.histograms.len(), 1);
        assert_eq!(&d.histograms[0].1..=&d.histograms[0].4, &1..=&42);
        assert_eq!(d.histograms[0].6, [(bucket(42, PRECISION) as u16, 1)]);
        Ok(())
    }

    #[test]
    fn what_does_not_fit_one_message_is_split_or_coarsened() -> TestResult {
        let metrics = Metrics::detached();
        let counters: Vec<Counter> = (0..100)
            .map(|i| metrics.counter("c", &[("i", &i.to_string())]))
            .collect();
        for (i, c) in counters.iter().enumerate() {
            c.add(i as u64);
        }
        let h = metrics.histogram("wide", &[]);
        for shift in 0..40 {
            for k in 0..32 {
                h.record((1u64 << shift) + k * ((1u64 << shift) / 32));
            }
        }
        let messages = metrics.drain(1, 1024);
        assert!(messages.iter().all(|m| m.len() <= 1024));
        let d = decode(&messages)?;
        assert_eq!(
            d.counters.len(),
            100,
            "every counter, over several messages"
        );
        let [(_, count, .., precision, buckets)] = d.histograms.as_slice() else {
            return Err("one histogram".into());
        };
        assert!(*precision < PRECISION, "coarsened to fit");
        assert_eq!(
            buckets.iter().map(|b| b.1).sum::<u64>(),
            *count,
            "no count lost"
        );
        assert_eq!(*count, 40 * 32);
        Ok(())
    }

    #[test]
    fn a_due_poll_publishes_one_message_per_call() -> TestResult {
        let metrics = Metrics::detached();
        metrics.start();
        let c = metrics.counter("c", &[]);
        let h = metrics.histogram("h", &[]);
        c.inc();
        h.record(5);
        let due = metrics.registry.next_due.load(Relaxed);
        let mut published = Vec::new();
        let mut poll = |at: i64| {
            let mut sent = 0;
            // `poll`, with a publisher that keeps what it is given.
            if at < metrics.registry.next_due.load(Relaxed) {
                return sent;
            }
            metrics.poll_with(Nanos(at), 64 * 1024, |len, write| {
                let mut buf = vec![0; len];
                assert!(write(&mut buf));
                published.push(buf);
                sent += 1;
            });
            sent
        };
        assert_eq!(poll(due - 1), 0, "not yet due");
        // Two defs, one Metrics, one Histogram: one per call, then done.
        let sent: Vec<i32> = (0..6).map(|_| poll(due)).collect();
        assert_eq!(sent, [1, 1, 1, 1, 0, 0]);
        assert!(
            metrics.registry.next_due.load(Relaxed) > due,
            "the next interval"
        );
        let d = decode(&published)?;
        assert_eq!(
            (d.defs.len(), d.counters.len(), d.histograms.len()),
            (2, 1, 1)
        );
        let end = u64::try_from(Nanos(due).epoch_ns())?;
        assert!(d.ts.iter().all(|&ts| ts == end), "{:?} != {end}", d.ts);
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
