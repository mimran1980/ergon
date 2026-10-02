//! `tracing` spans (`#[instrument]`, `info_span!`) into `otel_traces`, for
//! code off the hot path: each costs what the `tracing` registry costs, a
//! few hundred nanoseconds and an allocation. Hot paths use
//! [`crate::trace`].
//!
//! While `otel_traces` is on for this app, [`Persist::layer`] admits every
//! span at INFO and above (libraries' internal spans are DEBUG and TRACE);
//! while it is off, none, so the registry never stores one. A span's
//! fields are its attributes, and its parent the span it was entered in. A
//! root span starts a new trace.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Metadata, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use crate::clock::{Clock, Nanos};
use crate::event::{Kind, codec};
use crate::persist::Persist;
use crate::trace::{DefMessage, TraceDef, TraceId};

/// One field's value.
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
            Self::I64(v) => (Kind::I64 as u8, *v as u64, ""),
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

struct Collect<'a>(&'a mut Vec<Option<Attr>>);

impl Collect<'_> {
    fn put(&mut self, field: &Field, value: Attr) {
        if let Some(slot) = self.0.get_mut(field.index()) {
            *slot = Some(value);
        }
    }
}

impl Visit for Collect<'_> {
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
        let mut text = String::new();
        let _ = write!(text, "{value:?}");
        self.put(field, Attr::Str(text));
    }
}

/// Span ids: unique in the process, mixed with its source id so they
/// differ across processes too.
static IDS: AtomicU64 = AtomicU64::new(0);

/// Each span callsite's trace definition, by its metadata's address.
#[derive(Default)]
pub(crate) struct SpanDefs(std::sync::Mutex<HashMap<usize, (u64, Arc<DefMessage>)>>);

pub(crate) fn on_new_span<S>(
    persist: &Persist,
    attrs: &Attributes<'_>,
    id: &Id,
    ctx: &Context<'_, S>,
) where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let Some(span) = ctx.span(id) else {
        return;
    };
    let parent = span.parent().and_then(|p| {
        let ext = p.extensions();
        ext.get::<SpanData>().map(|d| (d.trace, d.span))
    });
    let source = persist.bus().source().id;
    let n = IDS.fetch_add(1, Relaxed) + 1;
    // Not `Tracer::next_id`'s namespace (`rotate_left(17) ^ nonce`).
    let (trace, parent) = parent.unwrap_or((TraceId::new(source.rotate_left(41), n), 0));
    let mut values = Vec::new();
    values.resize_with(attrs.metadata().fields().len(), || None);
    attrs.record(&mut Collect(&mut values));
    span.extensions_mut().insert(SpanData {
        trace,
        span: source.rotate_left(29) ^ n,
        parent,
        start: Clock::new().now(),
        values,
    });
}

pub(crate) fn on_record<S>(id: &Id, values: &Record<'_>, ctx: &Context<'_, S>)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    if let Some(span) = ctx.span(id)
        && let Some(data) = span.extensions_mut().get_mut::<SpanData>()
    {
        values.record(&mut Collect(&mut data.values));
    }
}

pub(crate) fn on_close<S>(persist: &Persist, id: &Id, ctx: &Context<'_, S>)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let Some(span) = ctx.span(id) else {
        return;
    };
    let Some(data) = span.extensions_mut().remove::<SpanData>() else {
        return;
    };
    let duration = Clock::new().now().since(data.start);
    let Some((def, message)) = def_of(persist, span.metadata()) else {
        return;
    };
    if !message.sent.load(Relaxed) {
        if !persist.publish(&message.message) {
            return;
        }
        message.sent.store(true, Relaxed);
    }
    let attrs: Vec<(u8, u64, &str)> = data
        .values
        .iter()
        .map(|v| v.as_ref().map_or((255, 0, ""), Attr::wire))
        .collect();
    let Ok(len) = codec::TraceEncodedLength::new()
        .marks(1)
        .and_then(|l| {
            l.attrs_ragged(attrs.len() as u16, |g| {
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
    persist.claim(len, |buf| {
        codec::TraceEncoder::wrap_and_apply_header(buf, 0)
            .fixed(&codec::TraceFixedFields {
                def,
                trace_hi: data.trace.hi,
                trace_lo: data.trace.lo,
                span: data.span,
                parent: data.parent,
                start: data.start.epoch_ns() as u64,
                why: codec::TraceWhy::Span,
            })
            .marks(1, |g| {
                g.add(|e| {
                    e.ns(duration);
                    Ok(())
                })
            })
            .and_then(|m| {
                m.attrs(attrs.len() as u16, |g| {
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

/// The definition of spans from `meta`'s callsite: its name, and its fields
/// as attributes. Made once per callsite.
fn def_of(persist: &Persist, meta: &'static Metadata<'static>) -> Option<(u64, Arc<DefMessage>)> {
    let shared = &persist.inner.shared;
    let mut defs = shared
        .span_defs
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((def, message)) = defs.get(&(std::ptr::from_ref(meta) as usize)) {
        return Some((*def, Arc::clone(message)));
    }
    let fields: Vec<&str> = meta.fields().iter().map(|f| f.name()).collect();
    let def = TraceDef::new(meta.name(), &[], &fields);
    let message = Arc::new(DefMessage {
        message: def.message().ok()?,
        sent: AtomicBool::new(false),
    });
    shared
        .trace_defs
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(Arc::clone(&message));
    defs.insert(
        std::ptr::from_ref(meta) as usize,
        (def.def, Arc::clone(&message)),
    );
    Some((def.def, message))
}
