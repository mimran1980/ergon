# L3 Order Book

Deep nested / ragged L3 order-book sample for **ergo-sbe**. `publish = false`.

## Conversion style: `with_domain_type` only

```rust,no_run
{{#include ../../examples/conversion-config.rs:with_domain_type}}
```
*(From `book/examples/conversion-config.rs`. Full L3 `build.rs`: [l3-book](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/build.rs).)*

**Generated API (concrete):**

```text
enc.try_price(rust_decimal::Decimal::new(100, 0))?;
let p: rust_decimal::Decimal = dec.try_price()?;
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
