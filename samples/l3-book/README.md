# L3 Order Book — SBE Sample

Dummy L3 order book demonstrating ergo-sbe's nested repeating groups with
domain-type converters, exact-length buffer sizing, and DTO round-tripping.

## What it shows

- **Nested repeating groups** — `bids` → `orders` and `asks` → `orders`
- **Domain-type converters** — `rust_decimal::Decimal`, `bool`, `chrono::DateTime<Utc>`
- **Concrete accessors** — `e.price()` returns `Decimal`, `dec.is_active()` returns `bool`, `dec.exchange_timestamp()` returns `DateTime<Utc>` (no turbofish)
- **Exact-length buffer sizing** — the encode buffer is sized to the exact wire length up-front; no oversized `vec![0u8; N]` buffer (see below)
- **DTO round-trip** — decode → `L3BookDomain` (owned, domain-typed) → re-encode → byte-identical wire buffer
- **Known-size group construction** — `bids(vec.len() as u16, |b| ...)` avoids back-patching

## Schema

`schemas/l3-book.xml` — 52 lines. Two messages:

| Message | Orders `orderId` | Notes |
|---|---|---|
| `L3Book` | fixed `u64` | ragged order *counts* per level |
| `L3BookVarData` | var-data `&[u8]` | ragged at two levels (count + var-data length) |

`L3Book` field types:

| Field | Wire type | Domain type |
|---|---|---|
| `exchangeTimestamp` | `u64` (semantic: `UTCTimestamp`) | `chrono::DateTime<Utc>` |
| `sequence` | `u64` | `u64` |
| `isActive` | `BooleanType` (enum) | `bool` |
| `bids.price` / `bids.size` | `Decimal` (composite) | `rust_decimal::Decimal` |
| `bids.orders.quantity` | `Decimal` | `rust_decimal::Decimal` |
| `symbol` | `varAsciiEncoding` | `&[u8]` |

## How encoded-length sizing works

The ergo-sbe generator emits a staged length builder (`{Message}EncodedLength`)
for structurally-dynamic messages (those with groups or var-data). It is a
consuming, type-state chain that computes the **exact** wire length before you
allocate the encode buffer:

```rust
let len = l3_book::book_encoded_length(&bids, &asks, symbol)?;
let mut buf = vec![0u8; len];                 // exact — no oversized buffer
let actual = l3_book::encode_book(&mut buf, &bids, &asks, symbol)?;
assert_eq!(len, actual);
```

`book_encoded_length` drives the staged builder. For ragged groups (entries
with different nested-group counts), it uses the `*_ragged` path: each entry's
fixed block is pre-counted, and the closure describes each entry's *variable*
contribution (`builder.group(dim, block, count)` for nested groups). The
generator guarantees the computed length equals the encoder's written length
(verified by `l3book_staged_length_matches_encoded`).

Three group shapes are generated (see the rustdoc on the `{group}` methods):

- **uniform** — all entries share one shape; length is `count × entry`.
- **ragged** (known count) — entries differ; the closure describes each entry.
- **unknown-size** — count discovered from completed entries.

> **`L3BookVarData`** is ragged at *two* levels (var-data `order_id` of
> differing length per order). The staged builder's nested-ragged support is a
> generator follow-up, so `L3BookVarData` exact sizing uses a direct length
> computation (`vardata_book_encoded_length`) — same outcome (exact buffer),
> verified by `l3book_vardata_direct_length_matches_encoded`.

## How DTO round-tripping works

With `enable_domain_objects()`, ergo-sbe generates an owned DTO
(`L3BookDomain`) whose fields use the configured **domain types**
(`DateTime<Utc>`, `bool`, `Decimal`, `Vec<...>`), not the raw wire types. It
converts both ways:

```rust
// wire bytes -> DTO (domain-typed fields)
let dto = L3BookDomain::from(L3BookDecoder::try_from(&buf[..len])?);
// DTO -> wire bytes
let mut buf2 = vec![0u8; len];
let n = dto.encode(&mut buf2)?;
assert_eq!(&buf[..len], &buf2[..n]);   // byte-identical round-trip
```

The DTO coexists with the zero-copy flyweight decoder — use the decoder for hot
paths, the DTO for application logic where owned, domain-typed values are
convenient.

## Build config

The `build.rs` enables domain objects + three domain-type mappings:

```rust
let config = GenerationConfig::new("l3_codec")
    .enable_domain_objects()
    .with_unchecked_companions()
    .with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
    .with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")
    .with_domain_type(ConversionSelector::semantic_type("UTCTimestamp"), "chrono::DateTime<chrono::Utc>");
```

## Run

```sh
cargo run    # encode -> decode -> DTO round-trip (byte-identical)
cargo test   # 6 tests: converters, empty groups, staged/direct length, ragged orders
```

