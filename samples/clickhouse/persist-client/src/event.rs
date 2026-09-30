//! Rows for tables with no SBE message of their own: `tracing` events that
//! name a table, and [`Persist::record_row`]. Their columns are known only at
//! run time.
//!
//! ```text
//! tracing::info!(table = "signal", instrument = %id, edge = 0.25);
//! tracing::info!(counter = "orders_sent", venue = "binance");
//! tracing::info!(gauge = "book_depth", side = "bid", value = 12.0);
//! tracing::info!(histogram = "tick_to_trade_ns", value = 850);
//! ```
//!
//! A counter, gauge, or histogram event updates [`crate::metrics::Metrics`], not a
//! table. `value` is optional on a counter and then means 1. Other fields
//! are labels. The handles in that module stay the path that does not allocate.
//!
//! Each kind of row, a [`Shape`] (the table, and its fields with their kinds
//! in order), is published once as a `Shape` message of `schema/events.xml`
//! before the first row that uses it, and again every 5 s, one shape per
//! later record. A `Row` then
//! carries values only, laid out by its shape: no names, no type tags.
//!
//! ```text
//! Row: header (8) | shape u32 | ts u64 | presence: a bit per field, in u64 words
//!      | each I64/U64/F64 field 8 bytes, each Bool 1, in shape order (0 when absent)
//!      then each Str field in shape order: u32 length + bytes (0 when absent)
//! ```
//!
//! A shape's id is a hash of the shape, so every application on the stream
//! gives it the same id without coordinating. All little-endian.
//!
//! A table event costs one visit of its fields, a lookup of its call site
//! in a thread-local cache, and one `try_claim` written in place: no lock, no
//! allocation, and no field name copied. A counter, gauge, or histogram event
//! does not take that path: it locks the metrics registry and copies its labels.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::field::{Field, FieldSet, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

use crate::Persist;

/// The codec generated from `schema/events.xml`.
#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
pub mod codec {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

/// Why encoding a source, metric, trace, or shape message failed.
pub type EncodeError = codec::sbe_rt::EncodeError;

/// A group count that fits the SBE `numInGroup` header.
pub(crate) fn group_count(n: usize) -> Result<u16, EncodeError> {
    u16::try_from(n).map_err(|_| EncodeError::GroupCountOverflow {
        maximum: u32::from(u16::MAX),
        actual: u32::try_from(n).unwrap_or(u32::MAX),
    })
}

/// `write` fills an exact-length buffer. The codec error is returned as itself.
pub(crate) fn owned_frame(
    len: usize,
    write: impl FnOnce(&mut [u8]) -> Result<usize, EncodeError>,
) -> Result<Vec<u8>, EncodeError> {
    let mut message = vec![0; len];
    let written = write(&mut message)?;
    debug_assert_eq!(written, len);
    Ok(message)
}

/// The schema id of `schema/events.xml`: marks a shape or a row.
pub const SCHEMA_ID: u16 = codec::ShapeEncoder::SCHEMA_ID;
/// Template id of the `Shape` message.
pub const SHAPE_TEMPLATE_ID: u16 = codec::ShapeEncoder::TEMPLATE_ID;
/// Template id of the `Row` message.
pub const ROW_TEMPLATE_ID: u16 = codec::RowEncoder::TEMPLATE_ID;

/// The field that names the table.
pub const TABLE: &str = "table";
/// The field that names a counter. Other fields are labels; `value` is how
/// many to add, or 1 when it is absent.
pub const COUNTER: &str = "counter";
/// The field that names a gauge. `value` is the `f64` to store.
pub const GAUGE: &str = "gauge";
/// The field that names a histogram. `value` is the `u64` sample.
pub const HISTOGRAM: &str = "histogram";
/// The numeric field of a counter, gauge, or histogram event.
pub(crate) const VALUE: &str = "value";

/// An event [`Persist::layer`](crate::Persist::layer) records: a table row,
/// or a counter, gauge, or histogram.
#[must_use]
pub(crate) fn layer_wants(meta: &tracing::Metadata<'_>) -> bool {
    meta.fields()
        .iter()
        .any(|field| matches!(field.name(), TABLE | COUNTER | GAUGE | HISTOGRAM))
}

fn metric_field(meta: &tracing::Metadata<'_>) -> Option<&'static str> {
    meta.fields().iter().find_map(|field| match field.name() {
        COUNTER | GAUGE | HISTOGRAM => Some(field.name()),
        _ => None,
    })
}

/// Apply a `tracing` counter, gauge, or histogram event to `metrics`.
/// A table event, or an event with none of those fields, does nothing.
pub(crate) fn record_metric_event(metrics: &crate::metrics::Metrics, event: &Event<'_>) {
    let Some(which) = metric_field(event.metadata()) else {
        return;
    };
    let mut collect = MetricCollect {
        which,
        name: None,
        value_present: false,
        integer: None,
        float: None,
        labels: Vec::new(),
    };
    event.record(&mut collect);
    collect.apply(metrics);
}

/// One visit of a metric event.
struct MetricCollect {
    which: &'static str,
    name: Option<String>,
    value_present: bool,
    integer: Option<u64>,
    float: Option<f64>,
    labels: Vec<(String, String)>,
}

impl MetricCollect {
    fn apply(self, metrics: &crate::metrics::Metrics) {
        let Some(name) = self.name.as_deref() else {
            return;
        };
        let labels: Vec<(&str, &str)> = self
            .labels
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        match self.which {
            COUNTER => {
                if self.value_present && self.integer.is_none() {
                    return;
                }
                metrics.add_counter(name, &labels, self.integer.unwrap_or(1));
            }
            GAUGE => {
                if let Some(value) = self.float {
                    metrics.set_gauge(name, &labels, value);
                }
            }
            HISTOGRAM => {
                if let Some(value) = self.integer {
                    metrics.record_histogram(name, &labels, value);
                }
            }
            _ => {}
        }
    }

    fn label(&mut self, field: &Field, text: &str) {
        if field.name() == VALUE {
            self.value_present = true;
        } else if field.name() == self.which {
            self.name = Some(text.to_owned());
        } else if field.name() != VALUE {
            self.labels.push((field.name().to_owned(), text.to_owned()));
        }
    }

    fn number(&mut self, field: &Field, integer: Option<u64>, float: f64) {
        if field.name() == VALUE {
            self.value_present = true;
            self.integer = integer;
            self.float = Some(float);
        } else if field.name() == self.which {
            self.name = Some(match integer {
                Some(value) => value.to_string(),
                None => float.to_string(),
            });
        } else {
            let text = match integer {
                Some(value) => value.to_string(),
                None => float.to_string(),
            };
            self.labels.push((field.name().to_owned(), text));
        }
    }
}

impl Visit for MetricCollect {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.number(field, u64::try_from(value).ok(), value as f64);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.number(field, Some(value), value as f64);
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.number(field, None, value);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.label(field, if value { "true" } else { "false" });
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.label(field, value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.label(field, &format!("{value:?}"));
    }
}

pub(crate) const HEADER: usize = 8;
/// A row's block before the presence bits: shape id and timestamp.
pub(crate) const ROW_START: usize = 12;
/// No shape field, or no offset in the block.
pub(crate) const NONE: usize = usize::MAX;

/// One field's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value<'a> {
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    /// `&str` fields, and `%display` / `?debug` fields formatted.
    Str(&'a str),
}

impl Value<'_> {
    #[must_use]
    pub const fn kind(&self) -> Kind {
        match self {
            Self::I64(_) => Kind::I64,
            Self::U64(_) => Kind::U64,
            Self::F64(_) => Kind::F64,
            Self::Bool(_) => Kind::Bool,
            Self::Str(_) => Kind::Str,
        }
    }
}

/// What a field holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    I64,
    U64,
    F64,
    Bool,
    Str,
    /// A list: the fields whose parent is this one, once per entry.
    Group,
}

impl Kind {
    /// Bytes in a block: `Str` and `Group` values follow the block instead.
    const fn width(self) -> usize {
        match self {
            Self::Bool => 1,
            Self::Str | Self::Group => 0,
            _ => 8,
        }
    }

    const fn wire(self) -> codec::Kind {
        match self {
            Self::I64 => codec::Kind::I64,
            Self::U64 => codec::Kind::U64,
            Self::F64 => codec::Kind::F64,
            Self::Bool => codec::Kind::Bool,
            Self::Str => codec::Kind::Str,
            Self::Group => codec::Kind::Group,
        }
    }

    const fn from_wire(kind: codec::Kind) -> Option<Self> {
        match kind {
            codec::Kind::I64 => Some(Self::I64),
            codec::Kind::U64 => Some(Self::U64),
            codec::Kind::F64 => Some(Self::F64),
            codec::Kind::Bool => Some(Self::Bool),
            codec::Kind::Str => Some(Self::Str),
            codec::Kind::Group => Some(Self::Group),
            _ => None,
        }
    }
}

/// One field of a shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef {
    /// Its name within its row or group entry; a nested struct's fields are
    /// dotted (`spread.bps`).
    pub name: String,
    pub kind: Kind,
    /// The `Group` field whose entries hold it; `None` for the row itself.
    pub parent: Option<usize>,
}

impl FieldDef {
    #[must_use]
    pub fn new(name: impl Into<String>, kind: Kind, parent: Option<usize>) -> Self {
        Self {
            name: name.into(),
            kind,
            parent,
        }
    }
}

/// A decoded row, column by column: for each shape field, the values it
/// has in the order they occur. A field of the row itself occurs once; a
/// field in a group, once per entry.
#[derive(Debug)]
pub struct Decoded<'a> {
    /// UNIX nanoseconds when it was recorded.
    pub ts: u64,
    pub table: &'a str,
    /// Indexed like [`Shape::fields`].
    pub columns: Vec<Column<'a>>,
}

/// One field's occurrences in a decoded row.
#[derive(Debug, PartialEq)]
pub enum Column<'a> {
    /// A value field: each occurrence, `None` where it is absent.
    Values(Vec<Option<Value<'a>>>),
    /// A `Group` field: each occurrence's number of entries.
    Counts(Vec<u32>),
}

/// The layout of the row, or of one group's entries.
#[derive(Debug)]
pub(crate) struct Level {
    /// Its fields, in order.
    pub(crate) fields: Vec<usize>,
    /// Where its presence bits start in its block: after the shape id and
    /// timestamp for the row, at once for a group entry.
    pub(crate) presence: usize,
    /// Each field's offset in the block; [`NONE`] for `Str` and `Group`.
    pub(crate) offsets: Vec<usize>,
    pub(crate) block: usize,
}

/// [`Shape::new`] rejected the fields before they could be encoded.
#[derive(Debug)]
pub enum ShapeError {
    /// `field`'s parent index is not an earlier group field.
    Parent {
        /// The field that named the bad parent.
        field: String,
        /// The parent index it named.
        parent: usize,
    },
    /// The row's fixed block does not fit in a `u16` block length.
    Block {
        /// Bytes the row block would occupy.
        bytes: usize,
    },
    /// The `Shape` message itself could not be encoded.
    Encode(EncodeError),
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parent { field, parent } => {
                write!(f, "field {field}: parent {parent} is not an earlier group")
            }
            Self::Block { bytes } => write!(f, "row block over 65535 bytes ({bytes})"),
            Self::Encode(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ShapeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(err) => Some(err),
            Self::Parent { .. } | Self::Block { .. } => None,
        }
    }
}

impl From<EncodeError> for ShapeError {
    fn from(err: EncodeError) -> Self {
        Self::Encode(err)
    }
}

/// The layout of one kind of row: its table, and its fields in order.
#[derive(Debug)]
pub struct Shape {
    pub id: u32,
    pub table: String,
    pub fields: Vec<FieldDef>,
    /// `levels[0]` is the row; each `Group` field has one for its entries.
    pub(crate) levels: Vec<Level>,
    /// A `Group` field's level; [`NONE`] for other fields.
    pub(crate) entries: Vec<usize>,
    /// Each field's ClickHouse column name: its path, groups included.
    columns: Vec<String>,
    /// No groups: [`Shape::write_row`] can write its rows.
    flat: bool,
    /// `Str` fields of the row itself.
    strs: usize,
    /// This shape as a `Shape` message.
    message: Vec<u8>,
    /// Its message is in the stream: rows may follow.
    sent: AtomicBool,
}

impl Shape {
    /// The shape of rows of `table` with these fields, in this order. A
    /// field's parent must be an earlier `Group` field.
    ///
    /// # Errors
    ///
    /// A parent that is not an earlier group, a row block that does not fit
    /// a `u16`, or a `Shape` message the codec rejects.
    pub fn new(table: &str, fields: Vec<FieldDef>) -> Result<Self, ShapeError> {
        let id = shape_id(
            table,
            fields.iter().map(|f| (f.name.as_str(), f.kind, f.parent)),
        );
        let mut levels = vec![Level {
            fields: Vec::new(),
            presence: ROW_START,
            offsets: Vec::new(),
            block: 0,
        }];
        let mut entries = vec![NONE; fields.len()];
        let mut columns: Vec<String> = Vec::with_capacity(fields.len());
        for (i, f) in fields.iter().enumerate() {
            let level = match f.parent {
                None => 0,
                Some(p) if p < i && fields[p].kind == Kind::Group => entries[p],
                Some(p) => {
                    return Err(ShapeError::Parent {
                        field: f.name.clone(),
                        parent: p,
                    });
                }
            };
            levels[level].fields.push(i);
            // ClickHouse takes array columns that share their name's first
            // segment for one Nested structure, and needs their lengths
            // equal. Only one group's entries are sure to be, so a group of
            // the row itself is one segment: `stats.bids` is `stats_bids`.
            let name = match (f.parent, f.kind) {
                (None, Kind::Group) => f.name.replace('.', "_"),
                _ => f.name.clone(),
            };
            let path = f.parent.map_or("", |p| columns[p].as_str());
            columns.push(match (path, name.as_str()) {
                ("", "") => "value".to_owned(),
                ("", _) => name,
                (path, "") => path.to_owned(),
                (path, name) => format!("{path}.{name}"),
            });
            if f.kind == Kind::Group {
                entries[i] = levels.len();
                levels.push(Level {
                    fields: Vec::new(),
                    presence: 0,
                    offsets: Vec::new(),
                    block: 0,
                });
            }
        }
        for level in &mut levels {
            let mut at = level.presence + 8 * level.fields.len().div_ceil(64);
            level.offsets = level
                .fields
                .iter()
                .map(|&f| match fields[f].kind.width() {
                    0 => NONE,
                    width => {
                        at += width;
                        at - width
                    }
                })
                .collect();
            level.block = at;
        }
        let block = levels[0].block;
        if u16::try_from(block).is_err() {
            return Err(ShapeError::Block { bytes: block });
        }
        let count = group_count(fields.len())?;
        let len = codec::ShapeEncodedLength::new()
            .fields_ragged(count, |g| {
                for f in &fields {
                    g.add()?.name(f.name.len())?;
                }
                Ok(())
            })?
            .table(table.len())?
            .encoded_length_with_header();
        let message = owned_frame(len, |message| {
            Ok(codec::ShapeEncoder::wrap_and_apply_header(message, 0)
                .fixed(&codec::ShapeFixedFields { shape: id })
                .fields(count, |g| {
                    for f in &fields {
                        g.add(|mut e| {
                            e.kind(f.kind.wire()).parent(
                                f.parent
                                    .map_or(codec::ShapeFieldsEntryDecoder::PARENT_NULL, |p| {
                                        p as u16
                                    }),
                            );
                            e.name(f.name.as_bytes())
                        })?;
                    }
                    Ok(())
                })?
                .table(table.as_bytes())?
                .encoded_length_with_header())
        })?;
        Ok(Self {
            id,
            table: table.to_owned(),
            flat: levels.len() == 1,
            strs: fields
                .iter()
                .filter(|f| f.kind == Kind::Str && f.parent.is_none())
                .count(),
            fields,
            levels,
            entries,
            columns,
            message,
            sent: AtomicBool::new(false),
        })
    }

    /// A shape from its `Shape` message (header included). `None` when the
    /// message is malformed or its id does not match its contents.
    #[must_use]
    pub fn decode(message: &[u8]) -> Option<Self> {
        let decoder = codec::ShapeDecoder::decode(message, 0).ok()?;
        let fields = decoder
            .fields()
            .ok()?
            .map(|e| {
                let e = e.ok()?;
                Some(FieldDef {
                    name: e.name_as_str().ok()?.to_owned(),
                    kind: Kind::from_wire(e.kind())?,
                    parent: e.parent().map(usize::from),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let shape = Self::new(decoder.table_as_str().ok()?, fields).ok()?;
        (shape.id == decoder.shape()).then_some(shape)
    }

    /// This shape as a `Shape` message, header included.
    #[must_use]
    pub fn message(&self) -> &[u8] {
        &self.message
    }

    /// The shape's message is in the stream; rows of it may follow.
    pub(crate) fn mark_sent(&self) {
        self.sent.store(true, Ordering::Release);
    }

    /// Field `i`'s ClickHouse column: its path, groups included (`bids.price`).
    #[must_use]
    pub fn column(&self, i: usize) -> &str {
        &self.columns[i]
    }

    /// The `Group` fields around field `i`, outermost first.
    #[must_use]
    pub fn groups_around(&self, i: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut at = self.fields[i].parent;
        while let Some(p) = at {
            chain.push(p);
            at = self.fields[p].parent;
        }
        chain.reverse();
        chain
    }

    /// Bytes of a row of a flat shape whose `Str` values total `text` bytes.
    #[must_use]
    pub fn row_len(&self, text: usize) -> usize {
        HEADER + self.levels[0].block + 4 * self.strs + text
    }

    /// Write a row of a flat shape (no groups) into `buf`, which must be
    /// [`Shape::row_len`] bytes for the text given (it panics if shorter).
    /// `value(i)` is field `i`'s value, or `None` when absent; a value of
    /// another kind than the field's is written as absent. Rows with groups
    /// are written by [`Persist::record_value`].
    pub fn write_row<'v>(
        &self,
        buf: &mut [u8],
        ts: u64,
        value: impl Fn(usize) -> Option<Value<'v>>,
    ) {
        assert!(
            self.flat,
            "write_row writes flat shapes; record_value writes groups"
        );
        let top = &self.levels[0];
        let (header, rest) = buf.split_at_mut(HEADER);
        self.write_header(header);
        let (block, mut tail) = rest.split_at_mut(top.block);
        block[0..4].copy_from_slice(&self.id.to_le_bytes());
        block[4..12].copy_from_slice(&ts.to_le_bytes());
        block[ROW_START..].fill(0);
        for (i, (&at, f)) in top.offsets.iter().zip(&self.fields).enumerate() {
            let value = value(i).filter(|v| v.kind() == f.kind);
            if value.is_some() {
                block[ROW_START + i / 8] |= 1 << (i % 8);
            }
            let bytes = match value {
                Some(Value::I64(v)) => v.to_le_bytes(),
                Some(Value::U64(v)) => v.to_le_bytes(),
                Some(Value::F64(v)) => v.to_le_bytes(),
                Some(Value::Bool(v)) => {
                    block[at] = u8::from(v);
                    continue;
                }
                Some(Value::Str(s)) => {
                    let (len, rest) = std::mem::take(&mut tail).split_at_mut(4);
                    len.copy_from_slice(&(s.len() as u32).to_le_bytes());
                    let (text, rest) = rest.split_at_mut(s.len());
                    text.copy_from_slice(s.as_bytes());
                    tail = rest;
                    continue;
                }
                None if at == NONE => {
                    let (len, rest) = std::mem::take(&mut tail).split_at_mut(4);
                    len.fill(0);
                    tail = rest;
                    continue;
                }
                None => continue,
            };
            block[at..at + 8].copy_from_slice(&bytes);
        }
    }

    pub(crate) fn write_header(&self, header: &mut [u8]) {
        header[0..2].copy_from_slice(&(self.levels[0].block as u16).to_le_bytes());
        header[2..4].copy_from_slice(&ROW_TEMPLATE_ID.to_le_bytes());
        header[4..6].copy_from_slice(&SCHEMA_ID.to_le_bytes());
        header[6..8].fill(0);
    }

    /// Decode a row of this shape (header included). `None` when it is not
    /// one, or is malformed.
    #[must_use]
    pub fn decode_row<'a>(&'a self, row: &'a [u8]) -> Option<Decoded<'a>> {
        let u16_at = |at: usize| Some(u16::from_le_bytes(row.get(at..at + 2)?.try_into().ok()?));
        let block = usize::from(u16_at(0)?);
        if (u16_at(2)?, u16_at(4)?) != (ROW_TEMPLATE_ID, SCHEMA_ID)
            || row.get(HEADER..HEADER + 4)? != self.id.to_le_bytes()
            || block < self.levels[0].block
        {
            return None;
        }
        let mut columns: Vec<Column<'a>> = self
            .fields
            .iter()
            .map(|f| match f.kind {
                Kind::Group => Column::Counts(Vec::new()),
                _ => Column::Values(Vec::new()),
            })
            .collect();
        let mut tail = HEADER + block;
        self.decode_level(0, row, HEADER, &mut tail, &mut columns)?;
        (tail == row.len()).then_some(Decoded {
            ts: u64::from_le_bytes(row.get(HEADER + 4..HEADER + 12)?.try_into().ok()?),
            table: &self.table,
            columns,
        })
    }

    /// Decode one row or group entry whose block starts at `base`; its
    /// `Str` and `Group` values start at `tail`, which ends past them.
    fn decode_level<'a>(
        &self,
        level: usize,
        row: &'a [u8],
        base: usize,
        tail: &mut usize,
        columns: &mut [Column<'a>],
    ) -> Option<()> {
        let u32_at = |at: usize| Some(u32::from_le_bytes(row.get(at..at + 4)?.try_into().ok()?));
        let u64_at = |at: usize| Some(u64::from_le_bytes(row.get(at..at + 8)?.try_into().ok()?));
        let l = &self.levels[level];
        for (p, (&f, &at)) in l.fields.iter().zip(&l.offsets).enumerate() {
            let present = row.get(base + l.presence + p / 8)? & (1 << (p % 8)) != 0;
            let value = match self.fields[f].kind {
                Kind::Group => {
                    let count = u32_at(*tail)?;
                    *tail += 4;
                    // Every entry takes at least a byte: a larger count is corrupt.
                    if count as usize > row.len() - *tail {
                        return None;
                    }
                    if let Column::Counts(counts) = &mut columns[f] {
                        counts.push(count);
                    }
                    let entries = self.entries[f];
                    for _ in 0..count {
                        let entry = *tail;
                        *tail += self.levels[entries].block;
                        self.decode_level(entries, row, entry, tail, columns)?;
                    }
                    continue;
                }
                Kind::Str => {
                    let len = u32_at(*tail)? as usize;
                    let text = std::str::from_utf8(row.get(*tail + 4..*tail + 4 + len)?).ok()?;
                    *tail += 4 + len;
                    Value::Str(text)
                }
                Kind::Bool => Value::Bool(*row.get(base + at)? != 0),
                Kind::I64 => Value::I64(u64_at(base + at)? as i64),
                Kind::U64 => Value::U64(u64_at(base + at)?),
                Kind::F64 => Value::F64(f64::from_bits(u64_at(base + at)?)),
            };
            if let Column::Values(values) = &mut columns[f] {
                values.push(present.then_some(value));
            }
        }
        Some(())
    }
}

/// FNV-1a over the table, then each field's name, kind and parent. A field
/// of the row itself hashes no parent, so flat shapes keep the ids they had
/// before groups existed.
fn shape_id<'a>(table: &str, fields: impl Iterator<Item = (&'a str, Kind, Option<usize>)>) -> u32 {
    let mut hash = 0x811c_9dc5_u32;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            hash = (hash ^ u32::from(b)).wrapping_mul(0x0100_0193);
        }
    };
    eat(table.as_bytes());
    for (name, kind, parent) in fields {
        eat(&[0xFF]);
        eat(name.as_bytes());
        eat(&[0xFE, kind.wire() as u8]);
        if let Some(p) = parent {
            eat(&[0xFD]);
            eat(&(p as u16).to_le_bytes());
        }
    }
    hash
}

/// FNV-1a, 64 bits: how series, sources and traces are keyed.
#[must_use]
pub fn fnv64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub(crate) fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

impl Persist {
    /// Record one row of an event table from fields known only at run time,
    /// such as data that arrives as JSON: the row a `tracing` event with
    /// these fields would make. Nothing is built when the table is off.
    pub fn record_row<'a>(
        &self,
        table: &str,
        fields: impl IntoIterator<Item = (&'a str, Value<'a>)>,
    ) {
        if !self.event_enabled(table) {
            return;
        }
        // ponytail: allocates; this path is for data that arrives as JSON,
        // not for the hot path, which is `tracing` events.
        let fields: Vec<(&str, Value<'_>)> = fields.into_iter().collect();
        let Some(shape) = self.shape(table, fields.iter().map(|(n, v)| (*n, v.kind(), None)))
        else {
            self.drop_one();
            return;
        };
        if !self.send_shape(&shape) {
            return;
        }
        let text = fields
            .iter()
            .map(|(_, v)| match v {
                Value::Str(s) => s.len(),
                _ => 0,
            })
            .sum();
        let ts = now_ns();
        self.claim(shape.row_len(text), |buf| {
            shape.write_row(buf, ts, |i| Some(fields[i].1));
            true
        });
    }

    /// The shape of `table` with these fields, made and registered the
    /// first time it is seen. `None`, logged, when it cannot be made or its
    /// id is taken by another shape.
    pub(crate) fn shape<'a>(
        &self,
        table: &str,
        fields: impl Iterator<Item = (&'a str, Kind, Option<usize>)> + Clone,
    ) -> Option<Arc<Shape>> {
        let id = shape_id(table, fields.clone());
        let mut shapes = self
            .inner
            .shared
            .shapes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(shape) = shapes.get(&id) {
            let same = shape.table == table
                && shape.fields.len() == fields.clone().count()
                && shape
                    .fields
                    .iter()
                    .zip(fields.clone())
                    .all(|(f, (name, kind, parent))| {
                        f.name == name && f.kind == kind && f.parent == parent
                    });
            if same {
                return Some(Arc::clone(shape));
            }
            log::error!(
                "event shape id {id} of {} is also {table}'s: not recording that row",
                shape.table
            );
            return None;
        }
        let fields = fields.map(|(name, kind, parent)| FieldDef::new(name, kind, parent));
        match Shape::new(table, fields.collect()) {
            Ok(shape) => Some(Arc::clone(shapes.entry(id).or_insert(Arc::new(shape)))),
            Err(e) => {
                log::error!("event table {table}: {e}; not recording that row");
                None
            }
        }
    }

    /// Put `shape`'s message in the stream unless it is there already.
    /// `false` when Aeron could not take it: rows of it must not follow.
    pub(crate) fn send_shape(&self, shape: &Shape) -> bool {
        if shape.sent.load(Ordering::Acquire) {
            return true;
        }
        let sent = self.publish(shape.message());
        if sent {
            shape.sent.store(true, Ordering::Release);
        }
        sent
    }

    /// Record the event collected in `scratch`, from the call site `site`.
    fn record_event(&self, site: &mut Site, scratch: &Scratch, names: &FieldSet) {
        let fits = site.shape.as_ref().is_some_and(|shape| {
            scratch
                .values
                .iter()
                .zip(&site.slot)
                .all(|(value, &slot)| match value {
                    None => true,
                    Some(v) => slot != NONE && shape.fields[slot].kind == v.kind(),
                })
        });
        if !fits && !self.reshape(site, scratch, names) {
            self.drop_one();
            return;
        }
        let Some(shape) = &site.shape else {
            return;
        };
        if !self.send_shape(shape) {
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
        let ts = now_ns();
        self.claim(shape.row_len(text), |buf| {
            shape.write_row(buf, ts, |i| scratch.value(site.src[i]));
            true
        });
    }

    /// Give `site` a shape that fits this event: its fields' kinds now, and
    /// for a field absent now, the kind it had before.
    #[cold]
    fn reshape(&self, site: &mut Site, scratch: &Scratch, names: &FieldSet) -> bool {
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
                fields.push((field.name(), kind, None));
                src.push(i);
            }
        }
        let Some(shape) = self.shape(&site.table, fields.into_iter()) else {
            return false;
        };
        site.slot = vec![NONE; names.len()];
        for (s, &i) in src.iter().enumerate() {
            site.slot[i] = s;
        }
        site.src = src;
        site.shape = Some(shape);
        true
    }
}

/// What one call site needs, cached per thread: nothing shared on the hot path.
struct Site {
    /// The table this call site last named, and its switch.
    table: String,
    switch: Arc<AtomicBool>,
    shape: Option<Arc<Shape>>,
    /// Shape field `i` is the call site's field `src[i]`.
    src: Vec<usize>,
    /// The call site's field `i` is shape field `slot[i]`, or [`NONE`].
    slot: Vec<usize>,
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

/// The event being recorded, reused so recording does not allocate.
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

/// Hashes the addresses and ids that key the call-site caches: they are
/// unique already.
#[derive(Default)]
pub(crate) struct AddressHasher(u64);

impl Hasher for AddressHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn write_usize(&mut self, n: usize) {
        self.0 = (self.0.rotate_left(29) ^ n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

type Sites = HashMap<(usize, usize), Site, BuildHasherDefault<AddressHasher>>;

thread_local! {
    /// Keyed by the `Persist`'s id and the call site's metadata.
    static SITES: RefCell<Sites> = RefCell::default();
    static SCRATCH: RefCell<Scratch> = RefCell::default();
}

/// Records every `tracing` event that has a `table` field whose table is
/// enabled in `tables.yaml`. Make it with [`Persist::layer`].
pub struct PersistLayer {
    pub(crate) persist: Persist,
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for PersistLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        crate::spans::on_new_span(&self.persist, attrs, id, &ctx);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        crate::spans::on_record(id, values, &ctx);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        crate::spans::on_close(&self.persist, &id, &ctx);
    }

    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let meta = event.metadata();
        if metric_field(meta).is_some() {
            record_metric_event(&self.persist.metrics(), event);
            return;
        }
        let key = (
            self.persist.inner.id as usize,
            std::ptr::from_ref(meta) as usize,
        );
        // An event recorded while recording one (a log line from inside
        // Aeron, say) finds both borrowed, and is not recorded.
        let _ = SITES.with(|sites| {
            SCRATCH.with(|scratch| {
                let (mut sites, mut scratch) = (sites.try_borrow_mut()?, scratch.try_borrow_mut()?);
                let site = sites.entry(key).or_insert_with(|| Site {
                    table: String::new(),
                    switch: Arc::new(AtomicBool::new(false)),
                    shape: None,
                    src: Vec::new(),
                    slot: Vec::new(),
                });
                let scratch = &mut *scratch;
                scratch.values.clear();
                scratch.values.resize(meta.fields().len(), None);
                scratch.text.clear();
                let mut collect = Collect {
                    persist: &self.persist,
                    site,
                    scratch,
                    on: None,
                };
                event.record(&mut collect);
                if collect.on == Some(true) {
                    self.persist.record_event(site, scratch, meta.fields());
                }
                Ok::<_, std::cell::BorrowMutError>(())
            })
        });
    }
}

/// Collects an event's values into the scratch row in one visit, and
/// formats nothing once its table turns out to be off.
struct Collect<'s> {
    persist: &'s Persist,
    site: &'s mut Site,
    scratch: &'s mut Scratch,
    /// `None` until the `table` field; then whether that table is on.
    on: Option<bool>,
}

impl Collect<'_> {
    fn table(&mut self, name: &str) {
        if self.site.table != name {
            name.clone_into(&mut self.site.table);
            self.site.switch = self.persist.event_switch(name);
            self.site.shape = None;
        }
        self.on = Some(self.site.switch.load(Ordering::Relaxed));
    }

    fn put(&mut self, field: &Field, value: Held) {
        if self.on != Some(false) {
            self.scratch.values[field.index()] = Some(value);
        }
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
        } else if self.on != Some(false) {
            let start = self.scratch.text.len();
            self.scratch.text.push_str(value);
            self.put(field, Held::Str(start, self.scratch.text.len()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == TABLE {
            // `table = %name`: rare, so a small allocation is fine here.
            self.table(&format!("{value:?}"));
        } else if self.on != Some(false) {
            let start = self.scratch.text.len();
            let _ = write!(self.scratch.text, "{value:?}");
            self.put(field, Held::Str(start, self.scratch.text.len()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct Take(
        crate::metrics::Metrics,
        std::sync::Arc<std::sync::atomic::AtomicU64>,
    );

    impl<S: Subscriber> Layer<S> for Take {
        fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
            if layer_wants(event.metadata()) {
                self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                record_metric_event(&self.0, event);
            }
        }
    }

    #[test]
    fn tracing_macros_update_the_metric_registry() {
        let metrics = crate::metrics::Metrics::detached();
        let admitted = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let subscriber = tracing_subscriber::registry()
            .with(Take(metrics.clone(), std::sync::Arc::clone(&admitted)));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(counter = "orders_sent", venue = "binance");
            tracing::info!(counter = "orders_sent", venue = "binance", value = 4u64);
            tracing::info!(gauge = "book_depth", side = "bid", value = 12.0);
            tracing::info!(histogram = "tick_to_trade_ns", value = 850u64);
            tracing::info!(table = "spread", bps = 1.0);
            tracing::info!(ignored = 1u64);
        });
        assert_eq!(
            metrics.counter_total("orders_sent", &[("venue", "binance")]),
            5
        );
        assert_eq!(metrics.gauge("book_depth", &[("side", "bid")]).get(), 12.0);
        assert_eq!(metrics.histogram_samples("tick_to_trade_ns", &[]), 1);
        // The table event is admitted and then ignored by the metric path.
        // The plain event is not admitted.
        assert_eq!(admitted.load(std::sync::atomic::Ordering::Relaxed), 5);
    }

    #[test]
    fn invalid_counter_values_do_not_turn_into_default_increments() {
        let metrics = crate::metrics::Metrics::detached();
        let subscriber = tracing_subscriber::registry().with(Take(
            metrics.clone(),
            std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        ));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(counter = "requests", value = "invalid");
            tracing::info!(counter = "requests", value = true);
            tracing::info!(counter = "requests", value = ?Some(1));
            tracing::info!(counter = "requests", value = -1);
            tracing::info!(counter = "requests", value = 1.5);
            tracing::info!(counter = "requests", value = 0u64);
            tracing::info!(counter = "requests");
        });
        assert_eq!(metrics.counter_total("requests", &[]), 1);
    }

    fn flat(table: &str, fields: &[(&str, Kind)]) -> Result<Shape, ShapeError> {
        Shape::new(
            table,
            fields
                .iter()
                .map(|(n, k)| FieldDef::new(*n, *k, None))
                .collect(),
        )
    }

    fn signal() -> Result<Shape, ShapeError> {
        flat(
            "signal",
            &[
                ("instrument", Kind::Str),
                ("edge", Kind::F64),
                ("n", Kind::I64),
                ("flag", Kind::Bool),
                ("note", Kind::Str),
                ("big", Kind::U64),
            ],
        )
    }

    fn values<'a>(decoded: &'a Decoded<'a>, i: usize) -> &'a [Option<Value<'a>>] {
        match &decoded.columns[i] {
            Column::Values(v) => v,
            Column::Counts(_) => &[],
        }
    }

    #[test]
    fn a_row_carries_values_only_and_round_trips() -> TestResult {
        let shape = signal()?;
        let row_values = [
            Some(Value::Str("BTCUSDT")),
            Some(Value::F64(0.25)),
            None,
            Some(Value::Bool(true)),
            None,
            Some(Value::U64(u64::MAX)),
        ];
        let mut row = vec![0; shape.row_len("BTCUSDT".len())];
        shape.write_row(&mut row, 42, |i| row_values[i]);
        // Header, id + ts, one presence word, 3 x 8 + 1 fixed, two lengths, the text.
        assert_eq!(row.len(), 8 + 12 + 8 + 25 + 8 + 7);
        let decoded = shape.decode_row(&row).ok_or("undecodable")?;
        assert_eq!((decoded.ts, decoded.table), (42, "signal"));
        for (i, expected) in row_values.iter().enumerate() {
            assert_eq!(values(&decoded, i), [*expected]);
        }
        assert!(
            shape.decode_row(&row[..row.len() - 1]).is_none(),
            "a cut row is malformed"
        );

        // A value of the wrong kind is absent, and the row stays well formed.
        let mut row = vec![0; shape.row_len(0)];
        shape.write_row(&mut row, 1, |i| (i == 1).then_some(Value::Str("oops")));
        let decoded = shape.decode_row(&row).ok_or("undecodable")?;
        assert!((0..6).all(|i| values(&decoded, i) == [None]));
        Ok(())
    }

    #[test]
    fn a_shape_round_trips_through_its_message() -> TestResult {
        let nested = Shape::new(
            "book",
            vec![
                FieldDef::new("symbol", Kind::Str, None),
                FieldDef::new("bids", Kind::Group, None),
                FieldDef::new("price", Kind::F64, Some(1)),
                FieldDef::new("orders", Kind::Group, Some(1)),
                FieldDef::new("", Kind::U64, Some(3)),
            ],
        )?;
        for shape in [signal()?, nested] {
            let decoded = Shape::decode(shape.message()).ok_or("undecodable")?;
            assert_eq!(
                (decoded.id, &decoded.table, &decoded.fields),
                (shape.id, &shape.table, &shape.fields)
            );
        }
        // The id is the shape's hash: another order, kind or parent is another shape.
        let a = flat("s", &[("x", Kind::F64), ("y", Kind::Str)])?;
        let b = flat("s", &[("y", Kind::Str), ("x", Kind::F64)])?;
        let c = flat("s", &[("x", Kind::I64), ("y", Kind::Str)])?;
        assert!(a.id != b.id && a.id != c.id && b.id != c.id);
        let mut tampered = signal()?.message().to_vec();
        tampered[8] ^= 1; // the id no longer matches the contents
        assert!(Shape::decode(&tampered).is_none());
        Ok(())
    }

    #[test]
    fn columns_are_named_by_their_path_and_parents_must_be_earlier_groups() -> TestResult {
        let shape = Shape::new(
            "book",
            vec![
                FieldDef::new("bids", Kind::Group, None),
                FieldDef::new("price", Kind::F64, Some(0)),
                FieldDef::new("orders", Kind::Group, Some(0)),
                FieldDef::new("", Kind::U64, Some(2)),
                FieldDef::new("", Kind::I64, None),
            ],
        )?;
        let names: Vec<&str> = (0..5).map(|i| shape.column(i)).collect();
        assert_eq!(
            names,
            ["bids", "bids.price", "bids.orders", "bids.orders", "value"]
        );
        assert_eq!(shape.groups_around(3), [0, 2]);
        assert!(Shape::new("x", vec![FieldDef::new("a", Kind::I64, Some(0))]).is_err());
        assert!(
            Shape::new(
                "x",
                vec![
                    FieldDef::new("a", Kind::I64, None),
                    FieldDef::new("b", Kind::I64, Some(0)),
                ]
            )
            .is_err(),
            "a parent must be a group"
        );
        Ok(())
    }

    #[test]
    fn more_than_64_fields_take_more_presence_words() -> TestResult {
        let fields: Vec<(String, Kind)> = (0..70).map(|i| (format!("f{i}"), Kind::I64)).collect();
        let names: Vec<(&str, Kind)> = fields.iter().map(|(n, k)| (n.as_str(), *k)).collect();
        let shape = flat("wide", &names)?;
        let mut row = vec![0; shape.row_len(0)];
        shape.write_row(&mut row, 1, |i| {
            (i % 2 == 1).then_some(Value::I64(i as i64))
        });
        let decoded = shape.decode_row(&row).ok_or("undecodable")?;
        assert_eq!(values(&decoded, 69), [Some(Value::I64(69))]);
        assert_eq!(values(&decoded, 68), [None]);
        Ok(())
    }
}
