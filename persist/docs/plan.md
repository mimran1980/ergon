# ergo-clickhouse-persist — design plan

Grilled 2026-07-07. Implement once ErgoSBE todo list is clear.

## Purpose

Debugging persistence crate. Take any annotated struct (or dynamically-defined
table) and persist it to ClickHouse with automatic schema management. Sits on
the *consumer* side — never on the hot path.

```
Producer:  struct → [ErgoSBE codegen] → SBE bytes → queue
Consumer:  SBE bytes → [ErgoSBE decode] → DTO → [Persist trait] → ClickHouse
```

## Crate layout

```
persist/                          # crate: ergo-clickhouse-persist
├── Cargo.toml
├── derive/                       # ergo-clickhouse-persist-derive (proc-macro, standalone)
│   ├── Cargo.toml
│   └── src/lib.rs
├── src/
│   ├── lib.rs                    # pub mod persist, dynamic, types, sink
│   ├── persist.rs                # Persist trait, PersistAs trait, TableSchema, ColumnType
│   ├── dynamic.rs                # DynamicRecorder (producer), SchemaRegistry + RowDecoder (consumer)
│   ├── types.rs                  # ColumnType enum, default rust→ClickHouse type mappings
│   └── sink.rs                   # ClickhouseSink: connection, batching, DDL, INSERT
├── tests/
│   ├── ddl_gen.rs                # unit: DDL string generation from TableSchema
│   ├── type_mapping.rs           # unit: rust → ClickHouse type mapping
│   └── integration.rs            # needs docker: full insert + query roundtrip
└── docs/
    └── plan.md                   # this file
```

Dependency direction:
```
ergo-clickhouse-persist (standalone, no SBE dep)
         ↑
    ergo_sbe (feature = "persist")  ← re-exports, no proc-macro coupling
```

Derive macro is a standalone crate to avoid circular dependencies with
`ergo_sbe`. The `ergo_sbe` feature flag just re-exports the persist traits.

## Storage strategy

Tiered, default to annotation override:

| Annotation | On | Result |
|---|---|---|
| *(default)* | Flat struct | One column per scalar field |
| *(default)* | Unknown nested struct | JSON column |
| `#[persist(flatten)]` | Nested struct | `parent_child` columns |
| `#[persist(array)]` | `Vec<T>` scalar fields | `field Array(T)` columns (positionally correlated) |
| `#[persist(json)]` | Any field | Single JSON column |
| `impl PersistAs` | Type | Full custom DDL + insert |

Resolution chain: `#[persist(type = "...")]` → `PersistAs` impl → scalar default → JSON fallback.

## Derive macro surface

```rust
#[derive(Persist)]
#[persist(name = "custom_table")]       // override default snake_case name (optional)
#[persist(order_by = "price, ts")]      // override ORDER BY (default: _persist_time)
struct Order {
    price: u64,
    qty: u32,
    side: String,

    #[persist(flatten)]
    metadata: OrderMetadata,

    #[persist(array)]
    bids: Vec<PriceLevel>,

    #[persist(json)]
    debug_context: HashMap<String, String>,

    #[persist(type = "Decimal(18,2)")]
    notional: Decimal,
}
```

Table name is passed at persist time (not baked into the trait) so the same
struct can write to multiple tables:

```rust
sink.persist(&order_book, "exchange_book")?;
sink.persist(&order_book, "risk_book")?;
```

## Persist trait

```rust
pub trait Persist {
    fn table_schema() -> TableSchema;              // column definitions + DDL
    fn encode_row(&self, row: &mut clickhouse::Row); // serialise into Row
}
```

Uses the official `clickhouse` crate's `Row` type directly — no wrapper.

`TableSchema` is cached per unique table name on first persist. Subsequent
calls skip DDL generation.

## PersistAs trait (escape hatch)

```rust
pub trait PersistAs {
    fn column_type() -> ColumnType;
    fn column_name(prefix: &str) -> String;
    fn encode_value(&self, row: &mut clickhouse::Row, prefix: &str);
}
```

Out-of-box impls behind feature flags:

| Feature | Type | ClickHouse |
|---------|------|-----------|
| `rust_decimal` | `rust_decimal::Decimal` | `Decimal(18,8)` |
| `chrono` | `NaiveDateTime` / `DateTime<Utc>` | `DateTime64(9)` |
| `chrono` | `NaiveDate` | `Date` |
| `duration` | `std::time::Duration` | `Interval` |
| `serde` | `impl Serialize` (blanket) | `String` (JSON) |

Feature flags: one per external crate (`rust_decimal`, `chrono`, `duration`, `serde`).

## Dynamic tables (runtime, no struct)

Producer side:
```rust
let rec = DynamicRecorder::new("order_book_snap")
    .field("price", UInt64)
    .field("qty", UInt32)
    .build()?;

// Hot path — positional values, pre-computed layout, zero allocation
rec.record(&[12345_u64.into(), 100_u32.into()])?;
```

Registration builds the SBE layout (field_id→group mapping, wire offsets).
Each `record()` is a fixed sequence of writes into a pre-allocated buffer.

### SBE wire format for dynamic rows

One repeating group per value type — no discriminated union needed:

```sbe
<message name="DynamicSchema">
  <!-- schema_id, table_name, columns -->
</message>

<message name="DynamicRow">
  <field name="schemaId" primitiveType="uint32"/>
  <group name="int64Fields">     <!-- (field_id, value) where value is i64 -->
  <group name="uint64Fields">    <!-- (field_id, value) where value is u64 -->
  <group name="stringFields">    <!-- (field_id, value) where value is stringRef -->
  <!-- ... one group per primitive type ... -->
  <data name="symbolTable" type="varDataEncoding"/>  <!-- string interning -->
</message>
```

This works with ErgoSBE today — no IR changes needed. The "variant" is
implicit in which group a field appears in.

## Schema migration

On first persist for a table name, generate DDL. On subsequent persists with a
changed struct shape:

| Change | Behaviour |
|--------|----------|
| New field | `ALTER TABLE ADD COLUMN`, old rows get NULL |
| Compatible type widen (u32→u64) | Alter column |
| Incompatible type change | Silently skip, keep old column |
| Removed field | Ignore (columns never dropped) |
| Type conflict | Keep existing, log warning |

## Connection model

Builder with sensible env-var defaults:

```rust
let sink = ClickhouseSink::builder()
    .url("http://localhost:8123")  // defaults to CLICKHOUSE_URL env var
    .user("default")
    .batch_size(1000)              // default: 1000 rows
    .flush_interval_ms(100)        // default: 100ms
    .build()?;
```

## Table lifecycle

- **Engine:** `MergeTree()`
- **ORDER BY:** Default `_persist_time`, override via `#[persist(order_by = "...")]`
- **`_persist_time`:** `DateTime64(9)` auto-added to every table, visible to queries
- **TTL:** None — user calls `sink.cleanup()` manually to drop empty tables
- **Auto-drop:** Not automatic — only via explicit `cleanup()` call

## Error handling

ClickHouse unreachable → drop data on the floor + log warning. Debugging data
is not worth the complexity of disk buffering or backpressure.

## Test strategy

- **Unit:** DDL string generation, type mapping, `PersistAs` impls
- **Integration:** Docker ClickHouse via shell script, full insert + query roundtrip
- **CI:** `docker run clickhouse/clickhouse-server` before integration tests

## Out of scope

- How the producer discovers the consumer (queue/channel/IPC) — separate design doc
- ErgoSBE changes — the dynamic SBE pattern needs no new IR features
- Row-level TTL
- Column removal
- Multi-node ClickHouse clusters
