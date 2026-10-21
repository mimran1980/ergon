# App Types on Composites

SBE composite types like `Decimal { mantissa, exponent }` have a fixed wire
layout. Mapping them to Rust domain types is done at configuration time — no
hand-rolled per-field converters needed.

**Option A — generic converter (`with_conversion`):** the generated codec
emits `price_from<T: TryToSbe<Decimal>>()` and `price_as<T: TryFromSbe<Decimal>>()`.
You implement the trait for any app type. One wire type, many app types.

**Option B — concrete mapping (`with_domain_type`):** the generated codec
emits `price(rust_decimal::Decimal)?` and `price()? -> rust_decimal::Decimal`.
Exactly one Rust type per wire type. The trait impls are generated for you.

**Option C — generated mapping (`with_enum_mapping`, `with_composite_mapping`):**
for an enum or composite whose application counterpart has the same shape,
ergo-sbe writes the `TryFromSbe` / `TryToSbe` impls too. See below.

Full comparison with code examples: [with_conversion vs with_domain_type](../configuration/conversion-vs-domain.md).

## Generated mappings

An application enum that mirrors an SBE enum, and a struct that mirrors a
composite, need no hand-written conversion:

```rust,ignore
{{#include ../../../../sbe/tests/type_mapping_test.rs:mapping_config}}
```

```rust,ignore
{{#include ../../../../sbe/tests/type_mapping_test.rs:app_types}}
```
*(From `sbe/tests/type_mapping_test.rs`, which generates, compiles and runs
these against the plain codec.)*

Each mapping also acts as a `with_domain_type` for its type, so every field of
that type takes the application type: `dec.side()? -> app::Side`,
`enc.leg(leg)?`, and the application type in a domain DTO (a DTO field of an
enum named `BooleanType`, or with `semanticType="Boolean"`, stays `bool`, as
under `with_domain_type`). A `presence="constant"` field of a mapped enum has
the same domain getter beside its `*_wire` getter, and no setter. The wire
bytes are the ones the plain codec writes.

A mapped setter's error is `sbe_rt::MappingError`, which converts into
`sbe_rt::EncodeError`, so `enc.leg(leg)?` works inside an encode or `send`
closure. A domain DTO's `encode` and `encode_from` report the same refusal as
`EncodeError::Conversion`, with the field's path and the index of its entry
in each group on the way (`fills[1].leg`) and the `MappingError` itself.

**Enum** (`EnumMapping`). A schema variant maps to the application variant of
the same generated name (`sellShort` → `SellShort`) unless renamed. Both
impls are exhaustive `match`es, so an application enum that lacks a schema
variant, or has one the schema lacks, does not compile.

**Composite** (`CompositeMapping`). `TryFromSbe` is a struct literal built
from the composite's getters, `TryToSbe` calls the composite's `new`; a
member fills the field of the same snake-case name unless renamed. Constant
members have no field (they are not on the wire). An optional scalar member is
an `Option` field: `None` writes the member's null value, and `Some` of the
null value — which would read back as `None` — is refused. A member whose enum
or composite has its own generated mapping uses that mapping's application
type (`price: app::Px`, `side: app::Side` above); every other member keeps its
generated wire type. A struct missing a member's field, or with a field no
member fills, does not compile.

### `NullVal` and unknown values

`NullVal` has no application value, so decoding it is an explicit
`sbe_rt::MappingError::NullEnum`, never a default variant. The generated
decoder folds every discriminant the schema does not declare into `NullVal`
before the mapping sees it, so an unknown value is the same error: the mapping
cannot tell it from the schema null, and only the decoder's `raw_<field>()`
still reads the wire byte. With `with_null_as_option` on the enum, both read as
`Ok(None)` instead:

```rust,ignore
{{#include ../../../../sbe/tests/type_mapping_test.rs:null_and_unknown}}
```

On a `presence="optional"` field, `NullVal` is not an error but the field
being absent, so a mapped enum that such a field uses needs
`with_null_as_option`: without it, generation fails with
`GenerateError::InvalidConfiguration` naming the field, instead of every
valid frame that leaves the field out failing to decode.

## Optional composites with a null image

A composite member itself can carry `presence="optional"` with a schema
`nullValue` — e.g. a `PriceNull9`-style Decimal whose `mantissa` is optional
while `exponent` stays constant. That member decodes as `Option<i64>`
(checked against the wire null sentinel), not a bare `i64`. This is distinct
from a composite *field* gated by `sinceVersion` (which the whole composite
accessor wraps in `Option<Decoder>`), and from a composite with no `nullValue`
anywhere (which has no null image to check, so it decodes as a plain value —
see [with_conversion vs with_domain_type](../configuration/conversion-vs-domain.md)).

Because the wire null sentinel is not a valid `rust_decimal::Decimal`, the
`with_domain_type` accessor fails closed with a typed error rather than
silently decoding the sentinel as a huge/wrong number:

```rust,ignore
{{#include ../../../../sbe/tests/baseline_test.rs:optional_composite_null_image}}
```
*(From `optional_composite_member_null_image_roundtrip` in
`sbe/tests/baseline_test.rs` — a real generated-and-compiled test, not a
standalone snippet.)*
