//! Integration tests: encode/decode roundtrips of DynamicSchema and DynamicRow.
#![allow(unused_must_use)] // ponytail: encoder builder calls return &mut Self, not used in tests
//!
//! These tests verify that the ErgoSBE-generated codecs produce wire-compatible
//! binary representations that decode back to the same values.

use ergo_clickhouse_persist::sbe as persist_sbe;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Buffer large enough for any message in the schema.
const BUF_SIZE: usize = 8192;

/// Build a symbol-table blob from a slice of string slices.
/// Each string is stored as raw bytes concatenated (no length prefix —
/// lengths live in the group entry fields).
fn build_symbol_table(strings: &[&str]) -> Vec<u8> {
    let mut blob = Vec::new();
    for s in strings {
        blob.extend_from_slice(s.as_bytes());
    }
    blob
}

// ── DynamicSchema tests ──────────────────────────────────────────────────

#[test]
fn test_dynamic_schema_metadata_only() {
    use persist_sbe::{DynamicSchemaDecoder, DynamicSchemaEncoder};

    let mut buf = [0u8; BUF_SIZE];

    // Encode: schema with metadata but no columns
    let mut encoder = DynamicSchemaEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(42);

    // metadata group: 2 entries — key_len=4 ("name"), val_len=8 ("sbe_test")
    let encoder = encoder
        .metadata(2, |group| {
            group
                .add(|entry| {
                    let _ = entry.key_len(4).val_len(8);
                })
                .unwrap();
            group
                .add(|entry| {
                    let _ = entry.key_len(3).val_len(3);
                })
                .unwrap();
        })
        .unwrap();

    // columns group: 0 entries
    let encoder = encoder.columns(0, |_| {}).unwrap();

    // tableName varString
    let encoder = encoder.table_name(b"sbe_test_tbl").unwrap();

    // symbolTable: "name" + "sbe_test" + "ver" + "1.0"
    let sym = build_symbol_table(&["name", "sbe_test", "ver", "1.0"]);
    let encoded = encoder.symbol_table(&sym).unwrap();
    let bytes = encoded.as_bytes();

    // Decode
    let decoder = DynamicSchemaDecoder::try_from(bytes).unwrap();
    assert_eq!(decoder.schema_id(), 42);

    let mut md = decoder.into_metadata().unwrap();
    assert_eq!(md.len(), 2);

    let e0 = md.next().unwrap();
    assert_eq!(e0.key_len(), 4);
    assert_eq!(e0.val_len(), 8);

    let e1 = md.next().unwrap();
    assert_eq!(e1.key_len(), 3);
    assert_eq!(e1.val_len(), 3);
    let decoder = md.finish().unwrap();

    let cols = decoder.into_columns().unwrap();
    assert_eq!(cols.len(), 0);
    let decoder = cols.finish().unwrap();

    let (table_name, decoder) = decoder.into_table_name().unwrap();
    assert_eq!(table_name, b"sbe_test_tbl");
    let (symbols, _) = decoder.into_symbol_table().unwrap();
    assert_eq!(symbols, sym.as_slice());
}

#[test]
fn test_dynamic_schema_with_columns() {
    use persist_sbe::{DynamicSchemaDecoder, DynamicSchemaEncoder};

    let mut buf = [0u8; BUF_SIZE];

    let mut encoder = DynamicSchemaEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(7);

    // No metadata
    let encoder = encoder.metadata(0, |_| {}).unwrap();

    // columns group: 3 entries
    let encoder = encoder
        .columns(3, |group| {
            // field_id=1, name_len=5, type_tag=1 (UInt64)
            group
                .add(|entry| {
                    let _ = entry.field_id(1).name_len(5).type_tag(1);
                })
                .unwrap();
            // field_id=2, name_len=6, type_tag=4 (String)
            group
                .add(|entry| {
                    let _ = entry.field_id(2).name_len(6).type_tag(4);
                })
                .unwrap();
            // field_id=3, name_len=8, type_tag=0 (Int64)
            group
                .add(|entry| {
                    let _ = entry.field_id(3).name_len(8).type_tag(0);
                })
                .unwrap();
        })
        .unwrap();

    let encoder = encoder.table_name(b"price_feed").unwrap();

    let sym = build_symbol_table(&["price", "symbol", "quantity"]);
    let encoded = encoder.symbol_table(&sym).unwrap();
    let bytes = encoded.as_bytes();

    // Decode and verify
    let decoder = DynamicSchemaDecoder::try_from(bytes).unwrap();
    assert_eq!(decoder.schema_id(), 7);

    let decoder = decoder.into_metadata().unwrap().finish().unwrap();
    let mut cols = decoder.into_columns().unwrap();
    assert_eq!(cols.len(), 3);

    let c0 = cols.next().unwrap();
    assert_eq!(c0.field_id(), 1);
    assert_eq!(c0.name_len(), 5);
    assert_eq!(c0.type_tag(), 1);

    let c1 = cols.next().unwrap();
    assert_eq!(c1.field_id(), 2);
    assert_eq!(c1.name_len(), 6);
    assert_eq!(c1.type_tag(), 4);

    let c2 = cols.next().unwrap();
    assert_eq!(c2.field_id(), 3);
    assert_eq!(c2.name_len(), 8);
    assert_eq!(c2.type_tag(), 0);
    let decoder = cols.finish().unwrap();

    let (table_name, decoder) = decoder.into_table_name().unwrap();
    assert_eq!(table_name, b"price_feed");
    let (symbols, _) = decoder.into_symbol_table().unwrap();
    assert_eq!(symbols, sym.as_slice());
}

#[test]
fn test_dynamic_schema_empty_metadata() {
    use persist_sbe::{DynamicSchemaDecoder, DynamicSchemaEncoder};

    let mut buf = [0u8; BUF_SIZE];

    let mut encoder = DynamicSchemaEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(99);

    // metadata + columns both empty; empty tableName; empty symbolTable
    let encoder = encoder.metadata(0, |_| {}).unwrap();
    let encoder = encoder.columns(0, |_| {}).unwrap();
    let encoder = encoder.table_name(b"").unwrap();
    let encoded = encoder.symbol_table(b"").unwrap();
    let bytes = encoded.as_bytes();

    let decoder = DynamicSchemaDecoder::try_from(bytes).unwrap();
    assert_eq!(decoder.schema_id(), 99);
    let md = decoder.into_metadata().unwrap();
    assert_eq!(md.len(), 0);
    let decoder = md.finish().unwrap();
    let cols = decoder.into_columns().unwrap();
    assert_eq!(cols.len(), 0);
    let decoder = cols.finish().unwrap();
    let (table_name, decoder) = decoder.into_table_name().unwrap();
    assert_eq!(table_name, b"");
    let (symbols, _) = decoder.into_symbol_table().unwrap();
    assert_eq!(symbols, b"");
}

// ── DynamicRow tests ─────────────────────────────────────────────────────

#[test]
fn test_dynamic_row_empty() {
    use persist_sbe::{DynamicRowDecoder, DynamicRowEncoder};

    let mut buf = [0u8; BUF_SIZE];

    let mut encoder = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(100);
    let encoder = encoder.row_metadata(0, |_| {}).unwrap();
    let encoder = encoder.int64_fields(0, |_| {}).unwrap();
    let encoder = encoder.uint64_fields(0, |_| {}).unwrap();
    let encoder = encoder.float64_fields(0, |_| {}).unwrap();
    let encoder = encoder.bool_fields(0, |_| {}).unwrap();
    let encoder = encoder.string_fields(0, |_| {}).unwrap();
    let encoder = encoder.null_fields(0, |_| {}).unwrap();
    let encoded = encoder.symbol_table(b"").unwrap();
    let bytes = encoded.as_bytes();

    let decoder = DynamicRowDecoder::try_from(bytes).unwrap();
    assert_eq!(decoder.schema_id(), 100);
    let g = decoder.into_row_metadata().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let g = decoder.into_int64_fields().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let g = decoder.into_uint64_fields().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let g = decoder.into_float64_fields().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let g = decoder.into_bool_fields().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let g = decoder.into_string_fields().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let g = decoder.into_null_fields().unwrap();
    assert!(g.is_empty());
    let decoder = g.finish().unwrap();
    let (symbols, _) = decoder.into_symbol_table().unwrap();
    assert_eq!(symbols, b"");
}

#[test]
fn test_dynamic_row_all_field_types() {
    use persist_sbe::{DynamicRowDecoder, DynamicRowEncoder};

    let mut buf = [0u8; BUF_SIZE];

    let mut encoder = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(200);

    // rowMetadata: 1 entry — key_len=4 ("type"), val_len=5 ("trade")
    let encoder = encoder
        .row_metadata(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.key_len(4).val_len(5);
                })
                .unwrap();
        })
        .unwrap();

    // int64: field_id=1, value=-42
    let encoder = encoder
        .int64_fields(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(1).value(-42i64);
                })
                .unwrap();
        })
        .unwrap();

    // uint64: field_id=2, value=1234567890
    let encoder = encoder
        .uint64_fields(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(2).value(1234567890u64);
                })
                .unwrap();
        })
        .unwrap();

    // float64: field_id=3, value=3.14159
    let encoder = encoder
        .float64_fields(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(3).value(std::f64::consts::PI);
                })
                .unwrap();
        })
        .unwrap();

    // bool: field_id=4, value=1 (true)
    let encoder = encoder
        .bool_fields(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(4).value(1u8);
                })
                .unwrap();
        })
        .unwrap();

    // string: field_id=5, str_len=5 ("hello")
    let encoder = encoder
        .string_fields(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(5).str_len(5);
                })
                .unwrap();
        })
        .unwrap();

    // null: field_id=6
    let encoder = encoder
        .null_fields(1, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(6);
                })
                .unwrap();
        })
        .unwrap();

    // symbolTable: "type" + "trade" + "hello"
    let sym = build_symbol_table(&["type", "trade", "hello"]);
    let encoded = encoder.symbol_table(&sym).unwrap();
    let bytes = encoded.as_bytes();

    // Decode and verify
    let decoder = DynamicRowDecoder::try_from(bytes).unwrap();
    assert_eq!(decoder.schema_id(), 200);

    let mut md = decoder.into_row_metadata().unwrap();
    assert_eq!(md.len(), 1);
    let e0 = md.next().unwrap();
    assert_eq!(e0.key_len(), 4);
    assert_eq!(e0.val_len(), 5);
    let decoder = md.finish().unwrap();

    let mut i64g = decoder.into_int64_fields().unwrap();
    assert_eq!(i64g.len(), 1);
    let e = i64g.next().unwrap();
    assert_eq!(e.field_id(), 1);
    assert_eq!(e.value(), -42i64);
    let decoder = i64g.finish().unwrap();

    let mut u64g = decoder.into_uint64_fields().unwrap();
    assert_eq!(u64g.len(), 1);
    let e = u64g.next().unwrap();
    assert_eq!(e.field_id(), 2);
    assert_eq!(e.value(), 1234567890u64);
    let decoder = u64g.finish().unwrap();

    let mut f64g = decoder.into_float64_fields().unwrap();
    assert_eq!(f64g.len(), 1);
    let e = f64g.next().unwrap();
    assert_eq!(e.field_id(), 3);
    assert!((e.value() - std::f64::consts::PI).abs() < 1e-10);
    let decoder = f64g.finish().unwrap();

    let mut bg = decoder.into_bool_fields().unwrap();
    assert_eq!(bg.len(), 1);
    let e = bg.next().unwrap();
    assert_eq!(e.field_id(), 4);
    assert_eq!(e.value(), 1u8);
    let decoder = bg.finish().unwrap();

    let mut sg = decoder.into_string_fields().unwrap();
    assert_eq!(sg.len(), 1);
    let e = sg.next().unwrap();
    assert_eq!(e.field_id(), 5);
    assert_eq!(e.str_len(), 5);
    let decoder = sg.finish().unwrap();

    let mut ng = decoder.into_null_fields().unwrap();
    assert_eq!(ng.len(), 1);
    let e = ng.next().unwrap();
    assert_eq!(e.field_id(), 6);
    let decoder = ng.finish().unwrap();

    let (symbols, _) = decoder.into_symbol_table().unwrap();
    assert_eq!(symbols, sym.as_slice());
}

#[test]
fn test_dynamic_row_multiple_entries() {
    use persist_sbe::{DynamicRowDecoder, DynamicRowEncoder};

    let mut buf = [0u8; BUF_SIZE];

    let mut encoder = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(300);

    // rowMetadata group: 0 entries
    let encoder = encoder.row_metadata(0, |_| {}).unwrap();

    // 3 int64 entries
    let encoder = encoder
        .int64_fields(3, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(10).value(100i64);
                })
                .unwrap();
            group
                .add(|entry| {
                    let _ = entry.field_id(20).value(200i64);
                })
                .unwrap();
            group
                .add(|entry| {
                    let _ = entry.field_id(30).value(300i64);
                })
                .unwrap();
        })
        .unwrap();

    // 2 uint64 entries
    let encoder = encoder
        .uint64_fields(2, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(40).value(400u64);
                })
                .unwrap();
            group
                .add(|entry| {
                    let _ = entry.field_id(50).value(500u64);
                })
                .unwrap();
        })
        .unwrap();

    // No other groups, empty symbolTable
    let encoder = encoder.float64_fields(0, |_| {}).unwrap();
    let encoder = encoder.bool_fields(0, |_| {}).unwrap();
    let encoder = encoder.string_fields(0, |_| {}).unwrap();
    let encoder = encoder.null_fields(0, |_| {}).unwrap();
    let encoded = encoder.symbol_table(b"").unwrap();
    let bytes = encoded.as_bytes();

    let decoder = DynamicRowDecoder::try_from(bytes).unwrap();
    assert_eq!(decoder.schema_id(), 300);

    let decoder = decoder.into_row_metadata().unwrap().finish().unwrap();
    let mut i64s = decoder.into_int64_fields().unwrap();
    assert_eq!(i64s.len(), 3);
    let e0 = i64s.next().unwrap();
    assert_eq!(e0.field_id(), 10);
    assert_eq!(e0.value(), 100i64);
    let e1 = i64s.next().unwrap();
    assert_eq!(e1.field_id(), 20);
    assert_eq!(e1.value(), 200i64);
    let e2 = i64s.next().unwrap();
    assert_eq!(e2.field_id(), 30);
    assert_eq!(e2.value(), 300i64);

    let decoder = i64s.finish().unwrap();
    let mut u64s = decoder.into_uint64_fields().unwrap();
    assert_eq!(u64s.len(), 2);
    let e0 = u64s.next().unwrap();
    assert_eq!(e0.field_id(), 40);
    assert_eq!(e0.value(), 400u64);
    let e1 = u64s.next().unwrap();
    assert_eq!(e1.field_id(), 50);
    assert_eq!(e1.value(), 500u64);
}

#[test]
fn test_dynamic_row_string_roundtrip() {
    use persist_sbe::{DynamicRowDecoder, DynamicRowEncoder};

    let mut buf = [0u8; BUF_SIZE];

    let mut encoder = DynamicRowEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.schema_id(400);

    // rowMetadata, int64, uint64, float64, bool, and null groups: all empty
    let encoder = encoder.row_metadata(0, |_| {}).unwrap();
    let encoder = encoder.int64_fields(0, |_| {}).unwrap();
    let encoder = encoder.uint64_fields(0, |_| {}).unwrap();
    let encoder = encoder.float64_fields(0, |_| {}).unwrap();
    let encoder = encoder.bool_fields(0, |_| {}).unwrap();

    // 3 string fields: "abc" (3), "hello world" (11), "SBE" (3)
    let encoder = encoder
        .string_fields(3, |group| {
            group
                .add(|entry| {
                    let _ = entry.field_id(1).str_len(3);
                })
                .unwrap();
            group
                .add(|entry| {
                    let _ = entry.field_id(2).str_len(11);
                })
                .unwrap();
            group
                .add(|entry| {
                    let _ = entry.field_id(3).str_len(3);
                })
                .unwrap();
        })
        .unwrap();
    let encoder = encoder.null_fields(0, |_| {}).unwrap();

    let sym = build_symbol_table(&["abc", "hello world", "SBE"]);
    let encoded = encoder.symbol_table(&sym).unwrap();
    let bytes = encoded.as_bytes();

    // Decode
    let decoder = DynamicRowDecoder::try_from(bytes).unwrap();

    let decoder = decoder.into_row_metadata().unwrap().finish().unwrap();
    let decoder = decoder.into_int64_fields().unwrap().finish().unwrap();
    let decoder = decoder.into_uint64_fields().unwrap().finish().unwrap();
    let decoder = decoder.into_float64_fields().unwrap().finish().unwrap();
    let decoder = decoder.into_bool_fields().unwrap().finish().unwrap();
    let mut sg = decoder.into_string_fields().unwrap();
    assert_eq!(sg.len(), 3);
    let e0 = sg.next().unwrap();
    assert_eq!(e0.str_len(), 3);
    let e1 = sg.next().unwrap();
    assert_eq!(e1.str_len(), 11);
    let e2 = sg.next().unwrap();
    assert_eq!(e2.str_len(), 3);
    let decoder = sg.finish().unwrap();

    // Verify symbolTable contains the packed string data
    let decoder = decoder.into_null_fields().unwrap().finish().unwrap();
    let (st, _) = decoder.into_symbol_table().unwrap();
    assert_eq!(st.len(), 3 + 11 + 3);
    assert_eq!(&st[0..3], b"abc");
    assert_eq!(&st[3..14], b"hello world");
    assert_eq!(&st[14..17], b"SBE");
}

// ── Cross-message tests ──────────────────────────────────────────────────

#[test]
fn test_schema_and_row_same_schema_id() {
    // Verify that a DynamicSchema + DynamicRow with matching schema_id
    // both encode/decode independently.
    use persist_sbe::{DynamicRowDecoder, DynamicRowEncoder};
    use persist_sbe::{DynamicSchemaDecoder, DynamicSchemaEncoder};

    let shared_schema_id: u32 = 777;
    let mut schema_buf = [0u8; BUF_SIZE];
    let mut row_buf = [0u8; BUF_SIZE];

    // Encode DynamicSchema
    let mut enc = DynamicSchemaEncoder::wrap_and_apply_header(&mut schema_buf, 0).unwrap();
    enc.schema_id(shared_schema_id);
    let enc = enc.metadata(0, |_| {}).unwrap();
    let enc = enc.columns(0, |_| {}).unwrap();
    let enc = enc.table_name(b"shared").unwrap();
    let schema_enc = enc.symbol_table(b"").unwrap();
    let schema_bytes = schema_enc.as_bytes();

    // Encode DynamicRow
    let mut enc = DynamicRowEncoder::wrap_and_apply_header(&mut row_buf, 0).unwrap();
    enc.schema_id(shared_schema_id);
    let enc = enc.row_metadata(0, |_| {}).unwrap();
    let enc = enc.int64_fields(0, |_| {}).unwrap();
    let enc = enc.uint64_fields(0, |_| {}).unwrap();
    let enc = enc.float64_fields(0, |_| {}).unwrap();
    let enc = enc.bool_fields(0, |_| {}).unwrap();
    let enc = enc.string_fields(0, |_| {}).unwrap();
    let enc = enc.null_fields(0, |_| {}).unwrap();
    let row_enc = enc.symbol_table(b"").unwrap();
    let row_bytes = row_enc.as_bytes();

    // Decode both
    let schema_dec = DynamicSchemaDecoder::try_from(schema_bytes).unwrap();
    assert_eq!(schema_dec.schema_id(), shared_schema_id);

    let row_dec = DynamicRowDecoder::try_from(row_bytes).unwrap();
    assert_eq!(row_dec.schema_id(), shared_schema_id);
}
