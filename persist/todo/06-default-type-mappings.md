# Default Rust → ClickHouse type mappings

**Status: DONE**

**Blocked by:** 00

Document and test every default scalar type mapping. The derive macro and
dynamic recorder both use these defaults when no `PersistAs` impl or annotation
is present.

## Mappings

| Rust type | ClickHouse type |
|-----------|----------------|
| `i8` | `Int8` |
| `i16` | `Int16` |
| `i32` | `Int32` |
| `i64` | `Int64` |
| `u8` | `UInt8` |
| `u16` | `UInt16` |
| `u32` | `UInt32` |
| `u64` | `UInt64` |
| `f32` | `Float32` |
| `f64` | `Float64` |
| `bool` | `Bool` |
| `String` | `String` |
| `&str` | `String` |
| `Vec<u8>` | `String` |
| `Option<T>` | `Nullable(T)` — via PersistAs blanket impl (todo 01) |
| Any `impl Serialize` (no other match) | `Json` |

The mapping logic lives in a function:
```rust
pub fn default_column_type<T: Reflect>() -> Option<ColumnType> { ... }
```

Use `std::any::TypeId` or a trait-based approach. Given this is used in the
proc-macro, the mapping must work at compile time (type-based dispatch).

## Acceptance criteria

- [x] Every mapping above has a unit test asserting the exact ColumnType
- [x] `i8` through `i64` → correct Int* variants
- [x] `u8` through `u64` → correct UInt* variants
- [x] `f32` → `Float32`, `f64` → `Float64`
- [x] `bool` → `Bool`
- [x] `String` → `String`
- [x] `Vec<u8>` → `String`
- [x] Fallback → `Json` for unknown types
- [x] Test infrastructure: a function `assert_maps_to::<T>(expected: ColumnType)` that works for any `T`
