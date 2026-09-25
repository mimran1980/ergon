//! Rows from `tracing` events, for tables that have no SBE message: their
//! columns are whatever fields the events carry.
//!
//! ```text
//! tracing::info!(table = "signal", instrument = %id, edge = 0.25);
//! ```
//!
//! A row travels on the same stream as the SBE messages, behind a standard
//! SBE message header whose schema id is [`SCHEMA_ID`], then: a `u64`
//! timestamp in UNIX nanoseconds, and every field as a `u8`-length name, a
//! [`Value`] kind byte and the value. `table` is one of the fields. All
//! little-endian; strings are `u32`-length prefixed.

use std::cell::RefCell;
use std::fmt::Write as _;

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

use crate::Persist;

/// The SBE schema id that marks an event row.
pub const SCHEMA_ID: u16 = 0xFFFF;

/// The field that names the table.
pub const TABLE: &str = "table";

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

/// A decoded row.
#[derive(Debug)]
pub struct Row<'a> {
    /// UNIX nanoseconds when the event was recorded.
    pub ts: u64,
    pub table: &'a str,
    /// Every field but `table`, in the order the event gave them.
    pub fields: Vec<(&'a str, Value<'a>)>,
}

/// Decode an event row (header included); `None` when it is malformed.
#[must_use]
pub fn decode(row: &[u8]) -> Option<Row<'_>> {
    fn take<'a>(rest: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
        let (head, tail) = rest.split_at_checked(n)?;
        *rest = tail;
        Some(head)
    }
    fn u64_le(bytes: &[u8]) -> Option<u64> {
        Some(u64::from_le_bytes(bytes.try_into().ok()?))
    }
    let mut rest = row.get(8..)?;
    let ts = u64_le(take(&mut rest, 8)?)?;
    let (mut table, mut fields) = (None, Vec::new());
    while !rest.is_empty() {
        let len = usize::from(*take(&mut rest, 1)?.first()?);
        let name = std::str::from_utf8(take(&mut rest, len)?).ok()?;
        let value = match *take(&mut rest, 1)?.first()? {
            0 => Value::I64(u64_le(take(&mut rest, 8)?)? as i64),
            1 => Value::U64(u64_le(take(&mut rest, 8)?)?),
            2 => Value::F64(f64::from_bits(u64_le(take(&mut rest, 8)?)?)),
            3 => Value::Bool(*take(&mut rest, 1)?.first()? != 0),
            4 => {
                let len = u32::from_le_bytes(take(&mut rest, 4)?.try_into().ok()?);
                Value::Str(std::str::from_utf8(take(&mut rest, len as usize)?).ok()?)
            }
            _ => return None,
        };
        match (name, value) {
            (TABLE, Value::Str(t)) => table = Some(t),
            _ => fields.push((name, value)),
        }
    }
    Some(Row {
        ts,
        table: table?,
        fields,
    })
}

/// Records every `tracing` event that has a `table` field whose table is
/// enabled in `tables.yaml`. Make it with [`Persist::layer`].
pub struct PersistLayer {
    pub(crate) persist: Persist,
}

thread_local! {
    /// The row being built, reused so recording does not allocate.
    static ROW: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

impl<S: Subscriber> Layer<S> for PersistLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().fields().field(TABLE).is_none() {
            return;
        }
        // Find the table first, formatting nothing: a disabled table stops here.
        let mut table = TableName {
            persist: &self.persist,
            enabled: false,
        };
        event.record(&mut table);
        if !table.enabled {
            return;
        }
        ROW.with_borrow_mut(|row| {
            row.clear();
            row.extend_from_slice(&[8, 0, 0, 0]); // block length 8 (the timestamp), template 0
            row.extend_from_slice(&SCHEMA_ID.to_le_bytes());
            row.extend_from_slice(&[0, 0]); // version 0
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64);
            row.extend_from_slice(&ts.to_le_bytes());
            event.record(&mut RowWriter(row));
            self.persist.publish(row);
        });
    }
}

/// Checks the `table` field against `tables.yaml`.
struct TableName<'a> {
    persist: &'a Persist,
    enabled: bool,
}

impl Visit for TableName<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == TABLE {
            self.enabled = self.persist.event_enabled(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == TABLE {
            // `table = %name`: rare, so a small allocation is fine here.
            self.enabled = self.persist.event_enabled(&format!("{value:?}"));
        }
    }
}

/// Appends each field to the row.
struct RowWriter<'a>(&'a mut Vec<u8>);

impl RowWriter<'_> {
    fn name(&mut self, field: &Field, kind: u8) {
        let name = field.name().as_bytes();
        let name = &name[..name.len().min(255)];
        self.0.push(name.len() as u8);
        self.0.extend_from_slice(name);
        self.0.push(kind);
    }
}

impl Visit for RowWriter<'_> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.name(field, 0);
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.name(field, 1);
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.name(field, 2);
        self.0.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.name(field, 3);
        self.0.push(u8::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.name(field, 4);
        self.0
            .extend_from_slice(&(value.len() as u32).to_le_bytes());
        self.0.extend_from_slice(value.as_bytes());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        // Formatted straight into the row; the length is patched after.
        self.name(field, 4);
        let at = self.0.len();
        self.0.extend_from_slice(&[0; 4]);
        let mut text = Text(self.0);
        let _ = write!(text, "{value:?}");
        let len = (self.0.len() - at - 4) as u32;
        self.0[at..at + 4].copy_from_slice(&len.to_le_bytes());
    }
}

/// `fmt::Write` into the row's bytes.
struct Text<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Text<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let mut row = vec![8, 0, 0, 0, 0xFF, 0xFF, 0, 0];
        row.extend_from_slice(&42u64.to_le_bytes());
        for (name, kind, value) in [
            ("table", 4u8, [&3u32.to_le_bytes()[..], b"sig"].concat()),
            ("edge", 2, 0.25f64.to_bits().to_le_bytes().to_vec()),
            ("n", 0, (-7i64).to_le_bytes().to_vec()),
            ("ok", 3, vec![1]),
        ] {
            row.push(name.len() as u8);
            row.extend_from_slice(name.as_bytes());
            row.push(kind);
            row.extend_from_slice(&value);
        }
        let decoded = decode(&row).ok_or("undecodable")?;
        assert_eq!((decoded.ts, decoded.table), (42, "sig"));
        assert_eq!(
            decoded.fields,
            [
                ("edge", Value::F64(0.25)),
                ("n", Value::I64(-7)),
                ("ok", Value::Bool(true))
            ]
        );
        assert!(
            decode(&row[..row.len() - 1]).is_none(),
            "a cut row is malformed"
        );
        Ok(())
    }
}
