# ergo-clickhouse-persist

[![crates.io](https://img.shields.io/crates/v/ergo-clickhouse-persist.svg)](https://crates.io/crates/ergo-clickhouse-persist)
[![docs.rs](https://img.shields.io/docsrs/ergo-clickhouse-persist)](https://docs.rs/ergo-clickhouse-persist)
[![license](https://img.shields.io/crates/l/ergo-clickhouse-persist)](../../LICENSE)
![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)

**Debugging persistence: auto-persist annotated Rust structs to ClickHouse with
automatic schema management.**

Part of the [ErgoSBE] ecosystem. Sits on the *consumer* side — never on the hot path.

```text
Producer:  struct -> [ErgoSBE codegen] -> SBE bytes -> queue
Consumer:  SBE bytes -> [ErgoSBE decode] -> DTO -> [Persist trait] -> ClickHouse
```

Also supports **dynamic** tables (`DynamicSchema` / `DynamicRow` V2 +
`SchemaRegistry` / `RowDecoder`) used by samples for books and HA
`feed_latency` rows.

### Canonical HA latency pattern

Shipped reference: `samples/cluster-ha-orderbook` → `LatencyPersistor`:

```text
DynamicSchema announce (once)
  → DynamicRecorder::record (hot path, reuse buffer)
  → SchemaRegistry + RowDecoder
  → PersistSender / ClickhouseSink batch flush
```

Do not invent a second raw-SQL insert path for the same fields — CH values must
come from decoded dynamic rows (sample tests assert this).

Derive crate: [`derive/README.md`](derive/README.md).

---

## Quick start

```sh
cargo add ergo-clickhouse-persist
cargo add ergo-clickhouse-persist-derive --dev   # proc-macro, dev-only
```

Annotate a struct and send it to ClickHouse:

```rust
use ergo_clickhouse_persist::{ClickhouseSink, ClickhouseSinkBuilder, PersistSender};
use ergo_clickhouse_persist_derive::Persist;

#[derive(Persist)]
struct Order {
    price: u64,
    qty: u32,
    side: String,
}

let sink = ClickhouseSinkBuilder::new().build()?;           // defaults to CLICKHOUSE_URL
let sender: PersistSender<Order> = sink.sender("orders").build();

sender.persist(&Order { price: 100, qty: 10, side: "BUY".into() })?;
sender.flush();                                               // send batch
```

Table is auto-created with `ORDER BY (_persist_time)`. On subsequent persists
with a changed struct, the schema migrates automatically (see below).

---

## Annotations reference

| Attribute | On | Effect |
|-----------|----|--------|
| *(none)* | scalar fields | One ClickHouse column per field |
| `#[persist(skip)]` | field | Exclude from schema |
| `#[persist(flatten)]` | nested struct | `parent_child` columns |
| `#[persist(array)]` | `Vec<T>` | `Array(T)` columns |
| `#[persist(json)]` | any field | JSON column |
| `#[persist(name = "...")]` | field | Override column name |
| `#[persist(type = "Decimal(18,2)")]` | field | Override ClickHouse type |
| `#[persist(order_by = "col1, col2")]` | struct | Override `ORDER BY` |

Resolution chain: `#[persist(type = "...")]` -> `<T as PersistAs>::column_type()`
-> primitive default -> `Json` fallback.

---

## Type mapping

| Rust type | ClickHouse type |
|-----------|----------------|
| `i8` / `i16` / `i32` / `i64` | `Int8` / `Int16` / `Int32` / `Int64` |
| `u8` / `u16` / `u32` / `u64` | `UInt8` / `UInt16` / `UInt32` / `UInt64` |
| `f32` / `f64` | `Float32` / `Float64` |
| `bool` | `Bool` |
| `String` / `&str` / `Vec<u8>` | `String` |
| `Option<T>` | `Nullable(T)` |
| `Vec<T>` (with `#[persist(array)]`) | `Array(T)` |
| unknown / generic | `Json` (via `serde`) |

---

## Feature flags

One feature flag per external crate — enable only what you need.

| Feature | Rust type | ClickHouse type |
|---------|-----------|----------------|
| `rust_decimal` | `rust_decimal::Decimal` | `Decimal(18,8)` |
| `chrono` | `NaiveDateTime` | `DateTime64(9)` |
| `chrono` | `DateTime<Utc>` | `DateTime64(9)` |
| `chrono` | `DateTime<FixedOffset>` | `DateTime64(9)` |
| `chrono` | `NaiveDate` | `Date` |
| `chrono` | `TimeDelta` | `Interval` |
| *(always)* | `std::time::Duration` | `Interval` |
| `serde` | `serde_json::Value` / `impl Serialize` | `String` (JSON) |

Note: `std::time::Duration` is always compiled (no feature gate). It is **not** a
Cargo feature — the `"duration"` row in old documentation referred to the same
always-available impl. `chrono::TimeDelta` requires the `chrono` feature.

---

## Dynamic tables (no struct needed)

For runtime-defined schemas where no Rust struct exists:

```rust
use ergo_clickhouse_persist::dynamic::{DynamicRecorderBuilder, DynamicValue};
use ergo_clickhouse_persist::ColumnType;

let mut rec = DynamicRecorderBuilder::new("order_book_snap")
    .field("price", ColumnType::UInt64)
    .field("qty", ColumnType::UInt32)
    .build()
    .unwrap();

// Hot path — positional values, pre-computed layout
let bytes = rec.record(&[
    DynamicValue::UInt64(12345),
    DynamicValue::UInt64(100),
])?;
```

The builder supports static metadata and optional TTL:

```rust
let mut rec = DynamicRecorderBuilder::new("trades")
    .field("price", ColumnType::Float64)
    .field("qty", ColumnType::UInt64)
    .metadata("source", "exchange_a")
    .ttl("_persist_time", "7 DAY")    // auto-expire after 7 days
    .build()
    .unwrap();
```

**Supported column types for dynamic tables:** `Int8`/`Int16`/`Int32`/`Int64`,
`UInt8`/`UInt16`/`UInt32`/`UInt64`, `Float32`/`Float64`, `Bool`, `String`,
`FixedString(N)`, and `Nullable(...)` wrappers of the above.  `Decimal`, `Date`,
`DateTime`, `Array`, and `Json` are **not** supported in the dynamic path.

The consumer side uses `SchemaRegistry` + `RowDecoder` to decode the SBE bytes
into a column-name to SQL-literal map:

```rust
use std::cell::RefCell;
use std::rc::Rc;
use ergo_clickhouse_persist::consumer::{SchemaRegistry, RowDecoder};

let registry = Rc::new(RefCell::new(SchemaRegistry::new()));
let decoder = RowDecoder::new(Rc::clone(&registry));

// Register schema on first sight
registry.borrow_mut().register(&schema_msg)?;

// Decode each row
let decoded: DecodedRow = decoder.decode(&row_msg)?;
```

Metadata keys discovered mid-stream are automatically added to the cached schema
(a side effect on the shared `SchemaRegistry`).  String values are SQL-escaped
and single-quoted.

---

## Sink setup

```rust
use ergo_clickhouse_persist::{ClickhouseSinkBuilder, PersistSender, PersistCompression};

let sink = ClickhouseSinkBuilder::new()
    .url("http://localhost:8123")       // defaults to CLICKHOUSE_URL env var
    .batch_size(1000)                   // default: 1000 rows
    .flush_interval(Duration::from_millis(100))  // default: 100ms
    .compression(PersistCompression::Lz4)   // default: Lz4
    .build()?;

// Per-table sender with optional metadata
let sender: PersistSender<Order> = sink
    .sender("orders")
    .metadata("app", "risk-engine")     // auto-stamped on every row
    .metadata("host", &hostname)
    .build();           // PersistSender<T>, not Result
```

Batching: rows accumulate in memory and flush at `batch_size` or
`flush_interval` (whichever comes first). On `Drop`, remaining rows are
flushed.

### Builder reference

`ClickhouseSinkBuilder` methods and their defaults:

| Method | Default | Description |
|--------|---------|-------------|
| `url()` | `CLICKHOUSE_URL` env var, or `http://localhost:8123` | ClickHouse HTTP URL |
| `user()` | (none) | ClickHouse user |
| `password()` | (none) | ClickHouse password |
| `database()` | `"default"` (or `CLICKHOUSE_DB` env var) | Database name |
| `batch_size()` | `1000` | Max rows per INSERT batch |
| `flush_interval()` | `100 ms` | Max time between flushes |
| `compression()` | `PersistCompression::Lz4` | Wire compression (`Lz4` or `None`) |
| `tls_skip_verify()` | `false` | Skip TLS certificate verification (dev only) |
| `tls_ca_cert()` | (none) | Path to PEM-encoded CA certificate bundle |
| `retry_config()` | `RetryConfig::default()` | Exponential backoff settings |

### RetryConfig

```rust
use std::time::Duration;
use ergo_clickhouse_persist::RetryConfig;

let cfg = RetryConfig {
    initial_backoff: Duration::from_millis(100),
    max_backoff: Duration::from_secs(10),
    max_retries: 5,
};
```

### Dead-letter callback

When retries are exhausted, the batch can be forwarded to a dead-letter callback:

```rust
use ergo_clickhouse_persist::DroppedBatch;

let sender: PersistSender<Order> = sink
    .sender("orders")
    .metadata("app", "risk-engine")
    .dead_letter(Box::new(|batch: DroppedBatch| {
        eprintln!("Dropped {} rows from {}: {}",
            batch.rows.len(), batch.table, batch.error);
    }))
    .build();
```

`DroppedBatch` contains the table name, the raw SQL value rows, and the error
string.

### ClickhouseSink global methods

- `sink.flush()` — flush all pending batches across every active sender.
- `sink.retries_total() -> u64` — total retry attempts across all senders.
- `sink.dropped_rows_total() -> u64` — total dropped rows across all senders.
- `sink.cleanup()` — drop all tables known to the schema cache (dev/test only).

---

## TTL (Time To Live)

Tables can be configured with ClickHouse TTL to auto-expire old data:

```rust
use ergo_clickhouse_persist::TtlConfig;

let schema = TableSchema::with_ttl(
    vec![
        ColumnDef { name: "price".into(), col_type: ColumnType::Float64 },
    ],
    vec!["_persist_time".into()],
    Some(TtlConfig::new("_persist_time", "7 DAY")),
);
```

The `TableSchema::with_ttl()` constructor adds the `_persist_time` column
(if not present) and generates:

```sql
CREATE TABLE IF NOT EXISTS trades (
    price Float64,
    _persist_time DateTime64(9)
) ENGINE = MergeTree() ORDER BY (_persist_time)
  TTL _persist_time + INTERVAL 7 DAY
```

Dynamic tables can set TTL via the builder:

```rust
let rec = DynamicRecorderBuilder::new("snapshots")
    .field("price", ColumnType::Float64)
    .ttl("_persist_time", "24 HOURS")
    .build()
    .unwrap();
```

TTL is metadata for the DDL and is not part of the SBE wire format or the
schema identity hash.

---

## Metrics / observability

The `PersistMetrics` trait lets you wire the persist crate into your metrics
system (prometheus, statsd, log-based counters, etc.):

```rust
use std::time::Duration;
use ergo_clickhouse_persist::metrics::PersistMetrics;

struct MyMetrics;

impl PersistMetrics for MyMetrics {
    fn row_persisted(&self, table: &str) { /* ... */ }
    fn batch_flushed(&self, table: &str, rows: usize, latency: Duration) { /* ... */ }
    fn request_failed(&self, table: &str) { /* ... */ }
    fn row_dropped(&self, table: &str, count: usize) { /* ... */ }
    fn retry_attempted(&self, table: &str, attempt: u32) { /* ... */ }
}
```

Wire it via `ClickhouseSinkBuilder`:

```rust
let sink = ClickhouseSinkBuilder::new()
    .url("http://localhost:8123")
    .build()?;
```

The default is `NoopMetrics` — all methods are empty, zero-cost (inlined by the
compiler).  A custom implementation must be wired at the sink level; currently
the default `NoopMetrics` is hard-coded in `build()` and custom metrics are
received through the metrics field (a future API will expose a builder setter).

---

## Schema migration

On first persist for a table name, DDL is generated. On subsequent persists
with a changed struct:

| Change | Behaviour |
|--------|-----------|
| New field | `ALTER TABLE ADD COLUMN`, old rows get NULL |
| Compatible widen (u32->u64) | MODIFY COLUMN (widening) |
| Incompatible type change | Skip + log warning, column unchanged |
| Removed field | Ignored (columns never dropped) |

Compatible widens: unsigned to signed of greater width, or any strictly wider
type within the same numeric family (signed, unsigned, float).

---

## Running tests

Unit tests need no external services:

```sh
cargo test -p ergo-clickhouse-persist          # unit tests only (includes dynamic/consumer tests)
cargo test -p ergo-clickhouse-persist --lib     # skip integration (and doc-tests)
```

Integration tests need a ClickHouse container:

```sh
# Start ClickHouse:
./persist/tests/run-clickhouse.sh start

# Run integration tests:
DOCKER_TEST=1 cargo test -p ergo-clickhouse-persist --test integration -- --ignored

# Stop ClickHouse:
./persist/tests/run-clickhouse.sh stop
```

---

## License

Apache License, Version 2.0. See [LICENSE](../../LICENSE).

[ErgoSBE]: https://github.com/mimran1980/ErgoSBE
