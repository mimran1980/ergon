//! Round trips through a real ClickHouse server.
//!
//! `CLICKHOUSE_TEST_URL` (default `http://localhost:18123`, user/password
//! `lab`) — `just test` starts that server. Every test fails, never skips,
//! when it is unreachable. Each test works in its own database.

use std::error::Error;
use std::time::Duration;

use persist_server::{ClickHouse, Writer};

#[path = "support/lab.rs"]
mod lab;
#[path = "support/v1.rs"]
mod v1;

use lab::{Lab, clean, test_url};

/// `shapes_v2`: `shapes_v1` plus the fields `extra` and `entries.fee`.
mod v2 {
    #[allow(unsafe_code, warnings, clippy::all, clippy::unwrap_used)]
    mod codec {
        include!(concat!(env!("OUT_DIR"), "/shapes_v2.rs"));
    }

    use codec::{
        Colour, Decimal9, EntriesEntry, ShapesEncoder, ShapesFixedFields, TimestampMs,
        sbe_rt::EncodeError,
    };

    pub const SCHEMA: &str = include_str!("schemas/shapes_v2.xml");
    const NOTE: &[u8] = b"v2";
    pub const LEN: usize = ShapesEncoder::compute_length_with_header(1, NOTE.len());

    pub fn encode(buf: &mut [u8]) -> Result<usize, EncodeError> {
        Ok(ShapesEncoder::wrap_and_apply_header(buf, 0)
            .fixed(&ShapesFixedFields {
                ts: 1_700_000_001_000_000_000,
                i8: 1,
                i16: 2,
                i32: 3,
                i64: 4,
                u8: 5,
                u16: 6,
                u32: 7,
                u64: 8,
                f32: 0.25,
                f64: 0.5,
                opt_i32: None,
                opt_u64: Some(9),
                opt_f64: None,
                colour: Colour::Red,
                code: *b"XYZXYZ",
                price: Decimal9::new(1),
                ts_ms: TimestampMs::new(1_700_000_001_000),
                extra: 42,
            })
            .entries(1, |e| {
                e.add_struct(&EntriesEntry {
                    qty: 1,
                    side: Colour::Red,
                    maybe: 1.5,
                    px: Decimal9::new(2),
                    fee: 0.75,
                })?;
                Ok(())
            })?
            .note(NOTE)?
            .encoded_length_with_header())
    }

    pub fn message() -> Result<[u8; LEN], EncodeError> {
        let mut buf = [0; LEN];
        encode(&mut buf)?;
        Ok(buf)
    }
}

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn every_field_shape_round_trips() -> TestResult {
    let lab = Lab::new("shapes", "tables:\n  shapes: { kind: dynamic }\n")?;
    let mut writer = lab.writer(v1::SCHEMA)?;
    writer.push(&v1::message()?);
    let report = writer.tick();
    clean(&report)?;
    assert_eq!(report.inserted.get("shapes"), Some(&1));

    let types = lab.query("SELECT name, type FROM system.columns WHERE database = 'DB' AND table = 'shapes' ORDER BY position FORMAT TSV")?;
    assert_eq!(
        types,
        [
            "ts\tDateTime64(9, \\'UTC\\')",
            "i8\tInt8",
            "i16\tInt16",
            "i32\tInt32",
            "i64\tInt64",
            "u8\tUInt8",
            "u16\tUInt16",
            "u32\tUInt32",
            "u64\tUInt64",
            "f32\tFloat32",
            "f64\tFloat64",
            "opt_i32\tNullable(Int32)",
            "opt_u64\tNullable(UInt64)",
            "opt_f64\tNullable(Float64)",
            "colour\tLowCardinality(String)",
            "code\tString",
            "price\tDecimal(18, 9)",
            "ts_ms\tDateTime64(3, \\'UTC\\')",
            "entries.qty\tArray(Int64)",
            "entries.side\tArray(LowCardinality(String))",
            "entries.maybe\tArray(Nullable(Float64))",
            "entries.px\tArray(Decimal(18, 9))",
            "note\tString",
            "inserted_at\tDateTime64(3, \\'UTC\\')",
        ]
        .join("\n")
    );
    let row = lab.query("SELECT * EXCEPT inserted_at FROM DB.shapes FORMAT TSV")?;
    assert_eq!(
        row,
        "2023-11-14 22:13:20.123456789\t-8\t-16\t-32\t-64\t8\t16\t32\t18446744073709551615\t1.5\t2.25\t-7\t\\N\t3.5\tGreen\tABC\t12345.678901234\t2023-11-14 22:13:20.123\t[10,-20]\t['Red','Green']\t[NULL,0.5]\t[-0.000000001,100]\thello"
    );
    Ok(())
}

#[test]
fn a_record_made_before_a_schema_change_still_loads() -> TestResult {
    // The archive can still hold v1 records when the ingester restarts with
    // the v2 schema: the fields they do not carry read as their defaults.
    let lab = Lab::new("older", "tables:\n  shapes: { kind: dynamic }\n")?;
    let mut writer = lab.writer(v2::SCHEMA)?;
    writer.push(&v1::message()?);
    let report = writer.tick();
    clean(&report)?;
    assert_eq!(report.inserted.get("shapes"), Some(&1));
    assert_eq!(
        lab.query("SELECT note, price, extra, entries.fee, entries.px FROM DB.shapes FORMAT TSV")?,
        "hello\t12345.678901234\t0\t[0,0]\t[-0.000000001,100]"
    );
    Ok(())
}

#[test]
fn dynamic_table_gains_new_schema_columns() -> TestResult {
    let lab = Lab::new("dynamic", "tables:\n  shapes: { kind: dynamic }\n")?;
    {
        let mut writer = lab.writer(v1::SCHEMA)?;
        writer.push(&v1::message()?);
        clean(&writer.tick())?;
    }
    // The recorder restarts with a schema that has two more fields.
    let mut writer = lab.writer(v2::SCHEMA)?;
    writer.push(&v2::message()?);
    let report = writer.tick();
    clean(&report)?;
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert_eq!(
        report.applied,
        [
            "ALTER TABLE `persist_test_dynamic`.`shapes` ADD COLUMN IF NOT EXISTS `extra` UInt32",
            "ALTER TABLE `persist_test_dynamic`.`shapes` ADD COLUMN IF NOT EXISTS `entries.fee` Array(Float64)",
        ]
    );
    let rows = lab.query(
        "SELECT note, extra, entries.fee, entries.qty FROM DB.shapes ORDER BY ts FORMAT TSV",
    )?;
    // Old rows read new columns as defaults; a new `entries.*` array is padded
    // to its siblings' length, so the nested arrays stay aligned.
    assert_eq!(rows, "hello\t0\t[0,0]\t[10,-20]\nv2\t42\t[0.75]\t[1]");
    Ok(())
}

#[test]
fn static_table_is_never_altered_and_keeps_recording() -> TestResult {
    let lab = Lab::new("static", "tables:\n  shapes: { kind: static }\n")?;
    {
        let mut writer = lab.writer(v1::SCHEMA)?;
        writer.push(&v1::message()?);
        clean(&writer.tick())?;
    }
    let before = lab.query(
        "SELECT name FROM system.columns WHERE database = 'DB' AND table = 'shapes' FORMAT TSV",
    )?;

    let mut writer = lab.writer(v2::SCHEMA)?;
    writer.push(&v2::message()?);
    let report = writer.tick();
    clean(&report)?;
    assert!(
        report.applied.is_empty(),
        "static table altered: {:?}",
        report.applied
    );
    assert_eq!(
        report.problems,
        [
            "shapes: static table is missing column extra; not writing it. Fix: ALTER TABLE `persist_test_static`.`shapes` ADD COLUMN IF NOT EXISTS `extra` UInt32",
            "shapes: static table is missing column entries.fee; not writing it. Fix: ALTER TABLE `persist_test_static`.`shapes` ADD COLUMN IF NOT EXISTS `entries.fee` Array(Float64)",
        ]
    );
    assert_eq!(report.inserted.get("shapes"), Some(&1));
    let after = lab.query(
        "SELECT name FROM system.columns WHERE database = 'DB' AND table = 'shapes' FORMAT TSV",
    )?;
    assert_eq!(before, after);
    assert_eq!(
        lab.query("SELECT note FROM DB.shapes ORDER BY ts FORMAT TSV")?,
        "hello\nv2"
    );

    // Running the suggested SQL is picked up by the next re-check.
    lab.query("ALTER TABLE DB.shapes ADD COLUMN IF NOT EXISTS `extra` UInt32")?;
    writer.push(&v2::message()?);
    let report = writer.tick();
    clean(&report)?;
    assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
    assert_eq!(
        lab.query("SELECT extra FROM DB.shapes WHERE note = 'v2' ORDER BY extra FORMAT TSV")?,
        "0\n42"
    );
    Ok(())
}

#[test]
fn removed_schema_fields_keep_their_columns() -> TestResult {
    for kind in ["static", "dynamic"] {
        let lab = Lab::new(
            &format!("removed_{kind}"),
            &format!("tables:\n  shapes: {{ kind: {kind} }}\n"),
        )?;
        {
            let mut writer = lab.writer(v2::SCHEMA)?;
            writer.push(&v2::message()?);
            clean(&writer.tick())?;
        }
        // The schema loses `extra` and the group field `entries.fee`; the
        // insert then omits `entries.fee` beside the `entries.*` it still sends.
        let mut writer = lab.writer(v1::SCHEMA)?;
        writer.push(&v1::message()?);
        let report = writer.tick();
        clean(&report)?;
        assert!(report.problems.is_empty(), "{kind}: {:?}", report.problems);
        assert!(report.applied.is_empty(), "{kind}: {:?}", report.applied);
        assert_eq!(report.inserted.get("shapes"), Some(&1), "{kind}");
        let rows = lab.query(
            "SELECT note, extra, entries.fee, entries.qty FROM DB.shapes ORDER BY ts FORMAT TSV",
        )?;
        assert_eq!(
            rows, "hello\t0\t[0,0]\t[10,-20]\nv2\t42\t[0.75]\t[1]",
            "{kind}"
        );
    }
    Ok(())
}

#[test]
fn changed_column_type_is_reported_not_altered() -> TestResult {
    for kind in ["static", "dynamic"] {
        let lab = Lab::new(
            &format!("type_{kind}"),
            &format!("tables:\n  shapes: {{ kind: {kind} }}\n"),
        )?;
        {
            let mut writer = lab.writer(v1::SCHEMA)?;
            writer.push(&v1::message()?);
            clean(&writer.tick())?;
        }
        lab.query("ALTER TABLE DB.shapes MODIFY COLUMN i16 Int32")?;
        let mut writer = lab.writer(v1::SCHEMA)?;
        writer.push(&v1::message()?);
        let report = writer.tick();
        clean(&report)?;
        assert_eq!(
            report.problems,
            [format!(
                "shapes: column i16 is Int32 but the schema says Int16; not writing it. Fix: ALTER TABLE `persist_test_type_{kind}`.`shapes` MODIFY COLUMN `i16` Int16"
            )],
            "{kind}"
        );
        assert!(report.applied.is_empty(), "{kind}: {:?}", report.applied);
        assert_eq!(lab.query("SELECT type FROM system.columns WHERE database = 'DB' AND table = 'shapes' AND name = 'i16'")?, "Int32");
        // Every other column is still written; i16 reads its default.
        assert_eq!(
            lab.query("SELECT i16, note FROM DB.shapes ORDER BY i16 FORMAT TSV")?,
            "-16\thello\n0\thello",
            "{kind}"
        );
    }
    Ok(())
}

#[test]
fn table_changed_while_recording_is_rechecked() -> TestResult {
    let lab = Lab::new("changed", "tables:\n  shapes: { kind: static }\n")?;
    let mut writer = lab.writer(v1::SCHEMA)?;
    writer.push(&v1::message()?);
    clean(&writer.tick())?;

    // A column of the static table is dropped under the running recorder:
    // the insert fails, the table is compared again, the ERROR names the fix,
    // and the same record is written without that column.
    lab.query("ALTER TABLE DB.shapes DROP COLUMN i8")?;
    writer.push(&v1::message()?);
    let failed = writer.tick();
    assert!(failed.inserted.is_empty(), "{failed:?}");
    assert_eq!(failed.errors.len(), 1, "{:?}", failed.errors);
    let report = writer.tick();
    clean(&report)?;
    assert_eq!(
        report.problems,
        [
            "shapes: static table is missing column i8; not writing it. Fix: ALTER TABLE `persist_test_changed`.`shapes` ADD COLUMN IF NOT EXISTS `i8` Int8"
        ]
    );
    assert_eq!(report.inserted.get("shapes"), Some(&1));

    // The whole database is dropped: it and the table are created again.
    lab.query("DROP DATABASE DB")?;
    writer.push(&v1::message()?);
    assert!(writer.tick().inserted.is_empty());
    let report = writer.tick();
    clean(&report)?;
    assert_eq!(report.applied.len(), 1, "{:?}", report.applied);
    assert_eq!(report.inserted.get("shapes"), Some(&1));
    assert_eq!(lab.query("SELECT count() FROM DB.shapes")?, "1");
    assert_eq!(writer.queued_bytes(), 0);
    Ok(())
}

#[test]
fn every_listed_table_exists_and_a_bad_edit_keeps_the_config() -> TestResult {
    let lab = Lab::new(
        "listed",
        "tables:\n  shapes: { kind: dynamic, enabled: false }\n",
    )?;
    let mut writer = lab.writer(v1::SCHEMA)?;
    clean(&writer.tick())?;
    // Listed but disabled: the table exists, empty, so queries against it work.
    assert_eq!(lab.query("SELECT count() FROM DB.shapes")?, "0");
    // Records made while it was still enabled are written all the same.
    writer.push(&v1::message()?);
    assert_eq!(writer.tick().inserted.get("shapes"), Some(&1));

    // An invalid edit is rejected and the last good configuration stays.
    lab.write_config("tables:\n  shapes: { kind: sometimes }\n")?;
    let report = writer.tick();
    assert_eq!(
        report.errors,
        [
            "tables.yaml: tables.shapes.kind: unknown variant `sometimes`, expected `static` or `dynamic` at line 2 column 19; keeping the previous configuration"
        ]
    );
    writer.push(&v1::message()?);
    assert_eq!(writer.tick().inserted.get("shapes"), Some(&1));

    // A table removed from tables.yaml: its records are skipped and counted.
    lab.write_config("tables: {}\n")?;
    clean(&writer.tick())?;
    assert!(!writer.push(&v1::message()?));
    assert_eq!(
        writer.tick().errors,
        ["1 records skipped: their table is not in tables.yaml"]
    );
    Ok(())
}

#[test]
fn unreachable_clickhouse_keeps_every_record_queued() -> TestResult {
    let dir = std::env::temp_dir().join(format!("persist-test-down-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("tables.yaml"),
        "tables:\n  shapes: { kind: static }\n",
    )?;
    let ch = ClickHouse::new("http://127.0.0.1:9", "lab", "lab", "nowhere");
    let mut writer = Writer::new(v1::SCHEMA, ch, dir.join("tables.yaml"), Duration::ZERO)?;
    for _ in 0..11 {
        assert!(writer.push(&v1::message()?));
    }
    let report = writer.tick();
    assert!(!report.errors.is_empty());
    assert!(report.inserted.is_empty());
    // Nothing is dropped. The ingester stops replaying past its limit
    // instead, and the rest waits in the archive.
    assert_eq!(writer.queued_bytes(), 11 * (4 + v1::LEN));
    Ok(())
}

#[test]
fn table_that_cannot_be_created_keeps_its_records_queued() -> TestResult {
    let lab = Lab::new("nocreate", "tables:\n  shapes: { kind: dynamic }\n")?;
    // A user that may create the database but not the table in it.
    let user = "persist_test_nocreate";
    lab.query(&format!("DROP USER IF EXISTS {user}"))?;
    lab.query(&format!("CREATE USER {user} IDENTIFIED BY 'x'"))?;
    lab.query(&format!("GRANT CREATE DATABASE ON DB.* TO {user}"))?;
    let ch = ClickHouse::new(&test_url(), user, "x", &lab.ch.database);
    let mut writer = Writer::new(v1::SCHEMA, ch, &lab.config, Duration::ZERO)?;
    writer.push(&v1::message()?);
    // The first tick fails to create the table; the second falls inside the
    // retry back-off. Neither may throw the queued record away.
    assert!(!writer.tick().errors.is_empty());
    writer.tick();
    assert_eq!(writer.queued_bytes(), 4 + v1::LEN);
    lab.query(&format!("DROP USER {user}"))?;
    Ok(())
}

#[test]
fn unsupported_field_shapes_are_rejected_up_front() -> TestResult {
    let composite = v1::SCHEMA.replace(
        r#"<field name="code" id="16" type="Code"/>"#,
        r#"<field name="code" id="16" type="groupSizeEncoding"/>"#,
    );
    let err = persist_server::tables_from_schema(&composite)
        .err()
        .map(|e| e.to_string());
    assert_eq!(
        err.as_deref(),
        Some(
            "schema: Shapes.code: a composite other than a decimal (mantissa + constant exponent) or a timestamp (time + constant unit) is not supported"
        )
    );
    // A decimal whose exponent travels with each value has no one column scale.
    let floating = v1::SCHEMA.replace(
        r#"<type name="exponent" primitiveType="int8" presence="constant">-9</type>"#,
        r#"<type name="exponent" primitiveType="int8"/>"#,
    );
    assert!(persist_server::tables_from_schema(&floating).is_err());
    let big = v1::SCHEMA.replace(r#"byteOrder="littleEndian""#, r#"byteOrder="bigEndian""#);
    let err = persist_server::tables_from_schema(&big)
        .err()
        .map(|e| e.to_string());
    assert_eq!(
        err.as_deref(),
        Some("schema: only littleEndian schemas are supported")
    );
    Ok(())
}

#[test]
fn market_schema_tables() -> TestResult {
    let tables = persist_server::tables_from_schema(include_str!("../../schema/market.xml"))?;
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "trade",
            "quote",
            "book_snapshot",
            "mark_price",
            "funding_rate"
        ]
    );
    let ch = ClickHouse::new("http://unused", "", "", "market");
    let book = tables
        .iter()
        .find(|t| t.name == "book_snapshot")
        .ok_or("book_snapshot")?;
    assert_eq!(
        ch.create_sql(&book.shape()),
        "CREATE TABLE IF NOT EXISTS `market`.`book_snapshot` (\n    `ts_event` DateTime64(9, 'UTC'),\n    `ts_init` DateTime64(9, 'UTC'),\n    `sequence` UInt64,\n    `bids.price` Array(Decimal(18, 9)),\n    `bids.size` Array(Decimal(18, 9)),\n    `asks.price` Array(Decimal(18, 9)),\n    `asks.size` Array(Decimal(18, 9)),\n    `symbol` String,\n    `venue` String,\n    inserted_at DateTime64(3, 'UTC') DEFAULT now64(3)\n)\nENGINE = MergeTree\nPARTITION BY toDate(`ts_event`)\nORDER BY (`symbol`, `venue`, `ts_event`)"
    );
    Ok(())
}
