# Persist trait definition

**Blocked by:** 01, 02
**Blocks:** 04, 05
**Status: DONE**

The main trait that structs implement to become persistable. Uses the official
`clickhouse` crate's `Row` type for serialization — no wrapper.

## What to build

```rust
pub trait Persist {
    /// Column definitions + DDL metadata.
    fn table_schema() -> TableSchema;

    /// Encode all fields into a clickhouse Row.
    fn encode_row(&self, row: &mut clickhouse::Row);
}
```

`TableSchema` provides the DDL. `encode_row` provides the data. The sink calls
schema once (cached) and `encode_row` once per row.

Must co-exist with the `clickhouse` crate — we vendor its `Row` type, not wrap it.

## Acceptance criteria

- [x] `Persist` trait defined in `persist/src/persist.rs`
- [x] Manual `impl Persist for SomeStruct` compiles and works
- [x] `clickhouse` crate dependency added with `Row` type accessible
- [x] Unit test: manual impl → `table_schema()` returns expected columns
- [x] Unit test: manual impl → `encode_row()` populates `Row` with correct values
- [x] Unit test: encode + decode roundtrip via `clickhouse::Row` (in-memory)
- [x] Verify that `Row` can be passed to `clickhouse::Client::insert()`
