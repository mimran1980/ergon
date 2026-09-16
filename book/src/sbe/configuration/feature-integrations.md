# Feature Integrations

Optional Cargo features. Enable them on **both** `[build-dependencies]` and
`[dependencies]` — the generator decides what to emit from its own feature set.

| Feature | Best for |
|---------|----------|
| `compact_str` | Tickers / symbols / venue codes in DTOs (≤24 B inline) |
| `smol_str` | Long-lived DTOs that clone often |
| `bytes` | Relay / forwarding, shared buffers |
| `chrono` | Typed timestamps on encode/decode |

```toml
[build-dependencies]
ergo-sbe = { version = "0.1", features = ["compact_str", "chrono"] }

[dependencies]
ergo-sbe = { version = "0.1", features = ["compact_str", "chrono"] }
```

Measure with `var_data_types_bench` / `chrono_converter_bench` rather than
quoting numbers from this page. DTO materialisation is never the hot path.

## CompactString

Primarily for domain DTOs. Replaces `String` with
`compact_str::CompactString` (≤24 bytes on the stack). The codec also gains
`into_<field>_as_compact_str()` on text var-data stages.

```rust,ignore
use ergo_sbe::{DomainVarData, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_domain_objects(DomainVarData::CompactStrings);
```

```rust,ignore
pub struct QuoteDomain {
    pub symbol: ergo_sbe::compact_str::CompactString,
    pub venue: ergo_sbe::compact_str::CompactString,
    pub price: Decimal,
}
```

```rust,ignore
let (symbol, _next_stage) = L3BookDecoder::try_decode(wire, 0)?
    .skip_bids()?
    .skip_asks()?
    .into_symbol_as_compact_str()?;
```

## SmolStr

O(1) clone regardless of length. `DomainVarData::SmolStrings` plus
`into_<field>_as_smol_str()`.

```rust,ignore
let config = GenerationConfig::new("msgs")
    .with_domain_objects(DomainVarData::SmolStrings);
```

## Bytes

Reference-counted buffer. `DomainVarData::BytesCrate` plus
`into_<field>_as_bytes()`.

```rust,ignore
let config = GenerationConfig::new("relay")
    .with_domain_objects(DomainVarData::BytesCrate);
```

```rust,ignore
let original = bytes::Bytes::copy_from_slice(b"payload");
let clone = original.clone();
let sub = original.slice(4..);
```

```rust,ignore
let (payload, next) = stage.into_payload_as_bytes()?;
```

## Chrono

Schema fields are typically `uint64` with a timestamp `semanticType`. The
converters take `i64`. Map them with `with_domain_type`:

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

```rust,ignore
let created: chrono::DateTime<chrono::Utc> = dec.try_created_at()?;
let updated: chrono::NaiveDateTime = dec.try_updated_at()?;
enc.try_created_at(chrono::Utc::now())?;
enc.try_updated_at(chrono::DateTime::from_timestamp(1_720_000_000, 0).unwrap().naive_utc())?;
```

One `semantic_type("UTCTimestamp")` covers every field with that attribute.
Worked example: [Timestamps](../recipes/timestamps.md#one-selector-many-fields).

Direct converters (`chrono` feature):

```rust,ignore
use ergo_sbe::chrono_converters::*;

let dt = i64_nanos_to_datetime(1_720_000_000_000_000_000);
let ns = datetime_to_i64_nanos(dt);
let naive = i64_micros_to_naive(1_720_000_000_000_000);
assert_eq!(naive_to_i64_micros(naive), 1_720_000_000_000_000);
```

## Combining

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
