# What Generated Code Looks Like

For a schema with one message (`Car`), `ergo-sbe` emits a single Rust module with:

## Decoder (flyweight over `&[u8]`)

```rust,ignore
// Each message gets a zero-allocation decoder.
pub struct CarDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}

impl<'a> CarDecoder<'a> {
    pub const SCHEMA_ID: u16 = 1;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 45;
    pub const HEADER_LENGTH: usize = 8;

    // Checked framed entry (message start). Validates header + fixed extent.
    pub fn decode(buf: &'a [u8], pos: usize)
        -> Result<Self, sbe_rt::DecodeError> { ... }

    // Checked external-metadata wrap (still returns Result).
    pub fn wrap(buf: &'a [u8], message_offset: usize,
               acting_block_length: usize, acting_version: u16)
        -> Result<Self, sbe_rt::DecodeError> { ... }

    // Full dynamic-tail structural check (associated, not `car.verify()`).
    pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::DecodeError> { ... }

    // Fixed fields are random-access — zero-copy reads after a checked wrap.
    #[inline]
    pub fn serial_number(&self) -> u64 {
        let offset = self.pos + 0;
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
    .manufacturer(5)?
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
