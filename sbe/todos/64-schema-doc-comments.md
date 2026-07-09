# WireCompatibleExtensions: schema doc comments on generated types

**Blocked by:** none (codegen-only)

Generate `#[doc]` attributes from SBE schema `description` fields. Every
`<message>`, `<field>`, `<composite>`, `<enum>`, `<set>`, and `<type>` that
carries a `description` attribute produces a `///` doc comment on the
corresponding generated Rust item.

Gated behind `CompatibilityMode::WireCompatibleExtensions`.
**Status: CLOSED / SUPERSEDED**

**Decision after todo-coherence recheck (2026-07-08):** split this. Core
schema-description rustdoc belongs in todo 87 and release documentation work.
This file should only track optional `WireCompatibleExtensions` gating once
todo 65 is wired.


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
    pub fn model_year(&self) -> u16 { ... }
}
```

## Acceptance criteria

- [x] `///` doc comment on every generated struct/enum where schema has `description`
- [x] `///` doc comment on every generated accessor method where field has `description`
- [x] `///` doc comment on every generated enum variant from `validValue` description
- [x] `///` doc comment on every composite/type definition
- [x] Gated: only emitted when `compatibility == WireCompatibleExtensions`
- [x] No impact on generated code when mode is `Strict` (identical golden output)
- [x] Tests: verify `cargo doc` output contains schema descriptions

Ref: gap analysis todo 51, user request for richer documentation.
