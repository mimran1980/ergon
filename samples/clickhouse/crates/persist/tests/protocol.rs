//! Protocol tests: declaration and envelope round trips, malformed
//! inputs, unknown kinds, and catalog ID behavior.

use ergo_clickhouse_persist::persist::EncodeError;
use ergo_clickhouse_persist::protocol::{
    self, ColumnDeclaration, Declaration, LayoutDeclaration, Policy, PolicyDeclaration,
    RowEncoding, SessionCatalog, SessionStartDeclaration, SymbolDeclaration, limits,
};
use ergo_clickhouse_persist::recording as rec;

fn round_trip(decl: &Declaration) -> Declaration {
    let mut buf = vec![0u8; decl.max_encoded_len() * 2];
    let len = decl.encode(&mut buf).expect("encode");
    Declaration::decode(decl.kind(), &buf[..len]).expect("decode")
}

#[test]
fn session_start_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let d = Declaration::SessionStart(SessionStartDeclaration {
        run_id: 0x1234_5678_9abc_def0,
        started_at_ns: 1_758_000_000_000_000_000,
        process: "market-recorder".into(),
        instance: "binance-0".into(),
        build: "0.1.0+a1b2c3".into(),
    });
    assert_eq!(round_trip(&d), d);
    Ok(())
}

#[test]
fn symbol_and_policy_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let s = Declaration::Symbol(SymbolDeclaration {
        symbol_id: 7,
        value: "BTC/USDT".into(),
    });
    assert_eq!(round_trip(&s), s);
    let p = Declaration::Policy(PolicyDeclaration {
        policy_id: 3,
        policy: Policy::Temporary,
        row_ttl_ns: 86_400_000_000_000,
        idle_ttl_ns: 7 * 86_400_000_000_000,
    });
    assert_eq!(round_trip(&p), p);
    Ok(())
}

#[test]
fn layout_round_trip_with_columns() -> Result<(), Box<dyn std::error::Error>> {
    let d = Declaration::Layout(LayoutDeclaration {
        layout_id: 5,
        policy_id: 2,
        payload_encoding: RowEncoding::OrderedRow,
        schema_fingerprint: 0xdead_beef_cafe_1234,
        projection_revision: 1,
        sbe_schema_id: 0,
        sbe_template_id: 0,
        sbe_version: 0,
        catalog_generation: 11,
        table_name: "order_book_deltas".into(),
        columns: vec![
            ColumnDeclaration {
                name: "instrument".into(),
                type_code: 8, // u32
                flags: 0,
                precision: 0,
                scale: 0,
            },
            ColumnDeclaration {
                name: "bids__price".into(),
                type_code: 12, // decimal
                flags: 2,      // array
                precision: 18,
                scale: 8,
            },
            ColumnDeclaration {
                name: "note".into(),
                type_code: 13, // utf8
                flags: 1,      // nullable
                precision: 0,
                scale: 0,
            },
        ],
    });
    assert_eq!(round_trip(&d), d);
    Ok(())
}

#[test]
fn data_record_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let payload = b"raw sbe bytes \x00\x01\xff";
    let len = protocol::encode_data_record(
        &mut buf,
        rec::RecordKind::RawSbe,
        1,
        5,
        2,
        42,
        1_758_000_000_000_000_000,
        3,
        payload,
    )?;
    let env = protocol::decode_data_record(&buf[..len])?;
    assert_eq!(env.payload, payload);
    assert_eq!(env.metadata.writer_id, 1);
    assert_eq!(env.metadata.sequence, 42);
    assert_eq!(env.metadata.captured_at_ns, 1_758_000_000_000_000_000);
    assert_eq!(env.metadata.catalog_generation, 3);
    assert_eq!(env.kind, rec::RecordKind::RawSbe);
    Ok(())
}

#[test]
fn data_record_exact_sizing() -> Result<(), Box<dyn std::error::Error>> {
    let payload = [7u8; 100];
    let needed = rec::DataRecordEncoder::compute_length_with_header(payload.len());
    let mut buf = vec![0u8; needed];
    let len = protocol::encode_data_record(
        &mut buf,
        rec::RecordKind::TypedRow,
        1,
        1,
        1,
        1,
        0,
        1,
        &payload,
    )?;
    assert_eq!(len, needed);
    // one byte less must be rejected, not truncated
    let mut small = vec![0u8; needed - 1];
    assert!(matches!(
        protocol::encode_data_record(
            &mut small,
            rec::RecordKind::TypedRow,
            1,
            1,
            1,
            1,
            0,
            1,
            &payload
        ),
        Err(EncodeError::BufferTooSmall { .. })
    ));
    Ok(())
}

#[test]
fn malformed_truncated_envelope_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 4096];
    let len = protocol::encode_data_record(
        &mut buf,
        rec::RecordKind::TypedRow,
        1,
        1,
        1,
        1,
        0,
        1,
        b"payload here",
    )?;
    // truncate inside the payload region
    assert!(protocol::decode_data_record(&buf[..len - 4]).is_err());
    // truncated header
    assert!(protocol::decode_data_record(&buf[..6]).is_err());
    Ok(())
}

#[test]
fn unknown_template_kind_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // DeclarationKind::NullVal is not a decodable kind
    let mut buf = vec![0u8; 64];
    let len = Declaration::SessionEnd {
        ended_at_ns: 1,
        final_sequence: 2,
    }
    .encode(&mut buf)?;
    assert!(matches!(
        Declaration::decode(rec::DeclarationKind::NullVal, &buf[..len]),
        Err(EncodeError::Custom(902))
    ));
    Ok(())
}

#[test]
fn session_end_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let d = Declaration::SessionEnd {
        ended_at_ns: 99,
        final_sequence: 1_000,
    };
    assert_eq!(round_trip(&d), d);
    Ok(())
}

#[test]
fn catalog_assigns_compact_ids_and_generations() -> Result<(), Box<dyn std::error::Error>> {
    let mut cat = SessionCatalog::new();
    assert_eq!(cat.generation(), 1);
    let pid = cat.register_policy(PolicyDeclaration {
        policy_id: 0,
        policy: Policy::Permanent,
        row_ttl_ns: 0,
        idle_ttl_ns: 0,
    })?;
    assert_eq!(pid, 1);
    let layout = |fp: u64| LayoutDeclaration {
        layout_id: 0,
        policy_id: pid,
        payload_encoding: RowEncoding::OrderedRow,
        schema_fingerprint: fp,
        projection_revision: 1,
        sbe_schema_id: 0,
        sbe_template_id: 0,
        sbe_version: 0,
        catalog_generation: 0,
        table_name: "t".into(),
        columns: vec![ColumnDeclaration {
            name: "c".into(),
            type_code: 5,
            flags: 0,
            precision: 0,
            scale: 0,
        }],
    };
    let l1 = cat.register_layout(layout(100))?;
    assert_eq!(l1, 1);
    let gen_after_l1 = cat.generation();
    assert_eq!(gen_after_l1, 3); // policy + layout each bump
    let l1_again = cat.register_layout(layout(100))?;
    assert_eq!(l1_again, 1, "identical retry returns the same id");
    assert_eq!(
        cat.generation(),
        gen_after_l1,
        "retry does not bump generation"
    );
    // conflicting definition under the same fingerprint
    let mut conflict = layout(100);
    conflict.table_name = "other".into();
    assert!(matches!(
        cat.register_layout(conflict),
        Err(EncodeError::Custom(917))
    ));
    // distinct fingerprint gets a new id
    let l2 = cat.register_layout(layout(200))?;
    assert_eq!(l2, 2);
    assert!(cat.layout(l1).is_some());
    assert_eq!(cat.symbol(99), None);
    Ok(())
}

#[test]
fn catalog_symbol_interning_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let mut cat = SessionCatalog::new();
    let a = cat.intern_symbol("BTC/USDT")?;
    let b = cat.intern_symbol("BTC/USDT")?;
    let c = cat.intern_symbol("ETH/USDT")?;
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(cat.symbol(a).map(|s| s.value.as_str()), Some("BTC/USDT"));
    Ok(())
}

#[test]
fn declaration_sizing_never_overruns() -> Result<(), Box<dyn std::error::Error>> {
    let d = Declaration::Layout(LayoutDeclaration {
        layout_id: 1,
        policy_id: 1,
        payload_encoding: RowEncoding::DynamicRow,
        schema_fingerprint: 7,
        projection_revision: 1,
        sbe_schema_id: 0,
        sbe_template_id: 0,
        sbe_version: 0,
        catalog_generation: 1,
        table_name: "strategy_state".into(),
        columns: (0..limits::MAX_COLUMNS)
            .map(|i| ColumnDeclaration {
                name: format!("column_with_a_reasonably_long_name_{i}"),
                type_code: 9,
                flags: 0,
                precision: 0,
                scale: 0,
            })
            .collect(),
    });
    let mut buf = vec![0u8; d.max_encoded_len()];
    let len = d.encode(&mut buf)?;
    assert_eq!(len, d.max_encoded_len() - 64);
    Declaration::decode(d.kind(), &buf[..len])?;
    Ok(())
}
