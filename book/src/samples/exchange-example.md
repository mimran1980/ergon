# Exchange Example

Multi-schema exchange-shaped sample. **`with_conversion` only** for Decimal —
no `with_domain_type`. `publish = false`.

## Conversion style

```rust,no_run
{{#include ../../examples/conversion-config.rs:with_conversion}}
```
*(From `book/examples/conversion-config.rs`. Full multi-schema `build.rs`: [exchange-example](https://github.com/mimran1980/ergon/blob/main/samples/exchange-example/build.rs).)*

That emits **generic** methods. The app supplies `TryFromSbe` / `TryToSbe`:

```text
// encode: app Decimal → wire
e.price_from(&d)?;

// decode: wire → app Decimal
let d: rust_decimal::Decimal = level.price_as()?;
```

You still have wire accessors (`price_value` / `price_wire`). You do **not**
get a concrete `price() -> rust_decimal::Decimal` — that requires
`with_domain_type(..., "rust_decimal::Decimal")` (see [L3 Order Book](l3-book.md)).

| Sample | Config |
|--------|--------|
| **This crate** | `with_conversion` only |
| [L3 Order Book](l3-book.md) | `with_domain_type` only |
| [SBE Feature Tour](sbe-feature-tour.md) | both (`demo_conversion_only` is conversion-only) |

## Run

```sh
cargo test --manifest-path samples/exchange-example/Cargo.toml
```
