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
        BufferTooShort { field: &'static str, offset: usize, needed: usize, available: usize },
        WrongSchema { expected: u16, actual: u16 },
        UnknownTemplateLength { template_id: u16 },
        InvalidVarDataLength { field: &'static str, length: u32 },
        Utf8(core::str::Utf8Error),
    }

    pub enum EncodeError {
        BufferTooShort { needed: usize, available: usize },
        VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
    }

    pub enum VerifyError {
        HeaderTooShort,
        InvalidBlockLength { expected_min: usize, actual: usize },
        GroupDimOutOfBounds { field: &'static str, offset: usize },
        VarDataOutOfBounds { field: &'static str, offset: usize, length: u32 },
        MessageTooShort { needed: usize, available: usize },
    }

    pub trait SbeMessage {
        const TEMPLATE_ID: u16;
        const BLOCK_LENGTH: usize;
        const SCHEMA_ID: u16;
        const SCHEMA_VERSION: u16;
    }
}
```

All error types implement `core::error::Error` and `core::fmt::Display`.

`VerifyError` is returned by `Decoder::verify()` for pre-decode buffer
validation. It reports the specific structural issue (bad block length, group
dimension out of bounds, var-data out of bounds).

## Composite types

Composites become `#[repr(transparent)]` structs wrapping `[u8; N]`:

```rust
#[repr(transparent)]
pub struct MessageHeader(pub [u8; 8]);

impl MessageHeader {
    pub fn block_length(&self) -> u16 { /* ... */ }
    pub fn template_id(&self) -> u16 { /* ... */ }
    pub fn schema_id(&self) -> u16 { /* ... */ }
    pub fn version(&self) -> u16 { /* ... */ }
    pub const fn new(block_length: u16, template_id: u16,
                     schema_id: u16, version: u16) -> Self { /* ... */ }
}
```

Each member gets an **infallible** accessor with no `Result`. Accessors that
read bytes from a runtime buffer are not required to be `const fn`; they use the
fast runtime path. Pure constructors and constants may still be `const fn`.

## Enum types

Enums are generated as flat Rust `enum`s with a `NullVal` variant for unknown
wire values. There is no separate `Kind` type -- the enum **is** the wire type:

```rust
#[repr(u8)]
pub enum Side {
    Buy = 1,
    Sell = 2,
    NullVal,
}

impl Side {
    pub fn raw(self) -> u8 { self as u8 }
    pub const fn from_raw(val: u8) -> Self { /* ... */ }
}

impl From<u8> for Side { /* ... */ }
impl From<Side> for u8 { /* ... */ }
```

The `NullVal` variant holds any unknown wire discriminant, ensuring the
decoder never panics on unexpected wire values. Use `from_raw()` to construct
from a raw byte, and `raw()` to extract the byte back.

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

All bit accessors are **infallible**.

## Message decoder

### Buffer verification

Before constructing a decoder, you can validate the entire message buffer:

```rust
// Static method on the decoder type
QuoteDecoder::verify(&buf)?;   // Returns Result<(), VerifyError>
```

This checks the header, block length, group dimension headers, and var-data
bounds in a single pass. Guards against truncated messages at the feed level.

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

For a `price` field (`int64`, `sinceVersion=0`, required) -- **infallible**:

```rust
pub fn price(&self) -> i64;
pub const unsafe fn price_unchecked(&self) -> i64;  // no bounds check
```

For a required field with `sinceVersion=0` -- the accessor is always a plain
return type with no `Result` wrapper.

For a `sinceVersion > 0` field (`uint32`, `sinceVersion=1`, required):

```rust
pub fn new_field(&self) -> Option<u32>;               // None if version below sinceVersion
pub const unsafe fn new_field_unchecked(&self) -> u32; // raw wire value
pub fn raw_new_field(&self) -> Option<u32>;            // no checked fallback
```

Version-gated fields return `Option<T>` directly -- `None` when the wire
`actingVersion` is below the field's introduction version.

For an optional field (`int64`, `presence="optional"`):

```rust
pub fn pegged_price(&self) -> Option<i64>;             // None if null sentinel or absent by version
pub unsafe fn pegged_price_unchecked(&self) -> i64;    // raw wire value, no null mapping
pub fn raw_pegged_price(&self) -> Option<i64>;         // None if absent by version, not null-sentinel
```

Optional fields also return `Option<T>` directly -- no `Result` wrapper. The
`raw_` variant distinguishes null-sentinel from version-absence.

For a constant field:

```rust
pub fn message_type(&self) -> &'static str;  // or typed value
```

For a fixed-size array (`int32[3]`) -- may be version-gated:

```rust
pub fn some_numbers(&self) -> Result<[i32; 3], sbe_rt::DecodeError>;
pub unsafe fn some_numbers_unchecked(&self) -> [i32; 3];
pub fn raw_some_numbers(&self) -> [i32; 3];
```

Fixed arrays return `Result` when version-gated (they can be conditionally
absent), or can be plain values when always present. Runtime array reads should
prefer slice/byte-copy fast paths over const-only byte loops.

Every field also exposes compile-time constants:

```rust
pub const PRICE_NULL: i64 = i64::MAX;
pub const PRICE_MIN: i64 = i64::MIN + 1;
pub const PRICE_MAX: i64 = i64::MAX - 1;
```

### Group access

```rust
pub fn orders(&self) -> Result<OrdersDecoder<'a>, sbe_rt::DecodeError>;
```

The group decoder implements `Iterator` and `ExactSizeIterator`:

```rust
let orders = quote.orders()?;
for order in orders {
    let id = order.order_id();    // infallible
    let qty = order.order_qty();  // infallible
}
```

Group entry accessors follow the same patterns as messages: scalar/enum/set
accessors are infallible, groups and var-data return `Result`, unchecked
methods exist as `unsafe fn`.

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
    .side(Side::from_raw(1));

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
- `FramingPolicy::LengthPrefixU32` -- 4-byte LE length prefix
- `FramingPolicy::LengthPrefixU16` -- 2-byte LE length prefix
- `FramingPolicy::Fixed(len)` -- every frame is exactly `len` bytes

Strict feed APIs should move the policy and schema identity into the type once
todo 134 lands:

```rust
let cursor = FrameCursor::<LengthPrefixedU32, MySchema>::new(buf);
```

That lets generic feed code reject schema/policy mismatches at compile time and
keeps unknown-template forwarding tied to a policy that actually supplies a
full frame length.
