//! DynamicRecorder — runtime table builder (producer side).
//!
//! The "no struct needed" path.  Register fields at runtime, pre-compute the
//! wire layout, then call [`record()`] on the hot path with positional values.
//!
//! ## Wire format
//!
//! Each [`record()`] call encodes a SBE [`DynamicRow`] message into the
//! internal pre-allocated buffer.  The layout follows `sbe_schema.xml`:
//!
//! ```text
//! [8-byte SBE header] [4-byte schemaId]
//! [rowMetadata group] (dim header + N entries × keyLen/valLen)
//! [int64Fields group] (dim header + N entries × fieldId+int64)
//! [uint64Fields group]
//! [float64Fields group]
//! [boolFields group]        (fieldId + u8 0/1)
//! [stringFields group]      (fieldId + strLen — data in symbolTable)
//! [nullFields group]        (fieldId only)
//! [symbolTable varData]     (4-byte length + concatenated metadata/string bytes)
//! ```
//!
//! ## Performance
//!
//! - `build()` allocates the internal buffer (typical max ~65 KB).
//! - `record()` reuses the buffer — zero heap allocations on the hot path when
//!   string lengths are within the pre-allocated capacity.
//! - Field values are written positionally: one pass over the values array with
//!   O(1) dispatch per value.

use crate::persist::TtlConfig;
use crate::sbe::DynamicRowEncoder;
use crate::types::ColumnType;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

// ── DynamicValueRef (borrowed, zero-allocation) ──────────────────────────

/// A borrowed positional value for zero-allocation recording.
///
/// Like [`DynamicValue`] but holds borrowed references instead of owned data.
/// Used by `record_into` to encode directly into a caller-provided buffer
/// without allocating strings or arrays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DynamicValueRef<'a> {
    /// Signed integer.
    Int64(i64),
    /// Unsigned integer.
    UInt64(u64),
    /// Double-precision float.
    Float64(f64),
    /// Boolean.
    Bool(bool),
    /// UTF-8 string slice.
    String(&'a str),
    /// Explicit null.
    Null,
    /// Decimal array: borrowed slice of (mantissa, exponent) pairs.
    DecimalArray(&'a [(i64, i8)]),
}

// ── DynamicValue (owned) ────────────────────────────────────────────────

/// A positional value for [`DynamicRecorder::record`].
///
/// Each variant corresponds to a typed group in the SBE `DynamicRow` message.
/// `Null` is encoded in the nullFields group — the underlying column type's
/// group is simply skipped for that position.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // DecimalArray — V2 only, actively used in sample crate
pub enum DynamicValue {
    /// Signed integer (stored in `int64Fields` group).
    Int64(i64),
    /// Unsigned integer (stored in `uint64Fields` group).
    UInt64(u64),
    /// Double-precision float (stored in `float64Fields` group).
    Float64(f64),
    /// Boolean (stored in `boolFields` group — 0 or 1).
    Bool(bool),
    /// UTF-8 string (stored as entries in `stringFields` group + symbolTable).
    String(String),
    /// Explicit null (stored in `nullFields` group).
    Null,
    /// Decimal array: list of (mantissa: i64, exponent: i8) pairs.
    /// Stored in `decimalArrayFields` group (V2 only).
    DecimalArray(Vec<(i64, i8)>),
}

// ── DynamicValueType ─────────────────────────────────────────────────────

/// The logical type of a [`DynamicValue`] variant.
///
/// Used internally by [`DynamicRecorder`] to map registered [`ColumnType`]s
/// to their wire groups.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // DecimalArray — V2 only
enum DynamicValueType {
    Int64,
    UInt64,
    Float64,
    Bool,
    String,
    /// Decimal array type (V2 only).
    DecimalArray,
}

impl DynamicValueType {
    /// Resolve a [`ColumnType`] (stripping `Nullable` wrappers) to the
    /// corresponding value type variant.
    fn from_column_type(ct: &ColumnType) -> Option<Self> {
        let inner = match ct {
            ColumnType::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        match inner {
            ColumnType::Int8 | ColumnType::Int16 | ColumnType::Int32 | ColumnType::Int64 => {
                Some(Self::Int64)
            }
            ColumnType::UInt8 | ColumnType::UInt16 | ColumnType::UInt32 | ColumnType::UInt64 => {
                Some(Self::UInt64)
            }
            ColumnType::Float32 | ColumnType::Float64 => Some(Self::Float64),
            ColumnType::Bool => Some(Self::Bool),
            ColumnType::String | ColumnType::FixedString(_) => Some(Self::String),
            // Unsupported types — Decimal, Date, DateTime, Array, etc.
            _ => None,
        }
    }
}

// ── Error ────────────────────────────────────────────────────────────────

/// Errors returned by [`DynamicRecorder::record`].
#[derive(Debug, Clone)]
pub enum DynamicRecorderError {
    /// The number of values does not match the number of registered fields.
    ValueCountMismatch {
        /// Number of fields registered via the builder.
        expected: usize,
        /// Number of values passed to [`record`].
        actual: usize,
    },
    /// A value's variant (`Int64`, `String`, …) does not match the registered
    /// column type for that position.
    ValueTypeMismatch {
        /// Position in the values slice.
        position: usize,
        /// Expected variant name.
        expected: &'static str,
        /// Actual variant name.
        actual: &'static str,
    },
    /// An internal SBE encoding error (buffer too short, var data too long, …).
    Encode(String),
    /// A field's [`ColumnType`] is not supported by the dynamic recorder.
    UnsupportedColumnType {
        column_name: String,
        column_type: crate::types::ColumnType,
    },
    /// Table name must not be empty.
    EmptyTableName,
    /// At least one field must be registered.
    NoFields,
}

impl fmt::Display for DynamicRecorderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueCountMismatch { expected, actual } => {
                write!(f, "expected {expected} values, got {actual}")
            }
            Self::ValueTypeMismatch {
                position,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "value at position {position}: expected {expected}, got {actual}"
                )
            }
            Self::Encode(msg) => write!(f, "encoding error: {msg}"),
            Self::UnsupportedColumnType {
                column_name,
                column_type,
            } => {
                write!(
                    f,
                    "unsupported column type '{column_type}' for field '{column_name}'"
                )
            }
            Self::EmptyTableName => write!(f, "table name must not be empty"),
            Self::NoFields => write!(f, "at least one field must be registered"),
        }
    }
}

impl std::error::Error for DynamicRecorderError {}

// ── DynamicRecorderBuilder ───────────────────────────────────────────────

/// Builder for [`DynamicRecorder`].
///
/// Register fields (with their [`ColumnType`]) and optional static metadata,
/// then call [`build()`](DynamicRecorderBuilder::build) to obtain the recorder.
///
/// # Example
///
/// ```ignore
/// let mut rec = DynamicRecorderBuilder::new("trades")
///     .field("price", ColumnType::Float64)
///     .field("qty", ColumnType::UInt64)
///     .field("symbol", ColumnType::String)
///     .metadata("source", "exchange_a")
///     .build().unwrap();
/// let buf = rec.record(&[
///     DynamicValue::Float64(100.50),
///     DynamicValue::UInt64(1000),
///     DynamicValue::String("AAPL".into()),
/// ]).unwrap();
/// ```
pub struct DynamicRecorderBuilder {
    table_name: String,
    fields: Vec<(String, ColumnType)>,
    metadata: Vec<(String, String)>,
    ttl: Option<TtlConfig>,
}

impl DynamicRecorderBuilder {
    /// Start building a recorder for `table_name`.
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            fields: Vec::new(),
            metadata: Vec::new(),
            ttl: None,
        }
    }

    /// Register a column with its ClickHouse type.
    ///
    /// # Panics
    ///
    /// Panics at `build()` time if `ty` is not supported for dynamic recording
    /// (e.g., `Decimal`, `Array`, `DateTime`).
    pub fn field(mut self, name: impl Into<String>, ty: ColumnType) -> Self {
        self.fields.push((name.into(), ty));
        self
    }

    /// Attach a static metadata key-value pair.
    ///
    /// Metadata is identical on every `record()` call.  Changing metadata
    /// produces a different `schema_id` (registering a new schema on the
    /// consumer side).
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    /// Set a TTL policy for the table.
    ///
    /// The TTL is used when generating the table's DDL — it is not part
    /// of the SBE wire format or the schema_id.  See [`TtlConfig`].
    pub fn ttl(mut self, column: impl Into<String>, interval: impl Into<String>) -> Self {
        self.ttl = Some(TtlConfig::new(column, interval));
        self
    }

    /// Consume the builder and produce a [`DynamicRecorder`].
    ///
    /// # Errors
    ///
    /// Returns [`DynamicRecorderError::UnsupportedColumnType`] if any registered
    /// field has an unsupported [`ColumnType`].
    pub fn build(self) -> Result<DynamicRecorder, DynamicRecorderError> {
        if self.table_name.is_empty() {
            return Err(DynamicRecorderError::EmptyTableName);
        }
        if self.fields.is_empty() {
            return Err(DynamicRecorderError::NoFields);
        }
        // Validate all column types are supported
        for (name, ct) in &self.fields {
            if DynamicValueType::from_column_type(ct).is_none() {
                return Err(DynamicRecorderError::UnsupportedColumnType {
                    column_name: name.clone(),
                    column_type: ct.clone(),
                });
            }
        }

        let schema_id = compute_schema_id(&self.table_name, &self.fields, &self.metadata);

        // Assign field_id = positional index (0..N) for each registered field.
        let field_descriptors: Vec<FieldDesc> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, (_name, ct))| FieldDesc {
                field_id: i as u8,
                col_type: ct.clone(),
                value_type: DynamicValueType::from_column_type(ct).expect("validated above"),
            })
            .collect();

        // Pre-compute per-type field counts (max, assuming all non-null).
        let int64_count = field_descriptors
            .iter()
            .filter(|fd| fd.value_type == DynamicValueType::Int64)
            .count() as u16;
        let uint64_count = field_descriptors
            .iter()
            .filter(|fd| fd.value_type == DynamicValueType::UInt64)
            .count() as u16;
        let float64_count = field_descriptors
            .iter()
            .filter(|fd| fd.value_type == DynamicValueType::Float64)
            .count() as u16;
        let bool_count = field_descriptors
            .iter()
            .filter(|fd| fd.value_type == DynamicValueType::Bool)
            .count() as u16;
        let string_count = field_descriptors
            .iter()
            .filter(|fd| fd.value_type == DynamicValueType::String)
            .count() as u16;
        let nullable_count = field_descriptors
            .iter()
            .filter(|fd| matches!(fd.col_type, ColumnType::Nullable(_)))
            .count() as u16;

        // Pre-encode metadata entries (key_len, val_len) and collect symbol
        // bytes.  The symbol table in the wire format contains metadata keys
        // and values concatenated before any string field data.
        let mut metadata_entries: Vec<(u16, u16)> = Vec::new();
        let mut metadata_symbols: Vec<u8> = Vec::new();
        for (k, v) in &self.metadata {
            let kl = k.len() as u16;
            let vl = v.len() as u16;
            metadata_entries.push((kl, vl));
            metadata_symbols.extend_from_slice(k.as_bytes());
            metadata_symbols.extend_from_slice(v.as_bytes());
        }
        let metadata_count = metadata_entries.len() as u16;

        // Compute the worst-case maximum encoded size so we can pre-allocate.
        // We assume every Nullable field produces a Null entry.
        let max_symbol_data = metadata_symbols.len();
        let max_size = compute_encoded_size(
            metadata_count as usize,
            int64_count as usize,
            uint64_count as usize,
            float64_count as usize,
            bool_count as usize,
            string_count as usize,
            nullable_count as usize,
            max_symbol_data,
        );

        let buffer = Vec::with_capacity(max_size);

        Ok(DynamicRecorder {
            schema_id,
            field_descriptors,
            metadata_entries,
            metadata_symbols,
            metadata_count,
            int64_count,
            uint64_count,
            float64_count,
            bool_count,
            string_count,
            nullable_count,
            ttl: self.ttl,
            buffer,
        })
    }
}

// ── FieldDesc ────────────────────────────────────────────────────────────

/// Internal descriptor for a single registered field.
#[derive(Debug, Clone)]
struct FieldDesc {
    field_id: u8,
    col_type: ColumnType,
    value_type: DynamicValueType,
}

// ── DynamicRecorder ──────────────────────────────────────────────────────

/// Runtime table builder — the "no struct needed" path.
///
/// Pre-computes wire layout at construction time; calls to
/// [`record()`](DynamicRecorder::record) are allocation-free on the hot path
/// (after the first call that sizes the buffer).
pub struct DynamicRecorder {
    /// Deterministic schema identifier — hash of table_name + sorted fields
    /// + sorted metadata.
    pub schema_id: u32,
    /// Per-field descriptors (positionally indexed).
    field_descriptors: Vec<FieldDesc>,
    /// Pre-computed metadata entry lengths.
    metadata_entries: Vec<(u16, u16)>,
    /// Concatenated metadata key+value bytes (go into symbolTable).
    metadata_symbols: Vec<u8>,

    // Pre-computed counts (for worst-case buffer sizing).
    #[expect(dead_code)]
    metadata_count: u16,
    #[expect(dead_code)]
    int64_count: u16,
    #[expect(dead_code)]
    uint64_count: u16,
    #[expect(dead_code)]
    float64_count: u16,
    #[expect(dead_code)]
    bool_count: u16,
    #[expect(dead_code)]
    string_count: u16,
    #[expect(dead_code)]
    nullable_count: u16,

    /// Table-level TTL policy, if any.  Used when generating DDL for the
    /// corresponding ClickHouse table.
    pub ttl: Option<TtlConfig>,

    /// Pre-allocated buffer reused on every [`record()`] call.
    buffer: Vec<u8>,
}

impl DynamicRecorder {
    /// The SBE header bytes for a DynamicRow message.
    const HEADER: [u8; 8] = DynamicRowEncoder::HEADER_TEMPLATE;

    /// Encode one row of positional values into the internal buffer.
    ///
    /// The returned byte slice references the internal buffer — it is valid
    /// until the next call to [`record()`].
    ///
    /// # Errors
    ///
    /// - [`ValueCountMismatch`] if `values.len()` != the number of registered
    ///   fields.
    /// - [`ValueTypeMismatch`] if a value's variant does not match the
    ///   registered column type for that position.
    ///
    /// [`ValueCountMismatch`]: DynamicRecorderError::ValueCountMismatch
    /// [`ValueTypeMismatch`]: DynamicRecorderError::ValueTypeMismatch
    pub fn record(&mut self, values: &[DynamicValue]) -> Result<&[u8], DynamicRecorderError> {
        if values.len() != self.field_descriptors.len() {
            return Err(DynamicRecorderError::ValueCountMismatch {
                expected: self.field_descriptors.len(),
                actual: values.len(),
            });
        }

        // ---- 1. Single pass: compute per-type counts, accumulate string
        //        lengths, and validate value types.

        let mut actual_int64 = 0u16;
        let mut actual_uint64 = 0u16;
        let mut actual_float64 = 0u16;
        let mut actual_bool = 0u16;
        let mut actual_string = 0u16;
        let mut actual_null = 0u16;
        let mut string_data_len = 0usize;

        for (i, (v, fd)) in values.iter().zip(&self.field_descriptors).enumerate() {
            // Validate variant matches registered column type.
            let expected = fd.value_type;
            match (&expected, v) {
                (DynamicValueType::Int64, DynamicValue::Int64(_)) => actual_int64 += 1,
                (DynamicValueType::UInt64, DynamicValue::UInt64(_)) => actual_uint64 += 1,
                (DynamicValueType::Float64, DynamicValue::Float64(_)) => actual_float64 += 1,
                (DynamicValueType::Bool, DynamicValue::Bool(_)) => actual_bool += 1,
                (DynamicValueType::String, DynamicValue::String(s)) => {
                    actual_string += 1;
                    string_data_len += s.len();
                }
                // Null is always valid regardless of column type.
                (_, DynamicValue::Null) => actual_null += 1,
                // DecimalArray — V2 only; rejected by V1 recorder.
                (_, DynamicValue::DecimalArray(_)) => {
                    return Err(DynamicRecorderError::UnsupportedColumnType {
                        column_name: format!("position {i}"),
                        column_type: ColumnType::Array(Box::new(ColumnType::Decimal {
                            precision: 38,
                            scale: 18,
                        })),
                    });
                }
                _ => {
                    let expected_name = match expected {
                        DynamicValueType::Int64 => "Int64",
                        DynamicValueType::UInt64 => "UInt64",
                        DynamicValueType::Float64 => "Float64",
                        DynamicValueType::Bool => "Bool",
                        DynamicValueType::String => "String",
                        DynamicValueType::DecimalArray => "DecimalArray",
                    };
                    let actual_name = match v {
                        DynamicValue::Int64(_) => "Int64",
                        DynamicValue::UInt64(_) => "UInt64",
                        DynamicValue::Float64(_) => "Float64",
                        DynamicValue::Bool(_) => "Bool",
                        DynamicValue::String(_) => "String",
                        DynamicValue::Null => "Null",
                        DynamicValue::DecimalArray(_) => "DecimalArray",
                    };
                    return Err(DynamicRecorderError::ValueTypeMismatch {
                        position: i,
                        expected: expected_name,
                        actual: actual_name,
                    });
                }
            }
        }

        // ---- 2. Compute encoded size and ensure buffer capacity.

        let symbol_total = self.metadata_symbols.len() + string_data_len;

        let total_size = compute_encoded_size(
            self.metadata_entries.len(),
            actual_int64 as usize,
            actual_uint64 as usize,
            actual_float64 as usize,
            actual_bool as usize,
            actual_string as usize,
            actual_null as usize,
            symbol_total,
        );

        if total_size > self.buffer.len() {
            self.buffer.resize(total_size, 0);
        }

        // ---- 3. Write the SBE message into the buffer.

        let buf = self.buffer.as_mut_slice();

        // 3a. SBE header (8 bytes).
        buf[0..8].copy_from_slice(&Self::HEADER);

        // 3b. schemaId field (4 bytes at offset 8).
        buf[8..12].copy_from_slice(&self.schema_id.to_le_bytes());

        // 3c. Compute wire offsets for each group.
        let off_meta_dim = 12usize;
        let off_meta_entries = off_meta_dim + 4;

        let off_int64_dim = off_meta_entries + self.metadata_entries.len() * 4;
        let off_int64_entries = off_int64_dim + 4;
        let int64_entries_size = actual_int64 as usize * 9;

        let off_uint64_dim = off_int64_entries + int64_entries_size;
        let off_uint64_entries = off_uint64_dim + 4;
        let uint64_entries_size = actual_uint64 as usize * 9;

        let off_float64_dim = off_uint64_entries + uint64_entries_size;
        let off_float64_entries = off_float64_dim + 4;
        let float64_entries_size = actual_float64 as usize * 9;

        let off_bool_dim = off_float64_entries + float64_entries_size;
        let off_bool_entries = off_bool_dim + 4;
        let bool_entries_size = actual_bool as usize * 2;

        let off_string_dim = off_bool_entries + bool_entries_size;
        let off_string_entries = off_string_dim + 4;
        let string_entries_size = actual_string as usize * 3;

        let off_null_dim = off_string_entries + string_entries_size;
        let off_null_entries = off_null_dim + 4;
        let null_entries_size = actual_null as usize;

        let off_symbol = off_null_entries + null_entries_size;

        // 3d. Write metadata group dim header.
        // blockLength = 4 (entry size: 2 keyLen + 2 valLen)
        buf[off_meta_dim..off_meta_dim + 2].copy_from_slice(&4u16.to_le_bytes());
        buf[off_meta_dim + 2..off_meta_dim + 4]
            .copy_from_slice(&(self.metadata_entries.len() as u16).to_le_bytes());

        // 3e. Write metadata entries.
        let mut meta_write = off_meta_entries;
        for &(kl, vl) in &self.metadata_entries {
            buf[meta_write..meta_write + 2].copy_from_slice(&kl.to_le_bytes());
            buf[meta_write + 2..meta_write + 4].copy_from_slice(&vl.to_le_bytes());
            meta_write += 4;
        }

        // 3f. Write per-type group dim headers.
        // Int64 group: blockLength = 9 (fieldId 1 + value 8)
        buf[off_int64_dim..off_int64_dim + 2].copy_from_slice(&9u16.to_le_bytes());
        buf[off_int64_dim + 2..off_int64_dim + 4].copy_from_slice(&actual_int64.to_le_bytes());

        // UInt64 group: blockLength = 9
        buf[off_uint64_dim..off_uint64_dim + 2].copy_from_slice(&9u16.to_le_bytes());
        buf[off_uint64_dim + 2..off_uint64_dim + 4].copy_from_slice(&actual_uint64.to_le_bytes());

        // Float64 group: blockLength = 9
        buf[off_float64_dim..off_float64_dim + 2].copy_from_slice(&9u16.to_le_bytes());
        buf[off_float64_dim + 2..off_float64_dim + 4]
            .copy_from_slice(&actual_float64.to_le_bytes());

        // Bool group: blockLength = 2 (fieldId 1 + value 1)
        buf[off_bool_dim..off_bool_dim + 2].copy_from_slice(&2u16.to_le_bytes());
        buf[off_bool_dim + 2..off_bool_dim + 4].copy_from_slice(&actual_bool.to_le_bytes());

        // String group: blockLength = 3 (fieldId 1 + strLen 2)
        buf[off_string_dim..off_string_dim + 2].copy_from_slice(&3u16.to_le_bytes());
        buf[off_string_dim + 2..off_string_dim + 4].copy_from_slice(&actual_string.to_le_bytes());

        // Null group: blockLength = 1 (fieldId 1)
        buf[off_null_dim..off_null_dim + 2].copy_from_slice(&1u16.to_le_bytes());
        buf[off_null_dim + 2..off_null_dim + 4].copy_from_slice(&actual_null.to_le_bytes());

        // 3g. Write field entries by dispatching each value to its group.
        let mut i64_w = off_int64_entries;
        let mut u64_w = off_uint64_entries;
        let mut f64_w = off_float64_entries;
        let mut bl_w = off_bool_entries;
        let mut str_w = off_string_entries;
        let mut nul_w = off_null_entries;

        for (v, fd) in values.iter().zip(&self.field_descriptors) {
            let fid = fd.field_id;
            match v {
                DynamicValue::Int64(val) => {
                    buf[i64_w] = fid;
                    buf[i64_w + 1..i64_w + 9].copy_from_slice(&val.to_le_bytes());
                    i64_w += 9;
                }
                DynamicValue::UInt64(val) => {
                    buf[u64_w] = fid;
                    buf[u64_w + 1..u64_w + 9].copy_from_slice(&val.to_le_bytes());
                    u64_w += 9;
                }
                DynamicValue::Float64(val) => {
                    buf[f64_w] = fid;
                    buf[f64_w + 1..f64_w + 9].copy_from_slice(&val.to_le_bytes());
                    f64_w += 9;
                }
                DynamicValue::Bool(val) => {
                    buf[bl_w] = fid;
                    buf[bl_w + 1] = if *val { 1 } else { 0 };
                    bl_w += 2;
                }
                DynamicValue::String(s) => {
                    buf[str_w] = fid;
                    let slen = s.len() as u16;
                    buf[str_w + 1..str_w + 3].copy_from_slice(&slen.to_le_bytes());
                    str_w += 3;
                }
                DynamicValue::Null => {
                    buf[nul_w] = fid;
                    nul_w += 1;
                }
                DynamicValue::DecimalArray(_) => {
                    return Err(DynamicRecorderError::Encode(
                        "DecimalArray requires V2 recorder".into(),
                    ));
                }
            }
        }

        // 3h. Write symbolTable varData: 4-byte length + concatenated bytes.
        buf[off_symbol..off_symbol + 4].copy_from_slice(&(symbol_total as u32).to_le_bytes());
        let mut sym_w = off_symbol + 4;

        // Metadata symbols first.
        if !self.metadata_symbols.is_empty() {
            buf[sym_w..sym_w + self.metadata_symbols.len()].copy_from_slice(&self.metadata_symbols);
            sym_w += self.metadata_symbols.len();
        }

        // Then string values in field order.
        for v in values.iter() {
            if let DynamicValue::String(s) = v {
                buf[sym_w..sym_w + s.len()].copy_from_slice(s.as_bytes());
                sym_w += s.len();
            }
        }

        Ok(&self.buffer[..total_size])
    }
}

// ── DynamicRecorderV2 ────────────────────────────────────────────────────

impl DynamicValueType {
    /// V2 mapping: like [`from_column_type`](Self::from_column_type) but
    /// additionally supports `Array(Decimal(p, s))` columns.
    fn from_column_type_v2(ct: &ColumnType) -> Option<Self> {
        if let Some(vt) = Self::from_column_type(ct) {
            return Some(vt);
        }
        let inner = match ct {
            ColumnType::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        match inner {
            ColumnType::Array(elem) if matches!(**elem, ColumnType::Decimal { .. }) => {
                Some(Self::DecimalArray)
            }
            _ => None,
        }
    }
}

impl DynamicRecorderBuilder {
    /// Build a V2 recorder (`DynamicRowV2`, template ID 4) supporting
    /// `Array(Decimal(p, s))` columns in addition to the V1 types.
    ///
    /// # Errors
    ///
    /// Same construction errors as [`build`](Self::build).
    pub fn build_v2(self) -> Result<DynamicRecorderV2, DynamicRecorderError> {
        if self.table_name.is_empty() {
            return Err(DynamicRecorderError::EmptyTableName);
        }
        if self.fields.is_empty() {
            return Err(DynamicRecorderError::NoFields);
        }
        for (name, ct) in &self.fields {
            if DynamicValueType::from_column_type_v2(ct).is_none() {
                return Err(DynamicRecorderError::UnsupportedColumnType {
                    column_name: name.clone(),
                    column_type: ct.clone(),
                });
            }
        }

        let schema_id = compute_schema_id(&self.table_name, &self.fields, &self.metadata);

        let field_descriptors: Vec<FieldDesc> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, (_name, ct))| FieldDesc {
                field_id: i as u8,
                col_type: ct.clone(),
                value_type: DynamicValueType::from_column_type_v2(ct).expect("validated above"),
            })
            .collect();

        let mut metadata_entries: Vec<(u16, u16)> = Vec::new();
        let mut metadata_symbols: Vec<u8> = Vec::new();
        for (k, v) in &self.metadata {
            metadata_entries.push((k.len() as u16, v.len() as u16));
            metadata_symbols.extend_from_slice(k.as_bytes());
            metadata_symbols.extend_from_slice(v.as_bytes());
        }

        Ok(DynamicRecorderV2 {
            schema_id,
            column_names: self.fields.iter().map(|(n, _)| n.clone()).collect(),
            table_name: self.table_name,
            field_descriptors,
            metadata_entries,
            metadata_symbols,
            ttl: self.ttl,
        })
    }
}

/// V2 schema wire type codes.
///
/// `outerType`: 0 = scalar, 1 = `Array(T)`, 2 = `Nullable(T)`.
/// `innerType`: 1 = Int64, 2 = UInt64, 3 = Float64, 4 = Bool, 5 = String,
/// 6 = Decimal (with `precision`/`scale` set). 0 = unknown.
fn v2_type_codes(ct: &ColumnType) -> (u8, u8, u8, u8) {
    match ct {
        ColumnType::Nullable(inner) => {
            let (_, i, p, s) = v2_type_codes(inner);
            (2, i, p, s)
        }
        ColumnType::Array(inner) => {
            let (_, i, p, s) = v2_type_codes(inner);
            (1, i, p, s)
        }
        ColumnType::Decimal { precision, scale } => (0, 6, *precision, *scale),
        ColumnType::Int8 | ColumnType::Int16 | ColumnType::Int32 | ColumnType::Int64 => {
            (0, 1, 0, 0)
        }
        ColumnType::UInt8 | ColumnType::UInt16 | ColumnType::UInt32 | ColumnType::UInt64 => {
            (0, 2, 0, 0)
        }
        ColumnType::Float32 | ColumnType::Float64 => (0, 3, 0, 0),
        ColumnType::Bool => (0, 4, 0, 0),
        ColumnType::String | ColumnType::FixedString(_) => (0, 5, 0, 0),
        _ => (0, 0, 0, 0),
    }
}

/// Per-call value tallies used by both length computation and encoding.
#[derive(Default)]
struct V2Counts {
    int64: u16,
    uint64: u16,
    float64: u16,
    bools: u16,
    strings: u16,
    nulls: u16,
    decimal_arrays: u16,
    decimal_values: usize,
    string_len: usize,
}

/// V2 recorder: borrowed values, caller-buffer encoding, `Array(Decimal)`
/// support. Publication encodes with [`record_into`](Self::record_into)
/// directly inside an Aeron claim — no owned intermediate buffer.
pub struct DynamicRecorderV2 {
    /// Deterministic schema identifier (same derivation as V1).
    schema_id: u32,
    table_name: String,
    column_names: Vec<String>,
    field_descriptors: Vec<FieldDesc>,
    metadata_entries: Vec<(u16, u16)>,
    metadata_symbols: Vec<u8>,
    /// Table-level TTL policy, if any.
    pub ttl: Option<TtlConfig>,
}

impl DynamicRecorderV2 {
    /// The deterministic schema identifier for this table definition.
    #[must_use]
    pub fn schema_id(&self) -> u32 {
        self.schema_id
    }

    /// Validate positional values against the registered columns and tally
    /// per-group counts.
    fn validate(&self, values: &[DynamicValueRef<'_>]) -> Result<V2Counts, DynamicRecorderError> {
        if values.len() != self.field_descriptors.len() {
            return Err(DynamicRecorderError::ValueCountMismatch {
                expected: self.field_descriptors.len(),
                actual: values.len(),
            });
        }
        let mut c = V2Counts::default();
        for (i, (v, fd)) in values.iter().zip(&self.field_descriptors).enumerate() {
            match (fd.value_type, v) {
                (DynamicValueType::Int64, DynamicValueRef::Int64(_)) => c.int64 += 1,
                (DynamicValueType::UInt64, DynamicValueRef::UInt64(_)) => c.uint64 += 1,
                (DynamicValueType::Float64, DynamicValueRef::Float64(_)) => c.float64 += 1,
                (DynamicValueType::Bool, DynamicValueRef::Bool(_)) => c.bools += 1,
                (DynamicValueType::String, DynamicValueRef::String(s)) => {
                    c.strings += 1;
                    c.string_len += s.len();
                }
                (DynamicValueType::DecimalArray, DynamicValueRef::DecimalArray(arr)) => {
                    c.decimal_arrays += 1;
                    c.decimal_values += arr.len();
                }
                (_, DynamicValueRef::Null) => c.nulls += 1,
                (expected, actual) => {
                    return Err(DynamicRecorderError::ValueTypeMismatch {
                        position: i,
                        expected: expected.name(),
                        actual: actual.name(),
                    });
                }
            }
        }
        Ok(c)
    }

    fn encoded_len(&self, c: &V2Counts) -> usize {
        // The generated helper covers every group's dim header and fixed
        // entry bytes; nested decimal `values` groups (4-byte dim + 9 bytes
        // per value) are dynamic per entry and added here.
        crate::sbe::v2::DynamicRowV2Encoder::compute_encoded_length_with_message_header(
            self.metadata_entries.len(),
            c.int64 as usize,
            c.uint64 as usize,
            c.float64 as usize,
            c.bools as usize,
            c.strings as usize,
            c.nulls as usize,
            c.decimal_arrays as usize,
            self.metadata_symbols.len() + c.string_len,
        ) + c.decimal_arrays as usize * 4
            + c.decimal_values * 9
    }

    /// Exact encoded length (header + body) for one row of values.
    ///
    /// # Errors
    ///
    /// [`ValueCountMismatch`](DynamicRecorderError::ValueCountMismatch) or
    /// [`ValueTypeMismatch`](DynamicRecorderError::ValueTypeMismatch).
    pub fn compute_encoded_length(
        &self,
        values: &[DynamicValueRef<'_>],
    ) -> Result<usize, DynamicRecorderError> {
        Ok(self.encoded_len(&self.validate(values)?))
    }

    /// Encode one `DynamicRowV2` message directly into `dst` and return the
    /// encoded prefix. Zero-allocation: all values are borrowed and the
    /// caller owns the buffer (typically an Aeron claim).
    ///
    /// # Errors
    ///
    /// Value count/type mismatches, or
    /// [`Encode`](DynamicRecorderError::Encode) when `dst` is too short.
    pub fn record_into<'a>(
        &self,
        dst: &'a mut [u8],
        values: &[DynamicValueRef<'_>],
    ) -> Result<&'a [u8], DynamicRecorderError> {
        use crate::sbe::v2::DynamicRowV2Encoder;

        let counts = self.validate(values)?;
        let total = self.encoded_len(&counts);
        let enc_err =
            |e: crate::sbe::v2::sbe_rt::EncodeError| DynamicRecorderError::Encode(e.to_string());

        {
            let mut enc =
                DynamicRowV2Encoder::wrap_and_apply_header(&mut dst[..], 0).map_err(enc_err)?;
            let _ = enc.schema_id(self.schema_id);

            let after = enc
                .row_metadata(self.metadata_entries.len() as u16, |g| {
                    for &(kl, vl) in &self.metadata_entries {
                        let _ = g.add(|e| {
                            let _ = e.key_len(kl).val_len(vl);
                        });
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .int64_fields(counts.int64, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::Int64(x) = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id).value(*x);
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .uint64_fields(counts.uint64, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::UInt64(x) = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id).value(*x);
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .float64_fields(counts.float64, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::Float64(x) = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id).value(*x);
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .bool_fields(counts.bools, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::Bool(x) = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id).value(u8::from(*x));
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .string_fields(counts.strings, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::String(s) = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id).str_len(s.len() as u16);
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .null_fields(counts.nulls, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::Null = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id);
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .decimal_array_fields(counts.decimal_arrays, |g| {
                    for (v, fd) in values.iter().zip(&self.field_descriptors) {
                        if let DynamicValueRef::DecimalArray(arr) = v {
                            let _ = g.add(|e| {
                                let _ = e.field_id(fd.field_id);
                                let _ = e.values(arr.len() as u16, |vg| {
                                    for &(m, x) in arr.iter() {
                                        let _ = vg.add(|ve| {
                                            let _ = ve.mantissa(m).exponent(x);
                                        });
                                    }
                                });
                            });
                        }
                    }
                })
                .map_err(enc_err)?;

            let sym_len = self.metadata_symbols.len() + counts.string_len;
            let _complete = after
                .symbol_table_with::<crate::sbe::v2::sbe_rt::EncodeError, _>(sym_len, |out| {
                    let mut w = self.metadata_symbols.len();
                    out[..w].copy_from_slice(&self.metadata_symbols);
                    for v in values {
                        if let DynamicValueRef::String(s) = v {
                            out[w..w + s.len()].copy_from_slice(s.as_bytes());
                            w += s.len();
                        }
                    }
                    Ok(())
                })
                .map_err(enc_err)?;
        }

        Ok(&dst[..total])
    }

    /// Exact encoded length of this table's `DynamicSchemaV2` message.
    #[must_use]
    pub fn schema_encoded_length(&self) -> usize {
        let names_len: usize = self.column_names.iter().map(String::len).sum();
        crate::sbe::v2::DynamicSchemaV2Encoder::compute_encoded_length_with_message_header(
            self.metadata_entries.len(),
            self.column_names.len(),
            self.table_name.len(),
            self.metadata_symbols.len() + names_len,
        )
    }

    /// Encode this table's `DynamicSchemaV2` (template 3) into `dst`.
    /// Symbol layout: metadata key/value pairs, then column names in field
    /// order.
    ///
    /// # Errors
    ///
    /// [`Encode`](DynamicRecorderError::Encode) when `dst` is too short.
    pub fn schema_into<'a>(&self, dst: &'a mut [u8]) -> Result<&'a [u8], DynamicRecorderError> {
        use crate::sbe::v2::DynamicSchemaV2Encoder;

        let total = self.schema_encoded_length();
        let enc_err =
            |e: crate::sbe::v2::sbe_rt::EncodeError| DynamicRecorderError::Encode(e.to_string());

        {
            let mut enc =
                DynamicSchemaV2Encoder::wrap_and_apply_header(&mut dst[..], 0).map_err(enc_err)?;
            let _ = enc.schema_id(self.schema_id);

            let after = enc
                .metadata(self.metadata_entries.len() as u16, |g| {
                    for &(kl, vl) in &self.metadata_entries {
                        let _ = g.add(|e| {
                            let _ = e.key_len(kl).val_len(vl);
                        });
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .columns(self.column_names.len() as u16, |g| {
                    for (name, fd) in self.column_names.iter().zip(&self.field_descriptors) {
                        let (outer, inner, precision, scale) = v2_type_codes(&fd.col_type);
                        let _ = g.add(|e| {
                            let _ = e
                                .field_id(fd.field_id)
                                .name_len(name.len() as u16)
                                .outer_type(outer)
                                .inner_type(inner)
                                .precision(precision)
                                .scale(scale);
                        });
                    }
                })
                .map_err(enc_err)?;

            let after = after
                .table_name(self.table_name.as_bytes())
                .map_err(enc_err)?;

            let names_len: usize = self.column_names.iter().map(String::len).sum();
            let sym_len = self.metadata_symbols.len() + names_len;
            let _complete = after
                .symbol_table_with::<crate::sbe::v2::sbe_rt::EncodeError, _>(sym_len, |out| {
                    let mut w = self.metadata_symbols.len();
                    out[..w].copy_from_slice(&self.metadata_symbols);
                    for name in &self.column_names {
                        out[w..w + name.len()].copy_from_slice(name.as_bytes());
                        w += name.len();
                    }
                    Ok(())
                })
                .map_err(enc_err)?;
        }

        Ok(&dst[..total])
    }
}

impl DynamicValueType {
    const fn name(self) -> &'static str {
        match self {
            Self::Int64 => "Int64",
            Self::UInt64 => "UInt64",
            Self::Float64 => "Float64",
            Self::Bool => "Bool",
            Self::String => "String",
            Self::DecimalArray => "DecimalArray",
        }
    }
}

impl DynamicValueRef<'_> {
    const fn name(&self) -> &'static str {
        match self {
            Self::Int64(_) => "Int64",
            Self::UInt64(_) => "UInt64",
            Self::Float64(_) => "Float64",
            Self::Bool(_) => "Bool",
            Self::String(_) => "String",
            Self::Null => "Null",
            Self::DecimalArray(_) => "DecimalArray",
        }
    }
}

// ── Schema ID determinism ────────────────────────────────────────────────

/// Compute a deterministic `schema_id` for the given table definition.
///
/// The hash input is `table_name || sorted(field_name + field_type) ||
/// sorted(metadata_key + metadata_value)`.  Sorting ensures that two
/// builders that register fields/metadata in different orders produce the
/// same schema id, so consumers can discover the schema on first sight.
fn compute_schema_id(
    table_name: &str,
    fields: &[(String, ColumnType)],
    metadata: &[(String, String)],
) -> u32 {
    let mut h = DefaultHasher::new();

    table_name.hash(&mut h);

    // Sort fields by name for determinism.
    let mut sorted_fields: Vec<&(String, ColumnType)> = fields.iter().collect();
    sorted_fields.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, ct) in &sorted_fields {
        name.hash(&mut h);
        ct.to_string().hash(&mut h);
    }

    // Sort metadata by key for determinism.
    let mut sorted_meta: Vec<&(String, String)> = metadata.iter().collect();
    sorted_meta.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, v) in &sorted_meta {
        k.hash(&mut h);
        v.hash(&mut h);
    }

    h.finish() as u32
}

// ── Size computation ─────────────────────────────────────────────────────

/// Compute the total encoded byte size of a DynamicRow message given the
/// per-group entry counts and the symbol table byte length.
#[expect(clippy::too_many_arguments)]
const fn compute_encoded_size(
    meta_entries: usize,
    int64_cnt: usize,
    uint64_cnt: usize,
    float64_cnt: usize,
    bool_cnt: usize,
    string_cnt: usize,
    null_cnt: usize,
    symbol_data_len: usize,
) -> usize {
    // 8 header + 4 schemaId
    let hdr = 12;
    let meta_size = 4 + meta_entries * 4;
    let int64_size = 4 + int64_cnt * 9;
    let uint64_size = 4 + uint64_cnt * 9;
    let float64_size = 4 + float64_cnt * 9;
    let bool_size = 4 + bool_cnt * 2;
    let string_size = 4 + string_cnt * 3;
    let null_size = 4 + null_cnt;
    let var_data_size = 4 + symbol_data_len;
    hdr + meta_size
        + int64_size
        + uint64_size
        + float64_size
        + bool_size
        + string_size
        + null_size
        + var_data_size
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────

    fn simple_recorder() -> DynamicRecorder {
        DynamicRecorderBuilder::new("test_table")
            .field("price", ColumnType::Float64)
            .field("qty", ColumnType::UInt64)
            .field("symbol", ColumnType::String)
            .field("is_active", ColumnType::Bool)
            .build()
            .unwrap()
    }

    fn simple_values() -> Vec<DynamicValue> {
        vec![
            DynamicValue::Float64(100.50),
            DynamicValue::UInt64(1000),
            DynamicValue::String("AAPL".into()),
            DynamicValue::Bool(true),
        ]
    }

    /// Fully-decoded row via the consuming-stage decoder chain.
    struct RowParts {
        schema_id: u32,
        meta: Vec<(u16, u16)>,
        i64s: Vec<(u8, i64)>,
        u64s: Vec<(u8, u64)>,
        f64s: Vec<(u8, f64)>,
        bools: Vec<(u8, u8)>,
        strs: Vec<(u8, u16)>,
        nulls: Vec<u8>,
        symbols: Vec<u8>,
    }

    fn decode_parts(buf: &[u8]) -> RowParts {
        use crate::sbe::DynamicRowDecoder;
        let dec = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();
        let schema_id = dec.schema_id();
        let mut g = dec.into_row_metadata().unwrap();
        let meta = g.by_ref().map(|e| (e.key_len(), e.val_len())).collect();
        let dec = g.finish().unwrap();
        let mut g = dec.into_int64_fields().unwrap();
        let i64s = g.by_ref().map(|e| (e.field_id(), e.value())).collect();
        let dec = g.finish().unwrap();
        let mut g = dec.into_uint64_fields().unwrap();
        let u64s = g.by_ref().map(|e| (e.field_id(), e.value())).collect();
        let dec = g.finish().unwrap();
        let mut g = dec.into_float64_fields().unwrap();
        let f64s = g.by_ref().map(|e| (e.field_id(), e.value())).collect();
        let dec = g.finish().unwrap();
        let mut g = dec.into_bool_fields().unwrap();
        let bools = g.by_ref().map(|e| (e.field_id(), e.value())).collect();
        let dec = g.finish().unwrap();
        let mut g = dec.into_string_fields().unwrap();
        let strs = g.by_ref().map(|e| (e.field_id(), e.str_len())).collect();
        let dec = g.finish().unwrap();
        let mut g = dec.into_null_fields().unwrap();
        let nulls = g.by_ref().map(|e| e.field_id()).collect();
        let dec = g.finish().unwrap();
        let (symbols, _) = dec.into_symbol_table().unwrap();
        RowParts {
            schema_id,
            meta,
            i64s,
            u64s,
            f64s,
            bools,
            strs,
            nulls,
            symbols: symbols.to_vec(),
        }
    }

    // ── build + record ───────────────────────────────────────────────

    #[test]
    fn test_build_and_record() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = simple_recorder();
        let values = simple_values();
        let schema_id = rec.schema_id;

        let buf = rec.record(&values).unwrap().to_vec();
        assert!(!buf.is_empty());

        let parts = decode_parts(&buf);
        assert_eq!(parts.schema_id, schema_id);
        assert!(parts.meta.is_empty());
        assert!(parts.i64s.is_empty());
        assert_eq!(parts.u64s, vec![(1, 1000)]);
        assert_eq!(parts.f64s, vec![(0, 100.50)]);
        assert_eq!(parts.bools, vec![(3, 1)]);
        assert_eq!(parts.strs, vec![(2, 4)]);
        assert!(parts.nulls.is_empty());
        assert_eq!(parts.symbols, b"AAPL");
    
        Ok(())
    }

    // ── schema_id determinism ─────────────────────────────────────────

    #[test]
    fn test_schema_id_determinism() -> Result<(), Box<dyn std::error::Error>> {
        let rec1 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .metadata("k1", "v1")
            .build()
            .unwrap();

        // Same registration, same metadata → same schema_id.
        let rec2 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .metadata("k1", "v1")
            .build()
            .unwrap();
        assert_eq!(rec1.schema_id, rec2.schema_id);

        // Different field → different schema_id.
        let rec3 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("c", ColumnType::String) // different name
            .metadata("k1", "v1")
            .build()
            .unwrap();
        assert_ne!(rec1.schema_id, rec3.schema_id);

        // Different metadata → different schema_id.
        let rec4 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .metadata("k1", "v2") // different value
            .build()
            .unwrap();
        assert_ne!(rec1.schema_id, rec4.schema_id);

        // Different registration order → same schema_id (sorted internally).
        let rec5 = DynamicRecorderBuilder::new("t")
            .field("b", ColumnType::String)
            .field("a", ColumnType::Int64)
            .metadata("k1", "v1")
            .build()
            .unwrap();
        assert_eq!(rec1.schema_id, rec5.schema_id);
    
        Ok(())
    }

    // ── metadata ──────────────────────────────────────────────────────

    #[test]
    fn test_metadata_values_present() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("test")
            .field("val", ColumnType::Int64)
            .metadata("source", "exchange_a")
            .metadata("env", "prod")
            .build()
            .unwrap();

        let buf = rec.record(&[DynamicValue::Int64(42)]).unwrap().to_vec();
        let parts = decode_parts(&buf);

        assert_eq!(parts.meta.len(), 2);

        // Verify symbol table contains metadata key+value strings.
        let sym = &parts.symbols;
        assert!(sym.len() >= 6 + 10 + 3 + 4);
        assert!(sym.windows(6).any(|w| w == b"source"));
        assert!(sym.windows(10).any(|w| w == b"exchange_a"));
        assert!(sym.windows(3).any(|w| w == b"env"));
        assert!(sym.windows(4).any(|w| w == b"prod"));
    
        Ok(())
    }

    #[test]
    fn test_metadata_consistency() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Int64)
            .metadata("key1", "val1")
            .build()
            .unwrap();

        let buf1 = rec.record(&[DynamicValue::Int64(1)]).unwrap().to_vec();
        let buf2 = rec.record(&[DynamicValue::Int64(2)]).unwrap().to_vec();

        // Metadata should be byte-identical across calls.
        let p1 = decode_parts(&buf1);
        let p2 = decode_parts(&buf2);
        assert_eq!(p1.meta.len(), p2.meta.len());
        assert_eq!(p1.symbols, p2.symbols);
    
        Ok(())
    }

    // ── String values ─────────────────────────────────────────────────

    #[test]
    fn test_string_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("name", ColumnType::String)
            .field("code", ColumnType::String)
            .build()
            .unwrap();

        let buf = rec
            .record(&[
                DynamicValue::String("hello".into()),
                DynamicValue::String("abc".into()),
            ])
            .unwrap()
            .to_vec();

        let parts = decode_parts(&buf);
        assert_eq!(parts.strs, vec![(0, 5), (1, 3)]);
        assert_eq!(parts.symbols, b"helloabc");
    
        Ok(())
    }

    #[test]
    fn test_string_symbol_table_with_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("msg", ColumnType::String)
            .metadata("tag", "xyz")
            .build()
            .unwrap();

        let buf = rec
            .record(&[DynamicValue::String("data".into())])
            .unwrap()
            .to_vec();
        let parts = decode_parts(&buf);
        // Metadata first: "tag" (3) + "xyz" (3) = 6 bytes, then string
        // "data" (4) = 10 bytes total
        assert_eq!(parts.symbols.len(), 3 + 3 + 4);
        assert!(parts.symbols.starts_with(b"tagxyz"));
        assert!(parts.symbols.ends_with(b"data"));
    
        Ok(())
    }

    // ── Null values ───────────────────────────────────────────────────

    #[test]
    fn test_null_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("val", ColumnType::Int64)
            .field("name", ColumnType::String)
            .build()
            .unwrap();

        let buf = rec
            .record(&[DynamicValue::Null, DynamicValue::Null])
            .unwrap()
            .to_vec();

        let parts = decode_parts(&buf);
        assert!(parts.i64s.is_empty());
        assert!(parts.strs.is_empty());
        assert_eq!(parts.nulls, vec![0, 1]);
    
        Ok(())
    }

    // ── Empty metadata ────────────────────────────────────────────────

    #[test]
    fn test_empty_metadata_produces_valid_sbe() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Float64)
            .build()
            .unwrap();

        let buf = rec.record(&[DynamicValue::Float64(1.0)]).unwrap().to_vec();
        let parts = decode_parts(&buf);
        assert!(parts.meta.is_empty());
        assert_eq!(parts.f64s.len(), 1);
    
        Ok(())
    }

    // ── Multi-key metadata ────────────────────────────────────────────

    #[test]
    fn test_multiple_metadata_keys() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Int64)
            .metadata("a", "1")
            .metadata("b", "2")
            .metadata("c", "3")
            .build()
            .unwrap();

        let buf = rec.record(&[DynamicValue::Int64(0)]).unwrap().to_vec();
        assert_eq!(decode_parts(&buf).meta.len(), 3);
    
        Ok(())
    }

    // ── Wrong value count ─────────────────────────────────────────────

    #[test]
    fn test_wrong_value_count_errors() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = simple_recorder();
        let err = rec.record(&[DynamicValue::Int64(1)]).unwrap_err();
        assert!(matches!(
            err,
            DynamicRecorderError::ValueCountMismatch { .. }
        ));
    
        Ok(())
    }

    // ── Value type mismatch ───────────────────────────────────────────

    #[test]
    fn test_value_type_mismatch_errors() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("price", ColumnType::Float64)
            .build()
            .unwrap();
        let err = rec.record(&[DynamicValue::Int64(42)]).unwrap_err();
        assert!(matches!(
            err,
            DynamicRecorderError::ValueTypeMismatch { .. }
        ));
    
        Ok(())
    }

    // ── 100k loop — no allocation ─────────────────────────────────────

    #[test]
    fn test_no_allocation_loop() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("price", ColumnType::Float64)
            .field("qty", ColumnType::UInt64)
            .field("symbol", ColumnType::String)
            .build()
            .unwrap();

        let price = DynamicValue::Float64(100.50);
        let qty = DynamicValue::UInt64(1000);
        let symbol = DynamicValue::String("AAPL".into());
        let values = [price, qty, symbol];

        // First call may resize the buffer.
        let first_buf = rec.record(&values).unwrap().len();
        let cap = rec.buffer.capacity();

        // Subsequent calls should not change capacity.
        for _ in 0..100_000 {
            let buf = rec.record(&values).unwrap();
            assert_eq!(buf.len(), first_buf);
            assert_eq!(rec.buffer.capacity(), cap);
        }
    
        Ok(())
    }

    // ── Schema ID w/ metadata ─────────────────────────────────────────

    #[test]
    fn test_schema_id_determinism_with_metadata() -> Result<(), Box<dyn std::error::Error>> {
        // Same fields + same metadata → same schema_id regardless of
        // registration order.
        let a = DynamicRecorderBuilder::new("x")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::UInt64)
            .metadata("z", "1")
            .metadata("y", "2")
            .build()
            .unwrap();

        let b = DynamicRecorderBuilder::new("x")
            .field("b", ColumnType::UInt64)
            .field("a", ColumnType::Int64)
            .metadata("y", "2")
            .metadata("z", "1")
            .build()
            .unwrap();

        assert_eq!(a.schema_id, b.schema_id);

        // Different metadata → different schema_id.
        let c = DynamicRecorderBuilder::new("x")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::UInt64)
            .metadata("z", "9") // changed value
            .metadata("y", "2")
            .build()
            .unwrap();

        assert_ne!(a.schema_id, c.schema_id);
    
        Ok(())
    }

    // ── General round-trip ────────────────────────────────────────────

    #[test]
    fn test_round_trip_all_types() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("rt_test")
            .field("i", ColumnType::Int64)
            .field("u", ColumnType::UInt64)
            .field("f", ColumnType::Float64)
            .field("b", ColumnType::Bool)
            .field("s", ColumnType::String)
            .field("n", ColumnType::Int64) // nullable field (Null value)
            .metadata("rt", "check")
            .build()
            .unwrap();
        let schema_id = rec.schema_id;

        let buf = rec
            .record(&[
                DynamicValue::Int64(-42),
                DynamicValue::UInt64(99),
                DynamicValue::Float64(std::f64::consts::PI),
                DynamicValue::Bool(false),
                DynamicValue::String("hello".into()),
                DynamicValue::Null,
            ])
            .unwrap()
            .to_vec();

        let parts = decode_parts(&buf);
        assert_eq!(parts.schema_id, schema_id);
        assert_eq!(parts.i64s, vec![(0, -42)]);
        assert_eq!(parts.u64s, vec![(1, 99)]);
        assert_eq!(parts.f64s.len(), 1);
        assert!((parts.f64s[0].1 - std::f64::consts::PI).abs() < 1e-10);
        assert_eq!(parts.bools, vec![(3, 0)]); // false
        assert_eq!(parts.strs, vec![(4, 5)]);
        assert_eq!(parts.nulls, vec![5]);

        // Symbol table: metadata "rt"+"check" (2+5=7) + "hello" (5) = 12 bytes
        assert_eq!(parts.symbols.len(), 2 + 5 + 5);
        assert!(parts.symbols.starts_with(b"rtcheck"));
        assert!(parts.symbols.ends_with(b"hello"));
    
        Ok(())
    }

    // ── Nullable column type ──────────────────────────────────────────

    #[test]
    fn test_nullable_column_type() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("val", ColumnType::Nullable(Box::new(ColumnType::Int64)))
            .build()
            .unwrap();

        // Null value is accepted for nullable field.
        let buf = rec.record(&[DynamicValue::Null]).unwrap().to_vec();
        assert_eq!(decode_parts(&buf).nulls.len(), 1);

        // Non-null value is also accepted.
        let buf2 = rec.record(&[DynamicValue::Int64(42)]).unwrap().to_vec();
        assert_eq!(decode_parts(&buf2).i64s.len(), 1);
    
        Ok(())
    }

    // ── All-null values ─────────────────────────────────────────

    #[test]
    fn test_all_null_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("i", ColumnType::Int64)
            .field("u", ColumnType::UInt64)
            .field("f", ColumnType::Float64)
            .field("b", ColumnType::Bool)
            .field("s", ColumnType::String)
            .build()
            .unwrap();
        let schema_id = rec.schema_id;

        let buf = rec
            .record(&[
                DynamicValue::Null,
                DynamicValue::Null,
                DynamicValue::Null,
                DynamicValue::Null,
                DynamicValue::Null,
            ])
            .unwrap()
            .to_vec();

        let parts = decode_parts(&buf);
        assert_eq!(parts.schema_id, schema_id);
        assert!(parts.i64s.is_empty());
        assert!(parts.u64s.is_empty());
        assert!(parts.f64s.is_empty());
        assert!(parts.bools.is_empty());
        assert!(parts.strs.is_empty());
        assert_eq!(parts.nulls, vec![0, 1, 2, 3, 4]);
    
        Ok(())
    }

    // ── Mixed null/non-null interleaved ─────────────────────────

    #[test]
    fn test_mixed_null_and_non_null() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .field("c", ColumnType::Bool)
            .field("d", ColumnType::UInt64)
            .build()
            .unwrap();
        let schema_id = rec.schema_id;

        let buf = rec
            .record(&[
                DynamicValue::Int64(10),  // not null (field 0)
                DynamicValue::Null,       // null (field 1)
                DynamicValue::Bool(true), // not null (field 2)
                DynamicValue::Null,       // null (field 3)
            ])
            .unwrap()
            .to_vec();

        let parts = decode_parts(&buf);
        assert_eq!(parts.schema_id, schema_id);
        assert_eq!(parts.i64s, vec![(0, 10)]);
        assert_eq!(parts.bools, vec![(2, 1)]);
        assert_eq!(parts.nulls, vec![1, 3]);
        assert!(parts.strs.is_empty());
        assert!(parts.u64s.is_empty());
    
        Ok(())
    }

    // ── Build with empty fields ─────────────────────────────────

    #[test]
    fn test_build_with_empty_fields_errors() -> Result<(), Box<dyn std::error::Error>> {
        match DynamicRecorderBuilder::new("t").build() {
            Err(e) => {
                assert!(matches!(e, DynamicRecorderError::NoFields));
                assert_eq!(e.to_string(), "at least one field must be registered");
            }
            Ok(_) => panic!("expected NoFields error"),
        }
    
        Ok(())
    }

    // ── Build with duplicate column names ───────────────────────

    #[test]
    fn test_build_with_duplicate_column_names() -> Result<(), Box<dyn std::error::Error>> {
        // Current behaviour: the builder does not validate uniqueness, so
        // duplicate column names are accepted.
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Int64)
            .field("x", ColumnType::Int64)
            .build()
            .unwrap();

        let buf = rec
            .record(&[DynamicValue::Int64(1), DynamicValue::Int64(2)])
            .unwrap()
            .to_vec();
        assert!(!buf.is_empty());
        assert_eq!(decode_parts(&buf).i64s, vec![(0, 1), (1, 2)]);
    
        Ok(())
    }

    // ── Maximum columns (u8 field_id range 0..=255) ─────────────

    #[test]
    fn test_maximum_columns() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = DynamicRecorderBuilder::new("max_cols");
        for i in 0..=255u16 {
            builder = builder.field(format!("col_{i}"), ColumnType::Int64);
        }
        let mut rec = builder.build().unwrap();

        let values: Vec<DynamicValue> = (0..=255u16)
            .map(|i| DynamicValue::Int64(i as i64))
            .collect();

        let buf = rec.record(&values).unwrap().to_vec();
        let parts = decode_parts(&buf);
        assert_eq!(parts.i64s.len(), 256);
        for (expected, &(fid, v)) in parts.i64s.iter().enumerate() {
            assert_eq!(fid as usize, expected);
            assert_eq!(v, expected as i64);
        }
        assert!(parts.nulls.is_empty());
    
        Ok(())
    }

    // ── Empty metadata + all-null values ────────────────────────

    #[test]
    fn test_all_null_no_metadata() -> Result<(), Box<dyn std::error::Error>> {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .build()
            .unwrap();

        let buf = rec.record(&[DynamicValue::Null]).unwrap().to_vec();
        let parts = decode_parts(&buf);
        assert!(parts.meta.is_empty());
        assert!(parts.i64s.is_empty());
        assert_eq!(parts.nulls, vec![0]);
    
        Ok(())
    }
}
