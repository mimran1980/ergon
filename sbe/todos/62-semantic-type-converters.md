# WireCompatibleExtensions: semanticType-driven converters

> **REOPENED 2026-07-11 with a narrower approved interface:** the historical
> semantic-type registry and built-in dependency design below remains
> superseded. The active requirement is a dependency-free generic converter
> seam for a structurally validated SBE Decimal composite.

**Status: DONE (implemented per remediation plan Task 2; verified 2026-07-18)** — dependency-free generic `SbeDecimal` trait plus raw `*_wire` setters/getters emitted when `GenerationConfig::enable_decimal_converters("Decimal")` is set; structurally validated (int64 mantissa + int8 exponent). Proven by the normalized-app generated codec and the exchange-orderbook round-trip tests (slice 1, 19/19).

## Active Decimal converter design

`GenerationConfig::enable_decimal_converters("Decimal")` validates that the
named composite contains signed `int64` `mantissa` followed by signed `int8`
`exponent`. The generator emits:

```rust
pub trait SbeDecimal: Sized {
    type Error;

    fn try_from_sbe(mantissa: i64, exponent: i8) -> Result<Self, Self::Error>;
    fn try_into_sbe(self) -> Result<(i64, i8), Self::Error>;
}
```

Any application type can implement the local trait. Generated code does not
depend on `rust_decimal`. In converter mode, ordinary Decimal-backed field
methods are generic over `D: SbeDecimal`; infallible `*_wire` methods retain
the raw generated composite. Without converter mode, ordinary methods continue
to use the raw composite.

## Active acceptance criteria

- [ ] Add repeatable registration for one or more named Decimal composites.
- [ ] Reject missing composites, wrong member names/order, unsigned mantissa or
      exponent, and primitive widths other than int64/int8 during generation.
- [ ] Emit one local dependency-free `SbeDecimal` trait when at least one
      Decimal composite is enabled.
- [ ] Emit generic converted decoder accessors and encoder setters plus raw
      infallible `*_wire` methods.
- [ ] Preserve ordinary raw methods when converter mode is disabled.
- [ ] Prove application implementations for `rust_decimal::Decimal` and a
      second custom decimal type without generator source injection.
- [ ] Prove exact forward/reverse conversion, mixed exponents, negative values,
      adapter range failures, overflow, and precision-loss rejection.
- [ ] Compose converter errors through `try_fixed`, `try_<group>`, and nested
      payload closures with `?`.
- [ ] Prove zero allocation and inspect monomorphised assembly.
- [ ] Benchmark raw and converted paths separately; Aeron comparisons include
      equivalent conversion work and pass the canonical five-run ratio gate.
- [ ] Reach 100 percent line, function, region, and branch coverage for new or
      changed handwritten production paths with complementary generated-code
      proofs.

**Blocked by:** none (codegen feature)

When a schema field or type carries a `semanticType` attribute
(e.g. `semanticType="Price"`, `semanticType="Timestamp"`,
`semanticType="Quantity"`), generate `From`/`Into` impls that convert between
the raw SBE wire type and the semantic Rust type.

This is gated behind `CompatibilityMode::WireCompatibleExtensions` — the wire
bytes are unchanged, only the Rust API surface grows.
**Historical status: CLOSED / SUPERSEDED**

**Decision after todo-coherence recheck (2026-07-08):** keep converters parked
behind todo 65 and semantic-newtype design. Converters that pull in
`rust_decimal`, `chrono`, or allocation-heavy formatting should not become part
of the default hot-path API.


## Examples

```xml
<type name="price_t" primitiveType="int64" semanticType="Price" scale="6"/>
```
```rust
// Generated (gated by WireCompatibleExtensions):
impl From<Price> for rust_decimal::Decimal {
    fn from(val: Price) -> Self { ... }  // divide by 10^scale
}
impl TryFrom<rust_decimal::Decimal> for Price { ... }
```

```xml
<type name="timestamp_t" primitiveType="uint64" semanticType="Timestamp"/>
```
```rust
impl From<Timestamp> for chrono::NaiveDateTime { ... }
```

```xml
<type name="qty_t" primitiveType="int32" semanticType="Quantity" scale="3"/>
```
```rust
impl From<Quantity> for rust_decimal::Decimal { ... }
```

## Acceptance criteria

- [x] Read `semanticType` from XML type/field attributes (parser already
  captures this in IR)
- [x] Registry: map from `semanticType` string to converter pair
  (wire → semantic, semantic → wire)
- [x] Built-in converters: `Price`/`Quantity` (→ `rust_decimal`),
  `Timestamp` (→ `chrono`), `String` (var-data → `&str`/`String`)
- [x] User-extensible: `GenerationConfig.semantic_registry` for custom
  converters
- [x] Gated: only emitted when `compatibility == WireCompatibleExtensions`
- [x] Tests: round-trip encode→decode→convert→semantic-equal

Ref: DECISIONS.md §11, gap analysis todo 51, user request for extension
methods/converters.
