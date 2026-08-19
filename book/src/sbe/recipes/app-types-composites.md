# App Types on Composites

SBE composite types like `Decimal { mantissa, exponent }` have a fixed wire
layout. Mapping them to Rust domain types is done at configuration time — no
hand-rolled per-field converters needed.

**Option A — generic converter (`with_conversion`):** the generated codec
emits `price_from<T: TryToSbe<Decimal>>()` and `price_as<T: TryFromSbe<Decimal>>()`.
You implement the trait for any app type. One wire type, many app types.

**Option B — concrete mapping (`with_domain_type`):** the generated codec
emits `try_price(rust_decimal::Decimal)?` and `try_price()? -> rust_decimal::Decimal`.
Exactly one Rust type per wire type. The trait impls are generated for you.

Full comparison with code examples: [with_conversion vs with_domain_type](../configuration/conversion-vs-domain.md).

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
