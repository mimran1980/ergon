//! Record any `Serialize` value as a row of an event table, nested as deep as
//! it goes: [`Persist::record_value`].
//!
//! | Rust                                    | in the row                           | ClickHouse column                 |
//! |-----------------------------------------|--------------------------------------|-----------------------------------|
//! | `bool`, integers, floats                | a value                              | `Nullable(Bool/Int64/UInt64/Float64)` |
//! | `str`, `String`, `char`, `i128`, `u128` | text                                 | `Nullable(String)`                |
//! | `Option<T>` (`None`), `()`              | absent                               | NULL                              |
//! | struct, tuple, newtype                  | its fields, dotted: `spread.bps`, `pair.0` | one column each             |
//! | `Vec`, slice, set, array                | a group: one entry per element       | `levels.price Array(…)`           |
//! | map                                     | a group of `key` and `value`         | `x.key`, `x.value` arrays         |
//! | `#[serde(flatten)]` field               | its fields, beside the others        | one column each                   |
//! | enum                                    | its variant's name, then its fields under `x.Variant` | `x`, `x.Variant.f` |
//!
//! A group inside a group is an `Array(Array(…))`. A list inside a struct
//! field is named with underscores up to it (`stats.bids` → `stats_bids`):
//! ClickHouse makes array columns that share a first name segment one
//! Nested structure, whose arrays must be equally long, and only one list's
//! own fields are sure to be. A value nested deeper than 8 lists, or 32
//! structs, is refused: a recursive type would otherwise add columns
//! without end.
//!
//! The first value of a type, recorded to a table, makes its [`Shape`]. After
//! that each value is written in one pass, compiled for its type, into a
//! reused buffer that also checks the shape covers it, then copied into
//! Aeron. A value the shape does not cover (an `Option` now `Some`, another
//! enum variant, a longer tuple) grows the shape, which keeps the order its
//! fields are visited in, and is sent again.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::hash::BuildHasherDefault;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde::ser::{self, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple};

use crate::Persist;
use crate::event::{
    AddressHasher, FieldDef, HEADER, Kind, NONE, ROW_START, Shape, Value, fnv64, now_ns,
};

/// Why a value could not be measured, learned or written.
#[derive(Debug)]
pub(crate) enum Problem {
    /// The shape does not cover the value: learn it.
    Misfit,
    /// A field is two kinds at once, or a kind persist does not record.
    Unrecordable(String),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Misfit => f.write_str("the shape does not cover the value"),
            Self::Unrecordable(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Problem {}

impl ser::Error for Problem {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Unrecordable(msg.to_string())
    }
}

/// The dotted name of the field being visited, relative to its row or
/// group entry. Reused, so visiting does not allocate.
#[derive(Default)]
struct Path {
    name: String,
    /// Where each pushed segment started.
    marks: Vec<usize>,
    /// Where each row or entry's names start.
    starts: Vec<usize>,
    /// The next index of each open tuple.
    tuples: Vec<usize>,
    /// Each open map: `true` when it is a struct's fields (a
    /// `#[serde(flatten)]`), `false` when a real map, a group.
    maps: Vec<bool>,
}

impl Path {
    fn reset(&mut self) {
        self.name.clear();
        self.marks.clear();
        self.starts.clear();
        self.starts.push(0);
        self.tuples.clear();
        self.maps.clear();
    }

    fn rel(&self) -> &str {
        let start = self.starts.last().copied().unwrap_or(0);
        let rel = &self.name[start..];
        rel.strip_prefix('.').unwrap_or(rel)
    }

    fn push(&mut self, key: &str) -> Result<(), Problem> {
        if self.marks.len() >= MAX_STRUCTS {
            return Err(Problem::Unrecordable(format!(
                "{}: nested deeper than {MAX_STRUCTS} structs (a recursive type?)",
                self.rel()
            )));
        }
        self.marks.push(self.name.len());
        self.name.push('.');
        self.name.push_str(key);
        Ok(())
    }

    fn pop(&mut self) {
        if let Some(mark) = self.marks.pop() {
            self.name.truncate(mark);
        }
    }

    fn enter(&mut self) {
        self.starts.push(self.name.len());
    }

    fn leave(&mut self) {
        self.starts.pop();
    }

    /// Push the next index of the innermost tuple.
    fn push_index(&mut self) -> Result<(), Problem> {
        let index = self.tuples.last().copied().unwrap_or(0);
        if let Some(next) = self.tuples.last_mut() {
            *next += 1;
        }
        let mut digits = [0u8; 20];
        let mut w = std::io::Cursor::new(&mut digits[..]);
        let _ = std::io::Write::write_fmt(&mut w, format_args!("{index}"));
        let len = w.position() as usize;
        self.push(std::str::from_utf8(&digits[..len]).unwrap_or("0"))
    }

    /// Check a list may open here.
    fn open_group(&self) -> Result<(), Problem> {
        // `starts` holds the row and each open list entry.
        if self.starts.len() > MAX_GROUPS {
            return Err(Problem::Unrecordable(format!(
                "{}: nested deeper than {MAX_GROUPS} lists (a recursive type?)",
                self.rel()
            )));
        }
        Ok(())
    }
}

/// Deepest a value may nest lists, and structs.
const MAX_GROUPS: usize = 8;
const MAX_STRUCTS: usize = 32;

/// What a visit does with what it finds.
trait Sink {
    fn leaf(&mut self, name: &str, value: Value<'_>) -> Result<(), Problem>;
    fn begin_group(&mut self, name: &str) -> Result<(), Problem>;
    fn begin_entry(&mut self) -> Result<(), Problem>;
    fn end_entry(&mut self) -> Result<(), Problem>;
    fn end_group(&mut self) -> Result<(), Problem>;
}

/// Visits a value for a [`Sink`]: the one place that decides how Rust maps
/// onto rows (see the module's table).
struct Visit<'a, S> {
    sink: &'a mut S,
    path: &'a mut Path,
}

impl<S: Sink> Visit<'_, S> {
    fn leaf(&mut self, value: Value<'_>) -> Result<(), Problem> {
        self.sink.leaf(self.path.rel(), value)
    }

    fn text(&mut self, value: impl std::fmt::Display) -> Result<(), Problem> {
        let mut text = [0u8; 48];
        let mut w = std::io::Cursor::new(&mut text[..]);
        std::io::Write::write_fmt(&mut w, format_args!("{value}"))
            .map_err(|e| Problem::Unrecordable(e.to_string()))?;
        let len = w.position() as usize;
        let text =
            std::str::from_utf8(&text[..len]).map_err(|e| Problem::Unrecordable(e.to_string()))?;
        self.leaf(Value::Str(text))
    }
}

impl<'r, 'a, S: Sink> ser::Serializer for &'r mut Visit<'a, S> {
    type Ok = ();
    type Error = Problem;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), Problem> {
        self.leaf(Value::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<(), Problem> {
        self.leaf(Value::I64(v.into()))
    }
    fn serialize_i16(self, v: i16) -> Result<(), Problem> {
        self.leaf(Value::I64(v.into()))
    }
    fn serialize_i32(self, v: i32) -> Result<(), Problem> {
        self.leaf(Value::I64(v.into()))
    }
    fn serialize_i64(self, v: i64) -> Result<(), Problem> {
        self.leaf(Value::I64(v))
    }
    fn serialize_i128(self, v: i128) -> Result<(), Problem> {
        self.text(v)
    }
    fn serialize_u8(self, v: u8) -> Result<(), Problem> {
        self.leaf(Value::U64(v.into()))
    }
    fn serialize_u16(self, v: u16) -> Result<(), Problem> {
        self.leaf(Value::U64(v.into()))
    }
    fn serialize_u32(self, v: u32) -> Result<(), Problem> {
        self.leaf(Value::U64(v.into()))
    }
    fn serialize_u64(self, v: u64) -> Result<(), Problem> {
        self.leaf(Value::U64(v))
    }
    fn serialize_u128(self, v: u128) -> Result<(), Problem> {
        self.text(v)
    }
    fn serialize_f32(self, v: f32) -> Result<(), Problem> {
        self.leaf(Value::F64(v.into()))
    }
    fn serialize_f64(self, v: f64) -> Result<(), Problem> {
        self.leaf(Value::F64(v))
    }
    fn serialize_char(self, v: char) -> Result<(), Problem> {
        self.leaf(Value::Str(v.encode_utf8(&mut [0; 4])))
    }
    fn serialize_str(self, v: &str) -> Result<(), Problem> {
        self.leaf(Value::Str(v))
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), Problem> {
        Err(Problem::Unrecordable(format!(
            "{}: raw bytes are not recorded; use a String or Vec<u8>",
            self.path.rel()
        )))
    }
    fn serialize_none(self) -> Result<(), Problem> {
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Problem> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Problem> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Problem> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Problem> {
        self.leaf(Value::Str(variant))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Problem> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Problem> {
        self.leaf(Value::Str(variant))?;
        self.path.push(variant)?;
        value.serialize(&mut *self)?;
        self.path.pop();
        Ok(())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self, Problem> {
        self.path.open_group()?;
        self.sink.begin_group(self.path.rel())?;
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, Problem> {
        self.path.tuples.push(0);
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Problem> {
        self.path.tuples.push(0);
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self, Problem> {
        self.leaf(Value::Str(variant))?;
        self.path.push(variant)?;
        self.path.tuples.push(0);
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self, Problem> {
        // A map of unknown length is a struct with a `#[serde(flatten)]`
        // field: its keys are field names. Real maps know their length.
        let fields = len.is_none();
        self.path.maps.push(fields);
        if !fields {
            self.path.open_group()?;
            self.sink.begin_group(self.path.rel())?;
        }
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Problem> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self, Problem> {
        self.leaf(Value::Str(variant))?;
        self.path.push(variant)?;
        Ok(self)
    }
}

impl<S: Sink> SerializeSeq for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Problem> {
        self.sink.begin_entry()?;
        self.path.enter();
        value.serialize(&mut **self)?;
        self.path.leave();
        self.sink.end_entry()
    }
    fn end(self) -> Result<(), Problem> {
        self.sink.end_group()
    }
}

impl<S: Sink> SerializeTuple for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Problem> {
        self.path.push_index()?;
        value.serialize(&mut **self)?;
        self.path.pop();
        Ok(())
    }
    fn end(self) -> Result<(), Problem> {
        self.path.tuples.pop();
        Ok(())
    }
}

impl<S: Sink> ser::SerializeTupleStruct for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Problem> {
        SerializeTuple::serialize_element(self, value)
    }
    fn end(self) -> Result<(), Problem> {
        SerializeTuple::end(self)
    }
}

impl<S: Sink> ser::SerializeTupleVariant for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Problem> {
        SerializeTuple::serialize_element(self, value)
    }
    fn end(self) -> Result<(), Problem> {
        self.path.tuples.pop();
        self.path.pop();
        Ok(())
    }
}

impl<S: Sink> SerializeMap for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Problem> {
        if self.path.maps.last() == Some(&true) {
            // A field name: the key itself.
            let mut name = FieldName(String::new());
            key.serialize(&mut name)?;
            return self.path.push(&name.0);
        }
        self.sink.begin_entry()?;
        self.path.enter();
        self.path.push("key")?;
        key.serialize(&mut **self)?;
        self.path.pop();
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Problem> {
        if self.path.maps.last() == Some(&true) {
            value.serialize(&mut **self)?;
            self.path.pop();
            return Ok(());
        }
        self.path.push("value")?;
        value.serialize(&mut **self)?;
        self.path.pop();
        self.path.leave();
        self.sink.end_entry()
    }
    fn end(self) -> Result<(), Problem> {
        if self.path.maps.pop() == Some(true) {
            Ok(())
        } else {
            self.sink.end_group()
        }
    }
}

/// A flattened struct's field name, from its map key.
struct FieldName(String);

impl ser::Serializer for &mut FieldName {
    type Ok = ();
    type Error = Problem;
    type SerializeSeq = ser::Impossible<(), Problem>;
    type SerializeTuple = ser::Impossible<(), Problem>;
    type SerializeTupleStruct = ser::Impossible<(), Problem>;
    type SerializeTupleVariant = ser::Impossible<(), Problem>;
    type SerializeMap = ser::Impossible<(), Problem>;
    type SerializeStruct = ser::Impossible<(), Problem>;
    type SerializeStructVariant = ser::Impossible<(), Problem>;

    fn serialize_str(self, v: &str) -> Result<(), Problem> {
        self.0.push_str(v);
        Ok(())
    }
    fn serialize_bool(self, v: bool) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_i8(self, v: i8) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_i16(self, v: i16) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_i32(self, v: i32) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_i64(self, v: i64) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_u8(self, v: u8) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_u16(self, v: u16) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_u32(self, v: u32) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_u64(self, v: u64) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_f32(self, v: f32) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_f64(self, v: f64) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_char(self, v: char) -> Result<(), Problem> {
        self.name(v)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Problem> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Problem> {
        value.serialize(self)
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), Problem> {
        Err(not_a_name())
    }
    fn serialize_none(self) -> Result<(), Problem> {
        Err(not_a_name())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Problem> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Problem> {
        Err(not_a_name())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Problem> {
        Err(not_a_name())
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), Problem> {
        Err(not_a_name())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Problem> {
        Err(not_a_name())
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Problem> {
        Err(not_a_name())
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Problem> {
        Err(not_a_name())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Problem> {
        Err(not_a_name())
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Problem> {
        Err(not_a_name())
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Problem> {
        Err(not_a_name())
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Problem> {
        Err(not_a_name())
    }
}

impl FieldName {
    fn name(&mut self, v: impl std::fmt::Display) -> Result<(), Problem> {
        write!(self.0, "{v}").map_err(|e| Problem::Unrecordable(e.to_string()))
    }
}

fn not_a_name() -> Problem {
    Problem::Unrecordable("a flattened map key that is not a string or number".into())
}

impl<S: Sink> SerializeStruct for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Problem> {
        self.path.push(key)?;
        value.serialize(&mut **self)?;
        self.path.pop();
        Ok(())
    }
    fn end(self) -> Result<(), Problem> {
        Ok(())
    }
}

impl<S: Sink> ser::SerializeStructVariant for &mut Visit<'_, S> {
    type Ok = ();
    type Error = Problem;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Problem> {
        SerializeStruct::serialize_field(self, key, value)
    }
    fn end(self) -> Result<(), Problem> {
        self.path.pop();
        Ok(())
    }
}

// ---------------------------------------------------------------- learning

/// One level of a value's structure (the row, or a group's entries): its
/// fields in the order they were visited.
#[derive(Clone, Debug, Default, PartialEq)]
struct Tree(Vec<Node>);

#[derive(Clone, Debug, PartialEq)]
struct Node {
    name: String,
    kind: Kind,
    /// A group's entries.
    entries: Tree,
}

impl Tree {
    /// Add a field, or check it against the one already there.
    fn add(&mut self, name: &str, kind: Kind) -> Result<&mut Node, Problem> {
        let at = match self.0.iter().position(|n| n.name == name) {
            Some(at) if self.0[at].kind == kind => at,
            Some(at) => {
                return Err(Problem::Unrecordable(format!(
                    "field {name:?} is both {:?} and {kind:?}",
                    self.0[at].kind
                )));
            }
            None => {
                self.0.push(Node {
                    name: name.to_owned(),
                    kind,
                    entries: Tree::default(),
                });
                self.0.len() - 1
            }
        };
        Ok(&mut self.0[at])
    }

    /// Both structures in one, each keeping its order: a field only `other`
    /// has goes where `other` has it, relative to the fields both have.
    fn merge(self, other: Self) -> Result<Self, Problem> {
        let mut mine: VecDeque<Node> = self.0.into();
        let mut out = Vec::with_capacity(mine.len() + other.0.len());
        for theirs in other.0 {
            match mine.iter().position(|n| n.name == theirs.name) {
                Some(at) => {
                    out.extend(mine.drain(..at));
                    let Some(node) = mine.pop_front() else {
                        continue;
                    };
                    if node.kind != theirs.kind {
                        return Err(Problem::Unrecordable(format!(
                            "field {:?} is both {:?} and {:?}",
                            node.name, node.kind, theirs.kind
                        )));
                    }
                    out.push(Node {
                        entries: node.entries.merge(theirs.entries)?,
                        ..node
                    });
                }
                None => out.push(theirs),
            }
        }
        out.extend(mine);
        Ok(Self(out))
    }

    /// The fields of a shape, as a structure.
    fn of(shape: &Shape) -> Self {
        fn level(shape: &Shape, parent: Option<usize>) -> Tree {
            Tree(
                shape
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.parent == parent)
                    .map(|(i, f)| Node {
                        name: f.name.clone(),
                        kind: f.kind,
                        entries: if f.kind == Kind::Group {
                            level(shape, Some(i))
                        } else {
                            Tree::default()
                        },
                    })
                    .collect(),
            )
        }
        level(shape, None)
    }

    /// The structure as a shape's fields: each group followed by its entries' fields.
    fn fields(&self, parent: Option<usize>, out: &mut Vec<FieldDef>) {
        for node in &self.0 {
            let i = out.len();
            out.push(FieldDef::new(node.name.clone(), node.kind, parent));
            if node.kind == Kind::Group {
                node.entries.fields(Some(i), out);
            }
        }
    }
}

/// Learns a value's structure.
#[derive(Default)]
struct Learner {
    /// The row, then each open group entry.
    levels: Vec<Tree>,
    /// Each open group: its name, and its entries' structure so far.
    groups: Vec<(String, Tree)>,
}

impl Sink for Learner {
    fn leaf(&mut self, name: &str, value: Value<'_>) -> Result<(), Problem> {
        let level = self.levels.last_mut().ok_or(Problem::Misfit)?;
        level.add(name, value.kind()).map(|_| ())
    }

    fn begin_group(&mut self, name: &str) -> Result<(), Problem> {
        let level = self.levels.last_mut().ok_or(Problem::Misfit)?;
        level.add(name, Kind::Group)?;
        self.groups.push((name.to_owned(), Tree::default()));
        Ok(())
    }

    fn begin_entry(&mut self) -> Result<(), Problem> {
        self.levels.push(Tree::default());
        Ok(())
    }

    fn end_entry(&mut self) -> Result<(), Problem> {
        let entry = self.levels.pop().ok_or(Problem::Misfit)?;
        let (_, entries) = self.groups.last_mut().ok_or(Problem::Misfit)?;
        *entries = std::mem::take(entries).merge(entry)?;
        Ok(())
    }

    fn end_group(&mut self) -> Result<(), Problem> {
        let (name, entries) = self.groups.pop().ok_or(Problem::Misfit)?;
        let level = self.levels.last_mut().ok_or(Problem::Misfit)?;
        let node = level.add(&name, Kind::Group)?;
        node.entries = std::mem::take(&mut node.entries).merge(entries)?;
        Ok(())
    }
}

/// The structure of `value`, merged into `old`'s.
fn learn<T: ?Sized + Serialize>(
    value: &T,
    old: Option<&Shape>,
    path: &mut Path,
) -> Result<Vec<FieldDef>, Problem> {
    path.reset();
    let mut learner = Learner {
        levels: vec![Tree::default()],
        groups: Vec::new(),
    };
    value.serialize(&mut Visit {
        sink: &mut learner,
        path,
    })?;
    let tree = learner.levels.pop().unwrap_or_default();
    let tree = match old {
        Some(old) => Tree::of(old).merge(tree)?,
        None => tree,
    };
    let mut fields = Vec::new();
    tree.fields(None, &mut fields);
    Ok(fields)
}

// ---------------------------------------------------------------- writing

/// One row or group entry being written.
struct Frame {
    level: usize,
    /// The next of its fields that may come.
    cursor: usize,
    /// Where its block starts.
    base: usize,
}

/// What writing a row reuses, so it does not allocate once warm.
#[derive(Default)]
struct Scratch {
    path: Path,
    row: Vec<u8>,
    frames: Vec<Frame>,
    /// Each open group: where its count goes, its entries so far, its entries' level.
    groups: Vec<(usize, u32, usize)>,
}

/// Writes a value's row against a shape, in one pass. The fields must come
/// in the shape's order; any it skips are absent.
struct Walker<'s, 'b> {
    shape: &'s Shape,
    row: &'b mut Vec<u8>,
    frames: &'b mut Vec<Frame>,
    groups: &'b mut Vec<(usize, u32, usize)>,
}

impl Walker<'_, '_> {
    /// The field `name` of `kind`, next in the current row or entry, after
    /// marking the fields skipped to reach it absent. Returns its position
    /// and index.
    fn take(&mut self, name: &str, kind: Kind) -> Result<(usize, usize), Problem> {
        let shape = self.shape;
        let frame = self.frames.last().ok_or(Problem::Misfit)?;
        let level = &shape.levels[frame.level];
        let p = (frame.cursor..level.fields.len())
            .find(|&p| shape.fields[level.fields[p]].name == name)
            .ok_or(Problem::Misfit)?;
        let f = level.fields[p];
        if shape.fields[f].kind != kind {
            return Err(Problem::Misfit);
        }
        self.skip(p)?;
        let frame = self.frames.last_mut().ok_or(Problem::Misfit)?;
        frame.cursor = p + 1;
        self.row[frame.base + level.presence + p / 8] |= 1 << (p % 8);
        Ok((p, f))
    }

    /// Mark the current row or entry's fields before `until` absent: a
    /// `Str` or group takes an empty length or count.
    fn skip(&mut self, until: usize) -> Result<(), Problem> {
        let frame = self.frames.last().ok_or(Problem::Misfit)?;
        let level = &self.shape.levels[frame.level];
        for p in frame.cursor..until {
            if level.offsets[p] == NONE {
                self.row.extend_from_slice(&[0; 4]);
            }
        }
        Ok(())
    }

    /// Start a row or entry of `level` at the end of the row.
    fn begin_level(&mut self, level: usize) {
        let base = self.row.len();
        self.row.resize(base + self.shape.levels[level].block, 0);
        self.frames.push(Frame {
            level,
            cursor: 0,
            base,
        });
    }

    fn end_level(&mut self) -> Result<(), Problem> {
        let len = self
            .frames
            .last()
            .map(|f| self.shape.levels[f.level].fields.len());
        self.skip(len.ok_or(Problem::Misfit)?)?;
        self.frames.pop();
        Ok(())
    }
}

impl Sink for Walker<'_, '_> {
    fn leaf(&mut self, name: &str, value: Value<'_>) -> Result<(), Problem> {
        let (p, _) = self.take(name, value.kind())?;
        // A fixed value's place in its block (`Str` has none: it is appended).
        let at = || {
            let frame = self.frames.last()?;
            Some(frame.base + self.shape.levels[frame.level].offsets[p])
        };
        let (at, bytes) = match value {
            Value::Str(s) => {
                let len = u32::try_from(s.len()).map_err(|_| Problem::Misfit)?;
                self.row.extend_from_slice(&len.to_le_bytes());
                self.row.extend_from_slice(s.as_bytes());
                return Ok(());
            }
            Value::Bool(v) => {
                let at = at().ok_or(Problem::Misfit)?;
                self.row[at] = u8::from(v);
                return Ok(());
            }
            Value::I64(v) => (at(), v.to_le_bytes()),
            Value::U64(v) => (at(), v.to_le_bytes()),
            Value::F64(v) => (at(), v.to_le_bytes()),
        };
        let at = at.ok_or(Problem::Misfit)?;
        self.row[at..at + 8].copy_from_slice(&bytes);
        Ok(())
    }

    fn begin_group(&mut self, name: &str) -> Result<(), Problem> {
        let (_, f) = self.take(name, Kind::Group)?;
        self.groups.push((self.row.len(), 0, self.shape.entries[f]));
        self.row.extend_from_slice(&[0; 4]);
        Ok(())
    }

    fn begin_entry(&mut self) -> Result<(), Problem> {
        let (_, count, level) = self.groups.last_mut().ok_or(Problem::Misfit)?;
        *count += 1;
        let level = *level;
        self.begin_level(level);
        Ok(())
    }

    fn end_entry(&mut self) -> Result<(), Problem> {
        self.end_level()
    }

    fn end_group(&mut self) -> Result<(), Problem> {
        let (at, count, _) = self.groups.pop().ok_or(Problem::Misfit)?;
        self.row[at..at + 4].copy_from_slice(&count.to_le_bytes());
        Ok(())
    }
}

/// Write `value`'s row into `scratch.row`, recorded at `ts`.
/// [`Problem::Misfit`] when the shape does not cover it.
fn walk<T: ?Sized + Serialize>(
    shape: &Shape,
    value: &T,
    ts: u64,
    scratch: &mut Scratch,
) -> Result<(), Problem> {
    let Scratch {
        path,
        row,
        frames,
        groups,
    } = scratch;
    path.reset();
    row.clear();
    frames.clear();
    groups.clear();
    row.resize(HEADER, 0);
    shape.write_header(row);
    let mut walker = Walker {
        shape,
        row,
        frames,
        groups,
    };
    walker.begin_level(0);
    walker.row[HEADER..HEADER + 4].copy_from_slice(&shape.id.to_le_bytes());
    walker.row[HEADER + 4..HEADER + ROW_START].copy_from_slice(&ts.to_le_bytes());
    value.serialize(&mut Visit {
        sink: &mut walker,
        path,
    })?;
    walker.end_level()
}

// ------------------------------------------------------------- recording

/// What recording one type into one table needs, cached per thread.
struct Site {
    table: String,
    switch: Arc<AtomicBool>,
    shape: Option<Arc<Shape>>,
    /// Why its values cannot be recorded, logged once, and when: it is not
    /// learned again for a second, so a type that cannot be recorded costs
    /// no more than a dropped record.
    broken: Option<(String, std::time::Instant)>,
}

#[derive(Default)]
struct Sites {
    /// Keyed by the `Persist`'s id, the type's name and the table's hash.
    sites: HashMap<(usize, usize, u64), Site, BuildHasherDefault<AddressHasher>>,
    scratch: Scratch,
}

thread_local! {
    static SITES: RefCell<Sites> = RefCell::default();
}

impl Persist {
    /// Record `value`, of any `Serialize` type however nested, as one row
    /// of the event table `table` (see the module docs for how it maps).
    /// Nothing is visited when the table is off.
    pub fn record_value<T: ?Sized + Serialize>(&self, table: &str, value: &T) {
        let key = (
            self.inner.id as usize,
            std::any::type_name::<T>().as_ptr() as usize,
            fnv64(table.as_bytes()),
        );
        let _ = SITES.with(|cell| {
            let mut sites = cell.try_borrow_mut()?;
            let Sites { sites, scratch } = &mut *sites;
            let site = sites.entry(key).or_insert_with(|| Site {
                table: String::new(),
                switch: Arc::new(AtomicBool::new(false)),
                shape: None,
                broken: None,
            });
            if site.table != table {
                table.clone_into(&mut site.table);
                site.switch = self.event_switch(table);
                site.shape = None;
                site.broken = None;
            }
            if site.switch.load(Ordering::Relaxed) {
                self.record_at(site, value, scratch);
            }
            Ok::<_, std::cell::BorrowMutError>(())
        });
    }

    fn record_at<T: ?Sized + Serialize>(&self, site: &mut Site, value: &T, scratch: &mut Scratch) {
        let ts = now_ns();
        let written = match &site.shape {
            Some(shape) => walk(shape, value, ts, scratch),
            None => Err(Problem::Misfit),
        };
        match written {
            Ok(()) => {}
            Err(Problem::Misfit) => {
                let cooling = site
                    .broken
                    .as_ref()
                    .is_some_and(|(_, at)| at.elapsed() < std::time::Duration::from_secs(1));
                if cooling || !self.grow(site, value, ts, scratch) {
                    self.drop_one();
                    return;
                }
            }
            Err(problem) => {
                self.broken(site, &problem);
                self.drop_one();
                return;
            }
        }
        let sent = site.shape.as_deref().is_some_and(|s| self.send_shape(s));
        if !(sent && self.publish(&scratch.row)) {
            self.drop_one();
        }
    }

    /// Learn `value` into `site`'s shape, and write it again. `false`,
    /// logged, when it cannot be recorded.
    #[cold]
    fn grow<T: ?Sized + Serialize>(
        &self,
        site: &mut Site,
        value: &T,
        ts: u64,
        scratch: &mut Scratch,
    ) -> bool {
        let written = learn(value, site.shape.as_deref(), &mut scratch.path).and_then(|fields| {
            let shape = self
                .shape(
                    &site.table,
                    fields.iter().map(|f| (f.name.as_str(), f.kind, f.parent)),
                )
                .ok_or(Problem::Unrecordable(
                    "no shape for it (see the error above)".into(),
                ))?;
            let written = walk(&shape, value, ts, scratch);
            site.shape = Some(shape);
            written
        });
        match written {
            Ok(()) => true,
            Err(problem) => {
                self.broken(site, &problem);
                false
            }
        }
    }

    #[cold]
    fn broken(&self, site: &mut Site, problem: &Problem) {
        let message = problem.to_string();
        if site.broken.as_ref().map(|(m, _)| m) != Some(&message) {
            log::error!(
                "event table {}: {message}; not recording those rows",
                site.table
            );
        }
        site.broken = Some((message, std::time::Instant::now()));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::event::Column;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[derive(Serialize)]
    struct Level {
        price: f64,
        size: u64,
        orders: Vec<u32>,
    }

    #[derive(Serialize)]
    enum Regime {
        Calm,
        Volatile { vol: f64 },
    }

    #[derive(Serialize)]
    struct Book<'a> {
        symbol: &'a str,
        spread: Spread,
        bids: Vec<Level>,
        regime: Regime,
        note: Option<String>,
        pair: (i32, &'a str),
        tags: BTreeMap<&'a str, i64>,
    }

    #[derive(Serialize)]
    struct Spread {
        bps: f64,
    }

    fn book(regime: Regime, note: Option<String>) -> Book<'static> {
        Book {
            symbol: "BTCUSDT",
            spread: Spread { bps: 1.5 },
            bids: vec![
                Level {
                    price: 100.5,
                    size: 2,
                    orders: vec![7, 8],
                },
                Level {
                    price: 100.0,
                    size: 1,
                    orders: vec![],
                },
            ],
            regime,
            note,
            pair: (-3, "x"),
            tags: BTreeMap::from([("a", 1), ("b", 2)]),
        }
    }

    /// Learn `value` onto `old`, and write its row.
    fn record<T: Serialize>(
        old: Option<&Shape>,
        value: &T,
    ) -> Result<(Shape, Vec<u8>), Box<dyn std::error::Error>> {
        let mut scratch = Scratch::default();
        let shape = Shape::new("book", learn(value, old, &mut scratch.path)?)?;
        walk(&shape, value, 42, &mut scratch)?;
        Ok((shape, scratch.row))
    }

    fn column<'a>(
        shape: &Shape,
        decoded: &'a crate::event::Decoded<'a>,
        name: &str,
    ) -> Option<&'a Column<'a>> {
        (0..shape.fields.len())
            .find(|&i| shape.column(i) == name && shape.fields[i].kind != Kind::Group)
            .map(|i| &decoded.columns[i])
    }

    #[test]
    fn a_nested_value_round_trips_column_by_column() -> TestResult {
        let (shape, row) = record(None, &book(Regime::Volatile { vol: 0.4 }, None))?;
        let columns: Vec<&str> = (0..shape.fields.len()).map(|i| shape.column(i)).collect();
        assert_eq!(
            columns,
            [
                "symbol",
                "spread.bps",
                "bids",
                "bids.price",
                "bids.size",
                "bids.orders",
                "bids.orders",
                "regime",
                "regime.Volatile.vol",
                "pair.0",
                "pair.1",
                "tags",
                "tags.key",
                "tags.value",
            ]
        );
        let decoded = shape.decode_row(&row).ok_or("undecodable")?;
        assert_eq!(decoded.ts, 42);
        let values = |name: &str| match column(&shape, &decoded, name) {
            Some(Column::Values(v)) => v.clone(),
            _ => Vec::new(),
        };
        assert_eq!(values("symbol"), [Some(Value::Str("BTCUSDT"))]);
        assert_eq!(values("spread.bps"), [Some(Value::F64(1.5))]);
        assert_eq!(
            values("bids.price"),
            [Some(Value::F64(100.5)), Some(Value::F64(100.0))]
        );
        assert_eq!(
            values("bids.orders"),
            [Some(Value::U64(7)), Some(Value::U64(8))]
        );
        assert_eq!(values("regime"), [Some(Value::Str("Volatile"))]);
        assert_eq!(values("regime.Volatile.vol"), [Some(Value::F64(0.4))]);
        assert_eq!(values("pair.0"), [Some(Value::I64(-3))]);
        assert_eq!(
            values("tags.key"),
            [Some(Value::Str("a")), Some(Value::Str("b"))]
        );
        assert_eq!(
            values("tags.value"),
            [Some(Value::I64(1)), Some(Value::I64(2))]
        );
        // The group counts: 2 bids; their orders 2 and 0.
        let counts: Vec<&Column<'_>> = (0..shape.fields.len())
            .filter(|&i| shape.fields[i].kind == Kind::Group)
            .map(|i| &decoded.columns[i])
            .collect();
        assert_eq!(counts[0], &Column::Counts(vec![2]));
        assert_eq!(counts[1], &Column::Counts(vec![2, 0]));
        Ok(())
    }

    #[test]
    fn a_value_the_shape_does_not_cover_grows_it_in_order() -> TestResult {
        let (first, _) = record(None, &book(Regime::Calm, None))?;
        let mut scratch = Scratch::default();
        let later = book(Regime::Volatile { vol: 0.4 }, Some("n".into()));
        assert!(matches!(
            walk(&first, &later, 1, &mut scratch),
            Err(Problem::Misfit)
        ));
        let (grown, row) = record(Some(&first), &later)?;
        // New fields sit where the type declares them, so one sequential
        // pass writes them.
        let order: Vec<&str> = grown.fields.iter().map(|f| f.name.as_str()).collect();
        let at = |n: &str| {
            order
                .iter()
                .position(|o| *o == n)
                .ok_or(format!("no {n} in {order:?}"))
        };
        assert!(at("regime")? < at("regime.Volatile.vol")?);
        assert!(at("regime.Volatile.vol")? < at("note")?);
        assert!(at("note")? < at("pair.0")?);
        assert!(grown.decode_row(&row).is_some());
        // And the first value still fits the grown shape, with the new fields absent.
        assert!(walk(&grown, &book(Regime::Calm, None), 1, &mut scratch).is_ok());
        Ok(())
    }

    fn columns(shape: &Shape) -> Vec<&str> {
        (0..shape.fields.len())
            .filter(|&i| shape.fields[i].kind != Kind::Group)
            .map(|i| shape.column(i))
            .collect()
    }

    #[test]
    fn serde_attributes_real_structs_use() -> TestResult {
        #[derive(Serialize)]
        struct Inner {
            b: f64,
            c: &'static str,
        }
        #[derive(Serialize)]
        #[serde(tag = "type")]
        enum Tagged {
            Calm,
            Wide { bps: f64 },
        }
        #[derive(Serialize)]
        struct Outer {
            a: i64,
            #[serde(flatten)]
            inner: Inner,
            regime: Tagged,
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<&'static str>,
            #[serde(rename = "renamed")]
            x: u8,
        }
        let value = Outer {
            a: 1,
            inner: Inner { b: 2.5, c: "c" },
            regime: Tagged::Wide { bps: 3.0 },
            note: None,
            x: 4,
        };
        let (shape, row) = record(None, &value)?;
        // Flattened fields sit beside the others; an internally tagged enum
        // is its tag and its fields.
        assert_eq!(
            columns(&shape),
            ["a", "b", "c", "regime.type", "regime.bps", "renamed"]
        );
        assert!(shape.decode_row(&row).is_some());
        let calm = Outer {
            regime: Tagged::Calm,
            note: Some("n"),
            ..value
        };
        let (grown, _) = record(Some(&shape), &calm)?;
        assert!(columns(&grown).contains(&"note"));
        Ok(())
    }

    #[test]
    fn a_list_inside_a_struct_is_named_to_stand_alone() -> TestResult {
        #[derive(Serialize)]
        struct Stats {
            count: u32,
            bids: Vec<f64>,
            asks: Vec<f64>,
        }
        #[derive(Serialize)]
        struct Row {
            stats: Stats,
        }
        let (shape, _) = record(
            None,
            &Row {
                stats: Stats {
                    count: 1,
                    bids: vec![1.0],
                    asks: vec![2.0, 3.0],
                },
            },
        )?;
        // Two lists of different lengths: two first segments, so ClickHouse
        // does not take them for one Nested structure.
        assert_eq!(columns(&shape), ["stats.count", "stats_bids", "stats_asks"]);
        Ok(())
    }

    #[test]
    fn a_recursive_type_is_recorded_to_its_depth_and_refused_past_the_limit() -> TestResult {
        #[derive(Serialize)]
        struct Node {
            value: i64,
            children: Vec<Node>,
        }
        fn tree(depth: usize) -> Node {
            Node {
                value: depth as i64,
                children: if depth == 0 {
                    vec![]
                } else {
                    vec![tree(depth - 1)]
                },
            }
        }
        let (shape, row) = record(None, &tree(2))?;
        assert_eq!(
            columns(&shape),
            ["value", "children.value", "children.children.value"]
        );
        assert!(shape.decode_row(&row).is_some());
        let mut scratch = Scratch::default();
        let err = learn(&tree(MAX_GROUPS + 1), None, &mut scratch.path)
            .err()
            .ok_or("recorded past the limit")?;
        assert!(
            err.to_string().contains("nested deeper than 8 lists"),
            "{err}"
        );
        Ok(())
    }

    #[test]
    fn unrecordable_values_are_named() -> TestResult {
        #[derive(Serialize)]
        struct Raw<'a> {
            #[serde(with = "serde_bytes_like")]
            blob: &'a [u8],
        }
        mod serde_bytes_like {
            pub fn serialize<S: serde::Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(v)
            }
        }
        let mut path = Path::default();
        let err = learn(&Raw { blob: b"x" }, None, &mut path)
            .err()
            .ok_or("recorded bytes")?;
        assert!(err.to_string().contains("blob: raw bytes"), "{err}");

        #[derive(Serialize)]
        #[serde(untagged)]
        enum Either {
            Int(i64),
            Text(String),
        }
        let (shape, _) = record(None, &vec![Either::Int(1)])?;
        let err = learn(&vec![Either::Text("x".into())], Some(&shape), &mut path)
            .err()
            .ok_or("merged two kinds")?;
        assert!(err.to_string().contains("is both I64 and Str"), "{err}");
        Ok(())
    }
}
