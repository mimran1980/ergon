# ergo-clickhouse-persist

[![crates.io](https://img.shields.io/crates/v/ergo-clickhouse-persist.svg)](https://crates.io/crates/ergo-clickhouse-persist)
[![docs.rs](https://img.shields.io/docsrs/ergo-clickhouse-persist)](https://docs.rs/ergo-clickhouse-persist)
[![license](https://img.shields.io/crates/l/ergo-clickhouse-persist)](../../LICENSE)
![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)

**Debugging persistence: auto-persist annotated Rust structs to ClickHouse with
automatic schema management.**

Part of the [ErgoSBE] ecosystem. Sits on the *consumer* side — never on the hot path.

```
Producer:  struct → [ErgoSBE codegen] → SBE bytes → queue
Consumer:  SBE bytes → [ErgoSBE decode] → DTO → [Persist trait] → ClickHouse
```

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

Resolution chain: `#[persist(type = "...")]` → `<T as PersistAs>::column_type()`
→ primitive default → `Json` fallback.

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

| Feature | Crate | ClickHouse type |
|---------|-------|----------------|
| `rust_decimal` | `rust_decimal::Decimal` | `Decimal(18,8)` |
| `chrono` | `NaiveDateTime` / `DateTime<Utc>` | `DateTime64(9)` |
| `chrono` | `NaiveDate` | `Date` |
| `duration` | `std::time::Duration` | `Interval` |
| `serde` | `impl Serialize` (blanket) | `String` (JSON) |

One feature flag per external crate — enable only what you need.

---

## Dynamic tables (no struct needed)

For runtime-defined schemas where no Rust struct exists:

```rust
use ergo_clickhouse_persist::dynamic::{DynamicRecorderBuilder, DynamicValue};
use ergo_clickhouse_persist::ColumnType;

let mut rec = DynamicRecorderBuilder::new("order_book_snap")
    .field("price", ColumnType::UInt64)
    .field("qty", ColumnType::UInt32)
    .build();

// Hot path — positional values, pre-computed layout
let bytes = rec.record(&[
    DynamicValue::UInt64(12345),
    DynamicValue::UInt64(100),
])?;
```

The consumer side uses `SchemaRegistry` + `RowDecoder` to decode the SBE bytes
into a column-name → SQL-literal map:

```rust
use ergo_clickhouse_persist::consumer::{SchemaRegistry, RowDecoder};

let registry = Rc::new(RefCell::new(SchemaRegistry::new()));
let decoder = RowDecoder::new(Rc::clone(&registry));

// Register schema on first sight
registry.borrow_mut().register(&schema_msg)?;

// Decode each row
let decoded: DecodedRow = decoder.decode(&row_msg)?;
```

---

## Sink setup

```rust
use ergo_clickhouse_persist::{ClickhouseSinkBuilder, PersistSender};

let sink = ClickhouseSinkBuilder::new()
    .url("http://localhost:8123")   // defaults to CLICKHOUSE_URL env var
    .batch_size(1000)               // default: 1000 rows
    .flush_interval_ms(100)         // default: 100ms
    .build()?;

// Per-table sender with optional metadata
let sender: PersistSender<Order> = sink
    .sender("orders")
    .metadata("app", "risk-engine") // auto-stamped on every row
    .metadata("host", &hostname)
    .build()?;
```

Batching: rows accumulate in memory and flush at `batch_size` or
`flush_interval_ms` (whichever comes first). On `Drop`, remaining rows are
flushed.

---

## Schema migration

On first persist for a table name, DDL is generated. On subsequent persists
with a changed struct:

| Change | Behaviour |
|--------|-----------|
| New field | `ALTER TABLE ADD COLUMN`, old rows get NULL |
| Compatible widen (u32→u64) | Alter column |
| Incompatible type change | Skip + log warning, column unchanged |
| Removed field | Ignored (columns never dropped) |

---

## Running tests

Unit tests need no external services:

```sh
cargo test -p ergo-clickhouse-persist          # unit tests only
cargo test -p ergo-clickhouse-persist --lib     # skip integration
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
