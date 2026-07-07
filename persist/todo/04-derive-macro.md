# #[derive(Persist)] proc-macro

**Blocked by:** 03
**Blocks:** 11

The standalone proc-macro crate `ergo-clickhouse-persist-derive`. Generates
`Persist` impls from annotated struct definitions.

## What to build

```rust
// ergo-clickhouse-persist-derive
#[proc_macro_derive(Persist, attributes(persist))]
pub fn derive_persist(input: TokenStream) -> TokenStream { ... }
```

Generated impl for a simple struct:
```rust
#[derive(Persist)]
struct Order { price: u64, qty: u32 }
```
→
```rust
impl Persist for Order {
    fn table_schema() -> TableSchema {
        TableSchema {
            columns: vec![
                ColumnDef { name: "price".into(), col_type: ColumnType::UInt64 },
                ColumnDef { name: "qty".into(), col_type: ColumnType::UInt32 },
                ColumnDef { name: "_persist_time".into(), col_type: ColumnType::DateTime64(9) },
            ],
            order_by: vec!["_persist_time".into()],
            engine: TableEngine::MergeTree,
        }
    }

    fn encode_row(&self, row: &mut clickhouse::Row) {
        row.push("price", self.price);
        row.push("qty", self.qty);
        row.push("_persist_time", chrono::Utc::now());
    }
}
```

### Annotations

| Annotation | Behaviour |
|---|---|
| `#[persist(name = "t")]` | Override column name |
| `#[persist(flatten)]` | Inline nested struct fields as `parent_child` |
| `#[persist(json)]` | Serialize field as JSON string column |
| `#[persist(array)]` | `Vec<T>` → one `Array(...)` column per field of T |
| `#[persist(type = "Decimal(18,2)")]` | Override ClickHouse type |
| `#[persist(order_by = "...")]` | Struct-level, override ORDER BY |
| `#[persist(skip)]` | Exclude field from schema |

Type resolution: `#[persist(type = "...")]` → `PersistAs` impl on the field's type → default scalar mapping → JSON fallback.

### Special cases

- `Option<T>` → `Nullable(T::column_type())`, encode `None` as null
- `Vec<u8>` → `String` (binary blob as hex/string)
- Unknown types → `Json` column (serde serialization)
- `_persist_time` auto-added to every schema

## Acceptance criteria

- [ ] Standalone proc-macro crate `ergo-clickhouse-persist-derive` compiles
- [ ] `#[derive(Persist)]` on a flat struct with primitives → correct impl
- [ ] `#[persist(name = "custom")]` renames a column
- [ ] `#[persist(flatten)]` on a nested struct → parent_child column names
- [ ] `#[persist(json)]` on a field → Json column type
- [ ] `#[persist(array)]` on `Vec<T>` → one Array column per scalar field of T
- [ ] `#[persist(type = "Decimal(18,2)")]` overrides column type
- [ ] `#[persist(order_by = "price, ts")]` overrides ORDER BY
- [ ] `#[persist(skip)]` excludes a field
- [ ] `Option<T>` fields → Nullable columns
- [ ] `Vec<u8>` field → String column
- [ ] Nested struct with no annotation → Json column (fallback)
- [ ] Compile-pass test: derive on a realistic struct with all annotations
- [ ] Compile-pass test: struct with generics (must produce useful error, not UB)
- [ ] Generated code passes `cargo fmt --check`
