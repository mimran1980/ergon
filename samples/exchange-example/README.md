# exchange-example

Multi-schema exchange-shaped sample. **`with_conversion` only** for Decimal —
no `with_domain_type`. `publish = false`.

## Conversion style

```rust
// build.rs — multi-schema via generate_to_out_dir
let config = GenerationConfig::new(module_name)
    .with_conversion(ConversionSelector::named_type("Decimal"));
ergo_sbe::generate_to_out_dir("schemas/normalized-app.xml", config)?;
// lib.rs: ergo_sbe::sbe_mod!(pub normalized_app);
```

That emits **generic** methods. The app supplies `TryFromSbe` / `TryToSbe`
(see [`src/decimal.rs`](src/decimal.rs)):

```rust
// encode: app Decimal → wire
e.price_from(&d)?;

// decode: wire → app Decimal
let d: rust_decimal::Decimal = level.price_as()?;
```

You still have wire accessors (`price_value` / `price_wire`). You do **not**
get a concrete `price() -> rust_decimal::Decimal` — that requires
`with_domain_type(..., "rust_decimal::Decimal")` (see [`../l3-book`](../l3-book)).

| Sample | Config |
|--------|--------|
| **This crate** | `with_conversion` only |
| [`../l3-book`](../l3-book) | `with_domain_type` only |
| [`../sbe-feature-tour`](../sbe-feature-tour) | both (`demo_conversion_only` is conversion-only) |

## Run

```sh
cargo test --manifest-path samples/exchange-example/Cargo.toml
```
