# l3-book

Deep nested / ragged L3 order-book sample for **ergo-sbe**. `publish = false`.

## Conversion style: `with_domain_type` only

```rust
// build.rs — one canonical Rust type per field
ergo_sbe::generate_to_out_dir(
    "schemas/l3-book.xml",
    GenerationConfig::new("l3_codec")
        .enable_domain_objects()
        .with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
        .with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
        .with_domain_type(
            ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        ),
)?;
// lib.rs — plain include (build-dep only; no runtime ergo-sbe):
// mod l3_codec { include!(concat!(env!("OUT_DIR"), "/l3_codec.rs")); }
```

**Generated API (concrete):**

```rust
enc.price(rust_decimal::Decimal::new(100, 0));
let p: rust_decimal::Decimal = dec.price();
let ts: DateTime<Utc> = dec.exchange_timestamp();
```

`with_domain_type` **already enables conversion** for that selector. Do not also
call `with_conversion(Decimal)` here — it would not change the surface.

| Need | Use |
|------|-----|
| Concrete `price() -> Decimal` | **This sample** (`with_domain_type`) |
| Generic `price_as::<T>()` / app adapters | [`../exchange-example`](../exchange-example) (`with_conversion`) |
| Side-by-side both styles | [`../sbe-feature-tour`](../sbe-feature-tour) |

## Layout

| Path | Role |
|------|------|
| `schemas/l3-book.xml` | Nested bids/asks, orders, var-data tails |
| `build.rs` | `generate_to_out_dir` + domain objects / `with_domain_type` (**build-dep only**) |
| `src/lib.rs` | plain `include!` of `$OUT_DIR` + EncodedLength helpers |
| `src/main.rs` | Runnable demos |
| `tests/l3_tests.rs` | Round-trips |

## Run

```sh
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
```
