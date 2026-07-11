# Authoring SBE schemas for ErgoSBE

ErgoSBE consumes standard SBE XML schemas. This guide covers schema elements,
ErgoSBE-specific considerations, and best practices.

## Schema structure

An SBE schema is an XML document with a `<messageSchema>` root:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<messageSchema
    package="myapp.sbe"
    id="1"
    version="0"
    byteOrder="littleEndian"
    description="My trading schema"
    semanticVersion="1.0.0"
    headerType="messageHeader">
  <types>
    <!-- Type definitions -->
  </types>
  <message name="..." id="...">
    <!-- Fields, groups, var-data -->
  </message>
</messageSchema>
```

### Root attributes

| Attribute | Required | Description |
|-----------|----------|-------------|
| `package` | Yes | Reverse-domain package name (e.g. `"myapp.sbe"`) |
| `id` | Yes | Schema identifier -- must match the wire header |
| `version` | No | Schema version; defaults to `0` |
| `byteOrder` | No | `"littleEndian"` (default) or `"bigEndian"` |
| `headerType` | No | Composite name for the message header; defaults to `"messageHeader"` |
| `description` | No | Human-readable schema description |
| `semanticVersion` | No | Schema semantic version string |

## Type definitions

### Primitive types

Simple types have no children:

```xml
<type name="uint32" primitiveType="uint32" presence="required"/>
```

### Composites

Composite types group multiple fields into a reusable structure. They become
`#[repr(transparent)]` value structs wrapping `[u8; N]`:

```xml
<composite name="messageHeader" description="SBE message header">
    <type name="blockLength" primitiveType="uint16"/>
    <type name="templateId"   primitiveType="uint16"/>
    <type name="schemaId"     primitiveType="uint16"/>
    <type name="version"      primitiveType="uint16"/>
</composite>
```

Composite accessors are **infallible** -- no `Result`, no `?`. Accessors that
read runtime buffers optimise for fast inline reads instead of preserving
`const fn`.

The default generated API keeps this raw composite behaviour. A planned,
not-yet-shipped opt-in for an exact `mantissa: int64`, `exponent: int8`
Decimal composite emits generic fallible `SbeDecimal` field methods while
retaining infallible `*_wire` raw access. See todo 62 and the generated API
guide; generated output will not depend on a particular application decimal
crate.

### Enums

Enums become a flat Rust `enum` with a `NullVal` variant for unknown wire
values. No separate `Kind` type is generated:

```xml
<enum name="Side" encodingType="uint8" description="Order side">
    <validValue name="Buy"  description="Buy side">1</validValue>
    <validValue name="Sell" description="Sell side">2</validValue>
</enum>
```

Generated access:
```rust
let side = order.side();             // Side (infallible)
match side {
    Side::Buy => println!("Buy"),
    Side::Sell => println!("Sell"),
    Side::NullVal => println!("Unknown side: {}", side.raw()),
}
// side.raw() returns the raw wire byte
```

The `NullVal` variant safely holds any unknown wire discriminant without
panicking.

### Sets (choices)

Sets become a `#[repr(transparent)]` newtype struct with bit-test accessors:

```xml
<set name="Flags" encodingType="uint8" description="Message flags">
    <choice name="EndOfSequence">0</choice>
    <choice name="Snapshot">1</choice>
</set>
```

Generated access:
```rust
let flags = order.flags();            // Flags (infallible)
let is_snapshot = flags.snapshot();   // bool (infallible)
```

## Message definition

### Fields

Fixed-offset fields in the message's fixed block:

```xml
<message name="Quote" id="1" description="A market quote">
    <field name="price"    id="1" type="int64" offset="0"
           description="Quote price" semanticType="Price"/>
    <field name="quantity" id="2" type="uint32" offset="8"
           description="Quote quantity" semanticType="Qty"/>
    <field name="side"     id="3" type="Side" offset="12"
           description="Quote side"/>
</message>
```

The `semanticType` attribute appears in rustdoc (e.g. `Semantic type: Price`).

Required scalar, enum, set, and composite field accessors are **infallible**:

```rust
let price = quote.price();    // i64 -- no ?, no unwrap
let qty = quote.quantity();   // u32
let side = quote.side();      // Side
```

### Optional fields

Fields with `presence="optional"` use the type's null value sentinel:

```xml
<field name="peggedPrice" id="4" type="int64" offset="16"
       presence="optional" nullValue="-1"
       description="Pegged price (null when not pegged)"/>
```

Accessors return `Option<i64>` -- `None` when the wire value equals the null
sentinel. No `Result` wrapper -- just `Option`.

### Version-gated fields

Fields with `sinceVersion > 0` return `Option<T>` -- `None` when the wire
`actingVersion` is below the field's introduction version:

```xml
<field name="newField" id="8" type="uint32" offset="24"
       sinceVersion="1" description="Added in version 1"/>
```

```rust
let new_field = quote.new_field();   // Option<u32> -- None if wire version < 1
```

### Constant fields

Fields with `presence="constant"` are not encoded on the wire. The accessor
returns the constant value directly:

```xml
<field name="messageType" id="5" type="char" offset="20"
       presence="constant">Q</field>
```

### Repeating groups

Groups define a repeating block. Each entry can have fields, nested groups,
and var-data:

```xml
<group name="orders" id="6" dimensionType="groupSizeEncoding"
       description="Repeating orders">
    <field name="orderId" id="1" type="uint64" offset="0"/>
    <field name="orderQty" id="2" type="uint32" offset="8"/>
</group>
```

Generated access:
```rust
let orders = quote.orders()?;            // consumes the current message stage
let after_orders = orders.finish()?;     // advances unread entries in wire order
```

Concrete group and entry stages enforce the schema order. An active entry
prevents its parent group from advancing; fixed entry fields remain infallible.
Runtime counts are still validated from the group dimension header.

### Variable-length data

Var-data fields carry a length prefix followed by raw bytes:

```xml
<data name="description" id="7" type="varDataEncoding"
      description="Free-text description"/>
```

Var-data uses the next concrete tail stage and returns borrowed bytes; use
`_as_str()` for UTF-8:

```rust
let description = after_orders.description()?;
let desc = description.as_bytes()?;         // &[u8]
let desc_str = description.as_str()?;       // &str
```

## Versioning

Fields, groups, and var-data can be versioned with `sinceVersion`:

```xml
<field name="newField" id="8" type="uint32" offset="24"
       sinceVersion="1" description="Added in version 1"/>
```

Accessors for `sinceVersion > 0` fields return `Option<T>` -- `None` when the
wire `actingVersion` is below the field's introduction version.

## Schema includes

Use `xi:include` to share type definitions across schemas:

```xml
<messageSchema package="market_data" id="2" version="0"
               byteOrder="littleEndian">
    <include href="common-types.xml"/>
    <message name="MarketDataIncrementalRefresh" id="31">
        <!-- types from common-types.xml are available -->
    </message>
</messageSchema>
```

ErgoSBE resolves includes relative to the parent schema directory, the
current working directory, and well-known paths in the
`simple-binary-encoding` submodule.

## Best practices

1. **Always set `offset` explicitly** on message fields -- this documents wire
   layout and avoids ambiguity.
2. **Use `description`** on types and fields -- these become rustdoc comments
   in the generated code.
   ErgoSBE also preserves `<description>` and supported `<comment>` child
   elements/tags, plus ordinary XML `<!-- -->` comments associated with the
   nearest schema element. When more than one source is present, generated
   rustdoc contains their deterministic combination.
3. **Use `semanticType`** for domain concepts (`Price`, `Qty`, `UTCTimestamp`)
   -- these appear in IDE hover docs.
4. **Prefer `uint8`-based enums** for small finite sets -- they produce compact
   wire encoding.
5. **Use `sinceVersion` for schema evolution** -- it produces version-aware
   accessors without breaking existing messages.
6. **Keep the `messageHeader` composite standard** (four `uint16` fields) unless
   you have a specific reason to customise it -- `headerType` is configurable.
