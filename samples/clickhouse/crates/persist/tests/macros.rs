//! Prepared-callsite macro tests: strict disabled laziness (side-effect
//! counters stay zero), single evaluation when enabled, and the derived
//! `Persistable` path.

use ergo_clickhouse_persist::persist::{Persistable, RecordOutcome, RowWriter};
use ergo_clickhouse_persist::protocol::Policy;
use ergo_clickhouse_persist::registration::{RecorderConfig, RecorderSession, TransportConfig};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};
use ergo_clickhouse_persist_derive::Persistable as DerivePersistable;
use ergo_clickhouse_persist_derive::{persist_dto, persist_event, persist_sbe, persist_table};

persist_table! {
    static BOOK_DEBUG: temporary "book_debug" {
        instrument: u32,
        update_id: u64,
        buffered_updates: u64,
    }
}

persist_table! {
    static PIPELINE_DEBUG: temporary "pipeline_debug" {
        stage: u32,
        queued: u64,
    }
}

/// Derived handwritten DTO.
#[derive(Clone, Copy, DerivePersistable)]
struct BookDebugRow {
    instrument: u32,
    update_id: u64,
    buffered_updates: u64,
}

static RAW_SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U64),
        ValueSchema::scalar(TypeCode::I64),
    ],
};

#[derive(Clone, Copy, DerivePersistable)]
struct ReceiveMetadata {
    receive_ns: u64,
    strategy_id: i64,
}

fn session() -> RecorderSession {
    let config = RecorderConfig {
        process: "market-recorder".into(),
        instance: "lab-0".into(),
        build: "test".into(),
        max_record_bytes: 1024 * 1024,
        diagnostics_quota_bytes_per_sec: 0,
        permanent: TransportConfig::Memory {
            slots: 256,
            slot_bytes: 4096,
        },
        diagnostics: TransportConfig::Memory {
            slots: 256,
            slot_bytes: 4096,
        },
    };
    RecorderSession::connect(config).expect("connect")
}

fn raw_message() -> Vec<u8> {
    // 16 bytes of pseudo raw-SBE bytes with an SBE-looking header
    let mut v = vec![20, 0, 9, 0, 77, 0, 0, 0];
    v.extend_from_slice(&42u64.to_le_bytes());
    v.extend_from_slice(&7i64.to_le_bytes());
    v
}

#[test]
fn disabled_rule_never_evaluates_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let debug = BOOK_DEBUG.prepare(&mut s)?;
    let mut w = s.writer()?;

    let mut expensive_calls = 0u64;
    let expensive = |c: &mut u64| {
        *c += 1;
        99u64
    };

    // Table disabled: the value expression must not run.
    let outcome = persist_event!(
        w,
        debug,
        at = 1,
        instrument = 7u32,
        update_id = 1u64,
        buffered_updates = expensive(&mut expensive_calls),
    );
    assert_eq!(outcome, RecordOutcome::Disabled);
    assert_eq!(
        expensive_calls, 0,
        "disabled event must not evaluate payload expressions"
    );
    assert_eq!(w.counters().published, 0);
    assert_eq!(w.sequence(), 1);
    Ok(())
}

#[test]
fn enabled_rule_evaluates_each_expression_once() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let debug = BOOK_DEBUG.prepare(&mut s)?;
    let mut w = s.writer()?;
    debug.slot().set_enabled(debug.policy_id(), 1);

    let mut calls = 0u64;
    let expensive = |c: &mut u64| {
        *c += 1;
        5u64
    };
    let outcome = persist_event!(
        w,
        debug,
        at = 1,
        instrument = 7u32,
        update_id = 2u64,
        buffered_updates = expensive(&mut calls),
    );
    assert_eq!(outcome, RecordOutcome::Published);
    assert_eq!(
        calls, 1,
        "each expression evaluates exactly once when enabled"
    );
    assert_eq!(w.counters().published, 1);
    Ok(())
}

#[test]
fn disabled_to_enabled_to_disabled_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let debug = BOOK_DEBUG.prepare(&mut s)?;
    let mut w = s.writer()?;

    let mut calls = 0u64;
    let bump = |c: &mut u64| {
        *c += 1;
        1u64
    };

    // disabled
    let o = persist_event!(
        w,
        debug,
        at = 1,
        instrument = 1u32,
        update_id = 1u64,
        buffered_updates = bump(&mut calls)
    );
    assert_eq!(o, RecordOutcome::Disabled);
    // enabled
    debug.slot().set_enabled(debug.policy_id(), 1);
    let o = persist_event!(
        w,
        debug,
        at = 2,
        instrument = 2u32,
        update_id = 2u64,
        buffered_updates = bump(&mut calls)
    );
    assert_eq!(o, RecordOutcome::Published);
    // disabled again: expression must not run
    debug.slot().set_disabled();
    let o = persist_event!(
        w,
        debug,
        at = 3,
        instrument = 3u32,
        update_id = 3u64,
        buffered_updates = bump(&mut calls)
    );
    assert_eq!(o, RecordOutcome::Disabled);
    assert_eq!(calls, 1, "only the enabled window evaluated the expression");
    assert_eq!(w.counters().published, 1);
    Ok(())
}

#[test]
fn persist_sbe_macro_gates_raw_and_extras() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    // Raw-SBE table with typed extras via the session-level API.
    static DESCRIPTOR: ergo_clickhouse_persist::schema::ProjectionDescriptor =
        ergo_clickhouse_persist::schema::ProjectionDescriptor {
            table: "sbe_messages",
            sbe_schema_id: 77,
            sbe_template_id: 9,
            sbe_version: 0,
            schema_fingerprint: 0xfeed,
            projection_revision: 1,
            row_schema: RAW_SCHEMA,
        };
    let raw = s.sbe_table::<ReceiveMetadata>(
        "sbe_messages",
        Policy::Permanent,
        &DESCRIPTOR,
        "receive_metadata",
    )?;
    let mut w = s.writer()?;
    trait ExtrasEnable {
        fn enable_extras(&self);
    }
    impl ExtrasEnable for ergo_clickhouse_persist::recorder::PreparedSbe {
        fn enable_extras(&self) {
            self.extras_slot().set_enabled(1, 1);
        }
    }

    let mut msg_built = 0u64;
    let _msg = || {
        // captured by mutable closure below
        msg_built += 1;
        raw_message()
    };
    let mut msg_built_ref = &mut msg_built;
    let _ = &mut msg_built_ref;

    // Disabled: message construction must not happen.
    let outcome = persist_sbe!(
        w,
        raw,
        at = 5,
        message = {
            msg_built += 1;
            raw_message()
        },
        extra = ReceiveMetadata {
            receive_ns: 1,
            strategy_id: 2
        },
    );
    assert_eq!(outcome, RecordOutcome::Disabled);
    assert_eq!(
        msg_built, 0,
        "disabled sbe event must not build the message"
    );

    // Enabled: raw + extras publish.
    raw.slot().set_enabled(raw.policy_id(), 1);
    raw.enable_extras();
    let outcome = persist_sbe!(
        w,
        raw,
        at = 6,
        message = raw_message(),
        extra = ReceiveMetadata {
            receive_ns: 1,
            strategy_id: 2
        },
    );
    assert_eq!(outcome, RecordOutcome::Published);
    assert_eq!(
        w.counters().published,
        2,
        "raw + extras envelopes published"
    );
    Ok(())
}

#[test]
fn persist_dto_macro_uses_derived_encoder() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let debug = BOOK_DEBUG.prepare(&mut s)?;
    let mut w = s.writer()?;
    debug.slot().set_enabled(debug.policy_id(), 1);

    let row = BookDebugRow {
        instrument: 3,
        update_id: 100,
        buffered_updates: 2,
    };
    let outcome = persist_dto!(w, debug, at = 9, value = row,);
    assert_eq!(outcome, RecordOutcome::Published);
    // encoded_len == bytes written for the derived encoder
    assert_eq!(row.encoded_len()?, 20);
    Ok(())
}

#[test]
fn derived_schema_matches_manual_layout() {
    let schema = <BookDebugRow as Persistable>::schema();
    assert_eq!(schema.len(), 3);
    assert_eq!(schema.columns[0].ty, TypeCode::U32);
    assert_eq!(schema.columns[1].ty, TypeCode::U64);
    assert_eq!(schema.columns[2].ty, TypeCode::U64);
    let _ = PIPELINE_DEBUG::new();
}

/// Every optional scalar width the derive can emit: the declared ClickHouse
/// column width and the derived `encoded_len` must agree, and both must equal
/// the bytes the encoder actually writes. Only `Option<i64>` was covered.
#[test]
fn derived_optional_scalars_encode_at_declared_width() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone, DerivePersistable)]
    struct OptionalWidths {
        a: Option<i8>,
        b: Option<u8>,
        c: Option<i16>,
        d: Option<u16>,
        e: Option<i32>,
        f: Option<u32>,
        g: Option<i64>,
        h: Option<u64>,
    }

    let schema = <OptionalWidths as Persistable>::schema();
    let expected = [
        TypeCode::I8,
        TypeCode::U8,
        TypeCode::I16,
        TypeCode::U16,
        TypeCode::I32,
        TypeCode::U32,
        TypeCode::I64,
        TypeCode::U64,
    ];
    assert_eq!(schema.len(), expected.len());
    for (i, ty) in expected.iter().enumerate() {
        assert_eq!(schema.columns[i].ty, *ty, "column {i} type");
        assert!(schema.columns[i].nullable, "column {i} must be nullable");
        assert_eq!(
            ty.fixed_width(),
            Some(TypeCode::fixed_width(*ty).unwrap()),
            "column {i} has no fixed width"
        );
    }

    let value = OptionalWidths {
        a: Some(-1),
        b: Some(2),
        c: Some(-3),
        d: Some(4),
        e: Some(-5),
        f: Some(6),
        g: Some(-7),
        h: Some(8),
    };
    let declared = value.encoded_len()?;
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, schema)?;
    value.encode(&mut row)?;
    assert_eq!(
        row.position(),
        declared,
        "derived encoded_len disagrees with the bytes written"
    );

    // A row of nulls must encode at the same width.
    let empty = OptionalWidths {
        a: None,
        b: None,
        c: None,
        d: None,
        e: None,
        f: None,
        g: None,
        h: None,
    };
    let declared = empty.encoded_len()?;
    let mut row = RowWriter::new(&mut buf, schema)?;
    empty.encode(&mut row)?;
    assert_eq!(row.position(), declared, "null row width mismatch");
    Ok(())
}
