//! Tables fed by `tracing` events and `record_row` (`persist_client::event`).
//! There is no schema: the columns are the fields of the rows' shapes, each
//! typed by the first shape that has it, after a `ts DateTime64(9, 'UTC')`
//! column.
//!
//! | field value              | ClickHouse          |
//! |--------------------------|---------------------|
//! | signed integer           | `Nullable(Int64)`   |
//! | unsigned integer         | `Nullable(UInt64)`  |
//! | float                    | `Nullable(Float64)` |
//! | bool                     | `Nullable(Bool)`    |
//! | `&str`, `%x`, `?x`       | `Nullable(String)`  |
//!
//! A row without a column's field writes NULL. A value of another type is
//! converted when that loses nothing (an integer into a `Float64` or `String`
//! column); one that cannot be writes NULL and is counted, and the count is
//! reported as an error.

use std::collections::HashMap;

use persist_client::event::{Column as EventColumn, Decoded, Kind, Shape as EventShape, Value};

use crate::table::{Column, DecodeError, Shape, write_string, write_varint};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Type {
    I64,
    U64,
    F64,
    Bool,
    Str,
}

impl Type {
    const fn of(kind: Kind) -> Self {
        match kind {
            Kind::I64 => Self::I64,
            Kind::U64 => Self::U64,
            Kind::F64 => Self::F64,
            Kind::Bool => Self::Bool,
            // A group is no column of its own: its fields are (see `learn`).
            Kind::Str | Kind::Group => Self::Str,
        }
    }

    fn clickhouse(self) -> &'static str {
        match self {
            Self::I64 => "Nullable(Int64)",
            Self::U64 => "Nullable(UInt64)",
            Self::F64 => "Nullable(Float64)",
            Self::Bool => "Nullable(Bool)",
            Self::Str => "Nullable(String)",
        }
    }
}

/// One event table and the columns seen so far: name, type, and how many
/// groups deep it is (each one an `Array` around it).
#[derive(Clone, Debug)]
pub(crate) struct EventTable {
    pub(crate) name: String,
    columns: Vec<(String, Type, usize)>,
}

impl EventTable {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
        }
    }

    /// Add the columns of `shape` not seen before; `true` if there were any.
    pub(crate) fn learn(&mut self, shape: &EventShape) -> bool {
        let before = self.columns.len();
        for (i, f) in shape.fields.iter().enumerate() {
            let name = shape.column(i);
            if f.kind != Kind::Group && !self.columns.iter().any(|(c, ..)| c == name) {
                let depth = shape.groups_around(i).len();
                self.columns
                    .push((name.to_owned(), Type::of(f.kind), depth));
            }
        }
        self.columns.len() > before
    }

    /// `None` until a row has shown its columns.
    pub(crate) fn shape(&self) -> Option<Shape> {
        if self.columns.is_empty() {
            return None;
        }
        let ts = Column {
            name: "ts".into(),
            ch_type: "DateTime64(9, 'UTC')".into(),
        };
        let fields = self.columns.iter().map(|(name, ty, depth)| Column {
            name: name.clone(),
            ch_type: format!(
                "{}{}{}",
                "Array(".repeat(*depth),
                ty.clickhouse(),
                ")".repeat(*depth)
            ),
        });
        Some(Shape {
            name: self.name.clone(),
            columns: std::iter::once(ts).chain(fields).collect(),
            order_by: vec!["ts".into()],
            partition: Some("ts".into()),
        })
    }

    /// Append `row` as one RowBinary row of the columns whose `include` flag
    /// is set. Returns how many values did not fit their column's type, or
    /// sat at another depth than their column.
    pub(crate) fn write_row(
        &self,
        row: &[u8],
        include: &[bool],
        out: &mut Vec<u8>,
        shapes: &HashMap<u32, EventShape>,
    ) -> Result<usize, DecodeError> {
        let shape = row
            .get(8..12)
            .and_then(|id| shapes.get(&u32::from_le_bytes(id.try_into().ok()?)))
            .ok_or(DecodeError("event row of an unknown shape"))?;
        let row = shape
            .decode_row(row)
            .ok_or(DecodeError("undecodable event row"))?;
        if include[0] {
            out.extend_from_slice(&row.ts.to_le_bytes());
        }
        let mut misfits = 0;
        for ((name, ty, depth), _) in self.columns.iter().zip(&include[1..]).filter(|(_, i)| **i) {
            // ponytail: a linear search per column per row; index the shape's
            // columns if wide event tables make the ingester slow.
            let field = (0..shape.fields.len())
                .find(|&i| shape.fields[i].kind != Kind::Group && shape.column(i) == name);
            let groups = field.map(|f| shape.groups_around(f));
            match (field, groups) {
                (Some(f), Some(groups)) if groups.len() == *depth => {
                    let mut at = vec![0; groups.len() + 1];
                    misfits += write_column(&row, &groups, f, *ty, &mut at, out);
                }
                (field, _) => {
                    // Not in this shape, or at another depth: NULL, or no entries.
                    if *depth == 0 {
                        out.push(1);
                    } else {
                        out.push(0);
                    }
                    misfits += usize::from(field.is_some());
                }
            }
        }
        Ok(misfits)
    }
}

/// Write field `leaf` of `row` as a column nested in `groups` (outermost
/// first): for each group an array of its entries, then the value. `at`
/// holds where each group's counts, and last the leaf's values, are read
/// next. Returns how many values did not fit `ty`.
fn write_column(
    row: &Decoded<'_>,
    groups: &[usize],
    leaf: usize,
    ty: Type,
    at: &mut [usize],
    out: &mut Vec<u8>,
) -> usize {
    let depth = at.len() - 1 - groups.len();
    match groups.split_first() {
        None => {
            let value = match &row.columns[leaf] {
                EventColumn::Values(values) => values.get(at[depth]).copied().flatten(),
                EventColumn::Counts(_) => None,
            };
            at[depth] += 1;
            usize::from(!write_value(ty, value, out))
        }
        Some((&group, inner)) => {
            let count = match &row.columns[group] {
                EventColumn::Counts(counts) => counts.get(at[depth]).copied().unwrap_or(0),
                EventColumn::Values(_) => 0,
            };
            at[depth] += 1;
            write_varint(u64::from(count), out);
            (0..count)
                .map(|_| write_column(row, inner, leaf, ty, at, out))
                .sum()
        }
    }
}

/// `value` as a `Nullable(ty)` column, or NULL: `false` when a value was
/// there but could not be written as `ty`.
fn write_value(ty: Type, value: Option<Value<'_>>, out: &mut Vec<u8>) -> bool {
    let null = out.len();
    out.push(0);
    match (ty, value) {
        (Type::I64, Some(Value::I64(v))) => out.extend_from_slice(&v.to_le_bytes()),
        (Type::I64, Some(Value::U64(v))) if i64::try_from(v).is_ok() => {
            out.extend_from_slice(&(v as i64).to_le_bytes());
        }
        (Type::U64, Some(Value::U64(v))) => out.extend_from_slice(&v.to_le_bytes()),
        (Type::U64, Some(Value::I64(v))) if v >= 0 => {
            out.extend_from_slice(&(v as u64).to_le_bytes());
        }
        (Type::F64, Some(Value::F64(v))) => out.extend_from_slice(&v.to_le_bytes()),
        (Type::F64, Some(Value::I64(v))) => out.extend_from_slice(&(v as f64).to_le_bytes()),
        (Type::F64, Some(Value::U64(v))) => out.extend_from_slice(&(v as f64).to_le_bytes()),
        (Type::Bool, Some(Value::Bool(v))) => out.push(u8::from(v)),
        (Type::Str, Some(Value::Str(v))) => write_string(v.as_bytes(), out),
        (Type::Str, Some(Value::I64(v))) => write_string(v.to_string().as_bytes(), out),
        (Type::Str, Some(Value::U64(v))) => write_string(v.to_string().as_bytes(), out),
        (Type::Str, Some(Value::F64(v))) => write_string(v.to_string().as_bytes(), out),
        (Type::Str, Some(Value::Bool(v))) => write_string(v.to_string().as_bytes(), out),
        (_, value) => {
            out[null] = 1;
            return value.is_none();
        }
    }
    true
}
