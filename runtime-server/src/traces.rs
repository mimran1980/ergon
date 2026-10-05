//! Traces from `ergon_runtime::trace` into `otel_traces`, the OpenTelemetry
//! `ClickHouse` exporter's table, which Grafana's trace view reads as it is.
//!
//! A checkpoint trace becomes a span for the whole (named after the trace)
//! and one child per stage, from the previous mark to its own. A `tracing`
//! span is one span, with its own parent. The application's name is the
//! service, and its host, pod and name the resource attributes.

use std::collections::HashMap;

use ergon_runtime::event::{Kind, codec};
use ergon_runtime::trace::TraceDef;

use crate::metrics::{Row, column};
use crate::table::{DecodeError, Shape, write_varint};

const TS: &str = "DateTime64(9)";
const ATTRIBUTES: &str = "Map(LowCardinality(String), String)";

/// The exporter's columns, in its order (`traces_table.sql`). Events and
/// links are always empty here.
pub fn traces_shape() -> Shape {
    Shape {
        name: ergon_runtime::persist::OTEL_TRACES.into(),
        columns: vec![
            column("Timestamp", TS),
            column("TraceId", "String"),
            column("SpanId", "String"),
            column("ParentSpanId", "String"),
            column("TraceState", "String"),
            column("SpanName", "LowCardinality(String)"),
            column("SpanKind", "LowCardinality(String)"),
            column("ServiceName", "LowCardinality(String)"),
            column("ResourceAttributes", ATTRIBUTES),
            column("ScopeName", "String"),
            column("ScopeVersion", "String"),
            column("SpanAttributes", ATTRIBUTES),
            column("Duration", "UInt64"),
            column("StatusCode", "LowCardinality(String)"),
            column("StatusMessage", "String"),
            column("Events.Timestamp", "Array(DateTime64(9))"),
            column("Events.Name", "Array(LowCardinality(String))"),
            column(
                "Events.Attributes",
                "Array(Map(LowCardinality(String), String))",
            ),
            column("Links.TraceId", "Array(String)"),
            column("Links.SpanId", "Array(String)"),
            column("Links.TraceState", "Array(String)"),
            column(
                "Links.Attributes",
                "Array(Map(LowCardinality(String), String))",
            ),
        ],
        // ponytail: the exporter sorts by `toDateTime(Timestamp)` and adds
        // bloom-filter indexes on the ids and attributes; add them if trace
        // lookups by id get slow.
        order_by: vec!["ServiceName".into(), "SpanName".into(), "Timestamp".into()],
        partition: Some("Timestamp".into()),
    }
}

/// The `TraceDef` a `Trace` message needs, if it has not arrived.
pub fn unknown_def(message: &[u8], defs: &HashMap<u64, TraceDef>) -> Option<bool> {
    let t = codec::TraceDecoder::decode(message, 0).ok()?;
    Some(!defs.contains_key(&t.def()))
}

/// When a `Trace` message's trace or span started, UNIX ns.
pub fn start(message: &[u8]) -> Option<u64> {
    codec::TraceDecoder::decode(message, 0)
        .ok()
        .map(|t| t.start())
}

fn span_id(parts: &[u64]) -> u64 {
    let bytes: Vec<u8> = parts.iter().flat_map(|p| p.to_le_bytes()).collect();
    ergon_runtime::event::fnv64(&bytes).max(1)
}

/// One span's columns.
struct Span<'a> {
    start: i64,
    trace: &'a str,
    id: u64,
    parent: u64,
    name: &'a str,
    duration: i64,
}

/// A `Trace` message as rows of `otel_traces`. `who` is the recording
/// application's host, pod and app.
#[allow(clippy::too_many_lines)]
pub fn write_trace(
    message: &[u8],
    include: &[bool],
    origin: &[u8],
    who: [&str; 3],
    defs: &HashMap<u64, TraceDef>,
    out: &mut Vec<u8>,
) -> Result<usize, DecodeError> {
    let bad = |_| DecodeError("undecodable Trace message");
    let t = codec::TraceDecoder::decode(message, 0).map_err(bad)?;
    let Some(def) = defs.get(&t.def()) else {
        return Ok(0);
    };
    let marks: Vec<i64> = t.marks().map_err(bad)?.map(|e| e.ns()).collect();
    let why = match t.why() {
        codec::TraceWhy::Sampled => "sampled",
        codec::TraceWhy::Slow => "slow",
        codec::TraceWhy::Kept => "kept",
        _ => "span",
    };
    let mut attributes: Vec<(&str, String)> = Vec::new();
    for (name, e) in def.attrs.iter().zip(t.attrs().map_err(bad)?) {
        let e = e.map_err(bad)?;
        let bits = e.bits();
        let value = match e.kind() {
            k if k == Kind::I64 as u8 => bits.cast_signed().to_string(),
            k if k == Kind::U64 as u8 => bits.to_string(),
            k if k == Kind::F64 as u8 => f64::from_bits(bits).to_string(),
            k if k == Kind::Bool as u8 => (bits != 0).to_string(),
            k if k == Kind::Str as u8 => e.text_as_str().map_err(bad)?.to_owned(),
            _ => continue, // absent
        };
        attributes.push((name, value));
    }
    attributes.push(("why", why.to_owned()));
    let [host, pod, app] = who;
    let resource = [
        ("host.name", host),
        ("k8s.pod.name", pod),
        ("service.name", app),
    ];
    let trace = format!("{:016x}{:016x}", t.trace_hi(), t.trace_lo());
    let start = t.start().cast_signed();
    let write = |span: Span<'_>, attributes: &[(&str, String)], out: &mut Vec<u8>| {
        let parent = match span.parent {
            0 => String::new(),
            p => format!("{p:016x}"),
        };
        Row::new(include, out)
            .u64(span.start.cast_unsigned())
            .str(span.trace)
            .str(&format!("{:016x}", span.id))
            .str(&parent)
            .str("")
            .str(span.name)
            .str("Internal")
            .str(app)
            .map(resource.iter().copied())
            .str("persist")
            .str("")
            .map(attributes.iter().map(|(k, v)| (*k, v.as_str())))
            .u64(span.duration.max(0).cast_unsigned())
            .str("Unset")
            .str("")
            .put(empty)
            .put(empty)
            .put(empty)
            .put(empty)
            .put(empty)
            .put(empty)
            .put(empty)
            .end(origin);
    };
    if why == "span" {
        let span = Span {
            start,
            trace: &trace,
            id: t.span(),
            parent: t.parent(),
            name: &def.name,
            duration: marks.first().copied().unwrap_or(0),
        };
        write(span, &attributes, out);
        return Ok(1);
    }
    let root = span_id(&[t.trace_hi(), t.trace_lo(), def.def, t.start()]);
    write(
        Span {
            start,
            trace: &trace,
            id: root,
            parent: 0,
            name: &def.name,
            duration: marks.last().copied().unwrap_or(0),
        },
        &attributes,
        out,
    );
    let mut prev = 0;
    for (i, &end) in marks.iter().enumerate() {
        let name = def.stages.get(i).map_or("stage", String::as_str);
        write(
            Span {
                start: start + prev,
                trace: &trace,
                id: span_id(&[root, i as u64]),
                parent: root,
                name,
                duration: end - prev,
            },
            &[],
            out,
        );
        prev = end;
    }
    Ok(1 + marks.len())
}

/// An empty array.
fn empty(out: &mut Vec<u8>) {
    write_varint(0, out);
}
