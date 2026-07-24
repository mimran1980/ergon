# L3 Order Book — SBE Sample

Dummy L3 order book demonstrating ergo-sbe's nested repeating groups with
domain-type converters.

## What it shows

- **Nested repeating groups** — `bids` → `orders` and `asks` → `orders`
- **Domain-type converters** — `rust_decimal::Decimal`, `bool`, `chrono::DateTime<Utc>`
- **Concrete accessors** — `e.price()` returns `Decimal`, `dec.is_active()` returns `bool`, `dec.exchange_timestamp()` returns `DateTime<Utc>` (no turbofish)
- **Exact-length buffer sizing** — `L3BookEncodedLength` pre-computes the wire size before allocation
- **Known-size group construction** — `bids(vec.len() as u16, |b| ...)` avoids back-patching

## Schema

`schemas/l3-book.xml` — 52 lines. One message `L3Book` with:

| Field | Wire type | Domain type |
|---|---|---|
| `exchangeTimestamp` | `u64` (semantic: `UTCTimestamp`) | `chrono::DateTime<Utc>` |
| `sequence` | `u64` | `u64` |
| `isActive` | `BooleanType` (enum) | `bool` |
| `bids.price` / `bids.size` | `Decimal` (composite) | `rust_decimal::Decimal` |
| `bids.orders.quantity` | `Decimal` | `rust_decimal::Decimal` |
| `symbol` | `varAsciiEncoding` | `&[u8]` |

## Build config

The `build.rs` configures three domain-type mappings:

```rust
let config = GenerationConfig::new("l3_codec")
    .with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
    .with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
    .with_domain_type(ConversionSelector::semantic_type("UTCTimestamp"), "chrono::DateTime<chrono::Utc>");
```

## Run

```sh
cargo run
cargo test
```
