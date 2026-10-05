//! `tracing` from any thread into the ingester's stream: an event that names
//! a `table` becomes a row of it, and a span at INFO or above becomes a span
//! of `otel_traces`.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use tracing_subscriber::layer::SubscriberExt;
//! # let settings = ergon_runtime::Settings::from_env();
//! # let bus = ergon_runtime::bus::Bus::connect(&settings)?;
//! # let persist = ergon_runtime::persist::Persist::connect("", &bus, settings)?;
//! // Once, on the loop thread at start-up.
//! let subscriber = tracing_subscriber::registry().with(persist.layer()?);
//! tracing::subscriber::set_global_default(subscriber)?;
//! // Then from any thread.
//! tracing::info!(table = "signal", instrument = "BTCUSDT", edge = 0.25);
//! tracing::info_span!("connect", venue = "binance").in_scope(|| {});
//! # Ok(()) }
//! ```
//!
//! [`Persist::layer`](crate::persist::Persist::layer) makes the bridge: one
//! concurrent Aeron publication on [`BRIDGE_CHANNEL`] and the application's
//! persist stream, and what never changes: the source id stamped into every
//! frame's reserved value, by which the ingester attributes the frame to this
//! application, and the `Source` message. Aeron's concurrent publication is
//! thread safe, so the layer is `Send` and `Sync` with nothing shared of
//! ours: each thread claims and writes its own frames in place.
//!
//! What a thread has sent is its own. Before its first frame it sends the
//! `Source` message, and a shape or span definition before the first row or
//! span that needs it (or the next, when Aeron could not take it); 5 s
//! later, again. Its span ids are its own too: a prefix of the thread's,
//! then a count. Rows and spans are stamped with the bridge's clock, paired
//! with the wall clock when it was made. A row's columns are its fields, as
//! with [`Persist::record_row`](crate::persist::Persist::record_row).
//!
//! The bridge applies no table switch. Every row and span goes out, and the
//! ingester keeps it only while `tables.yaml` has its table (`otel_traces`
//! for a span) on for this application at its time. A frame Aeron cannot
//! take is dropped and not counted. No loop path writes to the bridge: it
//! costs a `tracing` callback, a thread-local lookup and a claim, and a span
//! also the `tracing` registry and an allocation. What the loop records goes
//! through [`Persist`](crate::persist::Persist).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::hash::{BuildHasherDefault, Hash, Hasher as _};

use rusteron_archive::{Aeron, AeronBufferClaim, AeronPublication, IntoCString};
use tracing::field::{Field, FieldSet, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use crate::Error;
use crate::bus::{Claim, await_added, retry_admin};
use crate::clock::{Anchor, Nanos};
use crate::event::{AddressHasher, FieldDef, Kind, NONE, Shape, Value, codec};
use crate::trace::{TraceDef, TraceId};

/// The bridge's channel: IPC through the shared media driver, in small terms.
///
/// The application, the archive and the ingester share the driver.
/// Concurrent IPC publications of one stream share one session on it, and it
/// refuses another term length or MTU there: every bridge uses exactly this
/// string.
pub const BRIDGE_CHANNEL: &str = "aeron:ipc?term-length=1m|mtu=65504";

/// The field that names an event's table.
pub const TABLE: &str = "table";

/// How long what a thread has sent stays sent: then the `Source` message,
/// the shapes and the span definitions go out again, before the next frame
/// that needs them.
const ROUND_NS: i64 = 5_000_000_000;

/// The layer: one concurrent publication, and what never changes.
pub(crate) struct Bridge {
    publication: AeronPublication,
    /// Stamped into every frame's reserved value: who recorded it.
    source: u64,
    /// The application's `Source` message.
    source_message: Vec<u8>,
    /// The persist stream. With the source id, it names this bridge to the
    /// threads' state.
    stream: i32,
    /// The longest frame one claim takes.
    max_payload: usize,
    /// Stamps rows and spans; read by every thread, never cached.
    clock: Anchor,
}

impl Bridge {
    /// Add the publication on `stream` through `aeron`, driving the
    /// conductor until it is added (see
    /// [`Bus::poll`](crate::bus::Bus::poll)), for the application `source`
    /// with its `Source` message.
    ///
    /// The layer holds its only handle, and the runtime does not close it at
    /// shutdown: other threads may still be claiming, and the client frees a
    /// publication once its close is processed. It closes when the layer is
    /// dropped, when no thread can reach it, or with the client.
    pub(crate) fn new(
        aeron: &Aeron,
        source: u64,
        source_message: &[u8],
        stream: i32,
    ) -> Result<Self, Error> {
        let adding = aeron
            .async_add_publication(&BRIDGE_CHANNEL.into_c_string(), stream)
            .map_err(|e| Error::Aeron(format!("{BRIDGE_CHANNEL}: {e}")))?;
        let publication = await_added(aeron, BRIDGE_CHANNEL, || adding.poll())?;
        let max_payload = publication
            .max_payload_length()
            .map_err(|e| Error::Aeron(e.to_string()))?;
        Ok(Self {
            publication,
            source,
            source_message: source_message.to_vec(),
            stream,
            max_payload,
            clock: Anchor::new(),
        })
    }

    /// Claim exactly `len` bytes, stamped with the source id, let `write`
    /// fill them, and commit when it returns `true`. `false` when nothing
    /// was published.
    fn claim(&self, len: usize, write: impl FnOnce(&mut [u8]) -> bool) -> bool {
        if len > self.max_payload {
            return false;
        }
        let claim = AeronBufferClaim::new_zeroed_on_stack();
        if retry_admin(|| self.publication.try_claim(len, &claim)).is_err() {
            return false;
        }
        claim.frame_header_mut().reserved_value = self.source.cast_signed();
        let claim = Claim::new(claim);
        write(claim.data()) && claim.commit().is_ok()
    }

    /// Publish a message that is already built.
    fn publish(&self, message: &[u8]) -> bool {
        self.claim(message.len(), |buf| {
            buf.copy_from_slice(message);
            true
        })
    }

    /// This thread's round of this bridge, its `Source` message sent:
    /// `None` when Aeron could not take that, and nothing may follow yet.
    fn round<'r>(
        &self,
        rounds: &'r mut HashMap<(u64, i32), Round>,
        now: Nanos,
    ) -> Option<&'r mut Round> {
        let round = rounds
            .entry((self.source, self.stream))
            .or_insert_with(|| Round::new(now));
        if now.since(round.since) >= ROUND_NS {
            round.restart(now);
        }
        if !round.source {
            if !self.publish(&self.source_message) {
                return None;
            }
            round.source = true;
        }
        Some(round)
    }

    /// Collect `event` into `local`'s scratch, and record it as a row of the
    /// table it names.
    fn record_event(&self, local: &mut Local, event: &Event<'_>) {
        let meta = event.metadata();
        let Local {
            rounds,
            sites,
            scratch,
            ..
        } = local;
        let site = sites.entry(std::ptr::from_ref(meta) as usize).or_default();
        scratch.values.clear();
        scratch.values.resize(meta.fields().len(), None);
        scratch.text.clear();
        let mut collect = Collect {
            site,
            scratch,
            named: false,
        };
        event.record(&mut collect);
        if !collect.named {
            return;
        }
        let fits = site.shape.as_ref().is_some_and(|shape| {
            scratch.values.iter().zip(&site.slot).all(|(value, &slot)| {
                value
                    .as_ref()
                    .is_none_or(|v| slot != NONE && shape.fields[slot].kind == v.kind())
            })
        });
        if !fits && !reshape(site, scratch, meta.fields()) {
            return;
        }
        let Some(shape) = &site.shape else {
            return;
        };
        let now = self.clock.read();
        let Some(round) = self.round(rounds, now) else {
            return;
        };
        // A row must not reach the stream ahead of its shape.
        let message = shape.message();
        if !send_once(&mut round.shapes, shape.id, || self.publish(message)) {
            return;
        }
        let text = site
            .src
            .iter()
            .map(|&i| match scratch.values[i] {
                Some(Held::Str(start, end)) => end - start,
                _ => 0,
            })
            .sum();
        let ts = now.0.cast_unsigned();
        self.claim(shape.row_len(text), |buf| {
            shape.write_row(buf, ts, |i| scratch.value(site.src[i]));
            true
        });
    }

    /// Publish a closed span from `meta`'s call site, its definition first.
    fn publish_span(
        &self,
        local: &mut Local,
        meta: &'static Metadata<'static>,
        data: &SpanData,
        now: Nanos,
    ) {
        let Local { rounds, defs, .. } = local;
        let Some((def, message)) = defs
            .entry(std::ptr::from_ref(meta) as usize)
            .or_insert_with(|| span_def(meta))
            .as_ref()
        else {
            return;
        };
        let Some(round) = self.round(rounds, now) else {
            return;
        };
        // A span must not reach the stream ahead of its definition.
        if !send_once(&mut round.defs, *def, || self.publish(message)) {
            return;
        }
        let duration = now.since(data.start);
        let attrs: Vec<(u8, u64, &str)> = data
            .values
            .iter()
            .map(|v| v.as_ref().map_or((255, 0, ""), Attr::wire))
            .collect();
        let Ok(n_attrs) = crate::event::group_count(attrs.len()) else {
            return;
        };
        let Ok(len) = codec::TraceEncodedLength::new()
            .marks(1)
            .and_then(|l| {
                l.attrs_ragged(n_attrs, |g| {
                    for (_, _, text) in &attrs {
                        g.add()?.text(text.len())?;
                    }
                    Ok(())
                })
            })
            .map(|l| l.encoded_length_with_header())
        else {
            return;
        };
        self.claim(len, |buf| {
            codec::TraceEncoder::wrap_and_apply_header(buf, 0)
                .fixed(&codec::TraceFixedFields {
                    def: *def,
                    trace_hi: data.trace.hi,
                    trace_lo: data.trace.lo,
                    span: data.span,
                    parent: data.parent,
                    start: data.start.epoch_ns().cast_unsigned(),
                    why: codec::TraceWhy::Span,
                })
                .marks(1, |g| {
                    g.add(|e| {
                        e.ns(duration);
                        Ok(())
                    })
                })
                .and_then(|m| {
                    m.attrs(n_attrs, |g| {
                        for &(kind, bits, text) in &attrs {
                            g.add(|mut e| {
                                e.kind(kind).bits(bits);
                                e.text(text.as_bytes())
                            })?;
                        }
                        Ok(())
                    })
                })
                .is_ok()
        });
    }
}

/// What the bridge takes, decided once per call site: an event with a
/// [`TABLE`] field, or a span at INFO and above (below INFO are libraries'
/// internals: h2, hyper, tokio).
pub(crate) fn wants(meta: &Metadata<'_>) -> bool {
    if meta.is_span() {
        *meta.level() <= tracing::Level::INFO
    } else {
        meta.fields().field(TABLE).is_some()
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Bridge {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let (Some(span), Some(n)) = (ctx.span(id), next_span()) else {
            return;
        };
        let parent = span.parent().and_then(|p| {
            let ext = p.extensions();
            ext.get::<SpanData>().map(|d| (d.trace, d.span))
        });
        // Not `Tracer::next_id`'s namespace (`rotate_left(17) ^ nonce`).
        let (trace, parent) =
            parent.unwrap_or_else(|| (TraceId::new(self.source.rotate_left(41), n), 0));
        let mut values = Vec::new();
        values.resize_with(attrs.metadata().fields().len(), || None);
        attrs.record(&mut Attrs(&mut values));
        span.extensions_mut().insert(SpanData {
            trace,
            span: self.source.rotate_left(29) ^ n,
            parent,
            start: self.clock.read(),
            values,
        });
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(data) = span.extensions_mut().get_mut::<SpanData>()
        {
            values.record(&mut Attrs(&mut data.values));
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else {
            return;
        };
        let Some(data) = span.extensions_mut().remove::<SpanData>() else {
            return;
        };
        let now = self.clock.read();
        let meta = span.metadata();
        // A span that closes while this thread is recording is not recorded.
        let _ = LOCAL.try_with(|local| {
            if let Ok(mut local) = local.try_borrow_mut() {
                self.publish_span(&mut local, meta, &data, now);
            }
        });
    }

    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        // An event recorded while recording one (a log line from inside
        // Aeron, say) finds this thread's state borrowed, and is not recorded.
        let _ = LOCAL.try_with(|local| {
            if let Ok(mut local) = local.try_borrow_mut() {
                self.record_event(&mut local, event);
            }
        });
    }
}

type ByAddress<V> = HashMap<usize, V, BuildHasherDefault<AddressHasher>>;

thread_local! {
    /// What this thread has sent and learnt.
    static LOCAL: RefCell<Local> = RefCell::default();
    /// This thread's last span id: its prefix in the high half, a count below.
    static SPAN_IDS: Cell<u64> = Cell::new(thread_prefix());
}

/// One thread's state, for every bridge it records through.
#[derive(Default)]
struct Local {
    /// What it has sent each bridge, by source id and stream.
    rounds: HashMap<(u64, i32), Round>,
    /// Each table event's call site, by its metadata's address.
    sites: ByAddress<Site>,
    /// Each span call site's definition: its id and message, `None` when it
    /// cannot be encoded.
    defs: ByAddress<Option<(u64, Vec<u8>)>>,
    /// The event being recorded, reused so recording does not allocate.
    scratch: Scratch,
}

/// What a thread has sent one bridge since `since`.
struct Round {
    since: Nanos,
    source: bool,
    shapes: HashSet<u32>,
    defs: HashSet<u64>,
}

impl Round {
    fn new(now: Nanos) -> Self {
        Self {
            since: now,
            source: false,
            shapes: HashSet::new(),
            defs: HashSet::new(),
        }
    }

    /// Everything is to be sent again.
    fn restart(&mut self, now: Nanos) {
        self.since = now;
        self.source = false;
        self.shapes.clear();
        self.defs.clear();
    }
}

/// Publish the shape or span definition `id` unless `sent` has it, and mark
/// it sent once Aeron took it: one it could not take goes with the next
/// frame that needs it. `false` when it is not out, and nothing that needs
/// it may follow.
fn send_once<T: Hash + Eq>(sent: &mut HashSet<T>, id: T, publish: impl FnOnce() -> bool) -> bool {
    if sent.contains(&id) {
        return true;
    }
    if !publish() {
        return false;
    }
    sent.insert(id);
    true
}

/// This thread's span-id prefix: 32 bits of a hash of its id, in the high
/// half. A thread id is never reused within a process, so two threads' ids
/// meet only when those 32 bits collide.
fn thread_prefix() -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    hasher.finish() << 32
}

/// A span id no other thread of this process makes.
fn next_span() -> Option<u64> {
    SPAN_IDS
        .try_with(|ids| {
            let n = ids.get().wrapping_add(1);
            ids.set(n);
            n
        })
        .ok()
}

/// What one table event's call site needs.
#[derive(Default)]
struct Site {
    /// The table this call site last named.
    table: String,
    shape: Option<Shape>,
    /// Shape field `i` is the call site's field `src[i]`.
    src: Vec<usize>,
    /// The call site's field `i` is shape field `slot[i]`, or [`NONE`].
    slot: Vec<usize>,
}

/// Give `site` a shape that fits this event: its fields' kinds now, and for
/// a field absent now, the kind it had before. `false`, logged, when the
/// shape cannot be made.
#[cold]
fn reshape(site: &mut Site, scratch: &Scratch, names: &FieldSet) -> bool {
    let old = site.shape.take();
    let mut fields = Vec::new();
    let mut src = Vec::new();
    for (i, field) in names.iter().enumerate() {
        if field.name() == TABLE {
            continue;
        }
        let before = site.slot.get(i).copied().filter(|&s| s != NONE);
        let kind = scratch.values[i]
            .map(Held::kind)
            .or_else(|| Some(old.as_ref()?.fields[before?].kind));
        if let Some(kind) = kind {
            fields.push(FieldDef::new(field.name(), kind, None));
            src.push(i);
        }
    }
    match Shape::new(&site.table, fields) {
        Ok(shape) => {
            site.slot = vec![NONE; names.len()];
            for (s, &i) in src.iter().enumerate() {
                site.slot[i] = s;
            }
            site.src = src;
            site.shape = Some(shape);
            true
        }
        Err(e) => {
            log::error!("event table {}: {e}; not recording that row", site.table);
            false
        }
    }
}

/// A value held for the row being built; text is in [`Scratch::text`].
#[derive(Clone, Copy)]
enum Held {
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    Str(usize, usize),
}

impl Held {
    const fn kind(self) -> Kind {
        match self {
            Self::I64(_) => Kind::I64,
            Self::U64(_) => Kind::U64,
            Self::F64(_) => Kind::F64,
            Self::Bool(_) => Kind::Bool,
            Self::Str(..) => Kind::Str,
        }
    }
}

/// The event being recorded.
#[derive(Default)]
struct Scratch {
    /// By the call site's field index.
    values: Vec<Option<Held>>,
    text: String,
}

impl Scratch {
    fn value(&self, i: usize) -> Option<Value<'_>> {
        Some(match self.values[i]? {
            Held::I64(v) => Value::I64(v),
            Held::U64(v) => Value::U64(v),
            Held::F64(v) => Value::F64(v),
            Held::Bool(v) => Value::Bool(v),
            Held::Str(start, end) => Value::Str(&self.text[start..end]),
        })
    }
}

/// Collects an event's values into the scratch row in one visit.
struct Collect<'s> {
    site: &'s mut Site,
    scratch: &'s mut Scratch,
    /// The event named its table.
    named: bool,
}

impl Collect<'_> {
    fn table(&mut self, name: &str) {
        if self.site.table != name {
            name.clone_into(&mut self.site.table);
            self.site.shape = None;
        }
        self.named = true;
    }

    fn put(&mut self, field: &Field, value: Held) {
        self.scratch.values[field.index()] = Some(value);
    }
}

impl Visit for Collect<'_> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, Held::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, Held::U64(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.put(field, Held::F64(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, Held::Bool(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == TABLE {
            self.table(value);
        } else {
            let start = self.scratch.text.len();
            self.scratch.text.push_str(value);
            self.put(field, Held::Str(start, self.scratch.text.len()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == TABLE {
            // `table = %name`: rare, so a small allocation is fine here.
            self.table(&format!("{value:?}"));
        } else {
            let start = self.scratch.text.len();
            let _ = write!(self.scratch.text, "{value:?}");
            self.put(field, Held::Str(start, self.scratch.text.len()));
        }
    }
}

/// One span field's value.
enum Attr {
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    Str(String),
}

impl Attr {
    /// `(kind, bits, text)` as the `Trace` message carries it.
    fn wire(&self) -> (u8, u64, &str) {
        match self {
            Self::I64(v) => (Kind::I64 as u8, (*v).cast_unsigned(), ""),
            Self::U64(v) => (Kind::U64 as u8, *v, ""),
            Self::F64(v) => (Kind::F64 as u8, v.to_bits(), ""),
            Self::Bool(v) => (Kind::Bool as u8, u64::from(*v), ""),
            Self::Str(v) => (Kind::Str as u8, 0, v),
        }
    }
}

/// What a span keeps until it closes, in its registry extensions.
struct SpanData {
    trace: TraceId,
    span: u64,
    parent: u64,
    start: Nanos,
    /// By field index.
    values: Vec<Option<Attr>>,
}

/// Collects a span's fields, when it opens and when they are recorded later.
struct Attrs<'a>(&'a mut Vec<Option<Attr>>);

impl Attrs<'_> {
    fn put(&mut self, field: &Field, value: Attr) {
        if let Some(slot) = self.0.get_mut(field.index()) {
            *slot = Some(value);
        }
    }
}

impl Visit for Attrs<'_> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, Attr::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, Attr::U64(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.put(field, Attr::F64(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, Attr::Bool(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field, Attr::Str(value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.put(field, Attr::Str(format!("{value:?}")));
    }
}

/// The definition of spans from `meta`'s call site: its name as the trace
/// name, no stages, its fields as attributes.
fn span_def(meta: &'static Metadata<'static>) -> Option<(u64, Vec<u8>)> {
    let fields: Vec<&str> = meta.fields().iter().map(|f| f.name()).collect();
    let def = TraceDef::new(meta.name(), &[], &fields);
    Some((def.def, def.message().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    thread_local! {
        /// What [`wants`] said of each event and span, on the test's thread.
        static WANTED: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    }

    /// Asks [`wants`] about every event and span it sees.
    struct Ask;

    impl<S: Subscriber> Layer<S> for Ask {
        fn on_new_span(&self, attrs: &Attributes<'_>, _: &Id, _: Context<'_, S>) {
            WANTED.with_borrow_mut(|w| w.push(wants(attrs.metadata())));
        }

        fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
            WANTED.with_borrow_mut(|w| w.push(wants(event.metadata())));
        }
    }

    #[test]
    fn only_table_events_and_spans_at_info_and_above_are_taken() {
        let subscriber = tracing_subscriber::registry().with(Ask);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(table = "spread", bps = 1.0);
            tracing::debug!(table = "spread", bps = 1.0);
            // Metric events are not recorded: metrics are handles.
            tracing::info!(counter = "orders_sent", venue = "binance");
            tracing::info!(gauge = "book_depth", side = "bid", value = 12.0);
            tracing::info!(histogram = "tick_to_trade_ns", value = 850u64);
            tracing::info!(ignored = 1u64);
            tracing::info_span!("connect").in_scope(|| {});
            tracing::warn_span!("retry").in_scope(|| {});
            // A library's internals.
            tracing::debug_span!("framed_read").in_scope(|| {});
        });
        assert_eq!(
            WANTED.take(),
            [true, true, false, false, false, false, true, true, false]
        );
    }

    #[test]
    fn a_shape_aeron_could_not_take_is_sent_with_the_next_frame() {
        let mut sent = HashSet::new();
        let mut sends = 0;
        // Refused: not marked sent, and the row that needs it may not follow.
        assert!(!send_once(&mut sent, 7_u32, || {
            sends += 1;
            false
        }));
        // So the next row sends it, and then it is out.
        for _ in 0..2 {
            assert!(send_once(&mut sent, 7, || {
                sends += 1;
                true
            }));
        }
        assert_eq!(sends, 2);
    }
}
