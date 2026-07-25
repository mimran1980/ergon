# l3-book

Deep nested / ragged L3 order-book sample for **ergo-sbe**. `publish = false`.

## Why no bare `with_conversion`?

[`build.rs`](build.rs) uses **`with_domain_type` only**:

```rust
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
.with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
.with_domain_type(ConversionSelector::semantic_type("UTCTimestamp"), "chrono::DateTime<chrono::Utc>")
```

`with_domain_type` **implies** conversion for the same selector (it pushes into
`conversions` for you). This sample has one canonical Rust type per field, so
concrete methods are the right API:

- `price() -> rust_decimal::Decimal` / `price(Decimal)` on encode
- `is_active() -> bool`, timestamp → `DateTime<Utc>`
- DTO fields store those domain types when `enable_domain_objects()` is on

Calling `with_conversion(Decimal)` as well would be **redundant** for the same
selectors — you would still get domain-type methods, not the generic-only
surface.

Use bare **`with_conversion`** when you want pluggable adapters
(`price_as::<T>()` / `price_from(&T)` and **you** implement `TryFromSbe`):

| Sample | Config |
|--------|--------|
| **This crate** | `with_domain_type` (concrete, one type) |
| [`../exchange-example/`](../exchange-example/) | `with_conversion` + app-side rust_decimal adapter |
| [`../sbe-feature-tour/`](../sbe-feature-tour/) | both (see `demo_conversion_only`) |

## Layout

| Path | Role |
|------|------|
| `schemas/l3-book.xml` | Nested bids/asks, orders, var-data tails |
| `build.rs` | Domain objects + domain types (see above) |
| `src/lib.rs` | EncodedLength helpers + encode with concrete decimals |
| `src/main.rs` | Runnable demos |
| `tests/l3_tests.rs` | Round-trips, DTO, sizing |

## Run

```sh
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
```

From monorepo root. Absolute GitHub links for crates.io live in the
[ergo-sbe README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md).
