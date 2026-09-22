//! Generated-DTO persistence tests: scalar projections of domain DTOs
//! round-trip through the prepared recorder.

use ergo_clickhouse_persist::persist::{EncodeError, Persistable, RecordOutcome, RowWriter};
use ergo_clickhouse_persist::protocol::Policy;
use ergo_clickhouse_persist::registration::{RecorderConfig, RecorderSession, TransportConfig};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};
use ergo_market_schema::diagnostics::{
    BookDebugDomain, PipelineDebugDomain, ReceiveMetadataDomain,
};
use ergo_market_schema::market_data::{QuoteDomain, TradeDomain};

fn session() -> RecorderSession {
    RecorderSession::connect(RecorderConfig {
        process: "m".into(),
        instance: "i".into(),
        build: "t".into(),
        max_record_bytes: 1024 * 1024,
        diagnostics_quota_bytes_per_sec: 0,
        permanent: TransportConfig::Memory {
            slots: 128,
            slot_bytes: 4096,
        },
        diagnostics: TransportConfig::Memory {
            slots: 128,
            slot_bytes: 4096,
        },
    })
    .expect("connect")
}

#[test]
fn generated_trade_dto_persistable() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <TradeDomain as Persistable>::schema();
    assert_eq!(schema.len(), 5);
    assert_eq!(schema.columns[0].ty, TypeCode::U32);
    assert!(
        schema.columns[1].nullable,
        "optional price -> nullable storage"
    );
    assert_eq!(schema.columns[1].ty, TypeCode::I64);

    let trade = TradeDomain {
        instrument_id: 7,
        price: Some(123_456_789),
        quantity: Some(100),
        side: ergo_market_schema::market_data::Side::Buy,
        event_time_ns: 1_758_000_000_000_000_000,
        exchange_trade_id: 42,
    };
    // encoded_len == bytes written
    let len = trade.encoded_len()?;
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, schema)?;
    Persistable::encode(&trade, &mut row)?;
    assert_eq!(row.position(), len);
    Ok(())
}

#[test]
fn generated_trade_dto_optional_null_records_absence() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <TradeDomain as Persistable>::schema();
    let trade = TradeDomain {
        instrument_id: 7,
        price: None,
        quantity: None,
        side: ergo_market_schema::market_data::Side::Sell,
        event_time_ns: 5,
        exchange_trade_id: 6,
    };
    let len = trade.encoded_len()?;
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, schema)?;
    Persistable::encode(&trade, &mut row)?;
    assert_eq!(row.position(), len);
    // nullmap: price and quantity absent -> bits set
    let nullmap_len = schema.nullmap_len();
    let bits = buf[0];
    assert_eq!(bits & 0b11, 0b11, "both optional fields absent");
    let _ = nullmap_len;
    Ok(())
}

#[test]
fn diagnostics_dtos_record_through_prepared_tables() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let dbg = s.table::<BookDebugDomain>("book_debug", Policy::Temporary)?;
    dbg.slot().set_enabled(dbg.policy_id(), 1);
    let mut w = s.writer()?;
    let row = BookDebugDomain {
        instrument_id: 7,
        update_id: 123,
        buffered_updates: 3,
        sync_state: 2,
        captured_at_ns: 42,
    };
    let outcome = w.record(&dbg, &row, 42);
    assert_eq!(outcome, RecordOutcome::Published);
    assert_eq!(row.encoded_len()?, 4 + 8 + 4 + 1 + 8);
    Ok(())
}

#[test]
fn receive_metadata_records_as_extras() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = session();
    let meta = s.table::<ReceiveMetadataDomain>("receive_metadata", Policy::Permanent)?;
    meta.slot().set_enabled(meta.policy_id(), 1);
    let mut w = s.writer()?;
    let row = ReceiveMetadataDomain {
        receive_ns: 111,
        strategy_id: 2,
    };
    assert_eq!(w.record(&meta, &row, 7), RecordOutcome::Published);
    let _ = PipelineDebugDomain {
        stage: 1,
        queued: 0,
        dropped: 0,
        captured_at_ns: 0,
    };
    Ok(())
}

#[test]
fn quote_dto_scalar_projection() -> Result<(), Box<dyn std::error::Error>> {
    let schema = <QuoteDomain as Persistable>::schema();
    let q = QuoteDomain {
        instrument_id: 1,
        bid_price: Some(10),
        bid_quantity: Some(20),
        ask_price: Some(30),
        ask_quantity: Some(40),
        event_time_ns: 50,
    };
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, schema)?;
    Persistable::encode(&q, &mut row)?;
    assert_eq!(row.position(), q.encoded_len()?);
    Ok(())
}

/// Keep a static-schema reference for the l2 storage shape used by the
/// ingester projection tests.
static L2_STORAGE_SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U32), // instrument
        ValueSchema::scalar(TypeCode::U64), // update_id
        ValueSchema::array(TypeCode::I64),  // bids__price
        ValueSchema::array(TypeCode::I64),  // bids__quantity
        ValueSchema::array(TypeCode::I64),  // asks__price
        ValueSchema::array(TypeCode::I64),  // asks__quantity
    ],
};

#[test]
fn l2_storage_shape_arrays_align() -> Result<(), Box<dyn std::error::Error>> {
    // Sibling arrays with equal lengths; group counts preserved as arrays.
    let mut buf = [0u8; 256];
    let mut row = RowWriter::new(&mut buf, &L2_STORAGE_SCHEMA)?;
    row.set_raw(0, &7u32.to_le_bytes())?;
    row.set_raw(1, &9u64.to_le_bytes())?;
    let mut le = [0u8; 16];
    le[..8].copy_from_slice(&100i64.to_le_bytes());
    le[8..].copy_from_slice(&200i64.to_le_bytes());
    row.write_array(TypeCode::I64, 8, &le, 2)?; // bids__price
    let mut qty = [0u8; 16];
    qty[..8].copy_from_slice(&1i64.to_le_bytes());
    qty[8..].copy_from_slice(&2i64.to_le_bytes());
    row.write_array(TypeCode::I64, 8, &qty, 2)?; // bids__quantity
    let _ = &mut row.write_array(TypeCode::I64, 8, &[], 0)?; // asks empty
    row.write_array(TypeCode::I64, 8, &[], 0)?; // asks empty
    Ok(())
}

#[test]
fn encode_error_is_typed() {
    let _ = EncodeError::Custom(1);
}
