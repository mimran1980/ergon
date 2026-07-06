⚠️ **DEFERRED — post-v1.** WireCompatibleExtensions converters is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# WireCompatibleExtensions: semanticType-driven converters

**Blocked by:** none (codegen feature)

When a schema field or type carries a `semanticType` attribute
(e.g. `semanticType="Price"`, `semanticType="Timestamp"`,
`semanticType="Quantity"`), generate `From`/`Into` impls that convert between
the raw SBE wire type and the semantic Rust type.

This is gated behind `CompatibilityMode::WireCompatibleExtensions` — the wire
bytes are unchanged, only the Rust API surface grows.

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

- [ ] Read `semanticType` from XML type/field attributes (parser already
  captures this in IR)
- [ ] Registry: map from `semanticType` string to converter pair
  (wire → semantic, semantic → wire)
- [ ] Built-in converters: `Price`/`Quantity` (→ `rust_decimal`),
  `Timestamp` (→ `chrono`), `String` (var-data → `&str`/`String`)
- [ ] User-extensible: `GenerationConfig.semantic_registry` for custom
  converters
- [ ] Gated: only emitted when `compatibility == WireCompatibleExtensions`
- [ ] Tests: round-trip encode→decode→convert→semantic-equal

Ref: DECISIONS.md §11, gap analysis todo 51, user request for extension
methods/converters.
