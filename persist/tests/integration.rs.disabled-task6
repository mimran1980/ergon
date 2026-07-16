//! Integration tests against a real Docker ClickHouse instance (todo 11).
//!
//! All tests are `#[ignore]` by default.  Run with:
//!
//! ```sh
//! # Start ClickHouse:
//! ./persist/tests/run-clickhouse.sh start
//! # Run integration tests:
//! DOCKER_TEST=1 cargo test -p ergo-clickhouse-persist --test integration -- --ignored
//! # Stop ClickHouse:
//! ./persist/tests/run-clickhouse.sh stop
//! ```
//!
//! Each test cleans up its own tables.

use std::collections::HashMap;

use clickhouse::Row;
use ergo_clickhouse_persist::consumer::{RowDecoder, SchemaRegistry, column_type_to_tag};
use ergo_clickhouse_persist::dynamic::{DynamicRecorderBuilder, DynamicValue};
use ergo_clickhouse_persist::sbe::DynamicRowDecoder;
use ergo_clickhouse_persist::{ClickhouseSink, ClickhouseSinkBuilder, ColumnType, PersistSender};
use ergo_clickhouse_persist_derive::Persist;
use serde::{Deserialize, Serialize};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Run an async block on a **dedicated OS thread** with its own tokio runtime.
///
/// This is the only safe way to mix `ClickhouseSink` (which owns its own
/// tokio runtime and calls `Runtime::block_on`) with async ClickHouse queries
/// in the same test — the two runtimes live on separate threads so neither
/// observes the other at `block_on` time.
fn run_async<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(f)
    })
    .join()
    .expect("async thread panicked")
}

/// A `clickhouse` client pointing at the Docker instance on port 8123.
fn ch_client() -> clickhouse::Client {
    clickhouse::Client::default()
        .with_url("http://localhost:8123")
        .with_user("default")
        .with_password("test123")
        .with_database("default")
}

/// Build a sink that connects to the Docker instance.
///
/// The sink runs SQL queries on a dedicated background thread with its own
/// tokio runtime, so this is safe to call from any context (including tokio
/// test runtimes).
fn test_sink() -> ClickhouseSink {
    ClickhouseSinkBuilder::new()
        .user("default")
        .password("test123")
        .build()
        .expect("failed to build ClickhouseSink")
}

/// DROP TABLE IF EXISTS.
async fn drop_table(name: &str) {
    let client = ch_client();
    let _ = client
        .query(&format!("DROP TABLE IF EXISTS {name}"))
        .execute()
        .await;
}

/// CREATE TABLE with given columns plus _persist_time.
async fn create_table(table: &str, columns: &[(&str, &str)]) {
    let cols: Vec<String> = columns.iter().map(|(n, t)| format!("{n} {t}")).collect();
    let ddl = format!(
        "CREATE TABLE IF NOT EXISTS {table} (\n    {},\n    _persist_time DateTime64(9)\n) \
         ENGINE = MergeTree() ORDER BY (_persist_time)",
        cols.join(",\n    ")
    );
    ch_client()
        .query(&ddl)
        .execute()
        .await
        .expect("create table failed");
}

/// Insert a row from a DecodedRow (column name → SQL literal).
async fn insert_decoded_row(table: &str, row: &HashMap<String, Option<String>>) {
    let cols: Vec<&str> = row.keys().map(|s| s.as_str()).collect();
    let vals: Vec<&str> = row
        .values()
        .map(|v| v.as_deref().unwrap_or("NULL"))
        .collect();
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        cols.join(", "),
        vals.join(", ")
    );
    ch_client()
        .query(&sql)
        .execute()
        .await
        .expect("insert failed");
}

/// Check whether a table exists in the `default` database.
async fn table_exists(client: &clickhouse::Client, name: &str) -> bool {
    #[derive(Row, Deserialize)]
    struct TableCount {
        #[clickhouse(rename = "cnt")]
        cnt: u64,
    }
    let result: Result<TableCount, _> = client
        .query(&format!(
            "SELECT count() AS cnt FROM system.tables \
             WHERE database = 'default' AND name = '{name}'"
        ))
        .fetch_one()
        .await;
    result.map(|r| r.cnt > 0).unwrap_or(false)
}

// ── DynamicSchema SBE message builder ────────────────────────────────────

/// Build a DynamicSchema SBE message (for SchemaRegistry registration).
fn build_schema_message(
    schema_id: u32,
    table_name: &str,
    fields: &[(u8, &str, ColumnType)],
    metadata: &[(&str, &str)],
) -> Vec<u8> {
    use ergo_clickhouse_persist::sbe::{DynamicSchemaEncoder, dynamic_schema_encoder_state};

    let max_len =
        DynamicSchemaEncoder::<dynamic_schema_encoder_state::NeedsMetadata>::MAX_ENCODED_LENGTH;
    let mut buf = vec![0u8; max_len];
    let mut enc =
        DynamicSchemaEncoder::wrap_and_apply_header(&mut buf, 0).expect("wrap schema header");
    let _ = enc.schema_id(schema_id);

    // Symbol table: metadata keys+values, then column names.
    let mut sym = Vec::new();
    for (k, v) in metadata {
        sym.extend_from_slice(k.as_bytes());
        sym.extend_from_slice(v.as_bytes());
    }
    for (_, name, _) in fields {
        sym.extend_from_slice(name.as_bytes());
    }

    let enc = enc
        .metadata(metadata.len() as u16, |g| {
            for (k, v) in metadata {
                g.add(|e| {
                    let _ = e.key_len(k.len() as u16).val_len(v.len() as u16);
                })
                .expect("add metadata entry");
            }
        })
        .expect("metadata group");

    let enc = enc
        .columns(fields.len() as u16, |g| {
            for (fid, name, ct) in fields {
                let tag = column_type_to_tag(ct).expect("column type to tag");
                g.add(|e| {
                    let _ = e.field_id(*fid).name_len(name.len() as u16).type_tag(tag);
                })
                .expect("add column entry");
            }
        })
        .expect("columns group");

    let enc = enc.table_name(table_name.as_bytes()).expect("table name");
    let enc = enc.symbol_table(&sym).expect("symbol table");
    let len = enc.encoded_length_with_header();
    buf[..len].to_vec()
}

// ── Test 1: Derive + persist roundtrip ──────────────────────────────────────

#[derive(Persist, Serialize, Deserialize, Row, Clone, Debug, PartialEq)]
struct TradeV1 {
    price: u64,
    qty: u32,
}

#[test]
#[ignore]
fn test_derive_persist_roundtrip() {
    run_async(async {
        if std::env::var("DOCKER_TEST").is_err() {
            return;
        }
        let table = "int_test_derive_rnd";
        drop_table(table).await;

        let sink = test_sink();
        let sender: PersistSender<TradeV1> = sink.sender(table).build();

        // Row 1
        let row1 = TradeV1 {
            price: 100,
            qty: 10,
        };
        sender.persist(&row1).unwrap();

        // Row 2
        let row2 = TradeV1 {
            price: 200,
            qty: 20,
        };
        sender.persist(&row2).unwrap();
        sender.flush();

        // Query back
        let client = ch_client();
        let rows: Vec<TradeV1> = client
            .query(&format!(
                "SELECT price, qty FROM {table} ORDER BY _persist_time"
            ))
            .fetch_all()
            .await
            .expect("select failed");

        assert_eq!(rows.len(), 2, "expected 2 rows");
        assert_eq!(rows[0], row1, "first row mismatch");
        assert_eq!(rows[1], row2, "second row mismatch");

        drop_table(table).await;
    })
}

// ── Test 2: Dynamic record + decode + persist roundtrip ────────────────────

#[tokio::test]
#[ignore]
async fn test_dynamic_persist_roundtrip() {
    if std::env::var("DOCKER_TEST").is_err() {
        return;
    }
    let table = "int_test_dynamic_rnd";
    drop_table(table).await;

    // Create table matching the DynamicRecorder's schema.
    create_table(table, &[("price", "UInt64"), ("qty", "UInt32")]).await;

    // Build DynamicRecorder + encode a row.
    let mut rec = DynamicRecorderBuilder::new(table)
        .field("price", ColumnType::UInt64)
        .field("qty", ColumnType::UInt32)
        .build()
        .unwrap();
    let schema_id = rec.schema_id;
    let bytes = rec
        .record(&[DynamicValue::UInt64(150), DynamicValue::UInt64(5)])
        .unwrap()
        .to_vec();

    // Decode SBE bytes via DynamicRowDecoder.
    let row_decoder = DynamicRowDecoder::wrap_and_apply_header(&bytes, 0).expect("wrap row header");
    assert_eq!(row_decoder.schema_id(), schema_id);

    // Register schema so RowDecoder can decode.
    let reg = std::rc::Rc::new(std::cell::RefCell::new(SchemaRegistry::new()));
    {
        let mut reg_mut = reg.borrow_mut();
        let schema_bytes = build_schema_message(
            schema_id,
            table,
            &[
                (0u8, "price", ColumnType::UInt64),
                (1u8, "qty", ColumnType::UInt32),
            ],
            &[],
        );
        let schema_decoder =
            ergo_clickhouse_persist::sbe::DynamicSchemaDecoder::wrap_and_apply_header(
                &schema_bytes,
                0,
            )
            .expect("wrap schema header");
        reg_mut.register(&schema_decoder).expect("register schema");
    }

    // Decode to DecodedRow.
    let decoder = RowDecoder::new(std::rc::Rc::clone(&reg));
    let decoded = decoder.decode(&row_decoder).expect("decode row");

    // Verify decoded fields have correct SQL literals.
    assert_eq!(decoded.get("price").unwrap(), &Some("150".to_string()));
    assert_eq!(decoded.get("qty").unwrap(), &Some("5".to_string()));

    // Insert via raw INSERT SQL from decoded values.
    insert_decoded_row(table, &decoded).await;

    // Query back.
    let client = ch_client();
    let rows: Vec<TradeV1> = client
        .query(&format!(
            "SELECT price, qty FROM {table} ORDER BY _persist_time"
        ))
        .fetch_all()
        .await
        .expect("select failed");

    assert_eq!(rows.len(), 1, "expected 1 row");
    assert_eq!(rows[0], TradeV1 { price: 150, qty: 5 }, "row mismatch");

    drop_table(table).await;
}

// ── Test 3: Schema migration (add column) ──────────────────────────────────

#[derive(Persist, Serialize, Clone, Debug)]
struct MigrationV1 {
    price: u64,
    qty: u32,
}

#[derive(Persist, Serialize, Clone, Debug)]
struct MigrationV2 {
    price: u64,
    qty: u32,
    side: Option<String>,
}

#[test]
#[ignore]
fn test_schema_migration() {
    run_async(async {
        if std::env::var("DOCKER_TEST").is_err() {
            return;
        }
        let table = "int_test_migration";
        drop_table(table).await;

        let sink = test_sink();

        // V1: create table + insert row.
        let sender_v1: PersistSender<MigrationV1> = sink.sender(table).build();
        let row_v1 = MigrationV1 {
            price: 100,
            qty: 10,
        };
        sender_v1.persist(&row_v1).unwrap();
        sender_v1.flush();

        // V2: ALTER TABLE ADD COLUMN side Nullable(String) + insert row.
        let sender_v2: PersistSender<MigrationV2> = sink.sender(table).build();
        let row_v2 = MigrationV2 {
            price: 200,
            qty: 20,
            side: Some("BUY".into()),
        };
        sender_v2.persist(&row_v2).unwrap();
        sender_v2.flush();

        // Query back: first row has NULL side, second row has 'BUY'.
        let client = ch_client();
        #[derive(Row, Deserialize, Debug, PartialEq)]
        struct MigrationRow {
            price: u64,
            qty: u32,
            side: Option<String>,
        }
        let rows: Vec<MigrationRow> = client
            .query(&format!(
                "SELECT price, qty, side FROM {table} ORDER BY _persist_time"
            ))
            .fetch_all()
            .await
            .expect("select failed");

        assert_eq!(rows.len(), 2, "expected 2 rows");
        // V1 row: side should be NULL.
        assert_eq!(rows[0].price, 100);
        assert_eq!(rows[0].qty, 10);
        assert_eq!(rows[0].side, None, "V1 row should have NULL for new column");
        // V2 row: side should be Some("BUY").
        assert_eq!(rows[1].price, 200);
        assert_eq!(rows[1].qty, 20);
        assert_eq!(
            rows[1].side,
            Some("BUY".to_string()),
            "V2 row should have side"
        );

        drop_table(table).await;
    })
}

// ── Test 4: Type conflict (incompatible change skipped) ────────────────────

#[derive(Persist, Serialize, Clone, Debug)]
struct ConflictTypeU32 {
    price: u64,
    qty: u32,
}

#[derive(Persist, Serialize, Clone, Debug)]
struct ConflictTypeString {
    price: u64,
    qty: String,
}

#[test]
#[ignore]
fn test_type_conflict() {
    run_async(async {
        if std::env::var("DOCKER_TEST").is_err() {
            return;
        }
        let table = "int_test_type_conflict";
        drop_table(table).await;

        let sink = test_sink();

        // Insert row with qty as u32 — creates table and inserts.
        let sender_u32: PersistSender<ConflictTypeU32> = sink.sender(table).build();
        let row1 = ConflictTypeU32 {
            price: 100,
            qty: 42,
        };
        sender_u32.persist(&row1).unwrap();
        sender_u32.flush();

        // Now try to persist with qty as String → type conflict logged,
        // row silently dropped by ClickHouse type mismatch.
        let sender_str: PersistSender<ConflictTypeString> = sink.sender(table).build();
        let row2 = ConflictTypeString {
            price: 200,
            qty: "bad".into(),
        };
        // Should not panic — errors are swallowed.
        sender_str.persist(&row2).unwrap();
        sender_str.flush();

        // Query back: only the first row.
        #[derive(Row, Deserialize, Debug, PartialEq)]
        struct ConflictRow {
            price: u64,
            qty: u32,
        }
        let client = ch_client();
        let rows: Vec<ConflictRow> = client
            .query(&format!(
                "SELECT price, qty FROM {table} ORDER BY _persist_time"
            ))
            .fetch_all()
            .await
            .expect("select failed");

        assert_eq!(
            rows.len(),
            1,
            "type conflict should silently drop incompatible rows"
        );
        assert_eq!(rows[0].price, 100);
        assert_eq!(rows[0].qty, 42, "original data should be intact");

        // Verify column type is still UInt32 (not String).
        #[derive(Row, Deserialize, Debug, PartialEq)]
        struct ColTypeRow {
            // ponytail: alias to avoid the `type` keyword conflict
            col_type: String,
        }
        let col_type: ColTypeRow = client
            .query(&format!(
                "SELECT type AS col_type FROM system.columns \
                 WHERE database = 'default' AND table = '{table}' AND name = 'qty'"
            ))
            .fetch_one()
            .await
            .expect("column type query failed");

        assert_eq!(
            col_type.col_type, "UInt32",
            "column type should not have changed"
        );

        drop_table(table).await;
    })
}

// ── Test 5: Multiple table names from same struct ──────────────────────────

#[derive(Persist, Serialize, Deserialize, Row, Clone, Debug, PartialEq)]
struct SharedStruct {
    val: u64,
}

#[test]
#[ignore]
fn test_multiple_tables() {
    run_async(async {
        if std::env::var("DOCKER_TEST").is_err() {
            return;
        }
        let table_a = "int_test_multi_a";
        let table_b = "int_test_multi_b";
        drop_table(table_a).await;
        drop_table(table_b).await;

        let sink = test_sink();

        let sender_a: PersistSender<SharedStruct> = sink.sender(table_a).build();
        let sender_b: PersistSender<SharedStruct> = sink.sender(table_b).build();

        sender_a.persist(&SharedStruct { val: 1 }).unwrap();
        sender_b.persist(&SharedStruct { val: 2 }).unwrap();
        sender_a.flush();
        sender_b.flush();

        // Verify both tables exist and have correct data.
        let client = ch_client();
        let rows_a: Vec<SharedStruct> = client
            .query(&format!("SELECT val FROM {table_a}"))
            .fetch_all()
            .await
            .expect("select table_a failed");
        assert_eq!(rows_a.len(), 1);
        assert_eq!(rows_a[0].val, 1);

        let rows_b: Vec<SharedStruct> = client
            .query(&format!("SELECT val FROM {table_b}"))
            .fetch_all()
            .await
            .expect("select table_b failed");
        assert_eq!(rows_b.len(), 1);
        assert_eq!(rows_b[0].val, 2);

        drop_table(table_a).await;
        drop_table(table_b).await;
    })
}

// ── Test 6: Cleanup drops tables ───────────────────────────────────────────

#[test]
#[ignore]
fn test_cleanup_drops_tables() {
    run_async(async {
        if std::env::var("DOCKER_TEST").is_err() {
            return;
        }
        let table = "int_test_cleanup";
        drop_table(table).await;

        let sink = test_sink();
        let sender: PersistSender<SharedStruct> = sink.sender(table).build();
        sender.persist(&SharedStruct { val: 1 }).unwrap();
        sender.flush();

        // Verify table exists.
        let client = ch_client();
        assert!(
            table_exists(&client, table).await,
            "table should exist before cleanup"
        );

        // Cleanup drops all tables tracked by the schema cache.
        sink.cleanup().expect("cleanup failed");

        // Verify table was dropped.
        assert!(
            !table_exists(&client, table).await,
            "table should be dropped after cleanup"
        );
    })
}

// ── Test 7: Metadata injection ─────────────────────────────────────────────

#[derive(Persist, Serialize, Deserialize, Row, Clone, Debug, PartialEq)]
struct MetaStruct {
    price: u64,
}

#[test]
#[ignore]
fn test_metadata_injection() {
    run_async(async {
        if std::env::var("DOCKER_TEST").is_err() {
            return;
        }
        let table = "int_test_metadata";
        drop_table(table).await;

        let sink = test_sink();
        let sender: PersistSender<MetaStruct> = sink
            .sender(table)
            .metadata("source", "exchange_a")
            .metadata("env", "prod")
            .build();

        sender.persist(&MetaStruct { price: 42 }).unwrap();
        sender.flush();

        // Query back and verify metadata columns.
        let client = ch_client();
        #[derive(Row, Deserialize, Debug, PartialEq)]
        struct MetaRow {
            price: u64,
            source: String,
            env: String,
        }
        let rows: Vec<MetaRow> = client
            .query(&format!(
                "SELECT price, source, env FROM {table} ORDER BY _persist_time"
            ))
            .fetch_all()
            .await
            .expect("select failed");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].price, 42);
        assert_eq!(rows[0].source, "exchange_a");
        assert_eq!(rows[0].env, "prod");

        drop_table(table).await;
    })
}
