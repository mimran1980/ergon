# Advanced topics

## Multi-schema generation

When your project uses multiple SBE schemas that share type definitions (enums,
sets, composites), ErgoSBE can deduplicate them.

### Setup

Configure a `shared_module` in `GenerationConfig`:

```rust
let mut config = GenerationConfig::low_latency("common_types");
config.shared_module = Some("common_types".into());

let generator = Generator::new(config);
let output = generator.generate_multi(&[
    (&schema_common, "common_types"),
    (&schema_market, "market_data"),
    (&schema_history, "historical_data"),
]);
```

### Behaviour

- The **first schema** in the array is treated as the shared type source. Its
  enums, sets, and composites are emitted with `pub` visibility.
- **Subsequent schemas** emit `pub use super::common_types::*;` instead of
  regenerating those types.
- The `sbe_rt` runtime module is emitted only in the first schema's output.
- Every schema still generates its own decoders, encoders, and `AnyMessage` enum
  (those are per-schema, not shared).

### When to use

Use `generate_multi` when:
- You have a `common-types.xml` shared across multiple market data schemas.
- You are generating separate modules for different message groups (e.g. market
  data, order entry, historical).

## XInclude

ErgoSBE resolves `<xi:include href="..."/>` elements in SBE schemas.

### Resolution order

1. Relative to the parent schema's directory.
2. Relative to the current working directory.
3. Well-known paths in the `simple-binary-encoding` submodule.

### Cycle detection

Cyclic includes are detected and produce a `ParseError::IncludeError`.

### Example

```xml
<messageSchema package="example" id="1" version="0">
    <include href="common-types.xml"/>
    <message name="Car" id="1">
        <!-- messageHeader, groupSizeEncoding available from common-types.xml -->
    </message>
</messageSchema>
```

## Unsafe code

ErgoSBE is **safe by default**. All generated accessors are safe `fn`s that
return `Result`. Unsafe is opt-in.

### `_unchecked` methods

Every scalar field, composite, enum, set, and fixed array accessor has an
accompanying `unsafe fn foo_unchecked()` variant. These skip bounds checks
and panic on invalid data. Use them only when you have verified the buffer
is large enough and the offset is valid.

**Safety precondition**: The caller must ensure the buffer region starting at
`self.pos + offset` contains at least `N` readable bytes (where `N` is the
field size). Calling `_unchecked` with an undersized buffer is UB.

```rust
// Safe, checked:
let price = quote.price()?;                  // Result<i64>

// Unsafe, unchecked — caller must verify buffer:
let price = unsafe { quote.price_unchecked() };  // i64
```

### `raw_` accessors

`raw_` accessors return the wire value **without null-sentinel mapping**. They
are safe functions that still bounds-check (in checked mode). Use them in HFT
hot loops where you handle null sentinels yourself.

```rust
// Safe, but returns None if the field contains the null sentinel:
let price: Option<i64> = quote.optional_price()?;

// Safe, returns the raw wire value regardless:
let price_raw: i64 = quote.raw_optional_price();
```

### `as_str_unchecked`

Var-data `_as_str` accessors have an `unsafe fn _as_str_unchecked` variant
that skips UTF-8 validation:

```rust
// Safe, validates UTF-8:
let s: &str = quote.description_as_str()?;

// Unsafe, no UTF-8 validation — caller must ensure valid UTF-8:
let s: &str = unsafe { quote.description_as_str_unchecked() };
```

## HFT patterns

### Stack-allocate buffers

For fixed-size messages, use `ENCODED_LENGTH` to stack-allocate:

```rust
// Fixed-size: no groups or var-data
let mut buf = [0u8; QuoteEncoder::ENCODED_LENGTH];

// Variable-size: maximum possible size
let mut buf = [0u8; QuoteEncoder::MAX_ENCODED_LENGTH];
```

### Use `raw_` accessors in hot loops

Avoid null-sentinel branches in tight loops:

```rust
// HFT hot loop — skip the Option branch
for entry in orders {
    let id = unsafe { entry.order_id_unchecked() };
    let qty = unsafe { entry.order_qty_unchecked() };
    process_order(id, qty);
}
```

### Use `as_chunks` for fixed-entry groups

Groups without nested groups or var-data expose `as_chunks()`, which returns
a slice of fixed-size byte arrays for zero-copy access:

```rust
let chunks = orders.as_chunks()?;
for chunk in chunks {
    // chunk is &[u8; ORDER_ENTRY_BLOCK_LENGTH]
    // Decode directly from the raw bytes
}
```

### Use `FrameCursor` for feed buffers

When reading from a market-data feed with external framing:

```rust
let cursor = FrameCursor::new(feed_buffer, FramingPolicy::LengthPrefixU32);
for result in cursor {
    if let Ok(frame) = result {
        match frame.message {
            AnyMessage::Quote(quote) => {
                // Process quote — zero allocation
            }
            AnyMessage::Unknown { payload, .. } => {
                // Forward unknown templates unchanged
                forward_to_downstream(payload);
            }
            _ => {}
        }
    }
}
```

### Avoid allocation in error paths

The error types (`DecodeError`, `EncodeError`) are `const`-constructible with
`&'static str` messages — no heap allocation on failure.

## Compile-time constants

Generated code includes pre-computed byte arrays for common header patterns:

```rust
impl QuoteEncoder<'a> {
    // Pre-computed header bytes (blockLength + templateId + schemaId + version)
    pub const HEADER_TEMPLATE: [u8; 8] = [24, 0, 1, 0, 2, 0, 1, 0];
}

impl OrdersEncoder<'a> {
    // Pre-computed group dimension header bytes
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [32, 0, 0, 0];
}
```

These are used internally by `wrap_and_apply_header` to avoid runtime encoding
of the header. They are also available for advanced use cases where you need
to pre-populate header bytes without a full encoder.

## version-aware vs compiled decoding

ErgoSBE **always** uses the wire `actingBlockLength` for tail offset
calculation. This is the correct behaviour for forward/backward compatibility.

```rust
// The tail (groups + var-data) starts at:
//   body_offset + header.block_length()
//
// NOT at body_offset + QuoteDecoder::BLOCK_LENGTH
//
// This matters when a newer schema has a larger block length than the
// compiled value — using the compiled value would read tail data from
// the wrong offset.
```

## Safety summary

| Feature | Safe? | Notes |
|---------|-------|-------|
| `foo()` | Yes | Bounds-checked, null-mapped |
| `raw_foo()` | Yes | Bounds-checked, no null mapping |
| `foo_unchecked()` | **No** | No bounds check, no null mapping |
| `foo_as_str()` | Yes | UTF-8 validated |
| `foo_as_str_unchecked()` | **No** | No UTF-8 validation |
| `Iterator` on groups | Yes | Extent validated at accessor call |
| `as_chunks()` | Yes | Bounds-checked |
| Encoder setters | Yes | Bounds-checked on write |
