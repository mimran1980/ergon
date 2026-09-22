//! Persistence traits, row/value writers, and recording outcomes.
//!
//! The recording hot path borrows caller data and writes into pre-sized
//! writer-owned buffers. Nothing here allocates, formats, logs, or waits:
//! encoding either succeeds into the supplied buffer or returns a typed
//! error and aborts the row before anything is published.

use crate::schema::{RowSchema, TypeCode, ValueSchema};

/// Typed encoding failures. Numeric discriminants travel in counters; no
/// formatting happens on the recording path.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EncodeError {
    /// Supplied buffer cannot hold the encoded row.
    BufferTooSmall { needed: usize, actual: usize },
    /// Text value is not valid UTF-8.
    InvalidUtf8,
    /// Decimal value does not fit the declared precision/scale.
    DecimalOverflow { precision: u8, scale: i8 },
    /// Integer overflow converting a checked value.
    Overflow,
    /// Value does not match the column schema kind.
    TypeMismatch { column: usize },
    /// Column index outside the schema.
    ColumnOutOfRange { column: usize },
    /// Schema itself cannot be encoded (variable-length without length).
    InvalidSchema { column: usize },
    /// Callback/validation failure reported by a custom mapper.
    Custom(u16),
    /// SBE codec rejected the encode/decode (wire-level failure).
    SbeWire,
}

impl From<crate::recording::sbe_rt::EncodeError> for EncodeError {
    fn from(_: crate::recording::sbe_rt::EncodeError) -> Self {
        Self::SbeWire
    }
}

impl From<crate::recording::sbe_rt::DecodeError> for EncodeError {
    fn from(_: crate::recording::sbe_rt::DecodeError) -> Self {
        Self::SbeWire
    }
}

impl From<crate::recording::sbe_rt::VerifyError> for EncodeError {
    fn from(_: crate::recording::sbe_rt::VerifyError) -> Self {
        Self::SbeWire
    }
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall { needed, actual } => {
                write!(f, "buffer too small: needed {needed}, actual {actual}")
            }
            Self::InvalidUtf8 => f.write_str("invalid utf-8"),
            Self::DecimalOverflow { precision, scale } => {
                write!(f, "decimal overflow: precision {precision}, scale {scale}")
            }
            Self::Overflow => f.write_str("integer overflow"),
            Self::TypeMismatch { column } => write!(f, "type mismatch at column {column}"),
            Self::ColumnOutOfRange { column } => write!(f, "column {column} out of range"),
            Self::InvalidSchema { column } => write!(f, "invalid schema at column {column}"),
            Self::Custom(code) => write!(f, "custom error code {code}"),
            Self::SbeWire => f.write_str("sbe wire encode/decode failure"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Typed projection failures for `SbeProjection::project`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProjectionError {
    /// Message bytes are structurally invalid for the declared projection.
    MalformedMessage,
    /// Message does not match the projection's SBE identity.
    IdentityMismatch,
    /// A required computed value could not be produced.
    RequiredValueMissing { column: usize },
    /// Validation/conversion failure; typed output for this event is rolled back.
    Conversion { column: usize, code: u16 },
    /// Output buffer too small for the projected row.
    BufferTooSmall { needed: usize, actual: usize },
}

/// Compact publication outcomes. `Published` means the transport accepted
/// the bytes; it is not a durability acknowledgement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum RecordOutcome {
    /// Bytes were copied/committed into the transport.
    Published = 1,
    /// Table (or process) recording is disabled; nothing was evaluated.
    Disabled = 2,
    /// Bounded drop with a reason code (quota, overload, oversize...).
    Dropped(u16) = 3,
    /// Row was invalid and was rejected before publication.
    Invalid(u16) = 4,
}

impl RecordOutcome {
    /// Reason code for dropped/invalid outcomes.
    #[must_use]
    pub const fn code(self) -> Option<u16> {
        match self {
            Self::Dropped(c) | Self::Invalid(c) => Some(c),
            Self::Published | Self::Disabled => None,
        }
    }
}

/// Well-known drop/invalid reason codes.
pub mod reasons {
    /// Payload exceeded the configured record limit.
    pub const OVERSIZE: u16 = 1;
    /// Transport buffer full (backpressure/overload).
    pub const TRANSPORT_FULL: u16 = 2;
    /// Diagnostics quota exhausted for this interval.
    pub const QUOTA_EXHAUSTED: u16 = 4;
    /// Prepared handle belongs to another session.
    pub const SESSION_MISMATCH: u16 = 5;
    /// Row callback returned an error.
    pub const CALLBACK_ERROR: u16 = 6;
}

/// Immutable envelope/session metadata visible to projections in the
/// ingester. Decoded once per record, borrowed afterwards.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RecordMetadata {
    /// Producer run identity.
    pub run_id: u64,
    /// Writer within the run.
    pub writer_id: u16,
    /// Monotonic per-writer sequence.
    pub sequence: u64,
    /// Capture time, UTC nanoseconds since epoch.
    pub captured_at_ns: u64,
    /// Catalog generation required to decode the payload.
    pub catalog_generation: u32,
}

/// Typed writer for one ordered row, borrowing a pre-sized buffer.
///
/// Values are written in schema order; the null bitmap for nullable columns
/// is maintained automatically from the writes performed. The bitmap holds
/// one bit per nullable column, compacted in column order.
pub struct RowWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
    schema: &'a RowSchema,
    next_column: usize,
}

impl<'a> RowWriter<'a> {
    /// Create a writer over `buf` for `schema`. The buffer must be at least
    /// [`RowSchema::min_encoded_len`] bytes.
    pub fn new(buf: &'a mut [u8], schema: &'a RowSchema) -> Result<Self, EncodeError> {
        let min = schema.min_encoded_len()?;
        if buf.len() < min {
            return Err(EncodeError::BufferTooSmall {
                needed: min,
                actual: buf.len(),
            });
        }
        buf[..schema.nullmap_len()].fill(0);
        Ok(Self {
            buf,
            pos: schema.nullmap_len(),
            schema,
            next_column: 0,
        })
    }

    /// Schema this writer was created for.
    #[must_use]
    pub const fn schema(&self) -> &'a RowSchema {
        self.schema
    }

    /// Bytes written so far; a complete row's length when all columns are set.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    fn take_column(
        &mut self,
        ty: TypeCode,
        is_array: bool,
        nullable: bool,
    ) -> Result<usize, EncodeError> {
        let i = self.next_column;
        let col = self
            .schema
            .columns
            .get(i)
            .ok_or(EncodeError::ColumnOutOfRange { column: i })?;
        if col.ty != ty || col.is_array != is_array || col.nullable != nullable {
            return Err(EncodeError::TypeMismatch { column: i });
        }
        self.next_column += 1;
        Ok(i)
    }

    fn set_null_bit_for_previous(&mut self) {
        self.set_null_bit(self.next_column - 1);
    }

    fn set_null_bit(&mut self, column_index: usize) {
        let bit_index = self
            .schema
            .columns
            .iter()
            .take(column_index)
            .filter(|c| c.nullable)
            .count();
        self.buf[bit_index / 8] |= 1 << (bit_index % 8);
    }

    fn take_fixed(&mut self, n: usize) -> Result<&mut [u8], EncodeError> {
        if self.pos + n > self.buf.len() {
            return Err(EncodeError::BufferTooSmall {
                needed: self.pos + n,
                actual: self.buf.len(),
            });
        }
        let out = &mut self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    /// Write a boolean column.
    pub fn write_bool(&mut self, v: bool) -> Result<(), EncodeError> {
        self.take_column(TypeCode::Bool, false, false)?;
        self.take_fixed(1)?[0] = u8::from(v);
        Ok(())
    }

    /// Write a nullable boolean column (`None` records absence).
    pub fn write_bool_opt(&mut self, v: Option<bool>) -> Result<(), EncodeError> {
        self.take_column(TypeCode::Bool, false, true)?;
        if v.is_none() {
            self.set_null_bit_for_previous();
        }
        self.take_fixed(1)?[0] = v.map_or(0, u8::from);
        Ok(())
    }

    fn write_int_impl(&mut self, ty: TypeCode, bytes: &[u8]) -> Result<(), EncodeError> {
        self.take_column(ty, false, false)?;
        self.take_fixed(bytes.len())?.copy_from_slice(bytes);
        Ok(())
    }

    fn write_int_opt_impl(
        &mut self,
        ty: TypeCode,
        v: Option<i64>,
        width: usize,
    ) -> Result<(), EncodeError> {
        self.take_column(ty, false, true)?;
        if v.is_none() {
            self.set_null_bit_for_previous();
        }
        let raw = v.unwrap_or(0).to_le_bytes();
        self.take_fixed(width)?.copy_from_slice(&raw[..width]);
        Ok(())
    }

    /// Write an `i8` column.
    pub fn write_i8(&mut self, v: i8) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::I8, &v.to_le_bytes())
    }
    /// Write an `i16` column.
    pub fn write_i16(&mut self, v: i16) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::I16, &v.to_le_bytes())
    }
    /// Write an `i32` column.
    pub fn write_i32(&mut self, v: i32) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::I32, &v.to_le_bytes())
    }
    /// Write an `i64` column.
    pub fn write_i64(&mut self, v: i64) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::I64, &v.to_le_bytes())
    }
    /// Write a nullable `i64` column.
    pub fn write_i64_opt(&mut self, v: Option<i64>) -> Result<(), EncodeError> {
        self.write_int_opt_impl(TypeCode::I64, v, 8)
    }
    /// Write a `u8` column.
    pub fn write_u8(&mut self, v: u8) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::U8, &v.to_le_bytes())
    }
    /// Write a `u16` column.
    pub fn write_u16(&mut self, v: u16) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::U16, &v.to_le_bytes())
    }
    /// Write a `u32` column.
    pub fn write_u32(&mut self, v: u32) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::U32, &v.to_le_bytes())
    }
    /// Write a `u64` column.
    pub fn write_u64(&mut self, v: u64) -> Result<(), EncodeError> {
        self.write_int_impl(TypeCode::U64, &v.to_le_bytes())
    }
    /// Write a nullable `u64` column.
    pub fn write_u64_opt(&mut self, v: Option<u64>) -> Result<(), EncodeError> {
        self.take_column(TypeCode::U64, false, true)?;
        if v.is_none() {
            self.set_null_bit_for_previous();
        }
        self.take_fixed(8)?
            .copy_from_slice(&v.unwrap_or(0).to_le_bytes());
        Ok(())
    }
    /// Write an `f32` column.
    pub fn write_f32(&mut self, v: f32) -> Result<(), EncodeError> {
        self.take_column(TypeCode::F32, false, false)?;
        self.take_fixed(4)?.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }
    /// Write an `f64` column.
    pub fn write_f64(&mut self, v: f64) -> Result<(), EncodeError> {
        self.take_column(TypeCode::F64, false, false)?;
        self.take_fixed(8)?.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }

    /// Write a decimal mantissa column (exact `i128`, scale from the schema).
    pub fn write_decimal_i128(&mut self, v: i128) -> Result<(), EncodeError> {
        self.take_column(TypeCode::Decimal, false, false)?;
        self.take_fixed(16)?.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }

    /// Write UTF-8 text column (validated; stored length-prefixed).
    pub fn write_str(&mut self, v: &str) -> Result<(), EncodeError> {
        self.take_column(TypeCode::Utf8, false, false)?;
        self.write_len_prefixed(v.as_bytes())
    }

    /// Write a nullable UTF-8 text column.
    pub fn write_str_opt(&mut self, v: Option<&str>) -> Result<(), EncodeError> {
        self.take_column(TypeCode::Utf8, false, true)?;
        if v.is_none() {
            self.set_null_bit_for_previous();
        }
        self.write_len_prefixed(v.unwrap_or("").as_bytes())
    }

    /// Write bytes column (length-prefixed).
    pub fn write_bytes(&mut self, v: &[u8]) -> Result<(), EncodeError> {
        self.take_column(TypeCode::Bytes, false, false)?;
        self.write_len_prefixed(v)
    }

    /// Write an array column from concatenated little-endian items.
    pub fn write_array(
        &mut self,
        ty: TypeCode,
        item_width: usize,
        items_le: &[u8],
        count: usize,
    ) -> Result<(), EncodeError> {
        self.take_column(ty, true, false)?;
        let n = u32::try_from(count).map_err(|_| EncodeError::Overflow)?;
        self.write_fixed(&n.to_le_bytes())?;
        if items_le.len() != count.checked_mul(item_width).ok_or(EncodeError::Overflow)? {
            return Err(EncodeError::TypeMismatch {
                column: self.next_column,
            });
        }
        self.write_fixed(items_le)
    }

    fn write_fixed(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.take_fixed(bytes.len())?.copy_from_slice(bytes);
        Ok(())
    }

    fn write_len_prefixed(&mut self, data: &[u8]) -> Result<(), EncodeError> {
        let l = u32::try_from(data.len()).map_err(|_| EncodeError::Overflow)?;
        self.take_fixed(4)?.copy_from_slice(&l.to_le_bytes());
        self.take_fixed(data.len())?.copy_from_slice(data);
        Ok(())
    }

    /// Write an optional fixed-width scalar from raw little-endian bytes.
    /// `None` records absence; the column must be nullable.
    pub fn write_opt_le(&mut self, v: Option<&[u8]>) -> Result<(), EncodeError> {
        let i = self.next_column;
        let col = self
            .schema
            .columns
            .get(i)
            .ok_or(EncodeError::ColumnOutOfRange { column: i })?;
        if !col.nullable || col.is_array {
            return Err(EncodeError::TypeMismatch { column: i });
        }
        let w = col
            .ty
            .fixed_width()
            .ok_or(EncodeError::InvalidSchema { column: i })?;
        let bytes: &[u8] = match v {
            Some(b) => b,
            None => &[0u8; 16][..w],
        };
        if bytes.len() != w {
            return Err(EncodeError::TypeMismatch { column: i });
        }
        self.take_fixed(w)?.copy_from_slice(bytes);
        self.next_column += 1;
        if v.is_none() {
            self.set_null_bit_for_previous();
        }
        Ok(())
    }

    /// Set a dynamic column slot with a raw little-endian value of exactly
    /// the schema's fixed width.
    pub fn set_raw(&mut self, slot: usize, bytes: &[u8]) -> Result<(), EncodeError> {
        if slot != self.next_column {
            return Err(EncodeError::ColumnOutOfRange { column: slot });
        }
        let col = self
            .schema
            .columns
            .get(slot)
            .ok_or(EncodeError::ColumnOutOfRange { column: slot })?;
        let w = col
            .ty
            .fixed_width()
            .ok_or(EncodeError::InvalidSchema { column: slot })?;
        if bytes.len() != w {
            return Err(EncodeError::TypeMismatch { column: slot });
        }
        self.next_column += 1;
        self.write_fixed(bytes)
    }

    /// Delegate the remaining columns to a typed persistable body.
    pub fn write(&mut self, value: &impl Persistable) -> Result<(), EncodeError> {
        value.encode(self)
    }

    /// Write a `PersistAs`-customized value at the current column.
    pub fn write_persist_as<V: PersistAs>(&mut self, v: &V) -> Result<(), EncodeError> {
        let schema = V::value_schema();
        let i = self.next_column;
        let col = self
            .schema
            .columns
            .get(i)
            .ok_or(EncodeError::ColumnOutOfRange { column: i })?;
        if col.ty != schema.ty || col.is_array != schema.is_array {
            return Err(EncodeError::TypeMismatch { column: i });
        }
        let needed = v.encoded_len()?;
        if self.pos + needed > self.buf.len() {
            return Err(EncodeError::BufferTooSmall {
                needed: self.pos + needed,
                actual: self.buf.len(),
            });
        }
        let pos = self.pos;
        let nullable = schema.nullable;
        let written = {
            let mut vw = ValueWriter::new(&mut self.buf[pos..]);
            v.encode_value(&mut vw)?;
            vw.position()
        };
        if nullable {
            self.set_null_bit_for_previous();
        }
        self.pos += written;
        self.next_column += 1;
        Ok(())
    }
}

/// Trait for rows persistable through the prepared recorder.
///
/// Implementations are generated (`persist-derive`, SBE hooks) or
/// handwritten; both must be allocation-free and checked. Tests must
/// establish `encoded_len() == bytes_written` for every shape before a
/// buffer overrun can occur.
pub trait Persistable {
    /// Ordered storage schema of this row type.
    fn schema() -> &'static RowSchema
    where
        Self: Sized;
    /// Checked encoded length of this value.
    fn encoded_len(&self) -> Result<usize, EncodeError>;
    /// Encode into the supplied row writer.
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError>;
}

/// Customization trait for a field's storage type conversion.
///
/// Implemented by typed wrappers (e.g. exact-decimal price types) that
/// write directly into the supplied [`ValueWriter`] without allocating a
/// returned byte vector.
pub trait PersistAs {
    /// Storage schema of the mapped value.
    fn value_schema() -> &'static ValueSchema
    where
        Self: Sized;
    /// Checked encoded length of this value.
    fn encoded_len(&self) -> Result<usize, EncodeError>;
    /// Write the value into the supplied writer.
    fn encode_value(&self, out: &mut ValueWriter<'_>) -> Result<(), EncodeError>;
}

/// Typed value writer used by [`PersistAs`]; borrows a bounded buffer.
pub struct ValueWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ValueWriter<'a> {
    /// Writer over a bounded buffer.
    #[must_use]
    pub const fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Bytes written so far.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    fn take(&mut self, n: usize) -> Result<&mut [u8], EncodeError> {
        if self.pos + n > self.buf.len() {
            return Err(EncodeError::BufferTooSmall {
                needed: self.pos + n,
                actual: self.buf.len(),
            });
        }
        let out = &mut self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    /// Write fixed-width little-endian bytes.
    pub fn write_fixed(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let out = self.take(bytes.len())?;
        out.copy_from_slice(bytes);
        Ok(())
    }
}

impl From<EncodeError> for ProjectionError {
    fn from(e: EncodeError) -> Self {
        match e {
            EncodeError::BufferTooSmall { needed, actual } => {
                Self::BufferTooSmall { needed, actual }
            }
            _ => Self::Conversion { column: 0, code: 1 },
        }
    }
}

impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedMessage => f.write_str("malformed message"),
            Self::IdentityMismatch => f.write_str("identity mismatch"),
            Self::RequiredValueMissing { column } => {
                write!(f, "required value missing at column {column}")
            }
            Self::Conversion { column, code } => {
                write!(f, "conversion failure at column {column} (code {code})")
            }
            Self::BufferTooSmall { needed, actual } => {
                write!(f, "buffer too small: needed {needed}, actual {actual}")
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

/// Zero-column row marker for schema-only prepared tables (e.g. the
/// tracing adapter, which writes rows through the schema directly).
pub struct EmptyRow;

impl Persistable for EmptyRow {
    fn schema() -> &'static RowSchema {
        static EMPTY: &[ValueSchema] = &[];
        static SCHEMA: RowSchema = RowSchema { columns: EMPTY };
        &SCHEMA
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(0)
    }
    fn encode(&self, _out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        Ok(())
    }
}

/// Scalar width accessor for derive-generated length sums.
pub trait ScalarWidth {
    /// Fixed byte width of this scalar type.
    const WIDTH: usize;
}

impl ScalarWidth for bool {
    const WIDTH: usize = 1;
}
impl ScalarWidth for i8 {
    const WIDTH: usize = 1;
}
impl ScalarWidth for u8 {
    const WIDTH: usize = 1;
}
impl ScalarWidth for i16 {
    const WIDTH: usize = 2;
}
impl ScalarWidth for u16 {
    const WIDTH: usize = 2;
}
impl ScalarWidth for i32 {
    const WIDTH: usize = 4;
}
impl ScalarWidth for u32 {
    const WIDTH: usize = 4;
}
impl ScalarWidth for f32 {
    const WIDTH: usize = 4;
}
impl ScalarWidth for i64 {
    const WIDTH: usize = 8;
}
impl ScalarWidth for u64 {
    const WIDTH: usize = 8;
}
impl ScalarWidth for f64 {
    const WIDTH: usize = 8;
}

/// One scalar field of a prepared-callsite event; written into the slot at
/// the macro's expansion order.
pub trait EventField {
    /// Write this value as the current column.
    fn write_field(&self, out: &mut RowWriter<'_>, slot: usize) -> Result<(), EncodeError>;
}

impl EventField for bool {
    fn write_field(&self, out: &mut RowWriter<'_>, slot: usize) -> Result<(), EncodeError> {
        out.write_bool(*self)?;
        let _ = slot;
        Ok(())
    }
}
impl EventField for u32 {
    fn write_field(&self, out: &mut RowWriter<'_>, slot: usize) -> Result<(), EncodeError> {
        out.write_u32(*self)?;
        let _ = slot;
        Ok(())
    }
}
impl EventField for u64 {
    fn write_field(&self, out: &mut RowWriter<'_>, slot: usize) -> Result<(), EncodeError> {
        out.write_u64(*self)?;
        let _ = slot;
        Ok(())
    }
}
impl EventField for i64 {
    fn write_field(&self, out: &mut RowWriter<'_>, slot: usize) -> Result<(), EncodeError> {
        out.write_i64(*self)?;
        let _ = slot;
        Ok(())
    }
}
impl EventField for f64 {
    fn write_field(&self, out: &mut RowWriter<'_>, slot: usize) -> Result<(), EncodeError> {
        out.write_f64(*self)?;
        let _ = slot;
        Ok(())
    }
}

/// Projection output writer for `SbeProjection::project` in the ingester.
///
/// Output is staged per source event: a validation/conversion failure rolls
/// back the uncommitted typed output for that event.
pub struct ProjectedRowWriter<'a> {
    inner: RowWriter<'a>,
    committed: bool,
}

impl<'a> ProjectedRowWriter<'a> {
    /// Wrap a row writer; staged output rolls back on error.
    pub fn new(buf: &'a mut [u8], schema: &'a RowSchema) -> Result<Self, EncodeError> {
        Ok(Self {
            inner: RowWriter::new(buf, schema)?,
            committed: false,
        })
    }

    /// Borrow the inner writer for typed writes.
    #[must_use]
    pub fn row(&mut self) -> &mut RowWriter<'a> {
        &mut self.inner
    }

    /// Commit the staged row.
    pub fn commit(&mut self) {
        self.committed = true;
    }

    /// Whether the staged row was committed.
    #[must_use]
    pub const fn is_committed(&self) -> bool {
        self.committed
    }

    /// Bytes written by the staged row.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.inner.position()
    }
}

/// Declare and execute a compiled raw-message projection in the ingester.
pub trait SbeProjection {
    /// SBE identity and immutable mapping revision.
    fn descriptor() -> &'static crate::schema::ProjectionDescriptor
    where
        Self: Sized;
    /// Traverse the borrowed decoder for `message` and write the projected
    /// row without materializing an intermediate DTO.
    fn project(
        message: &[u8],
        metadata: &RecordMetadata,
        out: &mut ProjectedRowWriter<'_>,
    ) -> Result<(), ProjectionError>
    where
        Self: Sized;
}
