# Feature Integrations

Ergon ships optional integrations behind Cargo feature flags — add exactly what
your hot path needs, keep compilation lean otherwise.

## Quick reference

| Feature | Best for | Cost |
|---------|----------|------|
| `compact_str` | Tickers, symbols, venue codes (≤24 B) | 0 alloc, 46–56% faster than `String` at ≤24 B; converges at 32 B+ |
| `smol_str` | Long-lived DTOs, shared/cached objects | O(1) clone; from_utf8 cost grows with size (5–25 ns) |
| `bytes` | Relay/forwarding, zero-copy pipelines | Competitive at all sizes, best at 256 B+ |
| `chrono` | Typed timestamps on encode/decode | +2–6 ns vs raw `i64` (4–8× slower but negligible vs I/O) |

See [Measured performance](#measured-performance) below for the full benchmark table.

Add features in your `Cargo.toml`. The generator decides what to emit from its
own feature set, so declare them on **both** entries:

```toml
[build-dependencies]
ergo-sbe = { version = "0.1", features = ["compact_str", "chrono"] }

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
// Type paths use ergo_sbe re-exports — no need to add compact_str directly.
pub struct QuoteDomain {
    pub symbol: ergo_sbe::compact_str::CompactString,  // was String
    pub venue: ergo_sbe::compact_str::CompactString,
    pub price: Decimal,
    // …
}
```

### Codec accessors

When `compact_str` is enabled, every text var-data consuming stage gains an
`into_<field>_as_compact_str()` method. The generator decides at generation
time, so enable the feature on `ergo-sbe` in both `[build-dependencies]` and
`[dependencies]` — a build-dependency without it produces no accessor:

```rust,ignore
let stage = dec.fuel_figures()?;
let (symbol, next_stage) = stage.into_symbol_as_compact_str()?;
// symbol: CompactString — no heap allocation for ≤24B symbols
```

### Measured performance

*aarch64-apple-darwin, rustc 1.95.0 — nanoseconds per `from_utf8` + conversion*

| Payload | `String` | `CompactString` | `SmolStr` | `Bytes` | `Vec<u8>` |
|---------|----------|-----------------|-----------|---------|-----------|
| 3 B (ticker) | 5.9 ns | **3.2 ns** (−46%) | 5.4 ns | 8.6 ns | 10.1 ns |
| 8 B (venue) | 5.8 ns | **3.1 ns** (−47%) | 5.4 ns | 8.7 ns | 10.1 ns |
| 24 B (inline limit) | 8.2 ns | **3.6 ns** (−56%) | 5.1 ns | 9.7 ns | 10.5 ns |
| 32 B (over inline) | 10.2 ns | 14.2 ns | 10.0 ns | 10.7 ns | 10.9 ns |
| 128 B | 11.1 ns | 13.9 ns | 14.3 ns | 17.2 ns | 11.1 ns |
| 256 B | 19.1 ns | 19.2 ns | 25.3 ns | **13.6 ns** | 14.1 ns |

**Takeaway:** CompactString is 46–56% faster for symbols ≤24 bytes (no heap).
At larger sizes it converges with String. Bytes wins at 256 B+. SmolStr has
O(1) clone regardless of length; `from_utf8` cost grows with payload size.

Run: `cargo bench -p ergo-sbe-benchmarks --bench var_data_types_bench --all-features`

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
enc.try_updated_at(chrono::DateTime::from_timestamp(1_720_000_000, 0).unwrap().naive_utc())?;
```

Registering `semantic_type("UTCTimestamp")` **once** covers every field in
the schema carrying that `semanticType`, not just `created_at` — see
[One selector, many fields](../recipes/timestamps.md#one-selector-many-fields)
for a worked multi-field example.

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

### Measured conversion cost

*aarch64-apple-darwin, rustc 1.95.0 — ns per operation*

| Operation | Time | vs `raw i64` | Notes |
|-----------|------|-------------|-------|
| `i64_nanos_to_datetime` | **2.8 ns** | 4.1× | Wire → `DateTime<Utc>` (decode) |
| `datetime_to_i64_nanos` | **4.8 ns** | 7.0× | `DateTime<Utc>` → wire (encode) |
| `i64_micros_to_naive` | **5.5 ns** | 8.1× | Wire → NaiveDateTime (decode) |
| `naive_to_i64_micros` | **5.6 ns** | 8.2× | NaiveDateTime → wire (encode) |
| `raw i64` no-op | **0.68 ns** | baseline | Identity pass-through |

**Takeaway:** Conversions add 2–6 ns — negligible next to the I/O cost of a
network frame (~500 ns for 10 GbE) or the allocator cost of a var-data field
(3–19 ns). The type safety is worth it.

Run: `cargo bench -p ergo-sbe-benchmarks --bench chrono_converter_bench --all-features`

## Combining features

All four features are independent — enable any subset:

```toml
[build-dependencies]
ergo-sbe = { version = "0.1", features = ["compact_str", "bytes", "chrono"] }

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
