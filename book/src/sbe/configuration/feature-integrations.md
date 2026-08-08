# Feature Integrations

Ergon ships optional integrations behind Cargo feature flags — add exactly what
your hot path needs, keep compilation lean otherwise.

## Quick reference

| Feature | Crate | What it gives you |
|---------|-------|-------------------|
| `compact_str` | [`compact_str`](https://crates.io/crates/compact_str) | Inline strings ≤24 bytes; no heap for tickers/symbols |
| `smol_str` | [`smol_str`](https://crates.io/crates/smol_str) | O(1)-clone strings; good for long-lived DTOs |
| `bytes` | [`bytes`](https://crates.io/crates/bytes) | Zero-copy shared slices; relay/forward without copy |
| `chrono` | [`chrono`](https://crates.io/crates/chrono) | `DateTime<Utc>` / `NaiveDateTime` from timestamp fields |

Add features in your `Cargo.toml`:

```toml
[dependencies]
ergo-sbe = { version = "0.1", features = ["compact_str", "chrono"] }
```

## CompactString — inline symbols (DTOs)

> **Primarily for domain DTOs.** The codec-level accessor
> `into_<field>_as_compact_str()` exists but is secondary — the main win is
> replacing `String` with `CompactString` in generated `*Domain` structs.

`compact_str::CompactString` stores up to 24 bytes on the stack. Perfect for
tickers (`"AAPL"`), currency pairs (`"EUR/USD"`), venue codes (`"XNYS"`) —
every string that fits in a CPU register-sized inline buffer skips the
allocator entirely.

### Domain DTOs

```rust,ignore
use ergo_sbe::{DomainVarData, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_domain_objects(DomainVarData::CompactStrings);
```

Generated DTO:

```rust,ignore
pub struct QuoteDomain {
    pub symbol: compact_str::CompactString,  // was String
    pub venue: compact_str::CompactString,
    pub price: Decimal,
    // …
}
```

### Codec accessors

When `compact_str` is enabled, every text var-data consuming stage gains an
`into_<field>_as_compact_str()` method:

```rust,ignore
let stage = dec.fuel_figures()?;
let (symbol, next_stage) = stage.into_symbol_as_compact_str()?;
// symbol: CompactString — no heap allocation for ≤24B symbols
```

### Allocation profile

| Payload | `String` | `CompactString` |
|---------|----------|-----------------|
| 3 B (ticker) | 1 heap alloc | 0 alloc (inline) |
| 8 B (venue) | 1 heap alloc | 0 alloc (inline) |
| 24 B (exact threshold) | 1 heap alloc | 0 alloc (inline) |
| 32 B (description) | 1 heap alloc | 1 heap alloc |

Run the benchmarks: `cargo bench -p ergo-sbe-benchmarks --bench var_data_types_bench --all-features`

## SmolStr — cheap clones (DTOs)

> **Primarily for domain DTOs.** Like `CompactString`, the main use is
> `DomainVarData::SmolStrings` in generated `*Domain` structs.

`smol_str::SmolStr` clones in O(1) regardless of length. Best when DTOs are
long-lived and shared across threads or cached.

```rust,ignore
let config = GenerationConfig::new("msgs")
    .with_domain_objects(DomainVarData::SmolStrings);
```

Codec: `into_<field>_as_smol_str()` — returns `SmolStr`.

## Bytes — zero-copy relay

`bytes::Bytes` is a reference-counted byte buffer. Clone it without copying;
slice it without allocating. Ideal for relay/forwarding pipelines where the
same frame is dispatched to multiple consumers.

### Domain DTOs

```rust,ignore
let config = GenerationConfig::new("relay")
    .with_domain_objects(DomainVarData::BytesCrate);
```

Generated DTO fields are `bytes::Bytes`:

```rust,ignore
let original = bytes::Bytes::copy_from_slice(b"payload");
let clone = original.clone();   // increments refcount, no copy
let sub = original.slice(4..);  // view into the same buffer
```

### Codec accessors

`into_<field>_as_bytes()` returns `bytes::Bytes`:

```rust,ignore
let (payload, next) = stage.into_payload_as_bytes()?;
// payload: bytes::Bytes — share it, slice it, send it across threads
```

## Chrono — typed timestamps

SBE timestamp fields are `i64` on the wire (nanoseconds or microseconds since
the Unix epoch). The `chrono` feature adds converter functions and enables
`with_domain_type` for timestamp semantic types.

### Build-time config

```rust,ignore
use ergo_sbe::{GenerationConfig, ConversionSelector};

let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "chrono::DateTime<chrono::Utc>",
    )
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestampMicros"),
        "chrono::NaiveDateTime",
    );
```

### Generated API

```rust,ignore
// Decode
let created: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
let updated: chrono::NaiveDateTime = dec.try_updated_at()?;

// Encode
enc.try_created_at(chrono::Utc::now())?;
enc.try_updated_at(chrono::NaiveDateTime::from_timestamp_opt(1_720_000_000, 0).unwrap())?;
```

### Direct converters

```rust,ignore
use ergo_sbe::chrono_converters;

// Wire nanos → DateTime
let dt = i64_nanos_to_datetime(1_720_000_000_000_000_000);

// DateTime → wire nanos
let ns = datetime_to_i64_nanos(dt);

// Wire micros → NaiveDateTime
let naive = i64_micros_to_naive(1_720_000_000_000_000);

// Roundtrip is exact
assert_eq!(naive_to_i64_micros(naive), 1_720_000_000_000_000);
```

### Conversion cost

| Operation | Time (approx) | vs raw `i64` |
|-----------|---------------|-------------|
| `i64_nanos_to_datetime` | ~4 ns | ~20× slower |
| `datetime_to_i64_nanos` | ~3 ns | ~15× slower |
| Raw `i64` no-op | ~0.2 ns | baseline |

The conversion adds a few nanoseconds — negligible next to the I/O cost of a
network frame or the allocator cost of a var-data field. Use the converters;
the type safety is worth it.

Run: `cargo bench -p ergo-sbe-benchmarks --bench chrono_converter_bench --all-features`

## Combining features

All four features are independent — enable any subset:

```toml
[dependencies]
ergo-sbe = { version = "0.1", features = ["compact_str", "bytes", "chrono"] }
```

```rust,ignore
let config = GenerationConfig::new("msgs")
    .with_domain_objects(DomainVarData::CompactStrings)
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "chrono::DateTime<chrono::Utc>",
    );
```
