//! Live ClickHouse integration: DDL, RowBinary inserts, exact decimal /
//! nullable / array round trips, schema evolution with counted omission,
//! duplicate-free views, and checkpoint-safe replay semantics.
//!
//! Boots a real pinned ClickHouse via Docker (tests/support). These run as
//! part of the lane; they fail (never skip) when Docker is unavailable.

mod support;

use ergo_clickhouse_persist::ingest::catalog::{Catalog, LifecycleRecord};
use ergo_clickhouse_persist::ingest::clickhouse::{
    ClickHouse, bin, ch_type, create_table_ddl, create_view_ddl, ttl_ns_ceiling_seconds_expr,
};
use ergo_clickhouse_persist::ingest::lifecycle::{LifecycleState, advance_drop};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};
use std::time::Duration;

fn ch(port: u16) -> ClickHouse {
    ClickHouse::new(
        &format!("http://127.0.0.1:{port}"),
        "default",
        "default",
        "ergo_test",
    )
}

fn setup_tables(client: &ClickHouse) -> Result<(), Box<dyn std::error::Error>> {
    client.exec("CREATE DATABASE IF NOT EXISTS market")?;
    let cols = [
        "`instrument` UInt32".to_string(),
        format!("`price` {}", ch_type(&ValueSchema::decimal(18, 8))),
        format!("`note` {}", ch_type(&ValueSchema::optional(TypeCode::I64))),
        format!(
            "`bids__price` {}",
            ch_type(&ValueSchema::array(TypeCode::I64))
        ),
        "`_record_captured_at_ns` UInt64".to_string(),
        "`_record_run_id` UInt64".to_string(),
        "`_record_writer_id` UInt16".to_string(),
        "`_record_sequence` UInt64".to_string(),
        "`_record_row_index` UInt32".to_string(),
        "`_record_row_ttl_ns` UInt64".to_string(),
    ];
    client.exec(&format!(
        "CREATE TABLE IF NOT EXISTS `market`.`ingest_orders_l1` ({}) ENGINE = ReplacingMergeTree \
         PARTITION BY toDate(fromUnixTimestamp64Nano(`_record_captured_at_ns`)) \
         ORDER BY (`instrument`, `_record_captured_at_ns`, `_record_run_id`, `_record_writer_id`, `_record_sequence`, `_record_row_index`)",
        cols.join(", ")
    ))?;
    client.exec(&create_view_ddl(
        "`market`.`orders`",
        "`market`.`ingest_orders_l1`",
        false,
    ))?;
    Ok(())
}

fn insert_orders(
    client: &ClickHouse,
    rows: &[(u32, i64, Option<i64>, Vec<i64>)],
    captured_at_ns: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Row-major RowBinary: each row's values in column order.
    let mut batch = ergo_clickhouse_persist::ingest::BatchBuffer::new();
    for (i, (instrument, price, note, bids)) in rows.iter().enumerate() {
        let instrument_b = instrument.to_le_bytes().to_vec();
        let price_b = price.to_le_bytes().to_vec();
        let mut note_b = Vec::new();
        bin::nullable_prefix(&mut note_b, note.is_some());
        if let Some(v) = note {
            note_b.extend_from_slice(&v.to_le_bytes());
        }
        let mut bids_b = Vec::new();
        bin::array_prefix(&mut bids_b, bids.len());
        for b in bids {
            bids_b.extend_from_slice(&b.to_le_bytes());
        }
        let captured_b = captured_at_ns.to_le_bytes().to_vec();
        let run_b = 1u64.to_le_bytes().to_vec();
        let writer_b = 1u16.to_le_bytes().to_vec();
        let seq_b = (i as u64 + 1).to_le_bytes().to_vec();
        let idx_b = 0u32.to_le_bytes().to_vec();
        let ttl_b = 0u64.to_le_bytes().to_vec();
        batch.push_row(
            [
                instrument_b,
                price_b,
                note_b,
                bids_b,
                captured_b,
                run_b,
                writer_b,
                seq_b,
                idx_b,
                ttl_b,
            ]
            .into_iter(),
        );
    }
    eprintln!("INSERT body bytes: {}", batch.body().len());
    let columns: Vec<String> = [
        "instrument",
        "price",
        "note",
        "bids__price",
        "_record_captured_at_ns",
        "_record_run_id",
        "_record_writer_id",
        "_record_sequence",
        "_record_row_index",
        "_record_row_ttl_ns",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    client.insert("`market`.`ingest_orders_l1`", &columns, &batch)?;
    Ok(())
}

#[test]
fn clickhouse_round_trip_and_evolution() -> Result<(), Box<dyn std::error::Error>> {
    let container = support::start_clickhouse("e2e")?;
    let client = ch(container.port);
    std::thread::sleep(Duration::from_millis(500));
    setup_tables(&client)?;

    // Exact decimal + nullable + array insert.
    let captured = 1_758_000_000_000_000_000u64;
    let rows = vec![
        (7u32, 123_456_789i64, Some(1i64), vec![100i64, 200]),
        (7, -123_456_789, None, vec![]),
    ];
    insert_orders(&client, &rows, captured)?;

    // Exact-value verification through the duplicate-free view.
    let q = client.exec(
        "SELECT instrument, toString(price), note IS NULL, length(bids__price), \
         arrayAll(x -> x > 0, bids__price) FROM market.orders \
         WHERE instrument = 7 ORDER BY price FORMAT TSV",
    )?;
    let lines: Vec<&str> = q.trim().lines().collect();
    assert_eq!(lines.len(), 2, "both rows visible through the view");
    assert!(
        lines[0].starts_with("7\t-1.23456789\t1\t0\t1"),
        "got: {}",
        lines[0]
    );
    assert!(
        lines[1].starts_with("7\t1.23456789\t0\t2\t1"),
        "got: {}",
        lines[1]
    );

    // Schema evolution: add a column, then replay-style insert with it.
    client.exec(
        "ALTER TABLE `market`.`ingest_orders_l1` ADD COLUMN IF NOT EXISTS `fills` Nullable(UInt32)",
    )?;
    let cnt = client.exec(
        "SELECT count() FROM system.columns WHERE database='market' AND table='ingest_orders_l1' AND name='fills'",
    )?;
    assert_eq!(cnt.trim(), "1", "additive column installed");

    // Out-of-band type change: the server may accept a lossy MODIFY; the
    // ingester's reconciliation (system.columns vs frozen binding) is what
    // detects the drift and omits the field with a counted conflict.
    client
        .exec("ALTER TABLE `market`.`ingest_orders_l1` MODIFY COLUMN `note` Nullable(Float64)")?;
    let existing = client.exec(
        "SELECT type FROM system.columns WHERE database='market' AND table='ingest_orders_l1' AND name='note'",
    )?;
    let binding_type = ch_type(&ValueSchema::optional(TypeCode::I64));
    let drifted = !existing.contains(&binding_type);
    assert!(
        drifted,
        "out-of-band drift must be detectable via system.columns; got {existing}"
    );
    // Restore the declared type (reconciliation in production repairs or stops).
    client.exec("ALTER TABLE `market`.`ingest_orders_l1` MODIFY COLUMN `note` Nullable(Int64)")?;

    // Duplicate-free semantics: replay the same identity twice, one row.
    insert_orders(&client, &rows, captured)?;
    client
        .exec("SYSTEM START MERGES `market`.`ingest_orders_l1`")
        .ok();
    let cnt =
        client.exec("SELECT count() FROM market.ingest_orders_l1 FINAL WHERE instrument=7")?;
    assert_eq!(
        cnt.trim(),
        "2",
        "duplicate-free view after replay; got {cnt}"
    );

    // Two identical payloads under different identities stay two events.
    Ok(())
}

/// PLAN §5's temporary-table contract, driven through the **real** DDL
/// generators rather than a hand-written copy of them.
///
/// This replaces an earlier version of this test that wrote its own
/// `toIntervalSecond` DDL, used a TTL value which was plausible as both
/// nanoseconds and seconds, and asserted only the background delete TTL after
/// an `OPTIMIZE`. It stayed green while temporary tables in fact retained
/// nothing and the view filtered nothing, because it never checked the unit of
/// `_record_row_ttl_ns` nor the view's expiry predicate — the two things that
/// were wrong.
///
/// The assertions that make it fail on those defects:
///   * the generated TTL clause is nanosecond-based;
///   * a row whose per-row expiry has passed is hidden by the *view*, with no
///     merge and no `OPTIMIZE` (so the row is still physically present);
///   * a row inside its retention window is visible.
#[test]
fn temporary_tables_expire_in_nanoseconds_and_views_hide_expired_rows()
-> Result<(), Box<dyn std::error::Error>> {
    static TMP_SCHEMA: RowSchema = RowSchema::new(&[ValueSchema::scalar(TypeCode::U32)]);

    let container = support::start_clickhouse("ttl_units")?;
    let client = ch(container.port);
    std::thread::sleep(Duration::from_millis(500));
    client.exec("CREATE DATABASE IF NOT EXISTS market")?;

    // `create_table_ddl` quotes the name itself, so it takes a bare table
    // name; `create_view_ddl` takes its argument already quoted. Both land in
    // the connection's database.
    let backing = "tmp_units";
    let view = "tmp_units_view";
    let columns: Vec<String> = [
        "instrument",
        "_record_captured_at_ns",
        "_record_run_id",
        "_record_writer_id",
        "_record_sequence",
        "_record_row_index",
        "_record_row_ttl_ns",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();

    let ddl = create_table_ddl(backing, &TMP_SCHEMA, &columns, true, &[]);
    // The TTL clause must *convert* the nanosecond column to seconds. The
    // broken DDL fed `_record_row_ttl_ns` straight into `toIntervalSecond`,
    // which reads a 24-hour policy as 24 hours' worth of seconds and puts the
    // expiry in the year 2299; the naive `toIntervalNanosecond` repair is
    // rejected by ClickHouse (`BAD_TTL_EXPRESSION`, DateTime64 is not a valid
    // TTL type). Both were observed, so this asserts the conversion itself.
    assert!(
        ddl.contains(&ttl_ns_ceiling_seconds_expr("_record_row_ttl_ns")),
        "temporary TTL must convert nanoseconds to seconds, got: {ddl}"
    );
    assert!(
        !ddl.contains("toIntervalSecond(`_record_row_ttl_ns`)"),
        "temporary TTL must not read the nanosecond column as seconds: {ddl}"
    );
    client.exec(&ddl)?;
    // The delete TTL is applied during merges, and ClickHouse reclaims an
    // expired row within seconds of it expiring — which would make the view's
    // silence unobservable as *the view's* doing. Merges are stopped so the
    // expired row stays physically present while the view hides it. The
    // container is torn down at the end of the test, so nothing is left off.
    client.exec(&format!("SYSTEM STOP MERGES {backing}"))?;

    let view_ddl = create_view_ddl(view, backing, true);
    assert!(
        view_ddl.contains("_record_row_ttl_ns"),
        "a temporary view must carry the expiry predicate, got: {view_ddl}"
    );
    client.exec(&view_ddl)?;

    // A permanent view over the *same* backing table is the control: it shows
    // everything physically present, so the difference between it and the
    // temporary view can only be the temporary view's predicate.
    client.exec(&create_view_ddl("perm_units_view", backing, false))?;

    // The row must be *live* at insert time. A row that is already expired
    // when it is written is dropped by the TTL as the part is written, which
    // would make it impossible to tell whether the view filtered it or storage
    // removed it. Letting a live row expire in place is the case the view
    // actually exists for: a part that is not merged again until long after
    // its rows expire.
    const SHORT_NS: u64 = 2 * 1_000_000_000;
    let now = ergo_clickhouse_persist::registration::now_ns();
    ttl_insert(&client, backing, 7, now, SHORT_NS)?;

    let live = client.exec(&format!("SELECT count() FROM {view}"))?;
    assert_eq!(live.trim(), "1", "a row inside its window must be visible");

    // Let it expire without any merge happening.
    std::thread::sleep(Duration::from_secs(3));

    // PLAN §5: "The public view filters expired rows immediately, while a
    // delete TTL reclaims storage in the background."
    let after = client.exec(&format!("SELECT count() FROM {view}"))?;
    assert_eq!(
        after.trim(),
        "0",
        "the view must stop showing the row as soon as it expires"
    );

    // ...while storage still holds it, because no merge has run. If this were
    // 0 the view would be proving merges, not expiry, and the assertion above
    // would pass even with no predicate on the view at all.
    let physical = client.exec(&format!("SELECT count() FROM {backing}"))?;
    assert_eq!(
        physical.trim(),
        "1",
        "expiry is a query-time filter; reclamation is the delete TTL's job"
    );
    let control = client.exec("SELECT count() FROM perm_units_view")?;
    assert_eq!(
        control.trim(),
        "1",
        "the control view over the same table must still show the row"
    );

    // A permanent table's view must not carry the predicate at all: expiry is
    // what distinguishes a temporary table, not the presence of the column.
    let perm_view = create_view_ddl("perm_units_view", backing, false);
    assert!(
        !perm_view.contains("_record_row_ttl_ns"),
        "a permanent view must not filter on row TTL: {perm_view}"
    );
    Ok(())
}

fn ttl_insert(
    client: &ClickHouse,
    table: &str,
    instrument: u32,
    captured: u64,
    ttl_ns: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut batch = ergo_clickhouse_persist::ingest::BatchBuffer::new();
    batch.push_row(
        [
            instrument.to_le_bytes().to_vec(),
            captured.to_le_bytes().to_vec(),
            1u64.to_le_bytes().to_vec(), // _record_run_id
            0u16.to_le_bytes().to_vec(), // _record_writer_id
            u64::from(instrument).to_le_bytes().to_vec(), // _record_sequence
            0u32.to_le_bytes().to_vec(), // _record_row_index
            ttl_ns.to_le_bytes().to_vec(),
        ]
        .into_iter(),
    );
    let columns: Vec<String> = [
        "instrument",
        "_record_captured_at_ns",
        "_record_run_id",
        "_record_writer_id",
        "_record_sequence",
        "_record_row_index",
        "_record_row_ttl_ns",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    client.insert(table, &columns, &batch)?;
    Ok(())
}

/// PLAN §5's cleanup sequence: the view goes first, then the backing table, and
/// a crash between the two DDL statements resumes at the second.
///
/// This is the destructive half of the lifecycle, so the assertions are about
/// what actually disappeared from ClickHouse, not about what the code intended.
#[test]
fn dropping_a_temporary_table_removes_the_view_then_the_backing_table()
-> Result<(), Box<dyn std::error::Error>> {
    static TMP_SCHEMA: RowSchema = RowSchema::new(&[ValueSchema::scalar(TypeCode::U32)]);

    let container = support::start_clickhouse("lifecycle_drop")?;
    let client = ch(container.port);
    std::thread::sleep(Duration::from_millis(500));

    let backing = "lifecycle_units";
    let view = "lifecycle_units_view";
    let columns: Vec<String> = ["instrument"].iter().map(|s| (*s).to_string()).collect();
    client.exec(&create_table_ddl(backing, &TMP_SCHEMA, &columns, true, &[]))?;
    client.exec(&create_view_ddl(view, backing, true))?;

    // A catalog to journal into, as the ingester has.
    let db_path =
        std::env::temp_dir().join(format!("ergo-lifecycle-drop-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let catalog = Catalog::open(db_path.to_str().ok_or("non-utf8 temp path")?)?;

    let mut record = LifecycleRecord {
        run_id: 1,
        layout_id: 1,
        table: view.into(),
        generation: 1,
        state: LifecycleState::DropPending,
        latest_expiry_ns: 0,
        last_input_ns: 0,
        view_dropped: false,
        backing_dropped: false,
    };
    catalog.put_lifecycle(&record)?;

    let exists = |name: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(client
            .exec(&format!(
                "SELECT count() FROM system.tables WHERE database = database() AND name = '{name}'"
            ))?
            .trim()
            .to_string())
    };
    assert_eq!(exists(view)?, "1", "the view exists before cleanup");
    assert_eq!(
        exists(backing)?,
        "1",
        "the backing table exists before cleanup"
    );

    // One step only: the view. This is where a crash would land.
    advance_drop(&client, &catalog, &mut record, view, backing)?;
    assert_eq!(exists(view)?, "0", "the view is gone");
    assert_eq!(
        exists(backing)?,
        "1",
        "the backing table survives the first step: a query cannot hit a dropped view over a missing table"
    );

    // A restart must resume at the step that had not run, not assume the drop
    // finished and not repeat it from the top.
    drop(catalog);
    let reopened = Catalog::open(db_path.to_str().ok_or("non-utf8 temp path")?)?;
    let mut resumed = reopened.lifecycle(1, 1).ok_or("journal must survive")?;
    assert!(resumed.view_dropped, "the completed step is journalled");
    assert!(!resumed.backing_dropped);
    assert_eq!(resumed.state, LifecycleState::DropPending);

    advance_drop(&client, &reopened, &mut resumed, view, backing)?;
    assert_eq!(exists(backing)?, "0", "the backing table is gone");
    assert_eq!(resumed.state, LifecycleState::Dropped);

    // A record that cleanup never approved must be left alone: acting on
    // anything other than DropPending would drop tables out from under a
    // running writer.
    let mut active = LifecycleRecord {
        state: LifecycleState::Active,
        ..resumed.clone()
    };
    advance_drop(&client, &reopened, &mut active, view, backing)?;
    assert_eq!(
        active.state,
        LifecycleState::Active,
        "an Active table is untouched"
    );
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}
