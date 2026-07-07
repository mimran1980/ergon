# PersistAs trait + blanket Option impl

**Blocked by:** 00
**Blocks:** 03, 07

The escape-hatch trait for custom column mappings. A type implements `PersistAs`
to declare how it maps to a single ClickHouse column.

## What to build

```rust
pub trait PersistAs {
    /// The ClickHouse column type for this Rust type.
    fn column_type() -> ColumnType;

    /// Column name hint — used when this type is a struct field.
    fn column_name(field_name: &str) -> String {
        field_name.to_string()
    }

    /// Encode self into a clickhouse Row at the given column name.
    fn encode_value(&self, row: &mut clickhouse::Row, column_name: &str);
}
```

Blanket impl:
- `impl<T: PersistAs> PersistAs for Option<T>` → `Nullable(T::column_type())`, encodes `None` as null

## Acceptance criteria

- [ ] `PersistAs` trait defined with all three methods and sensible defaults
- [ ] `impl<T: PersistAs> PersistAs for Option<T>` — produces `Nullable(...)` column type, handles `None`/`Some`
- [ ] Unit test: custom type `Price(u64)` implements `PersistAs` → `Decimal(18,8)`, roundtrips correctly
- [ ] Unit test: `Option<Price>` → column type is `Nullable(Decimal(18, 8))`
- [ ] Unit test: `Option<Option<Price>>` — not allowed at type level, but `Option<T>` where `T: PersistAs` compiles and works
- [ ] Doc examples on the trait
