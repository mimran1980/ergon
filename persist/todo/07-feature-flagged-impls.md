# Feature-gated PersistAs impls for external crate types

**Blocked by:** 01

One `PersistAs` impl per external crate, each behind its own feature flag.
Each feature is independently testable.

## Feature flags (in `ergo-clickhouse-persist/Cargo.toml`)

```toml
[features]
rust_decimal = ["dep:rust_decimal"]
chrono = ["dep:chrono"]
duration = []   # std::time::Duration, no external dep needed
serde = ["dep:serde", "dep:serde_json"]
```

## Impls

### `rust_decimal`
- `rust_decimal::Decimal` → `Decimal(18, 8)`

### `chrono`
- `chrono::NaiveDateTime` → `DateTime64(9)`
- `chrono::DateTime<Utc>` → `DateTime64(9)`
- `chrono::DateTime<FixedOffset>` → `DateTime64(9)`
- `chrono::NaiveDate` → `Date`

### `duration`
- `std::time::Duration` → `Interval`
- Encoded as total nanoseconds (Duration is a time span, Interval is the ClickHouse type)

### `serde`
- Blanket: `impl<T: Serialize> PersistAs for T` → `String` (JSON)
- This is the catch-all — any type that derives `Serialize` gets a JSON column
- Must not conflict with other `PersistAs` impls. Consider using `#[persist(json)]` annotation
  as the explicit opt-in instead of a blanket impl, OR use negative trait bounds.
- **Decision:** blanket `impl<T: Serialize> PersistAs for T` with a note that more specific
  impls (like the ones above) take precedence via Rust's coherence rules. If conflicts arise,
  the user uses `#[persist(type = "...")]` to override.

## Acceptance criteria

- [ ] Feature `rust_decimal`: `Decimal` roundtrips through `PersistAs` → correct ColumnType + encoding
- [ ] Feature `chrono`: `NaiveDateTime` → `DateTime64(9)`, `NaiveDate` → `Date`
- [ ] Feature `chrono`: `DateTime<Utc>` → `DateTime64(9)`
- [ ] Feature `duration`: `Duration` → `Interval`
- [ ] Feature `serde`: custom type with `#[derive(Serialize)]` → `String`(Json) column
- [ ] Each feature compiles and tests pass independently (`cargo test --features rust_decimal`, etc.)
- [ ] No feature: none of these deps are compiled
- [ ] `cargo test --all-features` passes
- [ ] Doc example for each feature
