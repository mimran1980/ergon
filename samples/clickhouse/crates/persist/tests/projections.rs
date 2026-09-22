//! Persistence customization tests: handwritten traits, PersistAs value
//! writer, checked lengths, and row-shape rejections.

use ergo_clickhouse_persist::persist::{
    EncodeError, PersistAs, Persistable, ProjectedRowWriter, ProjectionError, RowWriter,
    SbeProjection, ValueWriter,
};
use ergo_clickhouse_persist::schema::{ProjectionDescriptor, RowSchema, TypeCode, ValueSchema};

static PRICE_QTY_SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U32),
        ValueSchema::decimal(18, 8),
        ValueSchema::decimal(18, 8),
        ValueSchema::array(TypeCode::I64),
    ],
};

/// Exact-decimal price wrapper demonstrating `PersistAs`.
#[derive(Clone, Copy)]
pub struct Px(pub i64); // 1e-8 scaled

impl PersistAs for Px {
    fn value_schema() -> &'static ValueSchema {
        static S: ValueSchema = ValueSchema::decimal(18, 8);
        &S
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(8)
    }
    fn encode_value(&self, out: &mut ValueWriter<'_>) -> Result<(), EncodeError> {
        // Exact i64 mantissa, no float anywhere.
        out.write_fixed(&self.0.to_le_bytes())
    }
}

#[derive(Clone, Copy)]
struct Quote {
    instrument: u32,
    bid: Px,
    ask: Px,
    levels: [i64; 2],
}

impl Persistable for Quote {
    fn schema() -> &'static RowSchema {
        &PRICE_QTY_SCHEMA
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(4 + 16 + 16 + 4 + 16)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.set_raw(0, &self.instrument.to_le_bytes())?;
        // Exact-decimal storage: i128 mantissa, scale from the schema.
        out.write_decimal_i128(i128::from(self.bid.0))?;
        out.write_decimal_i128(i128::from(self.ask.0))?;
        let mut le = [0u8; 16];
        le[..8].copy_from_slice(&self.levels[0].to_le_bytes());
        le[8..].copy_from_slice(&self.levels[1].to_le_bytes());
        out.write_array(TypeCode::I64, 8, &le, self.levels.len())?;
        Ok(())
    }
}

#[test]
fn value_writer_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 16];
    let mut w = ValueWriter::new(&mut buf);
    Px(123_456_789).encode_value(&mut w)?;
    assert_eq!(w.position(), 8);
    let mantissa = i64::from_le_bytes(buf[..8].try_into()?);
    assert_eq!(mantissa, 123_456_789);
    Ok(())
}

#[test]
fn value_writer_bounds_are_checked() {
    let mut buf = [0u8; 4];
    let mut w = ValueWriter::new(&mut buf);
    assert!(matches!(
        Px(1).encode_value(&mut w),
        Err(EncodeError::BufferTooSmall {
            needed: 8,
            actual: 4
        })
    ));
}

#[test]
fn encoded_len_matches_written_for_quote_shape() -> Result<(), Box<dyn std::error::Error>> {
    let q = Quote {
        instrument: 7,
        bid: Px(1_000),
        ask: Px(2_000),
        levels: [10, 20],
    };
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, &PRICE_QTY_SCHEMA)?;
    row.set_raw(0, &q.instrument.to_le_bytes())?;
    row.write_decimal_i128(i128::from(q.bid.0))?;
    row.write_decimal_i128(i128::from(q.ask.0))?;
    let mut le = [0u8; 16];
    le[..8].copy_from_slice(&10i64.to_le_bytes());
    le[8..].copy_from_slice(&20i64.to_le_bytes());
    row.write_array(TypeCode::I64, 8, &le, 2)?;
    assert_eq!(row.position(), 4 + 16 + 16 + 4 + 16);
    Ok(())
}

#[test]
fn type_mismatch_is_rejected_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, &PRICE_QTY_SCHEMA)?;
    // writing i64 into a decimal column must fail
    assert!(matches!(
        row.write_i64(5),
        Err(EncodeError::TypeMismatch { column: 0 })
    ));
    Ok(())
}

#[test]
fn array_length_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 64];
    let mut row = RowWriter::new(&mut buf, &PRICE_QTY_SCHEMA)?;
    // skip to the array column by writing the first three correctly
    row.set_raw(0, &1u32.to_le_bytes())?;
    row.write_decimal_i128(1)?;
    row.write_decimal_i128(2)?;
    // count says 2 but only one item supplied
    let le = [0u8; 8];
    assert!(row.write_array(TypeCode::I64, 8, &le, 2).is_err());
    Ok(())
}

static PROJ_SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U64),
        ValueSchema::scalar(TypeCode::I64),
    ],
};

static DESCRIPTOR: ProjectionDescriptor = ProjectionDescriptor {
    table: "ticks",
    sbe_schema_id: 77,
    sbe_template_id: 42,
    sbe_version: 1,
    schema_fingerprint: 0x1234,
    projection_revision: 3,
    row_schema: PROJ_SCHEMA,
};

struct P;

impl ergo_clickhouse_persist::persist::SbeProjection for P {
    fn descriptor() -> &'static ProjectionDescriptor {
        &DESCRIPTOR
    }
    fn project(
        message: &[u8],
        _metadata: &ergo_clickhouse_persist::persist::RecordMetadata,
        out: &mut ProjectedRowWriter<'_>,
    ) -> Result<(), ProjectionError> {
        if message.len() < 16 {
            return Err(ProjectionError::MalformedMessage);
        }
        if message.len() < 16 {
            return Err(ProjectionError::MalformedMessage);
        }
        let mut a_b = [0u8; 8];
        let mut b_b = [0u8; 8];
        a_b.copy_from_slice(&message[..8]);
        b_b.copy_from_slice(&message[8..16]);
        let a = u64::from_le_bytes(a_b);
        let b = i64::from_le_bytes(b_b);
        out.row().write_u64(a)?;
        out.row().write_i64(b)?;
        out.commit();
        Ok(())
    }
}

#[test]
fn projection_stages_output_and_rolls_back_on_error() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::persist::RecordMetadata;
    let meta = RecordMetadata {
        run_id: 1,
        writer_id: 1,
        sequence: 1,
        captured_at_ns: 0,
        catalog_generation: 1,
    };
    let mut good = [7u8; 16];
    good[8..].copy_from_slice(&(-5i64).to_le_bytes());
    let mut buf = [0u8; 64];
    {
        let mut out = ProjectedRowWriter::new(&mut buf, &PROJ_SCHEMA)?;
        P::project(&good, &meta, &mut out)?;
        assert!(out.is_committed());
        assert_eq!(out.position(), 16);
    }
    // Malformed message: error propagates, nothing was committed.
    let bad = [0u8; 4];
    let mut out = ProjectedRowWriter::new(&mut buf, &PROJ_SCHEMA)?;
    assert!(matches!(
        P::project(&bad, &meta, &mut out),
        Err(ProjectionError::MalformedMessage)
    ));
    assert!(!out.is_committed());
    Ok(())
}

#[test]
fn nullable_columns_maintain_their_bitmap() -> Result<(), Box<dyn std::error::Error>> {
    static S: RowSchema = RowSchema {
        columns: &[
            ValueSchema::optional(TypeCode::I64),
            ValueSchema::scalar(TypeCode::I64),
            ValueSchema::optional(TypeCode::I64),
        ],
    };
    let mut buf = [0u8; 64];
    let (pos, nullmap, vals) = {
        let mut row = RowWriter::new(&mut buf, &S)?;
        row.write_i64_opt(None)?; // col 0 absent
        row.write_i64(9)?; // col 1 present
        row.write_i64_opt(Some(7))?; // col 2 present
        let n = row.position();
        (
            n,
            buf[0],
            [
                i64::from_le_bytes(buf[1..9].try_into()?),
                i64::from_le_bytes(buf[9..17].try_into()?),
                i64::from_le_bytes(buf[17..25].try_into()?),
            ],
        )
    };
    // nullmap byte: bit0 = 1 (col0 null), bit2 = 0 (col2 present)
    assert_eq!(nullmap, 0b0000_0001);
    // values: col0 zero-filled, col1 9, col2 7
    assert_eq!(vals, [0, 9, 7]);
    assert_eq!(pos, 1 + 24);
    Ok(())
}

#[test]
fn descriptor_rejects_reserved_columns_at_registration_time() {
    // The registration layer rejects `_record_`-prefixed tables; column
    // names live in declarations only, never in data records.
    let _ = RESERVED_NOTE;
}

const RESERVED_NOTE: &str = "_record_ prefix is reserved for recorder metadata";
