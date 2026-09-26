//! Rows for tables with no SBE message of their own: `tracing` events that
//! name a table, and [`Persist::record_row`]. Their columns are known only at
//! run time.
//!
//! ```text
//! tracing::info!(table = "signal", instrument = %id, edge = 0.25);
//! ```
//!
//! Each kind of row, a [`Shape`] (the table, and its fields with their kinds
//! in order), is published once as a `Shape` message of `schema/events.xml`
//! before the first row that uses it, and again every 5 s. A `Row` then
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
//! A `tracing` event costs one visit of its fields, a lookup of its call site
//! in a thread-local cache, and one `try_claim` written in place: no lock, no
//! allocation, and no field name copied.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::field::{Field, FieldSet, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

use crate::Persist;

/// The codec generated from `schema/events.xml`.
#[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
#[rustfmt::skip]
pub mod codec {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

/// The schema id of `schema/events.xml`: marks a shape or a row.
pub const SCHEMA_ID: u16 = codec::ShapeEncoder::SCHEMA_ID;
/// Template id of the `Shape` message.
pub const SHAPE_TEMPLATE_ID: u16 = codec::ShapeEncoder::TEMPLATE_ID;
/// Template id of the `Row` message.
pub const ROW_TEMPLATE_ID: u16 = codec::RowEncoder::TEMPLATE_ID;

/// The field that names the table.
pub const TABLE: &str = "table";

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
    pub fn new(table: &str, fields: Vec<FieldDef>) -> Result<Self, String> {
        let id = shape_id(
            table,
            fields.iter().map(|f| (f.name.as_str(), f.kind, f.parent)),
        );
        let count = u16::try_from(fields.len()).map_err(|_| "more than 65535 fields")?;
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
                    return Err(format!(
                        "field {}: parent {p} is not an earlier group",
                        f.name
                    ));
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
        u16::try_from(levels[0].block).map_err(|_| "row block over 65535 bytes")?;
        let len = codec::ShapeEncodedLength::new()
            .fields_ragged(count, |g| {
                for f in &fields {
                    g.add()?.name(f.name.len())?;
                }
                Ok(())
            })
            .and_then(|l| l.table(table.len()))
            .map_err(|e| e.to_string())?
            .encoded_length_with_header();
        let mut message = vec![0; len];
        let written = codec::ShapeEncoder::wrap_and_apply_header(&mut message, 0)
            .fixed(&codec::ShapeFixedFields { shape: id })
            .fields(count, |g| {
                for f in &fields {
                    g.add(|mut e| {
                        e.kind(f.kind.wire()).parent(
                            f.parent
                                .map_or(codec::ShapeFieldsEntryDecoder::PARENT_NULL, |p| p as u16),
                        );
                        e.name(f.name.as_bytes())
                    })?;
                }
                Ok(())
            })
            .and_then(|m| m.table(table.as_bytes()))
            .map_err(|e| e.to_string())?
            .encoded_length_with_header();
        debug_assert_eq!(written, len);
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

/// FNV-1a, 64 bits.
pub(crate) fn fnv64(bytes: &[u8]) -> u64 {
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
            self.drop_one();
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
            self.drop_one();
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

impl<S: Subscriber> Layer<S> for PersistLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let meta = event.metadata();
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

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn flat(table: &str, fields: &[(&str, Kind)]) -> Result<Shape, String> {
        Shape::new(
            table,
            fields
                .iter()
                .map(|(n, k)| FieldDef::new(*n, *k, None))
                .collect(),
        )
    }

    fn signal() -> Result<Shape, String> {
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
