# What Generated Code Looks Like

For a schema with one message (`Car`), `ergo-sbe` emits a single Rust module with:

## Decoder (flyweight over `&[u8]`)

```rust,ignore
// Each message gets a zero-allocation decoder.
pub struct CarDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}

impl<'a> CarDecoder<'a> {
    // Illustrative only — real values from your schema.
    pub const SCHEMA_ID: u16 = 1;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 45;
    pub const HEADER_LENGTH: usize = 8;

    // Checked framed entry (message start). Validates header + fixed extent.
    pub fn try_decode(buf: &'a [u8], pos: usize)
        -> Result<Self, sbe_rt::DecodeError> { ... }

    // Proves the header+fixed-body extent and panics if short.
    // Caller does not need `unsafe` — the proof is in the constructor.
    pub fn wrap(buf: &'a [u8], message_offset: usize,
               acting_block_length: usize, acting_version: u16)
        -> Self { ... }

    // Full dynamic-tail structural check (associated, not `car.verify()`).
    pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::VerifyError> { ... }

    // Fixed fields are random-access — zero-copy reads after a checked wrap.
    #[inline]
    pub fn serial_number(&self) -> u64 {
        let offset = self.offset + 0;
        u64::from_le_bytes(/* private read after extent proof */)
    }
}
```

## Encoder (type-state stages)

```rust,ignore
// Wire order is enforced by named stage types.
pub struct CarEncoder<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> { ... }
pub struct CarAfterFuelFigures<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> { ... }
pub struct CarAfterPerformanceFigures<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> { ... }
pub struct CarComplete<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> { ... }

// Calling stages out of order is a type error — `CarEncoder` has no `asks()`.
impl<'a> CarEncoder<'a> {
    pub fn fixed(self, fields: &CarFixedFields) -> Self { ... }
    pub fn fuel_figures(self, count: u16, f: impl FnOnce(...) -> ...) -> Result<CarAfterFuelFigures> { ... }
}
impl<'a> CarAfterFuelFigures<'a> {
    pub fn performance_figures(self, ...) -> Result<CarAfterPerformanceFigures> { ... }
}
impl<'a> CarComplete<'a> {
    pub fn encoded_length_with_header(&self) -> usize { ... }
    pub fn as_bytes_with_header(&self) -> &[u8] { ... }
}
```

## Metadata: no field-name collisions

Utility methods like `remaining`, `buffer`, `as_bytes_with_header`, and
`as_body_bytes` are scoped inside a zero-copy **metadata struct** returned by
`get_metadata()`. This means a schema field named `remaining` or `buffer`
generates `dec.remaining()` / `dec.buffer()` as field accessors — no `_field`
suffix needed. No generated method name can ever collide with a user's schema
field name.

### What does `remaining()` mean?

| Receiver | `remaining()` means |
|----------|---------------------|
| Group decoder (e.g. `FuelFiguresDecoder`) | **Entry count** left (`usize`) — not bytes |
| `dec.get_metadata()` / `enc.get_metadata()` | **Byte slice** after the acting fixed block (`&[u8]`) |
| Schema field named `remaining` | Ordinary **field accessor** (natural name, no rename) |

Session framing (header then app payload) must use
`get_metadata().remaining()` for the payload bytes.

```rust,ignore
let dec = CarDecoder::try_decode(&buf, 0)?;
dec.serial_number();                              // field accessor — never collides
dec.get_metadata().remaining();                   // metadata — never collides
dec.get_metadata().buffer();                      // metadata — never collides
// Car has groups/var-data: metadata is fixed-block only (not a full frame).
dec.get_metadata().as_fixed_region_with_header()?;
// Complete frame after walking tails, or rescan without consuming:
// complete.as_bytes_with_header() / dec.as_bytes_with_header()?
dec.encoded_length_with_header()?;                // hot path stays on base struct
```

The metadata struct holds a reference to the parent (zero-copy):

```rust,ignore
pub struct CarDecoderMetadata<'m, 'a> {
    decoder: &'m CarDecoder<'a>,
}
```

Encoders have the same pattern. Fixed-only messages keep complete-sounding
`as_body_bytes` / `as_bytes_with_header` on metadata; messages with tails use
`as_fixed_body_bytes` / `as_fixed_region_with_header` until the complete stage.

```rust,ignore
let meta = enc.get_metadata();
meta.as_fixed_body_bytes();            // fixed block only when message has tails
meta.as_fixed_region_with_header();    // header + fixed block — not full frame
meta.message_offset();                 // message start in buffer
```

### Metadata limits (tailed messages)

| API | Span | Full Car frame? |
|-----|------|-----------------|
| `meta.limit()` | body start + **acting** block length | **No** — stops at fixed block end |
| `meta.as_fixed_region_with_header()?` | header + acting fixed block | **No** |
| `meta.remaining()` | bytes after that fixed end | May still include unread tails of *this* message |
| complete stage `as_bytes_with_header()` | header through last var-data | **Yes** (after walking tails) |
| `dec.as_bytes_with_header()?` (inherent rescan) | same full frame without consuming stages | **Yes** (rescans tails) |

Do **not** use `meta.limit()` as the next-message offset in a multi-message
buffer for Car-shaped schemas — that truncates at the fixed block.

### `acting_version` / `acting_block_length` (dual surface)

These remain **inherent** on the message decoder for the hot path
(`dec.acting_version()`, `dec.acting_block_length()`) and are also exposed on
metadata (`dec.get_metadata().acting_version()`) for a uniform placement facet.
Both paths return the same values. They are reserved method names: a schema
field named `actingVersion` becomes `acting_version_field` on the decoder.

## Exact buffer sizing

```rust,ignore
// Fixed-only messages: const length.
let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];

// Variable-length: staged builder (zero allocation).
let len = CarEncoder::compute_length()
    .fuel_figures_ragged(2, |ff| {
        ff.add()?.usage_description(5)?;
        Ok(())
    })?
    .performance_figures_ragged(0, |_| Ok(()))?
    .manufacturer(5)?
    .model(9)?
    .activation_code(6)?
    .encoded_length_with_header();
```

## Configuration controls output size

Every aspect of generated output is configurable:

```rust,ignore
GenerationConfig::new("msgs")
    .with_display_debug(false)   // omit Debug/Display impls
    .with_meta_attributes(false) // omit *_ENCODING_OFFSET etc.
    .with_dispatch(false)        // omit AnyMessage/FrameCursor
    .with_domain_objects(DomainVarData::Bytes) // owned DTOs
```

A default single-message schema with these knobs off produces minimal output.
All knobs default to `true` — lean output is opt-in with `with_*(false)`.

## Real example

The [sbe-feature-tour sample](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
builds and exercises a real Car schema. Run `cargo test` inside that directory
to see the generated API in action, or open `src/generated/feature_tour.rs`
after `cargo build`.
