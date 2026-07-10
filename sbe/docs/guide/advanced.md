# Advanced topics

## Multi-schema generation

When your project uses multiple SBE schemas that share type definitions (enums,
sets, composites), ErgoSBE can deduplicate them.

### Setup

Configure a `shared_module` in `GenerationConfig`:

```rust
let mut config = GenerationConfig::new("common_types");
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

## Trusted-input fast path

ErgoSBE is safe by default. Checked constructors or verification APIs establish
message structure at a trust boundary. For feeds that validate framing and
message extents outside the hot loop, the `bound-check-disabled` build mode may
route the same public field accessors through unchecked internals.

This is a whole-path contract, not a family of per-field escape hatches. Broad
`foo_unchecked()` variants are intentionally not part of the generated public
surface. The caller must satisfy the trusted-input preconditions documented by
the selected constructor/mode; otherwise unchecked internal reads may be
undefined behaviour.

### `raw_` accessors

`raw_` accessors are generated for optional fields, version-gated fields, and
fixed-size arrays. They return the wire value **without null-sentinel
mapping** but are safe functions that still bounds-check (in checked mode).

```rust
// Returns None if the field contains the null sentinel:
let price: Option<i64> = quote.pegged_price();

// Returns the raw wire value regardless, None only if absent by version:
let price_raw: Option<i64> = quote.raw_pegged_price();
```

For version-gated fields, `raw_` distinguishes between "field absent
by version" (returns `None`) and "field present but null sentinel" (returns
`Some(sentinel_value)`).

Unchecked UTF-8 helpers are also excluded from the default generated surface.
Use the checked string accessor, or perform an explicitly local unsafe
conversion in application code when profiling proves it necessary.

## Buffer verification

Every message decoder provides a static `verify()` method that validates the
entire buffer structure before decoding:

```rust
// Single-pass validation: header, block length, group dims, var-data bounds
QuoteDecoder::verify(&buf)?;

// Now safe to decode -- structural issues already ruled out
let quote = QuoteDecoder::wrap_and_apply_header(buf, 0)?;
```

`verify()` returns `Result<(), sbe_rt::VerifyError>` with specific variants
for each structural issue.

## HFT patterns

### Stack-allocate buffers

For fixed-size messages, use `ENCODED_LENGTH` to stack-allocate:

```rust
// Fixed-size: no groups or var-data
let mut buf = [0u8; QuoteEncoder::ENCODED_LENGTH];

// Variable-size: maximum possible size
let mut buf = [0u8; QuoteEncoder::MAX_ENCODED_LENGTH];
```

### Select trusted-input mode outside the hot loop

Validate framing and message extents at the boundary, then use the unchanged
generated accessor names in the hot loop. Do not mix checked and per-field
unchecked method families.

### Process ordered tails through concrete stages

For a message containing `bids` followed by `asks`, the decoder exposes
`asks()` only after `bids` has been completed or explicitly skipped. Starting a
group consumes the previous stage, and an active entry prevents its parent
group from advancing. Nested groups and var-data follow the same rule.

Fixed-block fields remain available through a zero-cost body view and do not
advance tail state. `finish()` and `skip_remaining()` move sequentially;
`rewind()` consumes the current stage and returns a fresh initial decoder.

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
                // Process quote -- zero allocation
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

For stricter feed handlers, the planned typed policy shape is
`FrameCursor<LengthPrefixedU32, MySchema>`. That makes external framing and
schema identity part of the type, which is the safer HFT-facing API when a
process handles multiple venues or schema generations.

### Avoid allocation in error paths

The error types (`DecodeError`, `EncodeError`, `VerifyError`) are
`const`-constructible with `&'static str` messages -- no heap allocation on
failure.

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

Every scalar field also exposes compile-time null, min, and max constants:

```rust
pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
```

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
// compiled value -- using the compiled value would read tail data from
// the wrong offset.
```

## Safety summary

| Feature | Safe? | Notes |
|---------|-------|-------|
| `foo()` | Yes | Bounds-checked, null-mapped, infallible for scalars/enums/sets/composites |
| `raw_foo()` | Yes | Bounds-checked, no null mapping (optional/versioned/array fields) |
| `foo_as_str()` | Yes | UTF-8 validated |
| Concrete tail stages | Yes | Compile-time component order; runtime counts remain checked separately |
| `bound-check-disabled` | Conditional | Explicit trusted-input mode with documented preconditions |
| `as_chunks()` | Yes | Bounds-checked |
| `verify()` | Yes | Pre-decode buffer structural validation |
| Encoder setters | Yes | Bounds-checked on write |
