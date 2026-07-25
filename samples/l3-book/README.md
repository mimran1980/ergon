# l3-book

Deep nested / ragged L3 order-book sample for **ergo-sbe**. `publish = false`.

## Conversion style: `with_domain_type` only

```rust
// build.rs — one canonical Rust type per field
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
.with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
.with_domain_type(
    ConversionSelector::semantic_type("UTCTimestamp"),
    "chrono::DateTime<chrono::Utc>",
)
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
| `build.rs` | Domain objects + `with_domain_type` |
| `src/lib.rs` | EncodedLength + encode helpers |
| `src/main.rs` | Runnable demos |
| `tests/l3_tests.rs` | Round-trips |

## Run

```sh
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
```
