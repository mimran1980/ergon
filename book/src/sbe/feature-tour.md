# Feature tour

Runnable examples pull live source from `sbe-feature-tour` (compiled by
`docs_validation_test`). Schematics that cannot run in the harness stay
`rust,ignore`.

- [Exact sizing](feature-tour/exact-sizing.md): `compute_length_with_header` before you encode
- [Bulk arrays](feature-tour/bulk-arrays.md): `bulk_add(&[Entry])` for fixed-stride leaf groups
- [Decoder lanes](feature-tour/decode-stages.md): random access, staged `into_*`, ordered callbacks, `.memoized()`
- [What generated code looks like](feature-tour/generated-code.md): stages, metadata, reserved names
- [Trust boundaries](feature-tour/trust-boundaries.md): `try_*` returns `Result`; `wrap` panics if short; `decode` is hybrid
- [Domain objects (DTOs)](feature-tour/domain-objects.md): owned snapshots (never on the hot path)
- [Multi-template dispatch](feature-tour/multi-template.md): `AnyMessage` routes by template ID
- [Code-generation hooks](configuration/hooks.md): append Rust tokens (serde, custom validation) after each generated item

## Every feature, one link each

| Area | Feature | Where |
|---|---|---|
| Generate | `generate(..)` in `build.rs`, `write_to_out_dir` / `write_to` | [Generate in build.rs](getting-started/generate.md) |
| Generate | `sbe_mod!` / `include_sbe!` | [Include generated code](getting-started/include.md) |
| Generate | build vs runtime dependency, generator as a library | [Depend on the generator](getting-started/depend.md) |
| Generate | several schemas, `SchemaSource::with_shared`, `with_shared_module` | [Multi-schema patterns](getting-started/multi-schema.md) |
| Generate | parse diagnostics (`miette` spans, `fancy` feature) | [Error diagnostics](getting-started/error-diagnostics.md) |
| Generate | `validate_against_sbe_xsd`, `SchemaSource::with_xsd_validation` | [Extension XML: what is captured](configuration/extension-xml.md#what-is-captured) |
| Generate | moving from sbe-tool | [Coming from sbe-tool](getting-started/from-sbe-tool.md) |
| Encode | type-state stages in wire order | [Wire order via named stages](core-concepts/wire-order-stages.md) |
| Encode | method chaining, `fixed(&FixedFields)` | [Method chaining](getting-started/method-chaining.md) |
| Encode | optional fields, `apply_nulls` | [Encode and decode](getting-started/encode-decode.md#optional-fields-and-apply_nulls) |
| Encode | exact buffer sizing, `*EncodedLength` | [Buffer sizing](core-concepts/buffer-sizing.md) |
| Encode | Aeron `try_claim`: `Claim`, `send_fixed`, `send`, `OfferClaim` | [Aeron try_claim](recipes/aeron-try-claim.md) |
| Decode | composites, endianness, layout | [Composite layout and endianness](core-concepts/composite-layout.md) |
| Decode | flyweights vs owned structs | [Flyweight vs whole-struct](core-concepts/flyweight-vs-struct.md) |
| Decode | checked entry points, tails checked as reached | [Trust boundary](core-concepts/trust-boundary.md) |
| Decode | `NullVal` and `Option` | [Why NullVal instead of Option](design-notes/nullval.md) |
| Decode | `Display` / `Debug` text | [Display and Debug](recipes/display-debug.md) |
| Decode | schema descriptions as rustdoc | [Schema descriptions as rustdoc](recipes/schema-rustdoc.md) |
| Config | presets: `new`, `lean`, `profile(GenerationProfile)` | [GenerationConfig options](configuration/generation-config.md) |
| Config | `with_conversion`, `with_domain_type`, `with_manual_domain_type`, selectors | [with_conversion vs with_domain_type](configuration/conversion-vs-domain.md) |
| Config | `with_enum_mapping`, `with_composite_mapping` | [App types on composites](recipes/app-types-composites.md) |
| Config | `with_null_as_option`, `with_all_enums_as_option` | [Opting into Option](design-notes/nullval.md#opting-into-optiont-with-with_null_as_option) |
| Config | `with_bool_domain_type` | [Null-aware accessors on BooleanType](design-notes/nullval.md#null-aware-accessors-on-booleantype) |
| Config | `with_domain_objects(DomainVarData)` | [Domain DTOs](recipes/domain-dtos.md) |
| Config | `compact_str`, `smol_str`, `bytes`, `chrono` features | [Feature integrations](configuration/feature-integrations.md) |
| Config | timestamp conversions | [Timestamp conversions](recipes/timestamps.md) |
| Config | `with_validate` | [Value validation](core-concepts/trust-boundary.md#value-validation-opt-in) |
| Config | `with_source_traits`, `with_encode_version`, `with_external_sbe_rt`, `with_module_name`, `with_keyword_append_token`, `with_deprecated_attrs`, `with_display_debug`, `with_meta_attributes`, `with_dispatch` | [GenerationConfig options](configuration/generation-config.md) |
| Extend | `with_hook`, contexts, accessor names and types | [Code-generation hooks](configuration/hooks.md#contexts) |
| Extend | `with_try_hook`, `GenerateError::Hook` | [Hooks that can fail](configuration/hooks.md#hooks-that-can-fail) |
| Extend | schema extension marks, any attribute, prefixed or not | [Extension XML](configuration/extension-xml.md) |
| Extend | `with_extension_consts` | [Every mark as a const](configuration/extension-xml.md#every-mark-as-a-const) |
| Extend | example label and field-pair hooks | [Example hooks](configuration/extension-xml.md#example-hooks) |
| Extend | `ConversionSelector::extension_attr` | [ExtensionAttr conversions](configuration/extension-xml.md#extensionattr-conversions) |
| Extend | a venue's SBE responses rendered as its JSON | [Exchange example](../samples/exchange-example.md#sbe-responses-back-to-the-venues-json) |
| Reference | type-state cost, API freeze, feature matrix | [Type-state](design-notes/type-state.md), [API freeze](design-notes/api-freeze.md), [Feature matrix](design-notes/feature-matrix.md) |
| Reference | benchmarks against sbe-tool | [Benchmarks](benchmarks.md), [Methodology](benchmarks/methodology.md) |
