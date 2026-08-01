# L3 Order Book

Deep nested / ragged L3 order-book sample for **ergo-sbe**. `publish = false`.

## Conversion style: `with_domain_type` only

```rust,no_run
  // build.rs — one canonical Rust type per field
  let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
  ergo_sbe::generate_to_dir(
      "schemas/l3-book.xml",
      GenerationConfig::new("l3_codec")
          .enable_domain_objects(DomainVarData::Bytes)
          .with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
          .with_domain_type(
              ConversionSelector::semantic_type("UTCTimestamp"),
              "chrono::DateTime<chrono::Utc>",
          ),
      &out,
  )?;
  // lib.rs — build-dep only; no runtime ergo-sbe:
  // #[path = "generated/l3_codec.rs"]
  // mod l3_codec;
```

**Generated API (concrete):**

```text
enc.price(rust_decimal::Decimal::new(100, 0));
let p: rust_decimal::Decimal = dec.price();
let ts: DateTime<Utc> = dec.exchange_timestamp();
```

`with_domain_type` **already enables conversion** for that selector. Do not also
call `with_conversion(Decimal)` here — it would not change the surface.

| Need | Use |
|------|-----|
| Concrete `price() -> Decimal` | **This sample** (`with_domain_type`) |
| Generic `price_as::<T>()` / app adapters | [Exchange Example](exchange-example.md) (`with_conversion`) |
| Side-by-side both styles | [SBE Feature Tour](sbe-feature-tour.md) |

## Layout

| Path | Role |
|------|------|
| `schemas/l3-book.xml` | Nested bids/asks, orders, var-data tails |
| `build.rs` | `generate_to_dir` into `src/generated/` + domain objects / `with_domain_type` (**build-dep only**) |
| `src/lib.rs` | `#[path = "generated/l3_codec.rs"]` + EncodedLength helpers |
| `src/main.rs` | Runnable demos |
| `tests/l3_tests.rs` | Round-trips |

## Run

```sh
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
```
