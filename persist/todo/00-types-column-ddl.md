# ColumnType enum + DDL string generation

**Blocks:** 01, 02, 06

## Status: DONE

The foundational type system for ClickHouse column types. Every other module
depends on this. Must be exhaustive, well-tested, and produce valid ClickHouse
DDL.

## What to build

```rust
pub enum ColumnType {
    // Integers
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    // Floats
    Float32, Float64,
    // Decimal
    Decimal { precision: u8, scale: u8 },
    // String types
    String,
    FixedString(usize),
    // Date/time
    Date,
    DateTime(u8),       // precision: 0-9
    DateTime64(u8),     // precision: 0-9
    // Compound
    Nullable(Box<ColumnType>),
    Array(Box<ColumnType>),
    // Special
    Bool,
    Interval,
    Json,
}
```

`ColumnType` must implement:
- `Display` → ClickHouse DDL string (e.g. `"Decimal(18, 8)"`, `"Nullable(UInt64)"`, `"Array(Int32)"`)
- `From<ColumnType> for String` (delegate to Display)

DDL generation functions:
- `create_table_ddl(table_name, schema) -> String`
- `alter_table_add_column_ddl(table_name, column_name, column_type) -> String`
- `column_definition_ddl(name, column_type) -> String` — e.g. `"price Decimal(18, 8)"`

## Acceptance criteria

- [x] `ColumnType` enum with all variants listed above
- [x] `Display` impl: every variant produces valid ClickHouse DDL
- [x] `create_table_ddl()` generates correct `CREATE TABLE IF NOT EXISTS ... ENGINE = MergeTree() ORDER BY (...)`
- [x] `alter_table_add_column_ddl()` generates correct `ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...`
- [x] Unit test for every variant's DDL string output
- [x] Unit test for Nullable nesting: `Nullable(Nullable(Int32))` produces `Nullable(Int32)` (no double-wrapping)
- [x] Unit test for Array + Nullable interaction
- [x] Unit test for Decimal bounds (precision 1-76, scale 0-precision)
- [x] Unit test for full CREATE TABLE with multiple columns + ORDER BY clause
- [x] No `unwrap()` in DDL generation — use `expect` with a message or `Result`
