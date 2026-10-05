//! Rows for tables with no SBE message of their own. Their columns are known
//! only at run time.
//!
//! * [`Persist::record_row`]: fields known at run time, such as data that
//!   arrives as JSON.
//! * [`Persist::record_value`]: any `Serialize` value, however nested.
//! * `tracing` events that name a table, from any thread, through the
//!   [`bridge`](crate::bridge) and its own publication:
//!
//! ```text
//! tracing::info!(table = "signal", instrument = %id, edge = 0.25);
//! ```
//!
//! Each kind of row, a [`Shape`] (the table, and its fields with their kinds
//! in order), is published once as a `Shape` message of `schema/events.xml`
//! before the first row that uses it, and again every 5 s from
//! [`Persist::poll`]. A `Row` then
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

use std::cell::Cell;
use std::hash::Hasher;
use std::rc::Rc;

use crate::persist::Persist;

/// The codec generated from `schema/events.xml`.
#[allow(
    unsafe_code,
    missing_docs,
    warnings,
    unused,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
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

pub(crate) const HEADER: usize = 8;
/// A row's block before the presence bits: shape id and timestamp.
pub(crate) const ROW_START: usize = 12;
/// No shape field, or no offset in the block.
pub(crate) const NONE: usize = usize::MAX;

/// One field's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value<'a> {
    /// A signed integer.
    I64(i64),
    /// An unsigned integer.
    U64(u64),
    /// A floating-point number.
    F64(f64),
    /// A boolean.
    Bool(bool),
    /// `&str` fields, and `%display` / `?debug` fields formatted.
    Str(&'a str),
}

impl Value<'_> {
    /// The [`Kind`] this value encodes as.
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
    /// A signed integer.
    I64,
    /// An unsigned integer.
    U64,
    /// A floating-point number.
    F64,
    /// A boolean.
    Bool,
    /// Text.
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
            codec::Kind::NullVal => None,
        }
    }
}

/// One field of a shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef {
    /// Its name within its row or group entry; a nested struct's fields are
    /// dotted (`spread.bps`).
    pub name: String,
    /// What the field holds.
    pub kind: Kind,
    /// The `Group` field whose entries hold it; `None` for the row itself.
    pub parent: Option<usize>,
}

impl FieldDef {
    /// A field named `name` of `kind`. `parent` is the group that holds it.
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
    /// The table the row was recorded into.
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
    /// Hash of the table and the fields, stable across processes.
    pub id: u32,
    /// The table these rows belong to.
    pub table: String,
    /// Fields in wire order.
    pub fields: Vec<FieldDef>,
    /// `levels[0]` is the row; each `Group` field has one for its entries.
    pub(crate) levels: Vec<Level>,
    /// A `Group` field's level; [`NONE`] for other fields.
    pub(crate) entries: Vec<usize>,
    /// Each field's `ClickHouse` column name: its path, groups included.
    columns: Vec<String>,
    /// No groups: [`Shape::write_row`] can write its rows.
    flat: bool,
    /// `Str` fields of the row itself.
    strs: usize,
    /// This shape as a `Shape` message.
    message: Vec<u8>,
}

/// A shape this application made, and whether its message is in the stream:
/// rows of it may follow.
pub(crate) struct ShapeEntry {
    pub(crate) shape: Shape,
    pub(crate) sent: Cell<bool>,
}

/// A shape's levels, the level of each group field's entries, and each
/// field's column path.
type FieldLayout = (Vec<Level>, Vec<usize>, Vec<String>);

/// Group levels, entry indexes, and column paths for `fields`.
///
/// # Errors
///
/// A parent that is not an earlier group.
fn layout_fields(fields: &[FieldDef]) -> Result<FieldLayout, ShapeError> {
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
        // segment for one Nested structure, and needs their lengths equal.
        // Only one group's entries are sure to be, so a group of the row
        // itself is one segment: `stats.bids` is `stats_bids`.
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
    Ok((levels, entries, columns))
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
        let (levels, entries, columns) = layout_fields(&fields)?;
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
                                        u16::try_from(p).unwrap_or(0)
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

    /// Field `i`'s `ClickHouse` column: its path, groups included (`bids.price`).
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
    /// [`Shape::row_len`] bytes for the text given.
    /// `value(i)` is field `i`'s value, or `None` when absent; a value of
    /// another kind than the field's is written as absent. Rows with groups
    /// are written by [`Persist::record_value`].
    ///
    /// # Panics
    ///
    /// Panics when the shape has groups, or when `buf` is shorter than the row.
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
                    len.copy_from_slice(&u32::try_from(s.len()).unwrap_or(u32::MAX).to_le_bytes());
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
        header[0..2].copy_from_slice(
            &u16::try_from(self.levels[0].block)
                .unwrap_or(0)
                .to_le_bytes(),
        );
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
                Kind::I64 => Value::I64(u64_at(base + at)?.cast_signed()),
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
            eat(&u16::try_from(p).unwrap_or(0).to_le_bytes());
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
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

impl Persist {
    /// Record one row of an event table from fields known only at run time,
    /// such as data that arrives as JSON: the row a `tracing` event with
    /// these fields would make, in their order. Nothing is built when the
    /// table is off.
    pub fn record_row<'a>(
        &self,
        table: &str,
        fields: impl IntoIterator<Item = (&'a str, Value<'a>)>,
    ) {
        if !self.event_enabled(table) {
            return;
        }
        // ponytail: every row collects its fields into a Vec (one allocation)
        // and finds its shape by hashing the table and fields and scanning
        // the shapes; md records a row per public trade and per JSON data
        // message this way. A shape cached per call site, as `record_value`
        // keeps, would make a repeat row a lookup with no allocation.
        let fields: Vec<(&str, Value<'_>)> = fields.into_iter().collect();
        let Some(entry) = self.shape(table, fields.iter().map(|(n, v)| (*n, v.kind(), None)))
        else {
            self.drop_one();
            return;
        };
        if !self.send_shape(&entry) {
            return;
        }
        let text = fields
            .iter()
            .map(|(_, v)| match v {
                Value::Str(s) => s.len(),
                _ => 0,
            })
            .sum();
        let ts = self.now().0.cast_unsigned();
        let shape = &entry.shape;
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
    ) -> Option<Rc<ShapeEntry>> {
        let id = shape_id(table, fields.clone());
        // ponytail: a scan by id; an index when an application makes hundreds
        // of shapes.
        let mut shapes = self.inner.shapes.try_borrow_mut().ok()?;
        if let Some(entry) = shapes.iter().find(|e| e.shape.id == id) {
            let shape = &entry.shape;
            let same = shape.table == table
                && shape.fields.len() == fields.clone().count()
                && shape
                    .fields
                    .iter()
                    .zip(fields)
                    .all(|(f, (name, kind, parent))| {
                        f.name == name && f.kind == kind && f.parent == parent
                    });
            if same {
                return Some(Rc::clone(entry));
            }
            log::error!(
                "event shape id {id} of {} is also {table}'s: not recording that row",
                shape.table
            );
            return None;
        }
        let fields = fields.map(|(name, kind, parent)| FieldDef::new(name, kind, parent));
        match Shape::new(table, fields.collect()) {
            Ok(shape) => {
                let (len, max) = (shape.message().len(), self.max_payload());
                if len > max {
                    log::error!(
                        "event table {table}: its shape is {len} bytes, more than one claim holds ({max}): its rows are dropped"
                    );
                }
                let entry = Rc::new(ShapeEntry {
                    shape,
                    sent: Cell::new(false),
                });
                shapes.push(Rc::clone(&entry));
                Some(entry)
            }
            Err(e) => {
                log::error!("event table {table}: {e}; not recording that row");
                None
            }
        }
    }

    /// Put `entry`'s message in the stream unless it is there already.
    /// `false` when Aeron could not take it: rows of it must not follow.
    pub(crate) fn send_shape(&self, entry: &ShapeEntry) -> bool {
        if entry.sent.get() {
            return true;
        }
        let sent = self.publish(entry.shape.message());
        if sent {
            entry.sent.set(true);
        }
        sent
    }
}

/// Hashes the addresses and ids that key the call-site caches (the
/// `tracing` bridge's, `record_value`'s): they are unique already.
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

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

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
            (i % 2 == 1).then_some(Value::I64(i64::try_from(i).unwrap_or(i64::MAX)))
        });
        let decoded = shape.decode_row(&row).ok_or("undecodable")?;
        assert_eq!(values(&decoded, 69), [Some(Value::I64(69))]);
        assert_eq!(values(&decoded, 68), [None]);
        Ok(())
    }
}
