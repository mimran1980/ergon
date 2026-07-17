//! SchemaRegistry + RowDecoder — consumer-side counterpart to DynamicRecorder.
//!
//! Receives decoded SBE [`DynamicSchema`] and [`DynamicRow`] messages, manages
//! table schemas, and produces column-name → SQL-literal maps for insertion.
//!
//! ## Flow
//!
//! ```ignore
//! let registry = Rc::new(RefCell::new(SchemaRegistry::new()));
//! let decoder = RowDecoder::new(Rc::clone(&registry));
//!
//! // On first sight of a schema:
//! let schema = DynamicSchemaDecoder::wrap_and_apply_header(bytes, 0)?;
//! registry.borrow_mut().register(&schema)?;
//!
//! // Per row:
//! let row = DynamicRowDecoder::wrap_and_apply_header(bytes, 0)?;
//! let decoded: DecodedRow = decoder.decode(&row)?;
//! ```

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::persist::{ColumnDef, TableSchema};
use crate::sbe::{DynamicRowDecoder, DynamicSchemaDecoder, sbe_rt};
use crate::types::ColumnType;

// ── Type tag constants ───────────────────────────────────────────────────
//
// These map a u8 tag in the DynamicSchema wire format to a ColumnType variant.

const TAG_INT8: u8 = 0;
const TAG_INT16: u8 = 1;
const TAG_INT32: u8 = 2;
const TAG_INT64: u8 = 3;
const TAG_UINT8: u8 = 4;
const TAG_UINT16: u8 = 5;
const TAG_UINT32: u8 = 6;
const TAG_UINT64: u8 = 7;
const TAG_FLOAT32: u8 = 8;
const TAG_FLOAT64: u8 = 9;
const TAG_BOOL: u8 = 10;
const TAG_STRING: u8 = 11;
const TAG_FIXED_STRING: u8 = 12;

/// Convert a wire type tag to a [`ColumnType`].
fn type_tag_to_column_type(tag: u8) -> Option<ColumnType> {
    match tag {
        TAG_INT8 => Some(ColumnType::Int8),
        TAG_INT16 => Some(ColumnType::Int16),
        TAG_INT32 => Some(ColumnType::Int32),
        TAG_INT64 => Some(ColumnType::Int64),
        TAG_UINT8 => Some(ColumnType::UInt8),
        TAG_UINT16 => Some(ColumnType::UInt16),
        TAG_UINT32 => Some(ColumnType::UInt32),
        TAG_UINT64 => Some(ColumnType::UInt64),
        TAG_FLOAT32 => Some(ColumnType::Float32),
        TAG_FLOAT64 => Some(ColumnType::Float64),
        TAG_BOOL => Some(ColumnType::Bool),
        TAG_STRING => Some(ColumnType::String),
        TAG_FIXED_STRING => Some(ColumnType::FixedString(0)),
        _ => None,
    }
}

/// Convert a [`ColumnType`] to its wire type tag (`None` for unsupported types).
pub fn column_type_to_tag(ct: &ColumnType) -> Option<u8> {
    match ct {
        ColumnType::Int8 => Some(TAG_INT8),
        ColumnType::Int16 => Some(TAG_INT16),
        ColumnType::Int32 => Some(TAG_INT32),
        ColumnType::Int64 => Some(TAG_INT64),
        ColumnType::UInt8 => Some(TAG_UINT8),
        ColumnType::UInt16 => Some(TAG_UINT16),
        ColumnType::UInt32 => Some(TAG_UINT32),
        ColumnType::UInt64 => Some(TAG_UINT64),
        ColumnType::Float32 => Some(TAG_FLOAT32),
        ColumnType::Float64 => Some(TAG_FLOAT64),
        ColumnType::Bool => Some(TAG_BOOL),
        ColumnType::String => Some(TAG_STRING),
        ColumnType::FixedString(_) => Some(TAG_FIXED_STRING),
        ColumnType::Nullable(inner) => column_type_to_tag(inner),
        _ => None,
    }
}

// ── DecodedRow ───────────────────────────────────────────────────────────

/// A decoded row: maps column name → SQL literal value (or `None` for NULL).
pub type DecodedRow = HashMap<String, Option<String>>;

// ── Error ────────────────────────────────────────────────────────────────

/// Errors returned by [`SchemaRegistry`] and [`RowDecoder`].
#[derive(Debug, Clone)]
pub enum RowDecodeError {
    /// No schema registered for the given schema_id.
    UnknownSchemaId(u32),
    /// A column entry in the DynamicSchema had an unsupported type tag.
    UnsupportedColumnType(u8),
    /// The symbol-table bytes contained invalid UTF-8.
    InvalidUtf8(String),
    /// An underyling SBE decode error.
    Sbe(sbe_rt::DecodeError),
}

impl fmt::Display for RowDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSchemaId(id) => write!(f, "unknown schema_id: {id}"),
            Self::UnsupportedColumnType(tag) => write!(f, "unsupported column type tag: {tag}"),
            Self::InvalidUtf8(s) => write!(f, "invalid UTF-8 in symbol table: {s}"),
            Self::Sbe(e) => write!(f, "SBE decode error: {e}"),
        }
    }
}

impl std::error::Error for RowDecodeError {}

impl From<sbe_rt::DecodeError> for RowDecodeError {
    fn from(e: sbe_rt::DecodeError) -> Self {
        Self::Sbe(e)
    }
}

// ── RegisteredSchema ─────────────────────────────────────────────────────

/// Cached schema for one schema_id.
#[derive(Debug, Clone)]
struct RegisteredSchema {
    table_name: String,
    /// (field_id, column_name, column_type) — ordered by field_id.
    columns: Vec<(u8, String, ColumnType)>,
    /// Metadata column names (all String type).
    metadata_keys: Vec<String>,
    /// Reconstructed TableSchema (data columns + metadata columns).
    table_schema: TableSchema,
}

// ── SchemaRegistry ───────────────────────────────────────────────────────

/// Manages discovered DynamicSchema definitions.
///
/// Schemas are keyed by their `schema_id` and are registered on first sight
/// (idempotent for repeated registrations).
pub struct SchemaRegistry {
    schemas: HashMap<u32, RegisteredSchema>,
}

impl SchemaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Register (or re-register) a schema from a decoded [`DynamicSchema`]
    /// SBE message.  Idempotent — registering the same `schema_id` twice is a
    /// no-op.
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedColumnType`] if a column entry's type tag is
    /// unrecognised, [`InvalidUtf8`] if the symbol table contains non-UTF-8
    /// bytes, or [`Sbe`] if the underlying decoder fails.
    ///
    /// [`UnsupportedColumnType`]: RowDecodeError::UnsupportedColumnType
    /// [`InvalidUtf8`]: RowDecodeError::InvalidUtf8
    /// [`Sbe`]: RowDecodeError::Sbe
    pub fn register(&mut self, schema: DynamicSchemaDecoder<'_>) -> Result<(), RowDecodeError> {
        let schema_id = schema.schema_id();

        // Idempotent: skip if already registered.
        if self.schemas.contains_key(&schema_id) {
            return Ok(());
        }

        // Consuming-stage wire order: metadata → columns → tableName → symbolTable.
        let owned = schema;

        // ── 1. Read metadata entries (collect key/val lengths) ──
        let mut meta_lens: Vec<(usize, usize)> = Vec::new();
        let mut meta_group = owned.into_metadata()?;
        for entry in meta_group.by_ref() {
            meta_lens.push((entry.key_len() as usize, entry.val_len() as usize));
        }
        let after_meta = meta_group.finish()?;

        // ── 2. Read column entries (collect field_id/name_len/type_tag) ──
        let mut col_specs: Vec<(u8, usize, u8)> = Vec::new();
        let mut col_group = after_meta.into_columns()?;
        for entry in col_group.by_ref() {
            col_specs.push((
                entry.field_id(),
                entry.name_len() as usize,
                entry.type_tag(),
            ));
        }
        let after_cols = col_group.finish()?;

        // ── 3. Read tableName and symbolTable var-data ──
        let (table_name_bytes, after_tn) = after_cols.into_table_name()?;
        let table_name = std::str::from_utf8(table_name_bytes)
            .map_err(|e| RowDecodeError::InvalidUtf8(e.to_string()))?
            .to_string();

        let (sym_table, _complete) = after_tn.into_symbol_table()?;

        // ── 4. Parse strings from symbolTable using collected lengths ──
        let mut sym_offset = 0usize;
        let mut metadata_keys: Vec<String> = Vec::new();
        for (kl, vl) in &meta_lens {
            let key_bytes = sym_table
                .get(sym_offset..sym_offset + *kl)
                .ok_or_else(|| RowDecodeError::InvalidUtf8("metadata key out of bounds".into()))?;
            let key = std::str::from_utf8(key_bytes)
                .map_err(|e| RowDecodeError::InvalidUtf8(e.to_string()))?;
            metadata_keys.push(key.to_string());
            sym_offset += kl + vl;
        }

        let mut columns: Vec<(u8, String, ColumnType)> = Vec::new();
        for (field_id, name_len, type_tag) in &col_specs {
            let col_type = type_tag_to_column_type(*type_tag)
                .ok_or(RowDecodeError::UnsupportedColumnType(*type_tag))?;
            let name_bytes = sym_table
                .get(sym_offset..sym_offset + *name_len)
                .ok_or_else(|| RowDecodeError::InvalidUtf8("column name out of bounds".into()))?;
            let name = std::str::from_utf8(name_bytes)
                .map_err(|e| RowDecodeError::InvalidUtf8(e.to_string()))?;
            columns.push((*field_id, name.to_string(), col_type));
            sym_offset += name_len;
        }

        // ── 5. Build TableSchema ──
        let mut schema_cols = Vec::new();
        for (_, name, ct) in &columns {
            schema_cols.push(ColumnDef {
                name: name.clone(),
                col_type: ct.clone(),
            });
        }
        for key in &metadata_keys {
            if !schema_cols.iter().any(|c| c.name == *key) {
                schema_cols.push(ColumnDef {
                    name: key.clone(),
                    col_type: ColumnType::String,
                });
            }
        }

        // ponytail: simple MergeTree with no order_by.  In practice the
        // caller sets the ordering key when creating the table.
        let table_schema = TableSchema {
            columns: schema_cols,
            order_by: Vec::new(),
            engine: crate::persist::TableEngine::MergeTree,
            ttl: None,
        };

        self.schemas.insert(
            schema_id,
            RegisteredSchema {
                table_name,
                columns,
                metadata_keys,
                table_schema,
            },
        );

        Ok(())
    }

    /// Look up the table name for a given `schema_id`.
    pub fn table_name(&self, schema_id: u32) -> Option<&str> {
        self.schemas.get(&schema_id).map(|r| r.table_name.as_str())
    }

    fn get(&self, schema_id: u32) -> Option<&RegisteredSchema> {
        self.schemas.get(&schema_id)
    }

    fn get_mut(&mut self, schema_id: u32) -> Option<&mut RegisteredSchema> {
        self.schemas.get_mut(&schema_id)
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── RowDecoder ───────────────────────────────────────────────────────────

/// Decodes [`DynamicRow`] SBE messages into [`DecodedRow`] maps using a shared
/// [`SchemaRegistry`].
pub struct RowDecoder {
    registry: Rc<RefCell<SchemaRegistry>>,
}

impl RowDecoder {
    /// Create a decoder that shares ownership of `registry`.
    pub fn new(registry: Rc<RefCell<SchemaRegistry>>) -> Self {
        Self { registry }
    }

    /// Decode a [`DynamicRow`] into a column-name → SQL-literal map.
    ///
    /// # Decoding order
    ///
    /// 1. **Metadata first** — every metadata key-value pair becomes a column.
    ///    New metadata keys (not in the registered schema) are added to the
    ///    registry cache (side-effect on the shared [`SchemaRegistry`]).
    /// 2. **Data fields** — each typed group entry is resolved through the
    ///    cached column map.
    /// 3. **Missing fields** (in schema, absent from the row) → `None` (NULL).
    /// 4. **Extra data fields** (in row, not in schema) → silently dropped.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownSchemaId`] when no schema is registered for the row's
    /// `schema_id`, [`InvalidUtf8`] for non-UTF-8 symbol-table bytes, or
    /// [`Sbe`] on decoder failure.
    ///
    /// [`UnknownSchemaId`]: RowDecodeError::UnknownSchemaId
    /// [`InvalidUtf8`]: RowDecodeError::InvalidUtf8
    /// [`Sbe`]: RowDecodeError::Sbe
    pub fn decode(&self, row: DynamicRowDecoder<'_>) -> Result<DecodedRow, RowDecodeError> {
        let schema_id = row.schema_id();

        // ── 1. Clone the registered schema (release borrow before mutating) ──
        let reg = {
            let reg_ref = self.registry.borrow();
            reg_ref.get(schema_id).cloned()
        };
        let mut reg = reg.ok_or(RowDecodeError::UnknownSchemaId(schema_id))?;

        // Build a fast lookup: field_id → (column_name, ColumnType)
        let field_map: HashMap<u8, (String, ColumnType)> = reg
            .columns
            .iter()
            .map(|(id, name, ct)| (*id, (name.clone(), ct.clone())))
            .collect();

        // ── 2. Walk consuming stages in wire order ──
        // DynamicRow wire order: rowMetadata → int64Fields → uint64Fields →
        // float64Fields → boolFields → stringFields → nullFields → symbolTable.
        let owned = row;

        // Collect metadata key/val lengths.
        let mut meta_lens: Vec<(usize, usize)> = Vec::new();
        let mut rm_group = owned.into_row_metadata()?;
        for entry in rm_group.by_ref() {
            meta_lens.push((entry.key_len() as usize, entry.val_len() as usize));
        }
        let after_rm = rm_group.finish()?;

        // Collect int64 fields.
        let mut i64_vals: Vec<(u8, i64)> = Vec::new();
        let mut i64_group = after_rm.into_int64_fields()?;
        for entry in i64_group.by_ref() {
            i64_vals.push((entry.field_id(), entry.value()));
        }
        let after_i64 = i64_group.finish()?;

        // Collect uint64 fields.
        let mut u64_vals: Vec<(u8, u64)> = Vec::new();
        let mut u64_group = after_i64.into_uint64_fields()?;
        for entry in u64_group.by_ref() {
            u64_vals.push((entry.field_id(), entry.value()));
        }
        let after_u64 = u64_group.finish()?;

        // Collect float64 fields.
        let mut f64_vals: Vec<(u8, f64)> = Vec::new();
        let mut f64_group = after_u64.into_float64_fields()?;
        for entry in f64_group.by_ref() {
            f64_vals.push((entry.field_id(), entry.value()));
        }
        let after_f64 = f64_group.finish()?;

        // Collect bool fields.
        let mut bool_vals: Vec<(u8, u8)> = Vec::new();
        let mut bool_group = after_f64.into_bool_fields()?;
        for entry in bool_group.by_ref() {
            bool_vals.push((entry.field_id(), entry.value()));
        }
        let after_bool = bool_group.finish()?;

        // Collect string field lengths.
        let mut str_specs: Vec<(u8, usize)> = Vec::new();
        let mut str_group = after_bool.into_string_fields()?;
        for entry in str_group.by_ref() {
            str_specs.push((entry.field_id(), entry.str_len() as usize));
        }
        let after_str = str_group.finish()?;

        // Collect null field_ids.
        let mut null_field_ids: HashSet<u8> = HashSet::new();
        let mut null_group = after_str.into_null_fields()?;
        for entry in null_group.by_ref() {
            null_field_ids.insert(entry.field_id());
        }
        let after_null = null_group.finish()?;

        // Read symbolTable var-data.
        let (sym_table, _complete) = after_null.into_symbol_table()?;

        // ── 3. Parse metadata strings from symbolTable ──
        let mut meta_entries: Vec<(String, String)> = Vec::new();
        let mut sym_offset = 0usize;
        for (kl, vl) in &meta_lens {
            let key_bytes = sym_table.get(sym_offset..sym_offset + *kl).ok_or_else(|| {
                RowDecodeError::InvalidUtf8("metadata key out of bounds in row".into())
            })?;
            let key = std::str::from_utf8(key_bytes)
                .map_err(|e| RowDecodeError::InvalidUtf8(e.to_string()))?;
            let val_bytes = sym_table
                .get(sym_offset + *kl..sym_offset + *kl + *vl)
                .ok_or_else(|| {
                    RowDecodeError::InvalidUtf8("metadata value out of bounds in row".into())
                })?;
            let val = std::str::from_utf8(val_bytes)
                .map_err(|e| RowDecodeError::InvalidUtf8(e.to_string()))?;
            meta_entries.push((key.to_string(), val.to_string()));
            sym_offset += kl + vl;
        }

        // ── 4. Build output map ──
        let mut output: DecodedRow = HashMap::new();
        let mut non_null_field_ids: HashSet<u8> = HashSet::new();

        // Check for new metadata keys and add to output.
        let mut schema_changed = false;
        for (key, val) in &meta_entries {
            if !reg.metadata_keys.iter().any(|k| k == key) {
                reg.metadata_keys.push(key.clone());
                reg.table_schema.columns.push(ColumnDef {
                    name: key.clone(),
                    col_type: ColumnType::String,
                });
                schema_changed = true;
            }
            output.insert(key.clone(), Some(format_sql_string(val)));
        }

        // Int64 fields
        for (fid, val) in &i64_vals {
            non_null_field_ids.insert(*fid);
            if let Some((name, _)) = field_map.get(fid) {
                output.insert(name.clone(), Some(val.to_string()));
            }
        }

        // UInt64 fields
        for (fid, val) in &u64_vals {
            non_null_field_ids.insert(*fid);
            if let Some((name, _)) = field_map.get(fid) {
                output.insert(name.clone(), Some(val.to_string()));
            }
        }

        // Float64 fields
        for (fid, val) in &f64_vals {
            non_null_field_ids.insert(*fid);
            if let Some((name, _)) = field_map.get(fid) {
                output.insert(name.clone(), Some(val.to_string()));
            }
        }

        // Bool fields
        for (fid, val) in &bool_vals {
            non_null_field_ids.insert(*fid);
            if let Some((name, _)) = field_map.get(fid) {
                let s = if *val != 0 { "1" } else { "0" };
                output.insert(name.clone(), Some(s.to_string()));
            }
        }

        // String fields — resolve from symbol table
        for (fid, slen) in &str_specs {
            non_null_field_ids.insert(*fid);
            if let Some((name, _)) = field_map.get(fid) {
                let str_bytes = sym_table
                    .get(sym_offset..sym_offset + *slen)
                    .ok_or_else(|| {
                        RowDecodeError::InvalidUtf8("string field data out of bounds".into())
                    })?;
                let s = std::str::from_utf8(str_bytes)
                    .map_err(|e| RowDecodeError::InvalidUtf8(e.to_string()))?;
                output.insert(name.clone(), Some(format_sql_string(s)));
            }
            sym_offset += *slen;
        }

        // ── 5. Null fields (present in nullFields group) ──
        for fid in &null_field_ids {
            if let Some((name, _)) = field_map.get(fid) {
                output.insert(name.clone(), None);
            }
        }

        // ── 6. Missing fields → NULL ──
        for (fid, name, _) in &reg.columns {
            if !non_null_field_ids.contains(fid) && !null_field_ids.contains(fid) {
                output.insert(name.clone(), None);
            }
        }

        // ── 7. Update registry cache with newly-discovered metadata keys ──
        if schema_changed {
            let mut reg_ref = self.registry.borrow_mut();
            if let Some(cached) = reg_ref.get_mut(schema_id) {
                cached.metadata_keys = reg.metadata_keys;
                cached.table_schema.columns = reg.table_schema.columns;
            }
        }

        Ok(output)
    }
}

// ── SQL formatting helper ────────────────────────────────────────────────

/// Escape and quote a string for ClickHouse SQL.
///
/// Escapes `\` → `\\` and `'` → `''`, wraps in single quotes.
pub fn format_sql_string(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('\'', "''");
    format!("'{escaped}'")
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::{DynamicRecorder, DynamicRecorderBuilder, DynamicValue};
    use crate::sbe::{
        DynamicRowDecoder, DynamicRowEncoder, DynamicSchemaDecoder, DynamicSchemaEncoder,
    };
    use crate::types::ColumnType;

    // ── helpers ───────────────────────────────────────────────────────

    /// Build a DynamicSchema SBE message for a given `schema_id`.
    fn encode_schema_with_id(
        schema_id: u32,
        table_name: &str,
        fields: &[(u8, &str, ColumnType)],
        metadata: &[(&str, &str)],
    ) -> Vec<u8> {
        let mut buf = vec![0u8; DynamicSchemaEncoder::MAX_ENCODED_LENGTH];
        let mut enc = DynamicSchemaEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(schema_id);

        // Symbol table: metadata key+val bytes, then column name bytes.
        let mut sym = Vec::new();
        for (k, v) in metadata {
            sym.extend_from_slice(k.as_bytes());
            sym.extend_from_slice(v.as_bytes());
        }
        for (_, name, _) in fields {
            sym.extend_from_slice(name.as_bytes());
        }

        let enc = enc
            .metadata(metadata.len() as u16, |g| {
                for (k, v) in metadata {
                    g.add(|e| {
                        let _ = e.key_len(k.len() as u16).val_len(v.len() as u16);
                    })
                    .unwrap();
                }
            })
            .unwrap();

        let enc = enc
            .columns(fields.len() as u16, |g| {
                for (fid, name, ct) in fields {
                    let tag = column_type_to_tag(ct).unwrap();
                    g.add(|e| {
                        let _ = e.field_id(*fid).name_len(name.len() as u16).type_tag(tag);
                    })
                    .unwrap();
                }
            })
            .unwrap();

        let enc = enc.table_name(table_name.as_bytes()).unwrap();
        let enc = enc.symbol_table(&sym).unwrap();
        let len = enc.encoded_length_with_header();
        buf[..len].to_vec()
    }

    /// Build a DynamicRecorder for test use.
    fn recorder_for(
        table_name: &str,
        fields: &[(&str, ColumnType)],
        metadata: &[(&str, &str)],
    ) -> DynamicRecorder {
        let mut b = DynamicRecorderBuilder::new(table_name);
        for (name, ct) in fields {
            b = b.field(*name, ct.clone());
        }
        for (k, v) in metadata {
            b = b.metadata(*k, *v);
        }
        b.build().unwrap()
    }

    /// Build a fixture: ONE recorder → matching schema + row bytes.
    fn make_fixture(
        table_name: &str,
        fields_schema: &[(u8, &str, ColumnType)],
        fields_rec: &[(&str, ColumnType)],
        metadata: &[(&str, &str)],
        values: &[DynamicValue],
    ) -> (DynamicRecorder, Vec<u8>, Vec<u8>) {
        let mut rec = recorder_for(table_name, fields_rec, metadata);
        let schema_id = rec.schema_id;
        let schema_bytes = encode_schema_with_id(schema_id, table_name, fields_schema, metadata);
        let row_bytes = rec.record(values).unwrap().to_vec();
        (rec, schema_bytes, row_bytes)
    }

    /// Register a schema and return the registry.
    fn register_schema(schema_bytes: &[u8]) -> Rc<RefCell<SchemaRegistry>> {
        let schema = DynamicSchemaDecoder::wrap_and_apply_header(schema_bytes, 0).unwrap();
        let reg = Rc::new(RefCell::new(SchemaRegistry::new()));
        reg.borrow_mut().register(schema).unwrap();
        reg
    }

    // ── SchemaRegistry tests ──────────────────────────────────────────

    #[test]
    fn test_register_schema() {
        let rec = recorder_for(
            "test_table",
            &[("price", ColumnType::Float64)],
            &[("src", "ex")],
        );
        let sid = rec.schema_id;
        let schema_bytes = encode_schema_with_id(
            sid,
            "test_table",
            &[(0, "price", ColumnType::Float64)],
            &[("src", "ex")],
        );
        let schema = DynamicSchemaDecoder::wrap_and_apply_header(&schema_bytes, 0).unwrap();
        let mut registry = SchemaRegistry::new();
        registry.register(schema).unwrap();

        assert_eq!(registry.table_name(sid), Some("test_table"));
        assert!(registry.table_name(sid + 1).is_none());
    }

    #[test]
    fn test_register_idempotent() {
        let rec = recorder_for("dup", &[("x", ColumnType::Int64)], &[]);
        let sid = rec.schema_id;
        let schema_bytes = encode_schema_with_id(sid, "dup", &[(0, "x", ColumnType::Int64)], &[]);
        let schema = DynamicSchemaDecoder::wrap_and_apply_header(&schema_bytes, 0).unwrap();

        let mut registry = SchemaRegistry::new();
        registry.register(schema).unwrap();
        let schema2 = DynamicSchemaDecoder::wrap_and_apply_header(&schema_bytes, 0).unwrap();
        registry.register(schema2).unwrap(); // second call — no-op

        assert_eq!(registry.table_name(sid), Some("dup"));
    }

    #[test]
    fn test_table_name_unknown() {
        let registry = SchemaRegistry::new();
        assert_eq!(registry.table_name(42), None);
    }

    // ── RowDecoder — basic ────────────────────────────────────────────

    #[test]
    fn test_decode_row_basic() {
        let (_rec, schema_bytes, row_bytes) = make_fixture(
            "test_table",
            &[
                (0, "price", ColumnType::Float64),
                (1, "qty", ColumnType::UInt64),
                (2, "symbol", ColumnType::String),
            ],
            &[
                ("price", ColumnType::Float64),
                ("qty", ColumnType::UInt64),
                ("symbol", ColumnType::String),
            ],
            &[("src", "ex")],
            &[
                DynamicValue::Float64(100.50),
                DynamicValue::UInt64(1000),
                DynamicValue::String("AAPL".into()),
            ],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();

        assert_eq!(decoded.get("price").unwrap(), &Some("100.5".to_string()));
        assert_eq!(decoded.get("qty").unwrap(), &Some("1000".to_string()));
        assert_eq!(decoded.get("symbol").unwrap(), &Some("'AAPL'".to_string()));
        assert_eq!(decoded.get("src").unwrap(), &Some("'ex'".to_string()));
    }

    // ── RowDecoder — metadata ─────────────────────────────────────────

    #[test]
    fn test_decode_row_metadata() {
        let (_rec, schema_bytes, row_bytes) = make_fixture(
            "meta_test",
            &[(0, "val", ColumnType::Float64)],
            &[("val", ColumnType::Float64)],
            &[("env", "prod"), ("app", "my_app")],
            &[DynamicValue::Float64(1.0)],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();

        assert_eq!(decoded.get("val").unwrap(), &Some("1".to_string()));
        assert_eq!(decoded.get("env").unwrap(), &Some("'prod'".to_string()));
        assert_eq!(decoded.get("app").unwrap(), &Some("'my_app'".to_string()));
    }

    // ── RowDecoder — strings ─────────────────────────────────────────

    #[test]
    fn test_decode_string_fields() {
        let (_rec, schema_bytes, row_bytes) = make_fixture(
            "str_test",
            &[
                (0, "name", ColumnType::String),
                (1, "code", ColumnType::String),
            ],
            &[("name", ColumnType::String), ("code", ColumnType::String)],
            &[],
            &[
                DynamicValue::String("hello".into()),
                DynamicValue::String("abc".into()),
            ],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();

        assert_eq!(decoded.get("name").unwrap(), &Some("'hello'".to_string()));
        assert_eq!(decoded.get("code").unwrap(), &Some("'abc'".to_string()));
    }

    // ── RowDecoder — nulls ────────────────────────────────────────────

    #[test]
    fn test_decode_null_fields() {
        let (_rec, schema_bytes, row_bytes) = make_fixture(
            "null_test",
            &[
                (0, "val", ColumnType::Int64),
                (1, "name", ColumnType::String),
            ],
            &[("val", ColumnType::Int64), ("name", ColumnType::String)],
            &[],
            &[DynamicValue::Null, DynamicValue::Null],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();

        assert_eq!(decoded.get("val").unwrap(), &None);
        assert_eq!(decoded.get("name").unwrap(), &None);
    }

    // ── RowDecoder — missing fields → NULL ────────────────────────────

    #[test]
    fn test_decode_missing_fields_null() {
        // "b" is null-encoded rather than truly absent (DynamicRecorder always
        // encodes every registered field).  The "missing field → NULL" path
        // is exercised by the null-encode path in this test.
        // ponytail: true "absent from wire" would need manual SBE encoding;
        // the nullFields group is the effective equivalent.
        let (_rec, schema_bytes, row_bytes) = make_fixture(
            "miss_test",
            &[(0, "a", ColumnType::Int64), (1, "b", ColumnType::Float64)],
            &[("a", ColumnType::Int64), ("b", ColumnType::Float64)],
            &[],
            &[DynamicValue::Int64(42), DynamicValue::Null],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();

        assert_eq!(decoded.get("a").unwrap(), &Some("42".to_string()));
        assert_eq!(decoded.get("b").unwrap(), &None);
    }

    // ── Roundtrip ─────────────────────────────────────────────────────

    #[test]
    fn test_roundtrip_all_types() {
        let (_rec, schema_bytes, row_bytes) = make_fixture(
            "rt_test",
            &[
                (0, "i", ColumnType::Int64),
                (1, "u", ColumnType::UInt64),
                (2, "f", ColumnType::Float64),
                (3, "b", ColumnType::Bool),
                (4, "s", ColumnType::String),
                (5, "n", ColumnType::Int64),
            ],
            &[
                ("i", ColumnType::Int64),
                ("u", ColumnType::UInt64),
                ("f", ColumnType::Float64),
                ("b", ColumnType::Bool),
                ("s", ColumnType::String),
                ("n", ColumnType::Int64),
            ],
            &[("tag", "rt")],
            &[
                DynamicValue::Int64(-42),
                DynamicValue::UInt64(99),
                DynamicValue::Float64(std::f64::consts::PI),
                DynamicValue::Bool(false),
                DynamicValue::String("hello".into()),
                DynamicValue::Null,
            ],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);
        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();

        assert_eq!(decoded.get("i").unwrap(), &Some("-42".to_string()));
        assert_eq!(decoded.get("u").unwrap(), &Some("99".to_string()));
        // Float may format as "3.14" or "3.1400000000000001"
        let f_val = decoded.get("f").unwrap();
        let f_parsed: f64 = f_val.as_deref().unwrap().parse().unwrap();
        assert!((f_parsed - std::f64::consts::PI).abs() < 1e-10);
        assert_eq!(decoded.get("b").unwrap(), &Some("0".to_string()));
        assert_eq!(decoded.get("s").unwrap(), &Some("'hello'".to_string()));
        assert_eq!(decoded.get("n").unwrap(), &None); // null field
        assert_eq!(decoded.get("tag").unwrap(), &Some("'rt'".to_string()));
    }

    // ── Multiple rows, no state leak ──────────────────────────────────

    #[test]
    fn test_multiple_rows_no_state_leak() {
        let rec = recorder_for(
            "seq_test",
            &[("price", ColumnType::Float64), ("qty", ColumnType::UInt64)],
            &[("seq", "0")],
        );
        let schema_bytes = encode_schema_with_id(
            rec.schema_id,
            "seq_test",
            &[
                (0, "price", ColumnType::Float64),
                (1, "qty", ColumnType::UInt64),
            ],
            &[("seq", "0")],
        );
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);

        // Row 1
        let mut rec1 = recorder_for(
            "seq_test",
            &[("price", ColumnType::Float64), ("qty", ColumnType::UInt64)],
            &[("seq", "0")],
        );
        // Note: schema_id matches because parameters are identical.
        // The metadata ("seq", "0") is part of the schema, so every
        // encode_row call will include it.  This test verifies that
        // different data values don't cross-contaminate.
        let _ = rec1
            .record(&[DynamicValue::Float64(100.0), DynamicValue::UInt64(10)])
            .unwrap();
        // Actually rec and rec1 have the same schema_id, but we need
        // row bytes that share the same schema_id as the registered schema.
        // Just use the original recorder for everything.
        let row1_bytes = recorder_for(
            "seq_test",
            &[("price", ColumnType::Float64), ("qty", ColumnType::UInt64)],
            &[("seq", "0")],
        )
        .record(&[DynamicValue::Float64(100.0), DynamicValue::UInt64(10)])
        .unwrap()
        .to_vec();

        let row1 = DynamicRowDecoder::wrap_and_apply_header(&row1_bytes, 0).unwrap();
        let decoded1 = decoder.decode(row1).unwrap();
        assert_eq!(decoded1.get("price").unwrap(), &Some("100".to_string()));
        assert_eq!(decoded1.get("qty").unwrap(), &Some("10".to_string()));

        // Row 2 (different values)
        let row2_bytes = recorder_for(
            "seq_test",
            &[("price", ColumnType::Float64), ("qty", ColumnType::UInt64)],
            &[("seq", "0")],
        )
        .record(&[DynamicValue::Float64(200.0), DynamicValue::UInt64(20)])
        .unwrap()
        .to_vec();

        let row2 = DynamicRowDecoder::wrap_and_apply_header(&row2_bytes, 0).unwrap();
        let decoded2 = decoder.decode(row2).unwrap();
        assert_eq!(decoded2.get("price").unwrap(), &Some("200".to_string()));
        assert_eq!(decoded2.get("qty").unwrap(), &Some("20".to_string()));

        // No cross-contamination.
        assert_ne!(
            decoded1.get("price").unwrap(),
            decoded2.get("price").unwrap()
        );
    }

    // ── Dynamic metadata discovery mid-stream ─────────────────────────

    /// Helper: encode a DynamicRow with given schema_id, metadata key/value,
    /// and a single int64 field (field_id=0).  Uses DynamicRowEncoder directly
    /// so the schema_id is controlled (not derived from a DynamicRecorder).
    fn encode_row_with_meta(
        schema_id: u32,
        meta_key: &str,
        meta_val: &str,
        int64_val: i64,
    ) -> Vec<u8> {
        let mut buf = vec![0u8; DynamicRowEncoder::MAX_ENCODED_LENGTH];
        let mut enc = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(schema_id);

        let sym: Vec<u8> = meta_key
            .as_bytes()
            .iter()
            .chain(meta_val.as_bytes())
            .copied()
            .collect();

        let enc = enc
            .row_metadata(1, |g| {
                let _ = g.add(|e| {
                    let _ = e
                        .key_len(meta_key.len() as u16)
                        .val_len(meta_val.len() as u16);
                });
            })
            .unwrap()
            .int64_fields(1, |g| {
                let _ = g.add(|e| {
                    let _ = e.field_id(0).value(int64_val);
                });
            })
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(0, |_| {})
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .symbol_table(&sym)
            .unwrap();
        let len = enc.encoded_length_with_header();
        buf[..len].to_vec()
    }

    #[test]
    fn test_dynamic_metadata_discovery_mid_stream() {
        // Schema with no metadata columns.
        let mut rec = recorder_for("live", &[("x", ColumnType::Int64)], &[]);
        let schema_bytes =
            encode_schema_with_id(rec.schema_id, "live", &[(0, "x", ColumnType::Int64)], &[]);
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(Rc::clone(&reg));

        // Row 1: no metadata — use the same recorder so schema_id matches.
        let r1 = rec.record(&[DynamicValue::Int64(1)]).unwrap().to_vec();
        let r1 = DynamicRowDecoder::wrap_and_apply_header(&r1, 0).unwrap();
        let d1 = decoder.decode(r1).unwrap();
        assert_eq!(d1.get("x").unwrap(), &Some("1".to_string()));
        assert_eq!(d1.len(), 1);

        // Row 2: introduces "env" metadata key.
        let r2_bytes = encode_row_with_meta(rec.schema_id, "env", "prod", 2);
        let r2 = DynamicRowDecoder::wrap_and_apply_header(&r2_bytes, 0).unwrap();
        let d2 = decoder.decode(r2).unwrap();
        assert_eq!(d2.get("x").unwrap(), &Some("2".to_string()));
        assert_eq!(d2.get("env").unwrap(), &Some("'prod'".to_string()));

        // Row 3: "env" should still be decoded.
        let r3_bytes = encode_row_with_meta(rec.schema_id, "env", "staging", 3);
        let r3 = DynamicRowDecoder::wrap_and_apply_header(&r3_bytes, 0).unwrap();
        let d3 = decoder.decode(r3).unwrap();
        assert_eq!(d3.get("env").unwrap(), &Some("'staging'".to_string()));
    }

    // ── format_sql_string ─────────────────────────────────────────────

    // ── Tag mapping matrix + error/Default coverage ──────────────────

    #[test]
    fn test_type_tag_roundtrip_all_supported() {
        let types = [
            ColumnType::Int8,
            ColumnType::Int16,
            ColumnType::Int32,
            ColumnType::Int64,
            ColumnType::UInt8,
            ColumnType::UInt16,
            ColumnType::UInt32,
            ColumnType::UInt64,
            ColumnType::Float32,
            ColumnType::Float64,
            ColumnType::Bool,
            ColumnType::String,
        ];
        for ct in types {
            let tag = column_type_to_tag(&ct).unwrap();
            assert_eq!(type_tag_to_column_type(tag), Some(ct));
        }
        // FixedString collapses to FixedString(0); Nullable delegates.
        let fs_tag = column_type_to_tag(&ColumnType::FixedString(9)).unwrap();
        assert_eq!(
            type_tag_to_column_type(fs_tag),
            Some(ColumnType::FixedString(0))
        );
        let n_tag = column_type_to_tag(&ColumnType::Nullable(Box::new(ColumnType::Int64))).unwrap();
        assert_eq!(type_tag_to_column_type(n_tag), Some(ColumnType::Int64));
        // Unsupported both ways.
        assert_eq!(column_type_to_tag(&ColumnType::Date), None);
        assert_eq!(type_tag_to_column_type(200), None);
    }

    #[test]
    fn test_row_decode_error_display() {
        assert_eq!(
            RowDecodeError::UnknownSchemaId(7).to_string(),
            "unknown schema_id: 7"
        );
        let invalid = RowDecodeError::InvalidUtf8("bad".into());
        assert!(invalid.to_string().contains("bad"));
        let unsupported = RowDecodeError::UnsupportedColumnType(99);
        assert!(unsupported.to_string().contains("99"));
    }

    #[test]
    fn test_schema_registry_default() {
        let reg = SchemaRegistry::default();
        assert_eq!(reg.table_name(1), None);
    }

    #[test]
    fn test_truncated_symbol_table_is_out_of_bounds_error() {
        // A row whose string entry claims more bytes than the symbol table
        // holds must fail with the bounds error, not panic.
        let rec = recorder_for("oob", &[("s", ColumnType::String)], &[]);
        let schema_bytes =
            encode_schema_with_id(rec.schema_id, "oob", &[(0, "s", ColumnType::String)], &[]);
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);

        let mut buf = vec![0u8; DynamicRowEncoder::MAX_ENCODED_LENGTH];
        let mut enc = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(rec.schema_id);
        let enc = enc
            .row_metadata(0, |_| {})
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(1, |g| {
                let _ = g.add(|e| {
                    let _ = e.field_id(0).str_len(64); // claims 64 bytes
                });
            })
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .symbol_table(b"tiny") // only 4 bytes present
            .unwrap();
        let len = enc.encoded_length_with_header();
        let row_bytes = buf[..len].to_vec();

        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let err = decoder.decode(row).unwrap_err();
        assert!(matches!(err, RowDecodeError::InvalidUtf8(_)));
    }

    #[test]
    fn test_truncated_metadata_symbols_is_out_of_bounds_error() {
        let rec = recorder_for("oob2", &[("x", ColumnType::Int64)], &[]);
        let schema_bytes =
            encode_schema_with_id(rec.schema_id, "oob2", &[(0, "x", ColumnType::Int64)], &[]);
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);

        let mut buf = vec![0u8; DynamicRowEncoder::MAX_ENCODED_LENGTH];
        let mut enc = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(rec.schema_id);
        let enc = enc
            .row_metadata(1, |g| {
                let _ = g.add(|e| {
                    let _ = e.key_len(10).val_len(10); // claims 20 bytes
                });
            })
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(0, |_| {})
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .symbol_table(b"") // empty
            .unwrap();
        let len = enc.encoded_length_with_header();
        let row_bytes = buf[..len].to_vec();

        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let err = decoder.decode(row).unwrap_err();
        assert!(matches!(err, RowDecodeError::InvalidUtf8(_)));
    }

    #[test]
    fn test_field_absent_from_wire_decodes_as_null() {
        // A schema column entirely absent from the row's typed and null
        // groups still appears in the output as NULL.
        let rec = recorder_for("absent", &[("x", ColumnType::Int64)], &[]);
        let schema_bytes =
            encode_schema_with_id(rec.schema_id, "absent", &[(0, "x", ColumnType::Int64)], &[]);
        let reg = register_schema(&schema_bytes);
        let decoder = RowDecoder::new(reg);

        let mut buf = vec![0u8; DynamicRowEncoder::MAX_ENCODED_LENGTH];
        let mut enc = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.schema_id(rec.schema_id);
        let enc = enc
            .row_metadata(0, |_| {})
            .unwrap()
            .int64_fields(0, |_| {})
            .unwrap()
            .uint64_fields(0, |_| {})
            .unwrap()
            .float64_fields(0, |_| {})
            .unwrap()
            .bool_fields(0, |_| {})
            .unwrap()
            .string_fields(0, |_| {})
            .unwrap()
            .null_fields(0, |_| {})
            .unwrap()
            .symbol_table(b"")
            .unwrap();
        let len = enc.encoded_length_with_header();
        let row_bytes = buf[..len].to_vec();

        let row = DynamicRowDecoder::wrap_and_apply_header(&row_bytes, 0).unwrap();
        let decoded = decoder.decode(row).unwrap();
        assert_eq!(decoded.get("x").unwrap(), &None);
    }

    #[test]
    fn test_format_sql_string_empty() {
        assert_eq!(format_sql_string(""), "''");
    }

    #[test]
    fn test_format_sql_string_plain() {
        assert_eq!(format_sql_string("hello"), "'hello'");
    }

    #[test]
    fn test_format_sql_string_with_quote() {
        assert_eq!(format_sql_string("it's"), "'it''s'");
    }

    #[test]
    fn test_format_sql_string_with_backslash() {
        assert_eq!(format_sql_string("a\\b"), "'a\\\\b'");
    }
}
