//! Integration tests for `#[derive(Persist)]` macro.
//!
//! These tests verify that the proc-macro generates correct trait impls.
//! "Correct" means the schema and encode_row functions produce expected output
//! for various combinations of field types and annotations.

use ergo_clickhouse_persist::{ColumnType, Persist};
use ergo_clickhouse_persist_derive::Persist;

// ── Basic primitive struct ───────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct Trade {
    price: u64,
    qty: u32,
}

#[test]
fn test_basic_schema() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <Trade as Persist>::table_schema();
    // _persist_time auto-added by TableSchema::new
    assert_eq!(schema.columns.len(), 3);
    assert!(schema.columns.iter().any(|c| c.name == "price"));
    assert!(schema.columns.iter().any(|c| c.name == "qty"));
    assert!(schema.columns.iter().any(|c| c.name == "_persist_time"));
    assert_eq!(
        schema
            .columns
            .iter()
            .find(|c| c.name == "price")
            .unwrap()
            .col_type,
        ColumnType::UInt64
    );
    assert_eq!(
        schema
            .columns
            .iter()
            .find(|c| c.name == "qty")
            .unwrap()
            .col_type,
        ColumnType::UInt32
    );

    Ok(())
}

#[test]
fn test_basic_encode_row() -> Result<(), Box<dyn std::error::Error>> {
    let src = Trade {
        price: 100,
        qty: 10,
    };
    let mut dst = Trade { price: 0, qty: 0 };
    src.encode_row(&mut dst);
    assert_eq!(dst.price, 100);
    assert_eq!(dst.qty, 10);

    Ok(())
}

// ── Custom name ──────────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct Renamed {
    #[persist(name = "custom_name")]
    field: u64,
}

#[test]
fn test_custom_name() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <Renamed as Persist>::table_schema();
    assert!(schema.columns.iter().any(|c| c.name == "custom_name"));
    assert!(!schema.columns.iter().any(|c| c.name == "field"));

    Ok(())
}

// ── Skip ─────────────────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
#[allow(dead_code)]
struct WithSkip {
    included: u64,
    #[persist(skip)]
    excluded: String,
}

#[test]
fn test_skip() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithSkip as Persist>::table_schema();
    assert!(schema.columns.iter().any(|c| c.name == "included"));
    assert!(!schema.columns.iter().any(|c| c.name == "excluded"));

    Ok(())
}

// ── Json annotation ──────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct WithJson {
    #[persist(json)]
    metadata: String,
    normal: u64,
}

#[test]
fn test_json_column_type() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithJson as Persist>::table_schema();
    let json_col = schema
        .columns
        .iter()
        .find(|c| c.name == "metadata")
        .unwrap();
    assert_eq!(json_col.col_type, ColumnType::Json);

    Ok(())
}

// ── Type override ────────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct WithTypeOverride {
    #[persist(type = "Decimal(18, 2)")]
    price: u64,
}

#[test]
fn test_type_override() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithTypeOverride as Persist>::table_schema();
    let col = schema.columns.iter().find(|c| c.name == "price").unwrap();
    assert_eq!(
        col.col_type,
        ColumnType::Decimal {
            precision: 18,
            scale: 2
        }
    );

    Ok(())
}

// ── Order by ─────────────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
#[persist(order_by = "price, qty")]
struct WithOrderBy {
    price: u64,
    qty: u64,
}

#[test]
fn test_custom_order_by() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithOrderBy as Persist>::table_schema();
    assert_eq!(
        schema.order_by,
        vec!["price".to_string(), "qty".to_string()]
    );

    Ok(())
}

// ── TTL ──────────────────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
#[persist(ttl = "ts, 24 HOURS")]
struct WithTtl {
    ts: u64,
    value: u64,
}

#[test]
fn test_ttl_attribute() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithTtl as Persist>::table_schema();
    let ttl = schema.ttl.expect("ttl should be Some");
    assert_eq!(ttl.column, "ts");
    assert_eq!(ttl.interval, "24 HOURS");

    Ok(())
}

// ── Option<T> → Nullable ─────────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct WithOption {
    nullable_field: Option<u64>,
    required: u64,
}

#[test]
fn test_option_nullable() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithOption as Persist>::table_schema();
    let col = schema
        .columns
        .iter()
        .find(|c| c.name == "nullable_field")
        .unwrap();
    assert_eq!(
        col.col_type,
        ColumnType::Nullable(Box::new(ColumnType::UInt64))
    );

    Ok(())
}

// ── Vec<u8> → String ─────────────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct WithBinary {
    blob: Vec<u8>,
}

#[test]
fn test_vec_u8_string() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithBinary as Persist>::table_schema();
    let col = schema.columns.iter().find(|c| c.name == "blob").unwrap();
    assert_eq!(col.col_type, ColumnType::String);

    Ok(())
}

// ── Vec<T> with array annotation (scalar element) ────────────────────────────

#[derive(Persist, Clone)]
struct WithArray {
    #[persist(array)]
    prices: Vec<u64>,
}

#[test]
fn test_array_primitive() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithArray as Persist>::table_schema();
    let col = schema.columns.iter().find(|c| c.name == "prices").unwrap();
    assert_eq!(
        col.col_type,
        ColumnType::Array(Box::new(ColumnType::UInt64))
    );

    Ok(())
}

// ── Flatten nested struct ────────────────────────────────────────────────────

#[derive(Persist, Clone)]
struct InnerFields {
    x: u64,
    y: u64,
}

#[derive(Persist, Clone)]
struct WithFlatten {
    #[persist(flatten)]
    inner: InnerFields,
    z: u64,
}

#[test]
fn test_flatten_schema() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <WithFlatten as Persist>::table_schema();
    assert!(schema.columns.iter().any(|c| c.name == "inner_x"));
    assert!(schema.columns.iter().any(|c| c.name == "inner_y"));
    assert!(schema.columns.iter().any(|c| c.name == "z"));
    // _persist_time auto-added
    assert!(schema.columns.iter().any(|c| c.name == "_persist_time"));
    assert_eq!(schema.columns.len(), 4);

    Ok(())
}

#[test]
fn test_flatten_encode_row() -> Result<(), Box<dyn std::error::Error>> {
    let src = WithFlatten {
        inner: InnerFields { x: 1, y: 2 },
        z: 3,
    };
    let mut dst = WithFlatten {
        inner: InnerFields { x: 0, y: 0 },
        z: 0,
    };
    src.encode_row(&mut dst);
    assert_eq!(dst.inner.x, 1);
    assert_eq!(dst.inner.y, 2);
    assert_eq!(dst.z, 3);

    Ok(())
}

// ── All annotations combined (chrono feature) ─────────────────────────────────

#[cfg(feature = "chrono")]
mod full_example_tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[derive(Persist, Clone)]
    #[persist(order_by = "event_time")]
    struct FullExample {
        event_time: DateTime<Utc>,
        #[persist(name = "sym")]
        symbol: String,
        #[persist(type = "Decimal(18, 8)")]
        price: u64,
        #[persist(json)]
        extra: String,
        #[persist(skip)]
        internal: String,
    }

    #[test]
    fn test_full_example_schema() -> Result<(), Box<dyn std::error::Error>> {
        let schema = <FullExample as Persist>::table_schema();
        // expected: event_time, sym (Decimal), extra (Json), _persist_time = 5
        assert_eq!(schema.columns.len(), 5);

        assert_eq!(
            schema
                .columns
                .iter()
                .find(|c| c.name == "event_time")
                .unwrap()
                .col_type,
            ColumnType::DateTime64(9)
        );
        assert_eq!(
            schema
                .columns
                .iter()
                .find(|c| c.name == "sym")
                .unwrap()
                .col_type,
            ColumnType::String
        );
        assert_eq!(
            schema
                .columns
                .iter()
                .find(|c| c.name == "price")
                .unwrap()
                .col_type,
            ColumnType::Decimal {
                precision: 18,
                scale: 8
            }
        );
        assert_eq!(
            schema
                .columns
                .iter()
                .find(|c| c.name == "extra")
                .unwrap()
                .col_type,
            ColumnType::Json
        );
        assert!(!schema.columns.iter().any(|c| c.name == "internal"));

        assert_eq!(schema.order_by, vec!["event_time".to_string()]);
    
        Ok(())
    }

    #[test]
    fn test_full_example_encode_row() -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let src = FullExample {
            event_time: now,
            symbol: "AAPL".into(),
            price: 15000,
            extra: "{\"note\":\"test\"}".into(),
            internal: "should be skipped".into(),
        };
        let mut dst = FullExample {
            event_time: DateTime::from_timestamp(0, 0).unwrap(),
            symbol: String::new(),
            price: 0,
            extra: String::new(),
            internal: "original".into(),
        };
        src.encode_row(&mut dst);
        assert_eq!(dst.event_time, now);
        assert_eq!(dst.symbol, "AAPL");
        assert_eq!(dst.price, 15000);
        assert_eq!(dst.extra, "{\"note\":\"test\"}");
        // internal should NOT be overwritten because it has #[persist(skip)]
        assert_eq!(dst.internal, "original");
    
        Ok(())
    }
}

// ── Generic struct (compile-pass test) ──────────────────────────────────────

#[derive(Persist, Clone)]
struct Generic<T: Clone + 'static> {
    value: T,
}

#[test]
fn test_generic_struct() -> Result<(), Box<dyn std::error::Error>> {
    // Schema: value (type depends on T), _persist_time
    let schema = <Generic<u64> as Persist>::table_schema();
    assert_eq!(schema.columns.len(), 2);
    // value's type is resolved via default_column_type::<u64>() → UInt64
    assert_eq!(
        schema
            .columns
            .iter()
            .find(|c| c.name == "value")
            .unwrap()
            .col_type,
        ColumnType::UInt64
    );
    Ok(())
}

// ── Combined array and flatten with generic ─────────────────────────────────

#[derive(Persist, Clone)]
struct InnerGen<T: Clone + 'static> {
    a: T,
    b: u64,
}

#[derive(Persist, Clone)]
struct OuterGeneric<T: Clone + 'static> {
    #[persist(flatten)]
    inner: InnerGen<T>,
    extra: u64,
}

#[test]
fn test_outer_generic_schema() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <OuterGeneric<u64> as Persist>::table_schema();
    // Columns: inner_a (UInt64 via default), inner_b (UInt64), extra (UInt64), _persist_time
    assert_eq!(schema.columns.len(), 4);
    assert!(schema.columns.iter().any(|c| c.name == "inner_a"));
    assert!(schema.columns.iter().any(|c| c.name == "inner_b"));
    assert!(schema.columns.iter().any(|c| c.name == "extra"));

    Ok(())
}

#[test]
fn test_outer_generic_encode_row() -> Result<(), Box<dyn std::error::Error>> {
    let src = OuterGeneric {
        inner: InnerGen { a: 42u64, b: 100 },
        extra: 7,
    };
    let mut dst = OuterGeneric {
        inner: InnerGen { a: 0, b: 0 },
        extra: 0,
    };
    src.encode_row(&mut dst);
    assert_eq!(dst.inner.a, 42);
    assert_eq!(dst.inner.b, 100);
    assert_eq!(dst.extra, 7);

    Ok(())
}
