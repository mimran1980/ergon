# DynamicRecorder: replace panics with Result

**Blocked by:** none
**Severity:** HIGH
**Status: DONE**

## Problem

`DynamicRecorderBuilder::build()` panics on unsupported column types:

```rust
pub fn build(self) -> DynamicRecorder {
    // ...
    panic!("DynamicRecorder: Decimal columns not supported yet");
}
```

Other panics: `Date`, `DateTime`, `DateTime64`, `Array(T)`, `Nullable(T)` all
cause panics at build time. This is a runtime crash path — the user passes a
`ColumnType` enum variant and gets a panic rather than a `Result::Err`.

The builder already returns `DynamicRecorder` (not `Result`). Changing the
return type to `Result<DynamicRecorder, DynamicRecorderError>` is a breaking
API change but necessary.

## Design

Change `build()` signature:

```rust
pub fn build(self) -> Result<DynamicRecorder, DynamicRecorderError>
```

Add error variants for unsupported types:

```rust
pub enum DynamicRecorderError {
    /// Column type not supported by DynamicRecorder.
    UnsupportedColumnType { column_name: String, column_type: ColumnType },
    /// Table name is empty.
    EmptyTableName,
    /// No fields configured.
    NoFields,
}
```

`DynamicRecorder::record()` already returns `Result<&[u8], DynamicRecorderError>`
— no change needed there.

For now, unsupported types stay unsupported (they return an error instead of
panicking). Adding support for Decimal, DateTime, etc. is a separate todo.

## Acceptance criteria

- [x] `DynamicRecorderBuilder::build()` returns `Result<DynamicRecorder, DynamicRecorderError>`
- [x] All existing panics on unsupported types replaced with `Err(...)`
- [x] `DynamicRecorderError` has `Display` + `std::error::Error` impls
- [x] Call sites in `consumer.rs` updated to handle the `Result`
- [x] Existing tests updated and pass
