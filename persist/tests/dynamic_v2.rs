//! DynamicRecorderV2 — borrowed-value, caller-buffer V2 row encoding tests.
//!
//! Plan Task 6: decimal arrays with mixed exponents, null/empty arrays,
//! malformed counts, exact `Array(Decimal(38,18))` type mapping, exact
//! length computation, and byte-level V2 wire round trips through the
//! generated `DynamicRowV2` decoder.

use ergo_clickhouse_persist::ColumnType;
use ergo_clickhouse_persist::dynamic::{
    DynamicRecorderBuilder, DynamicRecorderError, DynamicValueRef,
};
use ergo_clickhouse_persist::sbe::v2::DynamicRowV2Decoder;

fn decimal_array() -> ColumnType {
    ColumnType::Array(Box::new(ColumnType::Decimal {
        precision: 38,
        scale: 18,
    }))
}

fn builder() -> DynamicRecorderBuilder {
    DynamicRecorderBuilder::new("l2book_dynamic")
        .field("sequence", ColumnType::UInt64)
        .field("symbol", ColumnType::String)
        .field("bid_prices", decimal_array())
        .field("ask_prices", decimal_array())
}

#[test]
fn v1_build_rejects_decimal_array_column() {
    let err = match DynamicRecorderBuilder::new("t")
        .field("bids", decimal_array())
        .build()
    {
        Ok(_) => panic!("V1 build must reject Array(Decimal) columns"),
        Err(e) => e,
    };
    assert!(matches!(
        err,
        DynamicRecorderError::UnsupportedColumnType { .. }
    ));
}

#[test]
fn v2_build_accepts_decimal_array_column() {
    let rec = builder().build_v2().unwrap();
    assert_ne!(rec.schema_id(), 0);
}

#[test]
fn record_into_roundtrips_mixed_exponent_arrays() {
    let rec = builder().build_v2().unwrap();
    let bids = [(500005i64, -1i8), (49_999_000_000i64, -6i8)];
    let asks = [(500015i64, -1i8)];
    let values = [
        DynamicValueRef::UInt64(42),
        DynamicValueRef::String("BTCUSDT"),
        DynamicValueRef::DecimalArray(&bids),
        DynamicValueRef::DecimalArray(&asks),
    ];

    let len = rec.compute_encoded_length(&values).unwrap();
    let mut buf = vec![0u8; len];
    let encoded = rec.record_into(&mut buf, &values).unwrap();
    assert_eq!(
        encoded.len(),
        len,
        "record_into must fill exactly len bytes"
    );

    // Decode through the generated V2 consuming stages.
    let dec = DynamicRowV2Decoder::wrap_and_apply_header(encoded, 0).unwrap();
    assert_eq!(dec.schema_id(), rec.schema_id());

    let g = dec.into_row_metadata().unwrap();
    assert_eq!(g.len(), 0);
    let dec = g.finish().unwrap();

    let g = dec.into_int64_fields().unwrap();
    assert_eq!(g.len(), 0);
    let dec = g.finish().unwrap();

    let mut u64s = Vec::new();
    let mut g = dec.into_uint64_fields().unwrap();
    for e in g.by_ref() {
        u64s.push((e.field_id(), e.value()));
    }
    let dec = g.finish().unwrap();
    assert_eq!(u64s, vec![(0u8, 42u64)]);

    let dec = dec.into_float64_fields().unwrap().finish().unwrap();
    let dec = dec.into_bool_fields().unwrap().finish().unwrap();

    let mut strs = Vec::new();
    let mut g = dec.into_string_fields().unwrap();
    for e in g.by_ref() {
        strs.push((e.field_id(), e.str_len()));
    }
    let dec = g.finish().unwrap();
    assert_eq!(strs, vec![(1u8, 7u16)]);

    let g = dec.into_null_fields().unwrap();
    assert_eq!(g.len(), 0);
    let dec = g.finish().unwrap();

    let mut arrays: Vec<(u8, Vec<(i64, i8)>)> = Vec::new();
    let mut g = dec.into_decimal_array_fields().unwrap();
    for e in g.by_ref() {
        let e = e.unwrap();
        let fid = e.field_id();
        let vals: Vec<(i64, i8)> = e
            .values()
            .unwrap()
            .map(|v| (v.mantissa(), v.exponent()))
            .collect();
        arrays.push((fid, vals));
    }
    let dec = g.finish().unwrap();
    assert_eq!(
        arrays,
        vec![
            (2u8, vec![(500005, -1), (49_999_000_000, -6)]),
            (3u8, vec![(500015, -1)]),
        ]
    );

    // Symbol table carries the string bytes.
    let (symbols, _complete) = dec.into_symbol_table().unwrap();
    assert_eq!(&symbols[symbols.len() - 7..], b"BTCUSDT");
}

#[test]
fn record_into_empty_array_and_null() {
    let rec = builder().build_v2().unwrap();
    let empty: [(i64, i8); 0] = [];
    let values = [
        DynamicValueRef::UInt64(1),
        DynamicValueRef::Null,
        DynamicValueRef::DecimalArray(&empty),
        DynamicValueRef::Null,
    ];
    let len = rec.compute_encoded_length(&values).unwrap();
    let mut buf = vec![0u8; len];
    let encoded = rec.record_into(&mut buf, &values).unwrap();
    assert_eq!(encoded.len(), len);

    let dec = DynamicRowV2Decoder::wrap_and_apply_header(encoded, 0).unwrap();
    let dec = dec.into_row_metadata().unwrap().finish().unwrap();
    let dec = dec.into_int64_fields().unwrap().finish().unwrap();
    let dec = dec.into_uint64_fields().unwrap().finish().unwrap();
    let dec = dec.into_float64_fields().unwrap().finish().unwrap();
    let dec = dec.into_bool_fields().unwrap().finish().unwrap();
    let dec = dec.into_string_fields().unwrap().finish().unwrap();
    let g = dec.into_null_fields().unwrap();
    assert_eq!(g.len(), 2, "two null positions");
    let dec = g.finish().unwrap();
    let mut g = dec.into_decimal_array_fields().unwrap();
    let mut n_arrays = 0;
    let mut n_vals = 0;
    for e in g.by_ref() {
        let e = e.unwrap();
        n_arrays += 1;
        n_vals += e.values().unwrap().count();
    }
    assert_eq!((n_arrays, n_vals), (1, 0), "one empty array entry");
}

#[test]
fn record_into_value_count_mismatch() {
    let rec = builder().build_v2().unwrap();
    let values = [DynamicValueRef::UInt64(1)];
    assert!(matches!(
        rec.compute_encoded_length(&values),
        Err(DynamicRecorderError::ValueCountMismatch { .. })
    ));
    let mut buf = vec![0u8; 256];
    assert!(matches!(
        rec.record_into(&mut buf, &values),
        Err(DynamicRecorderError::ValueCountMismatch { .. })
    ));
}

#[test]
fn record_into_value_type_mismatch() {
    let rec = builder().build_v2().unwrap();
    let bids = [(1i64, 0i8)];
    let values = [
        DynamicValueRef::String("wrong"),
        DynamicValueRef::String("BTCUSDT"),
        DynamicValueRef::DecimalArray(&bids),
        DynamicValueRef::DecimalArray(&bids),
    ];
    let mut buf = vec![0u8; 512];
    assert!(matches!(
        rec.record_into(&mut buf, &values),
        Err(DynamicRecorderError::ValueTypeMismatch { .. })
    ));
}

#[test]
fn record_into_buffer_too_short_is_error() {
    let rec = builder().build_v2().unwrap();
    let bids = [(500005i64, -1i8)];
    let values = [
        DynamicValueRef::UInt64(42),
        DynamicValueRef::String("BTCUSDT"),
        DynamicValueRef::DecimalArray(&bids),
        DynamicValueRef::DecimalArray(&bids),
    ];
    let len = rec.compute_encoded_length(&values).unwrap();
    let mut buf = vec![0u8; len - 1];
    assert!(rec.record_into(&mut buf, &values).is_err());
}

#[test]
fn schema_into_roundtrips_column_metadata() {
    use ergo_clickhouse_persist::sbe::v2::DynamicSchemaV2Decoder;

    let rec = builder().build_v2().unwrap();
    let len = rec.schema_encoded_length();
    let mut buf = vec![0u8; len];
    let encoded = rec.schema_into(&mut buf).unwrap();
    assert_eq!(
        encoded.len(),
        len,
        "schema_into must fill exactly len bytes"
    );

    let dec = DynamicSchemaV2Decoder::wrap_and_apply_header(encoded, 0).unwrap();
    assert_eq!(dec.schema_id(), rec.schema_id());

    let dec = dec.into_metadata().unwrap().finish().unwrap();

    let mut cols = Vec::new();
    let mut g = dec.into_columns().unwrap();
    for e in g.by_ref() {
        cols.push((
            e.field_id(),
            e.name_len() as usize,
            e.outer_type(),
            e.inner_type(),
            e.precision(),
            e.scale(),
        ));
    }
    let dec = g.finish().unwrap();
    // sequence UInt64, symbol String, bid_prices/ask_prices Array(Decimal(38,18))
    assert_eq!(
        cols,
        vec![
            (0, "sequence".len(), 0, 2, 0, 0),
            (1, "symbol".len(), 0, 5, 0, 0),
            (2, "bid_prices".len(), 1, 6, 38, 18),
            (3, "ask_prices".len(), 1, 6, 38, 18),
        ]
    );

    let (table_name, dec) = dec.into_table_name().unwrap();
    assert_eq!(table_name, b"l2book_dynamic");
    let (symbols, _complete) = dec.into_symbol_table().unwrap();
    assert_eq!(symbols, b"sequencesymbolbid_pricesask_prices");
}

#[test]
fn v2_row_decodes_under_newer_acting_version() {
    // A frame stamped with a NEWER schema version (acting version 2) must
    // still decode: all V2 row fields exist since version 0/1.
    let rec = builder().build_v2().unwrap();
    let bids = [(500005i64, -1i8)];
    let values = [
        DynamicValueRef::UInt64(42),
        DynamicValueRef::String("BTCUSDT"),
        DynamicValueRef::DecimalArray(&bids),
        DynamicValueRef::DecimalArray(&bids),
    ];
    let len = rec.compute_encoded_length(&values).unwrap();
    let mut buf = vec![0u8; len];
    rec.record_into(&mut buf, &values).unwrap();

    // Bump the header version field (offset 6..8, little-endian).
    buf[6..8].copy_from_slice(&2u16.to_le_bytes());

    let dec = DynamicRowV2Decoder::wrap_and_apply_header(&buf, 0).unwrap();
    assert_eq!(dec.acting_version(), 2);
    assert_eq!(dec.schema_id(), rec.schema_id());
    let dec = dec.into_row_metadata().unwrap().finish().unwrap();
    let dec = dec.into_int64_fields().unwrap().finish().unwrap();
    let mut g = dec.into_uint64_fields().unwrap();
    let e = g.next().unwrap();
    assert_eq!((e.field_id(), e.value()), (0, 42));
}
