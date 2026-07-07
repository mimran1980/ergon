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

use crate::sbe::DynamicRowEncoder;
use crate::sbe::dynamic_row_encoder_state;
use crate::types::ColumnType;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

// ── DynamicValue ─────────────────────────────────────────────────────────

/// A positional value for [`DynamicRecorder::record`].
///
/// Each variant corresponds to a typed group in the SBE `DynamicRow` message.
/// `Null` is encoded in the nullFields group — the underlying column type's
/// group is simply skipped for that position.
#[derive(Debug, Clone, PartialEq)]
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
}

// ── DynamicValueType ─────────────────────────────────────────────────────

/// The logical type of a [`DynamicValue`] variant.
///
/// Used internally by [`DynamicRecorder`] to map registered [`ColumnType`]s
/// to their wire groups.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DynamicValueType {
    Int64,
    UInt64,
    Float64,
    Bool,
    String,
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
///     .build();
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
}

impl DynamicRecorderBuilder {
    /// Start building a recorder for `table_name`.
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            fields: Vec::new(),
            metadata: Vec::new(),
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

    /// Consume the builder and produce a [`DynamicRecorder`].
    ///
    /// # Panics
    ///
    /// Panics if any registered field has an unsupported [`ColumnType`].
    pub fn build(self) -> DynamicRecorder {
        // Validate all column types are supported
        for (name, ct) in &self.fields {
            if DynamicValueType::from_column_type(ct).is_none() {
                panic!(
                    "DynamicRecorder: unsupported column type '{}' for field '{}'. \
                     Supported types: Int*, UInt*, Float32, Float64, Bool, String, FixedString",
                    ct, name
                );
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

        DynamicRecorder {
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
            buffer,
        }
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

    /// Pre-allocated buffer reused on every [`record()`] call.
    buffer: Vec<u8>,
}

impl DynamicRecorder {
    /// The SBE header bytes for a DynamicRow message.
    const HEADER: [u8; 8] =
        DynamicRowEncoder::<dynamic_row_encoder_state::NeedsRowMetadata>::HEADER_TEMPLATE;

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
                _ => {
                    let expected_name = match expected {
                        DynamicValueType::Int64 => "Int64",
                        DynamicValueType::UInt64 => "UInt64",
                        DynamicValueType::Float64 => "Float64",
                        DynamicValueType::Bool => "Bool",
                        DynamicValueType::String => "String",

                    };
                    let actual_name = match v {
                        DynamicValue::Int64(_) => "Int64",
                        DynamicValue::UInt64(_) => "UInt64",
                        DynamicValue::Float64(_) => "Float64",
                        DynamicValue::Bool(_) => "Bool",
                        DynamicValue::String(_) => "String",
                        DynamicValue::Null => "Null",
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
    }

    fn simple_values() -> Vec<DynamicValue> {
        vec![
            DynamicValue::Float64(100.50),
            DynamicValue::UInt64(1000),
            DynamicValue::String("AAPL".into()),
            DynamicValue::Bool(true),
        ]
    }

    fn validate_standard_fields(buf: &[u8], schema_id: u32) {
        use crate::sbe::DynamicRowDecoder;

        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();
        assert_eq!(decoder.schema_id(), schema_id);

        // Check metadata group (empty for simple recorder).
        let meta = decoder.row_metadata().unwrap();
        assert!(meta.is_empty());
        assert_eq!(meta.len(), 0);

        // Check each field group.
        let int64 = decoder.int64_fields().unwrap();
        assert!(int64.is_empty());

        let uint64 = decoder.uint64_fields().unwrap();
        assert_eq!(uint64.len(), 1);
        for entry in uint64 {
            assert_eq!(entry.value(), 1000);
        }

        let float64 = decoder.float64_fields().unwrap();
        assert_eq!(float64.len(), 1);
        for entry in float64 {
            assert_eq!(entry.value(), 100.50);
        }

        let bool_fields = decoder.bool_fields().unwrap();
        assert_eq!(bool_fields.len(), 1);
        for entry in bool_fields {
            assert_eq!(entry.value(), 1);
        }

        let string_fields = decoder.string_fields().unwrap();
        assert_eq!(string_fields.len(), 1);
        for entry in string_fields {
            assert_eq!(entry.str_len(), 4);
        }

        let null_fields = decoder.null_fields().unwrap();
        assert!(null_fields.is_empty());

        let sym = decoder.symbol_table().unwrap();
        assert_eq!(sym, b"AAPL");
    }

    // ── build + record ───────────────────────────────────────────────

    #[test]
    fn test_build_and_record() {
        let mut rec = simple_recorder();
        let values = simple_values();
        let schema_id = rec.schema_id;

        let buf = rec.record(&values).unwrap();
        assert!(!buf.is_empty());

        validate_standard_fields(buf, schema_id);
    }

    // ── schema_id determinism ─────────────────────────────────────────

    #[test]
    fn test_schema_id_determinism() {
        let rec1 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .metadata("k1", "v1")
            .build();

        // Same registration, same metadata → same schema_id.
        let rec2 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .metadata("k1", "v1")
            .build();
        assert_eq!(rec1.schema_id, rec2.schema_id);

        // Different field → different schema_id.
        let rec3 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("c", ColumnType::String) // different name
            .metadata("k1", "v1")
            .build();
        assert_ne!(rec1.schema_id, rec3.schema_id);

        // Different metadata → different schema_id.
        let rec4 = DynamicRecorderBuilder::new("t")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::String)
            .metadata("k1", "v2") // different value
            .build();
        assert_ne!(rec1.schema_id, rec4.schema_id);

        // Different registration order → same schema_id (sorted internally).
        let rec5 = DynamicRecorderBuilder::new("t")
            .field("b", ColumnType::String)
            .field("a", ColumnType::Int64)
            .metadata("k1", "v1")
            .build();
        assert_eq!(rec1.schema_id, rec5.schema_id);
    }

    // ── metadata ──────────────────────────────────────────────────────

    #[test]
    fn test_metadata_values_present() {
        let mut rec = DynamicRecorderBuilder::new("test")
            .field("val", ColumnType::Int64)
            .metadata("source", "exchange_a")
            .metadata("env", "prod")
            .build();

        let buf = rec.record(&[DynamicValue::Int64(42)]).unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();

        // Check metadata group.
        let meta = decoder.row_metadata().unwrap();
        assert_eq!(meta.len(), 2);

        // The decoder only gives us key_len/val_len, not the actual keys.
        // Verify the entries exist and have expected lengths.
        let entries: Vec<_> = meta.collect();
        assert_eq!(entries.len(), 2);

        // Verify symbol table contains metadata key+value strings.
        let sym = decoder.symbol_table().unwrap();
        // metadata symbols are: "source" + "exchange_a" + "env" + "prod"
        assert!(sym.len() >= 6 + 10 + 3 + 4);
        assert!(sym.windows(6).any(|w| w == b"source"));
        assert!(sym.windows(10).any(|w| w == b"exchange_a"));
        assert!(sym.windows(3).any(|w| w == b"env"));
        assert!(sym.windows(4).any(|w| w == b"prod"));
    }

    #[test]
    fn test_metadata_consistency() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Int64)
            .metadata("key1", "val1")
            .build();

        let buf1 = rec.record(&[DynamicValue::Int64(1)]).unwrap().to_vec();
        let buf2 = rec.record(&[DynamicValue::Int64(2)]).unwrap().to_vec();

        // Metadata should be byte-identical across calls.
        use crate::sbe::DynamicRowDecoder;
        let dec1 = DynamicRowDecoder::wrap_and_apply_header(&buf1, 0).unwrap();
        let dec2 = DynamicRowDecoder::wrap_and_apply_header(&buf2, 0).unwrap();

        let meta1: Vec<_> = dec1.row_metadata().unwrap().collect();
        let meta2: Vec<_> = dec2.row_metadata().unwrap().collect();
        assert_eq!(meta1.len(), meta2.len());

        let sym1 = dec1.symbol_table().unwrap();
        let sym2 = dec2.symbol_table().unwrap();
        // Both have same metadata (key1/val1) and same string data (none
        // since x is Int64). Symbol tables should be identical.
        assert_eq!(sym1, sym2);
    }

    // ── String values ─────────────────────────────────────────────────

    #[test]
    fn test_string_values() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("name", ColumnType::String)
            .field("code", ColumnType::String)
            .build();

        let buf = rec
            .record(&[
                DynamicValue::String("hello".into()),
                DynamicValue::String("abc".into()),
            ])
            .unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();

        let string_fields = decoder.string_fields().unwrap();
        assert_eq!(string_fields.len(), 2);

        // Verify string lengths.
        let entries: Vec<_> = string_fields.collect();
        assert_eq!(entries[0].str_len(), 5);
        assert_eq!(entries[1].str_len(), 3);

        // Verify symbol table contains concatenated string data.
        let sym = decoder.symbol_table().unwrap();
        assert_eq!(sym, b"helloabc");
    }

    #[test]
    fn test_string_symbol_table_with_metadata() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("msg", ColumnType::String)
            .metadata("tag", "xyz")
            .build();

        let buf = rec.record(&[DynamicValue::String("data".into())]).unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();

        let sym = decoder.symbol_table().unwrap();
        // Metadata first: "tag" (3) + "xyz" (3) = 6 bytes, then string
        // "data" (4) = 10 bytes total
        assert_eq!(sym.len(), 3 + 3 + 4);
        assert!(sym.starts_with(b"tagxyz"));
        assert!(sym.ends_with(b"data"));
    }

    // ── Null values ───────────────────────────────────────────────────

    #[test]
    fn test_null_values() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("val", ColumnType::Int64)
            .field("name", ColumnType::String)
            .build();

        let buf = rec
            .record(&[DynamicValue::Null, DynamicValue::Null])
            .unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();

        // int64 group should be empty (value is null).
        let int64 = decoder.int64_fields().unwrap();
        assert!(int64.is_empty());

        // string group should be empty.
        let string_fields = decoder.string_fields().unwrap();
        assert!(string_fields.is_empty());

        // null group should have 2 entries.
        let null_fields = decoder.null_fields().unwrap();
        assert_eq!(null_fields.len(), 2);
        let entries: Vec<_> = null_fields.collect();
        assert_eq!(entries[0].field_id(), 0);
        assert_eq!(entries[1].field_id(), 1);
    }

    // ── Empty metadata ────────────────────────────────────────────────

    #[test]
    fn test_empty_metadata_produces_valid_sbe() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Float64)
            .build();

        let buf = rec.record(&[DynamicValue::Float64(1.0)]).unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();
        let meta = decoder.row_metadata().unwrap();
        assert!(meta.is_empty());
        assert_eq!(meta.len(), 0);

        let float64 = decoder.float64_fields().unwrap();
        assert_eq!(float64.len(), 1);
    }

    // ── Multi-key metadata ────────────────────────────────────────────

    #[test]
    fn test_multiple_metadata_keys() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("x", ColumnType::Int64)
            .metadata("a", "1")
            .metadata("b", "2")
            .metadata("c", "3")
            .build();

        let buf = rec.record(&[DynamicValue::Int64(0)]).unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();
        let meta = decoder.row_metadata().unwrap();
        assert_eq!(meta.len(), 3);
    }

    // ── Wrong value count ─────────────────────────────────────────────

    #[test]
    fn test_wrong_value_count_errors() {
        let mut rec = simple_recorder();
        let err = rec.record(&[DynamicValue::Int64(1)]).unwrap_err();
        assert!(matches!(
            err,
            DynamicRecorderError::ValueCountMismatch { .. }
        ));
    }

    // ── Value type mismatch ───────────────────────────────────────────

    #[test]
    fn test_value_type_mismatch_errors() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("price", ColumnType::Float64)
            .build();
        let err = rec.record(&[DynamicValue::Int64(42)]).unwrap_err();
        assert!(matches!(
            err,
            DynamicRecorderError::ValueTypeMismatch { .. }
        ));
    }

    // ── 100k loop — no allocation ─────────────────────────────────────

    #[test]
    fn test_no_allocation_loop() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("price", ColumnType::Float64)
            .field("qty", ColumnType::UInt64)
            .field("symbol", ColumnType::String)
            .build();

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
    }

    // ── Schema ID w/ metadata ─────────────────────────────────────────

    #[test]
    fn test_schema_id_determinism_with_metadata() {
        // Same fields + same metadata → same schema_id regardless of
        // registration order.
        let a = DynamicRecorderBuilder::new("x")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::UInt64)
            .metadata("z", "1")
            .metadata("y", "2")
            .build();

        let b = DynamicRecorderBuilder::new("x")
            .field("b", ColumnType::UInt64)
            .field("a", ColumnType::Int64)
            .metadata("y", "2")
            .metadata("z", "1")
            .build();

        assert_eq!(a.schema_id, b.schema_id);

        // Different metadata → different schema_id.
        let c = DynamicRecorderBuilder::new("x")
            .field("a", ColumnType::Int64)
            .field("b", ColumnType::UInt64)
            .metadata("z", "9") // changed value
            .metadata("y", "2")
            .build();

        assert_ne!(a.schema_id, c.schema_id);
    }

    // ── General round-trip ────────────────────────────────────────────

    #[test]
    fn test_round_trip_all_types() {
        let mut rec = DynamicRecorderBuilder::new("rt_test")
            .field("i", ColumnType::Int64)
            .field("u", ColumnType::UInt64)
            .field("f", ColumnType::Float64)
            .field("b", ColumnType::Bool)
            .field("s", ColumnType::String)
            .field("n", ColumnType::Int64) // nullable field (Null value)
            .metadata("rt", "check")
            .build();
        let schema_id = rec.schema_id;

        let buf = rec
            .record(&[
                DynamicValue::Int64(-42),
                DynamicValue::UInt64(99),
                DynamicValue::Float64(3.14),
                DynamicValue::Bool(false),
                DynamicValue::String("hello".into()),
                DynamicValue::Null,
            ])
            .unwrap();

        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();

        assert_eq!(decoder.schema_id(), schema_id);

        // Int64 field.
        {
            let mut fields = decoder.int64_fields().unwrap();
            assert_eq!(fields.len(), 1);
            let entry = fields.next().unwrap();
            assert_eq!(entry.value(), -42);
        }

        // UInt64 field.
        {
            let mut fields = decoder.uint64_fields().unwrap();
            assert_eq!(fields.len(), 1);
            let entry = fields.next().unwrap();
            assert_eq!(entry.value(), 99);
        }

        // Float64 field.
        {
            let mut fields = decoder.float64_fields().unwrap();
            assert_eq!(fields.len(), 1);
            let entry = fields.next().unwrap();
            assert!((entry.value() - 3.14).abs() < 1e-10);
        }

        // Bool field.
        {
            let mut fields = decoder.bool_fields().unwrap();
            assert_eq!(fields.len(), 1);
            let entry = fields.next().unwrap();
            assert_eq!(entry.value(), 0); // false
        }

        // String field.
        {
            let mut fields = decoder.string_fields().unwrap();
            assert_eq!(fields.len(), 1);
            let entry = fields.next().unwrap();
            assert_eq!(entry.str_len(), 5);
        }

        // Null field.
        {
            let mut fields = decoder.null_fields().unwrap();
            assert_eq!(fields.len(), 1);
            let entry = fields.next().unwrap();
            assert_eq!(entry.field_id(), 5); // 5th field (0-indexed)
        }

        // Symbol table: metadata "rt"+"check" (2+5=7) + "hello" (5) = 12 bytes
        let sym = decoder.symbol_table().unwrap();
        assert_eq!(sym.len(), 2 + 5 + 5);
        assert!(sym.starts_with(b"rtcheck"));
        assert!(sym.ends_with(b"hello"));
    }

    // ── Nullable column type ──────────────────────────────────────────

    #[test]
    fn test_nullable_column_type() {
        let mut rec = DynamicRecorderBuilder::new("t")
            .field("val", ColumnType::Nullable(Box::new(ColumnType::Int64)))
            .build();

        // Null value is accepted for nullable field.
        let buf = rec.record(&[DynamicValue::Null]).unwrap();
        use crate::sbe::DynamicRowDecoder;
        let decoder = DynamicRowDecoder::wrap_and_apply_header(buf, 0).unwrap();
        let null_fields = decoder.null_fields().unwrap();
        assert_eq!(null_fields.len(), 1);

        // Non-null value is also accepted.
        let buf2 = rec.record(&[DynamicValue::Int64(42)]).unwrap();
        let decoder2 = DynamicRowDecoder::wrap_and_apply_header(buf2, 0).unwrap();
        let int64_fields = decoder2.int64_fields().unwrap();
        assert_eq!(int64_fields.len(), 1);
    }
}
