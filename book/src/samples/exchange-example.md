# Exchange Example

Multi-schema exchange-shaped sample. **`with_conversion` only** for Decimal —
no `with_domain_type`. `publish = false`.

## Conversion style

```text
  // build.rs — multi-schema via generate_to_dir
  let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
  let config = GenerationConfig::new(module_name)
      .with_conversion(ConversionSelector::named_type("Decimal"));
  ergo_sbe::generate_to_dir("schemas/normalized-app.xml", config, &out)?;
  // lib.rs: #[path = "generated/normalized_app.rs"] mod normalized_app;
```

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
