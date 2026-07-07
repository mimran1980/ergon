//! Persist, PersistAs traits, and TableSchema — todos 01, 02, 03.
//!
//! The [`PersistAs`] trait maps a Rust type to a single `ClickHouse` column.
//! [`TableSchema`] manages column definitions and schema diff/migration.

use crate::types::ColumnType;
use std::fmt;

/// Maps a Rust type to a `ClickHouse` column type and binary encoding.
///
/// This is the escape hatch for custom column type mappings. By implementing
/// `PersistAs` on your type you declare:
///
/// * What `ClickHouse` column type it corresponds to (e.g. `Decimal(18, 8)`)
/// * What column name to use (by default the field name, overridable)
/// * How to encode the value into `ClickHouse` RowBinary format
///
/// # Blanket impl
///
/// `Option<T>` implements `PersistAs` when `T: PersistAs`, mapping to
/// `Nullable(T::column_type())`. It encodes `None` as the `ClickHouse`
/// null marker (`0x01`) and `Some(v)` as the not-null marker (`0x00`)
/// followed by `v.encode_value()`.
///
/// # Examples
///
/// ```
/// use ergo_clickhouse_persist::{PersistAs, ColumnType};
///
/// /// A custom price type backed by a scaled `u64`.
/// struct Price(u64);
///
/// impl PersistAs for Price {
///     fn column_type() -> ColumnType {
///         ColumnType::Decimal { precision: 18, scale: 8 }
///     }
///
///     fn encode_value(&self) -> Vec<u8> {
///         self.0.to_le_bytes().to_vec()
///     }
/// }
///
/// assert_eq!(Price::column_type().to_string(), "Decimal(18, 8)");
/// assert_eq!(<Price as PersistAs>::column_name("ask_price"), "ask_price");
/// assert_eq!(Price(42).encode_value(), vec![42, 0, 0, 0, 0, 0, 0, 0]);
/// ```
pub trait PersistAs {
    /// The `ClickHouse` column type for this Rust type.
    #[must_use]
    fn column_type() -> ColumnType;

    /// Column name hint — used when this type is a struct field.
    #[must_use]
    fn column_name(field_name: &str) -> String {
        field_name.to_string()
    }

    /// Encode `self` as bytes in `ClickHouse` RowBinary format.
    #[must_use]
    fn encode_value(&self) -> Vec<u8>;
}

/// `Option<T>` maps to `Nullable(T::column_type())`.
impl<T: PersistAs> PersistAs for Option<T> {
    fn column_type() -> ColumnType {
        ColumnType::Nullable(Box::new(T::column_type()))
    }

    fn encode_value(&self) -> Vec<u8> {
        match self {
            None => vec![1],
            Some(val) => {
                let mut buf = vec![0];
                buf.extend_from_slice(&val.encode_value());
                buf
            }
        }
    }
}

// ── TableSchema / SchemaDiff (todo 02) ────────────────────────────────────

/// A ClickHouse storage engine.
#[derive(Debug, Clone, PartialEq)]
pub enum TableEngine {
    /// `MergeTree()` — the only engine currently supported.
    MergeTree,
}

impl fmt::Display for TableEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MergeTree => f.write_str("MergeTree()"),
        }
    }
}

/// A column definition within a [`TableSchema`].
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
}

/// Describes a table schema with columns, ordering, engine, and optional TTL.
#[derive(Debug, Clone, PartialEq)]
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
    pub order_by: Vec<String>,
    pub engine: TableEngine,
    pub ttl: Option<TtlConfig>,
}

/// ClickHouse TTL configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct TtlConfig {
    pub column: String,
    pub interval: String, // e.g. "24 HOURS"
}

impl TableSchema {
    /// Create a new `TableSchema` with `MergeTree` engine.
    #[must_use]
    pub fn new(columns: Vec<ColumnDef>, order_by: Vec<String>) -> Self {
        Self::with_ttl(columns, order_by, None)
    }

    /// Create with optional TTL.
    #[must_use]
    pub fn with_ttl(columns: Vec<ColumnDef>, order_by: Vec<String>, ttl: Option<TtlConfig>) -> Self {
        let mut columns = columns;
        if !columns.iter().any(|c| c.name == "_persist_time") {
            columns.push(ColumnDef {
                name: "_persist_time".into(),
                col_type: ColumnType::DateTime64(9),
            });
        }
        let order_by = if order_by.is_empty() {
            vec!["_persist_time".into()]
        } else {
            order_by
        };
        Self {
            columns,
            order_by,
            engine: TableEngine::MergeTree,
            ttl,
        }
    }

    /// Diff this schema against a previous schema version.
    #[must_use]
    pub fn diff(&self, previous: &TableSchema) -> SchemaDiff {
        let mut new_columns = Vec::new();
        let mut type_conflicts = Vec::new();
        let mut compatible_widens = Vec::new();

        for col in &self.columns {
            match previous.columns.iter().find(|c| c.name == col.name) {
                None => {
                    new_columns.push(col.clone());
                }
                Some(prev) if prev.col_type != col.col_type => {
                    if is_compatible_widen(&prev.col_type, &col.col_type) {
                        compatible_widens.push(TypeWiden {
                            column: col.name.clone(),
                            old_type: prev.col_type.clone(),
                            new_type: col.col_type.clone(),
                        });
                    } else {
                        type_conflicts.push(TypeConflict {
                            column: col.name.clone(),
                            old_type: prev.col_type.clone(),
                            new_type: col.col_type.clone(),
                        });
                    }
                }
                Some(_) => { /* identical types */ }
            }
        }

        SchemaDiff {
            new_columns,
            type_conflicts,
            compatible_widens,
        }
    }
}

/// A type change that is compatible (widening within the same numeric family).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeWiden {
    pub column: String,
    pub old_type: ColumnType,
    pub new_type: ColumnType,
}

/// A type change that is incompatible and will be skipped during migration.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeConflict {
    pub column: String,
    pub old_type: ColumnType,
    pub new_type: ColumnType,
}

/// The result of diffing two [`TableSchema`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDiff {
    pub new_columns: Vec<ColumnDef>,
    pub type_conflicts: Vec<TypeConflict>,
    pub compatible_widens: Vec<TypeWiden>,
}

impl SchemaDiff {
    /// Returns `true` if there are no changes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.new_columns.is_empty()
            && self.type_conflicts.is_empty()
            && self.compatible_widens.is_empty()
    }

    /// Generate `ALTER TABLE` DDL statements for this diff.
    #[must_use]
    pub fn alter_table_ddl(&self, table_name: &str) -> Vec<String> {
        let mut statements = Vec::new();

        for col in &self.new_columns {
            statements.push(format!(
                "ALTER TABLE {table_name} ADD COLUMN IF NOT EXISTS {} {}",
                col.name, col.col_type
            ));
        }

        for widen in &self.compatible_widens {
            statements.push(format!(
                "ALTER TABLE {table_name} MODIFY COLUMN {} {}",
                widen.column, widen.new_type
            ));
        }

        statements
    }
}

// -- type-compatibility helpers --------------------------------------------

pub fn is_compatible_widen(old: &ColumnType, new: &ColumnType) -> bool {
    let (Some(old_f), Some(new_f)) = (numeric_family(old), numeric_family(new)) else {
        return false;
    };
    let (Some(old_w), Some(new_w)) = (numeric_width(old), numeric_width(new)) else {
        return false;
    };
    match (old_f, new_f) {
        (a, b) if a == b => new_w > old_w,
        (0, 1) => new_w > old_w, // unsigned → signed: only if strictly wider
        _ => false,
    }
}

/// 0 = unsigned, 1 = signed, 2 = float.
fn numeric_family(ct: &ColumnType) -> Option<u8> {
    match ct {
        ColumnType::UInt8 | ColumnType::UInt16 | ColumnType::UInt32 | ColumnType::UInt64 => Some(0),
        ColumnType::Int8 | ColumnType::Int16 | ColumnType::Int32 | ColumnType::Int64 => Some(1),
        ColumnType::Float32 | ColumnType::Float64 => Some(2),
        _ => None,
    }
}

fn numeric_width(ct: &ColumnType) -> Option<u8> {
    match ct {
        ColumnType::UInt8 | ColumnType::Int8 => Some(8),
        ColumnType::UInt16 | ColumnType::Int16 => Some(16),
        ColumnType::UInt32 | ColumnType::Int32 | ColumnType::Float32 => Some(32),
        ColumnType::UInt64 | ColumnType::Int64 | ColumnType::Float64 => Some(64),
        _ => None,
    }
}

// ── Persist trait (todo 03) ────────────────────────────────────────────────

/// The main trait that structs implement to become persistable.
///
/// A type implementing `Persist` provides:
///
/// * [`table_schema()`](Persist::table_schema) — column definitions and DDL
///   metadata for creating the ClickHouse table.
/// * [`encode_row()`](Persist::encode_row) — copies field values from `self`
///   into a mutable row instance.
///
/// The type is expected to also implement [`clickhouse::Row`] and
/// [`serde::Serialize`] (usually via derive), making it usable with
/// `clickhouse::Client::insert`.
///
/// # Examples
///
/// ```
/// use ergo_clickhouse_persist::{Persist, TableSchema, ColumnDef, ColumnType};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(clickhouse::Row, Serialize, Deserialize, Debug, PartialEq)]
/// struct Trade {
///     price: f64,
///     qty: u64,
///     symbol: String,
/// }
///
/// impl Persist for Trade {
///     fn table_schema() -> TableSchema {
///         TableSchema::new(
///             vec![
///                 ColumnDef { name: "price".into(), col_type: ColumnType::Float64 },
///                 ColumnDef { name: "qty".into(), col_type: ColumnType::UInt64 },
///                 ColumnDef { name: "symbol".into(), col_type: ColumnType::String },
///             ],
///             vec!["_persist_time".into()],
///         )
///     }
///
///     fn encode_row(&self, row: &mut Self) {
///         row.price = self.price;
///         row.qty = self.qty;
///         row.symbol.clone_from(&self.symbol);
///     }
/// }
///
/// assert_eq!(Trade::table_schema().columns.len(), 4);
/// ```
pub trait Persist {
    /// Column definitions + DDL metadata for the ClickHouse table.
    #[must_use]
    fn table_schema() -> TableSchema;

    /// Encode all fields from `self` into the provided row instance.
    ///
    /// For the simple case where the type itself is the row (the common
    /// pattern), this method copies field values. When a separate domain type
    /// exists, this is the place to map fields.
    fn encode_row(&self, row: &mut Self);
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct Price(u64);

    impl PersistAs for Price {
        fn column_type() -> ColumnType {
            ColumnType::Decimal {
                precision: 18,
                scale: 8,
            }
        }

        fn encode_value(&self) -> Vec<u8> {
            self.0.to_le_bytes().to_vec()
        }
    }

    #[test]
    fn test_price_column_type() {
        assert_eq!(Price::column_type().to_string(), "Decimal(18, 8)");
    }

    #[test]
    fn test_price_default_column_name() {
        assert_eq!(<Price as PersistAs>::column_name("ask_price"), "ask_price");
    }

    #[test]
    fn test_price_encode_value() {
        assert_eq!(Price(42).encode_value(), vec![42, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_option_price_column_type() {
        assert_eq!(
            <Option<Price> as PersistAs>::column_type().to_string(),
            "Nullable(Decimal(18, 8))"
        );
    }

    #[test]
    fn test_option_price_encode_some() {
        let val: Option<Price> = Some(Price(42));
        let expected = {
            let mut buf = vec![0];
            buf.extend_from_slice(&42u64.to_le_bytes());
            buf
        };
        assert_eq!(val.encode_value(), expected);
    }

    #[test]
    fn test_option_price_encode_none() {
        assert_eq!(Option::<Price>::None.encode_value(), vec![1]);
    }

    #[test]
    fn test_option_option_price_column_type() {
        let col_type = <Option<Option<Price>> as PersistAs>::column_type();
        assert_eq!(col_type.to_string(), "Nullable(Decimal(18, 8))");
    }

    #[test]
    fn test_option_option_price_encode_some_some() {
        let val: Option<Option<Price>> = Some(Some(Price(42)));
        assert_eq!(
            val.encode_value().as_slice(),
            &[0, 0, 42, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_option_option_price_encode_none() {
        assert_eq!(Option::<Option<Price>>::None.encode_value(), vec![1]);
    }

    #[test]
    fn test_option_option_price_encode_some_none() {
        let val: Option<Option<Price>> = Some(None);
        assert_eq!(val.encode_value(), vec![0, 1]);
    }
}

#[cfg(test)]
mod table_schema_tests {
    use super::*;

    fn col(name: &str, ct: ColumnType) -> ColumnDef {
        ColumnDef {
            name: name.into(),
            col_type: ct,
        }
    }

    #[test]
    fn identical_schemas_empty_diff() {
        let a = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec![]);
        let b = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec![]);
        assert!(a.diff(&b).is_empty());
    }

    #[test]
    fn new_column_added() {
        let old = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec![]);
        let new = TableSchema::new(
            vec![
                col("price", ColumnType::UInt64),
                col("qty", ColumnType::UInt32),
            ],
            vec![],
        );
        let diff = new.diff(&old);
        assert_eq!(diff.new_columns.len(), 1);
        assert_eq!(diff.new_columns[0].name, "qty");
    }

    #[test]
    fn widen_u32_to_u64_compatible() {
        let old = TableSchema::new(vec![col("qty", ColumnType::UInt32)], vec![]);
        let new = TableSchema::new(vec![col("qty", ColumnType::UInt64)], vec![]);
        let diff = new.diff(&old);
        assert_eq!(diff.compatible_widens.len(), 1);
        assert_eq!(diff.compatible_widens[0].column, "qty");
    }

    #[test]
    fn narrow_u64_to_u32_conflict() {
        let old = TableSchema::new(vec![col("qty", ColumnType::UInt64)], vec![]);
        let new = TableSchema::new(vec![col("qty", ColumnType::UInt32)], vec![]);
        let diff = new.diff(&old);
        assert_eq!(diff.type_conflicts.len(), 1);
    }

    #[test]
    fn i32_to_string_conflict() {
        let old = TableSchema::new(vec![col("tag", ColumnType::Int32)], vec![]);
        let new = TableSchema::new(vec![col("tag", ColumnType::String)], vec![]);
        let diff = new.diff(&old);
        assert_eq!(diff.type_conflicts.len(), 1);
    }

    #[test]
    fn u32_to_i64_compatible_widen() {
        let old = TableSchema::new(vec![col("x", ColumnType::UInt32)], vec![]);
        let new = TableSchema::new(vec![col("x", ColumnType::Int64)], vec![]);
        let diff = new.diff(&old);
        assert_eq!(diff.compatible_widens.len(), 1);
    }

    #[test]
    fn i32_to_u32_conflict() {
        let old = TableSchema::new(vec![col("x", ColumnType::Int32)], vec![]);
        let new = TableSchema::new(vec![col("x", ColumnType::UInt32)], vec![]);
        let diff = new.diff(&old);
        assert_eq!(diff.type_conflicts.len(), 1);
    }

    #[test]
    fn removed_column_ignored() {
        let old = TableSchema::new(
            vec![
                col("price", ColumnType::UInt64),
                col("qty", ColumnType::UInt32),
            ],
            vec![],
        );
        let new = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec![]);
        assert!(new.diff(&old).is_empty());
    }

    #[test]
    fn persist_time_auto_added() {
        let schema = TableSchema::new(vec![], vec![]);
        assert!(schema.columns.iter().any(|c| c.name == "_persist_time"));
        assert_eq!(schema.order_by, vec!["_persist_time"]);
    }

    #[test]
    fn persist_time_not_duplicated() {
        let schema = TableSchema::new(
            vec![col("_persist_time", ColumnType::DateTime64(9))],
            vec!["_persist_time".into()],
        );
        assert_eq!(
            schema
                .columns
                .iter()
                .filter(|c| c.name == "_persist_time")
                .count(),
            1
        );
    }

    #[test]
    fn default_order_by() {
        let schema = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec![]);
        assert_eq!(schema.order_by, vec!["_persist_time"]);
    }

    #[test]
    fn custom_order_by() {
        let schema = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec!["price".into()]);
        assert_eq!(schema.order_by, vec!["price"]);
    }

    #[test]
    fn alter_table_ddl_generation() {
        let old = TableSchema::new(vec![col("price", ColumnType::UInt64)], vec![]);
        let new = TableSchema::new(
            vec![
                col("price", ColumnType::UInt64),
                col("qty", ColumnType::UInt32),
            ],
            vec![],
        );
        let ddl = new.diff(&old).alter_table_ddl("trades");
        assert_eq!(ddl.len(), 1);
        assert!(ddl[0].contains("ADD COLUMN IF NOT EXISTS qty UInt32"));
    }

    #[test]
    fn full_migration() {
        let old = TableSchema::new(
            vec![
                col("price", ColumnType::UInt64),
                col("qty", ColumnType::UInt32),
                col("bad", ColumnType::UInt32),
            ],
            vec![],
        );
        let new = TableSchema::new(
            vec![
                col("price", ColumnType::UInt64),
                col("qty", ColumnType::UInt64),
                col("side", ColumnType::String),
                col("bad", ColumnType::Float32),
            ],
            vec![],
        );
        let diff = new.diff(&old);
        assert_eq!(diff.new_columns.len(), 1); // side
        assert_eq!(diff.compatible_widens.len(), 1); // qty
        assert_eq!(diff.type_conflicts.len(), 1); // bad
    }
}

#[cfg(test)]
mod persist_trait_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(clickhouse::Row, Serialize, Deserialize, Debug, PartialEq)]
    struct TradeRow {
        price: f64,
        qty: u64,
        symbol: String,
    }

    impl Persist for TradeRow {
        fn table_schema() -> TableSchema {
            TableSchema::new(
                vec![
                    ColumnDef {
                        name: "price".into(),
                        col_type: ColumnType::Float64,
                    },
                    ColumnDef {
                        name: "qty".into(),
                        col_type: ColumnType::UInt64,
                    },
                    ColumnDef {
                        name: "symbol".into(),
                        col_type: ColumnType::String,
                    },
                ],
                vec![],
            )
        }

        fn encode_row(&self, row: &mut Self) {
            row.price = self.price;
            row.qty = self.qty;
            row.symbol.clone_from(&self.symbol);
        }
    }

    #[test]
    fn table_schema_returns_expected_columns() {
        let schema = <TradeRow as Persist>::table_schema();
        // 3 custom columns + _persist_time auto-added = 4
        assert_eq!(schema.columns.len(), 4);
        assert!(schema.columns.iter().any(|c| c.name == "price"));
        assert!(schema.columns.iter().any(|c| c.name == "qty"));
        assert!(schema.columns.iter().any(|c| c.name == "symbol"));
        // _persist_time auto-added
        assert!(schema.columns.iter().any(|c| c.name == "_persist_time"));
    }

    #[test]
    fn encode_row_populates_row() {
        let src = TradeRow {
            price: 100.50,
            qty: 10,
            symbol: "AAPL".into(),
        };
        let mut dst = TradeRow {
            price: 0.0,
            qty: 0,
            symbol: String::new(),
        };
        src.encode_row(&mut dst);
        assert_eq!(dst.price, 100.50);
        assert_eq!(dst.qty, 10);
        assert_eq!(dst.symbol, "AAPL");
    }

    #[test]
    fn serde_roundtrip() {
        let original = TradeRow {
            price: 99.99,
            qty: 25,
            symbol: "MSFT".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: TradeRow = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    /// Compile-time check: TradeRow satisfies bounds needed by
    /// `clickhouse::Client::insert`.
    #[test]
    fn row_bounds_for_client_insert() {
        fn assert_row_write<T: clickhouse::Row + serde::Serialize>() {}
        assert_row_write::<TradeRow>();
    }
}
