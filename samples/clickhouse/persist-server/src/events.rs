//! Tables fed by `tracing` events (`persist_client::event`). There is no
//! schema: the columns are the fields the events carry, each typed by the
//! first value seen, after a `ts DateTime64(9, 'UTC')` column.
//!
//! | field value              | ClickHouse |
//! |--------------------------|------------|
//! | signed integer           | `Int64`    |
//! | unsigned integer         | `UInt64`   |
//! | float                    | `Float64`  |
//! | bool                     | `Bool`     |
//! | `&str`, `%x`, `?x`       | `String`   |
//!
//! A row without a column's field writes that column's default. A value of
//! another type is converted when that loses nothing (an integer into a
//! `Float64` or `String` column); one that cannot be writes the default and
//! is counted, and the count is reported as an error.

use persist_client::event::{Row, Value, decode};

use crate::table::{Column, DecodeError, Shape, write_string};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Type {
    I64,
    U64,
    F64,
    Bool,
    Str,
}

impl Type {
    fn of(value: Value<'_>) -> Self {
        match value {
            Value::I64(_) => Self::I64,
            Value::U64(_) => Self::U64,
            Value::F64(_) => Self::F64,
            Value::Bool(_) => Self::Bool,
            Value::Str(_) => Self::Str,
        }
    }

    fn clickhouse(self) -> &'static str {
        match self {
            Self::I64 => "Int64",
            Self::U64 => "UInt64",
            Self::F64 => "Float64",
            Self::Bool => "Bool",
            Self::Str => "String",
        }
    }
}

/// One event table and the columns seen so far.
#[derive(Clone, Debug)]
pub(crate) struct EventTable {
    pub(crate) name: String,
    columns: Vec<(String, Type)>,
}

impl EventTable {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
        }
    }

    /// Add the columns of `row` not seen before; `true` if there were any.
    pub(crate) fn learn(&mut self, row: &Row<'_>) -> bool {
        let before = self.columns.len();
        for (name, value) in &row.fields {
            if !self.columns.iter().any(|(c, _)| c == name) {
                self.columns.push(((*name).to_string(), Type::of(*value)));
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
        let fields = self.columns.iter().map(|(name, ty)| Column {
            name: name.clone(),
            ch_type: ty.clickhouse().into(),
        });
        Some(Shape {
            name: self.name.clone(),
            columns: std::iter::once(ts).chain(fields).collect(),
            order_by: vec!["ts".into()],
            partition: Some("ts".into()),
        })
    }

    /// Append `row` as one RowBinary row of the columns whose `include` flag
    /// is set. Returns how many values did not fit their column's type.
    pub(crate) fn write_row(
        &self,
        row: &[u8],
        include: &[bool],
        out: &mut Vec<u8>,
    ) -> Result<usize, DecodeError> {
        let row = decode(row).ok_or(DecodeError("undecodable event row"))?;
        if include[0] {
            out.extend_from_slice(&row.ts.to_le_bytes());
        }
        let mut misfits = 0;
        for ((name, ty), _) in self.columns.iter().zip(&include[1..]).filter(|(_, i)| **i) {
            let value = row.fields.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
            if !write_value(*ty, value, out) {
                misfits += 1;
            }
        }
        Ok(misfits)
    }
}

/// `value` as a `ty` column, or the column's default: `false` when a value
/// was there but could not be written as `ty`.
fn write_value(ty: Type, value: Option<Value<'_>>, out: &mut Vec<u8>) -> bool {
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
        (ty, value) => {
            match ty {
                Type::Str => write_string(b"", out),
                Type::Bool => out.push(0),
                Type::I64 | Type::U64 | Type::F64 => out.extend_from_slice(&[0; 8]),
            }
            return value.is_none();
        }
    }
    true
}
