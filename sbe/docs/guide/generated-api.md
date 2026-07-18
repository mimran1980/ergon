# Generated API reference

This guide documents every type and trait that ErgoSBE generates from an SBE
schema. The examples use a hypothetical `Quote` message from a market-data schema.

> **Encoder/decoder entry (current):** generated `wrap` / `wrap_and_apply_header`
> return `Result` (`EncodeError` / `DecodeError`). Prefer fallible `?` at the
> trust boundary. Tail groups/var-data use **concrete consuming stages**
> (`into_bids` → `finish` → next stage); see [DECISIONS.md](../../design/DECISIONS.md)
> and [advanced.md](advanced.md).

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
```

For a required field with `sinceVersion=0` -- the accessor is always a plain
return type with no `Result` wrapper.

For a `sinceVersion > 0` field (`uint32`, `sinceVersion=1`, required):

```rust
pub fn new_field(&self) -> Option<u32>;               // None if version below sinceVersion
pub fn raw_new_field(&self) -> Option<u32>;            // no checked fallback
```

Version-gated fields return `Option<T>` directly -- `None` when the wire
`actingVersion` is below the field's introduction version.

For an optional field (`int64`, `presence="optional"`):

```rust
pub fn pegged_price(&self) -> Option<i64>;             // None if null sentinel or absent by version
pub fn raw_pegged_price(&self) -> Option<i64>;         // None if absent by version, not null-sentinel
```

Optional fields also return `Option<T>` directly -- no `Result` wrapper. The
`raw_` variant distinguishes null-sentinel from version-absence.

For a constant field:

```rust
pub fn message_type(&self) -> &'static str;  // or typed value
```

For a required version-zero fixed-size array (`int32[3]`):

```rust
pub fn some_numbers(&self) -> [i32; 3];
```

Version-gated or optional arrays use `Option<[T; N]>` as required by the same
acting-version/null rules as scalars. Runtime array reads use slice/byte-copy
fast paths rather than const-only byte loops. The default public surface does
not generate a per-field unchecked variant; trusted-input mode changes
internals without changing accessor names.

Every field also exposes compile-time constants:

```rust
pub const PRICE_NULL: i64 = i64::MAX;
pub const PRICE_MIN: i64 = i64::MIN + 1;
pub const PRICE_MAX: i64 = i64::MAX - 1;
```

### Ordered group and var-data access

Tail components are available only through concrete consuming stages. For an
order book whose schema contains `bids` followed by `asks`:

```rust
OrderBookDecoder
  -> BidsDecoder
  -> OrderBookAfterBids
  -> AsksDecoder
  -> OrderBookComplete
```

`asks()` exists only on `OrderBookAfterBids`. Starting `bids` consumes the
initial decoder. An active bid entry owns the transition back to `BidsDecoder`,
so the group cannot advance while that entry or a nested tail is active.

`BidsDecoder::finish(self)` advances over unread entries in wire order.
`skip_remaining(self)` may be provided as the explicit skip spelling. Either
returns `OrderBookAfterBids`. `rewind(self)` consumes any current stage and
returns a fresh `OrderBookDecoder`.

Fixed-block scalar, enum, set, composite, and array accessors remain available
through a zero-cost body view and do not advance tail state. Compile-time types
enforce component order; group counts remain runtime values validated from the
wire dimension header.

Nested groups and var-data use equivalent concrete entry stages. A var-data
stage exposes borrowed bytes and, when character encoding is declared, a
checked string view:

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

Encoders use concrete consuming stages to enforce tail ordering at compile
time. For the same order-book schema:

```rust
OrderBookEncoder
  -> BidsEncoder
  -> OrderBookAfterBids
  -> AsksEncoder
  -> OrderBookComplete
```

Starting a group consumes the previous stage. Completing or explicitly writing
an empty/skipped group consumes the group stage and returns the next message
stage. An active entry prevents its parent group from advancing. Nested groups
and var-data use the same ownership pattern.

The canonical target puts complete-message `encoded_length()` / `as_bytes()` /
`AsRef<[u8]>` only on complete encoder stages. Incomplete stages expose
**explicitly partial** fallible length/bytes APIs (todo 157 — **DONE** 2026-07-18).
Do not treat a partial encoder buffer as a publishable complete message.

### Fallible chaining (shipped)

Optional fallible helpers such as `try_fixed`, `try_<group>`, and related
stage combinators are generated (todo **156** — **DONE** 2026-07-18). Nested
var-data may expose `into_<field>_as_message` / `try_<field>_as_message` on
some schemas; the full DECISIONS §3 bridge set (generic `as_decoder` /
remaining acceptance for todo **81**) may still be incomplete — see
[`docs/LIVING_BACKLOG.md`](../../../docs/LIVING_BACKLOG.md) item **SBE-81**.

### Generic Decimal conversion (shipped, opt-in)

For a schema-defined composite whose members are exactly `mantissa: int64`
followed by `exponent: int8`, enable converters:

```rust
let config = GenerationConfig::new("market_data")
    .enable_decimal_converters("Decimal");
```

The generated module owns a dependency-free `SbeDecimal` trait (todo **62** —
**DONE** 2026-07-18). Applications implement it for `rust_decimal::Decimal`
or another exact decimal type; generated code does not depend on
`rust_decimal`. Converter mode adds fallible generic field methods and keeps
infallible `*_wire` raw accessors.

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

Todo 135 (associated codec types on `SbeMessage`) is marked Phase-2 DONE in
the todo tree; the minimal constants API above is what all codecs implement.
Historical “planned associated types” sketch:

```rust
pub trait SbeMessage {
    type Decoder<'a>;
    type Encoder<'a>;
    type Schema;
}
```

That lets generic feed/proxy helpers use concrete generated codecs without
falling back to dynamic `AnyMessage` dispatch.

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
