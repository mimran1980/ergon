# Generated API reference

This guide documents every type and trait that ErgoSBE generates from an SBE
schema. The examples use a hypothetical `Quote` message from a market-data schema.

## Module structure

For a schema with package `"market_data"`, `id=2`, `version=1`, and module name
`"market_data"`, the generated file contains:

1. An `sbe_rt` inline module with runtime error types and traits.
2. Type definitions for all composites, enums, and sets.
3. A decoder and encoder pair for each message.
4. An `AnyMessage` dispatch enum.
5. A `FrameCursor` for iterating externally-framed buffers.

## Runtime module (`sbe_rt`)

Generated once per schema (or shared across schemas with `shared_module`).

```rust
pub mod sbe_rt {
    pub enum DecodeError {
        BufferTooShort { field: &'static str, needed: usize, available: usize },
        WrongSchema { expected: u16, actual: u16 },
        UnknownTemplateLength { template_id: u16 },
        InvalidVarDataLength { field: &'static str, length: u32 },
        Utf8(core::str::Utf8Error),
    }

    pub enum EncodeError {
        BufferTooShort { needed: usize, available: usize },
        VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
    }

    pub trait SbeMessage {
        const TEMPLATE_ID: u16;
        const BLOCK_LENGTH: usize;
        const SCHEMA_ID: u16;
        const SCHEMA_VERSION: u16;
    }
}
```

Both error types implement `core::error::Error` and `core::fmt::Display`.

## Composite types

Composites become `#[repr(transparent)]` structs wrapping `[u8; N]`:

```rust
#[repr(transparent)]
pub struct MessageHeader(pub [u8; 8]);

impl MessageHeader {
    pub const fn block_length(&self) -> u16 { /* ... */ }
    pub const fn template_id(&self) -> u16 { /* ... */ }
    pub const fn schema_id(&self) -> u16 { /* ... */ }
    pub const fn version(&self) -> u16 { /* ... */ }
    pub const fn new(block_length: u16, template_id: u16,
                     schema_id: u16, version: u16) -> Self { /* ... */ }
}
```

Each member gets a `const fn` accessor. The struct exposes its raw bytes via
`.0`.

## Enum types

Enums use the E3 pattern — a newtype struct for the wire value plus a proper
Rust enum for known variants:

```rust
#[repr(transparent)]
pub struct Side(pub u8);

pub enum SideKind {
    Buy = 1,
    Sell = 2,
}

impl Side {
    pub const Buy: Self = Self(1);
    pub const Sell: Self = Self(2);

    pub const fn kind(self) -> Option<SideKind> { /* ... */ }
    pub const fn into_kind(self) -> Option<SideKind> { /* ... */ }
    pub const fn raw(self) -> u8 { self.0 }
}

impl From<u8> for Side { /* ... */ }
impl From<Side> for u8 { /* ... */ }
impl TryFrom<Side> for SideKind { /* ... */ }
```

The `.kind()` method returns `None` for unknown wire values — an ordinary Rust
enum cannot hold unknown discriminants.

## Set (bitset) types

Sets become `#[repr(transparent)]` structs with per-bit accessors:

```rust
#[repr(transparent)]
pub struct Flags(pub u8);

impl Flags {
    pub const fn raw(self) -> u8 { self.0 }
    pub const fn end_of_sequence(self) -> bool { /* ... */ }
    pub fn set_end_of_sequence(&mut self, val: bool) { /* ... */ }
    pub const fn snapshot(self) -> bool { /* ... */ }
    pub fn set_snapshot(&mut self, val: bool) { /* ... */ }
}
```

## Message decoder

### Struct constants

```rust
impl QuoteDecoder<'a> {
    pub const SCHEMA_ID: u16 = 2;
    pub const SCHEMA_VERSION: u16 = 1;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 24;
    // Fixed-size messages only:
    pub const ENCODED_LENGTH: usize = 32;
    // Messages with groups/var-data:
    pub const MAX_ENCODED_LENGTH: usize = 1024;
}
```

### Construction

```rust
// From a buffer with SBE header at offset
let decoder = QuoteDecoder::wrap_and_apply_header(buf, 0)?;

// From a buffer with pre-parsed header
let decoder = QuoteDecoder::wrap(buf, 32, acting_block_length, acting_version);
```

### Field accessors

For a `price` field (`int64`, `sinceVersion=0`, required):

```rust
pub fn price(&self) -> Result<i64, sbe_rt::DecodeError>;
pub const unsafe fn price_unchecked(&self) -> i64;  // no bounds check
pub fn raw_price(&self) -> i64;                      // no null mapping
```

For a `sinceVersion > 0` field (`uint32`, `sinceVersion=1`, required):

```rust
pub fn new_field(&self) -> Result<Option<u32>, sbe_rt::DecodeError>;
pub const unsafe fn new_field_unchecked(&self) -> u32;
pub fn raw_new_field(&self) -> Option<u32>;
```

For an optional field (`int64`, `presence="optional"`):

```rust
pub fn pegged_price(&self) -> Result<Option<i64>, sbe_rt::DecodeError>;
pub unsafe fn pegged_price_unchecked(&self) -> i64;  // raw wire value
pub fn raw_pegged_price(&self) -> Option<i64>;       // None if absent by version
```

For a constant field:

```rust
pub fn message_type(&self) -> &'static str;  // or typed value
```

For a fixed-size array (`int32[3]`):

```rust
pub fn coordinates(&self) -> Result<[i32; 3], sbe_rt::DecodeError>;
pub unsafe fn coordinates_unchecked(&self) -> [i32; 3];
pub fn raw_coordinates(&self) -> [i32; 3];
```

### Group access

```rust
pub fn orders(&self) -> Result<OrdersDecoder<'a>, sbe_rt::DecodeError>;
```

The group decoder implements `Iterator` and `ExactSizeIterator`:

```rust
let orders = quote.orders()?;
for order in orders {
    let id = order.order_id()?;
}
```

Group entries have the same accessor patterns as messages: checked `Result`
accessors, `_unchecked` methods, and `raw_` accessors.

### Var-data access

```rust
pub fn description(&self) -> Result<&'a [u8], sbe_rt::DecodeError>;
pub fn description_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError>;
```

## Message encoder

### Fixed-size messages (no groups or var-data)

```rust
let mut buf = [0u8; QuoteEncoder::ENCODED_LENGTH];

let mut encoder = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)?;
encoder
    .price(12345)
    .quantity(100)
    .side(Side::Buy);

let bytes = encoder.as_ref();  // &[u8]
```

### Messages with tail (groups and/or var-data)

Encoders use type-state to enforce tail ordering at compile time:

```rust
// State: NeedsOrders
let encoder = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)?;

// Transition: NeedsOrders -> NeedsDescription
let encoder = encoder.orders(2, |group| {
    group.add(|entry| { entry.order_id(1).order_qty(100); })?;
    group.add(|entry| { entry.order_id(2).order_qty(200); })?;
    Ok(())
})?;

// Transition: NeedsDescription -> Complete
let encoder = encoder.description(b"Free text")?;

// Complete state exposes as_bytes()
let bytes = encoder.as_bytes();
```

The `add` method on group encoders returns `Result<(), EncodeError>`. Entry
setters return `&mut Self` for chaining.

## AnyMessage dispatch

```rust
#[non_exhaustive]
pub enum AnyMessage<'a> {
    Quote(QuoteDecoder<'a>),
    Trade(TradeDecoder<'a>),
    Unknown { header: MessageHeader, payload: &'a [u8] },
}

impl<'a> AnyMessage<'a> {
    pub fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError>;
    pub fn decode_frame(buf: &'a [u8], pos: usize, frame_len: usize)
        -> Result<DecodedFrame<'a>, sbe_rt::DecodeError>;
}
```

## SbeMessage trait

All decoder and encoder types implement `SbeMessage`:

```rust
pub trait SbeMessage {
    const TEMPLATE_ID: u16;
    const BLOCK_LENGTH: usize;
    const SCHEMA_ID: u16;
    const SCHEMA_VERSION: u16;
}
```

## FrameCursor

Iterates externally-framed SBE feed buffers:

```rust
let cursor = FrameCursor::new(buf, FramingPolicy::LengthPrefixU32);
for result in cursor {
    match result {
        Ok(frame) => {
            match frame.message {
                AnyMessage::Quote(quote) => { /* ... */ }
                AnyMessage::Unknown { payload, .. } => { /* forward */ }
                _ => {}
            }
        }
        Err(e) => { /* handle decode error */ }
    }
}
```

Framing policies:
- `FramingPolicy::LengthPrefixU32` — 4-byte LE length prefix
- `FramingPolicy::LengthPrefixU16` — 2-byte LE length prefix
- `FramingPolicy::Fixed(len)` — every frame is exactly `len` bytes
