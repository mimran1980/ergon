//! `ColumnType` enum and default type mappings — todo 00, 06.

use std::any::TypeId;
use std::fmt;

/// A `ClickHouse` column type.
///
/// Every variant maps 1:1 to a `ClickHouse` DDL type string.
/// The [`Display`] impl produces the canonical DDL representation.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    /// `Int8`
    Int8,
    /// `Int16`
    Int16,
    /// `Int32`
    Int32,
    /// `Int64`
    Int64,
    /// `UInt8`
    UInt8,
    /// `UInt16`
    UInt16,
    /// `UInt32`
    UInt32,
    /// `UInt64`
    UInt64,
    /// `Float32`
    Float32,
    /// `Float64`
    Float64,
    /// `Decimal(precision, scale)` — precision 1-76, scale 0-precision.
    Decimal {
        /// Total number of digits (1-76).
        precision: u8,
        /// Number of digits after the decimal point (0-precision).
        scale: u8,
    },
    /// `String`
    String,
    /// `FixedString(N)`
    FixedString(usize),
    /// `Date`
    Date,
    /// `DateTime(precision)` — sub-second precision 0-9.
    DateTime(u8),
    /// `DateTime64(precision)` — sub-second precision 0-9.
    DateTime64(u8),
    /// `Nullable(T)` — collapsed to `T` when `T` is also `Nullable`.
    Nullable(Box<Self>),
    /// `Array(T)`
    Array(Box<Self>),
    /// `Bool`
    Bool,
    /// `Interval`
    Interval,
    /// `Json`
    Json,
}

impl ColumnType {
    /// Validate Decimal bounds: `1 <= precision <= 76`, `0 <= scale <= precision`.
    #[must_use]
    pub fn decimal_bounds(precision: u8, scale: u8) -> bool {
        (1..=76).contains(&precision) && scale <= precision
    }
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int8 => f.write_str("Int8"),
            Self::Int16 => f.write_str("Int16"),
            Self::Int32 => f.write_str("Int32"),
            Self::Int64 => f.write_str("Int64"),
            Self::UInt8 => f.write_str("UInt8"),
            Self::UInt16 => f.write_str("UInt16"),
            Self::UInt32 => f.write_str("UInt32"),
            Self::UInt64 => f.write_str("UInt64"),
            Self::Float32 => f.write_str("Float32"),
            Self::Float64 => f.write_str("Float64"),
            Self::Decimal { precision, scale } => {
                assert!(
                    Self::decimal_bounds(*precision, *scale),
                    "Decimal: precision must be 1-76 and scale 0-precision, \
                     got precision={precision}, scale={scale}"
                );
                write!(f, "Decimal({precision}, {scale})")
            }
            Self::String => f.write_str("String"),
            Self::FixedString(n) => write!(f, "FixedString({n})"),
            Self::Date => f.write_str("Date"),
            Self::DateTime(p) => write!(f, "DateTime({p})"),
            Self::DateTime64(p) => write!(f, "DateTime64({p})"),
            Self::Nullable(inner) => {
                if matches!(inner.as_ref(), Self::Nullable(_)) {
                    // Collapse double-wrapping: Nullable(Nullable(T)) -> Nullable(T).
                    write!(f, "{inner}")
                } else {
                    write!(f, "Nullable({inner})")
                }
            }
            Self::Array(inner) => write!(f, "Array({inner})"),
            Self::Bool => f.write_str("Bool"),
            Self::Interval => f.write_str("Interval"),
            Self::Json => f.write_str("Json"),
        }
    }
}

impl From<ColumnType> for String {
    fn from(ct: ColumnType) -> Self {
        ct.to_string()
    }
}

/// Generate DDL for a column definition: `"name type"`.
#[must_use]
pub fn column_definition_ddl(name: &str, column_type: &ColumnType) -> String {
    format!("{name} {column_type}")
}

/// Generate `ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...` DDL.
#[must_use]
pub fn alter_table_add_column_ddl(
    table_name: &str,
    column_name: &str,
    column_type: &ColumnType,
) -> String {
    let def = column_definition_ddl(column_name, column_type);
    format!("ALTER TABLE {table_name} ADD COLUMN IF NOT EXISTS {def}")
}

/// Generate a `CREATE TABLE IF NOT EXISTS ...` DDL statement.
///
/// Uses `ENGINE = MergeTree()` and the provided `ORDER BY` columns.
#[must_use]
pub fn create_table_ddl(
    table_name: &str,
    schema: &[(&str, &ColumnType)],
    order_by: &[&str],
) -> String {
    let columns: Vec<String> = schema
        .iter()
        .map(|(name, ct)| column_definition_ddl(name, ct))
        .collect();
    let order_by_clause = order_by.join(", ");
    format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (\n    {}\n) \
         ENGINE = MergeTree() ORDER BY ({order_by_clause})",
        columns.join(",\n    ")
    )
}

/// Map a Rust type to its default [`ColumnType`] at compile time.
///
/// Every primitive type has a canonical ClickHouse column type:
///
/// | Rust type | ClickHouse type |
/// |-----------|----------------|
/// | `i8` | `Int8` |
/// | `i16` | `Int16` |
/// | `i32` | `Int32` |
/// | `i64` | `Int64` |
/// | `u8` | `UInt8` |
/// | `u16` | `UInt16` |
/// | `u32` | `UInt32` |
/// | `u64` | `UInt64` |
/// | `f32` | `Float32` |
/// | `f64` | `Float64` |
/// | `bool` | `Bool` |
/// | `String` | `String` |
/// | `&str` | `String` |
/// | `Vec<u8>` | `String` |
///
/// Types without an explicit mapping receive [`ColumnType::Json`] (the
/// catch-all for `impl Serialize` values).
///
/// `Option<T>`, `Vec<T>` (other than `Vec<u8>`), and user-defined structs
/// are not handled here — they receive the `Json` fallback or require a
/// custom `PersistAs` implementation.
#[must_use]
pub fn default_column_type<T: 'static>() -> ColumnType {
    let tid = TypeId::of::<T>();
    match () {
        _ if tid == TypeId::of::<i8>() => ColumnType::Int8,
        _ if tid == TypeId::of::<i16>() => ColumnType::Int16,
        _ if tid == TypeId::of::<i32>() => ColumnType::Int32,
        _ if tid == TypeId::of::<i64>() => ColumnType::Int64,
        _ if tid == TypeId::of::<u8>() => ColumnType::UInt8,
        _ if tid == TypeId::of::<u16>() => ColumnType::UInt16,
        _ if tid == TypeId::of::<u32>() => ColumnType::UInt32,
        _ if tid == TypeId::of::<u64>() => ColumnType::UInt64,
        _ if tid == TypeId::of::<f32>() => ColumnType::Float32,
        _ if tid == TypeId::of::<f64>() => ColumnType::Float64,
        _ if tid == TypeId::of::<bool>() => ColumnType::Bool,
        _ if tid == TypeId::of::<String>()
            || tid == TypeId::of::<&'static str>()
            || tid == TypeId::of::<Vec<u8>>() =>
        {
            ColumnType::String
        }
        _ => ColumnType::Json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- scalar variants ---------------------------------------------------

    #[test]
    fn test_int8_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Int8.to_string(), "Int8");

        Ok(())
    }

    #[test]
    fn test_int16_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Int16.to_string(), "Int16");

        Ok(())
    }

    #[test]
    fn test_int32_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Int32.to_string(), "Int32");

        Ok(())
    }

    #[test]
    fn test_int64_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Int64.to_string(), "Int64");

        Ok(())
    }

    #[test]
    fn test_uint8_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::UInt8.to_string(), "UInt8");

        Ok(())
    }

    #[test]
    fn test_uint16_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::UInt16.to_string(), "UInt16");

        Ok(())
    }

    #[test]
    fn test_uint32_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::UInt32.to_string(), "UInt32");

        Ok(())
    }

    #[test]
    fn test_uint64_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::UInt64.to_string(), "UInt64");

        Ok(())
    }

    #[test]
    fn test_float32_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Float32.to_string(), "Float32");

        Ok(())
    }

    #[test]
    fn test_float64_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Float64.to_string(), "Float64");

        Ok(())
    }

    #[test]
    fn test_decimal_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            ColumnType::Decimal {
                precision: 18,
                scale: 8,
            }
            .to_string(),
            "Decimal(18, 8)"
        );

        Ok(())
    }

    #[test]
    fn test_decimal_min_precision() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            ColumnType::Decimal {
                precision: 1,
                scale: 0,
            }
            .to_string(),
            "Decimal(1, 0)"
        );

        Ok(())
    }

    #[test]
    fn test_decimal_max_precision() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            ColumnType::Decimal {
                precision: 76,
                scale: 76,
            }
            .to_string(),
            "Decimal(76, 76)"
        );

        Ok(())
    }

    #[test]
    #[should_panic(expected = "Decimal: precision must be 1-76")]
    fn test_decimal_precision_zero() {
        let ct = ColumnType::Decimal {
            precision: 0,
            scale: 0,
        };
        let _ = ct.to_string();
    }

    #[test]
    #[should_panic(expected = "Decimal: precision must be 1-76")]
    fn test_decimal_precision_77() {
        let ct = ColumnType::Decimal {
            precision: 77,
            scale: 0,
        };
        let _ = ct.to_string();
    }

    #[test]
    #[should_panic(expected = "Decimal: precision must be 1-76")]
    fn test_decimal_scale_exceeds_precision() {
        let ct = ColumnType::Decimal {
            precision: 5,
            scale: 6,
        };
        let _ = ct.to_string();
    }

    #[test]
    fn test_string_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::String.to_string(), "String");

        Ok(())
    }

    #[test]
    fn test_fixed_string_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::FixedString(32).to_string(), "FixedString(32)");

        Ok(())
    }

    #[test]
    fn test_date_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Date.to_string(), "Date");

        Ok(())
    }

    #[test]
    fn test_datetime_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::DateTime(3).to_string(), "DateTime(3)");

        Ok(())
    }

    #[test]
    fn test_datetime64_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::DateTime64(6).to_string(), "DateTime64(6)");

        Ok(())
    }

    #[test]
    fn test_bool_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Bool.to_string(), "Bool");

        Ok(())
    }

    #[test]
    fn test_interval_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Interval.to_string(), "Interval");

        Ok(())
    }

    #[test]
    fn test_json_ddl() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::Json.to_string(), "Json");

        Ok(())
    }

    // -- compound types ----------------------------------------------------

    #[test]
    fn test_nullable() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Nullable(Box::new(ColumnType::UInt64));
        assert_eq!(ct.to_string(), "Nullable(UInt64)");

        Ok(())
    }

    #[test]
    fn test_nullable_nesting_collapsed() -> Result<(), Box<dyn std::error::Error>> {
        // Nullable(Nullable(Int32)) -> Nullable(Int32)
        let ct = ColumnType::Nullable(Box::new(ColumnType::Nullable(Box::new(ColumnType::Int32))));
        assert_eq!(ct.to_string(), "Nullable(Int32)");

        Ok(())
    }

    #[test]
    fn test_array() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Array(Box::new(ColumnType::Int32));
        assert_eq!(ct.to_string(), "Array(Int32)");

        Ok(())
    }

    #[test]
    fn test_array_nullable() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Array(Box::new(ColumnType::Nullable(Box::new(ColumnType::Int32))));
        assert_eq!(ct.to_string(), "Array(Nullable(Int32))");

        Ok(())
    }

    // -- DDL functions -----------------------------------------------------

    #[test]
    fn test_column_definition_ddl() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Decimal {
            precision: 18,
            scale: 8,
        };
        assert_eq!(column_definition_ddl("price", &ct), "price Decimal(18, 8)");

        Ok(())
    }

    #[test]
    fn test_alter_table_add_column_ddl() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Decimal {
            precision: 18,
            scale: 8,
        };
        assert_eq!(
            alter_table_add_column_ddl("trades", "price", &ct),
            "ALTER TABLE trades ADD COLUMN IF NOT EXISTS price Decimal(18, 8)"
        );

        Ok(())
    }

    #[test]
    fn test_create_table_ddl() -> Result<(), Box<dyn std::error::Error>> {
        let id = ColumnType::UInt64;
        let symbol = ColumnType::String;
        let price = ColumnType::Decimal {
            precision: 18,
            scale: 8,
        };
        let ts = ColumnType::DateTime(3);
        let schema = [
            ("id", &id),
            ("symbol", &symbol),
            ("price", &price),
            ("ts", &ts),
        ];
        let ddl = create_table_ddl("trades", &schema, &["id", "ts"]);
        let expected = "\
CREATE TABLE IF NOT EXISTS trades (
    id UInt64,
    symbol String,
    price Decimal(18, 8),
    ts DateTime(3)
) ENGINE = MergeTree() ORDER BY (id, ts)";
        assert_eq!(ddl, expected);

        Ok(())
    }

    // -- From<ColumnType> for String ---------------------------------------

    #[test]
    fn test_from_column_type_for_string() -> Result<(), Box<dyn std::error::Error>> {
        let s: String = ColumnType::UInt32.into();
        assert_eq!(s, "UInt32");

        Ok(())
    }

    // -- decimal_bounds utility --------------------------------------------

    #[test]
    fn test_decimal_bounds_valid() -> Result<(), Box<dyn std::error::Error>> {
        assert!(ColumnType::decimal_bounds(1, 0));
        assert!(ColumnType::decimal_bounds(76, 76));
        assert!(ColumnType::decimal_bounds(18, 8));

        Ok(())
    }

    #[test]
    fn test_decimal_bounds_invalid() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!ColumnType::decimal_bounds(0, 0));
        assert!(!ColumnType::decimal_bounds(77, 0));
        assert!(!ColumnType::decimal_bounds(5, 6));

        Ok(())
    }

    // -- default type mappings — todo 06 ---------------------------------

    fn assert_maps_to<T: 'static>(expected: ColumnType) {
        assert_eq!(default_column_type::<T>(), expected);
    }

    #[test]
    fn test_default_i8() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<i8>(ColumnType::Int8);

        Ok(())
    }

    #[test]
    fn test_default_i16() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<i16>(ColumnType::Int16);

        Ok(())
    }

    #[test]
    fn test_default_i32() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<i32>(ColumnType::Int32);

        Ok(())
    }

    #[test]
    fn test_default_i64() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<i64>(ColumnType::Int64);

        Ok(())
    }

    #[test]
    fn test_default_u8() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<u8>(ColumnType::UInt8);

        Ok(())
    }

    #[test]
    fn test_default_u16() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<u16>(ColumnType::UInt16);

        Ok(())
    }

    #[test]
    fn test_default_u32() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<u32>(ColumnType::UInt32);

        Ok(())
    }

    #[test]
    fn test_default_u64() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<u64>(ColumnType::UInt64);

        Ok(())
    }

    #[test]
    fn test_default_f32() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<f32>(ColumnType::Float32);

        Ok(())
    }

    #[test]
    fn test_default_f64() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<f64>(ColumnType::Float64);

        Ok(())
    }

    #[test]
    fn test_default_bool() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<bool>(ColumnType::Bool);

        Ok(())
    }

    #[test]
    fn test_default_string() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<String>(ColumnType::String);

        Ok(())
    }

    #[test]
    fn test_default_str() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<&str>(ColumnType::String);

        Ok(())
    }

    #[test]
    fn test_default_vec_u8() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<Vec<u8>>(ColumnType::String);

        Ok(())
    }

    #[test]
    fn test_default_unknown_type_falls_back_to_json() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<Vec<i32>>(ColumnType::Json);

        Ok(())
    }

    #[test]
    fn test_default_char_maps_to_json() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<char>(ColumnType::Json);

        Ok(())
    }

    #[test]
    fn test_default_unit_maps_to_json() -> Result<(), Box<dyn std::error::Error>> {
        assert_maps_to::<()>(ColumnType::Json);

        Ok(())
    }

    // -- Display edge cases ------------------------------------------------

    #[test]
    fn test_fixed_string_zero() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::FixedString(0).to_string(), "FixedString(0)");

        Ok(())
    }

    #[test]
    fn test_datetime_zero() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::DateTime(0).to_string(), "DateTime(0)");

        Ok(())
    }

    #[test]
    fn test_datetime64_zero() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ColumnType::DateTime64(0).to_string(), "DateTime64(0)");

        Ok(())
    }

    #[test]
    fn test_nullable_triple_nesting_collapsed() -> Result<(), Box<dyn std::error::Error>> {
        // Nullable(Nullable(Nullable(Int32))) -> Nullable(Int32)
        // after two collapses.
        let ct = ColumnType::Nullable(Box::new(ColumnType::Nullable(Box::new(
            ColumnType::Nullable(Box::new(ColumnType::Int32)),
        ))));
        assert_eq!(ct.to_string(), "Nullable(Int32)");

        Ok(())
    }

    #[test]
    fn test_array_nested() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Array(Box::new(ColumnType::Array(Box::new(ColumnType::UInt64))));
        assert_eq!(ct.to_string(), "Array(Array(UInt64))");

        Ok(())
    }

    #[test]
    fn test_array_nullable_deep() -> Result<(), Box<dyn std::error::Error>> {
        let ct = ColumnType::Array(Box::new(ColumnType::Nullable(Box::new(ColumnType::Array(
            Box::new(ColumnType::Nullable(Box::new(ColumnType::Int32))),
        )))));
        assert_eq!(ct.to_string(), "Array(Nullable(Array(Nullable(Int32))))");

        Ok(())
    }

    #[test]
    fn test_decimal_odd_bounds() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            ColumnType::Decimal {
                precision: 1,
                scale: 1,
            }
            .to_string(),
            "Decimal(1, 1)"
        );
        assert_eq!(
            ColumnType::Decimal {
                precision: 76,
                scale: 0,
            }
            .to_string(),
            "Decimal(76, 0)"
        );

        Ok(())
    }
}
