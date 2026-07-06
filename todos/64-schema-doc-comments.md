# WireCompatibleExtensions: schema doc comments on generated types

**Blocked by:** none (codegen-only)

Generate `#[doc]` attributes from SBE schema `description` fields. Every
`<message>`, `<field>`, `<composite>`, `<enum>`, `<set>`, and `<type>` that
carries a `description` attribute produces a `///` doc comment on the
corresponding generated Rust item.

Gated behind `CompatibilityMode::WireCompatibleExtensions`.

## Example

```xml
<message name="Car" id="1" description="Description of a basic Car">
  <field name="modelYear" id="2" type="ModelYear"
         description="Year of manufacture, e.g. 2013"/>
</message>
```
```rust
/// Description of a basic Car.
pub struct CarDecoder<'a> { ... }

impl<'a> CarDecoder<'a> {
    /// Year of manufacture, e.g. 2013.
    pub const fn model_year(&self) -> u16 { ... }
}
```

## Acceptance criteria

- [ ] `///` doc comment on every generated struct/enum where schema has `description`
- [ ] `///` doc comment on every generated accessor method where field has `description`
- [ ] `///` doc comment on every generated enum variant from `validValue` description
- [ ] `///` doc comment on every composite/type definition
- [ ] Gated: only emitted when `compatibility == WireCompatibleExtensions`
- [ ] No impact on generated code when mode is `Strict` (identical golden output)
- [ ] Tests: verify `cargo doc` output contains schema descriptions

Ref: gap analysis todo 51, user request for richer documentation.
