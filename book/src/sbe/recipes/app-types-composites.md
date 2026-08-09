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
