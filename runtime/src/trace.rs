//! Checkpoint traces for hot paths: timestamps at fixed stages of one event,
//! stamped into a record on the stack.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ergon_runtime::clock::{Clock, Nanos};
//! use ergon_runtime::trace::TraceId;
//! # let settings = ergon_runtime::Settings::from_env();
//! # let bus = ergon_runtime::bus::Bus::connect(&settings)?;
//! # let persist = ergon_runtime::persist::Persist::connect("", &bus, settings)?;
//! # let (ts_event, seq, levels) = (0, 7, 10);
//! let clock = Clock::new();
//! // Once: the trace, its stages and its numeric attributes.
//! let t2t = persist.tracer("tick_to_trade", &["wire", "decode", "decide", "send"], &["levels"]);
//! const MD: u64 = TraceId::namespace("md");
//!
//! // Stamps stay on the stack; finish locks each stage's histogram cell.
//! let mut t = t2t.start(Nanos::from_epoch(ts_event), TraceId::new(MD, seq));
//! t.mark(clock.now()); // wire: the venue's time to ours
//! t.mark(clock.now()); // decode
//! t.mark(clock.now()); // decide
//! t.attr(0, levels);
//! t.mark(clock.now()); // send
//! t.finish();
//! # Ok(()) }
//! ```
//!
//! The same waterfall from code that is not on the hot path is a `tracing`
//! span (`info_span!`, `#[instrument]`). [`crate::persist::Persist::layer`] records
//! those while `otel_traces` is on. Spans use the tracing registry and allocate;
//! checkpoint stamps stay on the stack. Finishing a checkpoint trace locks
//! its histogram cells to preserve complete summaries during concurrent polls.
//!
//! [`Trace::finish`] always records each stage, and the whole, into the
//! histogram `trace_ns{trace, stage}`: every event is counted. It publishes
//! the trace itself only when `otel_traces` is on for this app in
//! `tables.yaml` and the trace is one in `sample`, or slower than
//! `slower_than`:
//!
//! ```yaml
//! otel_traces:
//!   kind: static
//!   enabled: false
//!   apps: { binance: { until: 2026-09-27T18:00:00Z } }
//!   traces:
//!     tick_to_trade: { sample: 1000, slower_than: 50us }
//! ```
//!
//! The ingester turns each into spans of `otel_traces`, which Grafana's
//! trace view shows: one for the whole, one per stage. A business id as the
//! [`TraceId`] (an order id, say) puts every application's trace of that
//! order into one trace, with nothing passed between them.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};

use crate::clock::Nanos;
use crate::event::codec;
use crate::metrics::{Counter, Histogram, Metrics};
use crate::persist::Persist;

/// Template id of the `TraceDef` message.
pub const TRACE_DEF_TEMPLATE_ID: u16 = codec::TraceDefEncoder::TEMPLATE_ID;
/// Template id of the `Trace` message.
pub const TRACE_TEMPLATE_ID: u16 = codec::TraceEncoder::TEMPLATE_ID;

/// Stages a trace can have.
pub const MAX_STAGES: usize = 16;
/// Numeric attributes a trace can have.
pub const MAX_ATTRS: usize = 8;

/// A trace's id, as OpenTelemetry's 128 bits: a namespace and an id in it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TraceId {
    /// The namespace half, usually [`TraceId::namespace`].
    pub hi: u64,
    /// The id within that namespace.
    pub lo: u64,
}

impl TraceId {
    /// `namespace` in the high half and `id` in the low half.
    #[must_use]
    pub const fn new(namespace: u64, id: u64) -> Self {
        Self {
            hi: namespace,
            lo: id,
        }
    }

    /// A namespace's half of an id: FNV-1a of its name. `const`, so the
    /// hot path never hashes: `const ORDERS: u64 = TraceId::namespace("order");`.
    #[must_use]
    pub const fn namespace(name: &str) -> u64 {
        let bytes = name.as_bytes();
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut i = 0;
        while i < bytes.len() {
            hash = (hash ^ bytes[i] as u64).wrapping_mul(0x0100_0000_01b3);
            i += 1;
        }
        hash
    }

    /// `id` in `namespace`, hashing the name now.
    #[must_use]
    pub const fn of(namespace: &str, id: u64) -> Self {
        Self::new(Self::namespace(namespace), id)
    }
}

/// A trace's name, stages and attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceDef {
    /// Hash of the name, stages, and attributes.
    pub def: u64,
    /// The trace's name.
    pub name: String,
    /// Stage names, in order.
    pub stages: Vec<String>,
    /// Numeric attribute names, in order.
    pub attrs: Vec<String>,
}

impl TraceDef {
    /// A trace named `name` with those stages and numeric attributes.
    #[must_use]
    pub fn new(name: &str, stages: &[&str], attrs: &[&str]) -> Self {
        let mut key = Vec::new();
        for (tag, part) in std::iter::once((0, name))
            .chain(stages.iter().map(|s| (1, *s)))
            .chain(attrs.iter().map(|a| (2, *a)))
        {
            key.push(tag);
            key.extend_from_slice(part.as_bytes());
            key.push(0);
        }
        Self {
            def: crate::event::fnv64(&key),
            name: name.to_owned(),
            stages: stages.iter().map(|s| (*s).to_owned()).collect(),
            attrs: attrs.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    /// This trace as a `TraceDef` message, header included.
    ///
    /// # Errors
    ///
    /// More stages or attributes than a group header can count, or a name
    /// the codec rejects.
    pub fn message(&self) -> Result<Vec<u8>, crate::event::EncodeError> {
        let stages = crate::event::group_count(self.stages.len())?;
        let attrs = crate::event::group_count(self.attrs.len())?;
        let len = codec::TraceDefEncodedLength::new()
            .stages_ragged(stages, |g| {
                for s in &self.stages {
                    g.add()?.name(s.len())?;
                }
                Ok(())
            })?
            .attrs_ragged(attrs, |g| {
                for a in &self.attrs {
                    g.add()?.name(a.len())?;
                }
                Ok(())
            })?
            .name(self.name.len())?
            .encoded_length_with_header();
        crate::event::owned_frame(len, |message| {
            Ok(codec::TraceDefEncoder::wrap_and_apply_header(message, 0)
                .fixed(&codec::TraceDefFixedFields { def: self.def })
                .stages(stages, |g| {
                    for s in &self.stages {
                        g.add(|e| e.name(s.as_bytes()))?;
                    }
                    Ok(())
                })?
                .attrs(attrs, |g| {
                    for a in &self.attrs {
                        g.add(|e| e.name(a.as_bytes()))?;
                    }
                    Ok(())
                })?
                .name(self.name.as_bytes())?
                .encoded_length_with_header())
        })
    }

    /// A trace from its `TraceDef` message; `None` when malformed.
    #[must_use]
    pub fn decode(message: &[u8]) -> Option<Self> {
        let d = codec::TraceDefDecoder::decode(message, 0).ok()?;
        let stages = d
            .stages()
            .ok()?
            .map(|e| Some(e.ok()?.name_as_str().ok()?.to_owned()))
            .collect::<Option<Vec<_>>>()?;
        let attrs = d
            .attrs()
            .ok()?
            .map(|e| Some(e.ok()?.name_as_str().ok()?.to_owned()))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            def: d.def(),
            name: d.name_as_str().ok()?.to_owned(),
            stages,
            attrs,
        })
    }
}

/// A trace definition's message, and whether the stream has it yet.
pub(crate) struct DefMessage {
    pub(crate) message: Vec<u8>,
    pub(crate) sent: AtomicBool,
}

/// What `tables.yaml` says about one trace for this app; written by the
/// config watcher, read by [`Tracer`] with relaxed loads.
pub(crate) struct TraceSwitch {
    pub(crate) on: AtomicBool,
    /// Publish one in this many; 0 none.
    pub(crate) sample: AtomicU64,
    /// Publish every one slower than this (ns); 0 none.
    pub(crate) slower_than: AtomicI64,
}

impl TraceSwitch {
    /// On or off, and `config`'s sampling (every one when not listed).
    pub(crate) fn set(&self, on: bool, config: Option<&crate::persist::TraceConfig>) {
        self.on.store(on, Relaxed);
        self.sample.store(config.map_or(1, |c| c.sample), Relaxed);
        self.slower_than.store(
            config.and_then(|c| c.slower_than).map_or(0, |s| s.0),
            Relaxed,
        );
    }
}

impl Default for TraceSwitch {
    fn default() -> Self {
        Self {
            on: AtomicBool::new(false),
            sample: AtomicU64::new(1),
            slower_than: AtomicI64::new(0),
        }
    }
}

/// One trace's handle: make it once, then [`Tracer::start`] per event.
/// One writer at a time: `Send`, not `Sync`.
pub struct Tracer {
    def: Arc<DefMessage>,
    def_id: u64,
    stages: usize,
    attrs: usize,
    switch: Arc<TraceSwitch>,
    /// Starts left until the next head-sampled one.
    left: Cell<u64>,
    /// `trace_ns{trace, stage}` per stage, then the whole.
    histograms: Box<[Histogram]>,
    /// `trace_clamped{trace, stage}`: stages that ended before they began.
    clamped: Box<[Counter]>,
    persist: Option<Persist>,
    /// For [`Tracer::next_id`].
    id_hi: u64,
    ids: Cell<u64>,
}

impl Tracer {
    pub(crate) fn new(
        def: &TraceDef,
        message: Arc<DefMessage>,
        switch: Arc<TraceSwitch>,
        metrics: &Metrics,
        persist: Option<Persist>,
        id_hi: u64,
    ) -> Self {
        let stages = def.stages.len().min(MAX_STAGES);
        let names = def.stages[..stages]
            .iter()
            .map(String::as_str)
            .chain(["total"]);
        let histograms = names
            .clone()
            .map(|stage| metrics.histogram("trace_ns", &[("trace", &def.name), ("stage", stage)]))
            .collect();
        let clamped = names
            .map(|stage| {
                metrics.counter("trace_clamped", &[("trace", &def.name), ("stage", stage)])
            })
            .collect();
        Self {
            def: message,
            def_id: def.def,
            stages,
            attrs: def.attrs.len().min(MAX_ATTRS),
            switch,
            left: Cell::new(1),
            histograms,
            clamped,
            persist,
            id_hi,
            ids: Cell::new(0),
        }
    }

    /// Begin one trace at `at`: a countdown for sampling, no clock read.
    #[inline]
    pub fn start(&self, at: Nanos, id: TraceId) -> Trace<'_> {
        let left = self.left.get();
        let sampled = if left > 1 {
            self.left.set(left - 1);
            false
        } else {
            // One in `sample`, from this one; `sample: 0` is none.
            let sample = self.switch.sample.load(Relaxed);
            self.left.set(sample);
            sample != 0
        };
        Trace {
            tracer: self,
            id,
            start: at,
            marks: [0; MAX_STAGES],
            n: 0,
            attrs: [0; MAX_ATTRS],
            sampled,
            kept: false,
        }
    }

    /// Is `otel_traces` on for this app? Traces are published only then;
    /// their stage histograms count either way.
    #[must_use]
    pub fn is_on(&self) -> bool {
        self.switch.on.load(Relaxed)
    }

    /// A trace id no other trace of this process has: for traces with no
    /// business id.
    #[inline]
    pub fn next_id(&self) -> TraceId {
        let lo = self.ids.get().wrapping_add(1);
        self.ids.set(lo);
        TraceId::new(self.id_hi, lo)
    }

    #[inline]
    fn record(&self, stage: usize, ns: i64) {
        if ns < 0 {
            self.clamped[stage].inc();
        }
        self.histograms[stage].record(ns.max(0).cast_unsigned());
    }

    #[cold]
    fn publish(&self, t: &Trace<'_>, why: codec::TraceWhy) {
        let Some(persist) = &self.persist else {
            return;
        };
        if !self.def.sent.load(Relaxed) {
            if !persist.publish(&self.def.message) {
                return; // the trace would arrive without its def
            }
            self.def.sent.store(true, Relaxed);
        }
        let marks = &t.marks[..usize::from(t.n)];
        let attrs = &t.attrs[..self.attrs];
        let (Ok(n_marks), Ok(n_attrs)) = (
            crate::event::group_count(marks.len()),
            crate::event::group_count(attrs.len()),
        ) else {
            return;
        };
        let Ok(len) = codec::TraceEncodedLength::new()
            .marks(n_marks)
            .and_then(|l| {
                l.attrs_ragged(n_attrs, |g| {
                    for _ in attrs {
                        g.add()?.text(0)?;
                    }
                    Ok(())
                })
            })
            .map(|l| l.encoded_length_with_header())
        else {
            return;
        };
        persist.claim(len, |buf| {
            codec::TraceEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&codec::TraceFixedFields {
                    def: self.def_id,
                    trace_hi: t.id.hi,
                    trace_lo: t.id.lo,
                    span: 0,
                    parent: 0,
                    start: t.start.epoch_ns().cast_unsigned(),
                    why,
                })
                .marks(n_marks, |g| {
                    for &ns in marks {
                        g.add(|e| {
                            e.ns(ns);
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
                .and_then(|m| {
                    m.attrs(n_attrs, |g| {
                        for &v in attrs {
                            g.add(|mut e| {
                                e.kind(crate::event::Kind::I64 as u8)
                                    .bits(v.cast_unsigned());
                                e.text(b"")
                            })?;
                        }
                        Ok(())
                    })
                })
                .is_ok()
        });
    }
}

impl std::fmt::Debug for Tracer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tracer")
            .field("def", &self.def_id)
            .finish_non_exhaustive()
    }
}

/// One trace in flight, on the stack. [`Trace::finish`] records it;
/// dropping it unfinished records nothing.
pub struct Trace<'t> {
    tracer: &'t Tracer,
    id: TraceId,
    start: Nanos,
    /// Each stage's end, as ns after `start`.
    marks: [i64; MAX_STAGES],
    n: u8,
    attrs: [i64; MAX_ATTRS],
    sampled: bool,
    kept: bool,
}

impl Trace<'_> {
    /// The next stage ended at `at`. Marks past the last stage are ignored.
    #[inline]
    pub fn mark(&mut self, at: Nanos) {
        let n = usize::from(self.n);
        if n < self.tracer.stages {
            self.marks[n] = at.since(self.start);
            self.n += 1;
        }
    }

    /// Set numeric attribute `i` (in the order the tracer named them).
    #[inline]
    pub fn attr(&mut self, i: usize, value: i64) {
        if let Some(a) = self.attrs.get_mut(i) {
            *a = value;
        }
    }

    /// Publish this one whatever the sampling (while `otel_traces` is on):
    /// a tick that ended in an order, say.
    #[inline]
    pub const fn keep(&mut self) {
        self.kept = true;
    }

    /// Its id, when the business id is known only part way: the order id a
    /// tick led to, so the order's traces in other applications join it.
    #[inline]
    pub const fn set_id(&mut self, id: TraceId) {
        self.id = id;
    }

    /// Record each stage and the whole into the histograms, and publish the
    /// trace if it is kept, sampled or slow and `otel_traces` is on.
    #[inline]
    pub fn finish(self) {
        let t = self.tracer;
        let mut prev = 0;
        for (stage, &end) in self.marks[..usize::from(self.n)].iter().enumerate() {
            t.record(stage, end - prev);
            prev = end;
        }
        t.record(t.stages, prev);
        if !t.switch.on.load(Relaxed) {
            return;
        }
        if let Some(why) = why(
            self.kept,
            self.sampled,
            t.switch.slower_than.load(Relaxed),
            prev,
        ) {
            t.publish(&self, why);
        }
    }
}

/// Why a finished trace that took `total` ns is published, if it is.
#[inline]
const fn why(kept: bool, sampled: bool, slower_than: i64, total: i64) -> Option<codec::TraceWhy> {
    if kept {
        Some(codec::TraceWhy::Kept)
    } else if sampled {
        Some(codec::TraceWhy::Sampled)
    } else if slower_than > 0 && total >= slower_than {
        Some(codec::TraceWhy::Slow)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn tracer(switch: &Arc<TraceSwitch>) -> (Tracer, Metrics) {
        let metrics = Metrics::detached();
        let def = TraceDef::new("t2t", &["wire", "decode"], &["levels"]);
        let message = Arc::new(DefMessage {
            message: Vec::new(),
            sent: AtomicBool::new(true),
        });
        let tracer = Tracer::new(&def, message, Arc::clone(switch), &metrics, None, 1);
        (tracer, metrics)
    }

    #[test]
    fn head_sampling_takes_exactly_one_in_n() {
        let switch = Arc::new(TraceSwitch::default());
        switch.sample.store(4, Relaxed);
        let (t, _) = tracer(&switch);
        let sampled: Vec<bool> = (0..12)
            .map(|_| t.start(Nanos(0), TraceId::default()).sampled)
            .collect();
        // The first, then one in four.
        assert_eq!(
            sampled,
            [
                true, false, false, false, true, false, false, false, true, false, false, false
            ]
        );
        switch.sample.store(0, Relaxed);
        let (t, _) = tracer(&switch);
        assert!(
            (0..100).all(|_| !t.start(Nanos(0), TraceId::default()).sampled),
            "sample: 0 is none, not even the first"
        );
        // A change takes effect when the countdown next runs out.
        switch.sample.store(1, Relaxed);
        assert!(t.start(Nanos(0), TraceId::default()).sampled);
        assert!(
            t.start(Nanos(0), TraceId::default()).sampled,
            "1: every one"
        );
    }

    #[test]
    fn stages_are_counted_whether_or_not_the_trace_is_kept() {
        let switch = Arc::new(TraceSwitch::default()); // off: nothing published
        let (t, _) = tracer(&switch);
        for _ in 0..3 {
            let mut tr = t.start(Nanos(100), TraceId::default());
            tr.mark(Nanos(90)); // before the start: the venue's clock is ahead
            tr.mark(Nanos(250));
            tr.mark(Nanos(999)); // past the last stage: ignored
            tr.finish();
        }
        // Negative stage: clamped to 0, and counted.
        assert_eq!(t.clamped[0].get(), 3);
        assert_eq!(t.clamped[1].get(), 0);
        let counts: Vec<u64> = t
            .histograms
            .iter()
            .map(super::super::metrics::Histogram::cell_count)
            .collect();
        assert_eq!(counts, [3, 3, 3], "wire, decode, total");
    }

    #[test]
    fn kept_beats_sampling_and_speed() {
        use codec::TraceWhy::{Kept, Sampled, Slow};
        // (kept, sampled, slower_than, total)
        assert_eq!(why(true, false, 0, 1), Some(Kept));
        assert_eq!(why(true, true, 10, 99), Some(Kept));
        assert_eq!(why(false, true, 10, 99), Some(Sampled));
        assert_eq!(why(false, false, 10, 10), Some(Slow));
        assert_eq!(why(false, false, 10, 9), None);
        assert_eq!(why(false, false, 0, i64::MAX), None, "no threshold");
    }

    #[test]
    fn ids_are_namespaced_and_const() {
        const ORDERS: u64 = TraceId::namespace("order");
        assert_eq!(TraceId::new(ORDERS, 42), TraceId::of("order", 42));
        assert_ne!(TraceId::of("order", 42), TraceId::of("md", 42));
        assert_eq!(ORDERS, crate::event::fnv64(b"order"));
    }

    #[test]
    fn a_def_round_trips_through_its_message() -> TestResult {
        let def = TraceDef::new("t2t", &["wire", "decode"], &["levels", "side"]);
        assert_eq!(TraceDef::decode(&def.message()?).ok_or("undecodable")?, def);
        assert_ne!(
            def.def,
            TraceDef::new("t2t", &["wire"], &["decode", "levels", "side"]).def
        );
        Ok(())
    }
}
